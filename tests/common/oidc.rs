use axum::{extract::{Form, Extension}, routing::{get, post}, Json, Router};
use hbbs::oauth::{OAuthProviderConfig, OAuthProviderKind};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde_json::{json, Value};
use std::{collections::HashMap, sync::{Arc, Mutex}};

/// Isolated external OIDC adapter with a public test-only signing key.
pub struct MockOidc {
    pub base: String,
    tokens: Arc<Mutex<HashMap<String, Value>>>,
    profile: Arc<Mutex<Value>>,
    task: tokio::task::JoinHandle<()>,
}

impl MockOidc {
    pub fn start() -> Self {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let tokens: Arc<Mutex<HashMap<String, Value>>> = Arc::new(Mutex::new(HashMap::new()));
        let profile = Arc::new(Mutex::new(json!({"sub":"subject-42","preferred_username":"alice"})));
        let router = Router::new()
            .route("/token", post(|Extension(tokens): Extension<Arc<Mutex<HashMap<String, Value>>>>,
                Form(form): Form<HashMap<String, String>>| async move {
                Json(tokens.lock().unwrap().get(form.get("code").unwrap()).unwrap().clone())
            }))
            .route("/userinfo", get(|Extension(profile): Extension<Arc<Mutex<Value>>>| async move {
                Json(profile.lock().unwrap().clone())
            }))
            .route("/jwks", get(|| async {
                Json(serde_json::from_str::<Value>(include_str!("../fixtures/oidc-test-jwks.json")).unwrap())
            }))
            .layer(Extension(tokens.clone())).layer(Extension(profile.clone()));
        let server = axum::Server::from_tcp(listener).unwrap().serve(router.into_make_service());
        let task = tokio::spawn(async { server.await.unwrap(); });
        Self { base, tokens, profile, task }
    }

    pub fn config(&self) -> OAuthProviderConfig {
        OAuthProviderConfig {
            kind: OAuthProviderKind::Oidc, name: "test".to_owned(), client_id: "client".to_owned(),
            client_secret: "secret".to_owned(), authorization_url: format!("{}/authorize", self.base),
            token_url: format!("{}/token", self.base), userinfo_url: format!("{}/userinfo", self.base),
            issuer_url: self.base.clone(), jwks_url: format!("{}/jwks", self.base), scopes: "openid email".to_owned(),
        }
    }

    pub fn claims(&self, nonce: &str) -> Value {
        json!({"iss":self.base,"aud":"client","sub":"subject-42","nonce":nonce,
            "iat":hbbs::common::now(),"exp":hbbs::common::now()+300})
    }

    pub fn signed(claims: &Value) -> String {
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some("test-key".to_owned());
        Self::signed_with_header(claims, header)
    }

    pub fn signed_with_header(claims: &Value, header: Header) -> String {
        encode(&header, claims, &EncodingKey::from_rsa_pem(include_bytes!("../fixtures/oidc-test-key.pem")).unwrap()).unwrap()
    }

    pub fn token(&self, code: &str, value: Value) { self.tokens.lock().unwrap().insert(code.to_owned(), value); }
    pub fn profile(&self, value: Value) { *self.profile.lock().unwrap() = value; }
}

impl Drop for MockOidc {
    fn drop(&mut self) { self.task.abort(); }
}
