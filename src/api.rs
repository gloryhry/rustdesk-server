use crate::auth::{AuthError, AuthService, LoginDevice, Principal};
use crate::ldap::LdapConfig;
use crate::oauth::{OAuthError, OAuthRuntime};
use axum::{
    extract::{Extension, Json, Query},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
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
    pub oauth: OAuthRuntime,
    pub oauth_redirect_url: String,
    pub ldap: Arc<hbb_common::tokio::sync::RwLock<LdapConfig>>,
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

#[derive(Debug, Deserialize)]
pub struct OAuthQuery {
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub error: String,
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

#[derive(Debug, Deserialize)]
pub struct GroupRequest {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct GroupIdRequest {
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct TagRequest {
    pub name: String,
    #[serde(default)]
    pub color: String,
}

#[derive(Debug, Deserialize)]
pub struct TagDeleteRequest {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct LdapConfigRequest {
    pub enabled: bool,
    pub url: String,
    pub bind_dn: String,
    #[serde(default)]
    pub bind_password: String,
    pub user_base_dn: String,
    pub user_filter: String,
    pub username_attribute: String,
    pub email_attribute: String,
    pub use_tls: bool,
    pub timeout_seconds: u64,
}

#[derive(Debug, Default, Deserialize)]
pub struct DeviceReportRequest {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub uuid: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub os: String,
    #[serde(default, rename = "type")]
    pub device_type: String,
    #[serde(default)]
    pub info: String,
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
    oauth: OAuthRuntime,
    oauth_redirect_url: String,
    ldap: LdapConfig,
) -> Router {
    let state = Arc::new(ApiState {
        auth,
        registration_enabled,
        server_config,
        oauth,
        oauth_redirect_url,
        ldap: Arc::new(hbb_common::tokio::sync::RwLock::new(ldap)),
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
        .route("/api/oidc/auth", get(oauth_login).post(oauth_login))
        .route("/api/oidc/login", get(oauth_login))
        .route("/api/oidc/callback", get(oauth_callback))
        .route("/api/oauth/login", get(oauth_login))
        .route("/api/oauth/callback", get(oauth_callback))
        .route("/api/register", post(register))
        .route("/api/admin/user/register", post(register))
        .route("/api/currentUser", get(current_user).post(current_user))
        .route("/api/users", get(list_users))
        .route("/api/peers", get(list_devices))
        .route("/api/device-group/accessible", get(list_device_groups))
        .route("/api/admin/user/current", get(admin_current_user))
        .route("/api/admin/user/list", get(admin_user_list))
        .route("/api/admin/user/create", post(admin_user_create))
        .route("/api/admin/user/update", post(admin_user_status))
        .route("/api/admin/user/changePwd", post(admin_user_password))
        .route("/api/admin/user/delete", post(admin_user_delete))
        .route("/api/admin/ldap/config", get(admin_ldap_config).post(admin_ldap_update))
        .route("/api/logout", post(logout))
        .route("/api/admin/logout", post(admin_logout))
        .route("/api/ab", get(get_address_book).post(update_address_book))
        .route("/api/ab/tags", get(list_tags).post(upsert_tag))
        .route("/api/ab/tags/delete", post(delete_tag))
        .route("/api/groups", get(list_groups).post(create_group))
        .route("/api/groups/delete", post(delete_group))
        .route("/api/device-groups", get(list_device_groups).post(create_device_group))
        .route("/api/device-groups/delete", post(delete_device_group))
        .route("/api/device-groups/members", post(add_device_group_member))
        .route("/api/device-groups/members/delete", post(remove_device_group_member))
        .route("/api/devices", get(list_devices))
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
    oauth: OAuthRuntime,
    oauth_redirect_url: String,
    ldap: LdapConfig,
) -> Result<Router, AuthError> {
    let auth = AuthService::new(db, secret, token_ttl)?;
    if let Some((username, password)) = bootstrap_admin {
        auth.ensure_bootstrap_admin(&username, &password).await?;
    }
    Ok(build_router(
        auth,
        registration_enabled,
        server_config,
        web_root,
        oauth,
        oauth_redirect_url,
        ldap,
    ))
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

async fn sysinfo(
    Extension(state): Extension<Arc<ApiState>>,
    headers: HeaderMap,
    payload: Option<Json<DeviceReportRequest>>,
) -> Response {
    if let (Ok(principal), Some(Json(report))) = (authorize(&state, &headers).await, payload) {
        if report.id.len() > 128
            || report.uuid.len() > 128
            || report.name.len() > 256
            || report.os.len() > 128
            || report.device_type.len() > 64
            || report.info.len() > 64 * 1024
        {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "device_info_too_large" })),
            )
                .into_response();
        }
        let device_id = if report.id.is_empty() {
            uuid::Uuid::new_v4().to_string()
        } else {
            format!("{}:{}", principal.user_id, report.id)
        };
        if let Err(_) = state
            .auth
            .db()
            .upsert_api_device(
                &device_id,
                &principal.user_id,
                &report.uuid,
                &report.name,
                &report.os,
                &report.device_type,
                &report.info,
            )
            .await
        {
            return auth_error_response(AuthError::Internal, false);
        }
    }
    (StatusCode::OK, Json(json!({ "code": 0 }))).into_response()
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

async fn list_users(
    Extension(state): Extension<Arc<ApiState>>,
    headers: HeaderMap,
) -> Response {
    match authorize(&state, &headers).await {
        Ok(principal) => (StatusCode::OK, Json(json!({ "code": 0, "data": [principal.user] }))).into_response(),
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

async fn admin_ldap_config(
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
    let config = state.ldap.read().await;
    (StatusCode::OK, Json(config.view())).into_response()
}

async fn admin_ldap_update(
    Extension(state): Extension<Arc<ApiState>>,
    headers: HeaderMap,
    Json(request): Json<LdapConfigRequest>,
) -> Response {
    let principal = match authorize(&state, &headers).await {
        Ok(principal) => principal,
        Err(err) => return auth_error_response(err, true),
    };
    if !principal.user.is_admin {
        return admin_required_response();
    }
    let mut current = state.ldap.write().await;
    let config = LdapConfig {
        enabled: request.enabled,
        url: request.url.trim().to_owned(),
        bind_dn: request.bind_dn.trim().to_owned(),
        bind_password: if request.bind_password.is_empty() {
            current.bind_password.clone()
        } else {
            request.bind_password
        },
        user_base_dn: request.user_base_dn.trim().to_owned(),
        user_filter: request.user_filter.trim().to_owned(),
        username_attribute: request.username_attribute.trim().to_owned(),
        email_attribute: request.email_attribute.trim().to_owned(),
        use_tls: request.use_tls,
        timeout_seconds: request.timeout_seconds,
    };
    if let Err(crate::ldap::LdapConfigError::Invalid(message)) = config.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": message })),
        )
            .into_response();
    }
    *current = config;
    let view = current.view();
    (StatusCode::OK, Json(view)).into_response()
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

async fn list_devices(
    Extension(state): Extension<Arc<ApiState>>,
    headers: HeaderMap,
) -> Response {
    let principal = match authorize(&state, &headers).await {
        Ok(principal) => principal,
        Err(err) => return auth_error_response(err, true),
    };
    let devices = if principal.user.is_admin {
        state.auth.db().list_all_api_devices().await
    } else {
        state.auth.db().list_api_devices(&principal.user_id).await
    };
    match devices {
        Ok(devices) => (StatusCode::OK, Json(json!({ "code": 0, "data": devices }))).into_response(),
        Err(_) => auth_error_response(AuthError::Internal, false),
    }
}

async fn list_groups(
    Extension(state): Extension<Arc<ApiState>>,
    headers: HeaderMap,
) -> Response {
    let principal = match authorize(&state, &headers).await {
        Ok(principal) => principal,
        Err(err) => return auth_error_response(err, true),
    };
    match state.auth.db().list_api_user_groups(&principal.user_id).await {
        Ok(groups) => (StatusCode::OK, Json(json!({ "code": 0, "data": groups }))).into_response(),
        Err(_) => auth_error_response(AuthError::Internal, false),
    }
}

async fn create_group(
    Extension(state): Extension<Arc<ApiState>>,
    headers: HeaderMap,
    Json(request): Json<GroupRequest>,
) -> Response {
    let principal = match authorize(&state, &headers).await {
        Ok(principal) => principal,
        Err(err) => return auth_error_response(err, true),
    };
    let name = request.name.trim();
    if name.is_empty() || name.chars().count() > 128 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "invalid_group_name" })),
        )
            .into_response();
    }
    let id = uuid::Uuid::new_v4().to_string();
    match state
        .auth
        .db()
        .create_api_user_group(&id, name, &principal.user_id)
        .await
    {
        Ok(()) => (StatusCode::CREATED, Json(json!({ "id": id, "name": name }))).into_response(),
        Err(err) if err.to_string().to_lowercase().contains("unique") => (
            StatusCode::CONFLICT,
            Json(json!({ "error": "group_exists" })),
        )
            .into_response(),
        Err(_) => auth_error_response(AuthError::Internal, false),
    }
}

async fn delete_group(
    Extension(state): Extension<Arc<ApiState>>,
    headers: HeaderMap,
    Json(request): Json<GroupIdRequest>,
) -> Response {
    let principal = match authorize(&state, &headers).await {
        Ok(principal) => principal,
        Err(err) => return auth_error_response(err, true),
    };
    match state
        .auth
        .db()
        .delete_api_user_group(&request.id, &principal.user_id)
        .await
    {
        Ok(()) => (StatusCode::OK, Json(serde_json::Value::Null)).into_response(),
        Err(_) => auth_error_response(AuthError::Internal, false),
    }
}

async fn list_device_groups(
    Extension(state): Extension<Arc<ApiState>>,
    headers: HeaderMap,
) -> Response {
    let principal = match authorize(&state, &headers).await {
        Ok(principal) => principal,
        Err(err) => return auth_error_response(err, true),
    };
    let groups = match state
        .auth
        .db()
        .list_api_device_groups(&principal.user_id)
        .await
    {
        Ok(groups) => groups,
        Err(_) => return auth_error_response(AuthError::Internal, false),
    };
    match state
        .auth
        .db()
        .list_api_device_group_members(&principal.user_id)
        .await
    {
        Ok(memberships) => (
            StatusCode::OK,
            Json(json!({ "code": 0, "data": groups, "memberships": memberships })),
        )
            .into_response(),
        Err(_) => auth_error_response(AuthError::Internal, false),
    }
}

async fn create_device_group(
    Extension(state): Extension<Arc<ApiState>>,
    headers: HeaderMap,
    Json(request): Json<GroupRequest>,
) -> Response {
    let principal = match authorize(&state, &headers).await {
        Ok(principal) => principal,
        Err(err) => return auth_error_response(err, true),
    };
    let name = request.name.trim();
    if name.is_empty() || name.chars().count() > 128 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "invalid_group_name" })),
        )
            .into_response();
    }
    let id = uuid::Uuid::new_v4().to_string();
    match state
        .auth
        .db()
        .create_api_device_group(&id, name, &principal.user_id)
        .await
    {
        Ok(()) => (StatusCode::CREATED, Json(json!({ "id": id, "name": name }))).into_response(),
        Err(err) if err.to_string().to_lowercase().contains("unique") => (
            StatusCode::CONFLICT,
            Json(json!({ "error": "group_exists" })),
        )
            .into_response(),
        Err(_) => auth_error_response(AuthError::Internal, false),
    }
}

