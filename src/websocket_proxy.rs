use http::HeaderMap;
use once_cell::sync::Lazy;
use std::{collections::HashSet, net::{IpAddr, SocketAddr}};

#[derive(Debug)]
pub struct TrustedProxyPolicy { addresses: HashSet<IpAddr> }

fn normalized(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(ip) => ip.to_ipv4_mapped().map(IpAddr::V4).unwrap_or(IpAddr::V6(ip)),
        ip => ip,
    }
}

impl TrustedProxyPolicy {
    pub fn parse(value: &str) -> Result<Self,String> {
        let mut addresses = HashSet::new();
        if !value.trim().is_empty() {
            for (index, address) in value.split(',').enumerate() {
                if index >= 128 { return Err("WS_TRUSTED_PROXIES exceeds 128 addresses".to_owned()); }
                let ip = address.trim().parse::<IpAddr>().map_err(|_|"WS_TRUSTED_PROXIES requires exact comma-separated IP addresses".to_owned())?;
                if ip.is_unspecified() || ip.is_multicast() { return Err("WS_TRUSTED_PROXIES requires unicast proxy addresses".to_owned()); }
                addresses.insert(normalized(ip));
            }
        }
        Ok(Self { addresses })
    }

    pub fn client_addr(&self, peer: SocketAddr, headers: &HeaderMap) -> SocketAddr {
        if !self.addresses.contains(&normalized(peer.ip())) { return peer; }
        let single = |name: &str| -> Result<Option<IpAddr>,()> {
            let mut values = headers.get_all(name).iter();
            let value = match values.next() { Some(value) => value, None => return Ok(None) };
            if values.next().is_some() { return Err(()); }
            let ip = value.to_str().map_err(|_|())?.parse::<IpAddr>().map_err(|_|())?;
            if ip.is_unspecified() || ip.is_multicast() { return Err(()); }
            Ok(Some(normalized(ip)))
        };
        let (real,forwarded) = match (single("x-real-ip"),single("x-forwarded-for")) {
            (Ok(real),Ok(forwarded)) => (real,forwarded), _ => return peer,
        };
        match (real,forwarded) {
            (Some(real),Some(forwarded)) if real != forwarded => peer,
            (Some(ip),_) | (_,Some(ip)) => SocketAddr::new(ip,0),
            _ => peer,
        }
    }
}

static POLICY: Lazy<Result<TrustedProxyPolicy,String>> = Lazy::new(|| {
    match std::env::var("WS_TRUSTED_PROXIES") {
        Ok(value) => TrustedProxyPolicy::parse(&value),
        Err(std::env::VarError::NotPresent) => TrustedProxyPolicy::parse(""),
        Err(_) => Err("WS_TRUSTED_PROXIES must be UTF-8".to_owned()),
    }
});

pub fn policy() -> hbb_common::ResultType<&'static TrustedProxyPolicy> {
    POLICY.as_ref().map_err(|message|hbb_common::anyhow::anyhow!("{message}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn headers(values: &[(&str,&str)]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for (name,value) in values { headers.append(name.parse::<http::header::HeaderName>().unwrap(),value.parse().unwrap()); }
        headers
    }
    #[test]
    fn only_the_actual_trusted_tcp_peer_can_supply_an_ip() {
        let peer = "127.0.0.1:4567".parse().unwrap();
        let values = headers(&[("x-real-ip","203.0.113.9")]);
        for config in ["","127.0.0.2","::1"] { assert_eq!(TrustedProxyPolicy::parse(config).unwrap().client_addr(peer,&values),peer); }
        let policy = TrustedProxyPolicy::parse("127.0.0.1,::1").unwrap();
        assert_eq!(policy.client_addr(peer,&values),"203.0.113.9:0".parse().unwrap());
        assert_eq!(policy.client_addr("[::1]:4567".parse().unwrap(),&headers(&[("x-forwarded-for","2001:db8::9")])),"[2001:db8::9]:0".parse().unwrap());
        assert_eq!(policy.client_addr("[::ffff:127.0.0.1]:4567".parse().unwrap(),&values),"203.0.113.9:0".parse().unwrap());
    }
    #[test]
    fn invalid_duplicate_and_conflicting_headers_keep_the_real_peer() {
        let policy = TrustedProxyPolicy::parse("127.0.0.1").unwrap(); let peer = "127.0.0.1:4567".parse().unwrap();
        for values in [vec![],vec![("x-real-ip","not-an-ip")],vec![("x-forwarded-for","203.0.113.9, 198.51.100.1")],vec![("x-real-ip","203.0.113.9:123")],vec![("x-real-ip","203.0.113.9"),("x-real-ip","203.0.113.9")],vec![("x-real-ip","203.0.113.9"),("x-forwarded-for","198.51.100.1")],vec![("x-real-ip","203.0.113.9"),("x-forwarded-for","invalid")],vec![("x-real-ip","0.0.0.0")]] {
            assert_eq!(policy.client_addr(peer,&headers(&values)),peer);
        }
        assert_eq!(policy.client_addr(peer,&headers(&[("x-real-ip","203.0.113.9"),("x-forwarded-for","203.0.113.9")])),"203.0.113.9:0".parse().unwrap());
        let mut invalid = HeaderMap::new(); invalid.insert("x-real-ip",http::HeaderValue::from_bytes(&[0xff]).unwrap());
        assert_eq!(policy.client_addr(peer,&invalid),peer);
    }
    #[test]
    fn invalid_proxy_configuration_fails_and_the_list_is_bounded() {
        for config in ["*","127.0.0.0/8","127.0.0.1,,::1","localhost","0.0.0.0","::","224.0.0.1"] { assert!(TrustedProxyPolicy::parse(config).is_err()); }
        assert!(TrustedProxyPolicy::parse(&vec!["127.0.0.1";129].join(",")).is_err());
        assert!(TrustedProxyPolicy::parse(" 127.0.0.1, ::1 ").is_ok());
    }
}
