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
    let data = json!({"peers":[{"id":"123456","alias":"First","password":"first-secret"},{"id":"654321","alias":"Second","extension":{"preserved":true}}]});
    assert_eq!(auth(&app,"POST","/api/ab",json!({"data":data.to_string()}),&token).await.status(),StatusCode::OK);
    (app,token)
}

#[tokio::test]
async fn editing_one_imported_peer_keeps_all_other_document_entries_visible() {
    let (app,token) = fixture().await;
    assert_eq!(auth(&app,"POST","/api/web/ab/entries",json!({"peer_id":"123456","alias":"Edited"}),&token).await.status(),StatusCode::OK);
    let entries = value(auth(&app,"GET","/api/web/ab/entries",json!({}),&token).await).await;
    assert_eq!(entries["data"].as_array().unwrap().len(),2);
    let second = entries["data"].as_array().unwrap().iter().find(|entry|entry["peerId"]=="654321").unwrap();
    assert_eq!(second["extension"],json!({"preserved":true}));
}

#[tokio::test]
async fn address_book_guid_revision_and_entry_ids_are_stable_across_api_restart() {
    let (mut app,token) = fixture().await;
    let first = value(auth(&app,"GET","/api/ab",json!({}),&token).await).await;
    assert!(first["guid"].as_str().is_some_and(|guid|uuid::Uuid::parse_str(guid).is_ok()));
    assert!(first["revision"].as_u64().is_some());
    let entries = value(auth(&app,"GET","/api/web/ab/entries",json!({}),&token).await).await;
    app.reopen(OAuthRuntime::new(Vec::new()),Some(hbbs::oauth_admin::ProviderSecretKey::from_bytes(&[7;32]).unwrap())).await.unwrap();
    assert_eq!(value(auth(&app,"GET","/api/ab",json!({}),&token).await).await,first);
    assert_eq!(value(auth(&app,"GET","/api/web/ab/entries",json!({}),&token).await).await,entries);
}

#[tokio::test]
async fn stale_revision_cannot_overwrite_a_newer_peer_edit() {
    let (app,token) = fixture().await;
    let first = value(auth(&app,"GET","/api/ab",json!({}),&token).await).await;
    let revision = first["revision"].as_u64().expect("a persistent revision is required");
    assert_eq!(auth(&app,"POST","/api/web/ab/entries",json!({"peer_id":"123456","alias":"Newer","revision":revision}),&token).await.status(),StatusCode::OK);
    assert_eq!(auth(&app,"POST","/api/web/ab/entries",json!({"peer_id":"654321","alias":"Stale","revision":revision}),&token).await.status(),StatusCode::CONFLICT);
    let current = value(auth(&app,"GET","/api/ab",json!({}),&token).await).await;
    assert_eq!(current["guid"],first["guid"]); assert!(current["revision"].as_u64().unwrap()>revision);
    let document: Value = serde_json::from_str(current["data"].as_str().unwrap()).unwrap();
    assert_eq!(document["peers"][0]["alias"],"Newer"); assert_eq!(document["peers"][1]["alias"],"Second");
}

#[tokio::test]
async fn store_document_is_authoritative_and_cas_and_index_rebuild_are_transactional() {
    use hbbs::{address_book_store::{AddressBookStore,BookError},database::Database};
    let (app,_) = fixture().await;
    let db = Database::new(app.database_path().to_str().unwrap()).await.unwrap();
    let pool = sqlx::SqlitePool::connect(app.database_path().to_str().unwrap()).await.unwrap();
    let owner: String = sqlx::query_scalar("select id from api_user where username='admin'").fetch_one(&pool).await.unwrap();
    let store = AddressBookStore::new(db);
    let first = store.load(&owner).await.unwrap();
    let mut edited = first.document.clone(); edited["peers"][0]["alias"] = json!("Updated");
    let second = store.replace(&owner,Some(first.revision),edited).await.unwrap();
    assert_eq!(second.guid,first.guid); assert_eq!(second.revision,first.revision+1); assert_eq!(second.document["peers"].as_array().unwrap().len(),2);
    assert!(matches!(store.replace(&owner,Some(first.revision),first.document.clone()).await,Err(BookError::Conflict)));
    sqlx::query("delete from api_address_book_entry where user_id=?").bind(&owner).execute(&pool).await.unwrap();
    assert_eq!(store.load(&owner).await.unwrap().document,second.document);
    store.rebuild(&owner).await.unwrap();
    assert_eq!(sqlx::query_scalar::<_,i64>("select count(*) from api_address_book_entry where user_id=?").bind(&owner).fetch_one(&pool).await.unwrap(),2);
    assert_eq!(store.load(&owner).await.unwrap().revision,second.revision);
    sqlx::query("create trigger reject_store_snapshot before update on api_address_book_snapshot begin select raise(abort,'test-only failure'); end").execute(&pool).await.unwrap();
    assert!(matches!(store.replace(&owner,Some(second.revision),first.document).await,Err(BookError::Storage)));
    assert_eq!(store.load(&owner).await.unwrap().document,second.document); pool.close().await;
}

