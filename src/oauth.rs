use reqwest::Url;
use serde::Deserialize;
use std::{collections::HashMap, sync::Arc};
use hbb_common::tokio;

#[derive(Clone)]
pub struct OAuthProviderConfig {
    pub name: String,
    pub client_id: String,
    pub client_secret: String,
    pub authorization_url: String,
    pub token_url: String,
    pub userinfo_url: String,
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
    expires_at: u64,
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
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: Option<String>,
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
        let config = self
            .providers
            .get(provider)
            .ok_or(OAuthError::NotConfigured)?;
        let state = uuid::Uuid::new_v4().to_string();
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
            .append_pair("state", &state);
        Ok(url)
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
        let access_token = token.access_token.ok_or(OAuthError::InvalidResponse)?;
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
        })
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

    fn test_runtime() -> OAuthRuntime {
        OAuthRuntime::new(vec![OAuthProviderConfig {
            name: "test".to_owned(),
            client_id: "client".to_owned(),
            client_secret: "secret".to_owned(),
            authorization_url: "https://provider.example/authorize".to_owned(),
            token_url: "https://provider.example/token".to_owned(),
            userinfo_url: "https://provider.example/userinfo".to_owned(),
            scopes: "openid email".to_owned(),
        }])
    }

    #[tokio::test]
    async fn begin_stores_one_time_state_in_authorization_url() {
        let runtime = test_runtime();
        let url = runtime
            .begin("test", "https://api.example/callback")
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
        assert!(runtime
            .complete("code", &state, "https://api.example/callback")
            .await
            .is_err());
        assert!(runtime
            .complete("code", &state, "https://api.example/callback")
            .await
            .is_err());
    }
}
