#![allow(dead_code)]
pub mod oidc;
pub mod rendezvous;

use axum::{body::Body, http::{header, Request, StatusCode}, response::Response, Router};
use hbbs::{api::{build_service, CookiePolicy, PublicServerConfig}, database::Database,
    ldap::LdapConfig, oauth::{OAuthProviderConfig, OAuthRuntime}};
use serde_json::Value;
use std::{path::PathBuf, time::Duration};
use tower::ServiceExt;

pub struct TestApp {
    pub router: Router,
    directory: PathBuf,
}

impl TestApp {
    pub async fn new(policy: CookiePolicy, oauth: OAuthRuntime) -> Self {
        Self::new_with_key(policy, oauth, Some(hbbs::oauth_admin::ProviderSecretKey::from_bytes(&[7;32]).unwrap())).await
    }

    pub async fn new_with_key(policy: CookiePolicy, oauth: OAuthRuntime, key: Option<hbbs::oauth_admin::ProviderSecretKey>) -> Self {
        Self::new_with_browser(policy,oauth,key,None).await
    }

    pub async fn new_with_browser(policy: CookiePolicy, oauth: OAuthRuntime, key: Option<hbbs::oauth_admin::ProviderSecretKey>, browser: Option<hbbs::api::BrowserPolicy>) -> Self {
        let directory = std::env::temp_dir().join(format!("rustdesk-contract-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&directory).unwrap();
        let db = Database::new(directory.join("api.sqlite3").to_str().unwrap()).await.unwrap();
        let router = build_service(db, "01234567890123456789012345678901".to_owned(),
            Duration::from_secs(3600), true, PublicServerConfig {
                api_server: "https://api.example".to_owned(), id_server: String::new(),
                relay_server: String::new(), key: String::new(),
            }, directory.to_string_lossy().into_owned(),
            Some(("admin".to_owned(), "admin-password".to_owned())), oauth,
            "https://api.example/api/oidc/callback".to_owned(), LdapConfig::disabled(), policy, key, browser,
        ).await.unwrap();
        Self { router, directory }
    }

    pub fn database_path(&self) -> PathBuf { self.directory.join("api.sqlite3") }

    pub async fn reopen(&mut self, oauth: OAuthRuntime, key: Option<hbbs::oauth_admin::ProviderSecretKey>) -> Result<(), hbbs::auth::AuthError> {
        self.router = build_service(Database::new(self.database_path().to_str().unwrap()).await.unwrap(),
            "01234567890123456789012345678901".to_owned(), Duration::from_secs(3600), true,
            PublicServerConfig { api_server:"https://api.example".to_owned(), id_server:String::new(), relay_server:String::new(), key:String::new() },
            self.directory.to_string_lossy().into_owned(), None, oauth, "https://api.example/api/oidc/callback".to_owned(),
            LdapConfig::disabled(), CookiePolicy::default(), key, None).await?;
        Ok(())
    }

    // Older contract tests send valid browser mutations; security-negative tests use send_raw.
    pub async fn send(&self, mut request: Request<Body>) -> Response {
        if !matches!(*request.method(),axum::http::Method::GET|axum::http::Method::HEAD|axum::http::Method::OPTIONS)
            && request.headers().contains_key(header::COOKIE) && !request.headers().contains_key(header::AUTHORIZATION)
            && !request.headers().contains_key(header::ORIGIN) {
            let cookie = request.headers()[header::COOKIE].clone();
            let csrf = self.send_raw(Request::builder().uri("/api/session/csrf").header(header::COOKIE,cookie).body(Body::empty()).unwrap()).await;
            if csrf.status() == StatusCode::OK {
                let value: Value = serde_json::from_slice(&axum::body::to_bytes(csrf.into_body(), 4*1024*1024).await.unwrap()).unwrap();
                request.headers_mut().insert("x-csrf-token",value["csrf_token"].as_str().unwrap().parse().unwrap());
            }
            request.headers_mut().insert(header::ORIGIN,"https://api.example".parse().unwrap());
        }
        self.send_raw(request).await
    }

    pub async fn send_raw(&self, request: Request<Body>) -> Response {
        self.router.clone().oneshot(request).await.unwrap()
    }
}

impl Drop for TestApp {
    fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.directory); }
}

