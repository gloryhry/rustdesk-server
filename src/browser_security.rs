use std::net::IpAddr;

/// Browser cookie policy is configured at startup, never inferred from request headers.
#[derive(Debug, Clone, Copy)]
pub struct CookiePolicy {
    secure: bool,
    same_site: &'static str,
}

impl Default for CookiePolicy {
    fn default() -> Self {
        Self { secure: true, same_site: "Lax" }
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
        Ok(Self { secure: false, same_site: "Lax" })
    }

    pub fn cross_site(mut self) -> Result<Self, &'static str> {
        if !self.secure { return Err("cross-site cookies require Secure and cannot use local HTTP mode"); }
        self.same_site = "None";
        Ok(self)
    }

    pub fn cookie(&self, name: &str, value: &str, max_age: u64) -> String {
        let secure = if self.secure { "; Secure" } else { "" };
        format!("{name}={value}; HttpOnly; SameSite={}; Path=/; Max-Age={max_age}{secure}", self.same_site)
    }
}

/// Exact, startup-configured origins. Forwarded/Host headers never extend this set.
#[derive(Debug, Clone)]
pub struct BrowserPolicy {
    origins: std::collections::HashSet<String>,
}

impl BrowserPolicy {
    pub fn new(public_url: &str, allowed: &[String]) -> Result<Self, &'static str> {
        let public = reqwest::Url::parse(public_url).map_err(|_| "invalid API_PUBLIC_URL")?;
        if !matches!(public.scheme(), "http" | "https") || public.host_str().is_none()
            || !public.username().is_empty() || public.password().is_some() {
            return Err("API_PUBLIC_URL must be an HTTP(S) URL without credentials");
        }
        let mut origins = std::collections::HashSet::new();
        origins.insert(public.origin().ascii_serialization());
        for origin in allowed {
            let url = reqwest::Url::parse(origin).map_err(|_| "invalid API_ALLOWED_ORIGINS entry")?;
            if origin.contains('*') || !matches!(url.scheme(), "http" | "https") || url.host_str().is_none()
                || url.origin().ascii_serialization() != *origin {
                return Err("API_ALLOWED_ORIGINS requires exact origins without wildcards, paths or credentials");
            }
            origins.insert(origin.clone());
        }
        Ok(Self { origins })
    }

    pub fn allows(&self, origin: &str) -> bool { self.origins.contains(origin) }
}
