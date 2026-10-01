use crate::auth::{AuthError, AuthService, LoginDevice, Principal};
mod official_address_book;
use crate::ldap::LdapConfig;
use crate::address_book_store::{AddressBookStore,Book,BookError};
pub use crate::browser_security::{CookiePolicy, BrowserPolicy};
use crate::oauth::{OAuthError, OAuthRuntime, OAuthFlowKind};
use crate::native_oauth::NativeOAuthStore;
use crate::oauth_admin::{AdminError, OAuthProviderAdmin, ProviderRequest, ProviderSecretKey};
use axum::{
    extract::{Extension, Json, Path, Query},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Redirect, Response},
    routing::{delete as delete_route, get, post, put},
    Router,
};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tower_http::{
    services::ServeDir,
    limit::RequestBodyLimitLayer,
    timeout::RequestBodyTimeoutLayer,
};

#[derive(Clone)]
pub struct ApiState {
    pub auth: AuthService,
    pub registration_enabled: bool,
    pub server_config: PublicServerConfig,
    pub oauth: OAuthRuntime,
    pub oauth_redirect_url: String,
    pub ldap: Arc<hbb_common::tokio::sync::RwLock<LdapConfig>>,
    pub cookie_policy: CookiePolicy,
    browser_policy: BrowserPolicy,
    native_oauth: NativeOAuthStore,
    provider_admin: Arc<OAuthProviderAdmin>,
    pub tag_lock: Arc<hbb_common::tokio::sync::Mutex<()>>,
    web_root: String,
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
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub uuid: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct OAuthAuthRequest {
    #[serde(default)]
    pub op: String,    #[serde(default)]
    pub provider: String,
    #[serde(default, rename = "redirectUri")]
    pub redirect_uri: String,
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub uuid: String,
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
    pub revision: Option<i64>,
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
pub struct GroupMemberRequest {
    pub group_id: String,
    pub user_id: String,
}

#[derive(Debug, Deserialize)]
pub struct DeviceGroupMemberRequest {
    pub group_id: String,
    pub device_id: String,
}

#[derive(Debug, Deserialize)]
pub struct AddressBookEntryRequest {
    pub id: Option<String>,
    pub revision: Option<i64>,
    #[serde(default)]
    pub peer_id: String,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub hostname: Option<String>,
    #[serde(default)]
    pub alias: Option<String>,
    #[serde(default)]
    pub platform: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default,alias="forceAlwaysRelay",deserialize_with="crate::address_book_codec::optional_relay")]
    pub force_always_relay: Option<bool>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String,serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct AddressBookEntryDeleteRequest {
    pub id: String,
    pub revision: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct TagRequest {
    pub name: String,
    #[serde(default)]
    pub color: Option<String>,
    pub old_name: Option<String>,
    pub revision: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct TagDeleteRequest {
    pub name: String,
    pub revision: Option<i64>,
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

fn build_router(
    auth: AuthService,
    registration_enabled: bool,
    public_server_config: PublicServerConfig,
    web_root: String,
    oauth: OAuthRuntime,
    oauth_redirect_url: String,
    ldap: LdapConfig,
    cookie_policy: CookiePolicy,
    provider_admin: OAuthProviderAdmin,
    browser_policy: BrowserPolicy,
) -> Router {
    let native_clock = oauth.clone();
    let state = Arc::new(ApiState {
        auth,
        registration_enabled,
        server_config: public_server_config,
        oauth,
        oauth_redirect_url,
        ldap: Arc::new(hbb_common::tokio::sync::RwLock::new(ldap)),
        cookie_policy,
        browser_policy,
        provider_admin: Arc::new(provider_admin),
        native_oauth: NativeOAuthStore::new(Arc::new(move || native_clock.now())),
        tag_lock: Arc::new(hbb_common::tokio::sync::Mutex::new(())),
        web_root: web_root.clone(),
    });
    Router::new()
        .route("/health/live", get(health_live))
        .route("/health/ready", get(health_ready))
        .route("/api/", get(api_index))
        .route("/api/version", get(api_version))
        .route("/api/heartbeat", post(heartbeat).layer(RequestBodyLimitLayer::new(crate::device_registry::REPORT_MAX_BYTES)))
        .route("/api/sysinfo", post(sysinfo).layer(RequestBodyLimitLayer::new(crate::device_registry::REPORT_MAX_BYTES)))
        .route("/api/sysinfo_ver", post(sysinfo_version))
        .route("/api/login", post(login))
        .route("/api/admin/login", post(admin_login))
        .route("/api/login-options", get(login_options))
        .route("/api/oidc/auth", get(oauth_login).post(oauth_login_post))
        .route("/api/oidc/auth-query", get(oauth_auth_query))
        .route("/api/oidc/native", get(oauth_native_browser))
        .route("/api/oidc/msg", get(oauth_message))
        .route("/api/oauth/msg", get(oauth_message))
        .route("/api/oidc/login", get(oauth_login))
        .route("/api/oidc/callback", get(oauth_callback))
        .route("/api/oauth/login", get(oauth_login))
        .route("/api/oauth/callback", get(oauth_callback))
        .route("/api/register", post(register))
        .route("/api/admin/user/register", post(admin_user_create))
        .route("/api/currentUser", get(current_user).post(current_user))
        .route("/api/session/csrf", get(session_csrf))
        .route("/api/user/info", get(current_user))
        .route("/api/users", get(list_users))
        .route("/api/peers", get(list_official_peers))
        .route("/api/device-group/accessible", get(list_accessible_device_groups))
        .route("/api/admin/user/current", get(admin_current_user))
        .route("/api/admin/user/list", get(admin_user_list))
        .route("/api/admin/session/list", get(admin_session_list))
        .route("/api/admin/session/revoke", post(admin_session_revoke))
        .route("/api/admin/oauth/providers", get(admin_oauth_providers).post(admin_oauth_create))
        .route("/api/admin/oauth/providers/update", post(admin_oauth_update))
        .route("/api/admin/oauth/providers/toggle", post(admin_oauth_toggle))
        .route("/api/admin/oauth/providers/delete", post(admin_oauth_delete))
        .route("/api/admin/user/create", post(admin_user_create))
        .route("/api/admin/user/update", post(admin_user_status))
        .route("/api/admin/user/changePwd", post(admin_user_password))
        .route("/api/admin/user/delete", post(admin_user_delete))
        .route("/api/admin/ldap/config", get(admin_ldap_config).post(admin_ldap_update))
        .route("/api/logout", post(logout))
        .route("/api/admin/logout", post(admin_logout))
        .route("/api/ab", get(get_address_book).post(update_address_book))
        .route("/api/ab/personal", get(official_address_book::personal).post(official_address_book::personal))
        .route("/api/ab/settings", get(official_address_book::settings).post(official_address_book::settings))
        .route("/api/ab/shared/profiles", get(official_address_book::shared_profiles).post(official_address_book::shared_profiles))
        .route("/api/ab/peers", get(official_address_book::peers).post(official_address_book::peers))
        .route("/api/ab/peer/add/{guid}", post(official_address_book::add_peer))
        .route("/api/ab/peer/{guid}", delete_route(official_address_book::delete_peers))
        .route("/api/ab/peer/update/{guid}", put(official_address_book::update_peer))
        .route("/api/ab/tags/{guid}", get(official_address_book::tags).post(official_address_book::tags))
        .route("/api/ab/tag/add/{guid}", post(official_address_book::add_tag))
        .route("/api/ab/tag/rename/{guid}", put(official_address_book::rename_tag))
        .route("/api/ab/tag/update/{guid}", put(official_address_book::update_tag))
        .route("/api/ab/tag/{guid}", delete_route(official_address_book::delete_tags))
        .route("/api/web/ab/entries", get(list_address_book_entries).post(upsert_address_book_entry))
        .route("/api/web/ab/entries/batch", post(post_address_book_entries))
        .route("/api/web/ab/entries/delete", post(delete_address_book_entry))
        .route("/api/web/ab/entries/{id}", delete_route(delete_web_address_book_entry))
        .route("/api/web/ab/tags", get(list_tags).post(upsert_tag))
        .route("/api/web/ab/tags/delete", post(delete_tag))
        .route("/api/groups", get(list_groups).post(create_group))
        .route("/api/groups/members", post(add_group_member))
        .route("/api/groups/members/delete", post(remove_group_member))
        .route("/api/groups/delete", post(delete_group))
        .route("/api/device-groups", get(list_device_groups).post(create_device_group))
        .route("/api/device-groups/delete", post(delete_device_group))
        .route("/api/device-groups/members", post(add_device_group_member))
        .route("/api/device-groups/members/delete", post(remove_device_group_member))
        .route("/api/devices", get(list_devices))
        .route("/api/admin/device/list", get(admin_device_list))
        .route("/api/admin/device/registry", get(admin_device_registry))
        .route("/api/admin/device/bind", post(admin_device_bind))
        .route("/api/admin/device/unbind", post(admin_device_unbind))
        .route("/api/admin/device/delete", post(admin_device_delete))
        .route("/api/server-config", post(server_config))
        .route("/api/server-config-v2", post(server_config_v2))
        .layer(RequestBodyLimitLayer::new(1024 * 1024))
        .layer(RequestBodyTimeoutLayer::new(Duration::from_secs(15)))
        .fallback_service(ServeDir::new(web_root))
        .layer(axum::middleware::from_fn(browser_request))
        .layer(Extension(state))
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
    cookie_policy: CookiePolicy,
    provider_key: Option<ProviderSecretKey>,
    browser_policy: Option<BrowserPolicy>,
) -> Result<Router, AuthError> {
    let browser_policy = match browser_policy {
        Some(policy) => policy,
        None => BrowserPolicy::new(&server_config.api_server, &[]).map_err(AuthError::InvalidInput)?,
    };
    if provider_key.as_ref().is_some_and(|key| key.matches(secret.as_bytes())) {
        return Err(AuthError::InvalidInput("OAuth configuration key must be independent from JWT secret"));
    }
    let provider_admin = OAuthProviderAdmin::initialize(db.clone(), oauth.clone(), provider_key)
        .await.map_err(|error| {
            hbb_common::log::error!("OAuth provider registry initialization failed: {error:?}");
            AuthError::Internal
        })?;
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
        cookie_policy,
        provider_admin,
        browser_policy,
    ))
}

async fn health_live() -> impl IntoResponse {
    Json(json!({ "status": "ok" }))
}

async fn health_ready(Extension(state): Extension<Arc<ApiState>>) -> Response {
    let ready = crate::deployment::check_web_assets(std::path::Path::new(&state.web_root)).is_ok()
        && matches!(hbb_common::tokio::time::timeout(Duration::from_secs(2),state.auth.db().check_api_readiness()).await,Ok(Ok(())));
    let status = if ready { StatusCode::OK } else { StatusCode::SERVICE_UNAVAILABLE };
    let mut response = (status,Json(json!({"status":if ready { "ready" } else { "unavailable" }}))).into_response();
    response.headers_mut().insert(header::CACHE_CONTROL,HeaderValue::from_static("no-store")); response
}

async fn api_index() -> impl IntoResponse {
    Json(json!({ "code": 0, "data": "RustDesk API" }))
}

async fn api_version() -> impl IntoResponse {
    Json(json!({ "code": 0, "data": env!("CARGO_PKG_VERSION") }))
}

async fn heartbeat(Extension(state): Extension<Arc<ApiState>>, Json(report): Json<serde_json::Value>) -> Response {
    match state.auth.db().save_device_report(&report,true).await {
        Ok(true) => Json(json!({"sysinfo":true})).into_response(),
        Ok(false) => Json(json!({})).into_response(),
        Err(error) => device_registry_error(error),
    }
}

async fn sysinfo(Extension(state): Extension<Arc<ApiState>>, Json(report): Json<serde_json::Value>) -> Response {
    match state.auth.db().save_device_report(&report,false).await {
        Ok(_) => (StatusCode::OK,"SYSINFO_UPDATED").into_response(),
        Err(error) => device_registry_error(error),
    }
}

fn device_registry_error(error: crate::device_registry::RegistryError) -> Response {
    use crate::device_registry::RegistryError;
    let (status,message) = match error {
        RegistryError::NotFound => return (StatusCode::OK,"ID_NOT_FOUND").into_response(),
        RegistryError::Invalid(message) => (StatusCode::BAD_REQUEST,message),
        RegistryError::Conflict => (StatusCode::CONFLICT,"device_binding_conflict"),
        RegistryError::RateLimited => (StatusCode::TOO_MANY_REQUESTS,"device_report_rate_limited"),
        RegistryError::Capacity => (StatusCode::SERVICE_UNAVAILABLE,"device_report_capacity"),
        RegistryError::Storage => (StatusCode::INTERNAL_SERVER_ERROR,"device_registry_storage_failed"),
    };
    (status,Json(json!({"error":message}))).into_response()
}

async fn sysinfo_version() -> impl IntoResponse {
    (StatusCode::OK, "unsigned-report-v1")
}

async fn login(
    Extension(state): Extension<Arc<ApiState>>,
    Json(request): Json<LoginRequest>,
) -> Response {
    let (username, password, device) = login_parts(request);
    match state.auth.login(&username, &password, device).await {
        Ok(result) => with_auth_cookie(
            &state.cookie_policy,
            (StatusCode::OK, Json(result.clone())).into_response(),
            &result.access_token,
            result.expires_in,
        ),
        Err(err) => auth_error_response(err, true),
    }
}

async fn admin_login(
    Extension(state): Extension<Arc<ApiState>>,
    Json(request): Json<LoginRequest>,
) -> Response {
    let (username, password, device) = login_parts(request);
    match state.auth.login_admin(&username, &password, device).await {
        Ok(result) => with_auth_cookie(
            &state.cookie_policy,
            (StatusCode::OK, Json(result.clone())).into_response(),
            &result.access_token,
            result.expires_in,
        ),
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

async fn session_csrf(Extension(state): Extension<Arc<ApiState>>, headers: HeaderMap) -> Response {
    match authorize(&state, &headers).await {
        Ok(principal) => {
            let mut response = Json(json!({"csrf_token":state.auth.csrf_token(&principal),"user":principal.user})).into_response();
            response.headers_mut().insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
            response
        }
        Err(error) => auth_error_response(error, true),
    }
}

async fn browser_request(
    request: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> Response {
    if !request.uri().path().starts_with("/api/") { return next.run(request).await; }
    let state = match request.extensions().get::<Arc<ApiState>>().cloned() {
        Some(state) => state,
        None => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let origin = request.headers().get(header::ORIGIN).cloned();
    let allowed = origin.as_ref().is_some_and(|value| value.to_str().is_ok_and(|value| state.browser_policy.allows(value)));
    if origin.is_some() && !allowed {
        return cors_response(browser_error("origin_not_allowed"), None);
    }
    let cors_origin = if allowed { origin.as_ref() } else { None };
    if request.method() == axum::http::Method::OPTIONS {
        let valid_method = request.headers().get(header::ACCESS_CONTROL_REQUEST_METHOD)
            .and_then(|value| value.to_str().ok()).is_some_and(|method| matches!(method,"GET"|"POST"|"PUT"|"PATCH"|"DELETE"|"HEAD"));
        let valid_headers = request.headers().get(header::ACCESS_CONTROL_REQUEST_HEADERS).map(|value| {
            value.to_str().is_ok_and(|value| value.split(',').all(|name|
                matches!(name.trim().to_ascii_lowercase().as_str(), "authorization"|"content-type"|"x-csrf-token"|"accept")))
        }).unwrap_or(true);
        if !allowed || !valid_method || !valid_headers {
            return cors_response(browser_error("invalid_cors_preflight"),cors_origin);
        }
        let mut response = StatusCode::NO_CONTENT.into_response();
        response.headers_mut().insert(header::ACCESS_CONTROL_ALLOW_METHODS, HeaderValue::from_static("GET, POST, PUT, PATCH, DELETE, HEAD"));
        response.headers_mut().insert(header::ACCESS_CONTROL_ALLOW_HEADERS, HeaderValue::from_static("Authorization, Content-Type, X-CSRF-Token, Accept"));
        response.headers_mut().insert(header::ACCESS_CONTROL_MAX_AGE, HeaderValue::from_static("600"));
        response.headers_mut().append(header::VARY, HeaderValue::from_static("Access-Control-Request-Method, Access-Control-Request-Headers"));
        return cors_response(response,cors_origin);
    }
    let unsafe_method = !matches!(*request.method(),axum::http::Method::GET|axum::http::Method::HEAD);
    let public_login = matches!(request.uri().path(), "/api/login"|"/api/admin/login"|"/api/register"|"/api/users/register"|"/api/oidc/auth");
    let read_post = matches!(request.uri().path(), "/api/currentUser"|"/api/user/info"|"/api/server-config"|"/api/server-config-v2");
    let uses_cookie = !request.headers().contains_key(header::AUTHORIZATION)
        && cookie_value(request.headers().get(header::COOKIE),"rustdesk_api_token").is_some();
    if unsafe_method && uses_cookie && !public_login && !read_post {
        if !allowed { return cors_response(browser_error("csrf_origin_required"),cors_origin); }
        let principal = match authorize(&state,request.headers()).await {
            Ok(principal) => principal,
            Err(error) => return cors_response(auth_error_response(error,true),cors_origin),
        };
        let expected = state.auth.csrf_token(&principal);
        let matches = request.headers().get("x-csrf-token").and_then(|value| value.to_str().ok())
            .is_some_and(|value| sodiumoxide::utils::memcmp(value.as_bytes(),expected.as_bytes()));
        if !matches { return cors_response(browser_error("csrf_token_required"),cors_origin); }
    }
    let origin = origin.filter(|_| allowed);
    cors_response(next.run(request).await,origin.as_ref())
}

fn browser_error(message: &str) -> Response {
    (StatusCode::FORBIDDEN,Json(json!({"error":message}))).into_response()
}

fn cors_response(mut response: Response, origin: Option<&HeaderValue>) -> Response {
    response.headers_mut().append(header::VARY,HeaderValue::from_static("Origin"));
    if let Some(origin) = origin {
        response.headers_mut().insert(header::ACCESS_CONTROL_ALLOW_ORIGIN,origin.clone());
        response.headers_mut().insert(header::ACCESS_CONTROL_ALLOW_CREDENTIALS,HeaderValue::from_static("true"));
    }
    response
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

async fn list_users(Extension(state): Extension<Arc<ApiState>>, headers: HeaderMap, Query(query): Query<crate::pagination::ListQuery>) -> Response {
    let principal = match authorize(&state,&headers).await {
        Ok(principal) => principal,
        Err(error) => return auth_error_response(error,true),
    };
    let page = match query.parse() { Ok(page) => page, Err(error) => return invalid_list_query(error) };
    if !principal.user.is_admin {
        let visible = page.status.map_or(true,|status|principal.user.status==status)
            && page.name.as_ref().map_or(true,|name|principal.user.name.to_lowercase().contains(&name.to_lowercase()));
        let total = usize::from(visible);
        let data = if visible && page.offset==0 { vec![ListedUser { id:principal.user_id,user:principal.user }] } else { Vec::new() };
        return Json(crate::pagination::Page { total,data,code:0 }).into_response();
    }
    match state.auth.db().paged_api_users(&page).await {
        Ok(users) => Json(crate::pagination::Page { total:users.total,data:users.data.into_iter().map(admin_user_response).collect::<Vec<_>>(),code:0 }).into_response(),
        Err(_) => auth_error_response(AuthError::Internal,false),
    }
}

fn invalid_list_query(message: &str) -> Response {
    (StatusCode::BAD_REQUEST,Json(json!({"error":message}))).into_response()
}

#[derive(serde::Serialize)]
struct ListedUser {
    id: String,
    #[serde(flatten)]
    user: crate::auth::PublicUser,
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

async fn admin_session_list(
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
    match state.auth.db().list_api_sessions().await {
        Ok(sessions) => (StatusCode::OK, Json(json!({ "code": 0, "data": sessions }))).into_response(),
        Err(_) => auth_error_response(AuthError::Internal, false),
    }
}

async fn admin_session_revoke(
    Extension(state): Extension<Arc<ApiState>>,
    headers: HeaderMap,
    Json(request): Json<GroupIdRequest>,
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
        .db()
        .revoke_api_session_by_id(&request.id, crate::common::now() as i64)
        .await
    {
        Ok(true) => (StatusCode::OK, Json(serde_json::Value::Null)).into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "session_not_found" })),
        )
            .into_response(),
        Err(_) => auth_error_response(AuthError::Internal, false),
    }
}

async fn admin_oauth_providers(
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
    match state.provider_admin.list().await {
        Ok(providers) => Json(json!({"code":0,"data":providers,"redirect_url_configured":!state.oauth_redirect_url.is_empty()})).into_response(),
        Err(error) => provider_admin_error(error),
    }
}

async fn admin_oauth_create(Extension(state): Extension<Arc<ApiState>>, headers: HeaderMap, Json(request): Json<ProviderRequest>) -> Response {
    if let Err(response) = require_admin(&state, &headers).await { return response; }
    let _guard = state.provider_admin.lock().await;
    match state.provider_admin.create(request).await {
        Ok(provider) => (StatusCode::CREATED, Json(provider)).into_response(),
        Err(error) => provider_admin_error(error),
    }
}

async fn admin_oauth_update(Extension(state): Extension<Arc<ApiState>>, headers: HeaderMap, Json(request): Json<ProviderRequest>) -> Response {
    if let Err(response) = require_admin(&state, &headers).await { return response; }
    let _guard = state.provider_admin.lock().await;
    match state.provider_admin.update(request).await {
        Ok(provider) => {
            state.native_oauth.invalidate(&provider.config.name).await;
            Json(provider).into_response()
        }
        Err(error) => provider_admin_error(error),
    }
}

#[derive(Deserialize)]
struct ProviderToggleRequest { id: String, enabled: bool }

async fn admin_oauth_toggle(Extension(state): Extension<Arc<ApiState>>, headers: HeaderMap, Json(request): Json<ProviderToggleRequest>) -> Response {
    if let Err(response) = require_admin(&state, &headers).await { return response; }
    let _guard = state.provider_admin.lock().await;
    match state.provider_admin.toggle(&request.id, request.enabled).await {
        Ok(provider) => {
            state.native_oauth.invalidate(&provider.config.name).await;
            Json(provider).into_response()
        }
        Err(error) => provider_admin_error(error),
    }
}

async fn admin_oauth_delete(Extension(state): Extension<Arc<ApiState>>, headers: HeaderMap, Json(request): Json<UserIdRequest>) -> Response {
    if let Err(response) = require_admin(&state, &headers).await { return response; }
    let _guard = state.provider_admin.lock().await;
    match state.provider_admin.delete(&request.id).await {
        Ok(name) => {
            state.native_oauth.invalidate(&name).await;
            Json(serde_json::Value::Null).into_response()
        }
        Err(error) => provider_admin_error(error),
    }
}

async fn require_admin(state: &ApiState, headers: &HeaderMap) -> Result<(), Response> {
    match authorize(state, headers).await {
        Ok(principal) if principal.user.is_admin => Ok(()),
        Ok(_) => Err(admin_required_response()),
        Err(error) => Err(auth_error_response(error, true)),
    }
}

fn provider_admin_error(error: AdminError) -> Response {
    let (status, message) = match error {
        AdminError::Invalid(message) => (StatusCode::BAD_REQUEST, message),
        AdminError::Conflict => (StatusCode::CONFLICT, "oauth_provider_conflict"),
        AdminError::NotFound => (StatusCode::NOT_FOUND, "oauth_provider_not_found"),
        AdminError::ReadOnly => (StatusCode::FORBIDDEN, "environment_provider_is_read_only"),
        AdminError::KeyMissing => (StatusCode::SERVICE_UNAVAILABLE, "oauth_configuration_key_missing"),
        AdminError::Encryption => (StatusCode::INTERNAL_SERVER_ERROR, "oauth_secret_decryption_failed"),
        AdminError::Internal => (StatusCode::INTERNAL_SERVER_ERROR, "oauth_provider_storage_failed"),
    };
    (status, Json(json!({"error":message}))).into_response()
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
        Ok(true) => (StatusCode::OK, Json(serde_json::Value::Null)).into_response(),
        Ok(false) => (
            StatusCode::CONFLICT,
            Json(json!({ "error": "last_admin" })),
        )
            .into_response(),
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
        Ok(true) => (StatusCode::OK, Json(serde_json::Value::Null)).into_response(),
        Ok(false) => (
            StatusCode::CONFLICT,
            Json(json!({ "error": "last_admin_or_user_not_found" })),
        )
            .into_response(),
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
            Ok(()) => clear_auth_cookie(&state.cookie_policy, (StatusCode::OK, Json(serde_json::Value::Null)).into_response()),
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
        Ok(()) => clear_auth_cookie(&state.cookie_policy, (StatusCode::OK, Json(serde_json::Value::Null)).into_response()),
        Err(err) => auth_error_response(err, false),
    }
}

async fn list_official_peers(Extension(state): Extension<Arc<ApiState>>, headers: HeaderMap, Query(query): Query<crate::pagination::ListQuery>) -> Response {
    let principal = match authorize(&state,&headers).await {
        Ok(principal) => principal,
        Err(error) => return auth_error_response(error,true),
    };
    let page = match query.parse() { Ok(page) => page, Err(error) => return invalid_list_query(error) };
    match state.auth.db().official_peers(&principal.user_id,principal.user.is_admin,&page).await {
        Ok(records) => Json(crate::pagination::Page { total:records.total,data:records.data.into_iter().map(crate::official_peer::OfficialPeer::from).collect::<Vec<_>>(),code:0 }).into_response(),
        Err(_) => auth_error_response(AuthError::Internal,false),
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

#[derive(Default,Deserialize)]
struct RegisteredQuery { peer_id: Option<String> }

async fn admin_device_registry(Extension(state): Extension<Arc<ApiState>>, headers: HeaderMap, Query(query): Query<RegisteredQuery>) -> Response {
    if let Err(response) = require_admin(&state,&headers).await { return response; }
    match state.auth.db().registered_devices(query.peer_id.as_deref()).await {
        Ok(devices) => Json(json!({"data":devices})).into_response(),
        Err(_) => device_registry_error(crate::device_registry::RegistryError::Storage),
    }
}

async fn admin_device_bind(Extension(state): Extension<Arc<ApiState>>, headers: HeaderMap, Json(request): Json<crate::device_registry::BindDeviceRequest>) -> Response {
    let principal = match authorize(&state,&headers).await {
        Ok(principal) if principal.user.is_admin => principal,
        Ok(_) => return admin_required_response(),
        Err(error) => return auth_error_response(error,true),
    };
    match state.auth.db().bind_api_device(&principal.user_id,&request).await {
        Ok(id) => Json(json!({"id":id})).into_response(),
        Err(crate::device_registry::RegistryError::NotFound) => (StatusCode::NOT_FOUND,Json(json!({"error":"device_or_user_not_found"}))).into_response(),
        Err(error) => device_registry_error(error),
    }
}

async fn admin_device_unbind(Extension(state): Extension<Arc<ApiState>>, headers: HeaderMap, Json(request): Json<UserIdRequest>) -> Response {
    let principal = match authorize(&state,&headers).await {
        Ok(principal) if principal.user.is_admin => principal,
        Ok(_) => return admin_required_response(),
        Err(error) => return auth_error_response(error,true),
    };
    match state.auth.db().unbind_api_device(&principal.user_id,&request.id).await {
        Ok(()) => Json(serde_json::Value::Null).into_response(),
        Err(crate::device_registry::RegistryError::NotFound) => (StatusCode::NOT_FOUND,Json(json!({"error":"verified_device_not_found"}))).into_response(),
        Err(error) => device_registry_error(error),
    }
}

async fn admin_device_list(
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
    match state.auth.db().list_all_api_devices().await {
        Ok(devices) => (StatusCode::OK, Json(json!({ "code": 0, "data": devices }))).into_response(),
        Err(_) => auth_error_response(AuthError::Internal, false),
    }
}

async fn admin_device_delete(
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
    match state.auth.db().delete_api_device(&request.id,&principal.user_id).await {
        Ok(true) => (StatusCode::OK, Json(serde_json::Value::Null)).into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "device_not_found" })),
        )
            .into_response(),
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
    let groups = match state.auth.db().list_api_user_groups(&principal.user_id).await {
        Ok(groups) => groups,
        Err(_) => return auth_error_response(AuthError::Internal, false),
    };
    match state
        .auth
        .db()
        .list_api_user_group_members(&principal.user_id, principal.user.is_admin)
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

async fn add_group_member(
    Extension(state): Extension<Arc<ApiState>>,
    headers: HeaderMap,
    Json(request): Json<GroupMemberRequest>,
) -> Response {
    let principal = match authorize(&state, &headers).await {
        Ok(principal) => principal,
        Err(err) => return auth_error_response(err, true),
    };
    match state
        .auth
        .db()
        .add_api_user_group_member(
            &request.group_id,
            &request.user_id,
            &principal.user_id,
            principal.user.is_admin,
        )
        .await
    {
        Ok(true) => (StatusCode::OK, Json(serde_json::Value::Null)).into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "group_or_user_not_found" })),
        )
            .into_response(),
        Err(_) => auth_error_response(AuthError::Internal, false),
    }
}

