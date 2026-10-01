mod common;
use common::oidc::MockOidc;
use hbbs::oauth::{OAuthDevice, OAuthRuntime};
use serde_json::json;
use serde_json::Value;
use jsonwebtoken::{Algorithm, EncodingKey, Header};

async fn authorization(runtime: &OAuthRuntime) -> (String, String) {
    let (url, state) = runtime.begin_with_device("test", "https://api.example/callback", OAuthDevice::default(), "browser").await.unwrap();
    (state, url.query_pairs().find(|(key, _)| key == "nonce").unwrap().1.into_owned())
}

#[tokio::test]
async fn configured_oidc_cannot_complete_without_an_id_token() {
    let provider = MockOidc::start();
    let runtime = OAuthRuntime::try_new(vec![provider.config()]).unwrap();
    let (_, state) = runtime.begin_with_device("test", "https://api.example/callback", OAuthDevice::default(), "browser").await.unwrap();
    provider.token("code", json!({"access_token":"external-token"}));
    assert!(runtime.complete("code", &state, "https://api.example/callback", "browser").await.is_err());
}

#[tokio::test]
async fn google_issuer_with_signed_id_token_reaches_validated_login() {
    let provider = MockOidc::start();
    let mut config = provider.config();
    config.issuer_url = "https://accounts.google.com".to_owned();
    let runtime = OAuthRuntime::try_new(vec![config]).unwrap();
    let (url, state) = runtime.begin_with_device("test", "https://api.example/callback", OAuthDevice::default(), "browser").await.unwrap();
    let nonce = url.query_pairs().find(|(key, _)| key == "nonce").unwrap().1.into_owned();
    let mut claims = provider.claims(&nonce);
    claims["iss"] = json!("https://accounts.google.com");
    provider.token("code", json!({"access_token":"external-token","id_token":MockOidc::signed(&claims)}));
    let identity = runtime.complete("code", &state, "https://api.example/callback", "browser").await.unwrap();
    assert_eq!(identity.subject, "subject-42");
    assert_eq!(identity.username, "alice");
}

#[tokio::test]
async fn signed_oidc_tokens_must_match_all_required_claims() {
    let provider = MockOidc::start();
    let runtime = OAuthRuntime::try_new(vec![provider.config()]).unwrap();
    for (field, value) in [
        ("iss", Some(json!("https://attacker.example"))), ("aud", Some(json!("other-client"))),
        ("nonce", Some(json!("other-flow"))), ("sub", Some(json!(""))),
        ("exp", Some(json!(hbbs::common::now()-1))), ("nbf", Some(json!(hbbs::common::now()+600))),
        ("azp", Some(json!("other-client"))), ("aud", Some(json!(["client", "other-client"]))),
        ("nonce", None), ("exp", None), ("sub", None), ("iss", None), ("aud", None),
    ] {
        let (state, nonce) = authorization(&runtime).await;
        let mut claims = provider.claims(&nonce);
        if let Some(value) = value { claims[field] = value; }
        else { claims.as_object_mut().unwrap().remove(field); }
        provider.token("invalid", json!({"access_token":"external-token","id_token":MockOidc::signed(&claims)}));
        assert!(runtime.complete("invalid", &state, "https://api.example/callback", "browser").await.is_err(), "invalid {field} accepted");
    }
}

#[tokio::test]
async fn invalid_signature_algorithm_and_key_identifier_are_rejected() {
    let provider = MockOidc::start();
    let runtime = OAuthRuntime::try_new(vec![provider.config()]).unwrap();
    for variant in ["signature", "algorithm", "key", "jwks-algorithm"] {
        let (state, nonce) = authorization(&runtime).await;
        let claims = provider.claims(&nonce);
        let token = match variant {
            "algorithm" => jsonwebtoken::encode(&Header::new(Algorithm::HS256), &claims, &EncodingKey::from_secret(b"test-only")).unwrap(),
            "key" => {
                let mut header = Header::new(Algorithm::RS256); header.kid = Some("unknown-key".to_owned());
                MockOidc::signed_with_header(&claims, header)
            }
            "jwks-algorithm" => {
                let mut header = Header::new(Algorithm::RS384); header.kid = Some("test-key".to_owned());
                MockOidc::signed_with_header(&claims, header)
            }
            _ => {
                let valid = MockOidc::signed(&claims);
                let (payload, signature) = valid.rsplit_once('.').unwrap();
                format!("{payload}.{}{}", if signature.starts_with('A') { "B" } else { "A" }, &signature[1..])
            }
        };
        provider.token("invalid", json!({"access_token":"external-token","id_token":token}));
        assert!(runtime.complete("invalid", &state, "https://api.example/callback", "browser").await.is_err(), "invalid {variant} accepted");
    }
}

