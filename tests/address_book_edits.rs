mod common;
use common::*;
use axum::http::{header,StatusCode};
use hbbs::{api::CookiePolicy,oauth::OAuthRuntime};
use serde_json::{json,Value};

async fn value(response: axum::response::Response) -> Value { serde_json::from_slice(&hyper::body::to_bytes(response.into_body()).await.unwrap()).unwrap() }
async fn auth(app: &TestApp, method: &str, path: &str, body: Value, token: &str) -> axum::response::Response {
    let mut request = request(method,path,body); request.headers_mut().insert(header::AUTHORIZATION,format!("Bearer {token}").parse().unwrap()); app.send(request).await
}
async fn fixture() -> (TestApp,String) {
    let app = TestApp::new(CookiePolicy::default(),OAuthRuntime::new(Vec::new())).await;
    let token = value(app.send(request("POST","/api/login",json!({"username":"admin","password":"admin-password"}))).await).await["access_token"].as_str().unwrap().to_owned();
    let data = json!({"peers":[{"id":"123456","alias":"Original","username":"os-user","hostname":"Laptop","platform":"Linux","tags":["old"],"forceAlwaysRelay":true,"hash":"saved-hash","password":"saved-password","rdpPort":"3390","rdpUsername":"rdp-user","note":"saved note","extension":{"nested":[1,2]}}],"tags":["old"]});
    assert_eq!(auth(&app,"POST","/api/ab",json!({"data":data.to_string()}),&token).await.status(),StatusCode::OK);
    (app,token)
}
async fn book(app: &TestApp,token: &str) -> Value {
    serde_json::from_str(value(auth(app,"GET","/api/ab",json!({}),token).await).await["data"].as_str().unwrap()).unwrap()
}

#[tokio::test]
async fn partial_edits_preserve_saved_connection_and_unknown_fields() {
    let (app,token) = fixture().await;
    for patch in [json!({"peer_id":"123456","alias":"New alias"}),json!({"peer_id":"123456","tags":["new"]}),json!({"peer_id":"123456","note":"changed note"})] {
        let response = auth(&app,"POST","/api/ab/peer",patch,&token).await; assert_eq!(response.status(),StatusCode::OK);
        let peer = &book(&app,&token).await["peers"][0];
        assert_eq!(peer["hash"],"saved-hash"); assert_eq!(peer["password"],"saved-password");
        assert_eq!(peer["rdpPort"],"3390"); assert_eq!(peer["rdpUsername"],"rdp-user");
        assert_eq!(peer["extension"],json!({"nested":[1,2]})); assert_eq!(peer["username"],"os-user");
        assert_eq!(peer["hostname"],"Laptop"); assert_eq!(peer["platform"],"Linux"); assert_eq!(peer["forceAlwaysRelay"],true);
    }
    let peer = &book(&app,&token).await["peers"][0]; assert_eq!(peer["alias"],"New alias"); assert_eq!(peer["tags"],json!(["new"])); assert_eq!(peer["note"],"changed note");
}

#[tokio::test]
async fn explicit_empty_fields_clear_only_requested_values_and_reads_preserve_extensions() {
    let (app,token) = fixture().await;
    let response = value(auth(&app,"POST","/api/ab/peer",json!({"peer_id":"123456","alias":"","tags":[],"note":"","password":""}),&token).await).await;
    for peer in [response["data"].clone(),book(&app,&token).await["peers"][0].clone(),value(auth(&app,"GET","/api/ab/peers",json!({}),&token).await).await["data"][0].clone()] {
        assert_eq!(peer["alias"],""); assert_eq!(peer["tags"],json!([])); assert_eq!(peer["password"],""); assert_eq!(peer["note"],"");
        assert_eq!(peer["hash"],"saved-hash"); assert_eq!(peer["extension"],json!({"nested":[1,2]}));
    }
}

#[tokio::test]
async fn failed_snapshot_write_rolls_back_structured_entry_update() {
    let (app,token) = fixture().await;
    assert_eq!(auth(&app,"POST","/api/ab/peer",json!({"peer_id":"123456","alias":"Before failure"}),&token).await.status(),StatusCode::OK);
    let before = book(&app,&token).await;
    let pool = sqlx::SqlitePool::connect(app.database_path().to_str().unwrap()).await.unwrap();
    sqlx::query("create trigger reject_snapshot before update on api_address_book_snapshot begin select raise(abort,'test-only failure'); end").execute(&pool).await.unwrap();
    assert_eq!(auth(&app,"POST","/api/ab/peer",json!({"peer_id":"123456","alias":"Must roll back"}),&token).await.status(),StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(book(&app,&token).await,before);
    assert_eq!(auth(&app,"POST","/api/ab/peer",json!({"peer_id":"new-peer","alias":"Must not insert"}),&token).await.status(),StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(sqlx::query_scalar::<_,i64>("select count(*) from api_address_book_entry").fetch_one(&pool).await.unwrap(),1);
    assert_eq!(sqlx::query_scalar::<_,String>("select alias from api_address_book_entry where peer_id='123456'").fetch_one(&pool).await.unwrap(),"Before failure");
    pool.close().await;
}

#[tokio::test]
async fn identical_imported_peer_ids_in_different_accounts_have_independent_edits() {
    let (app,token) = fixture().await;
    assert_eq!(app.send(request("POST","/api/register",json!({"username":"other","password":"other-password"}))).await.status(),StatusCode::CREATED);
    let other = value(app.send(request("POST","/api/login",json!({"username":"other","password":"other-password"}))).await).await["access_token"].as_str().unwrap().to_owned();
    let imported = book(&app,&token).await;
    assert_eq!(auth(&app,"POST","/api/ab",json!({"data":imported.to_string()}),&other).await.status(),StatusCode::OK);
    let first = value(auth(&app,"POST","/api/ab/peer",json!({"peer_id":"123456","alias":"First"}),&token).await).await;
    let second = value(auth(&app,"POST","/api/ab/peer",json!({"peer_id":"123456","alias":"Second"}),&other).await).await;
    assert!(first["data"]["id"].is_string()); assert!(second["data"]["id"].is_string()); assert_ne!(first["data"]["id"],second["data"]["id"]);
    assert_eq!(book(&app,&token).await["peers"][0]["alias"],"First"); assert_eq!(book(&app,&other).await["peers"][0]["alias"],"Second");
}
