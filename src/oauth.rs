use hbb_common::tokio;
use jsonwebtoken::{decode, decode_header, DecodingKey, Validation};
use reqwest::Url;
use serde::Deserialize;
use std::{collections::HashMap, sync::Arc};

#[derive(Clone)]
pub struct OAuthProviderConfig {
    pub name: String,
    pub client_id: String,
    pub client_secret: String,
    pub authorization_url: String,
    pub token_url: String,
    pub userinfo_url: String,
    pub issuer_url: String,
    pub jwks_url: String,
    pub scopes: String,
}

#[derive(Clone)]
pub struct OAuthRuntime {
    providers: Arc<HashMap<String, OAuthProviderConfig>>,
    pending: Arc<tokio::sync::Mutex<HashMap<String, PendingState>>>,
}

struct PendingState {
    provider: String,
    redirect_uri: String,
    code_verifier: String,
    nonce: String,
    device: OAuthDevice,
    expires_at: u64,
}

#[derive(Debug, Clone, Default)]
pub struct OAuthDevice {
    pub id: String,
    pub uuid: String,
    pub name: String,
    pub os: String,
    pub device_type: String,
}

#[derive(Debug)]
pub enum OAuthError {
    InvalidState,
    NotConfigured,
    InvalidResponse,
    Remote,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct OAuthProviderView {
    pub name: String,
    pub authorization_url: String,
    pub userinfo_url: String,
    pub scopes: String,
}

#[derive(Debug, Clone)]
pub struct ExternalIdentity {
    pub provider: String,
    pub subject: String,
    pub username: String,
    pub email: String,
    pub device: OAuthDevice,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: Option<String>,
    id_token: Option<String>,
}

impl OAuthRuntime {
    pub fn new(configs: Vec<OAuthProviderConfig>) -> Self {
        let providers = configs
            .into_iter()
            .filter(|config| {
                !config.name.is_empty()
                    && !config.client_id.is_empty()
                    && !config.client_secret.is_empty()
                    && !config.authorization_url.is_empty()
                    && !config.token_url.is_empty()
                    && !config.userinfo_url.is_empty()
                    && valid_provider_endpoint(&config.authorization_url)
                    && valid_provider_endpoint(&config.token_url)
                    && valid_provider_endpoint(&config.userinfo_url)
                    && (config.issuer_url.is_empty() == config.jwks_url.is_empty())
                    && (config.issuer_url.is_empty() || valid_provider_endpoint(&config.issuer_url))
                    && (config.jwks_url.is_empty() || valid_provider_endpoint(&config.jwks_url))
            })
            .map(|config| (config.name.clone(), config))
            .collect();
        Self {
            providers: Arc::new(providers),
            pending: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        }
    }

    pub fn provider_names(&self) -> Vec<String> {
        let mut names = self.providers.keys().cloned().collect::<Vec<_>>();
        names.sort();
        names
    }

    pub fn provider_views(&self) -> Vec<OAuthProviderView> {
        let mut providers = self
            .providers
            .values()
            .map(|config| OAuthProviderView {
                name: config.name.clone(),
                authorization_url: config.authorization_url.clone(),
                userinfo_url: config.userinfo_url.clone(),
                scopes: config.scopes.clone(),
            })
            .collect::<Vec<_>>();
        providers.sort_by(|left, right| left.name.cmp(&right.name));
        providers
    }

    pub async fn begin(&self, provider: &str, redirect_uri: &str) -> Result<Url, OAuthError> {
        self.begin_with_device(provider, redirect_uri, OAuthDevice::default())
            .await
            .map(|(url, _)| url)
    }

