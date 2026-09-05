use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use hbbs::{
    api::{build_service, PublicServerConfig},
    database::Database,
    ldap::LdapConfig,
    oauth::OAuthRuntime,
};
use hyper::body::to_bytes;
use std::{fs, path::PathBuf, time::Duration};
use tower::ServiceExt;
use uuid::Uuid;

async fn test_app() -> (axum::Router, PathBuf) {
    let database_path = std::env::temp_dir().join(format!("rustdesk-api-routes-{}.sqlite3", Uuid::new_v4()));
    let database = Database::new(database_path.to_str().expect("database path should be UTF-8"))
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
    )
    .await
    .expect("test router should initialize");
    (router, database_path)
}

async fn send(
    app: &axum::Router,
    request: Request<Body>,
) -> axum::response::Response {
    app.clone().oneshot(request).await.expect("router should respond")
}

fn json_request(method: &str, path: &str, body: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(path)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_owned()))
        .expect("test request should build")
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
async fn api_routes_support_cookie_sessions_and_compatibility_health() {
    let (app, database_path) = test_app().await;

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

    let registration = send(
        &app,
        json_request(
            "POST",
            "/api/register",
            r#"{"username":"route-user","email":"route@example.com","password":"route-password"}"#,
        ),
    )
    .await;
    assert!(matches!(
        registration.status(),
        StatusCode::OK | StatusCode::CREATED
    ));

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
    assert!(login
        .headers()
        .get(header::SET_COOKIE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.contains("HttpOnly") && value.contains("SameSite=Lax")));

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

    let logout = send(
        &app,
        Request::builder()
            .method("POST")
            .uri("/api/logout")
            .header(header::COOKIE, &cookie)
            .body(Body::empty())
            .expect("logout request should build"),
    )
    .await;
    assert_eq!(logout.status(), StatusCode::OK);
    assert!(logout
        .headers()
        .get(header::SET_COOKIE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.contains("Max-Age=0")));

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
