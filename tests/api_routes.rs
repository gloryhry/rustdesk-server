use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use hbbs::{
    api::{build_service, PublicServerConfig},
    database::Database,
    browser_security::CookiePolicy,
    ldap::LdapConfig,
    oauth::OAuthRuntime,
};
use hyper::body::to_bytes;
use std::{fs, path::PathBuf, time::Duration};
use tower::ServiceExt;
use uuid::Uuid;

async fn test_app() -> (axum::Router, PathBuf) {
    let database_path = std::env::temp_dir().join(format!(
        "rustdesk-api-routes-{}.sqlite3",
        Uuid::new_v4()
    ));
    let database = Database::new(
        database_path
            .to_str()
            .expect("database path should be UTF-8"),
    )
    .await
    .expect("test database should initialize");
    let router = build_service(
        database,
        "01234567890123456789012345678901".to_owned(),
        Duration::from_secs(3600),
        true,
        PublicServerConfig {
            api_server: "http://127.0.0.1:21114".to_owned(),
            id_server: String::new(),
            relay_server: String::new(),
            key: String::new(),
        },
        std::env::temp_dir().to_string_lossy().into_owned(),
        None,
        OAuthRuntime::new(Vec::new()),
        String::new(),
        LdapConfig::disabled(),
        CookiePolicy::default(),
        None,
        None,
    )
    .await
    .expect("test router should initialize");
    (router, database_path)
}

async fn send(app: &axum::Router, request: Request<Body>) -> axum::response::Response {
    app.clone()
        .oneshot(request)
        .await
        .expect("router should respond")
}

fn json_request(method: &str, path: &str, body: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(path)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_owned()))
        .expect("test request should build")
}

fn authenticated_json_request(
    method: &str,
    path: &str,
    body: &str,
    cookie: &str,
    csrf: &str,
) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(path)
        .header(header::COOKIE, cookie)
        .header(header::ORIGIN, "http://127.0.0.1:21114")
        .header("x-csrf-token", csrf)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_owned()))
        .expect("authenticated request should build")
}