async fn add_device_group_member(
    Extension(state): Extension<Arc<ApiState>>,
    headers: HeaderMap,
    Json(request): Json<DeviceGroupMemberRequest>,
) -> Response {
    let principal = match authorize(&state, &headers).await {
        Ok(principal) => principal,
        Err(err) => return auth_error_response(err, true),
    };
    match state
        .auth
        .db()
        .add_api_device_group_member(
            &request.group_id,
            &request.device_id,
            &principal.user_id,
        )
        .await
    {
        Ok(true) => (StatusCode::OK, Json(serde_json::Value::Null)).into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "device_or_group_not_found" })),
        )
            .into_response(),
        Err(_) => auth_error_response(AuthError::Internal, false),
    }
}

async fn remove_device_group_member(
    Extension(state): Extension<Arc<ApiState>>,
    headers: HeaderMap,
    Json(request): Json<DeviceGroupMemberRequest>,
) -> Response {
    let principal = match authorize(&state, &headers).await {
        Ok(principal) => principal,
        Err(err) => return auth_error_response(err, true),
    };
    match state
        .auth
        .db()
        .remove_api_device_group_member(
            &request.group_id,
            &request.device_id,
            &principal.user_id,
        )
        .await
    {
        Ok(true) => (StatusCode::OK, Json(serde_json::Value::Null)).into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "device_group_membership_not_found" })),
        )
            .into_response(),
        Err(_) => auth_error_response(AuthError::Internal, false),
    }
}

