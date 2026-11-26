//! Одноразовый стенд живой проверки UI (фаза 42, D-21).
//!
//! Поднимает НАСТОЯЩИЙ `build_router` со встроенным `ui/dist` на 127.0.0.1 (HTTPS,
//! самоподписанный сертификат), создаёт вымышленного администратора и ждёт, пока
//! существует файл-флаг. Нужен, чтобы снять консоль браузера до и после переезда
//! фронтенда. В обычный прогон не попадает (`#[ignore]`).
//!
//! Запуск: `cargo test -p trackly-app --test stand_ui_move -- --ignored --nocapture`
//! Переменные: STAND_PORT (18480), STAND_HOLD_FILE (флаг, по умолчанию в temp_dir),
//! STAND_MAX_SECS (3600).

use std::net::SocketAddr;
use std::time::{Duration, Instant};

use trackly_app::http::build_router;
use trackly_app::server::rusqlite_session_store::RusqliteSessionStore;
use trackly_app::server::{start_server_on_addr, tls};

const STAND_LOGIN: &str = "stand_admin";
const STAND_PASSWORD: &str = "stand-password-123";

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "stand: запускать вручную -- --ignored"]
async fn stand_serves_real_ui() {
    tls::ensure_crypto_provider();

    let dir = tempfile::TempDir::new().expect("tempdir");
    let paths = trackly_infra::Paths::resolve_for_exe_dir(dir.keep()).expect("paths");
    let config = trackly_infra::AppConfig::default();
    let log_guard = trackly_app::logging::init(&paths, &config).unwrap_or_else(|_| {
        let (_nb, guard) = tracing_appender::non_blocking(std::io::sink());
        guard
    });
    let ctx = trackly_app::context::AppCtx::build(paths, config, log_guard)
        .await
        .expect("AppCtx::build");

    ctx.auth
        .create_user(
            trackly_app::dto::auth::UserNew {
                login: STAND_LOGIN.to_string(),
                full_name: "Иванов И.И.".to_string(),
                password: STAND_PASSWORD.to_string(),
                role: "admin".to_string(),
                email: None,
            },
            &trackly_core::auth::Identity::trusted_admin(),
        )
        .await
        .expect("create stand admin");

    let session_store = RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone());
    let router = build_router(&ctx, session_store);

    let port: u16 = std::env::var("STAND_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(18480);
    let addr: SocketAddr = ([127, 0, 0, 1], port).into();
    let bundle = tls::generate_self_signed("127.0.0.1").expect("self-signed");
    let token = ctx.shutdown.child_token();
    let server = tokio::spawn(start_server_on_addr(
        router,
        addr,
        bundle.acceptor,
        token.clone(),
    ));

    let hold = std::env::var("STAND_HOLD_FILE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir().join("trackly-stand-hold"));
    std::fs::write(&hold, b"remove this file to stop the stand").expect("hold file");
    let max_secs: u64 = std::env::var("STAND_MAX_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3600);

    println!(
        "STAND READY: https://127.0.0.1:{port}/  login={STAND_LOGIN} password={STAND_PASSWORD}"
    );
    println!("STAND HOLD FILE: {}", hold.display());

    let started = Instant::now();
    while hold.exists() && started.elapsed() < Duration::from_secs(max_secs) {
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    let _ = std::fs::remove_file(&hold);
    token.cancel();
    ctx.shutdown.cancel();
    let _ = tokio::time::timeout(Duration::from_secs(10), server).await;
    println!("STAND STOPPED");
}
