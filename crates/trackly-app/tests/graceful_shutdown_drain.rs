//! Интеграционные тесты graceful shutdown сервера.
//!
//! GREEN после Plan 02 Task 2.

use std::time::Duration;

use axum::Router;
use axum::routing::get;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use trackly_app::server::start_server;
use trackly_app::server::tls;

// ---------------------------------------------------------------------------
// graceful_shutdown_exits_within_timeout (D-Server-03)
// ---------------------------------------------------------------------------

/// При вызове `shutdown.cancel()` accept-loop должен завершиться в течение 5с.
/// Тест проверяет что сервер не зависает после отмены токена.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn graceful_shutdown_exits_within_timeout() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let bundle = tls::generate_self_signed("127.0.0.1").expect("tls bundle");
        let shutdown = CancellationToken::new();
        let shutdown_clone = shutdown.clone();

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind port");

        let app = Router::new().route("/", get(|| async { "ok" }));
        let server_task = tokio::spawn(async move {
            start_server(app, listener, bundle.acceptor, shutdown_clone)
                .await
                .expect("start_server")
        });

        // Allow server to enter accept loop
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Cancel shutdown token
        let cancel_at = std::time::Instant::now();
        shutdown.cancel();

        // Server must finish within 5 seconds
        let result = tokio::time::timeout(Duration::from_secs(5), server_task).await;

        let elapsed = cancel_at.elapsed();
        assert!(
            result.is_ok(),
            "server должен завершиться в течение 5с после shutdown.cancel(), прошло {elapsed:?}"
        );

        // Server task should not have panicked
        result
            .unwrap()
            .expect("server task не должен паниковать");

        tracing::info!("shutdown завершился за {elapsed:?}");
    })
    .await
    .expect("test exceeded 30s budget");
}

// ---------------------------------------------------------------------------
// shutdown_before_server_starts_is_noop (D-Server-04)
// ---------------------------------------------------------------------------

/// Если shutdown уже отменён до вызова start_server, сервер должен
/// немедленно завершиться без accept-loop.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn shutdown_before_server_starts_is_noop() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let bundle = tls::generate_self_signed("127.0.0.1").expect("tls bundle");
        let shutdown = CancellationToken::new();

        // Pre-cancel before spawning
        shutdown.cancel();

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind port");

        let app = Router::new().route("/", get(|| async { "ok" }));
        let server_task = tokio::spawn({
            let s = shutdown.clone();
            async move { start_server(app, listener, bundle.acceptor, s).await }
        });

        // Server should exit very quickly (biased select checks shutdown first)
        let result = tokio::time::timeout(Duration::from_secs(3), server_task).await;
        assert!(
            result.is_ok(),
            "сервер с pre-cancelled token должен немедленно завершиться"
        );
        result
            .unwrap()
            .expect("no panic")
            .expect("no error");
    })
    .await
    .expect("test exceeded 10s budget");
}