    pub async fn begin_with_device(
        &self,
        provider: &str,
        redirect_uri: &str,
        device: OAuthDevice,
    ) -> Result<(Url, String), OAuthError> {
        let config = self
            .providers
            .get(provider)
            .ok_or(OAuthError::NotConfigured)?;
        let state = uuid::Uuid::new_v4().to_string();
        let nonce = uuid::Uuid::new_v4().to_string();
        let code_verifier = format!(
            "{}{}",
            uuid::Uuid::new_v4().simple(),
            uuid::Uuid::new_v4().simple()
        );
        let challenge = base64::encode_config(
            sodiumoxide::crypto::hash::sha256::hash(
                code_verifier.as_bytes(),
            )
            .as_ref(),
            base64::URL_SAFE_NO_PAD,
        );
        let now = crate::common::now();
        let mut pending = self.pending.lock().await;
        pending.retain(|_, value| value.expires_at >= now);
        if pending.len() >= 10_000 {
            return Err(OAuthError::Remote);
        }
        pending.insert(
            state.clone(),
            PendingState {
                provider: provider.to_owned(),
                redirect_uri: redirect_uri.to_owned(),
                code_verifier,
                nonce: nonce.clone(),
                device,
                expires_at: now.saturating_add(300),
            },
        );
        let mut url = Url::parse(&config.authorization_url).map_err(|_| OAuthError::InvalidResponse)?;
        url.query_pairs_mut()
            .append_pair("client_id", &config.client_id)
            .append_pair("redirect_uri", redirect_uri)
            .append_pair("response_type", "code")
            .append_pair("scope", &config.scopes)
            .append_pair("code_challenge", &challenge)
            .append_pair("code_challenge_method", "S256")
            .append_pair("state", &state)
            .append_pair("nonce", &nonce);
        Ok((url, state))
    }

