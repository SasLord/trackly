//! Серверный режим — lifecycle управление axum HTTP/HTTPS сервером.
//!
//! ## Архитектура
//!
//! Горячий старт/стоп (D-Server-01) реализован через дочерний `CancellationToken`
//! (никогда не отменяет мастер `AppCtx.shutdown`):
//!
//! ```text
//! AppCtx.shutdown (master)
//!   └── server token (child, per-run)
//!         └── start_server() listen loop
//! ```
//!
//! `ServerHandle` хранит cancel-token и JoinHandle для управления lifecycle.
//!
//! ## TLS
//!
//! Использует `tokio-rustls::TlsAcceptor` поверх `TcpListener::bind`.
//! axum 0.8 не поддерживает TLS напрямую через `axum::serve` — используем
//! ручной accept-loop + `hyper::server::conn::http1`.
//!
//! ## Submodules
//!
//! - [`tls`] — генерация self-signed, загрузка из PEM, fingerprint
//! - [`rusqlite_session_store`] — tower-sessions SessionStore impl

pub mod rusqlite_session_store;
pub mod tls;

use axum::Router;
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use tokio_util::sync::CancellationToken;
use tower::ServiceExt;

/// Handle для управления жизненным циклом запущенного сервера.
///
/// Хранит cancel-token (дочерний к AppCtx.shutdown) и JoinHandle.
pub struct ServerHandle {
    /// Дочерний CancellationToken для этого инстанса сервера.
    /// `cancel()` останавливает сервер, не трогая мастер-shutdown.
    pub cancel: CancellationToken,
    /// JoinHandle фоновой задачи сервера.
    pub task: tokio::task::JoinHandle<()>,
}

/// Запустить HTTPS сервер.
///
/// Принимает уже забинженный `TcpListener` (caller отвечает за bind и получение
/// локального адреса до вызова этой функции), принимает TLS соединения через
/// `tls_acceptor`, раздаёт их hyper HTTP/1.1 сервис из `app` (axum Router).
///
/// Завершается при `shutdown.cancelled()` — цикл accept прерывается, функция
/// возвращает `Ok(())`. JoinHandle в `ServerHandle` завершится следом.
///
/// Новое соединение спаунится как независимая tokio-задача — shutdown не
/// дожидается уже запущенных connections (graceful drain не нужен на LAN-scale).
///
/// # Convenience
///
/// Для стандартного сценария используй [`start_server_on_addr`] — он сам
/// биндит `SocketAddr` и вызывает эту функцию.
pub async fn start_server(
    app: Router,
    listener: TcpListener,
    tls_acceptor: TlsAcceptor,
    shutdown: CancellationToken,
) -> anyhow::Result<()> {
    let addr = listener.local_addr()?;
    tracing::info!("HTTPS server listening on {addr}");

    loop {
        tokio::select! {
            biased;

            _ = shutdown.cancelled() => {
                tracing::info!("server shutdown signal received, stopping accept loop");
                break;
            }

            result = listener.accept() => {
                match result {
                    Ok((stream, peer_addr)) => {
                        let tls = tls_acceptor.clone();
                        let app_clone = app.clone();
                        tokio::spawn(async move {
                            match tls.accept(stream).await {
                                Ok(tls_stream) => {
                                    let io = TokioIo::new(tls_stream);
                                    // Use ServiceExt::oneshot to consume Router per-request
                                    let hyper_service = hyper::service::service_fn(move |req| {
                                        // Clone Router for each request — Router is cheap Clone
                                        app_clone.clone().oneshot(req)
                                    });
                                    if let Err(e) = hyper::server::conn::http1::Builder::new()
                                        .serve_connection(io, hyper_service)
                                        .await
                                    {
                                        tracing::debug!("HTTP connection error from {peer_addr}: {e}");
                                    }
                                }
                                Err(e) => {
                                    tracing::warn!("TLS accept error from {peer_addr}: {e}");
                                }
                            }
                        });
                    }
                    Err(e) => {
                        tracing::error!("TCP accept error: {e}");
                    }
                }
            }
        }
    }

    tracing::info!("HTTPS server stopped");
    Ok(())
}

/// Вспомогательная функция: биндит `addr`, затем запускает [`start_server`].
///
/// Удобно для продакшена когда адрес известен заранее и получать `local_addr`
/// не нужно. В тестах предпочтительнее использовать `start_server` напрямую
/// с предварительно забинженным `TcpListener::bind("127.0.0.1:0")`.
pub async fn start_server_on_addr(
    app: Router,
    addr: SocketAddr,
    tls_acceptor: TlsAcceptor,
    shutdown: CancellationToken,
) -> anyhow::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    start_server(app, listener, tls_acceptor, shutdown).await
}
