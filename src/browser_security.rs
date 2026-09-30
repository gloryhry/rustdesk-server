use std::net::IpAddr;

/// Browser cookie policy is configured at startup, never inferred from request headers.
#[derive(Debug, Clone, Copy)]
pub struct CookiePolicy {
    secure: bool,
}

impl Default for CookiePolicy {
    fn default() -> Self {
        Self { secure: true }
    }
}

impl CookiePolicy {
    pub fn local_http(bind: IpAddr, public_url: &str) -> Result<Self, &'static str> {
        let url = reqwest::Url::parse(public_url)
            .map_err(|_| "invalid API_PUBLIC_URL for local HTTP development")?;
        let local_host = url.host_str().is_some_and(|host| {
            host == "localhost"
                || host.trim_matches(['[', ']']).parse::<IpAddr>()
                    .is_ok_and(|ip| ip.is_loopback())
        });
        if !bind.is_loopback() || url.scheme() != "http" || !local_host
            || !url.username().is_empty() || url.password().is_some()
            || url.fragment().is_some()
        {
            return Err("insecure cookies require a loopback API_BIND and a local HTTP API_PUBLIC_URL");
        }
        Ok(Self { secure: false })
    }

    pub fn cookie(&self, name: &str, value: &str, max_age: u64) -> String {
        let secure = if self.secure { "; Secure" } else { "" };
        format!("{name}={value}; HttpOnly; SameSite=Lax; Path=/; Max-Age={max_age}{secure}")
    }
}
