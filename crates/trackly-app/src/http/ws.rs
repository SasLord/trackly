//! WebSocket handler — Phase 6 Plan 03.
//!
//! ## Security (T-06-09-E — ASVS V2/V4)
//!
//! Auth gate реализован через `axum::middleware::from_fn_with_state`:
//! middleware проверяет сессию и возвращает 401 ДО передачи запроса в ws_handler.
//! Это гарантирует что WS-соединение (on_upgrade) открывается только для
//! аутентифицированных клиентов (Pitfall 6 из RESEARCH.md).
//!
//! ## Liveness (Pitfall 5, Phase 43 D-07)
//!
//! `Lagged(n)` от `broadcast::Receiver` означает, что клиент отстал и n событий
//! потеряны. Сессию не завершаем (НЕ `break`), но и молча не продолжаем: клиенту
//! уходит служебный кадр `{"type":"resync_required"}`, по которому он перечитывает
//! данные. Ёмкость канала (128) не увеличивается. То же для канала IM (Lagged -> `resync_required`).
//!
//! ## Служебные кадры протокола (Phase 43 D-03, D-13)
//!
//! - сервер -> клиент, первым кадром: `{"type":"hello","protocol":1,
//!   "pingIntervalSecs":N,"idleTimeoutSecs":M}` (секунды, округление вверх —
//!   для коротких таймингов теста даёт 1);
//! - сервер -> клиент: WS-кадр `Ping` каждые `PING_INTERVAL`; соединение, от
//!   которого за `IDLE_TIMEOUT` не пришло ни одного кадра (Pong тоже считается),
//!   закрывается;
//! - клиент -> сервер: текст `{"type":"ping"}` -> ответ `{"type":"pong"}`;
//!   прочий клиентский ввод игнорируется;
//! - сервер -> клиент: `{"type":"resync_required"}` при потере событий.
//!
//! Служебные кадры НЕ являются вариантами `WsEvent` и не проходят `is_visible_to`.
//!
//! ## Остановка серверного режима
//!
//! `handle_ws_socket` обязан сам закрывать сокет по токену `ServerShutdown`:
//! апгрейженный сокет живёт в задаче, которую axum спаунит в `on_upgrade`, и
//! внешняя отмена hyper-соединения до него не доходит. Подробности — в докблоке
//! функции (отладочная сессия `ws-disconnect-toast-no-show`).
//!
//! ## Сигналы IM (Phase 43 D-04)
//!
//! Отдельная ветка `select!`: `{"type":"im_changed","conversationId":N,"rev":M}` из
//! `ImHub` пользователя, без тела сообщения; `is_visible_to` к IM не применяется.
//!
//! ## Visibility filter (T-06-06-I)
//!
//! Каждое событие проверяется через `WsEvent::is_visible_to(&identity)` перед
//! отправкой — сотрудник не получает PrinterAlert, которые видны только Admin/Manager.

use axum::{
    extract::ws::{Message, WebSocket},
    extract::{Extension, Request, State, WebSocketUpgrade},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    routing::any,
    Router,
};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use tower_sessions::Session;

use crate::context::AppCtx;
use crate::dto::printer::WsEvent;
use crate::http::auth::session_identity;
use crate::server::ServerShutdown;
use crate::services::im_hub::{ImHub, ImSignal};
use trackly_core::auth::Identity;

/// Сколько ждём отправку прощального Close-кадра при остановке сервера.
/// Не «сколько нужно», а «сколько не жалко»: кадр — вежливость, а не условие
/// закрытия. По истечении сокет всё равно дропается.
const CLOSE_SEND_TIMEOUT: Duration = Duration::from_secs(1);

/// Таймаут обычной отправки: зависший `send` к мёртвому peer не должен вешать задачу.
const SEND_TIMEOUT: Duration = Duration::from_secs(5);

/// Версия протокола служебных кадров (поле `protocol` в hello).
pub const WS_PROTOCOL_VERSION: u32 = 1;

/// Период серверных Ping (D-13: константа, не настройка).
pub const PING_INTERVAL: Duration = Duration::from_secs(25);

/// Через сколько тишины соединение считается мёртвым (D-13: константа, не настройка).
pub const IDLE_TIMEOUT: Duration = Duration::from_secs(60);

