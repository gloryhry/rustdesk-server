use serde::Serialize;
use std::time::Duration;

#[derive(Debug, Clone, Serialize)]
pub struct LdapConfigView {
    pub enabled: bool,
    pub url: String,
    pub bind_dn: String,
    pub user_base_dn: String,
    pub user_filter: String,
    pub username_attribute: String,
    pub email_attribute: String,
    pub use_tls: bool,
    pub timeout_seconds: u64,
    pub configured: bool,
}

#[derive(Debug, Clone)]
pub struct LdapConfig {
    pub enabled: bool,
    pub url: String,
    pub bind_dn: String,
    pub bind_password: String,
    pub user_base_dn: String,
    pub user_filter: String,
    pub username_attribute: String,
    pub email_attribute: String,
    pub use_tls: bool,
    pub timeout_seconds: u64,
}

#[derive(Debug)]
pub enum LdapConfigError {
    Invalid(&'static str),
}

#[derive(Debug, Clone)]
pub struct LdapIdentity {
    pub username: String,
    pub email: String,
}

#[derive(Debug)]
pub enum LdapAuthError {
    InvalidConfiguration,
    InvalidCredentials,
    Unavailable,
}

impl LdapConfig {
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            url: String::new(),
            bind_dn: String::new(),
            bind_password: String::new(),
            user_base_dn: String::new(),
            user_filter: String::new(),
            username_attribute: "uid".to_owned(),
            email_attribute: "mail".to_owned(),
            use_tls: false,
            timeout_seconds: 5,
        }
    }

    pub fn validate(&self) -> Result<(), LdapConfigError> {
        if !self.enabled {
            return Ok(());
        }
        if !(self.url.starts_with("ldap://") || self.url.starts_with("ldaps://")) {
            return Err(LdapConfigError::Invalid("LDAP URL must use ldap:// or ldaps://"));
        }
        if self.user_base_dn.is_empty() {
            return Err(LdapConfigError::Invalid("LDAP user base DN is required"));
        }
        if self.user_filter.is_empty() || !self.user_filter.contains("{username}") {
            return Err(LdapConfigError::Invalid(
                "LDAP user filter must contain {username}",
            ));
        }
        if !valid_attribute(&self.username_attribute) || !valid_attribute(&self.email_attribute) {
            return Err(LdapConfigError::Invalid("LDAP attribute name is invalid"));
        }
        if !(1..=60).contains(&self.timeout_seconds) {
            return Err(LdapConfigError::Invalid("LDAP timeout must be between 1 and 60 seconds"));
        }
        Ok(())
    }

    pub async fn authenticate(
        &self,
        username: &str,
        password: &str,
    ) -> Result<Option<LdapIdentity>, LdapAuthError> {
        if !self.enabled {
            return Ok(None);
        }
        self.validate()
            .map_err(|_| LdapAuthError::InvalidConfiguration)?;
        if username.is_empty() || password.is_empty() {
            return Ok(None);
        }
        let timeout = Duration::from_secs(self.timeout_seconds);
        let settings = ldap3::LdapConnSettings::new()
            .set_conn_timeout(timeout)
            .set_starttls(self.use_tls && self.url.starts_with("ldap://"));
        let (connection, mut ldap) = ldap3::LdapConnAsync::with_settings(settings, &self.url)
            .await
            .map_err(|_| LdapAuthError::Unavailable)?;
        ldap3::drive!(connection);
        if !self.bind_dn.is_empty() {
            ldap.simple_bind(&self.bind_dn, &self.bind_password)
                .await
                .map_err(|_| LdapAuthError::Unavailable)?
                .success()
                .map_err(|_| LdapAuthError::Unavailable)?;
        }
        let escaped_username = ldap3::ldap_escape(username);
        let filter = self.user_filter.replace("{username}", escaped_username.as_ref());
        let attrs = vec![self.username_attribute.as_str(), self.email_attribute.as_str()];
        let (entries, _) = ldap
            .with_timeout(timeout)
            .search(&self.user_base_dn, ldap3::Scope::Subtree, &filter, attrs)
            .await
            .map_err(|_| LdapAuthError::Unavailable)?
            .success()
            .map_err(|_| LdapAuthError::Unavailable)?;
        let entry = match entries.into_iter().next() {
            Some(entry) => ldap3::SearchEntry::construct(entry),
            None => return Ok(None),
        };
        let resolved_username = entry
            .attrs
            .get(&self.username_attribute)
            .and_then(|values| values.first())
            .cloned()
            .unwrap_or_else(|| username.to_owned());
        let email = entry
            .attrs
            .get(&self.email_attribute)
            .and_then(|values| values.first())
            .cloned()
            .unwrap_or_default();
        if entry.dn.is_empty() {
            return Err(LdapAuthError::Unavailable);
        }
        let user_settings = ldap3::LdapConnSettings::new()
            .set_conn_timeout(timeout)
            .set_starttls(self.use_tls && self.url.starts_with("ldap://"));
        let (user_connection, mut user_ldap) =
            ldap3::LdapConnAsync::with_settings(user_settings, &self.url)
                .await
                .map_err(|_| LdapAuthError::Unavailable)?;
        ldap3::drive!(user_connection);
        let bind = user_ldap
            .with_timeout(timeout)
            .simple_bind(&entry.dn, password)
            .await
            .map_err(|_| LdapAuthError::Unavailable)?;
        match bind.success() {
            Ok(_) => Ok(Some(LdapIdentity {
                username: resolved_username,
                email,
            })),
            Err(ldap3::LdapError::LdapResult { result }) if result.rc == 49 => {
                Err(LdapAuthError::InvalidCredentials)
            }
            Err(_) => Err(LdapAuthError::Unavailable),
        }
    }

    pub fn view(&self) -> LdapConfigView {
        LdapConfigView {
            enabled: self.enabled,
            url: self.url.clone(),
            bind_dn: self.bind_dn.clone(),
            user_base_dn: self.user_base_dn.clone(),
            user_filter: self.user_filter.clone(),
            username_attribute: self.username_attribute.clone(),
            email_attribute: self.email_attribute.clone(),
            use_tls: self.use_tls,
            timeout_seconds: self.timeout_seconds,
            configured: !self.url.is_empty() && !self.bind_password.is_empty(),
        }
    }
}

fn valid_attribute(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_configuration_is_valid() {
        assert!(LdapConfig::disabled().validate().is_ok());
    }

    #[test]
    fn enabled_configuration_requires_username_placeholder() {
        let mut config = LdapConfig::disabled();
        config.enabled = true;
        config.url = "ldaps://directory.example".to_owned();
        config.user_base_dn = "ou=people,dc=example,dc=com".to_owned();
        config.user_filter = "(uid=alice)".to_owned();
        assert!(config.validate().is_err());
        config.user_filter = "(uid={username})".to_owned();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn view_never_contains_bind_password() {
        let mut config = LdapConfig::disabled();
        config.url = "ldaps://directory.example".to_owned();
        config.bind_password = "secret".to_owned();
        let serialized = serde_json::to_string(&config.view()).expect("view should serialize");
        assert!(!serialized.contains("secret"));
        assert!(!serialized.contains("bind_password"));
    }
}