#[tokio::test]
async fn userinfo_subject_must_be_a_matching_oidc_string() {
    let provider = MockOidc::start();
    let runtime = OAuthRuntime::try_new(vec![provider.config()]).unwrap();
    for profile in [json!({"sub":"other-subject"}), json!({"id":"subject-42"}), json!({"sub":42})] {
        let (state, nonce) = authorization(&runtime).await;
        provider.token("code", json!({"access_token":"external-token","id_token":MockOidc::signed(&provider.claims(&nonce))}));
        provider.profile(profile);
        assert!(runtime.complete("code", &state, "https://api.example/callback", "browser").await.is_err());
    }
}

#[tokio::test]
async fn multiple_audiences_with_matching_authorized_party_are_accepted() {
    let provider = MockOidc::start();
    let runtime = OAuthRuntime::try_new(vec![provider.config()]).unwrap();
    let (state, nonce) = authorization(&runtime).await;
    let mut claims = provider.claims(&nonce);
    claims["aud"] = json!(["client", "other-client"]); claims["azp"] = json!("client");
    provider.token("code", json!({"access_token":"external-token","id_token":MockOidc::signed(&claims)}));
    assert_eq!(runtime.complete("code", &state, "https://api.example/callback", "browser").await.unwrap().subject, "subject-42");
}

#[tokio::test]
async fn rejected_oidc_callback_creates_neither_account_nor_session() {
    use axum::{body::Body, http::{header, Request, StatusCode}};
    use hbbs::api::CookiePolicy;
    use common::{TestApp, cookie, request};
    let provider = MockOidc::start();
    let app = TestApp::new(CookiePolicy::default(), OAuthRuntime::try_new(vec![provider.config()]).unwrap()).await;
    for failure in ["missing-token", "wrong-subject"] {
        let begin = app.send(Request::builder().uri("/api/oidc/login?provider=test").body(Body::empty()).unwrap()).await;
        let binding = cookie(&begin);
        let url = reqwest::Url::parse(begin.headers().get(header::LOCATION).unwrap().to_str().unwrap()).unwrap();
        let state = url.query_pairs().find(|(key, _)| key == "state").unwrap().1.into_owned();
        let nonce = url.query_pairs().find(|(key, _)| key == "nonce").unwrap().1.into_owned();
        let token = if failure == "missing-token" { json!({"access_token":"external-token"}) }
            else { provider.profile(json!({"sub":"attacker"})); json!({"access_token":"external-token","id_token":MockOidc::signed(&provider.claims(&nonce))}) };
        provider.token(failure, token);
        let response = app.send(Request::builder().uri(format!("/api/oidc/callback?code={failure}&state={state}"))
            .header(header::COOKIE, binding).body(Body::empty()).unwrap()).await;
        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
        assert!(!response.headers().get_all(header::SET_COOKIE).iter().any(|value| value.to_str().unwrap().starts_with("rustdesk_api_token=")));
    }
    let login = app.send(request("POST", "/api/admin/login", json!({"username":"admin","password":"admin-password"}))).await;
    let response = app.send(Request::builder().uri("/api/admin/user/list").header(header::COOKIE, cookie(&login)).body(Body::empty()).unwrap()).await;
    let body: Value = serde_json::from_slice(&axum::body::to_bytes(response.into_body(), 4*1024*1024).await.unwrap()).unwrap();
    assert_eq!(body["data"].as_array().unwrap().len(), 1);
    let sessions = app.send(Request::builder().uri("/api/admin/session/list").header(header::COOKIE, cookie(&login)).body(Body::empty()).unwrap()).await;
    let body: Value = serde_json::from_slice(&axum::body::to_bytes(sessions.into_body(), 4*1024*1024).await.unwrap()).unwrap();
    assert_eq!(body["data"].as_array().unwrap().len(), 1);
}
