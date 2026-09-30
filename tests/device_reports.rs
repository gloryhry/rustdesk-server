mod common;
use common::*;
use hbbs::{api::CookiePolicy,oauth::OAuthRuntime};
use serde_json::json;

#[tokio::test]
async fn unknown_official_device_report_cannot_claim_to_have_been_saved() {
    let app = TestApp::new(CookiePolicy::default(),OAuthRuntime::new(Vec::new())).await;
    let response = app.send(request("POST","/api/sysinfo",json!({"id":"123456","uuid":"ZGV2aWNlLXV1aWQ=","username":"alice","hostname":"Laptop","os":"Linux"}))).await;
    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    assert_eq!(&body[..],b"ID_NOT_FOUND");
}

use axum::http::{header,StatusCode};
use hbbs::{database::Database,device_registry::{RegistrationObservation,RegistrationWriter,key_fingerprint,now_ms}};
use serde_json::Value;

async fn body(response: axum::response::Response) -> String {
    String::from_utf8(hyper::body::to_bytes(response.into_body()).await.unwrap().to_vec()).unwrap()
}
async fn value(response: axum::response::Response) -> Value { serde_json::from_str(&body(response).await).unwrap() }
async fn db(app: &TestApp) -> Database { Database::new(app.database_path().to_str().unwrap()).await.unwrap() }
async fn pool(app: &TestApp) -> sqlx::SqlitePool { sqlx::SqlitePool::connect(app.database_path().to_str().unwrap()).await.unwrap() }
async fn seed(app: &TestApp) -> Vec<u8> { db(app).await.insert_peer("123456",b"device-uuid",&[3;32],"{\"ip\":\"127.0.0.1\"}").await.unwrap() }
fn report() -> Value { json!({"id":"123456","uuid":base64::encode(b"device-uuid"),"username":"alice","hostname":"Laptop","os":"Linux","version":"1.4.9","cpu":"test","extensions":{"vendor":"kept"}}) }
async fn admin(app: &TestApp) -> String {
    value(app.send(request("POST","/api/admin/login",json!({"username":"admin","password":"admin-password"}))).await).await["access_token"].as_str().unwrap().to_owned()
}
async fn owner(app: &TestApp) -> (String,String) {
    let registered = value(app.send(request("POST","/api/register",json!({"username":"owner","password":"owner-password"}))).await).await;
    let token = value(app.send(request("POST","/api/login",json!({"username":"owner","password":"owner-password"}))).await).await;
    (registered["id"].as_str().unwrap().to_owned(),token["access_token"].as_str().unwrap().to_owned())
}
async fn authenticated(app: &TestApp, method: &str, path: &str, payload: Value, token: &str) -> axum::response::Response {
    let mut request = request(method,path,payload); request.headers_mut().insert(header::AUTHORIZATION,format!("Bearer {token}").parse().unwrap());
    app.send(request).await
}
async fn bind(app: &TestApp, owner: &str, token: &str) -> String {
    let response = authenticated(app,"POST","/api/admin/device/bind",json!({"peer_id":"123456","user_id":owner,"pk_fingerprint":key_fingerprint(&[3;32])}),token).await;
    assert_eq!(response.status(),StatusCode::OK);
    value(response).await["id"].as_str().unwrap().to_owned()
}

#[tokio::test]
async fn official_unsigned_sysinfo_and_heartbeat_persist_without_ownership_or_online_claims() {
    let app = TestApp::new(CookiePolicy::default(),OAuthRuntime::new(Vec::new())).await;
    seed(&app).await;
    let heartbeat = app.send(request("POST","/api/heartbeat",json!({"id":"123456","uuid":base64::encode(b"device-uuid"),"ver":1004009,"conns":[123],"modified_at":0}))).await;
    assert_eq!(heartbeat.status(),StatusCode::OK); assert_eq!(value(heartbeat).await["sysinfo"],true);
    assert_eq!(body(app.send(request("POST","/api/sysinfo",report())).await).await,"SYSINFO_UPDATED");
    let pool = pool(&app).await;
    let stored: String = sqlx::query_scalar("select sysinfo from api_device_report").fetch_one(&pool).await.unwrap();
    assert_eq!(serde_json::from_str::<Value>(&stored).unwrap(),report());
    assert_eq!(sqlx::query_scalar::<_,i64>("select count(*) from api_device").fetch_one(&pool).await.unwrap(),0);
    assert_eq!(sqlx::query_scalar::<_,i64>("select count(*) from peer_registration").fetch_one(&pool).await.unwrap(),0);
    sqlx::query("update api_device_report set heartbeat_at_ms=0").execute(&pool).await.unwrap();
    let heartbeat = app.send(request("POST","/api/heartbeat",json!({"id":"123456","uuid":base64::encode(b"device-uuid"),"ver":1004009}))).await;
    assert_eq!(value(heartbeat).await,json!({})); pool.close().await;
}

