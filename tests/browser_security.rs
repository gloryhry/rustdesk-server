mod common;
use axum::{body::Body, http::{header, Request, StatusCode}};
use common::*;
use hbbs::{api::CookiePolicy, oauth::OAuthRuntime};
use serde_json::json;

#[tokio::test]
async fn admin_login_and_logout_use_secure_host_cookies_despite_forwarded_headers() {
    let app = TestApp::new(CookiePolicy::default(), OAuthRuntime::new(Vec::new())).await;
    let mut login = request("POST", "/api/admin/login", json!({"username":"admin","password":"admin-password"}));
    login.headers_mut().insert("x-forwarded-proto", "http".parse().unwrap());
    let response = app.send(login).await;
    assert_secure_cookie(&response, false);
    let logout = app.send(Request::builder().method("POST").uri("/api/admin/logout")
        .header(header::COOKIE, cookie(&response)).body(Body::empty()).unwrap()).await;
    assert_secure_cookie(&logout, true);
}

#[tokio::test]
async fn oauth_callback_uses_the_same_secure_cookie_policy() {
    let provider = MockOAuth::start();
    let app = TestApp::new(CookiePolicy::default(), provider.runtime()).await;
    let begin = app.send(Request::builder().uri("/api/oidc/login?provider=test")
        .body(Body::empty()).unwrap()).await;
    let url = reqwest::Url::parse(begin.headers().get(header::LOCATION).unwrap().to_str().unwrap()).unwrap();
    let state = url.query_pairs().find(|(key, _)| key == "state").unwrap().1.into_owned();
    let response = app.send(Request::builder().uri(format!("/api/oidc/callback?code=code&state={state}"))
        .body(Body::empty()).unwrap()).await;
    assert_secure_cookie(&response, false);
}

#[tokio::test]
async fn explicit_loopback_http_mode_uses_matching_login_and_logout_cookies() {
    let policy = CookiePolicy::local_http("127.0.0.1".parse().unwrap(), "http://localhost:21114").unwrap();
    let app = TestApp::new(policy, OAuthRuntime::new(Vec::new())).await;
    let response = app.send(request("POST", "/api/login", json!({"username":"admin","password":"admin-password"}))).await;
    assert_eq!(response.status(), StatusCode::OK);
    let attributes = response.headers().get(header::SET_COOKIE).unwrap().to_str().unwrap().split_once(';').unwrap().1.to_owned();
    assert!(!attributes.contains("Secure"));
    let logout = app.send(Request::builder().method("POST").uri("/api/logout")
        .header(header::COOKIE, cookie(&response)).body(Body::empty()).unwrap()).await;
    assert_eq!(logout.status(), StatusCode::OK);
    let deleted = logout.headers().get(header::SET_COOKIE).unwrap().to_str().unwrap();
    assert_eq!(deleted.split_once(';').unwrap().1, attributes.replace("Max-Age=3600", "Max-Age=0"));
}

#[test]
fn insecure_cookie_exception_rejects_nonlocal_deployments() {
    for (bind, url) in [
        ("0.0.0.0", "http://localhost"), ("127.0.0.1", "http://api.example"),
        ("127.0.0.1", "https://localhost"), ("127.0.0.1", "http://user:password@localhost"),
    ] {
        assert!(CookiePolicy::local_http(bind.parse().unwrap(), url).is_err());
    }
    assert!(CookiePolicy::local_http("::1".parse().unwrap(), "http://[::1]:21114").is_ok());
}
