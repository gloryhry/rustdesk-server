mod common;
use common::*;
use axum::{body::Body,http::{header,Request,StatusCode},response::Response};
use hbbs::{api::CookiePolicy,oauth::OAuthRuntime};
use serde::{Deserialize};
use serde_json::{json,Value};

async fn value(response: Response) -> Value { serde_json::from_slice(&hyper::body::to_bytes(response.into_body()).await.unwrap()).unwrap() }
async fn call(app: &TestApp, token: &str, method: &str, path: &str, body: Option<Value>) -> Response {
    let request = Request::builder().method(method).uri(path).header(header::AUTHORIZATION,format!("Bearer {token}"))
        .header(header::CONTENT_TYPE,"application/json").body(body.map_or_else(Body::empty,|body|Body::from(body.to_string()))).unwrap();
    app.send_raw(request).await
}
async fn fixture() -> (TestApp,String,String) {
    let app = TestApp::new(CookiePolicy::default(),OAuthRuntime::new(Vec::new())).await;
    let token = value(app.send(request("POST","/api/login",json!({"username":"admin","password":"admin-password"}))).await).await["access_token"].as_str().unwrap().to_owned();
    let guid = value(call(&app,&token,"GET","/api/ab",None).await).await["guid"].as_str().unwrap().to_owned();
    (app,token,guid)
}
async fn action(response: Response) { assert_eq!(response.status(),StatusCode::OK); assert!(hyper::body::to_bytes(response.into_body()).await.unwrap().is_empty(),"official action parser requires an empty successful response"); }
#[derive(Deserialize)]
struct Personal { guid: String }
#[derive(Deserialize)]
struct Settings { max_peer_one_ab: u64 }
#[derive(Deserialize)]
struct Page<T> { total: usize, data: Vec<T> }
#[derive(Deserialize)]
#[allow(dead_code)]
struct Peer {
    id: String, #[serde(default)] alias: String,
    #[serde(default,rename="forceAlwaysRelay")] relay: String,
    #[serde(default)] hash: Option<String>, #[serde(default)] password: Option<String>,
    #[serde(default)] username: String, #[serde(default)] hostname: String,
    #[serde(default)] platform: String, #[serde(default)] tags: Vec<Value>,
    #[serde(default,rename="rdpPort")] rdp_port: Option<String>,
    #[serde(default,rename="rdpUsername")] rdp_username: Option<String>,
    #[serde(default,rename="loginName")] login_name: Option<String>,
    #[serde(default)] device_group_name: Option<String>, #[serde(default)] note: Option<String>,
    same_server: Option<bool>,
}
#[derive(Deserialize)]
struct Tag { name: String, color: u32 }

#[tokio::test]
async fn official_empty_body_initialization_sequence_parses_root_objects_and_arrays() {
    let (app,token,guid) = fixture().await;
    let response = call(&app,&token,"POST","/api/ab/personal",None).await; assert_eq!(response.status(),StatusCode::OK);
    let personal: Personal = serde_json::from_value(value(response).await).unwrap(); assert_eq!(personal.guid,guid);
    let settings: Settings = serde_json::from_value(value(call(&app,&token,"POST","/api/ab/settings",None).await).await).unwrap(); assert_eq!(settings.max_peer_one_ab,0);
    let profiles: Page<Value> = serde_json::from_value(value(call(&app,&token,"POST","/api/ab/shared/profiles?current=1&pageSize=100",None).await).await).unwrap(); assert_eq!(profiles.total,0); assert!(profiles.data.is_empty());
    let peers: Page<Peer> = serde_json::from_value(value(call(&app,&token,"POST",&format!("/api/ab/peers?ab={guid}&current=1&pageSize=100"),None).await).await).unwrap(); assert_eq!(peers.total,0); assert!(peers.data.is_empty());
    let tags: Vec<Tag> = serde_json::from_value(value(call(&app,&token,"POST",&format!("/api/ab/tags/{guid}"),None).await).await).unwrap(); assert!(tags.is_empty());
}

