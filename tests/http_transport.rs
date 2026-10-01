mod common;
use common::TestApp;
use hbbs::{api::CookiePolicy, oauth::OAuthRuntime};

#[tokio::test]
async fn http1_and_http2_serve_api_and_static_assets_with_body_limits() {
    let app = TestApp::new(CookiePolicy::default(), OAuthRuntime::new(Vec::new())).await;
    std::fs::write(app.database_path().parent().unwrap().join("index.html"), "isolated static page").unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let router = app.router.clone();
    let task = tokio::spawn(async move { axum::serve(listener, router).await.unwrap(); });
    for (client, version) in [
        (reqwest::Client::builder().no_proxy().http1_only().build().unwrap(), reqwest::Version::HTTP_11),
        (reqwest::Client::builder().no_proxy().http2_prior_knowledge().build().unwrap(), reqwest::Version::HTTP_2),
    ] {
        let response = client.get(format!("{base}/api/version")).send().await.unwrap();
        assert_eq!(response.version(), version);
        assert!(response.status().is_success());
        assert!(response.json::<serde_json::Value>().await.unwrap().is_object());
        let response = client.get(format!("{base}/index.html")).send().await.unwrap();
        assert_eq!(response.text().await.unwrap(), "isolated static page");
        let response = client.post(format!("{base}/api/login")).json(&serde_json::json!({"username":"x","password":"x".repeat(2*1024*1024)})).send().await.unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::PAYLOAD_TOO_LARGE);
    }
    task.abort();
    let _ = task.await;
}