#[tokio::test]
async fn mismatched_uuid_malformed_payload_rate_limit_and_failed_storage_never_fake_success() {
    let app = TestApp::new(CookiePolicy::default(),OAuthRuntime::new(Vec::new())).await;
    seed(&app).await;
    let mut mismatch = report(); mismatch["uuid"] = json!(base64::encode(b"other-device"));
    assert_eq!(body(app.send(request("POST","/api/sysinfo",mismatch)).await).await,"ID_NOT_FOUND");
    for invalid in [json!([]),json!({}),json!({"id":"123456","uuid":"!not-base64!"}),json!({"id":1,"uuid":"ZGV2aWNlLXV1aWQ="})] {
        assert_eq!(app.send(request("POST","/api/sysinfo",invalid)).await.status(),StatusCode::BAD_REQUEST);
    }
    assert_eq!(body(app.send(request("POST","/api/sysinfo",report())).await).await,"SYSINFO_UPDATED");
    let mut changed = report(); changed["hostname"] = json!("rate-limited");
    assert_eq!(app.send(request("POST","/api/sysinfo",changed)).await.status(),StatusCode::TOO_MANY_REQUESTS);
    let pool = pool(&app).await;
    let stored: String = sqlx::query_scalar("select sysinfo from api_device_report").fetch_one(&pool).await.unwrap();
    assert_eq!(serde_json::from_str::<Value>(&stored).unwrap()["hostname"],"Laptop");
    sqlx::query("drop table api_device_report").execute(&pool).await.unwrap();
    let failure = app.send(request("POST","/api/sysinfo",report())).await;
    assert_eq!(failure.status(),StatusCode::INTERNAL_SERVER_ERROR);
    assert!(!body(failure).await.contains("SYSINFO_UPDATED")); pool.close().await;
}

#[tokio::test]
async fn admin_binding_is_fingerprint_checked_audited_persistent_and_report_cannot_change_it() {
    let mut app = TestApp::new(CookiePolicy::default(),OAuthRuntime::new(Vec::new())).await;
    let guid = seed(&app).await;
    let admin = admin(&app).await; let (owner,owner_token) = owner(&app).await;
    for path in ["/api/admin/device/bind","/api/admin/device/unbind","/api/admin/device/registry"] {
        let method = if path.ends_with("registry") { "GET" } else { "POST" };
        assert_eq!(authenticated(&app,method,path,json!({"peer_id":"123456","user_id":owner,"pk_fingerprint":key_fingerprint(&[3;32]),"id":"none"}),&owner_token).await.status(),StatusCode::FORBIDDEN);
    }
    let wrong = authenticated(&app,"POST","/api/admin/device/bind",json!({"peer_id":"123456","user_id":owner,"pk_fingerprint":key_fingerprint(&[4;32])}),&admin).await;
    assert_eq!(wrong.status(),StatusCode::CONFLICT);
    let id = bind(&app,&owner,&admin).await;
    let before = value(authenticated(&app,"GET","/api/devices",json!({}),&owner_token).await).await;
    assert_eq!(before["data"][0]["id"],id); assert_eq!(before["data"][0]["verified"],true); assert_eq!(before["data"][0]["online"],false);
    let mut spoofed = report(); spoofed["user_id"] = json!("attacker"); spoofed["pk"] = json!(base64::encode([4;32])); spoofed["verified"] = json!(true); spoofed["status"] = json!(0); spoofed["online"] = json!(true);
    assert_eq!(body(app.send(request("POST","/api/sysinfo",spoofed)).await).await,"SYSINFO_UPDATED");
    let after = value(authenticated(&app,"GET","/api/devices",json!({}),&owner_token).await).await;
    assert_eq!(before,after);
    app.reopen(OAuthRuntime::new(Vec::new()),Some(hbbs::oauth_admin::ProviderSecretKey::from_bytes(&[7;32]).unwrap())).await.unwrap();
    let reopened = value(authenticated(&app,"GET","/api/devices",json!({}),&owner_token).await).await;
    assert_eq!(reopened["data"][0]["id"],id);
    let registry = value(authenticated(&app,"GET","/api/admin/device/registry?peer_id=123456",json!({}),&admin).await).await;
    assert_eq!(registry["data"][0]["untrusted_sysinfo"]["user_id"],"attacker");
    assert_eq!(registry["data"][0]["owner_id"],owner);
    assert_eq!(authenticated(&app,"POST","/api/admin/device/unbind",json!({"id":id}),&admin).await.status(),StatusCode::OK);
    assert_eq!(value(authenticated(&app,"GET","/api/devices",json!({}),&owner_token).await).await["data"],json!([]));
    let pool = pool(&app).await;
    assert_eq!(sqlx::query_scalar::<_,i64>("select count(*) from api_device_binding_audit").fetch_one(&pool).await.unwrap(),2);
    let pk: Vec<u8> = sqlx::query_scalar("select pk from peer where guid=?").bind(guid).fetch_one(&pool).await.unwrap();
    assert_eq!(pk,[3;32]); pool.close().await;
}