    pub async fn complete(
        &self,
        code: &str,
        state: &str,
        redirect_uri: &str,
    ) -> Result<ExternalIdentity, OAuthError> {
        if code.is_empty() || state.is_empty() {
            return Err(OAuthError::InvalidState);
        }
        let pending = self
            .pending
            .lock()
            .await
            .remove(state)
            .ok_or(OAuthError::InvalidState)?;
        if pending.redirect_uri != redirect_uri || pending.expires_at < crate::common::now() {
            return Err(OAuthError::InvalidState);
        }
        let provider = pending.provider;
        let code_verifier = pending.code_verifier;
        let nonce = pending.nonce;
        let device = pending.device;
        let config = self
            .providers
            .get(&provider)
            .ok_or(OAuthError::NotConfigured)?;
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(|_| OAuthError::Remote)?;
        let token = client
            .post(&config.token_url)
            .header("Accept", "application/json")
            .form(&[
                ("client_id", config.client_id.as_str()),
                ("client_secret", config.client_secret.as_str()),
                ("code", code),
                ("redirect_uri", redirect_uri),
                ("grant_type", "authorization_code"),
                ("code_verifier", code_verifier.as_str()),
            ])
            .send()
            .await
            .map_err(|_| OAuthError::Remote)?
            .error_for_status()
            .map_err(|_| OAuthError::Remote)?
            .json::<TokenResponse>()
            .await
            .map_err(|_| OAuthError::InvalidResponse)?;
        let access_token = token
            .access_token
            .clone()
            .and_then(|value| {
                let value = value.trim();
                (!value.is_empty()).then(|| value.to_owned())
            })
            .ok_or(OAuthError::InvalidResponse)?;
        if let Some(id_token) = token.id_token.as_deref() {
            validate_id_token(&client, config, id_token, &nonce).await?;
        }
        let profile = client
            .get(&config.userinfo_url)
            .bearer_auth(access_token)
            .header("User-Agent", "rustdesk-api")
            .send()
            .await
            .map_err(|_| OAuthError::Remote)?
            .error_for_status()
            .map_err(|_| OAuthError::Remote)?
            .json::<serde_json::Value>()
            .await
            .map_err(|_| OAuthError::InvalidResponse)?;
        let subject = profile
            .get("sub")
            .or_else(|| profile.get("id"))
            .and_then(json_scalar_string)
            .filter(|value| !value.trim().is_empty())
            .ok_or(OAuthError::InvalidResponse)?;
        let username = profile
            .get("preferred_username")
            .or_else(|| profile.get("login"))
            .or_else(|| profile.get("name"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or(subject.as_str())
            .to_owned();
        let email = profile
            .get("email")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned();
        Ok(ExternalIdentity {
            provider,
            subject,
            username,
            email,
            device,
        })
    }
}

async fn validate_id_token(
    client: &reqwest::Client,
    config: &OAuthProviderConfig,
    token: &str,
    nonce: &str,
) -> Result<(), OAuthError> {
    let header = decode_header(token).map_err(|_| OAuthError::InvalidResponse)?;
    if !matches!(header.alg, jsonwebtoken::Algorithm::RS256 | jsonwebtoken::Algorithm::RS384 | jsonwebtoken::Algorithm::RS512) {
        return Err(OAuthError::InvalidResponse);
    }
    let key_set = client
        .get(&config.jwks_url)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|_| OAuthError::Remote)?
        .error_for_status()
        .map_err(|_| OAuthError::Remote)?
        .json::<jsonwebtoken::jwk::JwkSet>()
        .await
        .map_err(|_| OAuthError::InvalidResponse)?;
    let key_id = header.kid.as_deref().ok_or(OAuthError::InvalidResponse)?;
    let jwk = key_set.find(key_id).ok_or(OAuthError::InvalidResponse)?;
    if jwk.common.algorithm.is_some_and(|algorithm| algorithm != header.alg) {
        return Err(OAuthError::InvalidResponse);
    }
    let parameters = match &jwk.algorithm {
        jsonwebtoken::jwk::AlgorithmParameters::RSA(parameters) => parameters,
        _ => return Err(OAuthError::InvalidResponse),
    };
    let key = DecodingKey::from_rsa_components(&parameters.n, &parameters.e)
        .map_err(|_| OAuthError::InvalidResponse)?;
    let mut validation = Validation::new(header.alg);
    validation.set_issuer(&[config.issuer_url.as_str()]);
    validation.set_audience(&[config.client_id.as_str()]);
    let claims = decode::<IdTokenClaims>(token, &key, &validation)
        .map_err(|_| OAuthError::InvalidResponse)?
        .claims;
    if claims.iss != config.issuer_url
        || claims.sub.trim().is_empty()
        || claims.nonce.as_deref() != Some(nonce)
        || !claims.audience_contains(&config.client_id)
    {
        return Err(OAuthError::InvalidResponse);
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
struct IdTokenClaims {
    iss: String,
    sub: String,
    aud: serde_json::Value,
    nonce: Option<String>,
}

impl IdTokenClaims {
    fn audience_contains(&self, audience: &str) -> bool {
        match &self.aud {
            serde_json::Value::String(value) => value == audience,
            serde_json::Value::Array(values) => values.iter().any(|value| value.as_str() == Some(audience)),
            _ => false,
        }
    }
}

fn valid_provider_endpoint(value: &str) -> bool {
    let url = match Url::parse(value) {
        Ok(url) => url,
        Err(_) => return false,
    };
    if url.fragment().is_some() || url.username() != "" || url.password().is_some() {
        return false;
    }
    match url.scheme() {
        "https" => url.host_str().is_some(),
        "http" => matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]" | "::1")),
        _ => false,
    }
}

fn json_scalar_string(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(value) => Some(value.clone()),
        serde_json::Value::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hbb_common::tokio::io::{AsyncReadExt, AsyncWriteExt};

    fn test_runtime() -> OAuthRuntime {
        OAuthRuntime::new(vec![OAuthProviderConfig {
            name: "test".to_owned(),
            client_id: "client".to_owned(),
            client_secret: "secret".to_owned(),
            authorization_url: "https://provider.example/authorize".to_owned(),
            token_url: "https://provider.example/token".to_owned(),
            userinfo_url: "https://provider.example/userinfo".to_owned(),
            issuer_url: String::new(),
            jwks_url: String::new(),
            scopes: "openid email".to_owned(),
        }])
    }

    #[tokio::test]
    async fn begin_stores_one_time_state_in_authorization_url() {
        let runtime = test_runtime();
        let (url, returned_state) = runtime
            .begin_with_device(
                "test",
                "https://api.example/callback",
                OAuthDevice {
                    id: "device-id".to_owned(),
                    uuid: "device-uuid".to_owned(),
                    ..OAuthDevice::default()
                },
            )
            .await
            .expect("authorization URL should be created");
        assert_eq!(url.host_str(), Some("provider.example"));
        assert_eq!(
            url.query_pairs()
                .find(|(key, _)| key == "client_id")
                .map(|(_, value)| value.into_owned()),
            Some("client".to_owned())
        );
        assert_eq!(
            url.query_pairs()
                .find(|(key, _)| key == "redirect_uri")
                .map(|(_, value)| value.into_owned()),
            Some("https://api.example/callback".to_owned())
        );
        let state = url
            .query_pairs()
            .find(|(key, _)| key == "state")
            .map(|(_, value)| value.into_owned())
            .expect("state should be present");
        assert_eq!(returned_state, state);
        assert!(runtime
            .complete("code", &state, "https://api.example/callback")
            .await
            .is_err());
        assert!(runtime
            .complete("code", &state, "https://api.example/callback")
            .await
            .is_err());
    }

    #[tokio::test]
    async fn complete_exchanges_code_and_resolves_userinfo_identity() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("mock OAuth listener should bind");
        let address = listener.local_addr().expect("mock listener address should exist");
        let server = tokio::spawn(async move {
            for _ in 0..2 {
                let (mut stream, _) = listener.accept().await.expect("mock request should connect");
                let mut request = [0_u8; 4096];
                let size = stream.read(&mut request).await.expect("mock request should read");
                let request = String::from_utf8_lossy(&request[..size]);
                let body = if request.starts_with("POST /token ") {
                    r#"{"access_token":"mock-access-token"}"#
                } else {
                    r#"{"id":"42","login":"alice","email":"alice@example.com"}"#
                };
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                stream
                    .write_all(response.as_bytes())
                    .await
                    .expect("mock response should write");
            }
        });
        let base = format!("http://{address}");
        let runtime = OAuthRuntime::new(vec![OAuthProviderConfig {
            name: "local".to_owned(),
            client_id: "client".to_owned(),
            client_secret: "secret".to_owned(),
            authorization_url: format!("{base}/authorize"),
            token_url: format!("{base}/token"),
            userinfo_url: format!("{base}/userinfo"),
            issuer_url: String::new(),
            jwks_url: String::new(),
            scopes: "openid email".to_owned(),
        }]);
        let (_, state) = runtime
            .begin_with_device("local", "https://api.example/callback", OAuthDevice::default())
            .await
            .expect("local provider should begin");

        let identity = runtime
            .complete("authorization-code", &state, "https://api.example/callback")
            .await
            .expect("mock provider should complete");
        server.await.expect("mock OAuth server should finish");

        assert_eq!(identity.provider, "local");
        assert_eq!(identity.subject, "42");
        assert_eq!(identity.username, "alice");
        assert_eq!(identity.email, "alice@example.com");
    }

    #[test]
    fn oidc_validation_endpoints_must_be_paired() {
        let mut config = OAuthProviderConfig {
            name: "oidc".to_owned(),
            client_id: "client".to_owned(),
            client_secret: "secret".to_owned(),
            authorization_url: "https://provider.example/authorize".to_owned(),
            token_url: "https://provider.example/token".to_owned(),
            userinfo_url: "https://provider.example/userinfo".to_owned(),
            issuer_url: "https://provider.example".to_owned(),
            jwks_url: String::new(),
            scopes: "openid".to_owned(),
        };
        assert!(OAuthRuntime::new(vec![config.clone()]).provider_names().is_empty());
        config.jwks_url = "https://provider.example/.well-known/jwks.json".to_owned();
        assert_eq!(OAuthRuntime::new(vec![config]).provider_names(), vec!["oidc"]);
    }

    #[test]
    fn provider_endpoints_require_secure_or_local_urls() {
        assert!(valid_provider_endpoint("https://provider.example/token"));
        assert!(valid_provider_endpoint("http://127.0.0.1:8080/token"));
        assert!(!valid_provider_endpoint("http://provider.example/token"));
        assert!(!valid_provider_endpoint("https://user:pass@provider.example/token"));
        assert!(!valid_provider_endpoint("https://provider.example/token#fragment"));
    }

    #[test]
    fn empty_identity_subject_is_not_a_scalar_identity() {
        let value = serde_json::Value::String("   ".to_owned());
        assert!(json_scalar_string(&value)
            .filter(|subject| !subject.trim().is_empty())
            .is_none());
    }
}
