mod common;
use common::*;
use axum::http::{header,StatusCode};
use hbbs::{api::CookiePolicy,oauth::OAuthRuntime};
use serde_json::{json,Value};

async fn value(response: axum::response::Response) -> Value { serde_json::from_slice(&axum::body::to_bytes(response.into_body(), 4*1024*1024).await.unwrap()).unwrap() }
async fn admin(app: &TestApp) -> String { value(app.send(request("POST","/api/admin/login",json!({"username":"admin","password":"admin-password"}))).await).await["access_token"].as_str().unwrap().to_owned() }
async fn get(app: &TestApp, path: &str, token: &str) -> axum::response::Response {
    let mut request = request("GET",path,json!({}));request.headers_mut().insert(header::AUTHORIZATION,format!("Bearer {token}").parse().unwrap());app.send(request).await
}

#[tokio::test]
async fn official_lists_include_total_for_native_pagination() {
    let app = TestApp::new(CookiePolicy::default(),OAuthRuntime::new(Vec::new())).await; let token = admin(&app).await;
    for (path,total) in [("/api/users?current=1&pageSize=100&accessible=&status=1",1),("/api/peers?current=1&pageSize=100&accessible=&status=1",0),("/api/device-group/accessible?current=1&pageSize=100",0)] {
        let response = get(&app,path,&token).await; assert_eq!(response.status(),StatusCode::OK);
        assert_eq!(value(response).await["total"],total);
    }
}

async fn seed(app: &TestApp, count: usize, owner: &str) {
    let pool = sqlx::SqlitePool::connect(app.database_path().to_str().unwrap()).await.unwrap();
    let mut tx = pool.begin().await.unwrap();
    for index in 0..count {
        let id = format!("item-{index:03}");
        sqlx::query("insert into api_user(id,username,password_hash) values(?,?,'not-used')").bind(&id).bind(&id).execute(&mut *tx).await.unwrap();
        let guid = id.as_bytes();
        sqlx::query("insert into peer(guid,id,uuid,pk,info) values(?,?,?,?,'{}')").bind(guid).bind(&id).bind(guid).bind(guid).execute(&mut *tx).await.unwrap();
        sqlx::query("insert into api_device(id,user_id,uuid,name,peer_guid,verified,verified_uuid,verified_pk) values(?,?,?,?,?,1,?,?)")
            .bind(&id).bind(owner).bind(&id).bind(&id).bind(guid).bind(guid).bind(guid).execute(&mut *tx).await.unwrap();
        sqlx::query("insert into api_device_group(id,name,created_by) values(?,?,?)").bind(&id).bind(&id).bind(owner).execute(&mut *tx).await.unwrap();
    }
    tx.commit().await.unwrap(); pool.close().await;
}

#[tokio::test]
async fn boundary_counts_default_pages_and_stable_order_match_the_official_loop() {
    for count in [0,1,100,101] {
        let app = TestApp::new(CookiePolicy::default(),OAuthRuntime::new(Vec::new())).await; let token = admin(&app).await;
        let owner = value(get(&app,"/api/users",&token).await).await["data"][0]["id"].as_str().unwrap().to_owned();
        seed(&app,count,&owner).await;
        for path in ["/api/users","/api/peers","/api/device-group/accessible"] {
            let response = get(&app,&format!("{path}?name=item-"),&token).await;
            assert_eq!(response.status(),StatusCode::OK,"{path}");
            let first = value(response).await;
            assert_eq!(first["total"],count); assert_eq!(first["data"].as_array().unwrap().len(),count.min(100));
            let second = value(get(&app,&format!("{path}?name=item-&current=2&pageSize=100"),&token).await).await;
            assert_eq!(second["total"],count); assert_eq!(second["data"].as_array().unwrap().len(),count.saturating_sub(100));
            let mut ids = first["data"].as_array().unwrap().iter().chain(second["data"].as_array().unwrap()).map(|row|row["id"].as_str().unwrap()).collect::<Vec<_>>();
            assert_eq!(ids,(0..count).map(|index|format!("item-{index:03}")).collect::<Vec<_>>());
            ids.sort(); ids.dedup(); assert_eq!(ids.len(),count);
            let beyond = value(get(&app,&format!("{path}?current=1000"),&token).await).await;
            assert_eq!(beyond["data"],json!([]));
        }
    }
}

#[tokio::test]
async fn invalid_parameters_are_explicit_errors() {
    let app = TestApp::new(CookiePolicy::default(),OAuthRuntime::new(Vec::new())).await; let token = admin(&app).await;
    for path in ["/api/users","/api/peers","/api/device-group/accessible"] {
        for query in ["current=0","current=-1","current=x","current=18446744073709551615","pageSize=0","pageSize=101","pageSize=x","status=2","status=x"] {
            assert_eq!(get(&app,&format!("{path}?{query}"),&token).await.status(),StatusCode::BAD_REQUEST,"{path}?{query}");
        }
    }
    assert_eq!(get(&app,"/api/device-group/accessible?status=1",&token).await.status(),StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn totals_apply_authorization_status_and_literal_name_filters_first() {
    let app = TestApp::new(CookiePolicy::default(),OAuthRuntime::new(Vec::new())).await; let token = admin(&app).await;
    let owner = value(get(&app,"/api/users",&token).await).await["data"][0]["id"].as_str().unwrap().to_owned();
    seed(&app,3,&owner).await;
    let user = value(app.send(request("POST","/api/register",json!({"username":"isolated","password":"temporary-password"}))).await).await["id"].as_str().unwrap().to_owned();
    let own_token = value(app.send(request("POST","/api/login",json!({"username":"isolated","password":"temporary-password"}))).await).await["access_token"].as_str().unwrap().to_owned();
    let pool = sqlx::SqlitePool::connect(app.database_path().to_str().unwrap()).await.unwrap();
    sqlx::query("update api_user set status=0 where id='item-001'").execute(&pool).await.unwrap();
    sqlx::query("update api_device set status=0 where id='item-001'").execute(&pool).await.unwrap();
    sqlx::query("update api_device set user_id=? where id='item-002'").bind(&user).execute(&pool).await.unwrap();
    sqlx::query("update api_device_group set created_by=? where id='item-002'").bind(&user).execute(&pool).await.unwrap();
    for path in ["/api/users","/api/peers"] {
        let result = value(get(&app,&format!("{path}?name=item-&status=1&pageSize=1"),&token).await).await;
        assert_eq!(result["total"],2); assert_eq!(result["data"].as_array().unwrap().len(),1);
    }
    for (path,total) in [("/api/users",1),("/api/peers",1),("/api/device-group/accessible",1)] {
        let result = value(get(&app,path,&own_token).await).await; assert_eq!(result["total"],total); assert!(!result.to_string().contains("item-000"));
    }
    sqlx::query("update api_user set username='literal%_name' where id='item-000'").execute(&pool).await.unwrap();
    sqlx::query("update api_device set name='literal%_name' where id='item-000'").execute(&pool).await.unwrap();
    sqlx::query("update api_device_group set name='literal%_name' where id='item-000'").execute(&pool).await.unwrap();
    for path in ["/api/users","/api/peers","/api/device-group/accessible"] {
        let result = value(get(&app,&format!("{path}?name=%25_"),&token).await).await;
        assert_eq!(result["total"],1,"{path}: {result}"); assert_eq!(result["data"][0]["id"],"item-000");
    }
    pool.close().await;
}
