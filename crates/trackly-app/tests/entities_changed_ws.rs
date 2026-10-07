//! Phase 41.7, план 12: сквозная проверка D-02 на НАСТОЯЩИХ сокетах.
//!
//! Поднимается реальный сервер (`build_router` + `axum::serve` с `ConnectInfo`),
//! три WS-клиента подключаются к `/api/v1/ws` с cookie сессии своей роли
//! (admin, manager, employee). Мутация идёт через `ctx.places` — настоящий
//! сервисный слой. Проверяется:
//!   * admin и manager получают ТЕКСТОВЫЙ кадр `entities_changed` в camelCase;
//!   * employee его НЕ получает, при этом его сокет жив и получает другие
//!     кадры (молчание не следствие незавершённого апгрейда).
//!
//! Подписка каждого сокета подтверждается контрольными событиями ДО мутации:
//! `NumberSpaceChanged` (видят admin/manager) и `RequestStatusChanged` с автором
//! `employee` (видят все трое). Данные вымышленные.

mod entities_support;

use std::net::SocketAddr;
use std::time::Duration;

use futures_util::StreamExt;
use time::OffsetDateTime;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message as TMessage;
use tokio_tungstenite::{client_async, MaybeTlsStream, WebSocketStream};
use tokio_util::sync::CancellationToken;
use tower_sessions::session::{Id, Record};
use tower_sessions::SessionStore;

use entities_support::fixture::{admin, make_test_ctx, seed_place};
use trackly_app::dto::auth::UserNew;
use trackly_app::dto::printer::WsEvent;
use trackly_app::http::auth::SessionIdentity;
use trackly_app::http::build_router;
use trackly_app::server::rusqlite_session_store::RusqliteSessionStore;
use trackly_core::auth::Role;

type Ws = WebSocketStream<MaybeTlsStream<TcpStream>>;

const BUDGET: Duration = Duration::from_secs(60);
const RECV: Duration = Duration::from_secs(3);
const SILENCE: Duration = Duration::from_secs(1);

async fn create_session_cookie(store: &RusqliteSessionStore, user_id: i64, role: Role) -> String {
    let session_id = Id::default();
    let si = SessionIdentity {
        user_id: Some(user_id),
        role: role.as_str().to_string(),
    };
    let mut record = Record {
        id: session_id,
        data: Default::default(),
        expiry_date: OffsetDateTime::now_utc() + time::Duration::days(1),
    };
    record.data.insert(
        "identity".to_string(),
        serde_json::to_value(&si).expect("serialize identity"),
    );
    store.create(&mut record).await.expect("create session");
    format!("id={session_id}")
}

async fn mk_user(ctx: &trackly_app::context::AppCtx, login: &str, name: &str, role: &str) -> i64 {
    ctx.auth
        .create_user(
            UserNew {
                login: login.to_string(),
                full_name: name.to_string(),
                password: "password123".to_string(),
                role: role.to_string(),
                email: None,
            },
            &admin(),
        )
        .await
        .expect("create user")
        .id
}

async fn connect_ws(addr: SocketAddr, cookie: &str) -> Ws {
    let url = format!("ws://{addr}/api/v1/ws");
    let mut request = url.into_client_request().expect("client request");
    request
        .headers_mut()
        .insert("cookie", cookie.parse().expect("cookie header"));
    let tcp = TcpStream::connect(addr).await.expect("tcp connect");
    let (ws, resp) = client_async(request, MaybeTlsStream::Plain(tcp))
        .await
        .expect("ws handshake (101 expected, 401 means the cookie was rejected)");
    assert_eq!(resp.status().as_u16(), 101, "handshake must upgrade");
    ws
}

/// Следующий ТЕКСТОВЫЙ кадр как JSON; паника, если за `RECV` ничего не пришло.
async fn next_json(ws: &mut Ws, who: &str) -> serde_json::Value {
    loop {
        let msg = tokio::time::timeout(RECV, ws.next())
            .await
            .unwrap_or_else(|_| panic!("{who}: кадр не пришёл за {RECV:?}"))
            .unwrap_or_else(|| panic!("{who}: поток закрыт сервером"))
            .unwrap_or_else(|e| panic!("{who}: ошибка сокета: {e}"));
        match msg {
            TMessage::Text(t) => {
                return serde_json::from_str(t.as_str())
                    .unwrap_or_else(|e| panic!("{who}: кадр не JSON: {e}: {t}"));
            }
            TMessage::Ping(_) | TMessage::Pong(_) => continue,
            other => panic!("{who}: ожидали текстовый кадр, пришло {other:?}"),
        }
    }
}

