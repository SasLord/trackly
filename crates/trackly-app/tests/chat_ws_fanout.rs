//! Chat delivery over a REAL WebSocket (Phase 41.5 Plan 17, D-23, MSG-03):
//! the author, participants and staff receive `chat_message`; an unrelated
//! employee does not. The router is served over TCP, clients carry a session
//! cookie. Names are fictional.
//!
//! Negative checks avoid sleeps: events are totally ordered in the broadcast
//! channel, so the stranger's FIRST frame after a control event must be the
//! control event itself, not the event under test.

use std::net::SocketAddr;
use std::time::Duration;

use futures_util::StreamExt;
use serde_json::Value;
use time::{Duration as TDuration, OffsetDateTime};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message as TMessage;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tower_sessions::session::{Id, Record};
use tower_sessions::SessionStore;

use trackly_app::context::AppCtx;
use trackly_app::http::auth::SessionIdentity;
use trackly_app::http::build_router;
use trackly_app::server::rusqlite_session_store::RusqliteSessionStore;
use trackly_core::auth::Role;

mod chat_support;
use chat_support::{add_participant, ident, seed_request, seed_user};

type Ws = WebSocketStream<MaybeTlsStream<TcpStream>>;

const SECRET: &str = "секретный текст сообщения";

async fn cookie(ctx: &AppCtx, user_id: i64, role: Role) -> String {
    let store = RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone());
    let id = Id::default();
    let si = SessionIdentity {
        user_id: Some(user_id),
        role: role.as_str().to_string(),
    };
    let mut record = Record {
        id,
        data: Default::default(),
        expiry_date: OffsetDateTime::now_utc() + TDuration::days(1),
    };
    record
        .data
        .insert("identity".to_string(), serde_json::to_value(&si).unwrap());
    store.create(&mut record).await.expect("session");
    format!("id={id}")
}

async fn serve(ctx: &AppCtx) -> SocketAddr {
    let app = build_router(
        ctx,
        RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone()),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let _ = axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await;
    });
    addr
}

async fn connect(addr: SocketAddr, cookie: &str) -> Ws {
    let mut req = format!("ws://{addr}/api/v1/ws")
        .into_client_request()
        .unwrap();
    req.headers_mut().insert("cookie", cookie.parse().unwrap());
    let (ws, _) = tokio_tungstenite::connect_async(req)
        .await
        .expect("ws connect");
    ws
}

/// Next text frame as JSON, within a generous bound (only used where a frame is expected).
async fn frame(ws: &mut Ws, who: &str) -> (String, Value) {
    loop {
        let m = tokio::time::timeout(Duration::from_secs(5), ws.next())
            .await
            .unwrap_or_else(|_| panic!("{who}: no frame within 5s"))
            .unwrap_or_else(|| panic!("{who}: stream closed"))
            .expect("ws frame");
        if let TMessage::Text(t) = m {
            let raw = t.to_string();
            let v: Value = serde_json::from_str(&raw).expect("json frame");
            return (raw, v);
        }
    }
}

struct World {
    ctx: AppCtx,
    _dir: tempfile::TempDir,
    addr: SocketAddr,
    author_id: i64,
    other_author_id: i64,
    request_a: i64,
    request_b: i64,
    cookies: Cookies,
}

struct Cookies {
    author: String,
    participant: String,
    manager: String,
    admin: String,
    stranger: String,
}

