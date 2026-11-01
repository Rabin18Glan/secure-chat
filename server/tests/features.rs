//! End-to-end feature regressions against an isolated MongoDB database.
//! Run: cargo test --test features -- --ignored --nocapture
//! Set TEST_MONGO_URL if MongoDB is not on localhost:27017.
use futures_util::{FutureExt, SinkExt, StreamExt};
use reqwest::{header, Client, Method, StatusCode};
use serde_json::{json, Value};
use std::{
    collections::VecDeque,
    process::{Child, Command, Stdio},
    time::Duration,
};
use tokio::{
    net::TcpStream,
    time::{sleep, timeout},
};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{client::IntoClientRequest, Message},
    MaybeTlsStream, WebSocketStream,
};

struct Backend {
    process: Child,
    url: String,
    database: mongodb::Database,
}

impl Drop for Backend {
    fn drop(&mut self) {
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}

impl Backend {
    async fn start() -> Self {
        let mongo_url =
            std::env::var("TEST_MONGO_URL").unwrap_or_else(|_| "mongodb://127.0.0.1:27017".into());
        let mongo = mongodb::Client::with_uri_str(&mongo_url)
            .await
            .expect("MongoDB client");
        let database = mongo.database(&format!(
            "secure_chat_test_{}",
            uuid::Uuid::new_v4().simple()
        ));
        // The listener is released immediately before the backend takes the port.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let process = Command::new(env!("CARGO_BIN_EXE_secure-chat-server"))
            .env("PORT", port.to_string())
            .env("MONGO_URL", &mongo_url)
            .env("DB_NAME", database.name())
            .env("COOKIE_SECURE", "false")
            .env("GOOGLE_CLIENT_ID", "")
            .env("GOOGLE_CLIENT_SECRET", "")
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("start backend");
        let mut backend = Self {
            process,
            url: format!("http://127.0.0.1:{port}"),
            database,
        };
        let client = Client::new();
        for _ in 0..100 {
            assert!(
                backend.process.try_wait().unwrap().is_none(),
                "backend exited during startup"
            );
            if client
                .get(format!("{}/ping", backend.url))
                .send()
                .await
                .is_ok()
            {
                return backend;
            }
            sleep(Duration::from_millis(100)).await;
        }
        panic!("backend did not become ready; ensure MongoDB is available");
    }
}

struct User {
    http: Client,
    cookie: String,
    id: String,
    base: String,
}

impl User {
    fn anonymous(backend: &Backend) -> Self {
        Self {
            http: Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap(),
            cookie: String::new(),
            id: String::new(),
            base: backend.url.clone(),
        }
    }
    async fn request(&mut self, method: Method, path: &str, data: Value) -> (StatusCode, Value) {
        let response = self
            .http
            .request(method, format!("{}{path}", self.base))
            .header(header::COOKIE, &self.cookie)
            .json(&data)
            .send()
            .await
            .expect("HTTP request");
        let status = response.status();
        if let Some(cookie) = response.headers().get(header::SET_COOKIE) {
            self.cookie = cookie
                .to_str()
                .unwrap()
                .split(';')
                .next()
                .unwrap()
                .to_owned();
        }
        let bytes = response.bytes().await.unwrap();
        let body = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).expect("JSON response")
        };
        (status, body)
    }
    async fn register(backend: &Backend, name: &str) -> Self {
        let mut user = Self::anonymous(backend);
        let (status, body) = user.request(Method::POST, "/api/auth/register", json!({"username":name,"email":format!("{name}@example.com"),"password":"test-password-123"})).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["status"], true, "{body}");
        user.id = body["user"]["_id"].as_str().unwrap().into();
        assert!(user.cookie.starts_with("secure_chat_session="));
        user
    }
    async fn meeting(&mut self) -> String {
        let (status, meeting) = self
            .request(
                Method::POST,
                "/api/sessions/create",
                json!({"ownerId":self.id,"name":"Feature test"}),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{meeting}");
        meeting["sessionId"]
            .as_str()
            .expect("browser sessionId contract")
            .into()
    }
}

struct Socket {
    ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
    pending: VecDeque<Value>,
}