fn type_of(v: &serde_json::Value) -> &str {
    v["type"].as_str().expect("frame has string `type`")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn admin_and_manager_receive_entities_changed_employee_does_not() {
    tokio::time::timeout(BUDGET, async {
        let (ctx, _dir) = make_test_ctx().await;

        let admin_id = mk_user(&ctx, "ws_admin", "Иванов И.И.", "admin").await;
        let manager_id = mk_user(&ctx, "ws_manager", "Петров П.П.", "manager").await;
        let employee_id = mk_user(&ctx, "ws_employee", "Сидоров С.С.", "employee").await;

        let store = RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone());
        let admin_cookie = create_session_cookie(&store, admin_id, Role::Admin).await;
        let manager_cookie = create_session_cookie(&store, manager_id, Role::Manager).await;
        let employee_cookie = create_session_cookie(&store, employee_id, Role::Employee).await;

        let router = build_router(
            &ctx,
            RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone()),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("local_addr");
        let stop = CancellationToken::new();
        let stop_clone = stop.clone();
        let server = tokio::spawn(async move {
            axum::serve(
                listener,
                router.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .with_graceful_shutdown(async move { stop_clone.cancelled().await })
            .await
            .expect("axum::serve");
        });

        let mut ws_admin = connect_ws(addr, &admin_cookie).await;
        let mut ws_manager = connect_ws(addr, &manager_cookie).await;
        let mut ws_employee = connect_ws(addr, &employee_cookie).await;

        // --- Контрольные события: подписка каждого сокета подтверждена ДО мутации.
        // Апгрейд завершается в `on_upgrade`, а подписка на broadcast делается ещё
        // в `ws_handler`, поэтому к моменту 101 подписчик уже есть; контрольные кадры
        // это подтверждают, а не предполагают.
        ctx.ws_broadcast
            .send(WsEvent::NumberSpaceChanged {
                contexts: vec!["device_create".to_string()],
            })
            .expect("send NumberSpaceChanged");
        ctx.ws_broadcast
            .send(WsEvent::RequestStatusChanged {
                request_id: 1,
                new_status: "accepted".to_string(),
                requested_by_user_id: employee_id,
            })
            .expect("send RequestStatusChanged");

        for (who, ws) in [("admin", &mut ws_admin), ("manager", &mut ws_manager)] {
            let f1 = next_json(ws, who).await;
            assert_eq!(type_of(&f1), "number_space_changed", "{who}: control #1");
            let f2 = next_json(ws, who).await;
            assert_eq!(type_of(&f2), "request_status_changed", "{who}: control #2");
        }
        // Сотрудник: NumberSpaceChanged ему не виден, RequestStatusChanged (он автор) виден.
        let e1 = next_json(&mut ws_employee, "employee").await;
        assert_eq!(
            type_of(&e1),
            "request_status_changed",
            "employee: первым должен прийти ТОЛЬКО свой RequestStatusChanged, \
             number_space_changed ему не виден"
        );

        // --- Мутация через настоящий сервисный слой.
        let place_id = seed_place(&ctx, "Кабинет 101").await;
        ctx.places
            .rename(&admin(), place_id, "Кабинет 102".to_string(), 1)
            .await
            .expect("rename place");

        // admin и manager: кадр равен литералу (проверка провода, ключи camelCase).
        let expected = serde_json::json!({
            "type": "entities_changed",
            "placeIds": [place_id],
            "deviceIds": [],
            "groupIds": [],
        });
        for (who, ws) in [("admin", &mut ws_admin), ("manager", &mut ws_manager)] {
            let frame = next_json(ws, who).await;
            assert_eq!(frame, expected, "{who}: кадр entities_changed");
            assert_eq!(type_of(&frame), "entities_changed");
            assert!(
                frame["placeIds"]
                    .as_array()
                    .expect("placeIds is array")
                    .iter()
                    .all(|v| v.is_number()),
                "{who}: placeIds — массив чисел"
            );
        }

        // employee: за SILENCE не приходит НИЧЕГО.
        let silent = tokio::time::timeout(SILENCE, ws_employee.next()).await;
        assert!(
            silent.is_err(),
            "employee получил кадр, хотя entities_changed ему не виден: {silent:?}"
        );

        // Негатив осмысленный: сокет сотрудника жив, и следующий кадр — только что
        // посланный контрольный, а не отфильтрованный/залежавшийся entities_changed.
        ctx.ws_broadcast
            .send(WsEvent::RequestStatusChanged {
                request_id: 2,
                new_status: "completed".to_string(),
                requested_by_user_id: employee_id,
            })
            .expect("send trailing control");
        let tail = next_json(&mut ws_employee, "employee").await;
        assert_eq!(type_of(&tail), "request_status_changed");
        assert_eq!(tail["requestId"], 2, "employee: хвостовой контрольный кадр");

        stop.cancel();
        let _ = server.await;
        ctx.shutdown.cancel();
    })
    .await
    .expect("admin_and_manager_receive_entities_changed_employee_does_not вышел за бюджет");
}