async fn remove_group_member(
    Extension(state): Extension<Arc<ApiState>>,
    headers: HeaderMap,
    Json(request): Json<GroupMemberRequest>,
) -> Response {
    let principal = match authorize(&state, &headers).await {
        Ok(principal) => principal,
        Err(err) => return auth_error_response(err, true),
    };
    match state
        .auth
        .db()
        .remove_api_user_group_member(
            &request.group_id,
            &request.user_id,
            &principal.user_id,
            principal.user.is_admin,
        )
        .await
    {
        Ok(true) => (StatusCode::OK, Json(serde_json::Value::Null)).into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "group_member_not_found" })),
        )
            .into_response(),
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
        Ok(true) => (StatusCode::OK, Json(serde_json::Value::Null)).into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "group_not_found" })),
        )
            .into_response(),
        Err(_) => auth_error_response(AuthError::Internal, false),
    }
}

async fn list_accessible_device_groups(Extension(state): Extension<Arc<ApiState>>, headers: HeaderMap, Query(query): Query<crate::pagination::ListQuery>) -> Response {
    let principal = match authorize(&state,&headers).await {
        Ok(principal) => principal,
        Err(error) => return auth_error_response(error,true),
    };
    let page = match query.parse() { Ok(page) => page, Err(error) => return invalid_list_query(error) };
    if page.status.is_some() { return invalid_list_query("device_groups_have_no_status_filter"); }
    match state.auth.db().paged_accessible_device_groups(&principal.user_id,&page).await {
        Ok(groups) => Json(groups).into_response(),
        Err(_) => auth_error_response(AuthError::Internal,false),
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
        .list_api_device_group_members(&principal.user_id, principal.user.is_admin)
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
            principal.user.is_admin,
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
            principal.user.is_admin,
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
        Ok(true) => (StatusCode::OK, Json(serde_json::Value::Null)).into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "device_group_not_found" })),
        )
            .into_response(),
        Err(_) => auth_error_response(AuthError::Internal, false),
    }
}

