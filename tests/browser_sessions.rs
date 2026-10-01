mod common;
use common::*;
use axum::http::{header, StatusCode};
use hbbs::{api::CookiePolicy, oauth::OAuthRuntime};
use serde_json::json;

#[tokio::test]
async fn cookie_mutation_without_origin_and_csrf_is_rejected() {
    let app = TestApp::new(CookiePolicy::default(),OAuthRuntime::new(Vec::new())).await;
    let login = app.send(request("POST","/api/admin/login",json!({"username":"admin","password":"admin-password"}))).await;
    let mut logout = request("POST","/api/logout",json!({}));
    logout.headers_mut().insert(header::COOKIE,cookie(&login).parse().unwrap());
    assert_eq!(app.send_raw(logout).await.status(),StatusCode::FORBIDDEN);
}

use axum::{body::Body,http::Request};
use hbbs::api::BrowserPolicy;
use serde_json::Value;

async fn json_body(response: axum::response::Response) -> Value {
    serde_json::from_slice(&axum::body::to_bytes(response.into_body(), 4*1024*1024).await.unwrap()).unwrap()
}

async fn browser_app() -> TestApp {
    TestApp::new_with_browser(CookiePolicy::default(),OAuthRuntime::new(Vec::new()),
        Some(hbbs::oauth_admin::ProviderSecretKey::from_bytes(&[7;32]).unwrap()),
        Some(BrowserPolicy::new("https://api.example", &["https://web.example".to_owned()]).unwrap())).await
}

async fn login(app: &TestApp) -> (String,String) {
    let response = app.send(request("POST","/api/admin/login",json!({"username":"admin","password":"admin-password"}))).await;
    (cookie(&response),json_body(response).await["access_token"].as_str().unwrap().to_owned())
}

async fn csrf(app: &TestApp, cookie: &str) -> String {
    let response = app.send_raw(Request::builder().uri("/api/session/csrf").header(header::COOKIE,cookie)
        .header(header::ORIGIN,"https://web.example").body(Body::empty()).unwrap()).await;
    assert_eq!(response.status(),StatusCode::OK);
    assert_eq!(response.headers()[header::ACCESS_CONTROL_ALLOW_ORIGIN],"https://web.example");
    assert_eq!(response.headers()[header::CACHE_CONTROL],"no-store");
    json_body(response).await["csrf_token"].as_str().unwrap().to_owned()
}

fn browser_logout(cookie: &str, origin: Option<&str>, csrf: Option<&str>) -> Request<Body> {
    let mut request = request("POST","/api/logout",json!({}));
    request.headers_mut().insert(header::COOKIE,cookie.parse().unwrap());
    if let Some(origin) = origin { request.headers_mut().insert(header::ORIGIN,origin.parse().unwrap()); }
    if let Some(csrf) = csrf { request.headers_mut().insert("x-csrf-token",csrf.parse().unwrap()); }
    request
}

#[tokio::test]
async fn exact_origins_and_credentials_are_enforced_for_preflight_and_actual_requests() {
    let app = browser_app().await;
    for origin in ["https://web.example","https://api.example"] {
        let response = app.send_raw(Request::builder().method("OPTIONS").uri("/api/admin/oauth/providers")
            .header(header::ORIGIN,origin).header(header::ACCESS_CONTROL_REQUEST_METHOD,"POST")
            .header(header::ACCESS_CONTROL_REQUEST_HEADERS,"content-type,x-csrf-token")
            .body(Body::empty()).unwrap()).await;
        assert_eq!(response.status(),StatusCode::NO_CONTENT);
        assert_eq!(response.headers()[header::ACCESS_CONTROL_ALLOW_ORIGIN],origin);
        assert_eq!(response.headers()[header::ACCESS_CONTROL_ALLOW_CREDENTIALS],"true");
        let vary = response.headers().get_all(header::VARY).iter().map(|v|v.to_str().unwrap()).collect::<Vec<_>>().join(",");
        assert!(vary.contains("Origin")); assert!(vary.contains("Access-Control-Request-Headers"));
    }
    for origin in ["null","https://evil.example","https://web.example.evil","https://web.example:444","*"] {
        for method in ["OPTIONS","GET"] {
            let response = app.send_raw(Request::builder().method(method).uri("/api/login-options")
                .header(header::ORIGIN,origin).header(header::ACCESS_CONTROL_REQUEST_METHOD,"POST")
                .body(Body::empty()).unwrap()).await;
            assert_eq!(response.status(),StatusCode::FORBIDDEN);
            assert!(response.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN).is_none());
        }
    }
    let preflight = app.send_raw(Request::builder().method("OPTIONS").uri("/api/logout")
        .header(header::ORIGIN,"https://web.example").header(header::ACCESS_CONTROL_REQUEST_METHOD,"POST")
        .header(header::ACCESS_CONTROL_REQUEST_HEADERS,"x-unapproved").body(Body::empty()).unwrap()).await;
    assert_eq!(preflight.status(),StatusCode::FORBIDDEN);
    let native = app.send_raw(Request::builder().uri("/api/login-options").body(Body::empty()).unwrap()).await;
    assert_eq!(native.status(),StatusCode::OK);
    assert!(native.headers().get_all(header::VARY).iter().any(|v|v == "Origin"));
    assert!(native.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN).is_none());
}

