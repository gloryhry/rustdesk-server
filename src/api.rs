use crate::auth::{AuthError, AuthService, LoginDevice, Principal};
use axum::{
    extract::{Extension, Json},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub struct ApiState {
    pub auth: AuthService,
    pub registration_enabled: bool,
    pub server_config: PublicServerConfig,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PublicServerConfig {
    pub api_server: String,
    pub id_server: String,
    pub relay_server: String,
    pub key: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub uuid: String,
    #[serde(default, rename = "autoLogin")]
    pub auto_login: bool,
    #[serde(default, rename = "deviceInfo")]
    pub device_info: Option<DeviceInfoRequest>,
}

#[derive(Debug, Default, Deserialize)]
pub struct DeviceInfoRequest {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub os: String,
    #[serde(default, rename = "type")]
    pub device_type: String,
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct AddressBookRequest {
    pub data: String,
}

pub fn build_router(
    auth: AuthService,
    registration_enabled: bool,
    server_config: PublicServerConfig,
) -> Router {
    let state = Arc::new(ApiState {
        auth,
        registration_enabled,
        server_config,
    });
    Router::new()
        .route("/health/live", get(health_live))
        .route("/api/", get(api_index))
        .route("/api/version", get(api_version))
        .route("/api/heartbeat", post(heartbeat))
        .route("/api/sysinfo", post(sysinfo))
        .route("/api/sysinfo_ver", post(sysinfo_version))
        .route("/api/login", post(login))
        .route("/api/admin/login", post(login))
        .route("/api/login-options", get(login_options))
        .route("/api/register", post(register))
        .route("/api/admin/user/register", post(register))
        .route("/api/currentUser", get(current_user).post(current_user))
        .route("/api/admin/user/current", get(current_user))
        .route("/api/logout", post(logout))
        .route("/api/admin/logout", post(logout))
        .route("/api/ab", get(get_address_book).post(update_address_book))
        .route("/api/server-config", post(server_config))
        .route("/api/server-config-v2", post(server_config))
        .layer(Extension(state))
}

pub async fn build_service(
    db: crate::database::Database,
    secret: String,
    token_ttl: Duration,
    registration_enabled: bool,
    server_config: PublicServerConfig,
) -> Result<Router, AuthError> {
    let auth = AuthService::new(db, secret, token_ttl)?;
    Ok(build_router(auth, registration_enabled, server_config))
}

async fn health_live() -> impl IntoResponse {
    Json(json!({ "status": "ok" }))
}

async fn api_index() -> impl IntoResponse {
    Json(json!({ "code": 0, "data": "RustDesk API" }))
}

async fn api_version() -> impl IntoResponse {
    Json(json!({ "code": 0, "data": env!("CARGO_PKG_VERSION") }))
}

async fn heartbeat() -> impl IntoResponse {
    Json(json!({}))
}

async fn sysinfo() -> impl IntoResponse {
    (StatusCode::OK, "SYSINFO_UPDATED")
}

async fn sysinfo_version() -> impl IntoResponse {
    (StatusCode::OK, env!("CARGO_PKG_VERSION"))
}

async fn login(
    Extension(state): Extension<Arc<ApiState>>,
    Json(request): Json<LoginRequest>,
) -> Response {
    let device_info = request.device_info.unwrap_or_default();
    let device = LoginDevice {
        id: request.id,
        uuid: request.uuid,
        name: device_info.name,
        os: device_info.os,
        device_type: device_info.device_type,
    };
    match state
        .auth
        .login(&request.username, &request.password, device)
        .await
    {
        Ok(result) => (StatusCode::OK, Json(result)).into_response(),
        Err(err) => auth_error_response(err, true),
    }
}

async fn register(
    Extension(state): Extension<Arc<ApiState>>,
    Json(request): Json<RegisterRequest>,
) -> Response {
    if !state.registration_enabled {
        return (StatusCode::NOT_FOUND, Json(json!({
            "error": "registration_disabled"
        })))
            .into_response();
    }
    let is_admin = match state.auth.db().api_user_count().await {
        Ok(count) => count == 0,
        Err(_) => return auth_error_response(AuthError::Internal, false),
    };
    match state
        .auth
        .register(
            &request.username,
            &request.email,
            &request.password,
            is_admin,
        )
        .await
    {
        Ok(user) => (StatusCode::CREATED, Json(user)).into_response(),
        Err(err) => auth_error_response(err, false),
    }
}

async fn current_user(
    Extension(state): Extension<Arc<ApiState>>,
    headers: HeaderMap,
) -> Response {
    match authorize(&state, &headers).await {
        Ok(principal) => (StatusCode::OK, Json(principal.user)).into_response(),
        Err(err) => auth_error_response(err, true),
    }
}

async fn logout(
    Extension(state): Extension<Arc<ApiState>>,
    headers: HeaderMap,
) -> Response {
    match authorize(&state, &headers).await {
        Ok(principal) => match state
            .auth
            .revoke_session(&principal.user_id, &principal.session_id)
            .await
        {
            Ok(()) => (StatusCode::OK, Json(serde_json::Value::Null)).into_response(),
            Err(err) => auth_error_response(err, false),
        },
        Err(err) => auth_error_response(err, true),
    }
}

async fn server_config(
    Extension(state): Extension<Arc<ApiState>>,
    headers: HeaderMap,
) -> Response {
    if let Err(err) = authorize(&state, &headers).await {
        return auth_error_response(err, true);
    }
    (
        StatusCode::OK,
        Json(json!({
            "code": 0,
            "data": state.server_config.clone()
        })),
    )
        .into_response()
}

async fn get_address_book(
    Extension(state): Extension<Arc<ApiState>>,
    headers: HeaderMap,
) -> Response {
    let principal = match authorize(&state, &headers).await {
        Ok(principal) => principal,
        Err(err) => return auth_error_response(err, true),
    };
    match state.auth.db().get_api_address_book(&principal.user_id).await {
        Ok(Some(data)) => (StatusCode::OK, Json(json!({ "data": data }))).into_response(),
        Ok(None) => (
            StatusCode::OK,
            Json(json!({
                "data": "{\"peers\":[],\"tags\":[],\"tag_colors\":\"{}\"}"
            })),
        )
            .into_response(),
        Err(_) => auth_error_response(AuthError::Internal, false),
    }
}

async fn update_address_book(
    Extension(state): Extension<Arc<ApiState>>,
    headers: HeaderMap,
    Json(request): Json<AddressBookRequest>,
) -> Response {
    let principal = match authorize(&state, &headers).await {
        Ok(principal) => principal,
        Err(err) => return auth_error_response(err, true),
    };
    if request.data.len() > 2 * 1024 * 1024 {
        return (
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(json!({ "error": "address_book_too_large" })),
        )
            .into_response();
    }
    let document = match serde_json::from_str::<serde_json::Value>(&request.data) {
        Ok(serde_json::Value::Object(_)) => request.data,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "invalid_address_book" })),
            )
                .into_response()
        }
    };
    match state
        .auth
        .db()
        .upsert_api_address_book(&principal.user_id, &document)
        .await
    {
        Ok(()) => (StatusCode::OK, Json(serde_json::Value::Null)).into_response(),
        Err(_) => auth_error_response(AuthError::Internal, false),
    }
}

async fn authorize(state: &ApiState, headers: &HeaderMap) -> Result<Principal, AuthError> {
    let value = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or(AuthError::InvalidCredentials)?;
    let token = value
        .strip_prefix("Bearer ")
        .ok_or(AuthError::InvalidCredentials)?;
    if token.is_empty() {
        return Err(AuthError::InvalidCredentials);
    }
    state.auth.authorize(token).await
}

async fn login_options() -> impl IntoResponse {
    Json(Vec::<String>::new())
}

fn auth_error_response(error: AuthError, unauthorized: bool) -> Response {
    match error {
        AuthError::InvalidCredentials if unauthorized => (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "unauthorized" })),
        )
            .into_response(),
        AuthError::InvalidCredentials => (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "invalid_credentials" })),
        )
            .into_response(),
        AuthError::UsernameUnavailable => (
            StatusCode::CONFLICT,
            Json(json!({ "error": "username_unavailable" })),
        )
            .into_response(),
        AuthError::InvalidInput(message) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": message })),
        )
            .into_response(),
        AuthError::Internal => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "internal_error" })),
        )
            .into_response(),
    }
}
