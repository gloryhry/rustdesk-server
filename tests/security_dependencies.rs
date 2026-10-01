mod common;
use common::*;
use axum::http::StatusCode;
use hbbs::{api::CookiePolicy,oauth::OAuthRuntime};
use serde_json::json;

#[tokio::test]
async fn oversized_json_is_rejected_before_login_and_small_requests_still_work() {
    let app = TestApp::new(CookiePolicy::default(),OAuthRuntime::new(Vec::new())).await;
    let oversized = json!({"username":"isolated-admin","password":"x".repeat(2*1024*1024)});
    let response = app.send_raw(request("POST","/api/login",oversized)).await;
    assert_eq!(response.status(),StatusCode::PAYLOAD_TOO_LARGE);
    let response = app.send_raw(request("POST","/api/login",json!({"username":"missing-user","password":"invalid-password"}))).await;
    assert_eq!(response.status(),StatusCode::UNAUTHORIZED);
    let response = app.send_raw(request("POST","/api/sysinfo",json!({"id":"123456","uuid":"test","large":"x".repeat(64*1024)}))).await;
    assert_eq!(response.status(),StatusCode::PAYLOAD_TOO_LARGE);
}
