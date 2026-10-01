mod api_config;

use api_config::{
    load_ldap_config, load_oauth_redirect_url, load_oauth_runtime, load_public_key,
    parse_bool_arg,
};
use flexi_logger::*;
use hbb_common::{bail, log, tokio, ResultType};
use hbbs::{api, common, database::Database};
use std::{
    net::{IpAddr, SocketAddr},
    time::Duration,
};

const MAX_TOKEN_TTL_SECONDS: u64 = 7 * 24 * 60 * 60;

fn main() -> ResultType<()> {
    let _logger = Logger::try_with_env_or_str("info")?
        .log_to_stdout()
        .format(opt_format)
        .write_mode(WriteMode::Async)
        .start()?;
    common::init_args(
        "-c --config=[FILE] +takes_value 'Sets a custom config file'\n--initialize 'Prepare keys, migrations and API configuration without listening'",
        "rustdesk-api",
        "RustDesk HTTP API Server",
    );
    let initialize = std::env::args().any(|arg|arg=="--initialize");
    if common::get_arg_or("API_ENABLED", "0".to_owned()).to_lowercase() != "1" {
        if initialize { bail!("API_ENABLED=1 is required for initialization"); }
        log::info!("API_ENABLED=0, exiting without starting the API server");
        return Ok(());
    }
    let bind = common::get_arg_or("API_BIND", "127.0.0.1".to_owned());
    let port = common::get_arg_or("API_PORT", "21114".to_owned()).parse::<u16>()?;
    let bind_addr = SocketAddr::new(bind.parse::<IpAddr>()?, port);
    let secret = common::get_arg("API_JWT_SECRET");
    if secret.is_empty() {
        bail!("API_JWT_SECRET must be configured when the API server is enabled");
    }
    let token_ttl = common::get_arg_or("API_TOKEN_TTL", "3600".to_owned()).parse::<u64>()?;
    if !(60..=MAX_TOKEN_TTL_SECONDS).contains(&token_ttl) {
        bail!("API_TOKEN_TTL must be between 60 seconds and 7 days");
    }
    let registration_enabled = parse_bool_arg("API_REGISTER_ENABLED", false)?;
    let db_url = common::get_arg_or("DB_URL", "./db_v2.sqlite3".to_owned());
    if initialize { hbbs::deployment::prepare_keypair(&std::env::current_dir()?)?; }
    let key = load_public_key()?;
    let bootstrap_username = common::get_arg("API_BOOTSTRAP_ADMIN_USERNAME");
    let bootstrap_password = common::get_arg("API_BOOTSTRAP_ADMIN_PASSWORD");
    let bootstrap_admin = if bootstrap_username.is_empty() && bootstrap_password.is_empty() {
        None
    } else {
        Some((bootstrap_username, bootstrap_password))
    };
    let web_root = common::get_arg_or("API_WEB_ROOT", "./web/dist".to_owned());
    let oauth = load_oauth_runtime()?;
    let oauth_redirect_url = load_oauth_redirect_url();
    let ldap = load_ldap_config()?;
    let server_config = api::PublicServerConfig {
        api_server: common::get_arg_or(
            "API_PUBLIC_URL",
            format!("http://{bind_addr}"),
        ),
        id_server: common::get_arg("RUSTDESK_ID_SERVER"),
        relay_server: common::get_arg("RUSTDESK_RELAY_SERVER"),
        key,
    };
    let mut cookie_policy = if parse_bool_arg("API_ALLOW_INSECURE_LOCAL_HTTP", false)? {
        api::CookiePolicy::local_http(bind_addr.ip(), &server_config.api_server)
            .map_err(|message| hbb_common::anyhow::anyhow!(message))?
    } else {
        api::CookiePolicy::default()
    };
    if parse_bool_arg("API_COOKIE_CROSS_SITE", false)? {
        cookie_policy = cookie_policy.cross_site().map_err(|message| hbb_common::anyhow::anyhow!(message))?;
    }
    let allowed_origins = common::get_arg("API_ALLOWED_ORIGINS").split(',')
        .map(str::trim).filter(|origin| !origin.is_empty()).map(str::to_owned).collect::<Vec<_>>();
    let browser_policy = api::BrowserPolicy::new(&server_config.api_server, &allowed_origins)
        .map_err(|message| hbb_common::anyhow::anyhow!(message))?;
    let provider_key = match common::get_arg_opt("API_OAUTH_CONFIG_KEY") {
        Some(value) => {
            let bytes = base64::decode(&value)?;
            if bytes == secret.as_bytes() { bail!("API_OAUTH_CONFIG_KEY must be independent from API_JWT_SECRET"); }
            Some(hbbs::oauth_admin::ProviderSecretKey::from_bytes(&bytes).map_err(|message| hbb_common::anyhow::anyhow!(message))?)
        }
        None => None,
    };
    if initialize && provider_key.is_none() { bail!("API_OAUTH_CONFIG_KEY is required for container initialization"); }
    start(
        bind_addr,
        db_url,
        secret,
        Duration::from_secs(token_ttl),
        registration_enabled,
        server_config,
        web_root,
        bootstrap_admin,
        oauth,
        oauth_redirect_url,
        ldap,
        cookie_policy,
        provider_key,
        browser_policy,
        initialize,
    )
}

#[tokio::main(flavor = "multi_thread")]
async fn start(
    bind_addr: SocketAddr,
    db_url: String,
    secret: String,
    token_ttl: Duration,
    registration_enabled: bool,
    server_config: api::PublicServerConfig,
    web_root: String,
    bootstrap_admin: Option<(String, String)>,
    oauth: hbbs::oauth::OAuthRuntime,
    oauth_redirect_url: String,
    ldap: hbbs::ldap::LdapConfig,
    cookie_policy: api::CookiePolicy,
    provider_key: Option<hbbs::oauth_admin::ProviderSecretKey>,
    browser_policy: api::BrowserPolicy,
    initialize: bool,
) -> ResultType<()> {
    if initialize { hbbs::deployment::check_web_assets(std::path::Path::new(&web_root))?; }
    let database = Database::new(&db_url).await?;
    if initialize && bootstrap_admin.is_none() && database.api_user_count().await?==0 {
        bail!("API_BOOTSTRAP_ADMIN_USERNAME and API_BOOTSTRAP_ADMIN_PASSWORD are required for a new database");
    }
    database.check_api_readiness().await?;
    let router = api::build_service(
        database,
        secret,
        token_ttl,
        registration_enabled,
        server_config,
        web_root,
        bootstrap_admin,
        oauth,
        oauth_redirect_url,
        ldap,
        cookie_policy,
        provider_key,
        Some(browser_policy),
    )
        .await
        .map_err(|err| hbb_common::anyhow::anyhow!("failed to initialize API authentication: {err:?}"))?;
    if initialize { log::info!("RustDesk keys, API configuration and database initialization complete"); return Ok(()); }
    log::info!("RustDesk API listening on http://{bind_addr}");
    axum::serve(tokio::net::TcpListener::bind(bind_addr).await?, router.into_make_service())
        .await?;
    Ok(())
}
