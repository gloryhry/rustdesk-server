mod common;
use axum::{body::Body, http::{header, Request, StatusCode}, response::Response};
use common::*;
use hbbs::api::CookiePolicy;
use hbbs::oauth::{OAuthDevice, OAuthRuntime};
use std::sync::{Arc, atomic::{AtomicU64, Ordering}};

async fn begin(app: &TestApp) -> (String, Option<String>) {
    let response = app.send(Request::builder().uri("/api/oidc/login?provider=test")
        .body(Body::empty()).unwrap()).await;
    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
    let url = reqwest::Url::parse(response.headers().get(header::LOCATION).unwrap().to_str().unwrap()).unwrap();
    let state = url.query_pairs().find(|(key, _)| key == "state").unwrap().1.into_owned();
    let cookie = response.headers().get(header::SET_COOKIE).map(|v| v.to_str().unwrap().split(';').next().unwrap().to_owned());
    (state, cookie)
}

async fn callback(app: &TestApp, state: &str, cookie: Option<&str>) -> Response {
    let mut request = Request::builder().uri(format!("/api/oidc/callback?code=code&state={state}"))
        .header(header::ACCEPT, "text/html");
    if let Some(cookie) = cookie { request = request.header(header::COOKIE, cookie); }
    app.send(request.body(Body::empty()).unwrap()).await
}

fn has_auth_cookie(response: &Response) -> bool {
    response.headers().get_all(header::SET_COOKIE).iter().any(|v|
        v.to_str().unwrap().starts_with("rustdesk_api_token="))
}

#[tokio::test]
async fn forwarded_callback_without_initiating_browser_cannot_log_in() {
    let provider = MockOAuth::start();
    let app = TestApp::new(CookiePolicy::default(), provider.runtime()).await;
    let (state, _) = begin(&app).await;
    let rejected = callback(&app, &state, None).await;
    assert_eq!(rejected.status(), StatusCode::BAD_REQUEST);
    assert!(!has_auth_cookie(&rejected));
}

#[tokio::test]
async fn wrong_browser_cannot_consume_state_and_multiple_tabs_can_complete() {
    let provider = MockOAuth::start();
    let app = TestApp::new(CookiePolicy::default(), provider.runtime()).await;
    let (first, first_cookie) = begin(&app).await;
    let (second, second_cookie) = begin(&app).await;
    let first_cookie = first_cookie.expect("browser binding must be issued");
    let second_cookie = second_cookie.unwrap();
    let wrong = callback(&app, &first, Some(&second_cookie)).await;
    assert_eq!(wrong.status(), StatusCode::BAD_REQUEST);
    assert!(!has_auth_cookie(&wrong));
    let cookies = format!("{first_cookie}; {second_cookie}");
    for state in [&first, &second] {
        let result = callback(&app, state, Some(&cookies)).await;
        assert_eq!(result.status(), StatusCode::TEMPORARY_REDIRECT);
        assert!(has_auth_cookie(&result));
        let repeated = callback(&app, state, Some(&cookies)).await;
        assert_eq!(repeated.status(), StatusCode::BAD_REQUEST);
        assert!(!has_auth_cookie(&repeated));
    }
}

#[tokio::test]
async fn concurrent_callbacks_create_only_one_browser_session() {
    let provider = MockOAuth::start();
    let app = TestApp::new(CookiePolicy::default(), provider.runtime()).await;
    let (state, cookie) = begin(&app).await;
    let cookie = cookie.unwrap();
    let (one, two) = tokio::join!(callback(&app, &state, Some(&cookie)), callback(&app, &state, Some(&cookie)));
    let statuses = [one.status(), two.status()];
    assert_eq!(statuses.iter().filter(|&&v| v == StatusCode::TEMPORARY_REDIRECT).count(), 1);
    assert_eq!(statuses.iter().filter(|&&v| v == StatusCode::BAD_REQUEST).count(), 1);
    assert_eq!([has_auth_cookie(&one), has_auth_cookie(&two)].iter().filter(|&&v| v).count(), 1);
}

#[tokio::test]
async fn expired_bound_callback_cannot_create_a_session() {
    let provider = MockOAuth::start();
    let now = Arc::new(AtomicU64::new(1000));
    let clock = now.clone();
    let runtime = OAuthRuntime::new_with_clock(vec![provider.config()], Arc::new(move || clock.load(Ordering::SeqCst)));
    let app = TestApp::new(CookiePolicy::default(), runtime).await;
    let (state, cookie) = begin(&app).await;
    now.store(1300, Ordering::SeqCst);
    let response = callback(&app, &state, cookie.as_deref()).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(!has_auth_cookie(&response));
}

#[tokio::test]
async fn incorrect_redirect_uri_does_not_consume_the_bound_authorization() {
    let provider = MockOAuth::start();
    let runtime = provider.runtime();
    let (_, state) = runtime.begin_with_device("test", "https://api.example/callback", OAuthDevice::default(), "binding").await.unwrap();
    assert!(runtime.complete("code", &state, "https://attacker.example/callback", "binding").await.is_err());
    let identity = runtime.complete("code", &state, "https://api.example/callback", "binding").await.unwrap();
    assert_eq!(identity.subject, "42");
}
