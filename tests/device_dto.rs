mod common;
use common::*;
use axum::http::{header,StatusCode};
use hbbs::{api::CookiePolicy,oauth::OAuthRuntime,database::Database,device_registry::key_fingerprint};
use serde::{Deserialize};
use serde_json::{json,Value};

async fn value(response: axum::response::Response) -> Value {
    serde_json::from_slice(&hyper::body::to_bytes(response.into_body()).await.unwrap()).unwrap()
}
async fn auth(app: &TestApp, method: &str, path: &str, body: Value, token: &str) -> axum::response::Response {
    let mut request = request(method,path,body); request.headers_mut().insert(header::AUTHORIZATION,format!("Bearer {token}").parse().unwrap());
    app.send(request).await
}
async fn fixture() -> (TestApp,String,String,String,String) {
    let app = TestApp::new(CookiePolicy::default(),OAuthRuntime::new(Vec::new())).await;
    let db = Database::new(app.database_path().to_str().unwrap()).await.unwrap();
    db.insert_peer("123456",b"registered-device",&[3;32],"{}").await.unwrap();
    let admin = value(app.send(request("POST","/api/admin/login",json!({"username":"admin","password":"admin-password"}))).await).await["access_token"].as_str().unwrap().to_owned();
    let user = value(app.send(request("POST","/api/register",json!({"username":"owner","password":"owner-password"}))).await).await["id"].as_str().unwrap().to_owned();
    let token = value(app.send(request("POST","/api/login",json!({"username":"owner","password":"owner-password"}))).await).await["access_token"].as_str().unwrap().to_owned();
    let bound = auth(&app,"POST","/api/admin/device/bind",json!({"peer_id":"123456","user_id":user,"pk_fingerprint":key_fingerprint(&[3;32])}),&admin).await;
    assert_eq!(bound.status(),StatusCode::OK);
    let internal = value(bound).await["id"].as_str().unwrap().to_owned();
    (app,token,admin,internal,user)
}

#[tokio::test]
async fn official_peers_return_rustdesk_id_instead_of_internal_primary_key() {
    let (app,token,_,internal,_) = fixture().await;
    assert_ne!(internal,"123456");
    let response = value(auth(&app,"GET","/api/peers",json!({}),&token).await).await;
    assert_eq!(response["data"][0]["id"],"123456");
}

/// RustDesk 1.4.9 flutter/lib/common/hbbs/hbbs.dart::PeerPayload fields.
#[derive(Deserialize)]
struct OfficialPeerPayload {
    id: String,
    info: std::collections::HashMap<String,Value>,
    status: Option<i64>,
    user: String,
    user_name: String,
    device_group_name: Option<String>,
    note: String,
}

#[tokio::test]
async fn official_peer_payload_parses_information_object_and_preserves_management_status() {
    let (app,token,_,internal,user) = fixture().await;
    let report = app.send(request("POST","/api/sysinfo",json!({"id":"123456","uuid":base64::encode(b"registered-device"),"hostname":"Native laptop","username":"os-user","os":"Linux / x86_64","vendor":{"kept":true}}))).await;
    assert_eq!(report.status(),StatusCode::OK);
    let pool = sqlx::SqlitePool::connect(app.database_path().to_str().unwrap()).await.unwrap();
    sqlx::query("update api_device set status=0 where id=?").bind(&internal).execute(&pool).await.unwrap();
    let response = value(auth(&app,"GET","/api/peers",json!({}),&token).await).await;
    let peer: OfficialPeerPayload = serde_json::from_value(response["data"][0].clone()).unwrap();
    assert_eq!(peer.id,"123456"); assert_eq!(peer.info["device_name"],"Native laptop");
    assert_eq!(peer.info["username"],"os-user"); assert_eq!(peer.info["os"],"Linux / x86_64");
    assert_eq!(peer.info["vendor"]["kept"],true); assert_eq!(peer.status,Some(0));
    assert_eq!(peer.user,user); assert_eq!(peer.user_name,"owner"); assert_eq!(peer.note,""); assert!(peer.device_group_name.is_none());
    assert_eq!(response["data"][0]["online"],false);
    assert!(response["data"][0].get("uuid").is_none()); assert!(response["data"][0].get("user_id").is_none());
    pool.close().await;
}

