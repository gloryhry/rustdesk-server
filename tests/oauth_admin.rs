mod common;
use common::*;
use axum::{body::Body, http::{header, Request, StatusCode}};
use hbbs::{api::CookiePolicy, oauth::OAuthRuntime};
use serde_json::{json, Value};
use hbbs::oauth_admin::ProviderSecretKey;

async fn admin_cookie(app: &TestApp) -> String {
    cookie(&app.send(request("POST", "/api/admin/login", json!({"username":"admin","password":"admin-password"}))).await)
}

fn provider_request(provider: &MockOAuth, name: &str) -> Value {
    json!({"name":name,"kind":"oauth2","client_id":"client","client_secret":"provider-test-secret",
        "authorization_url":format!("{}/authorize",provider.base),"token_url":format!("{}/token",provider.base),
        "userinfo_url":format!("{}/userinfo",provider.base),"scopes":"read:user","enabled":true})
}

async fn post(app: &TestApp, path: &str, value: Value, cookie: &str) -> axum::response::Response {
    let mut request = request("POST", path, value);
    request.headers_mut().insert(header::COOKIE, cookie.parse().unwrap());
    app.send(request).await
}

#[tokio::test]
async fn administrator_can_create_an_oauth_provider() {
    let provider = MockOAuth::start();
    let app = TestApp::new(CookiePolicy::default(), OAuthRuntime::new(Vec::new())).await;
    let cookie = admin_cookie(&app).await;
    let response = post(&app, "/api/admin/oauth/providers", provider_request(&provider, "local"), &cookie).await;
    assert_eq!(response.status(), StatusCode::CREATED);
}

async fn value(response: axum::response::Response) -> Value {
    serde_json::from_slice(&axum::body::to_bytes(response.into_body(), 4*1024*1024).await.unwrap()).unwrap()
}

async fn get(app: &TestApp, path: &str, cookie: &str) -> axum::response::Response {
    app.send(Request::builder().uri(path).header(header::COOKIE, cookie).body(Body::empty()).unwrap()).await
}

#[tokio::test]
async fn crud_is_persistent_and_secrets_are_encrypted_and_never_returned() {
    let provider = MockOAuth::start();
    let mut app = TestApp::new(CookiePolicy::default(), OAuthRuntime::new(Vec::new())).await;
    let cookie = admin_cookie(&app).await;
    let created = value(post(&app, "/api/admin/oauth/providers", provider_request(&provider, "local"), &cookie).await).await;
    let id = created["id"].as_str().unwrap().to_owned();
    assert!(created.get("client_secret").is_none()); assert_eq!(created["secret_configured"], true);
    let options = value(app.send(Request::builder().uri("/api/login-options").body(Body::empty()).unwrap()).await).await;
    assert!(options.to_string().contains("oidc/local"));
    let connection = sqlx::SqlitePool::connect(app.database_path().to_str().unwrap()).await.unwrap();
    let (config, encrypted): (String, String) = sqlx::query_as("select config,encrypted_secret from api_oauth_provider where id=?")
        .bind(&id).fetch_one(&connection).await.unwrap();
    assert!(!config.contains("provider-test-secret")); assert!(!encrypted.contains("provider-test-secret"));
    assert!(base64::decode(&encrypted).unwrap().len() > 24);
    let mut update = provider_request(&provider, "local");
    update["id"] = json!(id); update["client_secret"] = json!("  "); update["scopes"] = json!("read:user user:email");
    assert_eq!(post(&app,"/api/admin/oauth/providers/update",update,&cookie).await.status(), StatusCode::OK);
    let encrypted_after: String = sqlx::query_scalar("select encrypted_secret from api_oauth_provider where id=?").bind(&id).fetch_one(&connection).await.unwrap();
    assert_ne!(encrypted_after, encrypted, "secret must be resealed with a fresh nonce");
    app.reopen(OAuthRuntime::new(Vec::new()),Some(ProviderSecretKey::from_bytes(&[7;32]).unwrap())).await.unwrap();
    let listed = value(get(&app,"/api/admin/oauth/providers",&cookie).await).await;
    assert_eq!(listed["data"][0]["id"], id); assert_eq!(listed["data"][0]["scopes"], "read:user user:email");
    assert!(!listed.to_string().contains("provider-test-secret"));
    assert_eq!(post(&app,"/api/admin/oauth/providers/toggle",json!({"id":id,"enabled":false}),&cookie).await.status(), StatusCode::OK);
    let begin = get(&app,"/api/oidc/login?provider=local",&cookie).await;
    assert_eq!(begin.status(),StatusCode::NOT_FOUND);
    assert_eq!(post(&app,"/api/admin/oauth/providers/toggle",json!({"id":id,"enabled":true}),&cookie).await.status(), StatusCode::OK);
    assert_eq!(get(&app,"/api/oidc/login?provider=local",&cookie).await.status(),StatusCode::TEMPORARY_REDIRECT);
    assert_eq!(post(&app,"/api/admin/oauth/providers/delete",json!({"id":id}),&cookie).await.status(), StatusCode::OK);
    assert_eq!(value(get(&app,"/api/admin/oauth/providers",&cookie).await).await["data"],json!([]));
    assert_eq!(post(&app,"/api/admin/oauth/providers",provider_request(&provider,"local"),&cookie).await.status(), StatusCode::CONFLICT);
    connection.close().await;
}

