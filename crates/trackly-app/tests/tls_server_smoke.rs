//! Интеграционные тесты TLS utilities.
//!
//! GREEN после Plan 02 Task 2.

use std::time::Duration;

use axum::Router;
use axum::routing::get;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio_util::sync::CancellationToken;
use trackly_app::server::{start_server, tls};

// ---------------------------------------------------------------------------
// fingerprint_is_95_char_colon_hex (D-TLS-01)
// ---------------------------------------------------------------------------

/// SHA-256 fingerprint формат: 32 байта × 2 hex + 31 двоеточие = 95 символов.
#[test]
fn fingerprint_is_95_char_colon_hex() {
    let bundle = tls::generate_self_signed("localhost")
        .expect("generate_self_signed should succeed");

    let fp = &bundle.fingerprint_hex;

    // Length: 32*2 hex chars + 31 colons = 95
    assert_eq!(
        fp.len(),
        95,
        "fingerprint должен быть 95 символов, получили {}: '{fp}'",
        fp.len()
    );

    // Format: each group is 2 uppercase hex chars, separated by colons
    for (i, part) in fp.split(':').enumerate() {
        assert_eq!(
            part.len(),
            2,
            "группа {i} должна быть 2 символа, получили '{part}'"
        );
        assert!(
            part.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_lowercase()),
            "группа {i} должна быть uppercase hex, получили '{part}'"
        );
    }

    // Exactly 32 groups
    let groups: Vec<_> = fp.split(':').collect();
    assert_eq!(groups.len(), 32, "должно быть 32 hex-группы");
}

// ---------------------------------------------------------------------------
// pem_round_trip (D-TLS-02)
// ---------------------------------------------------------------------------

/// Сгенерированный PEM можно загрузить обратно через load_from_pem.
#[test]
fn pem_round_trip() {
    let original = tls::generate_self_signed("test.local")
        .expect("generate_self_signed");

    let loaded = tls::load_from_pem(&original.cert_pem, &original.key_pem)
        .expect("load_from_pem should succeed");

    // Fingerprint should match
    assert_eq!(
        original.fingerprint_hex, loaded.fingerprint_hex,
        "fingerprint должен совпадать при round-trip"
    );
}

// ---------------------------------------------------------------------------
// key_path_resolution_handles_nonstandard_extensions (WR-01 regression)
// ---------------------------------------------------------------------------

/// Regression: the key path must be derived via `Path::with_extension`, which
/// handles ANY cert extension (.crt, .pem, .cer, .cert) — not the old brittle
/// `.replace(".crt", ".key").replace(".pem", ".key")` that left `.cer`/`.cert`
/// paths unchanged (→ silently reading the cert file as the key).
#[test]
fn key_path_resolution_handles_nonstandard_extensions() {
    for (cert, expected_key) in [
        ("/srv/certs/server.crt", "/srv/certs/server.key"),
        ("/srv/certs/server.pem", "/srv/certs/server.key"),
        ("/srv/certs/server.cer", "/srv/certs/server.key"),
        ("/srv/certs/server.cert", "/srv/certs/server.key"),
    ] {
        let resolved = tls::resolve_key_path(cert, "").expect("resolve_key_path");
        assert_eq!(
            resolved, expected_key,
            "cert {cert} should resolve to key {expected_key}, got {resolved}"
        );
        // The exact failure the old `.replace()` heuristic produced for .cer/.cert.
        assert_ne!(
            resolved, cert,
            "resolved key path must never equal the cert path ({cert})"
        );
    }

    // Explicit key_path override wins over extension derivation.
    let explicit = tls::resolve_key_path("/srv/certs/server.cer", "/srv/certs/custom.key")
        .expect("explicit key_path");
    assert_eq!(explicit, "/srv/certs/custom.key");
}

// ---------------------------------------------------------------------------
// load_from_files_resolves_key_for_cer_extension (WR-01 end-to-end)
// ---------------------------------------------------------------------------

/// End-to-end: `load_from_files` loads a cert with a `.cer` extension by
/// resolving its sibling `.key` — exactly the case the main.rs auto-start path
/// (config-driven server.enabled) used to break on. Mirrors the
/// build_server_toggle contract `load_from_files(&cert_path, &key_path)`.
#[test]
fn load_from_files_resolves_key_for_cer_extension() {
    let bundle = tls::generate_self_signed("127.0.0.1").expect("generate_self_signed");

    let dir = tempfile::TempDir::new().expect("tempdir");
    let cert_path = dir.path().join("server.cer");
    let key_path = dir.path().join("server.key");
    std::fs::write(&cert_path, &bundle.cert_pem).expect("write cert");
    std::fs::write(&key_path, &bundle.key_pem).expect("write key");

    // Empty key_path → derived from cert_path via with_extension (.cer → .key).
    let loaded = tls::load_from_files(&cert_path.to_string_lossy(), "")
        .expect("load_from_files should resolve .cer → .key and load");

    assert_eq!(
        loaded.fingerprint_hex, bundle.fingerprint_hex,
        "loaded fingerprint should match the generated cert"
    );
}

// ---------------------------------------------------------------------------
// tls_server_accepts_tcp_connection (D-TLS-03)
// ---------------------------------------------------------------------------

/// TLS сервер принимает TCP соединения — bind проходит и сокет слушает.
/// Тест использует TCP connect без TLS handshake (достаточно для проверки
/// что сервер работает на правильном порту).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn tls_server_accepts_tcp_connection() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let bundle = tls::generate_self_signed("127.0.0.1")
            .expect("generate_self_signed");

        let shutdown = CancellationToken::new();
        let shutdown_clone = shutdown.clone();

        // Pre-bind to get a random port
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind random port");
        let addr = listener.local_addr().expect("local_addr");

        // Start server with pre-bound listener
        let app = Router::new().route("/", get(|| async { "ok" }));
        let server_handle = tokio::spawn(async move {
            start_server(app, listener, bundle.acceptor, shutdown_clone)
                .await
                .expect("start_server");
        });

        // Give server time to enter accept loop
        tokio::time::sleep(Duration::from_millis(50)).await;

        // TCP connect should succeed — server is listening
        let tcp_result = TcpStream::connect(addr).await;
        assert!(tcp_result.is_ok(), "TCP connect должен успешно подключиться к серверу на {addr}");
        drop(tcp_result);

        // Shutdown server
        shutdown.cancel();
        let _ = tokio::time::timeout(Duration::from_secs(5), server_handle).await;
    })
    .await
    .expect("test exceeded 30s budget");
}