#[tokio::test]
async fn management_primary_keys_and_existing_group_memberships_remain_stable() {
    let (app,token,admin,internal,_) = fixture().await;
    let group = value(auth(&app,"POST","/api/device-groups",json!({"name":"Owned group"}),&token).await).await;
    let group_id = group["id"].as_str().unwrap();
    assert_eq!(auth(&app,"POST","/api/device-groups/members",json!({"group_id":group_id,"device_id":internal}),&token).await.status(),StatusCode::OK);
    let peers = value(auth(&app,"GET","/api/peers",json!({}),&token).await).await;
    assert_eq!(peers["data"][0]["device_group_name"],"Owned group");
    let groups = value(auth(&app,"GET","/api/device-groups",json!({}),&token).await).await;
    assert_eq!(groups["memberships"][0]["device_id"],internal);
    let managed = value(auth(&app,"GET","/api/devices",json!({}),&token).await).await;
    assert_eq!(managed["data"][0]["id"],internal); assert_eq!(managed["data"][0]["peer_id"],"123456");
    assert_eq!(auth(&app,"POST","/api/admin/device/delete",json!({"id":"123456"}),&admin).await.status(),StatusCode::NOT_FOUND);
    assert_eq!(auth(&app,"POST","/api/admin/device/delete",json!({"id":internal}),&admin).await.status(),StatusCode::OK);
    assert_eq!(value(auth(&app,"GET","/api/peers",json!({}),&token).await).await["data"],json!([]));
    assert_eq!(value(auth(&app,"GET","/api/device-groups",json!({}),&token).await).await["memberships"],json!([]));
}

#[tokio::test]
async fn other_users_and_pending_links_do_not_leak_into_official_peer_lists() {
    let (app,token,admin,_,_) = fixture().await;
    let db = Database::new(app.database_path().to_str().unwrap()).await.unwrap();
    db.insert_peer("654321",b"other-device",&[4;32],"{}").await.unwrap();
    let other = value(app.send(request("POST","/api/register",json!({"username":"other","password":"other-password"}))).await).await["id"].as_str().unwrap().to_owned();
    assert_eq!(auth(&app,"POST","/api/admin/device/bind",json!({"peer_id":"654321","user_id":other,"pk_fingerprint":key_fingerprint(&[4;32])}),&admin).await.status(),StatusCode::OK);
    db.upsert_api_device("unverified-private",&other,"legacy","Private","Linux","client","{\"secret\":true}").await.unwrap();
    let own = value(auth(&app,"GET","/api/peers",json!({}),&token).await).await;
    assert_eq!(own["data"].as_array().unwrap().len(),1); assert_eq!(own["data"][0]["id"],"123456");
    assert!(!own.to_string().contains("unverified-private")); assert!(!own.to_string().contains("654321"));
    let all = value(auth(&app,"GET","/api/peers",json!({}),&admin).await).await;
    assert_eq!(all["data"].as_array().unwrap().len(),2); assert!(!all.to_string().contains("unverified-private"));
}

#[tokio::test]
async fn damaged_historical_info_has_explicit_error_and_safe_object_fallback_without_rewriting_data() {
    let (app,token,_,internal,_) = fixture().await;
    let pool = sqlx::SqlitePool::connect(app.database_path().to_str().unwrap()).await.unwrap();
    for raw in ["broken-json","[]","null"] {
        sqlx::query("update api_device set info=?,name='Old laptop',os='Linux' where id=?").bind(raw).bind(&internal).execute(&pool).await.unwrap();
        let response = value(auth(&app,"GET","/api/peers",json!({}),&token).await).await;
        let parsed: OfficialPeerPayload = serde_json::from_value(response["data"][0].clone()).unwrap();
        assert_eq!(parsed.info["device_name"],"Old laptop"); assert_eq!(parsed.info["os"],"Linux");
        assert_eq!(response["data"][0]["info_error"],"invalid_legacy_device_info");
        assert_eq!(sqlx::query_scalar::<_,String>("select info from api_device where id=?").bind(&internal).fetch_one(&pool).await.unwrap(),raw);
    }
    sqlx::query("update api_device set info='{\"username\":\"legacy-os-user\",\"unknown\":{\"preserved\":true}}' where id=?").bind(internal).execute(&pool).await.unwrap();
    let response = value(auth(&app,"GET","/api/peers",json!({}),&token).await).await;
    assert_eq!(response["data"][0]["info"]["unknown"]["preserved"],true);
    assert_eq!(response["data"][0]["info"]["username"],"legacy-os-user");
    assert!(response["data"][0].get("info_error").is_none()); pool.close().await;
}
