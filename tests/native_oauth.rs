mod common;
use axum::{body::Body, http::{header, Request, StatusCode}};
use common::*;
use hbbs::api::CookiePolicy;
use serde::{Deserialize};
use serde_json::{json, Value};
use std::sync::{Arc, atomic::{AtomicU64, Ordering}};
use hbbs::oauth::OAuthRuntime;

/// Required root fields from rustdesk/rustdesk 1.4.9 src/hbbs_http/account.rs.
#[derive(Deserialize)]
struct OfficialAuthUrl { code: String, url: String }
#[derive(Deserialize)]
struct OfficialAuthBody { access_token: String, r#type: String, user: OfficialUser }
#[derive(Deserialize)]
struct OfficialUser { name: String, info: Value }

async fn body(response: axum::response::Response) -> Value {
    serde_json::from_slice(&hyper::body::to_bytes(response.into_body()).await.unwrap()).unwrap()
}

#[tokio::test]
async fn official_native_auth_request_returns_a_polling_code_and_browser_entry() {
    let provider = MockOAuth::start();
    let app = TestApp::new(CookiePolicy::default(), provider.runtime()).await;
    let response = app.send(request("POST", "/api/oidc/auth", json!({"op":"test","id":"123456","uuid":"device-uuid","deviceInfo":{"name":"Laptop"}}))).await;
    assert_eq!(response.status(), StatusCode::OK);
    let value = body(response).await;
    assert_eq!(value.as_object().unwrap().len(), 2);
    let auth: OfficialAuthUrl = serde_json::from_value(value).unwrap();
    assert!(!auth.code.is_empty());
    assert!(auth.url.starts_with("https://api.example/"));
    assert!(!auth.url.contains(&auth.code));
}

async fn issue(app: &TestApp) -> OfficialAuthUrl {
    let response = app.send(request("POST", "/api/oidc/auth", json!({"op":"test","id":"123456","uuid":"device-uuid",
        "deviceInfo":{"name":"Laptop","os":"Linux","type":"client"}}))).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().get(header::SET_COOKIE).is_none());
    serde_json::from_value(body(response).await).unwrap()
}

async fn poll(app: &TestApp, code: &str, id: &str, uuid: &str) -> Value {
    body(app.send(Request::builder().uri(format!("/api/oidc/auth-query?code={code}&id={id}&uuid={uuid}"))
        .body(Body::empty()).unwrap()).await).await
}

async fn open(app: &TestApp, auth: &OfficialAuthUrl) -> (String, String) {
    let url = reqwest::Url::parse(&auth.url).unwrap();
    let begin = app.send(Request::builder().uri(format!("{}?{}", url.path(), url.query().unwrap()))
        .body(Body::empty()).unwrap()).await;
    assert_eq!(begin.status(), StatusCode::TEMPORARY_REDIRECT);
    let binding = cookie(&begin);
    let redirect = reqwest::Url::parse(begin.headers().get(header::LOCATION).unwrap().to_str().unwrap()).unwrap();
    assert_eq!(redirect.query_pairs().find(|(key, _)| key == "code_challenge_method").unwrap().1, "S256");
    assert!(redirect.query_pairs().any(|(key, _)| key == "nonce"));
    (redirect.query_pairs().find(|(key, _)| key == "state").unwrap().1.into_owned(), binding)
}

async fn callback(app: &TestApp, state: &str, binding: Option<&str>, denial: bool) -> axum::response::Response {
    let suffix = if denial { "error=access_denied" } else { "code=code" };
    let mut request = Request::builder().uri(format!("/api/oidc/callback?state={state}&{suffix}"))
        .header(header::ACCEPT, "text/html");
    if let Some(binding) = binding { request = request.header(header::COOKIE, binding); }
    app.send(request.body(Body::empty()).unwrap()).await
}

fn no_credentials_in_browser(response: &axum::response::Response) {
    assert!(!response.headers().get_all(header::SET_COOKIE).iter().any(|v| v.to_str().unwrap().starts_with("rustdesk_api_token=")));
    assert!(response.headers().get(header::LOCATION).is_none());
}