#[tokio::test]
async fn official_peer_crud_is_partial_and_uses_peer_ids_and_empty_success_bodies() {
    let (app,token,guid) = fixture().await;
    action(call(&app,&token,"POST",&format!("/api/ab/peer/add/{guid}"),Some(json!({"id":"123456","alias":"Original","hash":"saved-hash","rdpPort":"3390","extension":{"keep":true},"forceAlwaysRelay":"true","tags":["ops"]}))).await).await;
    let page: Page<Peer> = serde_json::from_value(value(call(&app,&token,"POST",&format!("/api/ab/peers?ab={guid}"),None).await).await).unwrap(); assert_eq!(page.total,1); assert_eq!(page.data[0].id,"123456"); assert_eq!(page.data[0].relay,"true");
    action(call(&app,&token,"PUT",&format!("/api/ab/peer/update/{guid}"),Some(json!({"id":"123456","alias":"Edited","note":"new"}))).await).await;
    let response = value(call(&app,&token,"POST",&format!("/api/ab/peers?ab={guid}"),None).await).await;
    let saved = &response["data"][0]; assert_eq!(saved["alias"],"Edited"); assert_eq!(saved["hash"],"saved-hash"); assert_eq!(saved["extension"],json!({"keep":true})); assert!(saved.get("entryId").is_none());
    assert_eq!(call(&app,&token,"POST",&format!("/api/ab/peer/add/{guid}"),Some(json!({"id":"123456"}))).await.status(),StatusCode::CONFLICT);
    assert_eq!(call(&app,&token,"PUT",&format!("/api/ab/peer/update/{guid}"),Some(json!({"id":"missing","alias":"bad"}))).await.status(),StatusCode::NOT_FOUND);
    action(call(&app,&token,"DELETE",&format!("/api/ab/peer/{guid}"),Some(json!(["123456"]))).await).await;
    let page: Page<Peer> = serde_json::from_value(value(call(&app,&token,"POST",&format!("/api/ab/peers?ab={guid}"),None).await).await).unwrap(); assert_eq!(page.total,0);
}

#[tokio::test]
async fn official_tag_crud_uses_integer_argb_put_rename_and_array_deletion() {
    let (app,token,guid) = fixture().await;
    for (name,color) in [("old",0x12345678u32),("keep",u32::MAX)] { action(call(&app,&token,"POST",&format!("/api/ab/tag/add/{guid}"),Some(json!({"name":name,"color":color}))).await).await; }
    action(call(&app,&token,"POST",&format!("/api/ab/peer/add/{guid}"),Some(json!({"id":"123456","tags":["old","keep"],"hash":"keep"}))).await).await;
    action(call(&app,&token,"PUT",&format!("/api/ab/tag/rename/{guid}"),Some(json!({"old":"old","new":"renamed"}))).await).await;
    action(call(&app,&token,"PUT",&format!("/api/ab/tag/update/{guid}"),Some(json!({"name":"renamed","color":0xff112233u32}))).await).await;
    let tags: Vec<Tag> = serde_json::from_value(value(call(&app,&token,"POST",&format!("/api/ab/tags/{guid}"),None).await).await).unwrap();
    assert!(tags.iter().any(|tag|tag.name=="renamed" && tag.color==0xff112233)); assert!(tags.iter().any(|tag|tag.name=="keep" && tag.color==u32::MAX));
    let peers = value(call(&app,&token,"POST",&format!("/api/ab/peers?ab={guid}"),None).await).await; assert_eq!(peers["data"][0]["tags"],json!(["renamed","keep"]));
    action(call(&app,&token,"DELETE",&format!("/api/ab/tag/{guid}"),Some(json!(["renamed"]))).await).await;
    let legacy: Value = serde_json::from_str(value(call(&app,&token,"GET","/api/ab",None).await).await["data"].as_str().unwrap()).unwrap();
    assert_eq!(legacy["peers"][0]["tags"],json!(["keep"])); assert_eq!(legacy["peers"][0]["hash"],"keep");
    assert_eq!(serde_json::from_str::<Value>(legacy["tag_colors"].as_str().unwrap()).unwrap(),json!({"keep":u32::MAX}));
}

