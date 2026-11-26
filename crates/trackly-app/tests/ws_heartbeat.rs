//! Протокол служебных кадров WebSocket на НАСТОЯЩЕМ сокете (Phase 43, D-03/D-13):
//! hello первым кадром, серверный Ping, закрытие молчащего peer (и НЕ закрытие
//! живого), клиентский текстовый ping -> pong. Тайминги инжектируются через
//! `WsTimings` (tokio test-util в workspace нет). Имена вымышленные.

use std::net::SocketAddr;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
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

const PING: Duration = Duration::from_millis(100);
const IDLE: Duration = Duration::from_millis(300);

fn short() -> Option<WsTimings> {
    Some(WsTimings {
        ping_interval: PING,
        idle_timeout: IDLE,
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn hello_is_first_frame_with_timings() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (ctx, _d, c) = world().await;
        let addr = serve_with_timings(&ctx, short()).await;
        let mut ws = connect(addr, &c).await;
        let hello = frame(&mut ws, "hello").await;
        assert_eq!(hello["type"], "hello");
        assert_eq!(hello["protocol"], 1);
        // Секунды округляются вверх: 100 мс -> 1, 300 мс -> 1.
        assert_eq!(hello["pingIntervalSecs"], 1);
        assert_eq!(hello["idleTimeoutSecs"], 1);
        ctx.shutdown.cancel();
    })
    .await
    .expect("exceeded 30s");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn hello_default_timings_are_25_and_60() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (ctx, _d, c) = world().await;
        let addr = serve_with_timings(&ctx, None).await;
        let mut ws = connect(addr, &c).await;
        let hello = frame(&mut ws, "hello").await;
        assert_eq!(hello["pingIntervalSecs"], 25);
        assert_eq!(hello["idleTimeoutSecs"], 60);
        ctx.shutdown.cancel();
    })
    .await
    .expect("exceeded 30s");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn server_sends_periodic_pings() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (ctx, _d, c) = world().await;
        let addr = serve_with_timings(&ctx, short()).await;
        let mut ws = connect(addr, &c).await;
        let mut pings = 0;
        let end = tokio::time::Instant::now() + Duration::from_secs(1);
        while tokio::time::Instant::now() < end {
            if let Ok(Some(Ok(TMessage::Ping(_)))) =
                tokio::time::timeout(Duration::from_millis(200), ws.next()).await
            {
                pings += 1;
            }
        }
        assert!(pings >= 2, "ждали >=2 Ping за 1 с, пришло {pings}");
        ctx.shutdown.cancel();
    })
    .await
    .expect("exceeded 30s");
}

/// Мёртвый peer: клиент не читает сокет, значит не отвечает Pong; сервер обязан
/// закрыть соединение по idle. Парный тест ниже доказывает, что закрывают
/// именно молчащих, а не всех подряд.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn dead_peer_is_closed_after_idle() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (ctx, _d, c) = world().await;
        let addr = serve_with_timings(&ctx, short()).await;
        let mut ws = connect(addr, &c).await;
        // Не читаем 3 x idle.
        tokio::time::sleep(IDLE * 3).await;
        let closed = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                match ws.next().await {
                    None | Some(Err(_)) | Some(Ok(TMessage::Close(_))) => return true,
                    Some(Ok(_)) => continue,
                }
            }
        })
        .await
        .expect("поток остался открытым: idle-закрытие не работает");
        assert!(closed);
        ctx.shutdown.cancel();
    })
    .await
    .expect("exceeded 30s");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn live_peer_stays_connected_past_idle() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (ctx, _d, c) = world().await;
        let addr = serve_with_timings(&ctx, short()).await;
        let mut ws = connect(addr, &c).await;
        let end = tokio::time::Instant::now() + IDLE * 3;
        while tokio::time::Instant::now() < end {
            match tokio::time::timeout(Duration::from_millis(100), ws.next()).await {
                Ok(None) | Ok(Some(Err(_))) | Ok(Some(Ok(TMessage::Close(_)))) => {
                    panic!("живой peer закрыт сервером")
                }
                _ => {}
            }
        }
        // Всё ещё жив: контрольное событие доходит.
        ctx.ws_broadcast.send(space("alive")).expect("send");
        loop {
            let v = frame(&mut ws, "live").await;
            if v["type"] == "number_space_changed" {
                assert_eq!(v["contexts"][0], "alive");
                break;
            }
        }
        ctx.shutdown.cancel();
    })
    .await
    .expect("exceeded 30s");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn client_ping_gets_pong_and_garbage_is_ignored() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (ctx, _d, c) = world().await;
        let addr = serve_with_timings(&ctx, None).await;
        let mut ws = connect(addr, &c).await;
        let _hello = frame(&mut ws, "hello").await;

        ws.send(TMessage::Text("мусор, не json".into()))
            .await
            .unwrap();
        ws.send(TMessage::Binary(vec![1, 2, 3].into()))
            .await
            .unwrap();
        ws.send(TMessage::Text(r#"{"type":"nope"}"#.into()))
            .await
            .unwrap();
        ws.send(TMessage::Text(r#"{"type":"ping"}"#.into()))
            .await
            .unwrap();

        let v = frame(&mut ws, "pong").await;
        assert_eq!(v, serde_json::json!({"type":"pong"}));

        // Соединение живо после мусора: событие доходит.
        ctx.ws_broadcast.send(space("after")).expect("send");
        let v = frame(&mut ws, "event").await;
        assert_eq!(v["type"], "number_space_changed");
        ctx.shutdown.cancel();
    })
    .await
    .expect("exceeded 30s");
}
