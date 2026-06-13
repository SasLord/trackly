//! Интеграционные тесты горячего старта/стопа сервера.
//!
//! GREEN после Plan 02 Task 2.

use std::net::SocketAddr;
use std::time::Duration;

use axum::Router;
use axum::routing::get;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio_util::sync::CancellationToken;
use trackly_app::server::start_server;
use trackly_app::server::tls;

// ---------------------------------------------------------------------------
// server_starts_stops_port_freed (D-Server-01)
// ---------------------------------------------------------------------------

/// Сервер стартует, принимает TCP соединение, останавливается через
/// CancellationToken, и порт освобождается — новый bind на тот же адрес
/// должен пройти успешно.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn server_starts_stops_port_freed() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let bundle = tls::generate_self_signed("127.0.0.1").expect("tls bundle");

        let shutdown = CancellationToken::new();
        let shutdown_clone = shutdown.clone();

        // Pre-bind to get a random port
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind port");
        let addr: SocketAddr = listener.local_addr().expect("local_addr");

        let app = Router::new().route("/", get(|| async { "ok" }));
        let server_handle = tokio::spawn(async move {
            start_server(app, listener, bundle.acceptor, shutdown_clone)
                .await
                .expect("start_server");
        });

        // Allow server to enter accept loop
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Verify server is listening: TCP connect succeeds
        TcpStream::connect(addr).await.expect("should connect while server is up");

        // Stop server
        shutdown.cancel();
        tokio::time::timeout(Duration::from_secs(5), server_handle)
            .await
            .expect("server should finish within 5s")
            .expect("server task should not panic");

        // After stop: port should be freed — rebind on the same addr must succeed
        tokio::time::sleep(Duration::from_millis(50)).await;
        let rebind = TcpListener::bind(addr).await;
        assert!(
            rebind.is_ok(),
            "порт {addr} должен освободиться после остановки сервера, ошибка: {:?}",
            rebind.err()
        );
    })
    .await
    .expect("test exceeded 30s budget");
}

// ---------------------------------------------------------------------------
// server_hot_toggle (D-Server-02)
// ---------------------------------------------------------------------------

/// Горячий toggle: стартовать сервер, остановить, снова стартовать на том же
/// порту. Оба запуска должны принимать соединения.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn server_hot_toggle() {
    tokio::time::timeout(Duration::from_secs(60), async {
        // First run
        let bundle1 = tls::generate_self_signed("127.0.0.1").expect("tls bundle 1");
        let shutdown1 = CancellationToken::new();
        let listener1 = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind port 1");
        let addr = listener1.local_addr().expect("local_addr");

        let app = Router::new().route("/", get(|| async { "ok" }));
        let h1 = tokio::spawn({
            let s = shutdown1.clone();
            async move { start_server(app.clone(), listener1, bundle1.acceptor, s).await }
        });
        tokio::time::sleep(Duration::from_millis(50)).await;
        TcpStream::connect(addr).await.expect("first run: should connect");

        // Stop first run
        shutdown1.cancel();
        tokio::time::timeout(Duration::from_secs(5), h1)
            .await
            .expect("first run: server must finish")
            .expect("no panic")
            .expect("no error");

        // Port freed — small delay for OS to release
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Second run on the same port
        let bundle2 = tls::generate_self_signed("127.0.0.1").expect("tls bundle 2");
        let shutdown2 = CancellationToken::new();
        let listener2 = TcpListener::bind(addr)
            .await
            .expect("rebind port for second run");

        let app2 = Router::new().route("/", get(|| async { "ok2" }));
        let h2 = tokio::spawn({
            let s = shutdown2.clone();
            async move { start_server(app2, listener2, bundle2.acceptor, s).await }
        });
        tokio::time::sleep(Duration::from_millis(50)).await;
        TcpStream::connect(addr)
            .await
            .expect("second run: should connect after toggle");

        // Cleanup
        shutdown2.cancel();
        let _ = tokio::time::timeout(Duration::from_secs(5), h2).await;
    })
    .await
    .expect("test exceeded 60s budget");
}