#[tokio::test]
async fn ordinary_users_cannot_access_any_provider_management_operation() {
    let provider = MockOAuth::start();
    let app = TestApp::new(CookiePolicy::default(), OAuthRuntime::new(Vec::new())).await;
    let registered = app.send(request("POST","/api/register",json!({"username":"ordinary","password":"ordinary-password"}))).await;
    assert_eq!(registered.status(),StatusCode::CREATED);
    let cookie = cookie(&app.send(request("POST","/api/login",json!({"username":"ordinary","password":"ordinary-password"}))).await);
    assert_eq!(get(&app,"/api/admin/oauth/providers",&cookie).await.status(),StatusCode::FORBIDDEN);
    for (path, body) in [
        ("/api/admin/oauth/providers",provider_request(&provider,"local")),
        ("/api/admin/oauth/providers/update",provider_request(&provider,"local")),
        ("/api/admin/oauth/providers/toggle",json!({"id":"missing","enabled":false})),
        ("/api/admin/oauth/providers/delete",json!({"id":"missing"})),
    ] { assert_eq!(post(&app,path,body,&cookie).await.status(),StatusCode::FORBIDDEN); }
}

#[tokio::test]
async fn environment_providers_are_read_only_and_names_cannot_be_shadowed() {
    let provider = MockOAuth::start();
    let app = TestApp::new(CookiePolicy::default(),provider.runtime()).await;
    let cookie = admin_cookie(&app).await;
    let listed = value(get(&app,"/api/admin/oauth/providers",&cookie).await).await;
    let row = &listed["data"][0];
    assert_eq!(row["read_only"],true); assert_eq!(row["source"],"environment");
    assert_eq!(post(&app,"/api/admin/oauth/providers",provider_request(&provider,"test"),&cookie).await.status(),StatusCode::CONFLICT);
    for (path, body) in [
        ("/api/admin/oauth/providers/toggle",json!({"id":row["id"],"enabled":false})),
        ("/api/admin/oauth/providers/delete",json!({"id":row["id"]})),
    ] { assert_eq!(post(&app,path,body,&cookie).await.status(),StatusCode::FORBIDDEN); }
    let mut update = provider_request(&provider,"test"); update["id"] = row["id"].clone();
    assert_eq!(post(&app,"/api/admin/oauth/providers/update",update,&cookie).await.status(),StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn identity_authority_is_immutable_and_duplicate_names_are_rejected() {
    let provider = MockOAuth::start();
    let app = TestApp::new(CookiePolicy::default(),OAuthRuntime::new(Vec::new())).await;
    let cookie = admin_cookie(&app).await;
    let created = value(post(&app,"/api/admin/oauth/providers",provider_request(&provider,"local"),&cookie).await).await;
    assert_eq!(post(&app,"/api/admin/oauth/providers",provider_request(&provider,"local"),&cookie).await.status(),StatusCode::CONFLICT);
    for field in ["name","client_id","authorization_url","token_url","userinfo_url","issuer_url","jwks_url"] {
        let mut update = provider_request(&provider,"local"); update["id"] = created["id"].clone();
        update[field] = json!("https://other.example/authority");
        assert_eq!(post(&app,"/api/admin/oauth/providers/update",update,&cookie).await.status(),StatusCode::BAD_REQUEST,"changed {field}");
    }
    let listed = value(get(&app,"/api/admin/oauth/providers",&cookie).await).await;
    assert_eq!(listed["data"][0]["id"],created["id"]);
}

#[tokio::test]
async fn missing_or_wrong_encryption_key_never_falls_back_to_plaintext() {
    let provider = MockOAuth::start();
    let app = TestApp::new_with_key(CookiePolicy::default(),OAuthRuntime::new(Vec::new()),None).await;
    let cookie = admin_cookie(&app).await;
    assert_eq!(post(&app,"/api/admin/oauth/providers",provider_request(&provider,"local"),&cookie).await.status(),StatusCode::SERVICE_UNAVAILABLE);
    assert!(ProviderSecretKey::from_bytes(&[0;31]).is_err());
    assert!(ProviderSecretKey::from_base64("not-base64").is_err());
    let mut app = TestApp::new(CookiePolicy::default(),OAuthRuntime::new(Vec::new())).await;
    let cookie = admin_cookie(&app).await;
    assert_eq!(post(&app,"/api/admin/oauth/providers",provider_request(&provider,"local"),&cookie).await.status(),StatusCode::CREATED);
    assert!(app.reopen(OAuthRuntime::new(Vec::new()),None).await.is_err());
    assert!(app.reopen(OAuthRuntime::new(Vec::new()),Some(ProviderSecretKey::from_bytes(&[8;32]).unwrap())).await.is_err());
    assert!(app.reopen(OAuthRuntime::new(Vec::new()),Some(ProviderSecretKey::from_bytes(b"01234567890123456789012345678901").unwrap())).await.is_err());
}

#[tokio::test]
async fn disabling_a_provider_invalidates_existing_browser_authorizations() {
    let provider = MockOAuth::start();
    let app = TestApp::new(CookiePolicy::default(),OAuthRuntime::new(Vec::new())).await;
    let admin = admin_cookie(&app).await;
    let created = value(post(&app,"/api/admin/oauth/providers",provider_request(&provider,"local"),&admin).await).await;
    let begin = get(&app,"/api/oidc/login?provider=local",&admin).await;
    let binding = cookie(&begin);
    let url = reqwest::Url::parse(begin.headers().get(header::LOCATION).unwrap().to_str().unwrap()).unwrap();
    let state = url.query_pairs().find(|(key,_)| key == "state").unwrap().1.into_owned();
    assert_eq!(post(&app,"/api/admin/oauth/providers/toggle",json!({"id":created["id"],"enabled":false}),&admin).await.status(),StatusCode::OK);
    let callback = get(&app,&format!("/api/oidc/callback?code=code&state={state}"),&binding).await;
    assert_eq!(callback.status(),StatusCode::BAD_REQUEST);
    assert!(callback.headers().get(header::SET_COOKIE).is_none());
}

async fn begin_browser(app: &TestApp, path: &str) -> (String, String) {
    let response = get(app, path, "").await;
    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
    let binding = cookie(&response);
    let url = reqwest::Url::parse(response.headers()[header::LOCATION].to_str().unwrap()).unwrap();
    let state = url.query_pairs().find(|(key,_)| key == "state").unwrap().1.into_owned();
    (state, binding)
}

async fn native_issue(app: &TestApp) -> Value {
    value(app.send(request("POST","/api/oidc/auth",json!({"op":"local","id":"123456","uuid":"uuid"}))).await).await
}

async fn native_open(app: &TestApp, flow: &Value) -> (String, String) {
    let url = reqwest::Url::parse(flow["url"].as_str().unwrap()).unwrap();
    begin_browser(app,&format!("{}?{}",url.path(),url.query().unwrap())).await
}

async fn complete_browser(app: &TestApp, state: &str, binding: &str) -> axum::response::Response {
    get(app,&format!("/api/oidc/callback?state={state}&code=code"),binding).await
}

async fn native_poll(app: &TestApp, flow: &Value) -> Value {
    value(get(app,&format!("/api/oidc/auth-query?code={}&id=123456&uuid=uuid",flow["code"].as_str().unwrap()),"").await).await
}

#[tokio::test]
async fn secret_rotation_and_restart_keep_the_same_account_identity() {
    let provider = MockOAuth::start();
    let mut app = TestApp::new(CookiePolicy::default(),OAuthRuntime::new(Vec::new())).await;
    let admin = admin_cookie(&app).await;
    let created = value(post(&app,"/api/admin/oauth/providers",provider_request(&provider,"local"),&admin).await).await;
    let (state,binding) = begin_browser(&app,"/api/oidc/login?provider=local").await;
    let first = value(complete_browser(&app,&state,&binding).await).await;
    assert!(first.get("access_token").is_some());
    assert_eq!(provider.token_requests.lock().unwrap()[0]["client_secret"],"provider-test-secret");
    let mut update = provider_request(&provider,"local"); update["id"] = created["id"].clone(); update["client_secret"] = json!("rotated-secret");
    assert_eq!(post(&app,"/api/admin/oauth/providers/update",update,&admin).await.status(),StatusCode::OK);
    app.reopen(OAuthRuntime::new(Vec::new()),Some(ProviderSecretKey::from_bytes(&[7;32]).unwrap())).await.unwrap();
    let (state,binding) = begin_browser(&app,"/api/oidc/login?provider=local").await;
    let second = value(complete_browser(&app,&state,&binding).await).await;
    let subject = |login: &Value| {
        let payload = login["access_token"].as_str().unwrap().split('.').nth(1).unwrap();
        let claims: Value = serde_json::from_slice(&base64::decode_config(payload,base64::URL_SAFE_NO_PAD).unwrap()).unwrap();
        claims["sub"].as_str().unwrap().to_owned()
    };
    assert_eq!(subject(&second),subject(&first));
    assert_eq!(provider.token_requests.lock().unwrap()[1]["client_secret"],"rotated-secret");
    let pool = sqlx::SqlitePool::connect(app.database_path().to_str().unwrap()).await.unwrap();
    let count: i64 = sqlx::query_scalar("select count(*) from api_identity where subject='42'").fetch_one(&pool).await.unwrap();
    assert_eq!(count,1); pool.close().await;
}

#[tokio::test]
async fn disabling_a_provider_invalidates_waiting_authorizing_and_unclaimed_native_flows() {
    let provider = MockOAuth::start();
    let app = TestApp::new(CookiePolicy::default(),OAuthRuntime::new(Vec::new())).await;
    let admin = admin_cookie(&app).await;
    let created = value(post(&app,"/api/admin/oauth/providers",provider_request(&provider,"local"),&admin).await).await;
    let waiting = native_issue(&app).await;
    let authorizing = native_issue(&app).await;
    let (old_state,old_binding) = native_open(&app,&authorizing).await;
    let ready = native_issue(&app).await;
    let (state,binding) = native_open(&app,&ready).await;
    assert_eq!(complete_browser(&app,&state,&binding).await.status(),StatusCode::OK);
    assert_eq!(post(&app,"/api/admin/oauth/providers/toggle",json!({"id":created["id"],"enabled":false}),&admin).await.status(),StatusCode::OK);
    for flow in [&waiting,&authorizing,&ready] {
        assert_eq!(native_poll(&app,flow).await["error"],"oauth_provider_changed");
    }
    assert_eq!(complete_browser(&app,&old_state,&old_binding).await.status(),StatusCode::BAD_REQUEST);
    assert_eq!(post(&app,"/api/admin/oauth/providers/toggle",json!({"id":created["id"],"enabled":true}),&admin).await.status(),StatusCode::OK);
    let new = native_issue(&app).await;
    assert_eq!(native_poll(&app,&new).await["error"],"No authed oidc is found");
    assert_eq!(post(&app,"/api/admin/oauth/providers/delete",json!({"id":created["id"]}),&admin).await.status(),StatusCode::OK);
    assert_eq!(native_poll(&app,&new).await["error"],"oauth_provider_changed");
}

#[tokio::test]
async fn editing_during_token_exchange_cannot_complete_an_old_authorization() {
    let provider = MockOAuth::start();
    let app = TestApp::new(CookiePolicy::default(),OAuthRuntime::new(Vec::new())).await;
    let admin = admin_cookie(&app).await;
    let created = value(post(&app,"/api/admin/oauth/providers",provider_request(&provider,"local"),&admin).await).await;
    let flow = native_issue(&app).await;
    let (state,binding) = native_open(&app,&flow).await;
    provider.pause_tokens.store(true,std::sync::atomic::Ordering::SeqCst);
    let callback = complete_browser(&app,&state,&binding);
    let mutate = async {
        tokio::time::timeout(std::time::Duration::from_secs(5),provider.token_entered.acquire()).await.unwrap().unwrap().forget();
        let mut update = provider_request(&provider,"local"); update["id"] = created["id"].clone(); update["client_secret"] = json!("rotated-secret");
        let response = post(&app,"/api/admin/oauth/providers/update",update,&admin).await;
        provider.token_release.add_permits(1);
        assert_eq!(response.status(),StatusCode::OK);
    };
    let (response,()) = tokio::join!(callback,mutate);
    assert_eq!(response.status(),StatusCode::BAD_REQUEST);
    assert!(response.headers().get(header::SET_COOKIE).is_none());
    assert_eq!(native_poll(&app,&flow).await["error"],"oauth_provider_changed");
    let pool = sqlx::SqlitePool::connect(app.database_path().to_str().unwrap()).await.unwrap();
    assert_eq!(sqlx::query_scalar::<_,i64>("select count(*) from api_identity").fetch_one(&pool).await.unwrap(),0);
    pool.close().await;
}

#[tokio::test]
async fn encrypted_secrets_reject_tampering_and_cross_provider_copying_before_mutation() {
    let provider = MockOAuth::start();
    let mut app = TestApp::new(CookiePolicy::default(),OAuthRuntime::new(Vec::new())).await;
    let admin = admin_cookie(&app).await;
    let one = value(post(&app,"/api/admin/oauth/providers",provider_request(&provider,"one"),&admin).await).await;
    let two = value(post(&app,"/api/admin/oauth/providers",provider_request(&provider,"two"),&admin).await).await;
    let pool = sqlx::SqlitePool::connect(app.database_path().to_str().unwrap()).await.unwrap();
    let secret: String = sqlx::query_scalar("select encrypted_secret from api_oauth_provider where id=?").bind(one["id"].as_str().unwrap()).fetch_one(&pool).await.unwrap();
    sqlx::query("update api_oauth_provider set encrypted_secret=? where id=?").bind(&secret).bind(two["id"].as_str().unwrap()).execute(&pool).await.unwrap();
    // An unrelated toggle must validate the entire prospective registry before persisting.
    assert_eq!(post(&app,"/api/admin/oauth/providers/toggle",json!({"id":one["id"],"enabled":false}),&admin).await.status(),StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(sqlx::query_scalar::<_,i64>("select enabled from api_oauth_provider where id=?").bind(one["id"].as_str().unwrap()).fetch_one(&pool).await.unwrap(),1);
    assert!(app.reopen(OAuthRuntime::new(Vec::new()),Some(ProviderSecretKey::from_bytes(&[7;32]).unwrap())).await.is_err());
    sqlx::query("update api_oauth_provider set encrypted_secret='tampered' where id=?").bind(two["id"].as_str().unwrap()).execute(&pool).await.unwrap();
    assert!(app.reopen(OAuthRuntime::new(Vec::new()),Some(ProviderSecretKey::from_bytes(&[7;32]).unwrap())).await.is_err());
    pool.close().await;
}

#[tokio::test]
async fn concurrent_migrations_reserve_historical_identity_names_and_are_repeatable() {
    let provider = MockOAuth::start();
    let mut app = TestApp::new(CookiePolicy::default(),OAuthRuntime::new(Vec::new())).await;
    let admin = admin_cookie(&app).await;
    let pool = sqlx::SqlitePool::connect(app.database_path().to_str().unwrap()).await.unwrap();
    sqlx::query("insert into api_identity(provider,subject,user_id) select 'local','42',id from api_user where username='admin'").execute(&pool).await.unwrap();
    sqlx::query("drop table api_oauth_provider").execute(&pool).await.unwrap();
    sqlx::query("delete from api_schema_migration where version=1").execute(&pool).await.unwrap();
    let path = app.database_path(); let path = path.to_str().unwrap();
    let (first,second) = tokio::join!(hbbs::database::Database::new(path),hbbs::database::Database::new(path));
    drop(first.unwrap()); drop(second.unwrap());
    app.reopen(OAuthRuntime::new(Vec::new()),Some(ProviderSecretKey::from_bytes(&[7;32]).unwrap())).await.unwrap();
    assert_eq!(post(&app,"/api/admin/oauth/providers",provider_request(&provider,"local"),&admin).await.status(),StatusCode::CONFLICT);
    let mut config = provider.config(); config.name = "local".to_owned();
    app.reopen(OAuthRuntime::new(vec![config]),Some(ProviderSecretKey::from_bytes(&[7;32]).unwrap())).await.unwrap();
    let (state,binding) = begin_browser(&app,"/api/oidc/login?provider=local").await;
    let login = value(complete_browser(&app,&state,&binding).await).await;
    assert_eq!(login["user"]["name"],"admin");
    assert_eq!(sqlx::query_scalar::<_,i64>("select count(*) from api_schema_migration where version=1").fetch_one(&pool).await.unwrap(),1);
    assert_eq!(sqlx::query_scalar::<_,i64>("select count(*) from api_identity").fetch_one(&pool).await.unwrap(),1);
    pool.close().await;
}