#[tokio::test]
async fn csrf_is_session_bound_and_rejections_do_not_revoke_a_valid_session() {
    let app = browser_app().await;
    let (one,_) = login(&app).await; let (two,_) = login(&app).await;
    let token = csrf(&app,&one).await; let other = csrf(&app,&two).await;
    assert_ne!(token,other);
    for (origin, supplied) in [
        (Some("https://web.example"),None),
        (Some("https://web.example"),Some("invalid")),
        (Some("https://web.example"),Some(other.as_str())),
        (Some("https://evil.example"),Some(token.as_str())),
        (None,Some(token.as_str())),
    ] {
        assert_eq!(app.send_raw(browser_logout(&one,origin,supplied)).await.status(),StatusCode::FORBIDDEN);
        assert_eq!(csrf(&app,&one).await,token);
    }
    let response = app.send_raw(browser_logout(&one,Some("https://web.example"),Some(&token))).await;
    assert_eq!(response.status(),StatusCode::OK);
    assert!(response.headers()[header::SET_COOKIE].to_str().unwrap().contains("Max-Age=0"));
    let current = app.send_raw(Request::builder().uri("/api/session/csrf").header(header::COOKIE,one).body(Body::empty()).unwrap()).await;
    assert_eq!(current.status(),StatusCode::UNAUTHORIZED);
    assert!(!csrf(&app,&two).await.is_empty());
}

#[tokio::test]
async fn native_bearer_needs_no_browser_token_and_invalid_bearer_cannot_fall_back_to_cookie() {
    let app = browser_app().await;
    let (cookie,token) = login(&app).await;
    let expired = {
        let payload = token.split('.').nth(1).unwrap();
        let mut claims: Value = serde_json::from_slice(&base64::decode_config(payload,base64::URL_SAFE_NO_PAD).unwrap()).unwrap();
        claims["exp"] = json!(1);
        jsonwebtoken::encode(&jsonwebtoken::Header::default(),&claims,&jsonwebtoken::EncodingKey::from_secret(b"01234567890123456789012345678901")).unwrap()
    };
    for bearer in ["Bearer invalid".to_owned(),"Bearer ".to_owned(),"Basic invalid".to_owned(),format!("Bearer {expired}")] {
        let response = app.send_raw(Request::builder().uri("/api/currentUser").header(header::COOKIE,&cookie)
            .header(header::AUTHORIZATION,bearer).body(Body::empty()).unwrap()).await;
        assert_eq!(response.status(),StatusCode::UNAUTHORIZED);
    }
    let response = app.send_raw(Request::builder().method("POST").uri("/api/logout")
        .header(header::AUTHORIZATION,format!("Bearer {token}")).body(Body::empty()).unwrap()).await;
    assert_eq!(response.status(),StatusCode::OK);
}

#[test]
fn cross_site_cookies_are_explicit_and_insecure_or_wildcard_configuration_is_rejected() {
    let policy = CookiePolicy::default().cross_site().unwrap();
    let value = policy.cookie("rustdesk_api_token","test",60);
    for attribute in ["; SameSite=None","; Secure","; HttpOnly"] { assert!(value.contains(attribute)); }
    let local = CookiePolicy::local_http("127.0.0.1".parse().unwrap(),"http://localhost").unwrap();
    assert!(local.cross_site().is_err());
    for origin in ["*","https://*.example","https://web.example/","https://web.example/path","https://user:pass@web.example"] {
        assert!(BrowserPolicy::new("https://api.example",&[origin.to_owned()]).is_err());
    }
    assert!(BrowserPolicy::new("https://api.example",&[]).unwrap().allows("https://api.example"));
}
