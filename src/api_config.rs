use hbb_common::{bail, ResultType};
use hbbs::common;
use hbbs::ldap::LdapConfig;
use hbbs::oauth::{OAuthProviderConfig, OAuthProviderKind, OAuthRuntime};
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

pub(crate) fn load_oauth_runtime() -> ResultType<OAuthRuntime> {
    load_oauth_runtime_from(common::get_arg)
}

fn load_oauth_runtime_from(read: impl Fn(&str) -> String) -> ResultType<OAuthRuntime> {
    let mut configs = Vec::new();
    for (name, prefix, kind, auth, token, userinfo, issuer, jwks, scopes) in [
        ("github", "API_GITHUB", OAuthProviderKind::OAuth2,
            "https://github.com/login/oauth/authorize", "https://github.com/login/oauth/access_token",
            "https://api.github.com/user", "", "", "read:user user:email"),
        ("google", "API_GOOGLE", OAuthProviderKind::Oidc,
            "https://accounts.google.com/o/oauth2/v2/auth", "https://oauth2.googleapis.com/token",
            "https://openidconnect.googleapis.com/v1/userinfo", "https://accounts.google.com",
            "https://www.googleapis.com/oauth2/v3/certs", "openid email profile"),
    ] {
        let client_id = read(&format!("{prefix}_CLIENT_ID"));
        let client_secret = read(&format!("{prefix}_CLIENT_SECRET"));
        if !client_id.is_empty() || !client_secret.is_empty() {
            configs.push(OAuthProviderConfig {
                name: name.to_owned(), kind, client_id, client_secret,
                authorization_url: auth.to_owned(), token_url: token.to_owned(),
                userinfo_url: userinfo.to_owned(), issuer_url: issuer.to_owned(),
                jwks_url: jwks.to_owned(), scopes: scopes.to_owned(),
            });
        }
    }
    let oidc_keys = ["API_OIDC_CLIENT_ID", "API_OIDC_CLIENT_SECRET", "API_OIDC_AUTH_URL",
        "API_OIDC_TOKEN_URL", "API_OIDC_USERINFO_URL", "API_OIDC_ISSUER_URL", "API_OIDC_JWKS_URL", "API_OIDC_SCOPE"];
    if oidc_keys.iter().any(|key| !read(key).is_empty()) {
        let scopes = read("API_OIDC_SCOPE");
        configs.push(OAuthProviderConfig {
            name: "oidc".to_owned(), kind: OAuthProviderKind::Oidc,
            client_id: read("API_OIDC_CLIENT_ID"), client_secret: read("API_OIDC_CLIENT_SECRET"),
            authorization_url: read("API_OIDC_AUTH_URL"), token_url: read("API_OIDC_TOKEN_URL"),
            userinfo_url: read("API_OIDC_USERINFO_URL"), issuer_url: read("API_OIDC_ISSUER_URL"),
            jwks_url: read("API_OIDC_JWKS_URL"),
            scopes: if scopes.is_empty() { "openid email profile".to_owned() } else { scopes },
        });
    }
    OAuthRuntime::try_new(configs).map_err(|message| hbb_common::anyhow::anyhow!(message))
}

pub(crate) fn load_oauth_redirect_url() -> String {
    common::get_arg("API_OAUTH_REDIRECT_URL")
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


#[cfg(test)]
mod tests {
    use super::*;

    fn runtime_with(values: &[(&str, &str)]) -> ResultType<OAuthRuntime> {
        load_oauth_runtime_from(|key| values.iter().find(|(name, _)| *name == key)
            .map(|(_, value)| (*value).to_owned()).unwrap_or_default())
    }

    #[test]
    fn google_has_complete_oidc_validation_endpoints() {
        let runtime = runtime_with(&[("API_GOOGLE_CLIENT_ID", "client"), ("API_GOOGLE_CLIENT_SECRET", "secret")]).unwrap();
        let providers = runtime.provider_views();
        assert_eq!(providers[0].issuer_url, "https://accounts.google.com");
        assert_eq!(providers[0].jwks_url, "https://www.googleapis.com/oauth2/v3/certs");
    }

    #[test]
    fn incomplete_generic_oidc_configuration_is_an_explicit_error() {
        let mut values = vec![("API_OIDC_CLIENT_ID", "client"), ("API_OIDC_CLIENT_SECRET", "secret"),
            ("API_OIDC_AUTH_URL", "https://provider.example/authorize"),
            ("API_OIDC_TOKEN_URL", "https://provider.example/token"),
            ("API_OIDC_USERINFO_URL", "https://provider.example/userinfo")];
        assert!(runtime_with(&values).is_err());
        values.push(("API_OIDC_ISSUER_URL", "https://provider.example"));
        assert!(runtime_with(&values).is_err());
        values.push(("API_OIDC_JWKS_URL", "https://provider.example/jwks"));
        assert_eq!(runtime_with(&values).unwrap().provider_names(), vec!["oidc"]);
    }

    #[test]
    fn partial_credentials_fail_and_empty_configuration_is_valid() {
        assert!(runtime_with(&[("API_GOOGLE_CLIENT_ID", "client")]).is_err());
        assert!(runtime_with(&[]).unwrap().provider_names().is_empty());
        let runtime = runtime_with(&[("API_GITHUB_CLIENT_ID", "client"), ("API_GITHUB_CLIENT_SECRET", "secret")]).unwrap();
        assert_eq!(runtime.provider_views()[0].kind, hbbs::oauth::OAuthProviderKind::OAuth2);
    }
}
