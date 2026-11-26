//! Фан-аут IM на НАСТОЯЩЕМ WebSocket (Phase 43-08, DLV-02, D-04, D-05, D-07).
//! Сигнал `im_changed` — только conversationId и rev; не участник и исключённый
//! его не получают. Имена и текст вымышленные.
//!
//! Отрицательные проверки без sleep: сигналы строго упорядочены в канале
//! пользователя, поэтому ПЕРВЫЙ кадр после контрольного сигнала в другую беседу
//! обязан быть контрольным, а не проверяемым.

use std::net::SocketAddr;
use std::time::Duration;

use futures_util::StreamExt;
use serde_json::Value;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message as TMessage;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use trackly_app::context::AppCtx;
use trackly_app::http::build_router;
use trackly_app::server::rusqlite_session_store::RusqliteSessionStore;
use trackly_app::services::im_hub::ImSignal;
use trackly_core::auth::Role;

mod im_support;
use im_support::{build_ctx, cookie, ident, seed_user};

type Ws = WebSocketStream<MaybeTlsStream<TcpStream>>;

const SECRET: &str = "секретная фраза про закупку";

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
    let (mut ws, _) = tokio_tungstenite::connect_async(req)
        .await
        .expect("ws connect");
    let (raw, v) = frame(&mut ws, "hello").await;
    assert_eq!(v["type"], "hello", "первым кадром ждали hello: {raw}");
    ws
}

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
    a: i64,
    b: i64,
    d: i64,
    conv1: i64,
    conv2: i64,
    ck_a: String,
    ck_b: String,
    ck_c: String,
}

/// A, B — в беседе 1; C — нет. В беседе 2 участники D, B, C.
async fn world() -> World {
    let (ctx, dir) = build_ctx().await;
    let a = seed_user(&ctx, "fw_a", "Иванов И.И.", "employee").await;
    let b = seed_user(&ctx, "fw_b", "Петров П.П.", "employee").await;
    let c = seed_user(&ctx, "fw_c", "Сидорова А.А.", "employee").await;
    let d = seed_user(&ctx, "fw_d", "Кузнецов К.К.", "employee").await;
    let conv1 = ctx
        .im
        .create_conversation(a, "room", Some("Первая"), &[b])
        .await
        .unwrap();
    let conv2 = ctx
        .im
        .create_conversation(d, "room", Some("Вторая"), &[b, c])
        .await
        .unwrap();
    let addr = serve(&ctx).await;
    World {
        ck_a: cookie(&ctx, a, Role::Employee).await,
        ck_b: cookie(&ctx, b, Role::Employee).await,
        ck_c: cookie(&ctx, c, Role::Employee).await,
        ctx,
        _dir: dir,
        addr,
        a,
        b,
        d,
        conv1,
        conv2,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn im_signal_reaches_members_only_without_body() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let w = world().await;
        let mut a = connect(w.addr, &w.ck_a).await;
        let mut b = connect(w.addr, &w.ck_b).await;
        let mut c = connect(w.addr, &w.ck_c).await;

        let out = w
            .ctx
            .im
            .send(&ident(w.a, Role::Employee), w.conv1, "m1", SECRET)
            .await
            .expect("send");
        // Контроль: D пишет в беседу 2, где участник C.
        w.ctx
            .im
            .send(&ident(w.d, Role::Employee), w.conv2, "c1", "контроль")
            .await
            .expect("control");

        for (who, ws) in [("a", &mut a), ("b", &mut b)] {
            let (raw, v) = frame(ws, who).await;
            assert_eq!(v["type"], "im_changed", "{who}: {raw}");
            assert_eq!(v["conversationId"], w.conv1, "{who}: {raw}");
            assert_eq!(v["rev"], out.message.rev, "{who}: {raw}");
            assert!(!raw.contains(SECRET), "{who}: текст в кадре: {raw}");
            assert!(
                v.get("text").is_none() && v.get("body").is_none() && v.get("bodyText").is_none(),
                "{who}: {raw}"
            );
        }
        // C не участник беседы 1: первый кадр — контрольный сигнал беседы 2.
        let (raw, v) = frame(&mut c, "c").await;
        assert_eq!(v["type"], "im_changed", "{raw}");
        assert_eq!(v["conversationId"], w.conv2, "чужой сигнал дошёл: {raw}");
        w.ctx.shutdown.cancel();
    })
    .await
    .expect("exceeded 60s");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn removed_member_gets_no_further_signals() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let w = world().await;
        let b_id = w.b;
        let mut a = connect(w.addr, &w.ck_a).await;
        let mut b = connect(w.addr, &w.ck_b).await;
        let author = ident(w.a, Role::Employee);

        // До исключения B сигнал получает.
        w.ctx.im.send(&author, w.conv1, "p1", "до").await.unwrap();
        let (raw, v) = frame(&mut b, "b до").await;
        assert_eq!(v["conversationId"], w.conv1, "{raw}");
        let _ = frame(&mut a, "a до").await;

        w.ctx.im.remove_member(w.conv1, b_id).await.unwrap();
        w.ctx
            .im
            .send(&author, w.conv1, "p2", "после")
            .await
            .unwrap();
        // Контроль в беседу 2, где B остаётся участником.
        w.ctx
            .im
            .send(&ident(w.d, Role::Employee), w.conv2, "c2", "контроль")
            .await
            .unwrap();

        let (raw, v) = frame(&mut a, "a после").await;
        assert_eq!(v["conversationId"], w.conv1, "{raw}");
        let (raw, v) = frame(&mut b, "b после").await;
        assert_eq!(
            v["conversationId"], w.conv2,
            "исключённый получил сигнал: {raw}"
        );
        w.ctx.shutdown.cancel();
    })
    .await
    .expect("exceeded 60s");
}