/// Максимальная длина клиентского текста, который вообще разбирается как ping.
const CLIENT_TEXT_MAX: usize = 256;

/// Тайминги сокета. В production не подставляются (берётся `Default`); тесты
/// кладут короткие значения через `.layer(axum::Extension(WsTimings{..}))`.
#[derive(Clone, Copy, Debug)]
pub struct WsTimings {
    pub ping_interval: Duration,
    pub idle_timeout: Duration,
}

impl Default for WsTimings {
    fn default() -> Self {
        Self {
            ping_interval: PING_INTERVAL,
            idle_timeout: IDLE_TIMEOUT,
        }
    }
}

/// Отправить кадр с ограничением по времени; `false` — соединение считать мёртвым.
async fn send_msg(socket: &mut WebSocket, msg: Message) -> bool {
    matches!(
        tokio::time::timeout(SEND_TIMEOUT, socket.send(msg)).await,
        Ok(Ok(()))
    )
}

async fn send_text(socket: &mut WebSocket, text: &str) -> bool {
    send_msg(socket, Message::Text(text.to_string().into())).await
}

/// Клиентский текст — это ping? Только короткий JSON с `type == "ping"`.
fn is_client_ping(text: &str) -> bool {
    text.len() <= CLIENT_TEXT_MAX
        && serde_json::from_str::<serde_json::Value>(text)
            .map(|v| v["type"] == "ping")
            .unwrap_or(false)
}

/// Middleware: проверяет сессию перед WS upgrade.
///
/// ## Auth gate (T-06-09-E — ASVS V4 — Pitfall 6)
///
/// При отсутствии или невалидной сессии возвращает HTTP 401 ДО передачи
/// запроса в `ws_handler`. Это гарантирует что WS upgrade не происходит
/// для неаутентифицированных клиентов.
///
/// Успешно прошедший auth identity передаётся через `Extension<Identity>`
/// в `ws_handler` без повторной проверки.
pub async fn ws_auth_middleware(session: Session, mut req: Request, next: Next) -> Response {
    match session_identity(&session).await {
        Ok(identity) => {
            req.extensions_mut().insert(identity);
            next.run(req).await
        }
        Err(_) => StatusCode::UNAUTHORIZED.into_response(),
    }
}

/// WebSocket upgrade handler.
///
/// Identity уже проверена middleware — читается из `Extension<Identity>`.
/// `ws: WebSocketUpgrade` — стандартный axum WS extractor.
///
/// `server_shutdown` — токен остановки инстанса сервера, положенный в extensions
/// в `server::serve_connection_inner`. `Option`, потому что роутер поднимают и
/// без `start_server` (интеграционные тесты через `axum::serve`); в этом случае
/// сокет живёт до разрыва клиентом, как и раньше.
pub async fn ws_handler(
    State(ctx): State<AppCtx>,
    Extension(identity): Extension<Identity>,
    session: Session,
    server_shutdown: Option<Extension<ServerShutdown>>,
    timings: Option<Extension<WsTimings>>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    // Link (D-14): соединение устройства регистрируется в хабе, чтобы отзыв рвал его
    // сразу. Браузерные сессии токена не получают.
    let link_cancel = match session
        .get::<String>(crate::http::link::SESSION_DEVICE_KEY)
        .await
    {
        Ok(Some(device_id)) => identity
            .user_id
            .map(|uid| ctx.link_hub.register(&device_id, uid)),
        _ => None,
    };
    let rx = ctx.ws_broadcast.subscribe();
    // D-04: канал IM только у пользователя с личностью (у системного администратора
    // десктопа user_id нет — ветка IM для него неактивна).
    let im_rx = identity.user_id.map(|uid| ctx.im_hub.subscribe(uid));
    let im_hub = ctx.im_hub.clone();
    let shutdown = server_shutdown.map(|Extension(ServerShutdown(token))| token);
    let timings = timings.map(|Extension(t)| t).unwrap_or_default();
    ws.on_upgrade(move |socket| {
        handle_ws_socket(
            socket,
            identity,
            rx,
            im_rx,
            im_hub,
            shutdown,
            link_cancel,
            timings,
        )
    })
}

