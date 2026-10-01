mod common;
use common::*;
use axum::http::StatusCode;
use hbbs::{api::CookiePolicy,oauth::OAuthRuntime};
use serde_json::json;

#[tokio::test]
async fn readiness_checks_static_assets_and_database_while_liveness_stays_independent() {
    let app = TestApp::new(CookiePolicy::default(),OAuthRuntime::new(Vec::new())).await;
    let root = app.database_path().parent().unwrap().to_owned();
    assert_eq!(app.send_raw(request("GET","/health/ready",json!({}))).await.status(),StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(app.send_raw(request("GET","/health/live",json!({}))).await.status(),StatusCode::OK);
    std::fs::create_dir(root.join("assets")).unwrap(); std::fs::write(root.join("index.html"),"<script type=\"module\" src=\"/assets/main.js\"></script>").unwrap();
    assert_eq!(app.send_raw(request("GET","/health/ready",json!({}))).await.status(),StatusCode::SERVICE_UNAVAILABLE);
    std::fs::write(root.join("assets/main.js"),"console.log('test-only asset')").unwrap();
    assert_eq!(app.send_raw(request("GET","/health/ready",json!({}))).await.status(),StatusCode::OK);
    let pool = sqlx::SqlitePool::connect(app.database_path().to_str().unwrap()).await.unwrap();
    sqlx::query("drop table api_user").execute(&pool).await.unwrap();
    assert_eq!(app.send_raw(request("GET","/health/ready",json!({}))).await.status(),StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(app.send_raw(request("GET","/health/live",json!({}))).await.status(),StatusCode::OK); pool.close().await;
}
