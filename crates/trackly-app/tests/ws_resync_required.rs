//! Lagged общего канала -> кадр `resync_required` на НАСТОЯЩЕМ сокете
//! (Phase 43, D-07). Рантайм current_thread: пока тест синхронно (без `.await`)
//! шлёт пачку событий, серверная задача не может вычитывать канал, поэтому
//! переполнение буфера (128) детерминировано. Имена вымышленные.

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
use trackly_app::dto::printer::WsEvent;
use trackly_app::http::auth::SessionIdentity;
use trackly_app::http::build_router;
use trackly_app::http::ws::WsTimings;
use trackly_app::server::rusqlite_session_store::RusqliteSessionStore;
use trackly_core::auth::Role;

mod chat_support;
use chat_support::seed_user;

type Ws = WebSocketStream<MaybeTlsStream<TcpStream>>;

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

async fn serve_with_timings(ctx: &AppCtx, timings: Option<WsTimings>) -> SocketAddr {
    let mut app = build_router(
        ctx,
        RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone()),
    );
    if let Some(t) = timings {
        app = app.layer(axum::Extension(t));
    }
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

/// Next text frame as JSON (control frames skipped).
async fn frame(ws: &mut Ws, who: &str) -> Value {
    loop {
        let m = tokio::time::timeout(Duration::from_secs(5), ws.next())
            .await
            .unwrap_or_else(|_| panic!("{who}: no frame within 5s"))
            .unwrap_or_else(|| panic!("{who}: stream closed"))
            .expect("ws frame");
        if let TMessage::Text(t) = m {
            return serde_json::from_str(t.as_str()).expect("json frame");
        }
    }
}

async fn world() -> (AppCtx, tempfile::TempDir, String) {
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
    let admin = seed_user(&ctx, "hb_admin", "Смирнов С.С.", "admin").await;
    let c = cookie(&ctx, admin, Role::Admin).await;
    (ctx, dir, c)
}

fn space(tag: &str) -> WsEvent {
    WsEvent::NumberSpaceChanged {
        contexts: vec![tag.to_string()],
    }
}

#[tokio::test]
async fn lagged_client_gets_resync_required_and_stays_connected() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let (ctx, _d, c) = world().await;
        let addr = serve_with_timings(&ctx, None).await;
        let mut ws = connect(addr, &c).await;

        // До переполнения приходит только hello (тест не вакуумен).
        let hello = frame(&mut ws, "hello").await;
        assert_eq!(hello["type"], "hello");

        // Синхронная пачка > ёмкости канала (128): серверный цикл не успевает.
        for i in 0..300 {
            ctx.ws_broadcast
                .send(space(&format!("n{i}")))
                .expect("send");
        }

        let v = frame(&mut ws, "resync").await;
        assert_eq!(v, serde_json::json!({"type":"resync_required"}));

        // Соединение живо: контрольное событие приходит обычным кадром.
        ctx.ws_broadcast.send(space("control")).expect("send");
        loop {
            let v = frame(&mut ws, "control").await;
            assert_ne!(v["type"], "resync_required", "повторный resync без причины");
            if v["type"] == "number_space_changed" && v["contexts"][0] == "control" {
                break;
            }
        }
        ctx.shutdown.cancel();
    })
    .await
    .expect("exceeded 60s");
}
