mod common;
use common::*;
use axum::http::{header,StatusCode};
use hbbs::{api::CookiePolicy,oauth::OAuthRuntime};
use serde::Deserialize;
use serde_json::{json,Value};

async fn value(response: axum::response::Response) -> Value { serde_json::from_slice(&axum::body::to_bytes(response.into_body(), 4*1024*1024).await.unwrap()).unwrap() }
async fn auth(app: &TestApp, method: &str, path: &str, body: Value, token: &str) -> axum::response::Response {
    let mut request = request(method,path,body); request.headers_mut().insert(header::AUTHORIZATION,format!("Bearer {token}").parse().unwrap()); app.send(request).await
}
#[derive(Deserialize)]
struct OfficialUser { id: String, name: String }

#[tokio::test]
async fn ordinary_user_list_has_stable_id_for_authorized_group_membership() {
    let app = TestApp::new(CookiePolicy::default(),OAuthRuntime::new(Vec::new())).await;
    let id = value(app.send(request("POST","/api/register",json!({"username":"ordinary","password":"ordinary-password"}))).await).await["id"].as_str().unwrap().to_owned();
    let token = value(app.send(request("POST","/api/login",json!({"username":"ordinary","password":"ordinary-password"}))).await).await["access_token"].as_str().unwrap().to_owned();
    let other = value(app.send(request("POST","/api/register",json!({"username":"unrelated","password":"unrelated-password"}))).await).await["id"].as_str().unwrap().to_owned();
    let list = value(auth(&app,"GET","/api/users?current=1&pageSize=100&status=1",json!({}),&token).await).await;
    assert_eq!(list["total"],1);
    let user: OfficialUser = serde_json::from_value(list["data"][0].clone()).unwrap();
    assert_eq!(user.id,id); assert_eq!(user.name,"ordinary"); assert!(!list.to_string().contains("unrelated"));
    let group = value(auth(&app,"POST","/api/groups",json!({"name":"Own group"}),&token).await).await;
    assert_eq!(auth(&app,"POST","/api/groups/members",json!({"group_id":group["id"],"user_id":user.id}),&token).await.status(),StatusCode::OK);
    let other_token = value(app.send(request("POST","/api/login",json!({"username":"unrelated","password":"unrelated-password"}))).await).await["access_token"].as_str().unwrap().to_owned();
    let foreign = value(auth(&app,"POST","/api/groups",json!({"name":"Foreign group"}),&other_token).await).await;
    assert_eq!(auth(&app,"POST","/api/groups/members",json!({"group_id":foreign["id"],"user_id":other}),&token).await.status(),StatusCode::NOT_FOUND);
    assert_eq!(auth(&app,"POST","/api/groups/members",json!({"group_id":group["id"],"user_id":"ordinary"}),&token).await.status(),StatusCode::NOT_FOUND);
    let memberships = value(auth(&app,"GET","/api/groups",json!({}),&token).await).await;
    assert_eq!(memberships["memberships"][0]["user_id"],id);
    let second = value(auth(&app,"GET","/api/users?current=2",json!({}),&token).await).await;
    assert_eq!(second["total"],1); assert_eq!(second["data"],json!([]));
}