impl Socket {
    async fn connect(user: &User) -> Self {
        let url = format!(
            "{}/ws?userId={}",
            user.base.replace("http://", "ws://"),
            user.id
        );
        let mut request = url.into_client_request().unwrap();
        request
            .headers_mut()
            .insert(header::COOKIE, user.cookie.parse().unwrap());
        let (ws, _) = connect_async(request)
            .await
            .expect("authenticated WebSocket upgrade");
        Self {
            ws,
            pending: VecDeque::new(),
        }
    }
    async fn send(&mut self, value: Value) {
        self.ws
            .send(Message::Text(value.to_string().into()))
            .await
            .unwrap();
    }
    async fn event(&mut self, event: &str) -> Value {
        if let Some(index) = self.pending.iter().position(|v| v["event"] == event) {
            return self.pending.remove(index).unwrap();
        }
        timeout(Duration::from_secs(5), async {
            loop {
                let message = self
                    .ws
                    .next()
                    .await
                    .expect("socket remains open")
                    .expect("WebSocket message");
                if let Message::Text(text) = message {
                    let value: Value = serde_json::from_str(&text).unwrap();
                    if value["event"] == event {
                        return value;
                    }
                    self.pending.push_back(value);
                }
            }
        })
        .await
        .unwrap_or_else(|_| panic!("timed out waiting for {event}; pending={:?}", self.pending))
    }
    async fn close(&mut self) {
        let _ = self.ws.close(None).await;
    }
}

#[tokio::test]
#[ignore = "requires local MongoDB; creates and drops a unique test database"]
async fn authentication_messages_meetings_and_permissions() {
    let backend = Backend::start().await;
    // Catch test panics so the isolated database is dropped even if an assertion fails.
    let result = std::panic::AssertUnwindSafe(exercise_features(&backend))
        .catch_unwind()
        .await;
    backend
        .database
        .drop()
        .await
        .expect("drop isolated test database");
    if let Err(panic) = result {
        std::panic::resume_unwind(panic);
    }
}

