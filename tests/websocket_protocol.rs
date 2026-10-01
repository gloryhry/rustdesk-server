mod common;
use common::*;
use hbbs::{api::CookiePolicy,oauth::OAuthRuntime};
use hbb_common::{futures_util::{SinkExt,StreamExt},protobuf::Message,rendezvous_proto::{RendezvousMessage,RegisterPk,rendezvous_message,register_pk_response}};

#[tokio::test]
async fn real_hbbs_websocket_preserves_binary_protobuf_responses_after_dependency_upgrade() {
    let app = TestApp::new(CookiePolicy::default(),OAuthRuntime::new(Vec::new())).await;
    let path = app.database_path();
    let server = common::rendezvous::MockHbbs::start(path.parent().unwrap(),&path).await;
    let (mut socket,handshake) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{}/",server.port+2)).await.unwrap();
    assert_eq!(handshake.status().as_u16(),101);
    let mut message = RendezvousMessage::new();
    message.set_register_pk(RegisterPk { id:"456789".to_owned(),uuid:b"websocket-test-device".to_vec().into(),pk:vec![3;32].into(),..Default::default() });
    socket.send(tungstenite::Message::Binary(message.write_to_bytes().unwrap().into())).await.unwrap();
    let response = tokio::time::timeout(std::time::Duration::from_secs(3),socket.next()).await.unwrap().unwrap().unwrap();
    let bytes = match response { tungstenite::Message::Binary(bytes) => bytes,other => panic!("expected binary protobuf, got {other:?}") };
    let response = RendezvousMessage::parse_from_bytes(&bytes).unwrap();
    // RegisterPk over WS is deliberately unsupported by hbbs; upgrading must
    // preserve its explicit protocol reply, rather than dropping the connection.
    assert!(matches!(response.union,Some(rendezvous_message::Union::RegisterPkResponse(ref value)) if value.result.enum_value().unwrap()==register_pk_response::Result::NOT_SUPPORT));
    socket.close(None).await.unwrap();
}
