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
        assert_eq!(peer["hostname"],"Laptop"); assert_eq!(peer["platform"],"Linux"); assert_eq!(peer["forceAlwaysRelay"],"true");
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

#[tokio::test]
async fn snapshot_only_entry_can_be_deleted_from_both_read_paths_and_repeated_delete_is_explicit() {
    let (app,token) = fixture().await;
    assert_eq!(auth(&app,"DELETE","/api/ab/peer/123456",json!({}),&token).await.status(),StatusCode::OK);
    assert_eq!(book(&app,&token).await["peers"],json!([]));
    assert_eq!(value(auth(&app,"GET","/api/ab/peers",json!({}),&token).await).await["data"],json!([]));
    assert_eq!(auth(&app,"DELETE","/api/ab/peer/123456",json!({}),&token).await.status(),StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn deleting_own_snapshot_peer_does_not_delete_another_accounts_identical_peer() {
    let (app,token) = fixture().await;
    app.send(request("POST","/api/register",json!({"username":"other","password":"other-password"}))).await;
    let other = value(app.send(request("POST","/api/login",json!({"username":"other","password":"other-password"}))).await).await["access_token"].as_str().unwrap().to_owned();
    let imported = book(&app,&token).await;
    assert_eq!(auth(&app,"POST","/api/ab",json!({"data":imported.to_string()}),&other).await.status(),StatusCode::OK);
    let entry = value(auth(&app,"POST","/api/ab/peer",json!({"peer_id":"123456","alias":"Other"}),&other).await).await;
    let before = book(&app,&other).await;
    assert_eq!(auth(&app,"POST","/api/ab/peer/delete",json!({"id":entry["data"]["id"]}),&token).await.status(),StatusCode::NOT_FOUND);
    assert_eq!(auth(&app,"POST","/api/ab/peer/delete",json!({"id":"123456"}),&token).await.status(),StatusCode::OK);
    assert_eq!(book(&app,&other).await,before);
}

#[tokio::test]
async fn failed_delete_snapshot_write_rolls_back_index_deletion() {
    let (app,token) = fixture().await;
    let entry = value(auth(&app,"POST","/api/ab/peer",json!({"peer_id":"123456","alias":"Indexed"}),&token).await).await;
    let before = book(&app,&token).await;
    let pool = sqlx::SqlitePool::connect(app.database_path().to_str().unwrap()).await.unwrap();
    sqlx::query("create trigger reject_delete_snapshot before update on api_address_book_snapshot begin select raise(abort,'test-only failure'); end").execute(&pool).await.unwrap();
    let path = format!("/api/ab/peer/{}",entry["data"]["id"].as_str().unwrap());
    assert_eq!(auth(&app,"DELETE",&path,json!({}),&token).await.status(),StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(book(&app,&token).await,before);
    assert_eq!(sqlx::query_scalar::<_,i64>("select count(*) from api_address_book_entry where peer_id='123456'").fetch_one(&pool).await.unwrap(),1);
    pool.close().await;
}

#[tokio::test]
async fn changing_peer_id_keeps_entry_identity_and_saved_fields_and_allows_partial_edit_by_entry_id() {
    let (app,token) = fixture().await;
    let entry = value(auth(&app,"POST","/api/ab/peer",json!({"peer_id":"123456","alias":"Before rename"}),&token).await).await;
    let id = entry["data"]["id"].as_str().unwrap();
    let response = auth(&app,"POST","/api/ab/peer",json!({"id":id,"peer_id":"654321","alias":"After rename"}),&token).await;
    assert_eq!(response.status(),StatusCode::OK);
    let changed = value(response).await; assert_eq!(changed["data"]["id"],id); assert_eq!(changed["data"]["peerId"],"654321");
    let peers = book(&app,&token).await["peers"].as_array().unwrap().clone(); assert_eq!(peers.len(),1);
    assert_eq!(peers[0]["id"],"654321"); assert_eq!(peers[0]["entryId"],id); assert_eq!(peers[0]["hash"],"saved-hash"); assert_eq!(peers[0]["extension"],json!({"nested":[1,2]}));
    assert_eq!(auth(&app,"POST","/api/ab/peer",json!({"id":id,"note":"Only note"}),&token).await.status(),StatusCode::OK);
    assert_eq!(book(&app,&token).await["peers"][0]["note"],"Only note");
}

#[tokio::test]
async fn duplicate_target_and_foreign_entry_identity_cannot_change_address_book_data() {
    let (app,token) = fixture().await;
    let entry = value(auth(&app,"POST","/api/ab/peer",json!({"peer_id":"123456"}),&token).await).await;
    assert_eq!(auth(&app,"POST","/api/ab/peer",json!({"peer_id":"654321","alias":"Existing"}),&token).await.status(),StatusCode::OK);
    let before = book(&app,&token).await;
    assert_eq!(auth(&app,"POST","/api/ab/peer",json!({"id":entry["data"]["id"],"peer_id":"654321","alias":"Conflict"}),&token).await.status(),StatusCode::CONFLICT);
    assert_eq!(book(&app,&token).await,before);
    app.send(request("POST","/api/register",json!({"username":"other","password":"other-password"}))).await;
    let other = value(app.send(request("POST","/api/login",json!({"username":"other","password":"other-password"}))).await).await["access_token"].as_str().unwrap().to_owned();
    assert_eq!(auth(&app,"POST","/api/ab/peer",json!({"id":entry["data"]["id"],"peer_id":"foreign","alias":"Attack"}),&other).await.status(),StatusCode::NOT_FOUND);
    assert_eq!(book(&app,&other).await["peers"],json!([])); assert_eq!(book(&app,&token).await,before);
}

#[tokio::test]
async fn relay_string_and_boolean_inputs_round_trip_to_official_strings_and_web_booleans() {
    let (app,token) = fixture().await;
    for (input,enabled) in [(json!("true"),true),(json!("false"),false),(json!(true),true),(json!(false),false)] {
        let data = json!({"peers":[{"id":"123456","forceAlwaysRelay":input}]});
        assert_eq!(auth(&app,"POST","/api/ab",json!({"data":data.to_string()}),&token).await.status(),StatusCode::OK);
        assert_eq!(book(&app,&token).await["peers"][0]["forceAlwaysRelay"],enabled.to_string());
        let web = value(auth(&app,"GET","/api/ab/peers",json!({}),&token).await).await;
        assert_eq!(web["data"][0]["forceAlwaysRelay"],enabled);
        assert_eq!(auth(&app,"POST","/api/ab/peer",json!({"peer_id":"123456","alias":"Only alias"}),&token).await.status(),StatusCode::OK);
        assert_eq!(book(&app,&token).await["peers"][0]["forceAlwaysRelay"],enabled.to_string());
    }
    for (field,input,expected) in [("forceAlwaysRelay",json!("true"),true),("force_always_relay",json!("false"),false),("force_always_relay",json!(true),true),("forceAlwaysRelay",json!(false),false)] {
        let response = auth(&app,"POST","/api/ab/peer",json!({"peer_id":"123456",(field):input}),&token).await;
        assert_eq!(response.status(),StatusCode::OK); assert_eq!(value(response).await["data"]["forceAlwaysRelay"],expected);
        assert_eq!(book(&app,&token).await["peers"][0]["forceAlwaysRelay"],expected.to_string());
    }
}

#[tokio::test]
async fn invalid_relay_values_never_silently_become_false_or_modify_the_book() {
    let (app,token) = fixture().await; let before = book(&app,&token).await;
    for input in [json!("TRUE"),json!("no"),json!(0),json!(null),json!([]),json!({})] {
        let document = json!({"peers":[{"id":"123456","forceAlwaysRelay":input}]});
        assert_eq!(auth(&app,"POST","/api/ab",json!({"data":document.to_string()}),&token).await.status(),StatusCode::BAD_REQUEST);
        let response = auth(&app,"POST","/api/ab/peer",json!({"peer_id":"123456","force_always_relay":input}),&token).await;
        assert!(matches!(response.status(),StatusCode::BAD_REQUEST|StatusCode::UNPROCESSABLE_ENTITY));
        assert_eq!(book(&app,&token).await,before);
    }
    let pool = sqlx::SqlitePool::connect(app.database_path().to_str().unwrap()).await.unwrap();
    let damaged = json!({"peers":[{"id":"123456","forceAlwaysRelay":"damaged"}]}).to_string();
    sqlx::query("update api_address_book_snapshot set data=?").bind(&damaged).execute(&pool).await.unwrap();
    assert_eq!(auth(&app,"GET","/api/ab",json!({}),&token).await.status(),StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(sqlx::query_scalar::<_,String>("select data from api_address_book_snapshot").fetch_one(&pool).await.unwrap(),damaged); pool.close().await;
}

#[tokio::test]
async fn legacy_tag_color_map_is_a_json_string_and_preserves_all_argb_values_across_edits() {
    let (app,token) = fixture().await;
    let data = json!({"peers":[{"id":"123456","tags":["red","transparent"]}],"tags":["red","transparent","max"],"tag_colors":json!({"red":4294901760u32,"transparent":0x12345678u32,"max":u32::MAX}).to_string()});
    assert_eq!(auth(&app,"POST","/api/ab",json!({"data":data.to_string()}),&token).await.status(),StatusCode::OK);
    let colors: Value = serde_json::from_str(book(&app,&token).await["tag_colors"].as_str().unwrap()).unwrap();
    assert_eq!(colors,json!({"red":4294901760u32,"transparent":0x12345678u32,"max":u32::MAX}));
    let response = auth(&app,"POST","/api/ab/tags",json!({"name":"new","color":"#aabbcc"}),&token).await; assert_eq!(response.status(),StatusCode::OK);
    let colors: Value = serde_json::from_str(book(&app,&token).await["tag_colors"].as_str().unwrap()).unwrap();
    assert_eq!(colors["red"],4294901760u32); assert_eq!(colors["transparent"],0x12345678u32); assert_eq!(colors["max"],u32::MAX); assert_eq!(colors["new"],0xffaabbccu32);
    let tags = value(auth(&app,"GET","/api/ab/tags",json!({}),&token).await).await;
    assert_eq!(tags["data"].as_array().unwrap().iter().find(|tag|tag["name"]=="transparent").unwrap()["color"],"#34567812");
}

#[tokio::test]
async fn tag_rename_and_delete_update_only_the_related_colors_and_peer_references() {
    let (app,token) = fixture().await;
    let data = json!({"peers":[{"id":"123456","tags":["old","keep"],"password":"saved"}],"tags":["old","keep"],"tag_colors":{"old":"#11223344","keep":"#abcdef"}});
    assert_eq!(auth(&app,"POST","/api/ab",json!({"data":data.to_string()}),&token).await.status(),StatusCode::OK);
    assert_eq!(auth(&app,"POST","/api/ab/peer",json!({"peer_id":"123456","alias":"Indexed"}),&token).await.status(),StatusCode::OK);
    assert_eq!(auth(&app,"POST","/api/ab/tags",json!({"name":"renamed","old_name":"old"}),&token).await.status(),StatusCode::OK);
    let doc = book(&app,&token).await; let colors: Value = serde_json::from_str(doc["tag_colors"].as_str().unwrap()).unwrap();
    assert_eq!(colors,json!({"renamed":0x44112233u32,"keep":0xffabcdefu32})); assert_eq!(doc["tags"],json!(["renamed","keep"])); assert_eq!(doc["peers"][0]["tags"],json!(["renamed","keep"])); assert_eq!(doc["peers"][0]["password"],"saved");
    assert_eq!(auth(&app,"POST","/api/ab/tags/delete",json!({"name":"renamed"}),&token).await.status(),StatusCode::OK);
    let doc = book(&app,&token).await; let colors: Value = serde_json::from_str(doc["tag_colors"].as_str().unwrap()).unwrap();
    assert_eq!(colors,json!({"keep":0xffabcdefu32})); assert_eq!(doc["peers"][0]["tags"],json!(["keep"]));
}

#[tokio::test]
async fn damaged_tag_colors_are_reported_and_never_replaced_with_empty_data() {
    let (app,token) = fixture().await; let before = book(&app,&token).await;
    for invalid in [json!("not-json"),json!("[]"),json!({"old":-1}),json!({"old":4294967296u64}),json!({"old":"#gggggg"})] {
        assert_eq!(auth(&app,"POST","/api/ab",json!({"data":json!({"tags":["old"],"tag_colors":invalid}).to_string()}),&token).await.status(),StatusCode::BAD_REQUEST);
        assert_eq!(book(&app,&token).await,before);
    }
    let pool = sqlx::SqlitePool::connect(app.database_path().to_str().unwrap()).await.unwrap();
    let damaged = json!({"tags":["old"],"tag_colors":"not-json"}).to_string();
    sqlx::query("update api_address_book_snapshot set data=?").bind(&damaged).execute(&pool).await.unwrap();
    let response = auth(&app,"POST","/api/ab/tags",json!({"name":"new","color":"#ffffff"}),&token).await;
    assert_eq!(response.status(),StatusCode::BAD_REQUEST); assert_eq!(value(response).await["error"],"invalid_tag_colors");
    assert_eq!(sqlx::query_scalar::<_,String>("select data from api_address_book_snapshot").fetch_one(&pool).await.unwrap(),damaged); pool.close().await;
}

#[tokio::test]
async fn tag_rename_conflicts_and_failed_persistence_leave_colors_and_indexes_unchanged() {
    let (app,token) = fixture().await;
    assert_eq!(auth(&app,"POST","/api/ab/peer",json!({"peer_id":"123456"}),&token).await.status(),StatusCode::OK);
    assert_eq!(auth(&app,"POST","/api/ab/tags",json!({"name":"keep","color":"#abcdef"}),&token).await.status(),StatusCode::OK);
    let before = book(&app,&token).await;
    assert_eq!(auth(&app,"POST","/api/ab/tags",json!({"old_name":"old","name":"keep"}),&token).await.status(),StatusCode::CONFLICT);
    assert_eq!(book(&app,&token).await,before);
    let pool = sqlx::SqlitePool::connect(app.database_path().to_str().unwrap()).await.unwrap();
    sqlx::query("create trigger reject_tag_snapshot before update on api_address_book_snapshot begin select raise(abort,'test-only failure'); end").execute(&pool).await.unwrap();
    assert_eq!(auth(&app,"POST","/api/ab/tags",json!({"old_name":"old","name":"changed","color":"#11223344"}),&token).await.status(),StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(book(&app,&token).await,before);
    assert_eq!(sqlx::query_scalar::<_,String>("select tags from api_address_book_entry").fetch_one(&pool).await.unwrap(),"[\"old\"]"); pool.close().await;
}
