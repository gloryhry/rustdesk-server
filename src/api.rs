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
use tower_http::{
    services::ServeDir,
    limit::RequestBodyLimitLayer,
    timeout::RequestTimeoutLayer,
};

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

#[derive(Debug, Deserialize)]
pub struct UserStatusRequest {
    pub id: String,
    pub status: i64,
}

#[derive(Debug, Deserialize)]
pub struct UserIdRequest {
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct PasswordChangeRequest {
    pub id: String,
    pub password: String,
}

#[derive(Debug, serde::Serialize)]
pub struct AdminUserResponse {
    pub id: String,
    pub name: String,
    pub email: String,
    pub note: String,
    pub is_admin: bool,
    pub status: i64,
    pub created_at: String,
}

pub fn build_router(
    auth: AuthService,
    registration_enabled: bool,
    server_config: PublicServerConfig,
    web_root: String,
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
        .route("/api/admin/login", post(admin_login))
        .route("/api/login-options", get(login_options))
        .route("/api/register", post(register))
        .route("/api/admin/user/register", post(register))
        .route("/api/currentUser", get(current_user).post(current_user))
        .route("/api/admin/user/current", get(admin_current_user))
        .route("/api/admin/user/list", get(admin_user_list))
        .route("/api/admin/user/create", post(admin_user_create))
        .route("/api/admin/user/update", post(admin_user_status))
        .route("/api/admin/user/changePwd", post(admin_user_password))
        .route("/api/admin/user/delete", post(admin_user_delete))
        .route("/api/logout", post(logout))
        .route("/api/admin/logout", post(admin_logout))
        .route("/api/ab", get(get_address_book).post(update_address_book))
        .route("/api/server-config", post(server_config))
        .route("/api/server-config-v2", post(server_config))
        .layer(RequestBodyLimitLayer::new(1024 * 1024))
        .layer(RequestTimeoutLayer::new(Duration::from_secs(15)))
        .layer(Extension(state))
        .fallback_service(ServeDir::new(web_root))
}

pub async fn build_service(
    db: crate::database::Database,
    secret: String,
    token_ttl: Duration,
    registration_enabled: bool,
    server_config: PublicServerConfig,
    web_root: String,
    bootstrap_admin: Option<(String, String)>,
) -> Result<Router, AuthError> {
    let auth = AuthService::new(db, secret, token_ttl)?;
    if let Some((username, password)) = bootstrap_admin {
        auth.ensure_bootstrap_admin(&username, &password).await?;
    }
    Ok(build_router(auth, registration_enabled, server_config, web_root))
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
    let (username, password, device) = login_parts(request);
    match state.auth.login(&username, &password, device).await {
        Ok(result) => (StatusCode::OK, Json(result)).into_response(),
        Err(err) => auth_error_response(err, true),
    }
}

async fn admin_login(
    Extension(state): Extension<Arc<ApiState>>,
    Json(request): Json<LoginRequest>,
) -> Response {
    let (username, password, device) = login_parts(request);
    match state.auth.login_admin(&username, &password, device).await {
        Ok(result) => (StatusCode::OK, Json(result)).into_response(),
        Err(err) => auth_error_response(err, true),
    }
}

fn login_parts(request: LoginRequest) -> (String, String, LoginDevice) {
    let device_info = request.device_info.unwrap_or_default();
    (
        request.username,
        request.password,
        LoginDevice {
            id: request.id,
            uuid: request.uuid,
            name: device_info.name,
            os: device_info.os,
            device_type: device_info.device_type,
        },
    )
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
    match state
        .auth
        .register(
            &request.username,
            &request.email,
            &request.password,
            false,
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

async fn admin_current_user(
    Extension(state): Extension<Arc<ApiState>>,
    headers: HeaderMap,
) -> Response {
    match authorize(&state, &headers).await {
        Ok(principal) if principal.user.is_admin => {
            (StatusCode::OK, Json(principal.user)).into_response()
        }
        Ok(_) => (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "admin_required" })),
        )
            .into_response(),
        Err(err) => auth_error_response(err, true),
    }
}

async fn admin_user_list(
    Extension(state): Extension<Arc<ApiState>>,
    headers: HeaderMap,
) -> Response {
    let principal = match authorize(&state, &headers).await {
        Ok(principal) => principal,
        Err(err) => return auth_error_response(err, true),
    };
    if !principal.user.is_admin {
        return admin_required_response();
    }
    match state.auth.db().list_api_users().await {
        Ok(users) => {
            let users = users.into_iter().map(admin_user_response).collect::<Vec<_>>();
            (StatusCode::OK, Json(json!({ "code": 0, "data": users }))).into_response()
        }
        Err(_) => auth_error_response(AuthError::Internal, false),
    }
}

async fn admin_user_create(
    Extension(state): Extension<Arc<ApiState>>,
    headers: HeaderMap,
    Json(request): Json<RegisterRequest>,
) -> Response {
    let principal = match authorize(&state, &headers).await {
        Ok(principal) => principal,
        Err(err) => return auth_error_response(err, true),
    };
    if !principal.user.is_admin {
        return admin_required_response();
    }
    match state
        .auth
        .register(&request.username, &request.email, &request.password, false)
        .await
    {
        Ok(user) => (StatusCode::CREATED, Json(user)).into_response(),
        Err(err) => auth_error_response(err, false),
    }
}