pub fn request(method: &str, path: &str, body: Value) -> Request<Body> {
    Request::builder().method(method).uri(path).header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string())).unwrap()
}

pub fn cookie(response: &Response) -> String {
    response.headers().get(header::SET_COOKIE).unwrap().to_str().unwrap()
        .split(';').next().unwrap().to_owned()
}

pub fn assert_secure_cookie(response: &Response, cleared: bool) {
    assert_eq!(response.status(), StatusCode::OK);
    let value = response.headers().get(header::SET_COOKIE).unwrap().to_str().unwrap();
    for attribute in ["; Secure", "; HttpOnly", "; SameSite=Lax", "; Path=/"] {
        assert!(value.contains(attribute), "missing {attribute}: {value}");
    }
    assert!(!value.contains("Domain="));
    if cleared { assert!(value.contains("Max-Age=0")); }
}

pub struct MockOAuth {
    pub base: String,
    pub token_requests: std::sync::Arc<std::sync::Mutex<Vec<std::collections::HashMap<String, String>>>>,
    pub pause_tokens: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub token_entered: std::sync::Arc<tokio::sync::Semaphore>,
    pub token_release: std::sync::Arc<tokio::sync::Semaphore>,
    task: tokio::task::JoinHandle<()>,
}

impl MockOAuth {
    pub fn start() -> Self {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let token_requests = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let pause_tokens = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let token_entered = std::sync::Arc::new(tokio::sync::Semaphore::new(0));
        let token_release = std::sync::Arc::new(tokio::sync::Semaphore::new(0));
        let (requests, pause, entered, release) = (token_requests.clone(), pause_tokens.clone(), token_entered.clone(), token_release.clone());
        let router = Router::new()
            .route("/token", axum::routing::post(move |body: String| {
                let (requests, pause, entered, release) = (requests.clone(), pause.clone(), entered.clone(), release.clone());
                async move {
                    let url = reqwest::Url::parse(&format!("http://localhost/?{body}")).unwrap();
                    let form = url.query_pairs().map(|(key,value)| (key.into_owned(),value.into_owned())).collect();
                    requests.lock().unwrap().push(form);
                    if pause.load(std::sync::atomic::Ordering::SeqCst) {
                        entered.add_permits(1);
                        release.acquire().await.unwrap().forget();
                    }
                    axum::Json(serde_json::json!({"access_token":"external-token"}))
                }
            }))
            .route("/userinfo", axum::routing::get(|| async {
                axum::Json(serde_json::json!({"id":"42","login":"alice","email":"alice@example.com"}))
            }));
        let server = axum::serve(tokio::net::TcpListener::from_std(listener).unwrap(), router.into_make_service());
        let task = tokio::spawn(async { server.await.unwrap(); });
        Self { base, token_requests, pause_tokens, token_entered, token_release, task }
    }

    pub fn runtime(&self) -> OAuthRuntime {
        OAuthRuntime::new(vec![self.config()])
    }

    pub fn config(&self) -> OAuthProviderConfig {
        OAuthProviderConfig {
            kind: hbbs::oauth::OAuthProviderKind::OAuth2,
            name: "test".to_owned(), client_id: "client".to_owned(), client_secret: "secret".to_owned(),
            authorization_url: format!("{}/authorize", self.base), token_url: format!("{}/token", self.base),
            userinfo_url: format!("{}/userinfo", self.base), issuer_url: String::new(),
            jwks_url: String::new(), scopes: "read:user".to_owned(),
        }
    }
}

impl Drop for MockOAuth {
    fn drop(&mut self) { self.task.abort(); }
}