async fn server_config_v2(
    Extension(state): Extension<Arc<ApiState>>,
    headers: HeaderMap,
) -> Response {
    server_config_response(&state, &headers, false).await
}

async fn server_config_response(
    state: &ApiState,
    headers: &HeaderMap,
    include_peers: bool,
) -> Response {
    if let Err(err) = authorize(state, headers).await {
        return auth_error_response(err, true);
    }
    let config = &state.server_config;
    let peers = if config.relay_server.is_empty() {
        Vec::new()
    } else {
        vec![config.relay_server.clone()]
    };
    let mut data = json!({
        "api_server": config.api_server.clone(),
        "id_server": config.id_server.clone(),
        "relay_server": config.relay_server.clone(),
        "key": config.key.clone(),
    });
    if include_peers {
        data["peers"] = json!(peers);
    }
    (StatusCode::OK, Json(json!({ "code": 0, "data": data }))).into_response()
}

async fn server_config(
    Extension(state): Extension<Arc<ApiState>>,
    headers: HeaderMap,
) -> Response {
    server_config_response(&state, &headers, true).await
}

fn address_book_entry_from_snapshot(
    user_id: &str,
    value: &serde_json::Value,
) -> Option<crate::database::ApiAddressBookEntry> {
    let peer_id = value
        .get("peerId")
        .or_else(|| value.get("peer_id"))
        .or_else(|| value.get("id"))
        .and_then(serde_json::Value::as_str)?
        .trim();
    if peer_id.is_empty() {
        return None;
    }
    let text = |key: &str| {
        value
            .get(key)
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    let tags = value
        .get("tags")
        .and_then(serde_json::Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(tag_name)
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let id = value
        .get("entryId")
        .or_else(|| value.get("guid"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or(peer_id)
        .to_owned();
    Some(crate::database::ApiAddressBookEntry {
        id,
        user_id: user_id.to_owned(),
        peer_id: peer_id.to_owned(),
        username: text("username"),
        hostname: text("hostname"),
        alias: text("alias"),
        platform: text("platform"),
        tags: serde_json::to_string(&tags).unwrap_or_else(|_| "[]".to_owned()),
        force_always_relay: value
            .get("forceAlwaysRelay")
            .or_else(|| value.get("force_always_relay"))
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false) as i64,
        created_at: text("createdAt"),
        updated_at: text("updatedAt"),
    })
}

fn snapshot_peer_value(entry: &crate::database::ApiAddressBookEntry) -> serde_json::Value {
    let mut value = address_book_entry_response(entry);
    value["id"] = json!(entry.peer_id);
    value["peerId"] = json!(entry.peer_id);
    value["entryId"] = json!(entry.id);
    value
}

fn merge_address_book_entry(
    document: &mut serde_json::Map<String, serde_json::Value>,
    entry: &crate::database::ApiAddressBookEntry,
) {
    let peers = document
        .entry("peers".to_owned())
        .or_insert_with(|| serde_json::Value::Array(Vec::new()));
    let peers = match peers.as_array_mut() {
        Some(peers) => peers,
        None => return,
    };
    let value = snapshot_peer_value(entry);
    if let Some(existing) = peers.iter_mut().find(|peer| snapshot_peer_id(peer)==Some(entry.peer_id.as_str())) {
        if let (Some(existing),Some(fields)) = (existing.as_object_mut(),value.as_object()) {
            existing.extend(fields.clone());
        }
    } else {
        peers.push(value);
    }
}

fn remove_address_book_entry_from_document(
    document: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
) {
    if let Some(peers) = document
        .get_mut("peers")
        .and_then(serde_json::Value::as_array_mut)
    {
        peers.retain(|peer| {
            snapshot_peer_id(peer) != Some(key)
        });
    }
}

fn entry_response_with_snapshot(document: &serde_json::Map<String,serde_json::Value>, entry: &crate::database::ApiAddressBookEntry) -> serde_json::Value {
    let mut value = document.get("peers").and_then(serde_json::Value::as_array)
        .and_then(|peers|peers.iter().find(|peer|snapshot_peer_id(peer)==Some(entry.peer_id.as_str())))
        .and_then(serde_json::Value::as_object).cloned().unwrap_or_default();
    if let Some(fields) = address_book_entry_response(entry).as_object() { value.extend(fields.clone()); }
    serde_json::Value::Object(value)
}

fn snapshot_peer_id(peer: &serde_json::Value) -> Option<&str> {
    peer.get("peerId").or_else(||peer.get("peer_id")).or_else(||peer.get("id")).and_then(serde_json::Value::as_str)
}

fn address_books(state: &ApiState) -> AddressBookStore { AddressBookStore::new(state.auth.db().clone()) }
fn book_error_response(error: BookError) -> Response {
    match error {
        BookError::Conflict => (StatusCode::CONFLICT,Json(json!({"error":"address_book_revision_conflict"}))).into_response(),
        BookError::Invalid(report) => (StatusCode::BAD_REQUEST,Json(json!({"error":report}))).into_response(),
        BookError::Storage => auth_error_response(AuthError::Internal,false),
    }
}
fn book_revision(headers: &HeaderMap, requested: Option<i64>, current: i64) -> Result<i64,BookError> {
    match requested {
        Some(revision) if revision<1 => Err(BookError::Invalid("invalid_address_book_revision".to_owned())),
        Some(revision) if revision!=current => Err(BookError::Conflict),
        Some(revision) => Ok(revision),
        None if headers.contains_key(header::AUTHORIZATION) => Ok(current),
        None => Err(BookError::Conflict),
    }
}
#[derive(Deserialize)]
struct BookRevision { revision: Option<i64> }
fn book_entries(user: &str, document: &serde_json::Map<String,serde_json::Value>) -> Vec<crate::database::ApiAddressBookEntry> {
    document.get("peers").and_then(serde_json::Value::as_array).into_iter().flatten()
        .filter_map(|peer|address_book_entry_from_snapshot(user,peer)).collect()
}

async fn get_address_book(Extension(state): Extension<Arc<ApiState>>, headers: HeaderMap) -> Response {
    let principal = match authorize(&state,&headers).await { Ok(principal) => principal, Err(error) => return auth_error_response(error,true) };
    let book = match address_books(&state).load(&principal.user_id).await { Ok(book) => book, Err(error) => return book_error_response(error) };
    let mut document = book.document;
    crate::address_book_codec::official_relays(&mut document);
    if let Err(error) = crate::address_book_codec::legacy_colors(&mut document) { return invalid_list_query(error); }
    match serde_json::to_string(&document) {
        Ok(data) => Json(json!({"data":data,"guid":book.guid,"revision":book.revision})).into_response(),
        Err(_) => auth_error_response(AuthError::Internal,false),
    }
}

async fn update_address_book(Extension(state): Extension<Arc<ApiState>>, headers: HeaderMap, Json(request): Json<AddressBookRequest>) -> Response {
    let principal = match authorize(&state,&headers).await { Ok(principal) => principal, Err(error) => return auth_error_response(error,true) };
    let _guard = state.tag_lock.lock().await;
    if request.data.len()>2*1024*1024 { return (StatusCode::PAYLOAD_TOO_LARGE,Json(json!({"error":"address_book_too_large"}))).into_response(); }
    let document = match serde_json::from_str(&request.data) { Ok(serde_json::Value::Object(document)) => document, _ => return invalid_list_query("invalid_address_book") };
    if request.revision.is_none() && !headers.contains_key(header::AUTHORIZATION) { return book_error_response(BookError::Conflict); }
    if request.revision.is_some_and(|revision|revision<1) { return invalid_list_query("invalid_address_book_revision"); }
    // Native legacy uploads intentionally replace the entire book; Web supplies a revision.
    match address_books(&state).replace(&principal.user_id,request.revision,document).await {
        Ok(_) => StatusCode::OK.into_response(),
        Err(error) => book_error_response(error),
    }
}

async fn delete_web_address_book_entry(Extension(state): Extension<Arc<ApiState>>, headers: HeaderMap, Path(key): Path<String>, Query(query): Query<BookRevision>) -> Response {
    let principal = match authorize(&state,&headers).await { Ok(principal) => principal, Err(error) => return auth_error_response(error,true) };
    let _guard = state.tag_lock.lock().await;
    deleted_book_response(remove_address_book_entry(&state,&principal.user_id,&key,&headers,query.revision).await)
}

async fn list_address_book_entries(Extension(state): Extension<Arc<ApiState>>, headers: HeaderMap) -> Response {
    let principal = match authorize(&state,&headers).await { Ok(principal) => principal, Err(error) => return auth_error_response(error,true) };
    let book = match address_books(&state).load(&principal.user_id).await { Ok(book) => book, Err(error) => return book_error_response(error) };
    listed_book_response(&principal.user_id,&book)
}
fn listed_book_response(user: &str, book: &Book) -> Response {
    let data = book_entries(user,&book.document).iter().map(|entry|entry_response_with_snapshot(&book.document,entry)).collect::<Vec<_>>();
    Json(json!({"code":0,"data":data,"guid":book.guid,"revision":book.revision})).into_response()
}

async fn post_address_book_entries(Extension(state): Extension<Arc<ApiState>>, headers: HeaderMap, Json(payload): Json<serde_json::Value>) -> Response {
    let principal = match authorize(&state,&headers).await { Ok(principal) => principal, Err(error) => return auth_error_response(error,true) };
    let _guard = state.tag_lock.lock().await;
    let revision = payload.get("revision").and_then(serde_json::Value::as_i64);
    let peers = if payload.is_array() { payload } else { match payload.get("peers") { Some(peers) if peers.is_array() => peers.clone(), _ => return invalid_list_query("invalid_address_book_peers") } };
    let book = match address_books(&state).load(&principal.user_id).await { Ok(book) => book, Err(error) => return book_error_response(error) };
    let revision = match book_revision(&headers,revision,book.revision) { Ok(revision) => revision, Err(error) => return book_error_response(error) };
    let mut document = book.document; document.insert("peers".to_owned(),peers);
    match address_books(&state).replace(&principal.user_id,Some(revision),document).await {
        Ok(book) => listed_book_response(&principal.user_id,&book),
        Err(error) => book_error_response(error),
    }
}

async fn upsert_address_book_entry(
    Extension(state): Extension<Arc<ApiState>>,
    headers: HeaderMap,
    Json(request): Json<AddressBookEntryRequest>,
) -> Response {
    let principal = match authorize(&state, &headers).await {
        Ok(principal) => principal,
        Err(err) => return auth_error_response(err, true),
    };
    if request.extra.keys().any(|key|matches!(key.as_str(),"peerId"|"entryId"|"guid"|"user_id"|"createdAt"|"updatedAt")) {
        return invalid_list_query("reserved_address_book_field");
    }
    let _tag_guard = state.tag_lock.lock().await;
    let book = match address_books(&state).load(&principal.user_id).await { Ok(book) => book, Err(error) => return book_error_response(error) };
    let revision = match book_revision(&headers,request.revision,book.revision) { Ok(revision) => revision, Err(error) => return book_error_response(error) };
    let mut document = book.document;
    let entries = book_entries(&principal.user_id,&document);
    let indexed = entries.iter().find(|entry|request.id.as_ref().map_or(entry.peer_id==request.peer_id.trim(),|id|entry.id==*id)).cloned();
    let snapshot = document.get("peers").and_then(serde_json::Value::as_array)
        .and_then(|peers|peers.iter().find(|peer| {
            if let Some(id) = request.id.as_deref() {
                let stable = peer.get("entryId").or_else(||peer.get("guid")).and_then(serde_json::Value::as_str);
                stable==Some(id) || (stable.is_none() && snapshot_peer_id(peer)==Some(id))
            } else { snapshot_peer_id(peer)==Some(request.peer_id.trim()) }
        }))
        .and_then(|peer| {
            let mut entry = address_book_entry_from_snapshot(&principal.user_id,peer)?;
            if peer.get("entryId").or_else(||peer.get("guid")).is_none() { entry.id = uuid::Uuid::new_v4().to_string(); }
            Some(entry)
        });
    let existing = indexed.or(snapshot);
    if request.id.is_some() && existing.is_none() {
        return (StatusCode::NOT_FOUND,Json(json!({"error":"address_book_entry_not_found"}))).into_response();
    }
    let peer_id = if request.peer_id.trim().is_empty() { existing.as_ref().map(|entry|entry.peer_id.clone()).unwrap_or_default() }
        else { request.peer_id.trim().to_owned() };
    if peer_id.is_empty() || peer_id.chars().count()>128 { return invalid_list_query("invalid_peer_id"); }
    let mut entry = existing.unwrap_or_else(||crate::database::ApiAddressBookEntry {
        id:uuid::Uuid::new_v4().to_string(),user_id:principal.user_id,peer_id:peer_id.clone(),
        username:String::new(),hostname:String::new(),alias:String::new(),platform:String::new(),
        tags:"[]".to_owned(),force_always_relay:0,created_at:String::new(),updated_at:String::new(),
    });
    if entry.peer_id != peer_id {
        let duplicate = entries.iter().any(|other|other.peer_id==peer_id && other.id!=entry.id)
            || document.get("peers").and_then(serde_json::Value::as_array).is_some_and(|peers|peers.iter().any(|peer|snapshot_peer_id(peer)==Some(peer_id.as_str())));
        if duplicate { return (StatusCode::CONFLICT,Json(json!({"error":"address_book_entry_conflict"}))).into_response(); }
        if let Some(peer) = document.get_mut("peers").and_then(serde_json::Value::as_array_mut)
            .and_then(|peers|peers.iter_mut().find(|peer|snapshot_peer_id(peer)==Some(entry.peer_id.as_str()))).and_then(serde_json::Value::as_object_mut) {
            peer.insert("id".to_owned(),json!(peer_id)); peer.insert("peerId".to_owned(),json!(peer_id));
            if peer.contains_key("peer_id") { peer.insert("peer_id".to_owned(),json!(peer_id)); }
        }
        entry.peer_id = peer_id.clone();
    }
    if let Some(username) = request.username { entry.username = username; }
    if let Some(hostname) = request.hostname { entry.hostname = hostname; }
    if let Some(alias) = request.alias { entry.alias = alias; }
    if let Some(platform) = request.platform { entry.platform = platform; }
    if let Some(tags) = request.tags {
        if tags.len()>64 || tags.iter().any(|tag|tag.trim().is_empty() || tag.chars().count()>64) {
            return invalid_list_query("invalid_address_book_tags");
        }
        entry.tags = match serde_json::to_string(&tags.into_iter().map(|tag|tag.trim().to_owned()).collect::<Vec<_>>()) {
            Ok(tags) => tags, Err(_) => return auth_error_response(AuthError::Internal,false),
        };
    }
    if let Some(relay) = request.force_always_relay { entry.force_always_relay = i64::from(relay); }
    if entry.id.is_empty() || entry.id.chars().count()>128 || entry.username.chars().count()>256
        || entry.hostname.chars().count()>256 || entry.alias.chars().count()>256 || entry.platform.chars().count()>128 {
        return invalid_list_query("invalid_address_book_entry");
    }
    let updated_at = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.f").to_string();
    if entry.created_at.is_empty() { entry.created_at = updated_at.clone(); }
    entry.updated_at = updated_at;
    merge_address_book_entry(&mut document,&entry);
    if let Some(peer) = document.get_mut("peers").and_then(serde_json::Value::as_array_mut)
        .and_then(|peers|peers.iter_mut().find(|peer|snapshot_peer_id(peer)==Some(peer_id.as_str()))).and_then(serde_json::Value::as_object_mut) {
        peer.extend(request.extra);
    } else { return invalid_list_query("invalid_address_book_peers"); }
    match address_books(&state).replace(&entry.user_id,Some(revision),document).await {
        Ok(book) => {
            let saved = book_entries(&entry.user_id,&book.document).into_iter().find(|saved|saved.peer_id==entry.peer_id);
            match saved {
                Some(saved) => Json(json!({"code":0,"data":entry_response_with_snapshot(&book.document,&saved),"revision":book.revision,"guid":book.guid})).into_response(),
                None => auth_error_response(AuthError::Internal,false),
            }
        },
        Err(error) => book_error_response(error),
    }
}

async fn remove_address_book_entry(state: &ApiState, user: &str, key: &str, headers: &HeaderMap, requested: Option<i64>) -> Result<Option<Book>,BookError> {
    let book = address_books(state).load(user).await?;
    let revision = book_revision(headers,requested,book.revision)?;
    let mut document = book.document;
    let entries = book_entries(user,&document);
    let entry = entries.iter().find(|entry|entry.id==key)
        .or_else(||entries.iter().find(|entry|entry.peer_id==key));
    let entry = match entry { Some(entry) => entry, None => return Ok(None) };
    remove_address_book_entry_from_document(&mut document,&entry.peer_id);
    address_books(state).replace(user,Some(revision),document).await.map(Some)
}

async fn delete_address_book_entry(Extension(state): Extension<Arc<ApiState>>, headers: HeaderMap, Json(request): Json<AddressBookEntryDeleteRequest>) -> Response {
    let principal = match authorize(&state,&headers).await { Ok(principal) => principal, Err(error) => return auth_error_response(error,true) };
    let _guard = state.tag_lock.lock().await;
    deleted_book_response(remove_address_book_entry(&state,&principal.user_id,&request.id,&headers,request.revision).await)
}
fn deleted_book_response(result: Result<Option<Book>,BookError>) -> Response {
    match result {
        Ok(Some(book)) => Json(json!({"revision":book.revision})).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND,Json(json!({"error":"address_book_entry_not_found"}))).into_response(),
        Err(error) => book_error_response(error),
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
    let book = match address_books(&state).load(&principal.user_id).await { Ok(book) => book, Err(error) => return book_error_response(error) };
    let document = book.document;
    let colors = document.get("tag_colors").and_then(serde_json::Value::as_object);
    let mut tags = crate::address_book_codec::official_tag_values(&document);
    for tag in &mut tags {
        let color = tag["name"].as_str().and_then(|name|colors.and_then(|colors|colors.get(name)))
            .and_then(serde_json::Value::as_u64).and_then(|color|u32::try_from(color).ok())
            .map(crate::address_book_codec::web_color).unwrap_or_default();
        tag["color"] = json!(color);
    }
    (StatusCode::OK, Json(json!({ "code": 0, "data": tags,"revision":book.revision,"guid":book.guid }))).into_response()
}

async fn upsert_tag(Extension(state): Extension<Arc<ApiState>>, headers: HeaderMap, Json(request): Json<TagRequest>) -> Response {
    let principal = match authorize(&state,&headers).await { Ok(principal) => principal, Err(error) => return auth_error_response(error,true) };
    let _tag_guard = state.tag_lock.lock().await;
    let name = request.name.trim();
    if name.is_empty() || name.chars().count()>64 { return invalid_list_query("invalid_tag"); }
    let color = match request.color.as_deref().map(str::trim) {
        Some("") => Some(None),
        Some(value) => match crate::address_book_codec::css_color(value) { Ok(color) => Some(Some(color)), Err(error) => return invalid_list_query(error) },
        None => None,
    };
    let book = match address_books(&state).load(&principal.user_id).await { Ok(book) => book, Err(error) => return book_error_response(error) };
    let revision = match book_revision(&headers,request.revision,book.revision) { Ok(revision) => revision, Err(error) => return book_error_response(error) };
    let mut document = book.document;
    let tags = document.entry("tags".to_owned()).or_insert_with(||json!([]));
    let tags = match tags.as_array_mut() { Some(tags) => tags, None => return invalid_list_query("invalid_tags") };
    let source = request.old_name.as_deref().unwrap_or(name).trim();
    let position = tags.iter().position(|tag|tag_name(tag).is_some_and(|tag|tag.eq_ignore_ascii_case(source)));
    if request.old_name.is_some() && position.is_none() { return (StatusCode::NOT_FOUND,Json(json!({"error":"tag_not_found"}))).into_response(); }
    if tags.iter().enumerate().any(|(index,tag)|Some(index)!=position && tag_name(tag).is_some_and(|tag|tag.eq_ignore_ascii_case(name))) {
        return (StatusCode::CONFLICT,Json(json!({"error":"tag_exists"}))).into_response();
    }
    let old = if let Some(position) = position {
        let old = tag_name(&tags[position]).unwrap_or(source).to_owned();
        if tags[position].is_object() { tags[position]["name"] = json!(name); } else { tags[position] = json!(name); }
        old
    } else { tags.push(json!(name)); name.to_owned() };
    let colors = match document.get_mut("tag_colors").and_then(serde_json::Value::as_object_mut) { Some(colors) => colors, None => return invalid_list_query("invalid_tag_colors") };
    let keys = colors.keys().filter(|key|key.eq_ignore_ascii_case(&old)).cloned().collect::<Vec<_>>();
    if keys.len()>1 { return invalid_list_query("ambiguous_tag_colors"); }
    let previous = keys.first().and_then(|key|colors.remove(key));
    if let Some(previous) = previous { colors.insert(name.to_owned(),previous); }
    if let Some(color) = color { if let Some(color) = color { colors.insert(name.to_owned(),json!(color)); } else { colors.remove(name); } }
    if let Err(error) = crate::address_book_codec::rewrite_tag_refs(&mut document,&old,Some(name)) { return invalid_list_query(error); }
    match address_books(&state).replace(&principal.user_id,Some(revision),document.clone()).await {
        Ok(book) => Json(json!({"revision":book.revision,"name":name,"color":document["tag_colors"].get(name).and_then(serde_json::Value::as_u64).and_then(|color|u32::try_from(color).ok()).map(crate::address_book_codec::web_color).unwrap_or_default()})).into_response(),
        Err(error) => book_error_response(error),
    }
}

async fn delete_tag(Extension(state): Extension<Arc<ApiState>>, headers: HeaderMap, Json(request): Json<TagDeleteRequest>) -> Response {
    let principal = match authorize(&state,&headers).await { Ok(principal) => principal, Err(error) => return auth_error_response(error,true) };
    let _tag_guard = state.tag_lock.lock().await;
    let name = request.name.trim();
    if name.is_empty() || name.chars().count()>64 { return invalid_list_query("invalid_tag"); }
    let book = match address_books(&state).load(&principal.user_id).await { Ok(book) => book, Err(error) => return book_error_response(error) };
    let revision = match book_revision(&headers,request.revision,book.revision) { Ok(revision) => revision, Err(error) => return book_error_response(error) };
    let mut document = book.document;
    if let Some(tags) = document.get_mut("tags").and_then(serde_json::Value::as_array_mut) { tags.retain(|tag|!tag_name(tag).is_some_and(|tag|tag.eq_ignore_ascii_case(name))); }
    if let Some(colors) = document.get_mut("tag_colors").and_then(serde_json::Value::as_object_mut) { colors.retain(|key,_|!key.eq_ignore_ascii_case(name)); }
    if let Err(error) = crate::address_book_codec::rewrite_tag_refs(&mut document,name,None) { return invalid_list_query(error); }
    match address_books(&state).replace(&principal.user_id,Some(revision),document).await {
        Ok(book) => Json(json!({"revision":book.revision})).into_response(),
        Err(error) => book_error_response(error),
    }
}

fn address_book_entry_response(entry: &crate::database::ApiAddressBookEntry) -> serde_json::Value {
    let tags = serde_json::from_str::<serde_json::Value>(&entry.tags)
        .unwrap_or_else(|_| serde_json::Value::Array(Vec::new()));
    json!({
        "id": entry.id,
        "peerId": entry.peer_id,
        "username": entry.username,
        "hostname": entry.hostname,
        "alias": entry.alias,
        "platform": entry.platform,
        "tags": tags,
        "forceAlwaysRelay": entry.force_always_relay != 0,
        "createdAt": entry.created_at,
        "updatedAt": entry.updated_at,
    })
}

fn tag_name(value: &serde_json::Value) -> Option<&str> {
    value
        .as_str()
        .or_else(|| value.get("name").and_then(serde_json::Value::as_str))
}

async fn authorize(state: &ApiState, headers: &HeaderMap) -> Result<Principal, AuthError> {
    let token = if let Some(value) = headers.get(header::AUTHORIZATION) {
        value.to_str().ok().and_then(|value| value.strip_prefix("Bearer "))
            .filter(|token| !token.is_empty()).ok_or(AuthError::InvalidCredentials)?
    } else {
        cookie_value(headers.get(header::COOKIE), "rustdesk_api_token").ok_or(AuthError::InvalidCredentials)?
    };
    state.auth.authorize(token).await
}

fn cookie_value<'a>(value: Option<&'a HeaderValue>, name: &str) -> Option<&'a str> {
    value
        .and_then(|value| value.to_str().ok())
        .and_then(|cookies| {
            cookies.split(';').find_map(|cookie| {
                let (cookie_name, cookie_value) = cookie.trim().split_once('=')?;
                (cookie_name == name && !cookie_value.is_empty()).then_some(cookie_value)
            })
        })
}

fn with_auth_cookie(policy: &CookiePolicy, mut response: Response, token: &str, expires_in: u64) -> Response {
    let cookie = policy.cookie("rustdesk_api_token", token, expires_in);
    if let Ok(value) = HeaderValue::from_str(&cookie) {
        response.headers_mut().append(header::SET_COOKIE, value);
    }
    response
}

fn clear_auth_cookie(policy: &CookiePolicy, response: Response) -> Response {
    with_auth_cookie(policy, response, "", 0)
}

async fn oauth_login(
    Extension(state): Extension<Arc<ApiState>>,
    Query(query): Query<OAuthQuery>,
) -> Response {
    if state.oauth_redirect_url.is_empty() {
        return oauth_error_response(OAuthError::NotConfigured);
    }
    let binding = uuid::Uuid::new_v4().to_string();
    match state
        .oauth
        .begin_with_device(&query.provider, &state.oauth_redirect_url, crate::oauth::OAuthDevice::default(), &binding)
        .await
    {
        Ok((url, flow)) => with_oauth_binding(&state.cookie_policy,
            Redirect::temporary(url.as_str()).into_response(), &flow, &binding, 300),
        Err(err) => oauth_error_response(err),
    }
}

async fn oauth_login_post(
    Extension(state): Extension<Arc<ApiState>>,
    Json(request): Json<OAuthAuthRequest>,
) -> Response {
    let _guard = state.provider_admin.lock().await;
    if state.oauth_redirect_url.is_empty() { return oauth_error_response(OAuthError::NotConfigured); }
    let op = request.op.strip_prefix("oidc/").unwrap_or(&request.op);
    let provider_alias = request.provider.strip_prefix("oidc/").unwrap_or(&request.provider);
    if !op.is_empty() && !provider_alias.is_empty() && op != provider_alias {
        return native_error("conflicting_oauth_provider");
    }
    let provider = if op.is_empty() { provider_alias } else { op };
    if !state.oauth.provider_names().iter().any(|name| name == provider) {
        return oauth_error_response(OAuthError::NotConfigured);
    }
    let mut url = match reqwest::Url::parse(&state.oauth_redirect_url) {
        Ok(url) => url,
        Err(_) => return oauth_error_response(OAuthError::NotConfigured),
    };
    let info = request.device_info.unwrap_or_default();
    let device = crate::oauth::OAuthDevice { id: request.id, uuid: request.uuid,
        name: info.name, os: info.os, device_type: info.device_type };
    let (code, launch) = match state.native_oauth.issue(provider.to_owned(), device).await {
        Ok(result) => result,
        Err(error) => return native_error(error),
    };
    url.set_path("/api/oidc/native");
    url.set_query(None);
    url.set_fragment(None);
    url.query_pairs_mut().append_pair("launch", &launch);
    Json(json!({"code":code,"url":url.as_str()})).into_response()
}

#[derive(Deserialize)]
struct NativeLaunchQuery { launch: String }

async fn oauth_native_browser(
    Extension(state): Extension<Arc<ApiState>>,
    Query(query): Query<NativeLaunchQuery>,
) -> Response {
    let (code, provider, device) = match state.native_oauth.launch(&query.launch).await {
        Ok(result) => result,
        Err(error) => return native_error(error),
    };
    let binding = uuid::Uuid::new_v4().to_string();
    match state.oauth.begin_native(&provider, &state.oauth_redirect_url, device, &binding).await {
        Ok((url, flow)) => {
            if let Err(error) = state.native_oauth.bind(&code, flow.clone()).await { return native_error(error); }
            with_oauth_binding(&state.cookie_policy, Redirect::temporary(url.as_str()).into_response(), &flow, &binding, 300)
        }
        Err(error) => {
            state.native_oauth.fail_start(&code).await;
            oauth_error_response(error)
        }
    }
}

async fn oauth_auth_query(
    Extension(state): Extension<Arc<ApiState>>,
    Query(query): Query<OAuthQuery>,
) -> Response {
    let identity = match state.native_oauth.poll(&query.code, &query.id, &query.uuid).await {
        Ok(identity) => identity,
        Err(error) => return Json(json!({"error":error})).into_response(),
    };
    let _guard = state.provider_admin.lock().await;
    if !state.oauth.identity_is_current(&identity) { return native_error("oauth_provider_changed"); }
    match state.auth.login_external(&identity.provider, &identity.subject, &identity.username, &identity.email,
        LoginDevice { id: identity.device.id, uuid: identity.device.uuid, name: identity.device.name,
            os: identity.device.os, device_type: identity.device.device_type }).await
    {
        Ok(result) => Json(result).into_response(),
        Err(error) => auth_error_response(error, false),
    }
}

fn native_error(error: &str) -> Response {
    (StatusCode::BAD_REQUEST, Json(json!({"error":error}))).into_response()
}

async fn oauth_message() -> impl IntoResponse {
    Json(json!({ "code": 0, "data": "" }))
}

async fn oauth_callback(
    Extension(state): Extension<Arc<ApiState>>,
    headers: HeaderMap,
    Query(query): Query<OAuthQuery>,
) -> Response {
    if state.oauth_redirect_url.is_empty() {
        return oauth_error_response(OAuthError::NotConfigured);
    }
    let binding_name = oauth_binding_name(&query.state);
    let binding = cookie_value(headers.get(header::COOKIE), &binding_name).unwrap_or_default();
    if !query.error.is_empty() {
        match state.oauth.cancel(&query.state, &state.oauth_redirect_url, binding).await {
            Ok(OAuthFlowKind::Native) => {
                let _ = state.native_oauth.finish(&query.state, Err("authorization_denied")).await;
                return with_oauth_binding(&state.cookie_policy, native_error("authorization_denied"), &query.state, "", 0);
            }
            Ok(OAuthFlowKind::Browser) => return with_oauth_binding(&state.cookie_policy,
                oauth_error_response(OAuthError::InvalidState), &query.state, "", 0),
            Err(error) => return oauth_error_response(error),
        }
    }
    let identity = match state
        .oauth
        .complete(
            &query.code,
            &query.state,
            &state.oauth_redirect_url,
            binding,
        )
        .await
    {
        Ok(identity) => identity,
        Err(err) => {
            if state.native_oauth.is_authorizing(&query.state).await {
                // Only exchange errors follow a successful binding claim; invalid callbacks cannot cancel a flow.
                if !matches!(err, OAuthError::InvalidState) {
                    let _ = state.native_oauth.finish(&query.state, Err("oauth_provider_failed")).await;
                }
            }
            return oauth_error_response(err);
        }
    };
    if !query.provider.is_empty() && query.provider != identity.provider_name {
        if identity.flow == OAuthFlowKind::Native {
            let _ = state.native_oauth.finish(&query.state, Err("conflicting_oauth_provider")).await;
        }
        return oauth_error_response(OAuthError::InvalidState);
    }
    let _guard = state.provider_admin.lock().await;
    if !state.oauth.identity_is_current(&identity) { return oauth_error_response(OAuthError::InvalidState); }
    if identity.flow == OAuthFlowKind::Native {
        let response = match state.native_oauth.finish(&query.state, Ok(identity)).await {
            Ok(()) => (StatusCode::OK, "Authorization complete. Return to RustDesk.").into_response(),
            Err(error) => native_error(error),
        };
        return with_oauth_binding(&state.cookie_policy, response, &query.state, "", 0);
    }
    let response = match state
        .auth
        .login_external(
            &identity.provider,
            &identity.subject,
            &identity.username,
            &identity.email,
            LoginDevice {
                id: identity.device.id,
                uuid: identity.device.uuid,
                name: identity.device.name,
                os: identity.device.os,
                device_type: identity.device.device_type,
            },
        )
        .await
    {
        Ok(result) => {
            let accepts_html = headers
                .get(header::ACCEPT)
                .and_then(|value| value.to_str().ok())
                .map(|value| value.contains("text/html"))
                .unwrap_or(false);
            if accepts_html {
                let response = Redirect::temporary("/").into_response();
                with_auth_cookie(&state.cookie_policy, response, &result.access_token, result.expires_in)
            } else {
                with_auth_cookie(
                    &state.cookie_policy,
                    (StatusCode::OK, Json(result.clone())).into_response(),
                    &result.access_token,
                    result.expires_in,
                )
            }
        }
        Err(err) => auth_error_response(err, false),
    };
    with_oauth_binding(&state.cookie_policy, response, &query.state, "", 0)
}

fn oauth_binding_name(state: &str) -> String {
    // Invalid state never reaches a successful callback, but keep header names bounded.
    let suffix = state.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').take(64).collect::<String>();
    format!("rustdesk_oauth_{suffix}")
}

fn with_oauth_binding(policy: &CookiePolicy, mut response: Response, state: &str, binding: &str, max_age: u64) -> Response {
    if let Ok(value) = HeaderValue::from_str(&policy.cookie(&oauth_binding_name(state), binding, max_age)) {
        response.headers_mut().append(header::SET_COOKIE, value);
    }
    response
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


#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, peer_id: &str, tags: &[&str]) -> crate::database::ApiAddressBookEntry {
        crate::database::ApiAddressBookEntry {
            id: id.to_owned(),
            user_id: "user-1".to_owned(),
            peer_id: peer_id.to_owned(),
            username: "alice".to_owned(),
            hostname: "workstation".to_owned(),
            alias: "Office".to_owned(),
            platform: "Linux".to_owned(),
            tags: serde_json::to_string(tags).expect("tags should serialize"),
            force_always_relay: 1,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    #[test]
    fn snapshot_conversion_round_trips_peer_fields() {
        let original = entry("entry-1", "peer-1", &["ops", "linux"]);
        let snapshot = snapshot_peer_value(&original);
        let parsed = address_book_entry_from_snapshot("user-1", &snapshot)
            .expect("snapshot peer should parse");

        assert_eq!(parsed.id, "entry-1");
        assert_eq!(parsed.peer_id, "peer-1");
        assert_eq!(parsed.tags, "[\"ops\",\"linux\"]");
        assert_eq!(parsed.force_always_relay, 1);
    }

    #[test]
    fn merge_replaces_existing_peer_without_duplicate() {
        let original = entry("entry-1", "peer-1", &["old"]);
        let replacement = entry("entry-2", "peer-1", &["new"]);
        let mut document = serde_json::Map::new();
        document.insert(
            "peers".to_owned(),
            serde_json::Value::Array(vec![snapshot_peer_value(&original)]),
        );

        merge_address_book_entry(&mut document, &replacement);

        let peers = document
            .get("peers")
            .and_then(serde_json::Value::as_array)
            .expect("peers should remain an array");
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0]["peerId"], "peer-1");
        assert_eq!(peers[0]["entryId"], "entry-2");
        assert_eq!(peers[0]["tags"], serde_json::json!(["new"]));
    }

    #[test]
    fn cookie_value_extracts_named_cookie_without_prefix_confusion() {
        let headers = HeaderMap::from_iter([(
            header::COOKIE,
            HeaderValue::from_static("other=one; rustdesk_api_token=token-123; rustdesk_api_token_extra=no"),
        )]);

        assert_eq!(
            cookie_value(headers.get(header::COOKIE), "rustdesk_api_token"),
            Some("token-123")
        );
        assert_eq!(cookie_value(headers.get(header::COOKIE), "missing"), None);
    }

    #[test]
    fn auth_cookie_response_sets_browser_session_attributes() {
        let response = with_auth_cookie(
            &CookiePolicy::default(),
            (StatusCode::OK, Json(json!({ "ok": true }))).into_response(),
            "token-123",
            900,
        );
        let cookie = response
            .headers()
            .get(header::SET_COOKIE)
            .and_then(|value| value.to_str().ok())
            .expect("auth cookie should be present");

        assert!(cookie.contains("rustdesk_api_token=token-123"));
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("SameSite=Lax"));
        assert!(cookie.contains("Max-Age=900"));
    }

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
