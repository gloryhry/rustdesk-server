mod common;

use common::{rendezvous::MockHbbs, TestApp};
use hbbs::{api::CookiePolicy, oauth::OAuthRuntime};
use hbb_common::{protobuf::Message, rendezvous_proto::{rendezvous_message, KeyExchange, PunchHoleRequest, RendezvousMessage, TestNatRequest}, tcp::FramedStream};
use sodiumoxide::crypto::{box_, secretbox, sign};

#[tokio::test]
async fn authenticated_native_client_can_secure_rendezvous_and_plain_clients_still_work() {
    let app = TestApp::new(CookiePolicy::default(), OAuthRuntime::new(Vec::new())).await;
    let path = app.database_path();
    let directory = path.parent().unwrap();
    let server = MockHbbs::start(directory, &path).await;
    let mut stream = FramedStream::new(format!("127.0.0.1:{}", server.port), None, 3000).await.unwrap();
    let offer = stream.next_timeout(1000).await.expect("server must offer signed key exchange before native client sends its token").unwrap();
    let offer = RendezvousMessage::parse_from_bytes(&offer).unwrap();
    let exchange = match offer.union { Some(rendezvous_message::Union::KeyExchange(value)) => value, other => panic!("expected key exchange: {other:?}") };
    assert_eq!(exchange.keys.len(), 1);
    let signing_pk = sign::PublicKey::from_slice(&base64::decode(std::fs::read_to_string(directory.join("id_ed25519.pub")).unwrap().trim()).unwrap()).unwrap();
    let server_pk = box_::PublicKey::from_slice(&sign::verify(&exchange.keys[0], &signing_pk).unwrap()).unwrap();
    let (pk, sk) = box_::gen_keypair();
    let key = secretbox::gen_key();
    let mut response = RendezvousMessage::new();
    response.set_key_exchange(KeyExchange { keys: vec![pk.0.to_vec().into(), box_::seal(&key.0, &box_::Nonce([0; box_::NONCEBYTES]), &server_pk, &sk).into()], ..Default::default() });
    stream.send(&response).await.unwrap();
    stream.set_key(key);
    let mut request = RendezvousMessage::new();
    request.set_punch_hole_request(PunchHoleRequest { id: "missing-peer".to_owned(), ..Default::default() });
    stream.send(&request).await.unwrap();
    let result = stream.next_timeout(3000).await.unwrap().unwrap();
    assert!(matches!(RendezvousMessage::parse_from_bytes(&result).unwrap().union, Some(rendezvous_message::Union::PunchHoleResponse(_))));

    // Clients without a token ignore the unsolicited exchange and use ordinary frames.
    let mut plain = FramedStream::new(format!("127.0.0.1:{}", server.port), None, 3000).await.unwrap();
    request.set_test_nat_request(TestNatRequest::default());
    plain.send(&request).await.unwrap();
    let greeting = plain.next_timeout(3000).await.unwrap().unwrap();
    assert!(matches!(RendezvousMessage::parse_from_bytes(&greeting).unwrap().union, Some(rendezvous_message::Union::KeyExchange(_))));
    let result = plain.next_timeout(3000).await.unwrap().unwrap();
    assert!(matches!(RendezvousMessage::parse_from_bytes(&result).unwrap().union, Some(rendezvous_message::Union::TestNatResponse(_))));
}

#[tokio::test]
async fn malformed_key_exchanges_close_only_the_offending_connection() {
    let app = TestApp::new(CookiePolicy::default(), OAuthRuntime::new(Vec::new())).await;
    let path = app.database_path();
    let server = MockHbbs::start(path.parent().unwrap(), &path).await;
    for keys in [vec![vec![1;32].into()], vec![vec![2;32].into(), vec![3;48].into()]] {
        let mut stream = FramedStream::new(format!("127.0.0.1:{}", server.port), None, 3000).await.unwrap();
        assert!(stream.next_timeout(3000).await.unwrap().is_ok());
        let mut response = RendezvousMessage::new();
        response.set_key_exchange(KeyExchange { keys, ..Default::default() });
        stream.send(&response).await.unwrap();
        assert!(stream.next_timeout(3000).await.is_none(), "invalid exchange must close rather than downgrade encryption");
    }
    let mut plain = FramedStream::new(format!("127.0.0.1:{}", server.port), None, 3000).await.unwrap();
    assert!(plain.next_timeout(3000).await.unwrap().is_ok());
    let mut request = RendezvousMessage::new();
    request.set_test_nat_request(TestNatRequest::default());
    plain.send(&request).await.unwrap();
    let result = plain.next_timeout(3000).await.unwrap().unwrap();
    assert!(matches!(RendezvousMessage::parse_from_bytes(&result).unwrap().union, Some(rendezvous_message::Union::TestNatResponse(_))));
}