async fn admin_user_password(
    Extension(state): Extension<Arc<ApiState>>,
    headers: HeaderMap,
    Json(request): Json<PasswordChangeRequest>,
) -> Response {
    let principal = match authorize(&state, &headers).await {
        Ok(principal) => principal,
        Err(err) => return auth_error_response(err, true),
    };
    if !principal.user.is_admin {
        return admin_required_response();
    }
    match state.auth.change_password(&request.id, &request.password).await {
        Ok(()) => (StatusCode::OK, Json(serde_json::Value::Null)).into_response(),
        Err(err) => auth_error_response(err, false),
    }
}

async fn admin_user_status(
    Extension(state): Extension<Arc<ApiState>>,
    headers: HeaderMap,
    Json(request): Json<UserStatusRequest>,
) -> Response {
    let principal = match authorize(&state, &headers).await {
        Ok(principal) => principal,
        Err(err) => return auth_error_response(err, true),
    };
    if !principal.user.is_admin {
        return admin_required_response();
    }
    if request.status != 0 && request.status != 1 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "invalid_status" })),
        )
            .into_response();
    }
    match state.auth.db().get_api_user_by_id(&request.id).await {
        Ok(Some(target)) if target.is_admin != 0 && request.status == 0 => {
            match state.auth.db().api_admin_count().await {
                Ok(1) => (
                    StatusCode::CONFLICT,
                    Json(json!({ "error": "last_admin" })),
                )
                    .into_response(),
                Ok(_) => update_user_status(&state, &request).await,
                Err(_) => auth_error_response(AuthError::Internal, false),
            }
        }
        Ok(Some(_)) => update_user_status(&state, &request).await,
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "user_not_found" })),
        )
            .into_response(),
        Err(_) => auth_error_response(AuthError::Internal, false),
    }
}

async fn update_user_status(state: &ApiState, request: &UserStatusRequest) -> Response {
    match state
        .auth
        .db()
        .set_api_user_status(&request.id, request.status)
        .await
    {
        Ok(()) => (StatusCode::OK, Json(serde_json::Value::Null)).into_response(),
        Err(_) => auth_error_response(AuthError::Internal, false),
    }
}

async fn admin_user_delete(
    Extension(state): Extension<Arc<ApiState>>,
    headers: HeaderMap,
    Json(request): Json<UserIdRequest>,
) -> Response {
    let principal = match authorize(&state, &headers).await {
        Ok(principal) => principal,
        Err(err) => return auth_error_response(err, true),
    };
    if !principal.user.is_admin {
        return admin_required_response();
    }
    match state.auth.db().get_api_user_by_id(&request.id).await {
        Ok(Some(target)) if target.is_admin != 0 => {
            match state.auth.db().api_admin_count().await {
                Ok(1) => (
                    StatusCode::CONFLICT,
                    Json(json!({ "error": "last_admin" })),
                )
                    .into_response(),
                Ok(_) => delete_user(&state, &request.id).await,
                Err(_) => auth_error_response(AuthError::Internal, false),
            }
        }
        Ok(Some(_)) => delete_user(&state, &request.id).await,
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "user_not_found" })),
        )
            .into_response(),
        Err(_) => auth_error_response(AuthError::Internal, false),
    }
}

async fn delete_user(state: &ApiState, id: &str) -> Response {
    match state.auth.db().delete_api_user(id).await {
        Ok(()) => (StatusCode::OK, Json(serde_json::Value::Null)).into_response(),
        Err(_) => auth_error_response(AuthError::Internal, false),
    }
}

fn admin_user_response(user: crate::database::ApiUser) -> AdminUserResponse {
    AdminUserResponse {
        id: user.id,
        name: user.username,
        email: user.email,
        note: user.nickname,
        is_admin: user.is_admin != 0,
        status: user.status,
        created_at: user.created_at,
    }
}

fn admin_required_response() -> Response {
    (
        StatusCode::FORBIDDEN,
        Json(json!({ "error": "admin_required" })),
    )
        .into_response()
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

async fn admin_logout(
    Extension(state): Extension<Arc<ApiState>>,
    headers: HeaderMap,
) -> Response {
    match authorize(&state, &headers).await {
        Ok(principal) if principal.user.is_admin => logout_authorized(&state, principal).await,
        Ok(_) => admin_required_response(),
        Err(err) => auth_error_response(err, true),
    }
}

async fn logout_authorized(state: &ApiState, principal: Principal) -> Response {
    match state
        .auth
        .revoke_session(&principal.user_id, &principal.session_id)
        .await
    {
        Ok(()) => (StatusCode::OK, Json(serde_json::Value::Null)).into_response(),
        Err(err) => auth_error_response(err, false),
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
        AuthError::Busy => (
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({ "error": "too_many_requests" })),
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