#[tokio::test]
async fn native_browser_completes_without_logging_in_and_client_claims_auth_once() {
    let provider = MockOAuth::start();
    let app = TestApp::new(CookiePolicy::default(), provider.runtime()).await;
    let auth = issue(&app).await;
    let pending = poll(&app, &auth.code, "123456", "device-uuid").await;
    assert!(pending["error"].as_str().unwrap().contains("No authed oidc is found"));
    let (state, binding) = open(&app, &auth).await;
    assert_ne!(auth.code, state);
    let wrong = callback(&app, &state, None, false).await;
    assert_eq!(wrong.status(), StatusCode::BAD_REQUEST);
    let response = callback(&app, &state, Some(&binding), false).await;
    assert_eq!(response.status(), StatusCode::OK);
    no_credentials_in_browser(&response);
    let browser_body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    assert!(!String::from_utf8_lossy(&browser_body).contains("access_token"));
    assert!(poll(&app, &auth.code, "other-device", "device-uuid").await.get("access_token").is_none());
    assert!(poll(&app, &auth.code, "123456", "other-uuid").await.get("access_token").is_none());
    let value = poll(&app, &auth.code, "123456", "device-uuid").await;
    let result: OfficialAuthBody = serde_json::from_value(value).unwrap();
    assert_eq!(result.r#type, "access_token");
    assert_eq!(result.user.name, "alice"); assert!(result.user.info.is_object());
    let current = app.send(Request::builder().uri("/api/currentUser").header(header::AUTHORIZATION, format!("Bearer {}", result.access_token))
        .body(Body::empty()).unwrap()).await;
    assert_eq!(current.status(), StatusCode::OK);
    assert!(poll(&app, &auth.code, "123456", "device-uuid").await.get("access_token").is_none());
    let replay = callback(&app, &state, Some(&binding), false).await;
    assert_eq!(replay.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn provider_alias_is_supported_but_conflicting_provider_names_are_rejected() {
    let provider = MockOAuth::start();
    let app = TestApp::new(CookiePolicy::default(), provider.runtime()).await;
    let alias = app.send(request("POST", "/api/oidc/auth", json!({"provider":"test","id":"123456","uuid":"device-uuid"}))).await;
    assert_eq!(alias.status(), StatusCode::OK);
    let conflict = app.send(request("POST", "/api/oidc/auth", json!({"op":"test","provider":"other","id":"123456","uuid":"device-uuid"}))).await;
    assert_eq!(conflict.status(), StatusCode::BAD_REQUEST);
    let conflict = app.send(request("POST", "/api/oidc/auth", json!({"op":"oidc/test","provider":"test","id":"123456","uuid":"device-uuid"}))).await;
    assert_eq!(conflict.status(), StatusCode::OK);
}

#[tokio::test]
async fn concurrent_client_polls_issue_only_one_session() {
    let provider = MockOAuth::start();
    let app = TestApp::new(CookiePolicy::default(), provider.runtime()).await;
    let auth = issue(&app).await;
    let (state, binding) = open(&app, &auth).await;
    assert_eq!(callback(&app, &state, Some(&binding), false).await.status(), StatusCode::OK);
    let (one, two) = tokio::join!(poll(&app, &auth.code, "123456", "device-uuid"), poll(&app, &auth.code, "123456", "device-uuid"));
    assert_eq!([one, two].iter().filter(|value| value.get("access_token").is_some()).count(), 1);
}

#[tokio::test]
async fn browser_denial_is_reported_to_the_polling_client_without_a_session() {
    let provider = MockOAuth::start();
    let app = TestApp::new(CookiePolicy::default(), provider.runtime()).await;
    let auth = issue(&app).await;
    let (state, binding) = open(&app, &auth).await;
    assert_eq!(callback(&app, &state, None, true).await.status(), StatusCode::BAD_REQUEST);
    assert_eq!(poll(&app, &auth.code, "123456", "device-uuid").await["error"], "No authed oidc is found");
    let denial = callback(&app, &state, Some(&binding), true).await;
    assert_eq!(denial.status(), StatusCode::BAD_REQUEST);
    no_credentials_in_browser(&denial);
    assert_eq!(poll(&app, &auth.code, "123456", "device-uuid").await["error"], "authorization_denied");
}

#[tokio::test]
async fn expired_native_authorization_cannot_be_claimed_or_fall_back_to_browser_login() {
    let provider = MockOAuth::start();
    let now = Arc::new(AtomicU64::new(1000)); let clock = now.clone();
    let runtime = OAuthRuntime::new_with_clock(vec![provider.config()], Arc::new(move || clock.load(Ordering::SeqCst)));
    let app = TestApp::new(CookiePolicy::default(), runtime).await;
    let auth = issue(&app).await;
    now.store(1100, Ordering::SeqCst);
    let (state, binding) = open(&app, &auth).await;
    now.store(1300, Ordering::SeqCst);
    // Runtime state still lives, but native ticket expiry must stop the entire login.
    let response = callback(&app, &state, Some(&binding), false).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    no_credentials_in_browser(&response);
    assert_eq!(poll(&app, &auth.code, "123456", "device-uuid").await["error"], "native_authorization_expired");
}

#[tokio::test]
async fn password_login_also_returns_the_required_official_user_info() {
    let app = TestApp::new(CookiePolicy::default(), OAuthRuntime::new(Vec::new())).await;
    let response = app.send(request("POST", "/api/login", json!({"username":"admin","password":"admin-password"}))).await;
    assert_eq!(response.status(), StatusCode::OK);
    let result: OfficialAuthBody = serde_json::from_value(body(response).await).unwrap();
    assert!(result.user.info.is_object());
}

#[tokio::test]
async fn launch_capability_cannot_poll_and_browser_entry_cannot_restart_a_flow() {
    let provider = MockOAuth::start();
    let app = TestApp::new(CookiePolicy::default(), provider.runtime()).await;
    let auth = issue(&app).await;
    let url = reqwest::Url::parse(&auth.url).unwrap();
    let launch = url.query_pairs().find(|(key, _)| key == "launch").unwrap().1.into_owned();
    assert_eq!(poll(&app, &launch, "123456", "device-uuid").await["error"], "invalid_native_authorization");
    let _ = open(&app, &auth).await;
    let replay = app.send(Request::builder().uri(format!("{}?{}", url.path(), url.query().unwrap()))
        .body(Body::empty()).unwrap()).await;
    assert_eq!(replay.status(), StatusCode::BAD_REQUEST);
    for _ in 0..5 {
        assert_eq!(poll(&app, &auth.code, "123456", "device-uuid").await["error"], "No authed oidc is found");
    }
}

#[tokio::test]
async fn oidc_validation_failure_is_reported_to_the_native_client() {
    let provider = common::oidc::MockOidc::start();
    let app = TestApp::new(CookiePolicy::default(), OAuthRuntime::try_new(vec![provider.config()]).unwrap()).await;
    let auth = issue(&app).await;
    let (state, binding) = open(&app, &auth).await;
    provider.token("code", json!({"access_token":"external-token"}));
    let response = callback(&app, &state, Some(&binding), false).await;
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    no_credentials_in_browser(&response);
    assert_eq!(poll(&app, &auth.code, "123456", "device-uuid").await["error"], "oauth_provider_failed");
}

#[tokio::test]
async fn pending_native_authorizations_have_a_bounded_capacity_and_expire() {
    let provider = MockOAuth::start();
    let now = Arc::new(AtomicU64::new(1000)); let clock = now.clone();
    let runtime = OAuthRuntime::new_with_clock(vec![provider.config()], Arc::new(move || clock.load(Ordering::SeqCst)));
    let app = TestApp::new(CookiePolicy::default(), runtime).await;
    for _ in 0..10_000 {
        let response = app.send(request("POST", "/api/oidc/auth", json!({"op":"test","id":"123456","uuid":"device-uuid"}))).await;
        assert_eq!(response.status(), StatusCode::OK);
    }
    let rejected = app.send(request("POST", "/api/oidc/auth", json!({"op":"test","id":"123456","uuid":"device-uuid"}))).await;
    assert_eq!(body(rejected).await["error"], "too_many_native_authorizations");
    now.store(1300, Ordering::SeqCst);
    let _ = issue(&app).await;
}