fn cookie_from(response: &axum::response::Response) -> String {
    response
        .headers()
        .get(header::SET_COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .map(str::to_owned)
        .expect("login should set a session cookie")
}

#[tokio::test]
async fn api_routes_support_auth_groups_and_cookie_sessions() {
    let (app, database_path) = test_app().await;
    Database::new(database_path.to_str().unwrap()).await.unwrap()
        .insert_peer("client-a",b"uuid-a",&[3;32],"{}").await.unwrap();

    let health = send(
        &app,
        Request::builder()
            .uri("/health/live")
            .body(Body::empty())
            .expect("health request should build"),
    )
    .await;
    assert_eq!(health.status(), StatusCode::OK);

    let unauthorized = send(
        &app,
        Request::builder()
            .uri("/api/currentUser")
            .body(Body::empty())
            .expect("unauthorized request should build"),
    )
    .await;
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

    let api_root = send(
        &app,
        Request::builder()
            .uri("/api/")
            .body(Body::empty())
            .expect("api root request should build"),
    )
    .await;
    assert_eq!(api_root.status(), StatusCode::OK);
    let api_root_body = to_bytes(api_root.into_body())
        .await
        .expect("api root response should read");
    assert!(String::from_utf8_lossy(&api_root_body).contains("RustDesk API"));

    let version = send(
        &app,
        Request::builder()
            .uri("/api/version")
            .body(Body::empty())
            .expect("version request should build"),
    )
    .await;
    assert_eq!(version.status(), StatusCode::OK);

    let heartbeat = send(
        &app,
        json_request("POST","/api/heartbeat",r#"{"id":"client-a","uuid":"dXVpZC1h","ver":1004009}"#),
    )
    .await;
    assert_eq!(heartbeat.status(), StatusCode::OK);

    let registration = send(
        &app,
        json_request(
            "POST",
            "/api/register",
            r#"{"username":"route-user","email":"route@example.com","password":"route-password"}"#,
        ),
    )
    .await;
    assert_eq!(registration.status(), StatusCode::CREATED);
    let registration_body = to_bytes(registration.into_body())
        .await
        .expect("registration response should read");
    let user_id = serde_json::from_slice::<serde_json::Value>(&registration_body)
        .expect("registration response should be JSON")["id"]
        .as_str()
        .expect("registration response should contain an id")
        .to_owned();

    let login = send(
        &app,
        json_request(
            "POST",
            "/api/login",
            r#"{"username":"route-user","password":"route-password","id":"web","uuid":"route-test"}"#,
        ),
    )
    .await;
    assert_eq!(login.status(), StatusCode::OK);
    let cookie = cookie_from(&login);
    let csrf_response = send(&app, Request::builder().uri("/api/session/csrf")
        .header(header::COOKIE,&cookie).body(Body::empty()).unwrap()).await;
    assert_eq!(csrf_response.status(),StatusCode::OK);
    let csrf: serde_json::Value = serde_json::from_slice(&to_bytes(csrf_response.into_body()).await.unwrap()).unwrap();
    let csrf = csrf["csrf_token"].as_str().unwrap();
    assert!(login
        .headers()
        .get(header::SET_COOKIE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.contains("HttpOnly") && value.contains("SameSite=Lax") && value.contains("; Secure")));

    let sysinfo = send(
        &app,
        authenticated_json_request(
            "POST",
            "/api/sysinfo",
            r#"{"id":"client-a","uuid":"dXVpZC1h","hostname":"Office laptop","os":"Linux"}"#,
            &cookie,
            csrf,
        ),
    )
    .await;
    assert_eq!(sysinfo.status(), StatusCode::OK);

    let peers_list = send(
        &app,
        Request::builder()
            .uri("/api/peers")
            .header(header::COOKIE, &cookie)
            .body(Body::empty())
            .expect("peers list request should build"),
    )
    .await;
    assert_eq!(peers_list.status(), StatusCode::OK);
    let peers_list_body = to_bytes(peers_list.into_body())
        .await
        .expect("peers list response should read");
    let peers: serde_json::Value = serde_json::from_slice(&peers_list_body).unwrap();
    assert_eq!(peers["data"],serde_json::json!([]), "unsigned reports never grant account ownership");

    let current_user = send(
        &app,
        Request::builder()
            .uri("/api/currentUser")
            .header(header::COOKIE, &cookie)
            .body(Body::empty())
            .expect("cookie request should build"),
    )
    .await;
    assert_eq!(current_user.status(), StatusCode::OK);
    let current_user_body = to_bytes(current_user.into_body())
        .await
        .expect("current user response should read");
    assert!(String::from_utf8_lossy(&current_user_body).contains("route-user"));

    let book = send(&app,Request::builder().uri("/api/ab").header(header::COOKIE,&cookie).body(Body::empty()).unwrap()).await;
    let book: serde_json::Value = serde_json::from_slice(&to_bytes(book.into_body()).await.unwrap()).unwrap();
    let revision = book["revision"].as_i64().unwrap();
    let peer_upsert = send(
        &app,
        authenticated_json_request(
            "POST",
            "/api/web/ab/entries",
            &serde_json::json!({"peer_id":"peer-42","username":"alice","hostname":"office","alias":"Office","platform":"Linux","tags":["ops"],"force_always_relay":true,"revision":revision}).to_string(),
            &cookie,
            csrf,
        ),
    )
    .await;
    assert_eq!(peer_upsert.status(), StatusCode::OK);
    let peer_saved: serde_json::Value = serde_json::from_slice(&to_bytes(peer_upsert.into_body()).await.unwrap()).unwrap();

    let peers = send(
        &app,
        Request::builder()
            .uri("/api/web/ab/entries")
            .header(header::COOKIE, &cookie)
            .body(Body::empty())
            .expect("peers request should build"),
    )
    .await;
    assert_eq!(peers.status(), StatusCode::OK);
    let peers_body = to_bytes(peers.into_body())
        .await
        .expect("peers response should read");
    let peers_json = serde_json::from_slice::<serde_json::Value>(&peers_body)
        .expect("peers response should be JSON");
    assert_eq!(peers_json["data"][0]["peerId"], "peer-42");
    assert_eq!(peers_json["data"][0]["forceAlwaysRelay"], true);

    let peer_delete = send(
        &app,
        Request::builder()
            .method("DELETE")
            .uri(format!("/api/web/ab/entries/peer-42?revision={}",peer_saved["revision"].as_i64().unwrap()))
            .header(header::COOKIE, &cookie)
            .header(header::ORIGIN, "http://127.0.0.1:21114")
            .header("x-csrf-token", csrf)
            .body(Body::empty())
            .expect("peer delete request should build"),
    )
    .await;
    assert_eq!(peer_delete.status(), StatusCode::OK);

    let forbidden_device_delete = send(
        &app,
        authenticated_json_request(
            "POST",
            "/api/admin/device/delete",
            r#"{"id":"missing-device"}"#,
            &cookie,
            csrf,
        ),
    )
    .await;
    assert_eq!(forbidden_device_delete.status(), StatusCode::FORBIDDEN);

    let group_create = send(
        &app,
        authenticated_json_request(
            "POST",
            "/api/groups",
            r#"{"name":"route-group"}"#,
            &cookie,
            csrf,
        ),
    )
    .await;
    assert_eq!(group_create.status(), StatusCode::CREATED);
    let group_body = to_bytes(group_create.into_body())
        .await
        .expect("group response should read");
    let group_id = serde_json::from_slice::<serde_json::Value>(&group_body)
        .expect("group response should be JSON")["id"]
        .as_str()
        .expect("group response should contain an id")
        .to_owned();

    let add_member = send(
        &app,
        authenticated_json_request(
            "POST",
            "/api/groups/members",
            &format!(r#"{{"group_id":"{group_id}","user_id":"{user_id}"}}"#),
            &cookie,
            csrf,
        ),
    )
    .await;
    assert_eq!(add_member.status(), StatusCode::OK);

    let groups = send(
        &app,
        Request::builder()
            .uri("/api/groups")
            .header(header::COOKIE, &cookie)
            .body(Body::empty())
            .expect("groups request should build"),
    )
    .await;
    assert_eq!(groups.status(), StatusCode::OK);
    let groups_body = to_bytes(groups.into_body())
        .await
        .expect("groups response should read");
    let groups_json = serde_json::from_slice::<serde_json::Value>(&groups_body)
        .expect("groups response should be JSON");
    assert_eq!(groups_json["data"][0]["name"], "route-group");
    assert_eq!(groups_json["memberships"][0]["user_id"], user_id);

    let remove_member = send(
        &app,
        authenticated_json_request(
            "POST",
            "/api/groups/members/delete",
            &format!(r#"{{"group_id":"{group_id}","user_id":"{user_id}"}}"#),
            &cookie,
            csrf,
        ),
    )
    .await;
    assert_eq!(remove_member.status(), StatusCode::OK);

    let delete_group = send(
        &app,
        authenticated_json_request(
            "POST",
            "/api/groups/delete",
            &format!(r#"{{"id":"{group_id}"}}"#),
            &cookie,
            csrf,
        ),
    )
    .await;
    assert_eq!(delete_group.status(), StatusCode::OK);

    let logout = send(
        &app,
        authenticated_json_request("POST", "/api/logout", "", &cookie, csrf),
    )
    .await;
    assert_eq!(logout.status(), StatusCode::OK);
    assert!(logout
        .headers()
        .get(header::SET_COOKIE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.contains("Max-Age=0") && value.contains("; Secure")));

    let revoked = send(
        &app,
        Request::builder()
            .uri("/api/currentUser")
            .header(header::COOKIE, &cookie)
            .body(Body::empty())
            .expect("revoked request should build"),
    )
    .await;
    assert_eq!(revoked.status(), StatusCode::UNAUTHORIZED);

    fs::remove_file(database_path).expect("test database should be removable");
}
