use crate::{database::Database, oauth::{OAuthProviderConfig, OAuthRuntime, RuntimeProvider}};
use hbb_common::tokio::sync::{Mutex, MutexGuard};
use serde::{Deserialize, Serialize};
use sodiumoxide::crypto::secretbox;

#[derive(Clone)]
pub struct ProviderSecretKey(secretbox::Key);

impl ProviderSecretKey {
    pub(crate) fn matches(&self, bytes: &[u8]) -> bool { sodiumoxide::utils::memcmp(self.0.as_ref(), bytes) }
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
        sodiumoxide::init().map_err(|_| "unable to initialize provider encryption")?;
        secretbox::Key::from_slice(bytes).map(Self).ok_or("API_OAUTH_CONFIG_KEY must contain exactly 32 bytes")
    }

    pub fn from_base64(value: &str) -> Result<Self, &'static str> {
        let bytes = base64::decode(value).map_err(|_| "API_OAUTH_CONFIG_KEY must be base64")?;
        Self::from_bytes(&bytes)
    }

    fn seal(&self, namespace: &str, secret: &str) -> Result<String, AdminError> {
        let nonce = secretbox::gen_nonce();
        let message = serde_json::to_vec(&(namespace, secret)).map_err(|_| AdminError::Internal)?;
        let mut bytes = nonce.as_ref().to_vec();
        bytes.extend(secretbox::seal(&message, &nonce, &self.0));
        Ok(base64::encode(bytes))
    }

    fn open(&self, namespace: &str, encrypted: &str) -> Result<String, AdminError> {
        let bytes = base64::decode(encrypted).map_err(|_| AdminError::Encryption)?;
        if bytes.len() < secretbox::NONCEBYTES { return Err(AdminError::Encryption); }
        let nonce = secretbox::Nonce::from_slice(&bytes[..secretbox::NONCEBYTES]).ok_or(AdminError::Encryption)?;
        let message = secretbox::open(&bytes[secretbox::NONCEBYTES..], &nonce, &self.0).map_err(|_| AdminError::Encryption)?;
        let (bound_namespace, secret): (String, String) = serde_json::from_slice(&message).map_err(|_| AdminError::Encryption)?;
        if bound_namespace != namespace { return Err(AdminError::Encryption); }
        Ok(secret)
    }
}

#[derive(Clone, sqlx::FromRow)]
pub(crate) struct ProviderRecord {
    pub id: String,
    pub name: String,
    pub namespace: String,
    pub authority: String,
    pub source: String,
    pub config: String,
    pub encrypted_secret: String,
    pub enabled: i64,
    pub deleted: i64,
    pub revision: String,
}

#[derive(Debug)]
pub(crate) enum AdminError {
    Invalid(&'static str), Conflict, NotFound, ReadOnly, KeyMissing, Encryption, Internal,
}

#[derive(Deserialize)]
pub(crate) struct ProviderRequest {
    #[serde(default)] pub id: String,
    #[serde(default)] pub enabled: bool,
    #[serde(flatten)] pub config: OAuthProviderConfig,
}

#[derive(Serialize)]
pub(crate) struct ProviderView {
    pub id: String,
    pub enabled: bool,
    pub source: String,
    pub read_only: bool,
    pub secret_configured: bool,
    #[serde(flatten)] pub config: OAuthProviderConfig,
}

/// Owns validation, persistence and runtime replacement. HTTP mutations hold `lock()`
/// through native-flow invalidation, so new requests cannot slip between those steps.
pub(crate) struct OAuthProviderAdmin {
    db: Database,
    runtime: OAuthRuntime,
    environment: Vec<OAuthProviderConfig>,
    key: Option<ProviderSecretKey>,
    mutation: Mutex<()>,
}

impl OAuthProviderAdmin {
    pub async fn initialize(db: Database, runtime: OAuthRuntime, key: Option<ProviderSecretKey>) -> Result<Self, AdminError> {
        let environment = runtime.configs().map_err(|_| AdminError::Internal)?;
        let manager = Self { db, runtime, environment, key, mutation: Mutex::new(()) };
        for config in &manager.environment {
            let record = record(config, config.name.clone(), "environment", "", true)?;
            manager.db.register_environment_provider(&record).await.map_err(|_| AdminError::Conflict)?;
        }
        manager.reload().await?;
        Ok(manager)
    }

    pub async fn lock(&self) -> MutexGuard<'_, ()> { self.mutation.lock().await }

    pub async fn list(&self) -> Result<Vec<ProviderView>, AdminError> {
        let records = self.db.oauth_provider_records().await.map_err(|_| AdminError::Internal)?;
        records.into_iter().filter(|row| row.deleted == 0 && (row.source != "environment" || self.environment.iter().any(|config| config.name == row.name))).map(|row| {
            let config = serde_json::from_str(&row.config).map_err(|_| AdminError::Internal)?;
            Ok(ProviderView { id: row.id, enabled: row.enabled != 0, source: row.source.clone(),
                read_only: row.source != "database", secret_configured: row.source == "environment" || !row.encrypted_secret.is_empty(), config })
        }).collect()
    }

    async fn reload(&self) -> Result<(), AdminError> {
        let rows = self.db.oauth_provider_records().await.map_err(|_| AdminError::Internal)?;
        self.runtime.replace(self.prepare(rows)?).await.map_err(|_| AdminError::Internal)
    }