#[tokio::test]
async fn historical_links_remain_pending_and_key_changes_hide_verified_links() {
    let app = TestApp::new(CookiePolicy::default(),OAuthRuntime::new(Vec::new())).await;
    let guid = seed(&app).await; let (owner,token) = owner(&app).await; let admin = admin(&app).await;
    db(&app).await.upsert_api_device("legacy",&owner,&base64::encode(b"device-uuid"),"Legacy","Linux","client","{\"old\":true}").await.unwrap();
    assert_eq!(value(authenticated(&app,"GET","/api/devices",json!({}),&token).await).await["data"],json!([]));
    let admin_list = value(authenticated(&app,"GET","/api/admin/device/list",json!({}),&admin).await).await;
    assert_eq!(admin_list["data"][0]["id"],"legacy"); assert_eq!(admin_list["data"][0]["verified"],false);
    assert_eq!(bind(&app,&owner,&admin).await,"legacy");
    db(&app).await.update_pk(&guid,"123456",&[4;32],"{}").await.unwrap();
    assert_eq!(value(authenticated(&app,"GET","/api/devices",json!({}),&token).await).await["data"],json!([]));
    let list = value(authenticated(&app,"GET","/api/admin/device/list",json!({}),&admin).await).await;
    assert_eq!(list["data"][0]["info"],"{\"old\":true}"); assert_eq!(list["data"][0]["verified"],false);
}

#[tokio::test]
async fn registration_queue_is_bounded_flushes_and_online_expires_without_api_reports_reviving_it() {
    let app = TestApp::new(CookiePolicy::default(),OAuthRuntime::new(Vec::new())).await;
    let guid = seed(&app).await; let admin = admin(&app).await; let (owner,token) = owner(&app).await;
    bind(&app,&owner,&admin).await;
    let (writer,task) = RegistrationWriter::start(db(&app).await,1);
    let now = now_ms();
    let observation = RegistrationObservation { guid:guid.clone(),uuid:b"device-uuid".to_vec(),pk:vec![3;32],registered_at_ms:now };
    assert!(writer.observe(observation.clone()));
    assert!(!writer.observe(observation),"a full queue must return immediately without accepting the second event");
    drop(writer); task.await.unwrap();
    let list = value(authenticated(&app,"GET","/api/devices",json!({}),&token).await).await;
    assert_eq!(list["data"][0]["online"],true);
    let pool = pool(&app).await;
    sqlx::query("update peer_registration set registered_at_ms=?").bind(now_ms()-30_001).execute(&pool).await.unwrap();
    assert_eq!(body(app.send(request("POST","/api/sysinfo",report())).await).await,"SYSINFO_UPDATED");
    assert_eq!(value(authenticated(&app,"GET","/api/devices",json!({}),&token).await).await["data"][0]["online"],false);
    let (writer,task) = RegistrationWriter::start(db(&app).await,8);
    let now = now_ms();
    for timestamp in [now,now-1000] {
        assert!(writer.observe(RegistrationObservation { guid:guid.clone(),uuid:b"device-uuid".to_vec(),pk:vec![3;32],registered_at_ms:timestamp }));
    }
    assert!(writer.observe(RegistrationObservation { guid,uuid:b"wrong-uuid".to_vec(),pk:vec![3;32],registered_at_ms:now+1_000_000 }));
    drop(writer); task.await.unwrap();
    assert_eq!(sqlx::query_scalar::<_,i64>("select registered_at_ms from peer_registration").fetch_one(&pool).await.unwrap(),now);
    pool.close().await;
}