async fn delete_device_group(
    Extension(state): Extension<Arc<ApiState>>,
    headers: HeaderMap,
    Json(request): Json<GroupIdRequest>,
) -> Response {
    let principal = match authorize(&state, &headers).await {
        Ok(principal) => principal,
        Err(err) => return auth_error_response(err, true),
    };
    match state
        .auth
        .db()
        .delete_api_device_group(&request.id, &principal.user_id)
        .await
    {
        Ok(_) => (StatusCode::OK, Json(serde_json::Value::Null)).into_response(),
        Err(_) => auth_error_response(AuthError::Internal, false),
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

async fn list_tags(
    Extension(state): Extension<Arc<ApiState>>,
    headers: HeaderMap,
) -> Response {
    let principal = match authorize(&state, &headers).await {
        Ok(principal) => principal,
        Err(err) => return auth_error_response(err, true),
    };
    let document = match address_book_document(&state, &principal.user_id).await {
        Ok(document) => document,
        Err(_) => return auth_error_response(AuthError::Internal, false),
    };
    let tags = document
        .get("tags")
        .cloned()
        .unwrap_or_else(|| serde_json::Value::Array(Vec::new()));
    (StatusCode::OK, Json(json!({ "code": 0, "data": tags }))).into_response()
}

async fn upsert_tag(
    Extension(state): Extension<Arc<ApiState>>,
    headers: HeaderMap,
    Json(request): Json<TagRequest>,
) -> Response {
    let principal = match authorize(&state, &headers).await {
        Ok(principal) => principal,
        Err(err) => return auth_error_response(err, true),
    };
    let name = request.name.trim();
    let color = request.color.trim();
    if name.is_empty() || name.chars().count() > 64 || !valid_tag_color(color) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "invalid_tag" })),
        )
            .into_response();
    }
    let mut document = match address_book_document(&state, &principal.user_id).await {
        Ok(document) => document,
        Err(_) => return auth_error_response(AuthError::Internal, false),
    };
    if !document
        .get("tags")
        .map(serde_json::Value::is_array)
        .unwrap_or(false)
    {
        document["tags"] = serde_json::Value::Array(Vec::new());
    }
    let tags = match document
        .get_mut("tags")
        .and_then(serde_json::Value::as_array_mut)
    {
        Some(tags) => tags,
        None => return auth_error_response(AuthError::Internal, false),
    };
    let tag = json!({ "name": name, "color": color });
    if let Some(existing) = tags.iter_mut().find(|tag| {
        tag.get("name")
            .and_then(serde_json::Value::as_str)
            .map(|value| value.eq_ignore_ascii_case(name))
            .unwrap_or(false)
    }) {
        *existing = tag;
    } else {
        tags.push(tag);
    }
    let data = match serde_json::to_string(&document) {
        Ok(data) => data,
        Err(_) => return auth_error_response(AuthError::Internal, false),
    };
    match state
        .auth
        .db()
        .upsert_api_address_book(&principal.user_id, &data)
        .await
    {
        Ok(()) => (StatusCode::OK, Json(json!({ "name": name, "color": color }))).into_response(),
        Err(_) => auth_error_response(AuthError::Internal, false),
    }
}