/// WebSocket connection loop.
///
/// Первым кадром уходит hello. Затем `tokio::select!` на шести ветвях (шестая — сигналы IM):
///   1. `rx.recv()` — fan-out broadcast events to the connected client
///      (Lagged -> `resync_required`).
///   2. `socket.recv()` — клиентские кадры: disconnect, текстовый ping -> pong;
///      любой кадр сбрасывает idle-таймер.
///   3. `shutdown.cancelled()` — серверный режим останавливают: закрыть сокет.
///   4. `ping_tick` — серверный Ping каждые `ping_interval`.
///   5. `idle` — `idle_timeout` тишины: закрыть соединение.
///
/// Ветвь 3 ОБЯЗАТЕЛЬНА, и вот почему её нельзя заменить ничем снаружи. Задачу с
/// этим сокетом спаунит axum внутри `on_upgrade`, владение IO ей передаёт
/// `hyper::upgrade::on`, поэтому дроп future hyper-соединения в
/// `server::start_server` её НЕ трогает (проверено рантаймом — тест
/// `ws_close_on_server_stop` оставался красным). А сама по себе эта функция не
/// завершится никогда: `socket.recv()` вечно pending (клиент молчит), а `rx.recv()` отдаст `Closed` только при
/// дропе `broadcast::Sender`, который лежит в `AppCtx.ws_broadcast` и остановку
/// сервера переживает. Итог до фикса: после «Остановить сервер» сокет оставался
/// открытым, браузерный `ws.onclose` не наступал, и тост D-18 «Соединение с
/// сервером потеряно» не показывался НИКОГДА (отладочная сессия
/// `ws-disconnect-toast-no-show`, GAP-1 фазы 41.7).
// Восемь аргументов: каждый — независимый источник завершения/сигналов соединения.
#[allow(clippy::too_many_arguments)]
async fn handle_ws_socket(
    socket: WebSocket,
    identity: Identity,
    rx: broadcast::Receiver<WsEvent>,
    im_rx: Option<broadcast::Receiver<ImSignal>>,
    im_hub: Arc<ImHub>,
    shutdown: Option<CancellationToken>,
    link_cancel: Option<CancellationToken>,
    timings: WsTimings,
) {
    ws_loop(socket, identity, rx, im_rx, shutdown, link_cancel, timings).await;
    // Подписка IM снимается при любом выходе; пустые каналы чистятся.
    im_hub.prune();
}

