mod common;
use common::TestApp;
use hbbs::{api::CookiePolicy,oauth::OAuthRuntime};
use hbb_common::{futures_util::{SinkExt,StreamExt},protobuf::Message,rendezvous_proto::{RendezvousMessage,TestNatRequest,RequestRelay,rendezvous_message}};
use std::{net::TcpListener,path::Path,process::{Child,Command,Stdio},time::Duration};
use tungstenite::client::IntoClientRequest;

async fn nat_port(port: u16, headers: &[(&str,&str)]) -> i32 {
    let mut request = format!("ws://127.0.0.1:{}/",port+2).into_client_request().unwrap();
    for (key,value) in headers { request.headers_mut().append((*key).parse::<http::header::HeaderName>().unwrap(),value.parse().unwrap()); }
    let (mut socket,_) = tokio_tungstenite::connect_async(request).await.unwrap();
    let mut message = RendezvousMessage::new(); message.set_test_nat_request(TestNatRequest::default());
    socket.send(tungstenite::Message::Binary(message.write_to_bytes().unwrap().into())).await.unwrap();
    let response = tokio::time::timeout(Duration::from_secs(3),socket.next()).await.unwrap().unwrap().unwrap();
    let bytes = match response { tungstenite::Message::Binary(bytes) => bytes,other => panic!("expected binary protobuf: {other:?}") };
    let message = RendezvousMessage::parse_from_bytes(&bytes).unwrap();
    socket.close(None).await.unwrap();
    match message.union { Some(rendezvous_message::Union::TestNatResponse(value)) => value.port,other => panic!("unexpected NAT response: {other:?}") }
}

