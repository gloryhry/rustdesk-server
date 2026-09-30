use std::{net::{TcpListener,UdpSocket},path::Path,process::{Child,Command,Stdio},time::Duration};

pub struct MockHbbs { child: Child, pub port: u16 }

impl MockHbbs {
    pub async fn start(directory: &Path, database: &Path) -> Self {
        let (port,guards,udp) = loop {
            let main = TcpListener::bind("127.0.0.1:0").unwrap();
            let port = main.local_addr().unwrap().port();
            if port < 1024 || port > 65_530 { continue; }
            if let (Ok(nat),Ok(web),Ok(udp)) = (TcpListener::bind(("127.0.0.1",port-1)),TcpListener::bind(("127.0.0.1",port+2)),UdpSocket::bind(("127.0.0.1",port))) {
                break (port,vec![main,nat,web],udp);
            }
        };
        drop(guards); drop(udp);
        let child = Command::new(env!("CARGO_BIN_EXE_hbbs"))
            .args(["--bind","127.0.0.1","--port",&port.to_string(),"--key",""])
            .current_dir(directory).env_clear().env("DB_URL",database).env("TEST_HBBS","no")
            // The existing updater cannot reach an external server during this isolated test.
            .env("HTTP_PROXY","http://127.0.0.1:9").env("HTTPS_PROXY","http://127.0.0.1:9")
            .stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap();
        let mut server = Self { child,port };
        for _ in 0..100 {
            if std::net::TcpStream::connect(("127.0.0.1",port)).is_ok() { return server; }
            assert!(server.child.try_wait().unwrap().is_none(),"isolated hbbs exited before readiness");
            hbb_common::tokio::time::sleep(Duration::from_millis(50)).await;
        }
        panic!("isolated hbbs did not become ready");
    }

    pub fn exchange(&self, socket: &UdpSocket, message: hbb_common::rendezvous_proto::RendezvousMessage) -> hbb_common::rendezvous_proto::RendezvousMessage {
        use hbb_common::protobuf::Message;
        socket.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
        socket.send_to(&message.write_to_bytes().unwrap(),("127.0.0.1",self.port)).unwrap();
        let mut buffer = vec![0;65536];
        let (size,_) = socket.recv_from(&mut buffer).unwrap();
        hbb_common::rendezvous_proto::RendezvousMessage::parse_from_bytes(&buffer[..size]).unwrap()
    }
}

impl Drop for MockHbbs {
    fn drop(&mut self) { let _ = self.child.kill(); let _ = self.child.wait(); }
}
