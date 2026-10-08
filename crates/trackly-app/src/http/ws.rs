//! WebSocket handler — Phase 6 Plan 03.
//!
//! ## Security (T-06-09-E — ASVS V2/V4)
//!
//! Auth gate реализован через `axum::middleware::from_fn_with_state`:
//! middleware проверяет сессию и возвращает 401 ДО передачи запроса в ws_handler.
//! Это гарантирует что WS-соединение (on_upgrade) открывается только для
//! аутентифицированных клиентов (Pitfall 6 из RESEARCH.md).
//!
//! ## Liveness (Pitfall 5)
//!
//! `Lagged(n)` ошибка от `broadcast::Receiver` означает что клиент отстал
//! (dropped events). Обрабатываем через `continue` — НЕ `break` — иначе
//! потеря нескольких событий завершит WS-сессию.
//!
//! ## Остановка серверного режима
//!
//! `handle_ws_socket` обязан сам закрывать сокет по токену `ServerShutdown`:
//! апгрейженный сокет живёт в задаче, которую axum спаунит в `on_upgrade`, и
//! внешняя отмена hyper-соединения до него не доходит. Подробности — в докблоке
//! функции (отладочная сессия `ws-disconnect-toast-no-show`).
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
use std::time::Duration;

use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use tower_sessions::Session;

use crate::context::AppCtx;
use crate::dto::printer::WsEvent;
use crate::http::auth::session_identity;
use crate::server::ServerShutdown;
use trackly_core::auth::Identity;

/// Сколько ждём отправку прощального Close-кадра при остановке сервера.
/// Не «сколько нужно», а «сколько не жалко»: кадр — вежливость, а не условие
/// закрытия. По истечении сокет всё равно дропается.
const CLOSE_SEND_TIMEOUT: Duration = Duration::from_secs(1);

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
    server_shutdown: Option<Extension<ServerShutdown>>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let rx = ctx.ws_broadcast.subscribe();
    let shutdown = server_shutdown.map(|Extension(ServerShutdown(token))| token);
    ws.on_upgrade(move |socket| handle_ws_socket(socket, identity, rx, shutdown))
}

/// WebSocket connection loop.
///
/// Runs a `tokio::select!` on three branches:
///   1. `rx.recv()` — fan-out broadcast events to the connected client.
///   2. `socket.recv()` — detect client disconnect (None / Close / Error).
///   3. `shutdown.cancelled()` — серверный режим останавливают: закрыть сокет.
///
/// Ветвь 3 ОБЯЗАТЕЛЬНА, и вот почему её нельзя заменить ничем снаружи. Задачу с
/// этим сокетом спаунит axum внутри `on_upgrade`, владение IO ей передаёт
/// `hyper::upgrade::on`, поэтому дроп future hyper-соединения в
/// `server::start_server` её НЕ трогает (проверено рантаймом — тест
/// `ws_close_on_server_stop` оставался красным). А сама по себе эта функция не
/// завершится никогда: `socket.recv()` вечно pending (транспорт
/// server-push-only, клиент молчит), а `rx.recv()` отдаст `Closed` только при
/// дропе `broadcast::Sender`, который лежит в `AppCtx.ws_broadcast` и остановку
/// сервера переживает. Итог до фикса: после «Остановить сервер» сокет оставался
/// открытым, браузерный `ws.onclose` не наступал, и тост D-18 «Соединение с
/// сервером потеряно» не показывался НИКОГДА (отладочная сессия
/// `ws-disconnect-toast-no-show`, GAP-1 фазы 41.7).
async fn handle_ws_socket(
    mut socket: WebSocket,
    identity: Identity,
    mut rx: broadcast::Receiver<WsEvent>,
    shutdown: Option<CancellationToken>,
) {
    // `select!` требует future в каждой ветви. Когда токена нет (роутер поднят
    // без `start_server`), подставляем вечно-pending ветвь, чтобы поведение
    // осталось прежним, а не «мгновенно закрыть сокет».
    let shutdown = shutdown.unwrap_or_default();
    loop {
        tokio::select! {
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
                        if socket.send(Message::Text(json.into())).await.is_err() {
                            // Client disconnected or send buffer full.
                            break;
                        }
                    }
                    Ok(_) => {
                        // Event not visible to this identity — silently skip.
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        // Client is slow — skipped n events. Don't break (Pitfall 5).
                        tracing::warn!("ws: client lagged {n} events — continuing");
                        // continue is implicit
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
                    Some(Ok(_)) => {
                        // Ping, Pong, Text from client — ignore (server push only).
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
