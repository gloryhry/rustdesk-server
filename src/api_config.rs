use hbb_common::{bail, ResultType};
use hbbs::common;
use hbbs::ldap::LdapConfig;
use hbbs::oauth::{OAuthProviderConfig, OAuthRuntime};
use sodiumoxide::crypto::sign;

pub(crate) fn parse_bool_arg(name: &str, default: bool) -> ResultType<bool> {
    match common::get_arg_opt(name).as_deref() {
        None => Ok(default),
        Some("1" | "true" | "TRUE" | "yes" | "YES") => Ok(true),
        Some("0" | "false" | "FALSE" | "no" | "NO") => Ok(false),
        Some(value) => bail!("{name} must be a boolean, got {value}"),
    }
}

pub(crate) fn load_ldap_config() -> ResultType<LdapConfig> {
    let config = LdapConfig {
        enabled: parse_bool_arg("API_LDAP_ENABLED", false)?,
        url: common::get_arg("API_LDAP_URL"),
        bind_dn: common::get_arg("API_LDAP_BIND_DN"),
        bind_password: common::get_arg("API_LDAP_BIND_PASSWORD"),
        user_base_dn: common::get_arg("API_LDAP_USER_BASE_DN"),
        user_filter: common::get_arg_or(
            "API_LDAP_USER_FILTER",
            "(&(objectClass=person)(uid={username}))".to_owned(),
        ),
        username_attribute: common::get_arg_or(
            "API_LDAP_USERNAME_ATTRIBUTE",
            "uid".to_owned(),
        ),
        email_attribute: common::get_arg_or(
            "API_LDAP_EMAIL_ATTRIBUTE",
            "mail".to_owned(),
        ),
        use_tls: parse_bool_arg("API_LDAP_USE_TLS", false)?,
        timeout_seconds: common::get_arg_or("API_LDAP_TIMEOUT", "5".to_owned()).parse()?,
    };
    config
        .validate()
        .map_err(|error| match error {
            hbbs::ldap::LdapConfigError::Invalid(message) => hbb_common::anyhow::anyhow!(message),
        })?;
    Ok(config)
}

pub(crate) fn load_oauth_runtime() -> OAuthRuntime {
    let mut configs = Vec::new();
    add_provider(
        &mut configs,
        "github",
        "API_GITHUB_CLIENT_ID",
        "API_GITHUB_CLIENT_SECRET",
        "https://github.com/login/oauth/authorize",
        "https://github.com/login/oauth/access_token",
        "https://api.github.com/user",
        "read:user user:email",
    );
    add_provider(
        &mut configs,
        "google",
        "API_GOOGLE_CLIENT_ID",
        "API_GOOGLE_CLIENT_SECRET",
        "https://accounts.google.com/o/oauth2/v2/auth",
        "https://oauth2.googleapis.com/token",
        "https://openidconnect.googleapis.com/v1/userinfo",
        "openid email profile",
    );
    let oidc_auth = common::get_arg("API_OIDC_AUTH_URL");
    let oidc_token = common::get_arg("API_OIDC_TOKEN_URL");
    let oidc_userinfo = common::get_arg("API_OIDC_USERINFO_URL");
    let oidc_issuer = common::get_arg("API_OIDC_ISSUER_URL");
    let oidc_jwks = common::get_arg("API_OIDC_JWKS_URL");
    if !oidc_auth.is_empty() && !oidc_token.is_empty() && !oidc_userinfo.is_empty() {
        configs.push(OAuthProviderConfig {
            name: "oidc".to_owned(),
            client_id: common::get_arg("API_OIDC_CLIENT_ID"),
            client_secret: common::get_arg("API_OIDC_CLIENT_SECRET"),
            authorization_url: oidc_auth,
            token_url: oidc_token,
            userinfo_url: oidc_userinfo,
            issuer_url: oidc_issuer,
            jwks_url: oidc_jwks,
            scopes: common::get_arg_or("API_OIDC_SCOPE", "openid email profile".to_owned()),
        });
    }
    OAuthRuntime::new(configs)
}

pub(crate) fn load_oauth_redirect_url() -> String {
    common::get_arg("API_OAUTH_REDIRECT_URL")
}

fn add_provider(
    configs: &mut Vec<OAuthProviderConfig>,
    name: &str,
    client_id_name: &str,
    client_secret_name: &str,
    authorization_url: &str,
    token_url: &str,
    userinfo_url: &str,
    scopes: &str,
) {
    let client_id = common::get_arg(client_id_name);
    let client_secret = common::get_arg(client_secret_name);
    if !client_id.is_empty() && !client_secret.is_empty() {
        configs.push(OAuthProviderConfig {
            name: name.to_owned(),
            client_id,
            client_secret,
            authorization_url: authorization_url.to_owned(),
            token_url: token_url.to_owned(),
            userinfo_url: userinfo_url.to_owned(),
            issuer_url: String::new(),
            jwks_url: String::new(),
            scopes: scopes.to_owned(),
        });
    }
}
pub(crate) fn load_public_key() -> ResultType<String> {
    let value = match common::get_arg_opt("RUSTDESK_KEY") {
        Some(value) => value,
        None => {
            let path = common::get_arg_or("RUSTDESK_KEY_FILE", "id_ed25519.pub".to_owned());
            std::fs::read_to_string(path)?
        }
    };
    let decoded = base64::decode(value.trim())?;
    if decoded.len() != sign::PUBLICKEYBYTES {
        bail!("RUSTDESK_KEY must be a base64-encoded Ed25519 public key");
    }
    Ok(base64::encode(decoded))
}