/// D-07: переполнение канала IM -> resync_required, соединение живо.
#[tokio::test(flavor = "current_thread")]
async fn im_lagged_yields_resync_required_and_keeps_connection() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let w = world().await;
        let mut a = connect(w.addr, &w.ck_a).await;
        let sig = |rev| ImSignal::Changed {
            conversation_id: w.conv1,
            rev,
        };
        // Синхронный цикл без await: сокет-задача не успевает читать.
        for rev in 0..400 {
            w.ctx.im_hub.publish(&[w.a], sig(rev));
        }
        let (raw, v) = frame(&mut a, "a lagged").await;
        assert_eq!(v["type"], "resync_required", "{raw}");

        w.ctx.im_hub.publish(&[w.a], sig(1000));
        let mut got = false;
        for _ in 0..300 {
            let (_, v) = frame(&mut a, "a после lagged").await;
            if v["type"] == "im_changed" && v["rev"] == 1000 {
                got = true;
                break;
            }
        }
        assert!(got, "соединение не доставило обычный сигнал после resync");
        w.ctx.shutdown.cancel();
    })
    .await
    .expect("exceeded 60s");
}

/// Системный администратор десктопа (без user_id) подключается без ошибки.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn identity_without_user_id_connects() {
    tokio::time::timeout(Duration::from_secs(60), async {
        use tower_sessions::session::{Id, Record};
        use tower_sessions::SessionStore;
        let w = world().await;
        let store = RusqliteSessionStore::new(w.ctx.writer.clone(), w.ctx.readers.clone());
        let id = Id::default();
        let si = trackly_app::http::auth::SessionIdentity {
            user_id: None,
            role: "admin".to_string(),
        };
        let mut record = Record {
            id,
            data: Default::default(),
            expiry_date: time::OffsetDateTime::now_utc() + time::Duration::days(1),
        };
        record
            .data
            .insert("identity".to_string(), serde_json::to_value(&si).unwrap());
        store.create(&mut record).await.unwrap();
        let _ws = connect(w.addr, &format!("id={id}")).await;
        w.ctx.shutdown.cancel();
    })
    .await
    .expect("exceeded 60s");
}