#[tokio::test]
async fn every_guid_route_rejects_foreign_books_and_web_entry_ids_have_separate_routes() {
    let (app,token,guid) = fixture().await;
    assert_eq!(app.send(request("POST","/api/register",json!({"username":"other","password":"other-password"}))).await.status(),StatusCode::CREATED);
    let other = value(app.send(request("POST","/api/login",json!({"username":"other","password":"other-password"}))).await).await["access_token"].as_str().unwrap().to_owned();
    for (method,path,body) in [
        ("POST",format!("/api/ab/peers?ab={guid}"),None),("POST",format!("/api/ab/tags/{guid}"),None),
        ("POST",format!("/api/ab/peer/add/{guid}"),Some(json!({"id":"123456"}))),
        ("PUT",format!("/api/ab/peer/update/{guid}"),Some(json!({"id":"123456","alias":"attack"}))),
        ("DELETE",format!("/api/ab/peer/{guid}"),Some(json!(["123456"]))),
        ("POST",format!("/api/ab/tag/add/{guid}"),Some(json!({"name":"evil","color":0}))),
        ("PUT",format!("/api/ab/tag/rename/{guid}"),Some(json!({"old":"ops","new":"evil"}))),
        ("PUT",format!("/api/ab/tag/update/{guid}"),Some(json!({"name":"ops","color":0}))),
        ("DELETE",format!("/api/ab/tag/{guid}"),Some(json!(["ops"])))
    ] { assert_eq!(call(&app,&other,method,&path,body).await.status(),StatusCode::NOT_FOUND,"{path}"); }
    let saved = call(&app,&token,"POST","/api/web/ab/entries",Some(json!({"peer_id":"123456","alias":"Web"}))).await; assert_eq!(saved.status(),StatusCode::OK);
    let saved = value(saved).await; let entry = saved["data"]["id"].as_str().unwrap();
    assert_eq!(call(&app,&token,"DELETE",&format!("/api/ab/peer/{entry}"),Some(json!(["123456"]))).await.status(),StatusCode::NOT_FOUND);
    assert_eq!(call(&app,&token,"DELETE",&format!("/api/web/ab/entries/{entry}"),None).await.status(),StatusCode::OK);
}

#[tokio::test]
async fn official_pagination_reads_all_101_peers_and_legacy_replace_is_visible_to_new_protocol() {
    let (app,token,guid) = fixture().await;
    let peers = (0..101).rev().map(|i|json!({"id":format!("{i:06}"),"alias":format!("Peer {i}"),"hash":"keep"})).collect::<Vec<_>>();
    action(call(&app,&token,"POST","/api/ab",Some(json!({"data":json!({"peers":peers}).to_string()}))).await).await;
    let first: Page<Peer> = serde_json::from_value(value(call(&app,&token,"POST",&format!("/api/ab/peers?ab={guid}&current=1&pageSize=100"),None).await).await).unwrap();
    let second: Page<Peer> = serde_json::from_value(value(call(&app,&token,"POST",&format!("/api/ab/peers?ab={guid}&current=2&pageSize=100"),None).await).await).unwrap();
    assert_eq!(first.total,101); assert_eq!(first.data.len(),100); assert_eq!(second.total,101); assert_eq!(second.data.len(),1); assert_eq!(second.data[0].id,"000100"); assert_eq!(first.data[0].alias,"Peer 0");
    assert!(!first.data.iter().any(|peer|peer.id==second.data[0].id));
    for query in ["current=0","pageSize=101","current=bad","pageSize=-1"] { assert_eq!(call(&app,&token,"POST",&format!("/api/ab/peers?ab={guid}&{query}"),None).await.status(),StatusCode::BAD_REQUEST); }
    let beyond: Page<Peer> = serde_json::from_value(value(call(&app,&token,"POST",&format!("/api/ab/peers?ab={guid}&current=3"),None).await).await).unwrap(); assert_eq!(beyond.total,101); assert!(beyond.data.is_empty());
    action(call(&app,&token,"PUT",&format!("/api/ab/peer/update/{guid}"),Some(json!({"id":"000000","alias":"Native edit"}))).await).await;
    let legacy: Value = serde_json::from_str(value(call(&app,&token,"GET","/api/ab",None).await).await["data"].as_str().unwrap()).unwrap();
    assert_eq!(legacy["peers"].as_array().unwrap().len(),101); assert_eq!(legacy["peers"].as_array().unwrap().iter().find(|peer|peer["id"]=="000000").unwrap()["alias"],"Native edit");
}