    fn prepare(&self, rows: Vec<ProviderRecord>) -> Result<Vec<RuntimeProvider>, AdminError> {
        let mut providers = Vec::new();
        for row in rows.into_iter().filter(|row| row.deleted == 0) {
            let config = if row.source == "environment" {
                match self.environment.iter().find(|config| config.name == row.name) {
                    Some(config) => config.clone(),
                    None => continue,
                }
            } else {
                let key = self.key.as_ref().ok_or(AdminError::KeyMissing)?;
                let mut config: OAuthProviderConfig = serde_json::from_str(&row.config).map_err(|_| AdminError::Internal)?;
                config.client_secret = key.open(&row.namespace, &row.encrypted_secret)?;
                config
            };
            config.validate().map_err(AdminError::Invalid)?;
            if row.enabled != 0 { providers.push(RuntimeProvider { config, namespace: row.namespace, revision: row.revision }); }
        }
        Ok(providers)
    }

    pub async fn create(&self, request: ProviderRequest) -> Result<ProviderView, AdminError> {
        request.config.validate().map_err(AdminError::Invalid)?;
        let key = self.key.as_ref().ok_or(AdminError::KeyMissing)?;
        let namespace = uuid::Uuid::new_v4().to_string();
        let encrypted = key.seal(&namespace, &request.config.client_secret)?;
        let record = record(&request.config, namespace, "database", &encrypted, request.enabled)?;
        let mut rows = self.db.oauth_provider_records().await.map_err(|_| AdminError::Internal)?;
        if rows.iter().any(|row| row.name == record.name) { return Err(AdminError::Conflict); }
        rows.push(record.clone());
        let providers = self.prepare(rows)?;
        let view = provider_view(&record)?;
        self.db.insert_oauth_provider(&record).await.map_err(|_| AdminError::Internal)?;
        self.runtime.replace(providers).await.map_err(|_| AdminError::Internal)?;
        Ok(view)
    }

    pub async fn update(&self, mut request: ProviderRequest) -> Result<ProviderView, AdminError> {
        let mut row = self.editable(&request.id).await?;
        if authority(&request.config)? != row.authority || request.config.name != row.name {
            return Err(AdminError::Invalid("identity authority is immutable; create a new provider name"));
        }
        let key = self.key.as_ref().ok_or(AdminError::KeyMissing)?;
        if request.config.client_secret.trim().is_empty() { request.config.client_secret = key.open(&row.namespace, &row.encrypted_secret)?; }
        request.config.validate().map_err(AdminError::Invalid)?;
        let previous = row.revision.clone();
        row.encrypted_secret = key.seal(&row.namespace, &request.config.client_secret)?;
        row.config = redacted(&request.config)?;
        row.enabled = request.enabled as i64;
        row.revision = uuid::Uuid::new_v4().to_string();
        self.save(&row, &previous).await?;
        provider_view(&row)
    }

    pub async fn toggle(&self, id: &str, enabled: bool) -> Result<ProviderView, AdminError> {
        let mut row = self.editable(id).await?;
        let previous = row.revision.clone();
        row.enabled = enabled as i64;
        row.revision = uuid::Uuid::new_v4().to_string();
        self.save(&row, &previous).await?;
        provider_view(&row)
    }

    pub async fn delete(&self, id: &str) -> Result<String, AdminError> {
        let mut row = self.editable(id).await?;
        let previous = row.revision.clone();
        row.deleted = 1;
        row.enabled = 0;
        row.encrypted_secret.clear();
        row.revision = uuid::Uuid::new_v4().to_string();
        self.save(&row, &previous).await?;
        Ok(row.name)
    }

    async fn editable(&self, id: &str) -> Result<ProviderRecord, AdminError> {
        let row = self.db.oauth_provider_records().await.map_err(|_| AdminError::Internal)?
            .into_iter().find(|row| row.id == id && row.deleted == 0).ok_or(AdminError::NotFound)?;
        if row.source != "database" { return Err(AdminError::ReadOnly); }
        Ok(row)
    }

    async fn save(&self, row: &ProviderRecord, previous: &str) -> Result<(), AdminError> {
        let mut rows = self.db.oauth_provider_records().await.map_err(|_| AdminError::Internal)?;
        let existing = rows.iter_mut().find(|existing| existing.id == row.id).ok_or(AdminError::NotFound)?;
        *existing = row.clone();
        // Validate/decrypt the complete replacement before any durable mutation.
        let providers = self.prepare(rows)?;
        if !self.db.update_oauth_provider(row, previous).await.map_err(|_| AdminError::Internal)? { return Err(AdminError::Conflict); }
        self.runtime.replace(providers).await.map_err(|_| AdminError::Internal)
    }
}

fn authority(config: &OAuthProviderConfig) -> Result<String, AdminError> {
    serde_json::to_string(&(config.kind, &config.client_id, &config.authorization_url,
        &config.token_url, &config.userinfo_url, &config.issuer_url, &config.jwks_url)).map_err(|_| AdminError::Internal)
}

fn redacted(config: &OAuthProviderConfig) -> Result<String, AdminError> {
    let mut config = config.clone();
    config.client_secret.clear();
    serde_json::to_string(&config).map_err(|_| AdminError::Internal)
}

fn record(config: &OAuthProviderConfig, namespace: String, source: &str, encrypted: &str, enabled: bool) -> Result<ProviderRecord, AdminError> {
    Ok(ProviderRecord { id: uuid::Uuid::new_v4().to_string(), name: config.name.clone(), namespace,
        authority: authority(config)?, source: source.to_owned(), config: redacted(config)?,
        encrypted_secret: encrypted.to_owned(), enabled: enabled as i64, deleted: 0, revision: uuid::Uuid::new_v4().to_string() })
}

fn provider_view(row: &ProviderRecord) -> Result<ProviderView, AdminError> {
    let config = serde_json::from_str(&row.config).map_err(|_| AdminError::Internal)?;
    Ok(ProviderView { id: row.id.clone(), enabled: row.enabled != 0, source: row.source.clone(),
        read_only: row.source != "database", secret_configured: !row.encrypted_secret.is_empty(), config })
}