async fn exercise_features(backend: &Backend) {
    let mut anonymous = User::anonymous(backend);
    let mut alice = User::register(backend, "alice").await;
    let mut bob = User::register(backend, "bob").await;
    let mut carol = User::register(backend, "carol").await;
    assert_eq!(
        anonymous
            .request(Method::GET, "/api/auth/me", Value::Null)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        alice
            .request(Method::GET, "/api/auth/me", Value::Null)
            .await
            .1["username"],
        "alice"
    );
    assert_eq!(
        alice
            .request(
                Method::GET,
                &format!("/api/auth/allusers/{}", alice.id),
                Value::Null
            )
            .await
            .1
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        alice
            .request(
                Method::GET,
                &format!("/api/auth/allusers/{}", bob.id),
                Value::Null
            )
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    let (status, avatar) = alice
        .request(
            Method::POST,
            &format!("/api/auth/setavatar/{}", alice.id),
            json!({"image":"avatar-data"}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(avatar["isSet"], true);
    assert_eq!(
        alice
            .request(
                Method::POST,
                &format!("/api/auth/setavatar/{}", bob.id),
                json!({"image":"bad"})
            )
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        anonymous
            .request(
                Method::POST,
                "/api/auth/login",
                json!({"username":"alice","password":"wrong"})
            )
            .await
            .1["status"],
        false
    );
    assert_eq!(
        anonymous
            .request(
                Method::POST,
                "/api/auth/login",
                json!({"username":"alice","password":"test-password-123"})
            )
            .await
            .1["status"],
        true
    );
    let revoked_cookie = anonymous.cookie.clone();
    assert_eq!(
        anonymous
            .request(
                Method::GET,
                &format!("/api/auth/logout/{}", alice.id),
                Value::Null
            )
            .await
            .0,
        StatusCode::OK
    );
    anonymous.cookie = revoked_cookie;
    assert_eq!(
        anonymous
            .request(Method::GET, "/api/auth/me", Value::Null)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        anonymous
            .request(
                Method::POST,
                "/api/auth/register",
                json!({"username":"x","email":"bad","password":"a"})
            )
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        anonymous
            .request(
                Method::POST,
                "/api/auth/register",
                json!({"username":"alice","email":"unused@example.com","password":"password123"})
            )
            .await
            .1["status"],
        false
    );
    let origin_response = Client::new()
        .get(format!("{}/ping", backend.url))
        .header(header::ORIGIN, "https://untrusted.example")
        .send()
        .await
        .unwrap();
    assert_eq!(origin_response.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        anonymous
            .request(Method::POST, "/api/auth/google", json!({}))
            .await
            .1["status"],
        false
    );
    assert_eq!(
        anonymous
            .request(
                Method::POST,
                "/api/auth/google",
                json!({"code":"fake","state":"fake"})
            )
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    println!("Authentication, validation, profiles, logout, origin and OAuth guards passed");

    let message = json!({"from":alice.id,"to":bob.id,"message":"Hello Bob"});
    assert_eq!(
        alice
            .request(Method::POST, "/api/messages/addmsg", message.clone())
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        carol
            .request(Method::POST, "/api/messages/getmsg", message.clone())
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        alice
            .request(
                Method::POST,
                "/api/messages/addmsg",
                json!({"from":alice.id,"to":"invalid","message":"hello"})
            )
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        alice
            .request(
                Method::POST,
                "/api/messages/addmsg",
                json!({"from":alice.id,"to":bob.id,"message":" "})
            )
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    let (status, history) = bob
        .request(
            Method::POST,
            "/api/messages/getmsg",
            json!({"from":bob.id,"to":alice.id}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(history[0], json!({"fromSelf":false,"message":"Hello Bob"}));
    let mut direct_a = Socket::connect(&alice).await;
    let mut direct_b = Socket::connect(&bob).await;
    direct_b
        .send(json!({"event":"add-user","userId":bob.id}))
        .await;
    sleep(Duration::from_millis(50)).await;
    direct_a
        .send(json!({"event":"send-msg","to":bob.id,"msg":"Live hello"}))
        .await;
    let live = direct_b.event("msg-recieve").await;
    assert_eq!(live["msg"], "Live hello");
    assert_eq!(live["from"], alice.id);
    direct_a
        .send(json!({"event":"add-user","userId":bob.id}))
        .await;
    assert_eq!(
        direct_a.event("session-error").await["msg"],
        "Identity mismatch"
    );
    println!("Direct message persistence, history, delivery and identity checks passed");

    assert_eq!(
        bob.request(
            Method::POST,
            "/api/sessions/create",
            json!({"ownerId":alice.id})
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let session = alice.meeting().await;
    assert_eq!(
        alice
            .request(
                Method::GET,
                &format!("/api/sessions/{session}"),
                Value::Null
            )
            .await
            .1["name"],
        "Feature test"
    );
    let mut a = Socket::connect(&alice).await;
    let mut b = Socket::connect(&bob).await;
    let mut c = Socket::connect(&carol).await;
    a.send(
        json!({"event":"session-knock","sessionId":session,"userId":alice.id,"username":"alice"}),
    )
    .await;
    assert_eq!(a.event("session-admitted").await["role"], "owner");
    b.send(json!({"event":"session-knock","sessionId":session,"userId":bob.id,"username":"bob"}))
        .await;
    b.event("session-waiting").await;
    assert_eq!(
        a.event("session-knock-request").await["user"]["userId"],
        bob.id
    );
    b.send(json!({"event":"webrtc-offer","sessionId":session,"to":alice.id,"sdp":{"type":"offer","sdp":"bad"}})).await;
    a.send(json!({"event":"session-approve-user","sessionId":session,"targetUserId":bob.id}))
        .await;
    assert_eq!(b.event("session-admitted").await["role"], "member");
    a.event("session-user-joined").await;
    assert!(!a.pending.iter().any(|v| v["event"] == "webrtc-offer"));
    b.send(json!({"event":"webrtc-offer","sessionId":session,"to":alice.id,"sdp":{"type":"offer","sdp":"test-offer"}})).await;
    assert_eq!(a.event("webrtc-offer").await["from"], bob.id);
    a.send(json!({"event":"webrtc-answer","sessionId":session,"to":bob.id,"sdp":{"type":"answer","sdp":"test-answer"}})).await;
    assert_eq!(b.event("webrtc-answer").await["sdp"]["sdp"], "test-answer");
    b.send(json!({"event":"webrtc-ice-candidate","sessionId":session,"to":alice.id,"candidate":{"candidate":"test"}})).await;
    a.event("webrtc-ice-candidate").await;
    b.send(json!({"event":"webrtc-media-toggle","sessionId":session,"data":{"audio":false,"video":false,"screen":true}})).await;
    assert_eq!(a.event("webrtc-media-toggle").await["data"]["screen"], true);
    b.send(json!({"event":"session-chat-message","sessionId":session,"ciphertext":"opaque-encrypted-text","iv":"opaque-iv"})).await;
    assert_eq!(
        a.event("session-chat-message").await["ciphertext"],
        "opaque-encrypted-text"
    );
    b.event("session-chat-message").await;
    a.send(json!({"event":"session-promote-admin","sessionId":session,"targetUserId":bob.id}))
        .await;
    assert!(b.event("session-roles-updated").await["admins"]
        .as_array()
        .unwrap()
        .contains(&json!(bob.id)));
    a.send(json!({"event":"session-demote-admin","sessionId":session,"targetUserId":bob.id}))
        .await;
    assert!(!b.event("session-roles-updated").await["admins"]
        .as_array()
        .unwrap()
        .contains(&json!(bob.id)));
    c.send(
        json!({"event":"session-knock","sessionId":session,"userId":carol.id,"username":"carol"}),
    )
    .await;
    c.event("session-waiting").await;
    a.send(json!({"event":"session-reject-user","sessionId":session,"targetUserId":carol.id}))
        .await;
    c.event("session-denied").await;
    a.send(json!({"event":"session-remove-user","sessionId":session,"targetUserId":bob.id}))
        .await;
    b.event("session-kicked").await;
    a.event("session-user-left").await;
    a.send(json!({"event":"session-terminate","sessionId":session}))
        .await;
    a.event("session-terminated").await;
    let mut deleted = false;
    for _ in 0..40 {
        if alice
            .request(
                Method::GET,
                &format!("/api/sessions/{session}"),
                Value::Null,
            )
            .await
            .0
            == StatusCode::NOT_FOUND
        {
            deleted = true;
            break;
        }
        sleep(Duration::from_millis(50)).await;
    }
    assert!(deleted, "ended meeting must be deleted");
    a.close().await;
    b.close().await;
    c.close().await;
    // Closing a meeting socket must preserve the user's separate direct-message connection.
    direct_a
        .send(json!({"event":"send-msg","to":bob.id,"msg":"Still connected"}))
        .await;
    assert_eq!(
        direct_b.event("msg-recieve").await["msg"],
        "Still connected"
    );
    println!("Waiting room, signalling, media state, encrypted relay, roles, removal and termination passed");

    let unstarted = alice.meeting().await;
    let mut guest = Socket::connect(&bob).await;
    guest
        .send(
            json!({"event":"session-knock","sessionId":unstarted,"userId":bob.id,"username":"bob"}),
        )
        .await;
    guest.event("session-waiting").await;
    guest.close().await;
    sleep(Duration::from_millis(100)).await;
    assert_eq!(
        alice
            .request(
                Method::GET,
                &format!("/api/sessions/{unstarted}"),
                Value::Null
            )
            .await
            .0,
        StatusCode::OK
    );
    let mut host = Socket::connect(&alice).await;
    host.send(
        json!({"event":"session-knock","sessionId":unstarted,"userId":alice.id,"username":"alice"}),
    )
    .await;
    assert_eq!(
        host.event("session-admitted").await["waitingRoom"],
        json!([])
    );
    host.send(json!({"event":"session-leave","sessionId":unstarted}))
        .await;
    host.close().await;
    direct_a.close().await;
    direct_b.close().await;
    println!("Disconnect cleanup and unstarted meeting regressions passed");
}