#[tokio::test]
async fn batch_peer_and_tag_deletion_are_atomic_on_bad_input_and_storage_failure() {
    let (app,token,guid) = fixture().await;
    for id in ["123456","654321"] { action(call(&app,&token,"POST",&format!("/api/ab/peer/add/{guid}"),Some(json!({"id":id}))).await).await; }
    for name in ["one","two"] { action(call(&app,&token,"POST",&format!("/api/ab/tag/add/{guid}"),Some(json!({"name":name,"color":u32::MAX}))).await).await; }
    let before = value(call(&app,&token,"GET","/api/ab",None).await).await;
    for (path,body) in [(format!("/api/ab/peer/{guid}"),json!(["123456",42])),(format!("/api/ab/tag/{guid}"),json!(["one",null]))] { assert_eq!(call(&app,&token,"DELETE",&path,Some(body)).await.status(),StatusCode::BAD_REQUEST); }
    let pool = sqlx::SqlitePool::connect(app.database_path().to_str().unwrap()).await.unwrap();
    sqlx::query("create trigger reject_native_index before insert on api_address_book_entry begin select raise(abort,'test-only failure'); end").execute(&pool).await.unwrap();
    assert_eq!(call(&app,&token,"DELETE",&format!("/api/ab/peer/{guid}"),Some(json!(["123456"]))).await.status(),StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(call(&app,&token,"DELETE",&format!("/api/ab/tag/{guid}"),Some(json!(["one","two"]))).await.status(),StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(value(call(&app,&token,"GET","/api/ab",None).await).await,before); pool.close().await;
}

#[tokio::test]
async fn pinned_official_request_samples_replay_and_legacy_colors_survive_both_protocols() {
    let samples: Value = serde_json::from_str(include_str!("fixtures/rustdesk-1.4.9-address-book.json")).unwrap();
    assert_eq!(samples["commit"],"6c578292e8ebbbec708b76986ba8c4bc7c509747");
    let (app,token,guid) = fixture().await;
    for sample in samples["sequence"].as_array().unwrap() {
        let path = sample["path"].as_str().unwrap().replace("$guid",&guid);
        let body = if sample["body"].is_null() { None } else { Some(sample["body"].clone()) };
        let response = call(&app,&token,sample["method"].as_str().unwrap(),&path,body).await;
        assert_eq!(response.status(),StatusCode::OK,"{path}");
        match sample["response"].as_str().unwrap() {
            "empty" => action(response).await,
            "guid" => assert_eq!(serde_json::from_value::<Personal>(value(response).await).unwrap().guid,guid),
            "settings" => assert_eq!(serde_json::from_value::<Settings>(value(response).await).unwrap().max_peer_one_ab,0),
            "empty_page" => { let page: Page<Value> = serde_json::from_value(value(response).await).unwrap(); assert_eq!(page.total,0); assert!(page.data.is_empty()); },
            "empty_tags" => assert!(serde_json::from_value::<Vec<Tag>>(value(response).await).unwrap().is_empty()),
            "peer_page" => { let page = value(response).await; let parsed: Page<Peer> = serde_json::from_value(page.clone()).unwrap(); assert_eq!(parsed.total,1); assert_eq!(parsed.data[0].alias,"Renamed"); assert_eq!(page["data"][0]["hash"],"test-only-hash"); assert_eq!(page["data"][0]["tags"],json!(["work"])); },
            "tag_array" => { let tags: Vec<Tag> = serde_json::from_value(value(response).await).unwrap(); assert_eq!(tags.len(),1); assert_eq!(tags[0].name,"work"); assert_eq!(tags[0].color,0x12345678); },
            other => panic!("unknown sample assertion {other}"),
        }
    }
    action(call(&app,&token,"POST","/api/ab",Some(json!({"data":samples["legacy"].to_string()}))).await).await;
    let tags: Vec<Tag> = serde_json::from_value(value(call(&app,&token,"POST",&format!("/api/ab/tags/{guid}"),None).await).await).unwrap(); assert_eq!(tags[0].color,u32::MAX);
    let page: Page<Peer> = serde_json::from_value(value(call(&app,&token,"POST",&format!("/api/ab/peers?ab={guid}"),None).await).await).unwrap(); assert_eq!(page.data[0].relay,"true");
}

#[tokio::test]
async fn native_invalid_connection_types_and_reserved_identity_edits_do_not_modify_the_book() {
    let (app,token,guid) = fixture().await;
    let before = value(call(&app,&token,"GET","/api/ab",None).await).await;
    for patch in [json!({"hash":42}),json!({"password":{}}),json!({"rdpPort":3389}),json!({"same_server":"true"}),json!({"entryId":"foreign"})] {
        let mut peer = json!({"id":"123456"}); peer.as_object_mut().unwrap().extend(patch.as_object().unwrap().clone());
        assert_eq!(call(&app,&token,"POST",&format!("/api/ab/peer/add/{guid}"),Some(peer)).await.status(),StatusCode::BAD_REQUEST);
    }
    for color in [json!(-1),json!(4294967296u64),json!("#ffffff")] { assert_eq!(call(&app,&token,"POST",&format!("/api/ab/tag/add/{guid}"),Some(json!({"name":"bad","color":color}))).await.status(),StatusCode::BAD_REQUEST); }
    assert_eq!(value(call(&app,&token,"GET","/api/ab",None).await).await,before);
}