async fn delete_tag(
    Extension(state): Extension<Arc<ApiState>>,
    headers: HeaderMap,
    Json(request): Json<TagDeleteRequest>,
) -> Response {
    let principal = match authorize(&state, &headers).await {
        Ok(principal) => principal,
        Err(err) => return auth_error_response(err, true),
    };
    let name = request.name.trim();
    if name.is_empty() || name.chars().count() > 64 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "invalid_tag" })),
        )
            .into_response();
    }
    let mut document = match address_book_document(&state, &principal.user_id).await {
        Ok(document) => document,
        Err(_) => return auth_error_response(AuthError::Internal, false),
    };
    let tags = document
        .get_mut("tags")
        .and_then(serde_json::Value::as_array_mut);
    if let Some(tags) = tags {
        tags.retain(|tag| {
            !tag.get("name")
                .and_then(serde_json::Value::as_str)
                .map(|value| value.eq_ignore_ascii_case(name))
                .unwrap_or(false)
        });
    }
    let data = match serde_json::to_string(&document) {
        Ok(data) => data,
        Err(_) => return auth_error_response(AuthError::Internal, false),
    };
    match state
        .auth
        .db()
        .upsert_api_address_book(&principal.user_id, &data)
        .await
    {
        Ok(()) => (StatusCode::OK, Json(serde_json::Value::Null)).into_response(),
        Err(_) => auth_error_response(AuthError::Internal, false),
    }
}

