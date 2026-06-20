//! Regression coverage for Bug A (debug session ui-ws-toast-reports-flicker):
//! the manual hyper accept-loop in `crates/trackly-app/src/server/mod.rs` drove
//! connections with `http1::Builder::new().serve_connection(io, svc)` WITHOUT
//! `.with_upgrades()`. axum's `WebSocketUpgrade` emits a 101 response and then
//! awaits `hyper::upgrade::on(req)` inside `on_upgrade`; that upgrade future only
//! resolves when the hyper connection is driven with `.with_upgrades()`. Without
//! it, hyper writes the 101, completes the connection, and the socket is closed
//! ~1s later — the browser observed "101 Switching Protocols" then "network
//! connection was lost" on a reconnect loop, and `handle_ws_socket` never ran.
//!
//! These tests replicate the EXACT plumbing from `server/mod.rs` (service_fn +
//! `serve_connection`) over plain TCP — TLS is orthogonal to the upgrade
//! mechanism — with a minimal axum WS echo router. They prove:
//!   1. WITHOUT `.with_upgrades()`: the upgrade future never resolves; the WS
//!      client never completes a round-trip (the negative control / repro).
//!   2. WITH `.with_upgrades()`: the WS upgrades, stays open, and round-trips a
//!      message — the fix.

use std::time::Duration;

use axum::{
    extract::ws::{Message as AxumMessage, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    routing::any,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use hyper_util::rt::TokioIo;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::Message as TMessage;
use tower::ServiceExt;

/// Minimal echo WS handler — mirrors the shape of the real `ws_handler`:
/// `WebSocketUpgrade` extractor + `on_upgrade` callback.
async fn echo_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_echo_socket)
}

async fn handle_echo_socket(mut socket: WebSocket) {
    while let Some(Ok(msg)) = socket.recv().await {
        if let AxumMessage::Text(t) = msg {
            // Echo back so the client can confirm the upgrade truly completed
            // server-side (proves on_upgrade resolved, not just that 101 was sent).
            if socket
                .send(AxumMessage::Text(format!("echo:{t}").into()))
                .await
                .is_err()
            {
                break;
            }
        }
    }
}

/// Spawn the accept loop using the SAME pattern as `server/mod.rs`. `with_upgrades`
/// toggles the only difference under test.
async fn spawn_server(with_upgrades: bool) -> std::net::SocketAddr {
    let app: Router = Router::new().route("/api/v1/ws", any(echo_handler));
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("local_addr");

    tokio::spawn(async move {
        loop {
            let (stream, peer_addr) = match listener.accept().await {
                Ok(pair) => pair,
                Err(_) => break,
            };
            let app_clone = app.clone();
            tokio::spawn(async move {
                let io = TokioIo::new(stream);
                let hyper_service = hyper::service::service_fn(move |mut req| {
                    req.extensions_mut()
                        .insert(axum::extract::ConnectInfo(peer_addr));
                    app_clone.clone().oneshot(req)
                });
                let conn = hyper::server::conn::http1::Builder::new()
                    .serve_connection(io, hyper_service);
                if with_upgrades {
                    let _ = conn.with_upgrades().await;
                } else {
                    let _ = conn.await;
                }
            });
        }
    });

    addr
}

/// Connect a real WS client and attempt one text round-trip within a timeout.
/// Returns Ok(echoed_string) on success.
async fn try_ws_roundtrip(addr: std::net::SocketAddr) -> anyhow::Result<String> {
    let url = format!("ws://{addr}/api/v1/ws");
    // Use a raw TcpStream-based client so we don't depend on connector config.
    let tcp = TcpStream::connect(addr).await?;
    let (mut ws_stream, _resp) =
        tokio_tungstenite::client_async(url, tcp).await?;

    ws_stream.send(TMessage::Text("ping".into())).await?;

    // The decisive assertion: a reply only arrives if the server-side upgrade
    // future resolved and handle_echo_socket is actually running.
    let reply = tokio::time::timeout(Duration::from_secs(3), ws_stream.next())
        .await
        .map_err(|_| anyhow::anyhow!("timed out waiting for echo — socket never carried data"))?
        .ok_or_else(|| anyhow::anyhow!("stream closed before any echo (socket torn down post-101)"))??;

    match reply {
        TMessage::Text(t) => Ok(t.to_string()),
        other => anyhow::bail!("unexpected frame: {other:?}"),
    }
}

/// FIX: with `.with_upgrades()`, the WS upgrades and round-trips.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ws_upgrade_succeeds_with_upgrades() {
    let addr = spawn_server(/* with_upgrades = */ true).await;
    let result = try_ws_roundtrip(addr).await;
    assert!(
        result.is_ok(),
        "with .with_upgrades(), the WS must upgrade and echo; got {result:?}"
    );
    assert_eq!(result.unwrap(), "echo:ping");
}

/// NEGATIVE CONTROL / REPRO: without `.with_upgrades()`, the upgrade future never
/// resolves — the client either fails to complete the handshake or never receives
/// data before the socket is dropped. This is the exact bug the fix addresses.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ws_upgrade_fails_without_upgrades() {
    let addr = spawn_server(/* with_upgrades = */ false).await;
    let result = try_ws_roundtrip(addr).await;
    assert!(
        result.is_err(),
        "without .with_upgrades(), the WS must NOT complete a round-trip \
         (this is the reproduced bug); unexpectedly got {result:?}"
    );
}