async fn world() -> World {
    let dir = tempfile::TempDir::new().unwrap();
    let paths = trackly_infra::Paths::resolve_for_exe_dir(dir.path().to_path_buf()).unwrap();
    let config = trackly_infra::AppConfig::default();
    let log_guard = trackly_app::logging::init(&paths, &config).or_else(|_| {
        let (_nb, guard) = tracing_appender::non_blocking(std::io::sink());
        Ok::<_, anyhow::Error>(guard)
    });
    let ctx = AppCtx::build(paths, config, log_guard.unwrap())
        .await
        .unwrap();
    let author = seed_user(&ctx, "wf_author", "Иванов И.И.", "employee").await;
    let participant = seed_user(&ctx, "wf_part", "Сидорова А.А.", "employee").await;
    let manager = seed_user(&ctx, "wf_mgr", "Петров П.П.", "manager").await;
    let admin = seed_user(&ctx, "wf_admin", "Смирнов С.С.", "admin").await;
    let stranger = seed_user(&ctx, "wf_other", "Кузнецов К.К.", "employee").await;
    let request_a = seed_request(&ctx, author, "free_form", "open").await;
    let request_b = seed_request(&ctx, stranger, "free_form", "open").await;
    add_participant(&ctx, request_a, participant).await;
    let cookies = Cookies {
        author: cookie(&ctx, author, Role::Employee).await,
        participant: cookie(&ctx, participant, Role::Employee).await,
        manager: cookie(&ctx, manager, Role::Manager).await,
        admin: cookie(&ctx, admin, Role::Admin).await,
        stranger: cookie(&ctx, stranger, Role::Employee).await,
    };
    let addr = serve(&ctx).await;
    World {
        ctx,
        _dir: dir,
        addr,
        author_id: author,
        other_author_id: stranger,
        request_a,
        request_b,
        cookies,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn chat_message_reaches_author_participant_staff_not_stranger() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let w = world().await;
        let mut author = connect(w.addr, &w.cookies.author).await;
        let mut participant = connect(w.addr, &w.cookies.participant).await;
        let mut manager = connect(w.addr, &w.cookies.manager).await;
        let mut admin = connect(w.addr, &w.cookies.admin).await;
        let mut stranger = connect(w.addr, &w.cookies.stranger).await;

        // The manager writes into the author's request (event under test).
        let manager_ident = ident(
            chat_support::count(&w.ctx, "SELECT id FROM users WHERE login = 'wf_mgr'").await,
            Role::Manager,
        );
        let out = w
            .ctx
            .chat
            .send(&manager_ident, w.request_a, SECRET, &[])
            .await
            .expect("send");
        let message_id = out.message.expect("message").id;

        // Control event: the stranger writes into their own request.
        let stranger_ident = ident(w.other_author_id, Role::Employee);
        w.ctx
            .chat
            .send(&stranger_ident, w.request_b, "контроль", &[])
            .await
            .expect("control send");

        for (who, ws) in [
            ("author", &mut author),
            ("participant", &mut participant),
            ("manager", &mut manager),
            ("admin", &mut admin),
        ] {
            let (raw, v) = frame(ws, who).await;
            assert_eq!(v["type"], "chat_message", "{who}: {raw}");
            assert_eq!(v["requestId"], w.request_a, "{who}: {raw}");
            assert_eq!(v["messageId"], message_id, "{who}: {raw}");
            // The frame carries ids and the author name only, never the text.
            assert!(
                !raw.contains(SECRET),
                "{who}: текст сообщения в кадре: {raw}"
            );
            assert!(
                v.get("text").is_none() && v.get("body").is_none(),
                "{who}: {raw}"
            );
        }

        // The stranger's first frame is the control event, so the message under
        // test was never delivered to them (broadcast order is total).
        let (raw, v) = frame(&mut stranger, "stranger").await;
        assert_eq!(v["type"], "chat_message", "{raw}");
        assert_eq!(v["requestId"], w.request_b, "чужое сообщение дошло: {raw}");
        w.ctx.shutdown.cancel();
    })
    .await
    .expect("chat_message_reaches_author_participant_staff_not_stranger exceeded 60s");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn chat_read_changed_only_to_marker_owner() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let w = world().await;
        let mut author1 = connect(w.addr, &w.cookies.author).await;
        let mut author2 = connect(w.addr, &w.cookies.author).await;
        let mut participant = connect(w.addr, &w.cookies.participant).await;
        let mut admin = connect(w.addr, &w.cookies.admin).await;

        let author_ident = ident(w.author_id, Role::Employee);
        w.ctx
            .chat
            .mark_read(&author_ident, w.request_a)
            .await
            .expect("mark_read");
        // Control event visible to everyone on request A.
        w.ctx
            .chat
            .send(&author_ident, w.request_a, "контроль", &[])
            .await
            .expect("control send");

        for (who, ws) in [("author#1", &mut author1), ("author#2", &mut author2)] {
            let (raw, v) = frame(ws, who).await;
            assert_eq!(v["type"], "chat_read_changed", "{who}: {raw}");
            assert_eq!(v["userId"], w.author_id, "{who}: {raw}");
            let (raw, v) = frame(ws, who).await;
            assert_eq!(v["type"], "chat_message", "{who} control: {raw}");
        }
        for (who, ws) in [("participant", &mut participant), ("admin", &mut admin)] {
            let (raw, v) = frame(ws, who).await;
            assert_eq!(
                v["type"], "chat_message",
                "{who}: чужая отметка прочтения дошла: {raw}"
            );
        }
        w.ctx.shutdown.cancel();
    })
    .await
    .expect("chat_read_changed_only_to_marker_owner exceeded 60s");
}