struct MockHbbr { child: Child, port: u16, log: std::path::PathBuf }
impl MockHbbr {
    async fn start(directory: &Path, proxy: Option<&str>) -> Self {
        let port = loop {
            let main = TcpListener::bind("127.0.0.1:0").unwrap(); let port = main.local_addr().unwrap().port();
            if port<1024 || port>65530 { continue; }
            if let Ok(web) = TcpListener::bind(("127.0.0.1",port+2)) { drop(web); drop(main); break port; }
        };
        let log = directory.join("relay.log");
        let output = std::fs::File::create(&log).unwrap();
        let mut command = Command::new(env!("CARGO_BIN_EXE_hbbr"));
        command.args(["--bind","127.0.0.1","--port",&port.to_string(),"--key",""])
            .current_dir(directory).env_clear().env("HOME",directory.join("home"))
            .env("XDG_CONFIG_HOME",directory.join("client-config"))
            .stdout(Stdio::from(output.try_clone().unwrap())).stderr(Stdio::from(output));
        if let Some(proxy) = proxy { command.env("WS_TRUSTED_PROXIES",proxy); }
        let mut server = Self { child:command.spawn().unwrap(),port,log };
        for _ in 0..100 {
            if std::net::TcpStream::connect(("127.0.0.1",port)).is_ok() { return server; }
            assert!(server.child.try_wait().unwrap().is_none(),"hbbr exited before readiness");
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        panic!("hbbr readiness timed out");
    }
    async fn source(&self, headers: &[(&str,&str)]) -> String {
        let mut request = format!("ws://127.0.0.1:{}/",self.port+2).into_client_request().unwrap();
        for (key,value) in headers { request.headers_mut().append((*key).parse::<http::header::HeaderName>().unwrap(),value.parse().unwrap()); }
        let (mut socket,_) = tokio_tungstenite::connect_async(request).await.unwrap();
        let uuid = uuid::Uuid::new_v4().to_string();
        let mut message = RendezvousMessage::new(); message.set_request_relay(RequestRelay { uuid:uuid.clone(),..Default::default() });
        socket.send(tungstenite::Message::Binary(message.write_to_bytes().unwrap().into())).await.unwrap();
        let marker = format!("New relay request {uuid} from ");
        for _ in 0..100 {
            let text = std::fs::read_to_string(&self.log).unwrap();
            if let Some(line) = text.lines().find(|line|line.contains(&marker)) {
                return line.split_once(&marker).unwrap().1.to_owned();
            }
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
        panic!("relay request log timed out");
    }
}
impl Drop for MockHbbr { fn drop(&mut self) { let _ = self.child.kill(); let _ = self.child.wait(); } }

#[tokio::test]
async fn direct_hbbs_websocket_cannot_replace_real_source_using_forwarded_headers() {
    let app = TestApp::new(CookiePolicy::default(),OAuthRuntime::new(Vec::new())).await;
    let path = app.database_path(); let server = common::rendezvous::MockHbbs::start(path.parent().unwrap(),&path).await;
    for headers in [vec![("x-real-ip","203.0.113.99")],vec![("x-forwarded-for","2001:db8::99")]] {
        assert_ne!(nat_port(server.port,&headers).await,0,"direct connection must retain its real source port");
    }
}

#[tokio::test]
async fn direct_hbbr_websocket_logs_real_peer_instead_of_spoofed_forwarded_ip() {
    let app = TestApp::new(CookiePolicy::default(),OAuthRuntime::new(Vec::new())).await;
    let path = app.database_path(); let server = MockHbbr::start(path.parent().unwrap(),None).await;
    for headers in [vec![("x-real-ip","203.0.113.99")],vec![("x-forwarded-for","2001:db8::99")]] {
        let source = server.source(&headers).await;
        assert!(source.starts_with("127.0.0.1:"),"untrusted source: {source}");
    }
}

#[tokio::test]
async fn trusted_hbbs_proxy_accepts_one_valid_ip_and_rejects_ambiguous_headers() {
    let app = TestApp::new(CookiePolicy::default(),OAuthRuntime::new(Vec::new())).await;
    let path = app.database_path();
    for config in ["127.0.0.1","127.0.0.2"] {
        let server = common::rendezvous::MockHbbs::start_with_proxy(path.parent().unwrap(),&path,Some(config)).await;
        for headers in [vec![("x-real-ip","203.0.113.9")],vec![("x-forwarded-for","2001:db8::9")],vec![("x-real-ip","203.0.113.9"),("x-forwarded-for","203.0.113.9")]] {
            assert_eq!(nat_port(server.port,&headers).await==0,config=="127.0.0.1");
        }
        for headers in [vec![],vec![("x-real-ip","invalid")],vec![("x-forwarded-for","203.0.113.9, 198.51.100.1")],vec![("x-real-ip","203.0.113.9"),("x-real-ip","203.0.113.9")],vec![("x-real-ip","203.0.113.9"),("x-forwarded-for","198.51.100.1")]] {
            assert_ne!(nat_port(server.port,&headers).await,0);
        }
    }
}

#[tokio::test]
async fn trusted_hbbr_proxy_logs_valid_ip_and_keeps_tcp_identity_for_bad_headers() {
    let app = TestApp::new(CookiePolicy::default(),OAuthRuntime::new(Vec::new())).await;
    let path = app.database_path();
    for config in ["127.0.0.1","127.0.0.2"] {
        let server = MockHbbr::start(path.parent().unwrap(),Some(config)).await;
        for (headers,expected) in [(vec![("x-real-ip","203.0.113.9")],"203.0.113.9:0"),(vec![("x-forwarded-for","2001:db8::9")],"[2001:db8::9]:0"),(vec![("x-real-ip","203.0.113.9"),("x-forwarded-for","203.0.113.9")],"203.0.113.9:0")] {
            let source = server.source(&headers).await;
            if config=="127.0.0.1" { assert_eq!(source,expected); } else { assert!(source.starts_with("127.0.0.1:")); }
        }
        for headers in [vec![],vec![("x-real-ip","invalid")],vec![("x-forwarded-for","203.0.113.9, 198.51.100.1")],vec![("x-real-ip","203.0.113.9"),("x-real-ip","203.0.113.9")],vec![("x-real-ip","203.0.113.9"),("x-forwarded-for","198.51.100.1")]] {
            assert!(server.source(&headers).await.starts_with("127.0.0.1:"));
        }
    }
}

#[test]
fn invalid_proxy_configuration_prevents_both_servers_from_starting() {
    let directory = std::env::temp_dir().join(format!("rustdesk-invalid-proxy-{}",uuid::Uuid::new_v4()));
    std::fs::create_dir(&directory).unwrap();
    for binary in [env!("CARGO_BIN_EXE_hbbs"),env!("CARGO_BIN_EXE_hbbr")] {
        let output = Command::new(binary).args(["--bind","127.0.0.1","--key",""])
            .current_dir(&directory).env_clear().env("HOME",directory.join("home"))
            .env("XDG_CONFIG_HOME",directory.join("client-config"))
            .env("HTTP_PROXY","http://127.0.0.1:9").env("HTTPS_PROXY","http://127.0.0.1:9")
            .env("WS_TRUSTED_PROXIES","*").output().unwrap();
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("WS_TRUSTED_PROXIES"));
    }
    assert!(!directory.join("id_ed25519").exists());
    std::fs::remove_dir_all(directory).unwrap();
}
