mod common;
use common::oidc::MockOidc;
use hbbs::oauth::{OAuthDevice, OAuthRuntime};
use serde_json::json;

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