async fn legacy_schema(pool: &sqlx::SqlitePool) {
    sqlx::query("alter table api_address_book_snapshot rename to book_before_downgrade;
        create table api_address_book_snapshot(user_id text primary key not null,data text not null,updated_at datetime not null default current_timestamp,
            foreign key(user_id) references api_user(id) on delete cascade);
        insert into api_address_book_snapshot(user_id,data,updated_at) select user_id,data,updated_at from book_before_downgrade;
        drop table book_before_downgrade;
        delete from api_schema_migration where version=3").execute(pool).await.unwrap();
}

#[tokio::test]
async fn historical_migration_unions_snapshot_and_index_preserves_extensions_and_can_repeat() {
    use hbbs::{address_book_store::AddressBookStore,database::Database};
    let (app,_) = fixture().await;
    let pool = sqlx::SqlitePool::connect(app.database_path().to_str().unwrap()).await.unwrap();
    let owner: String = sqlx::query_scalar("select id from api_user where username='admin'").fetch_one(&pool).await.unwrap();
    let historical = json!({"peers":[{"id":"123456","alias":"First","password":"first-secret"},{"id":"654321","alias":"Second","extension":{"preserved":true}}]});
    sqlx::query("update api_address_book_snapshot set data=?,updated_at='2000-01-01 00:00:00'").bind(historical.to_string()).execute(&pool).await.unwrap();
    sqlx::query("delete from api_address_book_entry").execute(&pool).await.unwrap();
    sqlx::query("insert into api_address_book_entry(id,user_id,peer_id,alias,updated_at) values('index-first',?,'123456','Newer index','2099-01-01 00:00:00')").bind(&owner).execute(&pool).await.unwrap();
    sqlx::query("insert into api_address_book_entry(id,user_id,peer_id,alias,updated_at) values('index-only',?,'777777','Only index','2099-01-01 00:00:00')").bind(&owner).execute(&pool).await.unwrap();
    legacy_schema(&pool).await;
    let db = Database::new(app.database_path().to_str().unwrap()).await.unwrap();
    let store = AddressBookStore::new(db); let book = store.load(&owner).await.unwrap();
    let peers = book.document["peers"].as_array().unwrap(); assert_eq!(peers.len(),3);
    let first = peers.iter().find(|peer|peer["id"]=="123456").unwrap(); assert_eq!(first["alias"],"Newer index"); assert_eq!(first["password"],"first-secret"); assert_eq!(first["entryId"],"index-first");
    let second = peers.iter().find(|peer|peer["id"]=="654321").unwrap(); assert_eq!(second["extension"],json!({"preserved":true}));
    let reopened = AddressBookStore::new(Database::new(app.database_path().to_str().unwrap()).await.unwrap()).load(&owner).await.unwrap();
    assert_eq!(reopened.guid,book.guid); assert_eq!(reopened.revision,book.revision); assert_eq!(reopened.document,book.document); pool.close().await;
}

#[tokio::test]
async fn historical_duplicate_identifiers_roll_back_the_entire_migration_and_report_conflict() {
    use hbbs::database::Database;
    let (app,_) = fixture().await;
    let pool = sqlx::SqlitePool::connect(app.database_path().to_str().unwrap()).await.unwrap();
    let damaged = json!({"peers":[{"id":"123456","alias":"First"},{"id":"123456","alias":"Conflicting"}]}).to_string();
    sqlx::query("update api_address_book_snapshot set data=?").bind(&damaged).execute(&pool).await.unwrap(); legacy_schema(&pool).await;
    let error = match Database::new(app.database_path().to_str().unwrap()).await { Ok(_) => panic!("duplicate migration must fail"), Err(error) => error };
    assert!(error.to_string().contains("duplicate_peer_id:123456"));
    assert_eq!(sqlx::query_scalar::<_,String>("select data from api_address_book_snapshot").fetch_one(&pool).await.unwrap(),damaged);
    assert_eq!(sqlx::query_scalar::<_,i64>("select count(*) from api_schema_migration where version=3").fetch_one(&pool).await.unwrap(),0);
    assert_eq!(sqlx::query_scalar::<_,i64>("select count(*) from pragma_table_info('api_address_book_snapshot') where name in ('guid','revision')").fetch_one(&pool).await.unwrap(),0); pool.close().await;
}

#[tokio::test]
async fn contradictory_identifier_aliases_are_rejected_without_changing_the_document() {
    let (app,token) = fixture().await;
    let before = value(auth(&app,"GET","/api/ab",json!({}),&token).await).await;
    for peer in [json!({"id":"123456","peerId":"654321"}),json!({"id":"123456","entryId":"one","guid":"two"})] {
        let response = auth(&app,"POST","/api/ab",json!({"data":json!({"peers":[peer]}).to_string()}),&token).await;
        assert_eq!(response.status(),StatusCode::BAD_REQUEST);
        assert!(value(response).await["error"].as_str().unwrap().contains("identity_alias_conflict"));
        assert_eq!(value(auth(&app,"GET","/api/ab",json!({}),&token).await).await,before);
    }
}

#[tokio::test]
async fn independent_store_writers_have_one_winner_and_index_failure_rolls_back_revision() {
    use hbbs::{address_book_store::{AddressBookStore,BookError},database::Database};
    let (app,_) = fixture().await;
    let pool = sqlx::SqlitePool::connect(app.database_path().to_str().unwrap()).await.unwrap();
    let owner: String = sqlx::query_scalar("select id from api_user where username='admin'").fetch_one(&pool).await.unwrap();
    let one = AddressBookStore::new(Database::new(app.database_path().to_str().unwrap()).await.unwrap());
    let two = AddressBookStore::new(Database::new(app.database_path().to_str().unwrap()).await.unwrap());
    let original = one.load(&owner).await.unwrap();
    let mut left = original.document.clone(); left["peers"][0]["alias"] = json!("Left");
    let mut right = original.document.clone(); right["peers"][0]["alias"] = json!("Right");
    let (left,right) = tokio::join!(one.replace(&owner,Some(original.revision),left),two.replace(&owner,Some(original.revision),right));
    assert_eq!(usize::from(left.is_ok())+usize::from(right.is_ok()),1);
    assert!(matches!(left,Err(BookError::Conflict)) || matches!(right,Err(BookError::Conflict)));
    let saved = one.load(&owner).await.unwrap();
    sqlx::query("create trigger reject_index before insert on api_address_book_entry begin select raise(abort,'test-only index failure'); end").execute(&pool).await.unwrap();
    assert!(matches!(one.replace(&owner,Some(saved.revision),original.document).await,Err(BookError::Storage)));
    let after = two.load(&owner).await.unwrap(); assert_eq!(after.document,saved.document); assert_eq!(after.revision,saved.revision);
    assert_eq!(sqlx::query_scalar::<_,i64>("select count(*) from api_address_book_entry").fetch_one(&pool).await.unwrap(),2); pool.close().await;
}

#[tokio::test]
async fn browser_mutations_require_revision_and_foreign_imports_get_local_entry_identity() {
    let (app,token) = fixture().await;
    let response = app.send(request("POST","/api/login",json!({"username":"admin","password":"admin-password"}))).await;
    let session = cookie(&response);
    let mut csrf = request("GET","/api/session/csrf",json!({})); csrf.headers_mut().insert(header::COOKIE,session.parse().unwrap());
    let csrf = value(app.send_raw(csrf).await).await;
    let before = value(auth(&app,"GET","/api/ab",json!({}),&token).await).await;
    let mut update = request("POST","/api/web/ab/entries",json!({"peer_id":"123456","alias":"No revision"}));
    update.headers_mut().insert(header::COOKIE,session.parse().unwrap());
    update.headers_mut().insert(header::ORIGIN,"https://api.example".parse().unwrap());
    update.headers_mut().insert("x-csrf-token",csrf["csrf_token"].as_str().unwrap().parse().unwrap());
    assert_eq!(app.send_raw(update).await.status(),StatusCode::CONFLICT);
    assert_eq!(value(auth(&app,"GET","/api/ab",json!({}),&token).await).await,before);
    let registered = app.send(request("POST","/api/register",json!({"username":"other","password":"other-password"}))).await;
    assert_eq!(registered.status(),StatusCode::CREATED);
    let other = value(app.send(request("POST","/api/login",json!({"username":"other","password":"other-password"}))).await).await["access_token"].as_str().unwrap().to_owned();
    assert_eq!(auth(&app,"POST","/api/ab",json!({"data":before["data"]}),&other).await.status(),StatusCode::OK);
    let imported = value(auth(&app,"GET","/api/ab",json!({}),&other).await).await;
    let original: Value = serde_json::from_str(before["data"].as_str().unwrap()).unwrap();
    let local: Value = serde_json::from_str(imported["data"].as_str().unwrap()).unwrap();
    assert_ne!(local["peers"][0]["entryId"],original["peers"][0]["entryId"]);
    assert_eq!(local["peers"][0]["password"],original["peers"][0]["password"]);
}

#[tokio::test]
async fn equal_and_older_index_times_keep_snapshot_values_and_parallel_migration_is_idempotent() {
    use hbbs::{address_book_store::AddressBookStore,database::Database};
    let (app,_) = fixture().await;
    let pool = sqlx::SqlitePool::connect(app.database_path().to_str().unwrap()).await.unwrap();
    let owner: String = sqlx::query_scalar("select id from api_user where username='admin'").fetch_one(&pool).await.unwrap();
    sqlx::query("update api_address_book_snapshot set data=?,updated_at='2020-01-01 00:00:00'")
        .bind(json!({"peers":[{"id":"123456","alias":"Snapshot one","password":"keep"},{"id":"654321","alias":"Snapshot two"}]}).to_string()).execute(&pool).await.unwrap();
    sqlx::query("update api_address_book_entry set alias='Index',updated_at=case peer_id when '123456' then '2020-01-01T00:00:00Z' else '2019-01-01 00:00:00' end").execute(&pool).await.unwrap();
    legacy_schema(&pool).await;
    let path = app.database_path(); let path = path.to_str().unwrap();
    let (one,two) = tokio::join!(Database::new(path),Database::new(path));
    let one = AddressBookStore::new(one.unwrap()).load(&owner).await.unwrap();
    let two = AddressBookStore::new(two.unwrap()).load(&owner).await.unwrap();
    assert_eq!(one.guid,two.guid); assert_eq!(one.document,two.document);
    assert_eq!(one.document["peers"][0]["alias"],"Snapshot one"); assert_eq!(one.document["peers"][1]["alias"],"Snapshot two");
    assert_eq!(one.document["peers"][0]["password"],"keep"); pool.close().await;
}

#[tokio::test]
async fn corrupt_historical_documents_and_identity_conflicts_rollback_with_reports() {
    use hbbs::database::Database;
    for (document,report) in [
        ("{broken".to_owned(),"invalid_address_book_document"),
        (json!({"peers":[{"id":"123456","alias":42}]}).to_string(),"invalid_peer_field"),
        (json!({"peers":[{"id":"123456","entryId":"not-index-id"}]}).to_string(),"entry_identity_conflict"),
        (json!({"peers":[{"id":"123456","entryId":"same"},{"id":"654321","entryId":"same"}]}).to_string(),"duplicate_or_invalid_entry_id"),
        (json!({"tag_colors":"invalid"}).to_string(),"invalid_tag_colors"),
        (json!({"peers":[{"id":"123456","updatedAt":"broken-time"}]}).to_string(),"invalid_address_book_timestamp"),
    ] {
        let (app,_) = fixture().await;
        let pool = sqlx::SqlitePool::connect(app.database_path().to_str().unwrap()).await.unwrap();
        sqlx::query("update api_address_book_snapshot set data=?").bind(&document).execute(&pool).await.unwrap(); legacy_schema(&pool).await;
        let error = match Database::new(app.database_path().to_str().unwrap()).await { Ok(_) => panic!("damaged migration must fail"), Err(error) => error };
        assert!(error.to_string().contains(report),"{error}, expected {report}");
        assert_eq!(sqlx::query_scalar::<_,String>("select data from api_address_book_snapshot").fetch_one(&pool).await.unwrap(),document);
        assert_eq!(sqlx::query_scalar::<_,i64>("select count(*) from api_schema_migration where version=3").fetch_one(&pool).await.unwrap(),0); pool.close().await;
    }
}

#[tokio::test]
async fn concurrent_empty_book_initialization_keeps_one_persistent_guid() {
    use hbbs::{address_book_store::AddressBookStore,database::Database};
    let app = TestApp::new(CookiePolicy::default(),OAuthRuntime::new(Vec::new())).await;
    let pool = sqlx::SqlitePool::connect(app.database_path().to_str().unwrap()).await.unwrap();
    let owner: String = sqlx::query_scalar("select id from api_user where username='admin'").fetch_one(&pool).await.unwrap();
    let one = AddressBookStore::new(Database::new(app.database_path().to_str().unwrap()).await.unwrap());
    let two = AddressBookStore::new(Database::new(app.database_path().to_str().unwrap()).await.unwrap());
    let (one,two) = tokio::join!(one.load(&owner),two.load(&owner)); let (one,two) = (one.unwrap(),two.unwrap());
    assert_eq!(one.guid,two.guid); assert_eq!(one.revision,1); assert_eq!(one.document,two.document);
    assert_eq!(one.document["peers"],json!([])); assert_eq!(one.document["tags"],json!([])); pool.close().await;
}