async fn ws_loop(
    mut socket: WebSocket,
    identity: Identity,
    mut rx: broadcast::Receiver<WsEvent>,
    mut im_rx: Option<broadcast::Receiver<ImSignal>>,
    shutdown: Option<CancellationToken>,
    link_cancel: Option<CancellationToken>,
    timings: WsTimings,
) {
    // Токен отзыва Link: у браузерных соединений его нет — вечно-pending ветвь.
    let link_cancelled = async {
        match link_cancel.as_ref() {
            Some(t) => t.cancelled().await,
            None => std::future::pending().await,
        }
    };
    tokio::pin!(link_cancelled);
    // `select!` требует future в каждой ветви. Когда токена нет (роутер поднят
    // без `start_server`), подставляем вечно-pending ветвь, чтобы поведение
    // осталось прежним, а не «мгновенно закрыть сокет».
    let shutdown = shutdown.unwrap_or_default();

    let hello = serde_json::json!({
        "type": "hello",
        "protocol": WS_PROTOCOL_VERSION,
        "pingIntervalSecs": timings.ping_interval.as_secs_f64().ceil() as u64,
        "idleTimeoutSecs": timings.idle_timeout.as_secs_f64().ceil() as u64,
    });
    if !send_text(&mut socket, &hello.to_string()).await {
        return;
    }

    let mut ping_tick = tokio::time::interval_at(
        tokio::time::Instant::now() + timings.ping_interval,
        timings.ping_interval,
    );
    ping_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let idle = tokio::time::sleep(timings.idle_timeout);
    tokio::pin!(idle);

    loop {
        tokio::select! {
            () = &mut link_cancelled => {
                // D-14: доступ отозван — закрываем кодом 4401 без ожидания запроса.
                let frame = axum::extract::ws::CloseFrame {
                    code: link_proto::dto::REVOKED_CLOSE_CODE,
                    reason: "revoked".into(),
                };
                let _ = tokio::time::timeout(
                    CLOSE_SEND_TIMEOUT,
                    socket.send(Message::Close(Some(frame))),
                )
                .await;
                break;
            }
            () = shutdown.cancelled() => {
                // Закрываемся вежливо: браузер получит onclose с кодом нормального
                // закрытия вместо обрыва 1006. Ошибку отправки игнорируем — сокет
                // всё равно дропается следующей строкой.
                // Таймаут обязателен: если peer перестал читать, окно отправки
                // заполнено и `send` висит — задача снова жила бы вечно, т.е.
                // ровно тот дефект, который этой ветвью и лечится.
                let _ = tokio::time::timeout(
                    CLOSE_SEND_TIMEOUT,
                    socket.send(Message::Close(None)),
                )
                .await;
                break;
            }
            () = &mut idle => {
                // Тишина дольше idle_timeout: peer мёртв (полуоткрытый сокет).
                let _ = tokio::time::timeout(
                    CLOSE_SEND_TIMEOUT,
                    socket.send(Message::Close(None)),
                )
                .await;
                break;
            }
            _ = ping_tick.tick() => {
                if !send_msg(&mut socket, Message::Ping(Vec::new().into())).await {
                    break;
                }
            }
            im_event = async {
                match im_rx.as_mut() {
                    Some(r) => r.recv().await,
                    None => std::future::pending().await,
                }
            } => {
                // D-04: сигнал IM не проходит is_visible_to — получатели определены
                // сервисом по членству; кадр без тела: conversationId и rev.
                match im_event {
                    Ok(signal) => match serde_json::to_string(&signal) {
                        Ok(json) => {
                            if !send_text(&mut socket, &json).await {
                                break;
                            }
                        }
                        Err(e) => tracing::warn!("ws: serialize ImSignal failed: {e}"),
                    },
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        // D-07: как и у общего канала — сообщаем о потере сигналов.
                        tracing::warn!("ws: im client lagged {n} signals — resync_required");
                        if !send_text(&mut socket, r#"{"type":"resync_required"}"#).await {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        // Канал закрыт: читать больше нечего, соединение остаётся.
                        im_rx = None;
                    }
                }
            }
            event = rx.recv() => {
                match event {
                    Ok(evt) if evt.is_visible_to(&identity) => {
                        // Serialize and send — disconnect on send error.
                        let json = match serde_json::to_string(&evt) {
                            Ok(j) => j,
                            Err(e) => {
                                tracing::warn!("ws: serialize WsEvent failed: {e}");
                                continue;
                            }
                        };
                        if !send_text(&mut socket, &json).await {
                            // Client disconnected, send buffer full or send timed out.
                            break;
                        }
                    }
                    Ok(_) => {
                        // Event not visible to this identity — silently skip.
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        // Клиент отстал: n событий потеряны. Сессию не рвём, но
                        // сообщаем клиенту, чтобы он перечитал данные (D-07).
                        tracing::warn!("ws: client lagged {n} events — resync_required");
                        if !send_text(&mut socket, r#"{"type":"resync_required"}"#).await {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        // Sender dropped — server shutting down.
                        break;
                    }
                }
            }
            msg = socket.recv() => {
                // Client side: disconnect signals.
                match msg {
                    None => break,
                    Some(Err(_)) => break,
                    Some(Ok(Message::Close(_))) => break,
                    Some(Ok(other)) => {
                        // Любой входящий кадр — признак жизни.
                        idle.as_mut().reset(tokio::time::Instant::now() + timings.idle_timeout);
                        if let Message::Text(t) = other {
                            if is_client_ping(t.as_str())
                                && !send_text(&mut socket, r#"{"type":"pong"}"#).await
                            {
                                break;
                            }
                        }
                        // Остальное (Binary, Ping, Pong, прочий текст) игнорируется.
                    }
                }
            }
        }
    }
}

/// Router for `/api/v1/ws` (GET — WebSocket upgrade).
///
/// Auth gate middleware применяется через `route_layer` — только к этому маршруту.
/// Middleware проверяет Session и возвращает 401 при отсутствии identity
/// ДО передачи управления в `ws_handler` (Pitfall 6 mitigation — T-06-09-E).
pub fn router() -> Router<AppCtx> {
    Router::new()
        .route("/api/v1/ws", any(ws_handler))
        .route_layer(axum::middleware::from_fn(ws_auth_middleware))
}
