#![cfg(feature = "realtime-ws")]
use ht_core::{
    Db, Transport, Worker,
    http::{Response, TransportError},
    realtime::ws::Subscriber,
};
use serde_json::{Value, json};
use std::{net::TcpListener, sync::mpsc, time::Duration};
use tungstenite::Message;

#[test]
fn websocket_subscribes_authenticates_pongs_and_dispatches_to_sqlite() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let (tcp, _) = listener.accept().unwrap();
        tcp.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let mut socket = tungstenite::accept(tcp).unwrap();
        socket.send(Message::Text(json!({"event":"pusher:connection_established","data":json!({"socket_id":"123.456","activity_timeout":120}).to_string()}).to_string().into())).unwrap();
        let subscribe: Value =
            serde_json::from_str(&socket.read().unwrap().into_text().unwrap()).unwrap();
        assert_eq!(subscribe["event"], "pusher:subscribe");
        assert_eq!(subscribe["data"]["channel"], "private-task-1");
        assert_eq!(subscribe["data"]["auth"], "public-key:signature");
        socket
            .send(Message::Text(
                json!({"event":"pusher:ping","data":{}}).to_string().into(),
            ))
            .unwrap();
        let pong: Value =
            serde_json::from_str(&socket.read().unwrap().into_text().unwrap()).unwrap();
        assert_eq!(pong["event"], "pusher:pong");
        socket.send(Message::Text(json!({"event":"task:changed","channel":"private-task-1","data":"{\"originUserId\":6}"}).to_string().into())).unwrap();
        let _ = socket.close(None);
    });
    struct Server {
        send: mpsc::Sender<()>,
    }
    impl Transport for Server {
        fn get(&mut self, path: &str) -> std::result::Result<Response, TransportError> {
            assert_eq!(path, "/tasks?task_id=1");
            self.send.send(()).unwrap();
            Ok(Response {
                status: 200,
                body: json!({"success":true,"tasks":[{"id":1,"title":"websocket update","projectId":15,"status":"Normal"}]}),
            })
        }
        fn post(
            &mut self,
            _: &str,
            _: &Value,
            _: &str,
        ) -> std::result::Result<Response, TransportError> {
            unreachable!()
        }
    }
    let file = tempfile::NamedTempFile::new().unwrap();
    let (send, receive) = mpsc::channel();
    let worker = Worker::start(file.path(), Server { send }).unwrap();
    let subscriber = Subscriber::spawn(
        format!("ws://{address}/app/public-key?protocol=7"),
        vec!["private-task-1".into()],
        worker.handle(),
        |socket, channel| {
            assert_eq!(socket, "123.456");
            assert_eq!(channel, "private-task-1");
            Ok(json!({"auth":"public-key:signature"}))
        },
    )
    .unwrap();
    receive.recv_timeout(Duration::from_secs(5)).unwrap();
    drop(subscriber);
    drop(worker);
    server.join().unwrap();
    assert_eq!(
        Db::open(file.path())
            .unwrap()
            .task(1)
            .unwrap()
            .unwrap()
            .title,
        "websocket update"
    );
}
