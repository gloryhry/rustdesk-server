mod api_config;

use api_config::{load_public_key, parse_bool_arg};
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
        "-c --config=[FILE] +takes_value 'Sets a custom config file'",
        "rustdesk-api",
        "RustDesk HTTP API Server",
    );
    if common::get_arg_or("API_ENABLED", "0".to_owned()).to_lowercase() != "1" {
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
    let key = load_public_key()?;
    let bootstrap_username = common::get_arg("API_BOOTSTRAP_ADMIN_USERNAME");
    let bootstrap_password = common::get_arg("API_BOOTSTRAP_ADMIN_PASSWORD");
    let bootstrap_admin = if bootstrap_username.is_empty() && bootstrap_password.is_empty() {
        None
    } else {
        Some((bootstrap_username, bootstrap_password))
    };
    let server_config = api::PublicServerConfig {
        api_server: common::get_arg_or(
            "API_PUBLIC_URL",
            format!("http://{bind_addr}"),
        ),
        id_server: common::get_arg("RUSTDESK_ID_SERVER"),
        relay_server: common::get_arg("RUSTDESK_RELAY_SERVER"),
        key,
    };
    start(
        bind_addr,
        db_url,
        secret,
        Duration::from_secs(token_ttl),
        registration_enabled,
        bootstrap_admin,
        server_config,
    )
}

#[tokio::main(flavor = "multi_thread")]
async fn start(
    bind_addr: SocketAddr,
    db_url: String,
    secret: String,
    token_ttl: Duration,
    registration_enabled: bool,
    bootstrap_admin: Option<(String, String)>,
    server_config: api::PublicServerConfig,
) -> ResultType<()> {
    let database = Database::new(&db_url).await?;
    let router = api::build_service(
        database,
        secret,
        token_ttl,
        registration_enabled,
        bootstrap_admin,
        server_config,
    )
        .await
        .map_err(|err| hbb_common::anyhow::anyhow!("failed to initialize API authentication: {err:?}"))?;
    log::info!("RustDesk API listening on http://{bind_addr}");
    axum::Server::bind(&bind_addr)
        .serve(router.into_make_service())
        .await?;
    Ok(())
}