async fn address_book_document(
    state: &ApiState,
    user_id: &str,
) -> Result<serde_json::Map<String, serde_json::Value>, AuthError> {
    let data = state
        .auth
        .db()
        .get_api_address_book(user_id)
        .await
        .map_err(|_| AuthError::Internal)?
        .unwrap_or_else(|| "{}".to_owned());
    match serde_json::from_str::<serde_json::Value>(&data).map_err(|_| AuthError::Internal)? {
        serde_json::Value::Object(document) => Ok(document),
        _ => Err(AuthError::Internal),
    }
}

fn valid_tag_color(value: &str) -> bool {
    value.is_empty()
        || ((value.len() == 4 || value.len() == 7)
            && value.starts_with('#')
            && value[1..].bytes().all(|byte| byte.is_ascii_hexdigit()))
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

async fn oauth_login(
    Extension(state): Extension<Arc<ApiState>>,
    Query(query): Query<OAuthQuery>,
) -> Response {
    if state.oauth_redirect_url.is_empty() {
        return oauth_error_response(OAuthError::NotConfigured);
    }
    match state
        .oauth
        .begin(&query.provider, &state.oauth_redirect_url)
        .await
    {
        Ok(url) => Redirect::temporary(url.as_str()).into_response(),
        Err(err) => oauth_error_response(err),
    }
}

async fn oauth_callback(
    Extension(state): Extension<Arc<ApiState>>,
    Query(query): Query<OAuthQuery>,
) -> Response {
    if !query.error.is_empty() {
        return oauth_error_response(OAuthError::InvalidState);
    }
    if state.oauth_redirect_url.is_empty() {
        return oauth_error_response(OAuthError::NotConfigured);
    }
    let identity = match state
        .oauth
        .complete(
            &query.code,
            &query.state,
            &state.oauth_redirect_url,
        )
        .await
    {
        Ok(identity) => identity,
        Err(err) => return oauth_error_response(err),
    };
    if !query.provider.is_empty() && query.provider != identity.provider {
        return oauth_error_response(OAuthError::InvalidState);
    }
    match state
        .auth
        .login_external(
            &identity.provider,
            &identity.subject,
            &identity.username,
            &identity.email,
            LoginDevice::default(),
        )
        .await
    {
        Ok(result) => (StatusCode::OK, Json(result)).into_response(),
        Err(err) => auth_error_response(err, false),
    }
}

fn oauth_error_response(error: OAuthError) -> Response {
    let (status, code) = match error {
        OAuthError::InvalidState => (StatusCode::BAD_REQUEST, "invalid_oauth_state"),
        OAuthError::NotConfigured => (StatusCode::NOT_FOUND, "oauth_not_configured"),
        OAuthError::InvalidResponse => (StatusCode::BAD_GATEWAY, "invalid_oauth_response"),
        OAuthError::Remote => (StatusCode::BAD_GATEWAY, "oauth_provider_unavailable"),
    };
    (status, Json(json!({ "error": code }))).into_response()
}


async fn login_options(Extension(state): Extension<Arc<ApiState>>) -> impl IntoResponse {
    let providers = if state.oauth_redirect_url.is_empty() {
        Vec::new()
    } else {
        state.oauth.provider_names()
    };
    let provider_json = providers
        .iter()
        .map(|provider| json!({ "name": provider }))
        .collect::<Vec<_>>();
    let mut options = vec![format!(
        "common-oidc/{}",
        serde_json::to_string(&provider_json).unwrap_or_else(|_| "[]".to_owned())
    )];
    options.extend(providers.into_iter().map(|provider| format!("oidc/{provider}")));
    Json(options)
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
