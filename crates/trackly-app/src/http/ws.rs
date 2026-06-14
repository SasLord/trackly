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
use tokio::sync::broadcast;
use tower_sessions::Session;

use crate::context::AppCtx;
use crate::dto::printer::WsEvent;
use crate::http::auth::session_identity;
use trackly_core::auth::Identity;

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
pub async fn ws_auth_middleware(
    session: Session,
    mut req: Request,
    next: Next,
) -> Response {
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
pub async fn ws_handler(
    State(ctx): State<AppCtx>,
    Extension(identity): Extension<Identity>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let rx = ctx.ws_broadcast.subscribe();
    ws.on_upgrade(move |socket| handle_ws_socket(socket, identity, rx))
}

/// WebSocket connection loop.
///
/// Runs a `tokio::select!` on two branches:
///   1. `rx.recv()` — fan-out broadcast events to the connected client.
///   2. `socket.recv()` — detect client disconnect (None / Close / Error).
async fn handle_ws_socket(
    mut socket: WebSocket,
    identity: Identity,
    mut rx: broadcast::Receiver<WsEvent>,
) {
    loop {
        tokio::select! {
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