#[tokio::test]
async fn oversized_reports_and_capacity_are_rejected_without_success_or_growth() {
    let app = TestApp::new(CookiePolicy::default(),OAuthRuntime::new(Vec::new())).await;
    seed(&app).await;
    let mut oversized = report(); oversized["padding"] = json!("x".repeat(65_536));
    let length = oversized.to_string().len();
    let mut large_request = request("POST","/api/sysinfo",oversized);
    large_request.headers_mut().insert(header::CONTENT_LENGTH,length.to_string().parse().unwrap());
    assert_eq!(app.send(large_request).await.status(),StatusCode::PAYLOAD_TOO_LARGE);
    let pool = pool(&app).await;
    sqlx::query("with recursive n(x) as (select 1 union all select x+1 from n where x<10000)
        insert into peer(guid,id,uuid,pk,info) select cast('capacity-'||x as blob),'capacity-'||x,x'01',x'01','{}' from n").execute(&pool).await.unwrap();
    sqlx::query("insert into api_device_report(peer_guid,uuid,pk) select guid,uuid,pk from peer where id like 'capacity-%'").execute(&pool).await.unwrap();
    let response = app.send(request("POST","/api/sysinfo",report())).await;
    assert_eq!(response.status(),StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(sqlx::query_scalar::<_,i64>("select count(*) from api_device_report").fetch_one(&pool).await.unwrap(),10_000);
    pool.close().await;
}

#[tokio::test]
async fn historical_database_migration_preserves_links_and_group_rows_without_auto_verifying() {
    let mut app = TestApp::new(CookiePolicy::default(),OAuthRuntime::new(Vec::new())).await;
    let (owner,token) = owner(&app).await;
    let pool = pool(&app).await;
    sqlx::query("insert into api_device(id,user_id,uuid,name,info) values('legacy',?,'legacy-uuid','Historical','{\"kept\":true}')").bind(&owner).execute(&pool).await.unwrap();
    sqlx::query("insert into api_device_group(id,name,created_by) values('group','historical',?)").bind(&owner).execute(&pool).await.unwrap();
    sqlx::query("insert into api_device_group_device(group_id,device_id) values('group','legacy')").execute(&pool).await.unwrap();
    sqlx::query("drop index api_device_verified_peer;
        alter table api_device drop column peer_guid;
        alter table api_device drop column verified;
        alter table api_device drop column verified_uuid;
        alter table api_device drop column verified_pk;
        drop table peer_registration;
        drop table api_device_report;
        drop table api_device_binding_audit;
        delete from api_schema_migration where version=2").execute(&pool).await.unwrap();
    app.reopen(OAuthRuntime::new(Vec::new()),Some(hbbs::oauth_admin::ProviderSecretKey::from_bytes(&[7;32]).unwrap())).await.unwrap();
    app.reopen(OAuthRuntime::new(Vec::new()),Some(hbbs::oauth_admin::ProviderSecretKey::from_bytes(&[7;32]).unwrap())).await.unwrap();
    assert_eq!(sqlx::query_scalar::<_,i64>("select verified from api_device where id='legacy'").fetch_one(&pool).await.unwrap(),0);
    assert_eq!(sqlx::query_scalar::<_,String>("select info from api_device where id='legacy'").fetch_one(&pool).await.unwrap(),"{\"kept\":true}");
    assert_eq!(sqlx::query_scalar::<_,i64>("select count(*) from api_device_group_device").fetch_one(&pool).await.unwrap(),1);
    assert_eq!(value(authenticated(&app,"GET","/api/devices",json!({}),&token).await).await["data"],json!([]));
    assert!(db(&app).await.list_api_device_group_members(&owner,false).await.unwrap().is_empty());
    pool.close().await;
}

#[tokio::test]
async fn real_hbbs_udp_registrations_persist_their_time_and_survive_api_restart() {
    use hbb_common::rendezvous_proto::{RendezvousMessage,RegisterPk,RegisterPeer,rendezvous_message,register_pk_response};
    let mut app = TestApp::new(CookiePolicy::default(),OAuthRuntime::new(Vec::new())).await;
    let path = app.database_path();
    let server = common::rendezvous::MockHbbs::start(path.parent().unwrap(),&path).await;
    let socket = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    let mut message = RendezvousMessage::new();
    message.set_register_pk(RegisterPk { id:"654321".to_owned(),uuid:b"udp-device".to_vec().into(),pk:vec![5;32].into(),..Default::default() });
    let response = server.exchange(&socket,message);
    assert!(matches!(response.union,Some(rendezvous_message::Union::RegisterPkResponse(ref result)) if result.result.enum_value().unwrap()==register_pk_response::Result::OK));
    let pool = pool(&app).await;
    for _ in 0..100 {
        if sqlx::query_scalar::<_,i64>("select count(*) from peer_registration").fetch_one(&pool).await.unwrap()==1 { break; }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    let first: i64 = sqlx::query_scalar("select registered_at_ms from peer_registration").fetch_one(&pool).await.unwrap();
    assert!(first > now_ms()-5000);
    sqlx::query("update peer_registration set registered_at_ms=1").execute(&pool).await.unwrap();
    let mut message = RendezvousMessage::new(); message.set_register_peer(RegisterPeer { id:"654321".to_owned(),..Default::default() });
    let response = server.exchange(&socket,message);
    assert!(matches!(response.union,Some(rendezvous_message::Union::RegisterPeerResponse(ref result)) if !result.request_pk));
    for _ in 0..100 {
        if sqlx::query_scalar::<_,i64>("select registered_at_ms from peer_registration").fetch_one(&pool).await.unwrap()>1 { break; }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    let refreshed: i64 = sqlx::query_scalar("select registered_at_ms from peer_registration").fetch_one(&pool).await.unwrap();
    assert!(refreshed>=first);
    drop(server);
    app.reopen(OAuthRuntime::new(Vec::new()),Some(hbbs::oauth_admin::ProviderSecretKey::from_bytes(&[7;32]).unwrap())).await.unwrap();
    let token = admin(&app).await;
    let registry = value(authenticated(&app,"GET","/api/admin/device/registry?peer_id=654321",json!({}),&token).await).await;
    assert_eq!(registry["data"][0]["registered_at_ms"],refreshed);
    assert_eq!(registry["data"][0]["online"],true); pool.close().await;
}

#[tokio::test]
async fn real_hbbs_failed_key_persistence_returns_error_and_retry_does_not_use_failed_cache() {
    use hbb_common::rendezvous_proto::{RendezvousMessage,RegisterPk,rendezvous_message,register_pk_response};
    for operation in ["insert","update"] {
        let app = TestApp::new(CookiePolicy::default(),OAuthRuntime::new(Vec::new())).await;
        let path = app.database_path();
        let database = db(&app).await;
        if operation=="update" { database.insert_peer("765432",b"failure-device",&[3;32],"{\"ip\":\"127.0.0.1\"}").await.unwrap(); }
        let server = common::rendezvous::MockHbbs::start(path.parent().unwrap(),&path).await;
        let socket = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
        let pool = pool(&app).await;
        sqlx::query(&format!("create trigger fail_registration before {operation} on peer begin select raise(abort,'isolated persistence failure'); end")).execute(&pool).await.unwrap();
        let registration = || {
            let mut message = RendezvousMessage::new();
            message.set_register_pk(RegisterPk { id:"765432".to_owned(),uuid:b"failure-device".to_vec().into(),pk:vec![7;32].into(),..Default::default() }); message
        };
        for _ in 0..2 {
            let response = server.exchange(&socket,registration());
            assert!(matches!(response.union,Some(rendezvous_message::Union::RegisterPkResponse(ref result)) if result.result.enum_value().unwrap()==register_pk_response::Result::SERVER_ERROR),"{operation}: persistence failure must not return OK");
        }
        assert_eq!(sqlx::query_scalar::<_,i64>("select count(*) from peer_registration").fetch_one(&pool).await.unwrap(),0);
        if operation=="update" { assert_eq!(database.get_peer("765432").await.unwrap().unwrap().pk,vec![3;32]); }
        else { assert!(database.get_peer("765432").await.unwrap().is_none()); }
        sqlx::query("drop trigger fail_registration").execute(&pool).await.unwrap();
        let response = server.exchange(&socket,registration());
        assert!(matches!(response.union,Some(rendezvous_message::Union::RegisterPkResponse(ref result)) if result.result.enum_value().unwrap()==register_pk_response::Result::OK));
        assert_eq!(database.get_peer("765432").await.unwrap().unwrap().pk,vec![7;32]);
        drop(server); pool.close().await;
    }
}

#[tokio::test]
async fn concurrent_sysinfo_reports_allow_only_one_write_and_authority_changes_request_new_sysinfo() {
    let app = TestApp::new(CookiePolicy::default(),OAuthRuntime::new(Vec::new())).await;
    let guid = seed(&app).await;
    let (one,two) = tokio::join!(app.send(request("POST","/api/sysinfo",report())),app.send(request("POST","/api/sysinfo",report())));
    let statuses = [one.status(),two.status()];
    assert_eq!(statuses.iter().filter(|status|**status==StatusCode::OK).count(),1);
    assert_eq!(statuses.iter().filter(|status|**status==StatusCode::TOO_MANY_REQUESTS).count(),1);
    db(&app).await.update_pk(&guid,"123456",&[4;32],"{}").await.unwrap();
    let heartbeat = app.send(request("POST","/api/heartbeat",json!({"id":"123456","uuid":base64::encode(b"device-uuid")}))).await;
    assert_eq!(value(heartbeat).await["sysinfo"],true);
}

#[tokio::test]
async fn ownership_operations_roll_back_if_their_audit_cannot_be_saved() {
    let app = TestApp::new(CookiePolicy::default(),OAuthRuntime::new(Vec::new())).await;
    seed(&app).await; let (owner,owner_token) = owner(&app).await; let admin = admin(&app).await;
    let pool = pool(&app).await;
    sqlx::query("create trigger deny_binding_audit before insert on api_device_binding_audit begin select raise(abort,'test audit failure'); end").execute(&pool).await.unwrap();
    let failed = authenticated(&app,"POST","/api/admin/device/bind",json!({"peer_id":"123456","user_id":owner,"pk_fingerprint":key_fingerprint(&[3;32])}),&admin).await;
    assert_eq!(failed.status(),StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(sqlx::query_scalar::<_,i64>("select count(*) from api_device").fetch_one(&pool).await.unwrap(),0);
    sqlx::query("drop trigger deny_binding_audit").execute(&pool).await.unwrap();
    let id = bind(&app,&owner,&admin).await;
    sqlx::query("create trigger deny_binding_audit before insert on api_device_binding_audit begin select raise(abort,'test audit failure'); end").execute(&pool).await.unwrap();
    for path in ["/api/admin/device/unbind","/api/admin/device/delete"] {
        assert_eq!(authenticated(&app,"POST",path,json!({"id":id}),&admin).await.status(),StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(value(authenticated(&app,"GET","/api/devices",json!({}),&owner_token).await).await["data"][0]["verified"],true);
    }
    sqlx::query("drop trigger deny_binding_audit").execute(&pool).await.unwrap();
    assert_eq!(authenticated(&app,"POST","/api/admin/device/delete",json!({"id":id}),&admin).await.status(),StatusCode::OK);
    assert_eq!(sqlx::query_scalar::<_,i64>("select count(*) from api_device_binding_audit where action='delete'").fetch_one(&pool).await.unwrap(),1);
    pool.close().await;
}

#[tokio::test]
async fn rebinding_an_owner_cannot_silently_replace_another_verified_peer() {
    let app = TestApp::new(CookiePolicy::default(),OAuthRuntime::new(Vec::new())).await;
    seed(&app).await; let (owner,token) = owner(&app).await; let admin = admin(&app).await;
    let id = bind(&app,&owner,&admin).await;
    db(&app).await.insert_peer("654321",b"device-uuid",&[3;32],"{}").await.unwrap();
    let response = authenticated(&app,"POST","/api/admin/device/bind",json!({"peer_id":"654321","user_id":owner,"pk_fingerprint":key_fingerprint(&[3;32])}),&admin).await;
    assert_eq!(response.status(),StatusCode::CONFLICT);
    let list = value(authenticated(&app,"GET","/api/devices",json!({}),&token).await).await;
    assert_eq!(list["data"][0]["id"],id); assert_eq!(list["data"][0]["peer_id"],"123456");
}
