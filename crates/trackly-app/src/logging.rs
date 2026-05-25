//! `tracing-subscriber` + `tracing-appender` daily rotation (D-Logging-01).
//!
//! Контракт:
//! - Файл-аппендер: `tracing_appender::rolling::daily(paths.logs_dir(),
//!   "trackly.log")` → файлы вида `trackly.log.<YYYY-MM-DD>` рядом с .exe
//!   (portable-mode invariant — никакой утечки в `%APPDATA%` или `~/Library/Logs`).
//! - Stdout-аппендер: всегда `compact`-format в stderr (для systemd-style
//!   capture).
//! - File-аппендер: `compact` ИЛИ `json` по `config.logging.format`.
//! - `with_ansi(false)` на file-layer — никаких ANSI escape-кодов в .log.
//! - Уровень: env `TRACKLY_LOG` (full `EnvFilter` синтаксис), fallback на
//!   `config.logging.level` + жёсткие `hyper=warn,tower_http=warn` (HTTP-шум).
//! - Возвращает `WorkerGuard` — caller (AppCtx) ОБЯЗАН его держать живым
//!   до конца процесса; иначе background-thread аппендера остановится
//!   и буферизованные сообщения потеряются (Pitfall #6).
//!
//! `.try_init()` (НЕ `.init()`) чтобы тесты не падали на double-init,
//! когда integration-тест вызывает `init` повторно после своего `init`.
//!
//! Secret-leak guard test (`secret_leak_guard_*` ниже) ходит полным путём
//! tracing → file → re-read и проверяет, что `Secret<T>::Debug` ("***")
//! действительно появляется в .log, а оригинальное значение — нет
//! (T-05-03 в threat model плана 05).

use std::fs;

use anyhow::{Context, Result};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, prelude::*, EnvFilter, Registry};

use trackly_infra::{AppConfig, Paths};

/// Инициализирует глобальный `tracing` subscriber. Возвращает `WorkerGuard`
/// для non-blocking file-аппендера — caller (AppCtx) держит его живым до
/// конца процесса.
///
/// Двойной вызов из тестов — no-op (silent: `try_init` возвращает Err, мы
/// игнорируем). Это нужно потому что `cargo test` параллелит тесты в одном
/// процессе, и второй вызов `init` иначе бы паниковал.
pub fn init(paths: &Paths, config: &AppConfig) -> Result<WorkerGuard> {
    // Logs dir создаём ДО открытия аппендера — иначе rolling-аппендер падает
    // на первом write'е.
    fs::create_dir_all(paths.logs_dir())
        .with_context(|| format!("create logs dir {}", paths.logs_dir().display()))?;

    let file_appender = tracing_appender::rolling::daily(paths.logs_dir(), "trackly.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // EnvFilter: TRACKLY_LOG (full syntax) overrides config.logging.level.
    // Fallback: <config_level>,hyper=warn,tower_http=warn чтобы HTTP-фреймворки
    // не зашумляли info-логи.
    let env_filter = EnvFilter::try_from_env("TRACKLY_LOG").unwrap_or_else(|_| {
        EnvFilter::new(format!(
            "{},hyper=warn,tower_http=warn",
            config.logging.level
        ))
    });

    // try_init: возвращает Err если subscriber уже глобально установлен
    // (двойной вызов в тестах). В проде main() вызывает один раз — ошибки
    // не будет; в тестах вторые вызовы тихо игнорируются.
    //
    // Каждая ветка делает `.try_init()` отдельно — типы Layered<...>
    // расходятся между json/compact вариантами и нельзя сохранить их в
    // одной let-binding'е.
    let init_err: Option<String> = if config.logging.format == "json" {
        let file_layer = fmt::layer()
            .json()
            .with_writer(non_blocking)
            .with_ansi(false);
        let stdout_layer = fmt::layer().compact().with_writer(std::io::stderr);
        Registry::default()
            .with(env_filter)
            .with(file_layer)
            .with(stdout_layer)
            .try_init()
            .err()
            .map(|e| e.to_string())
    } else {
        let file_layer = fmt::layer()
            .compact()
            .with_writer(non_blocking)
            .with_ansi(false);
        let stdout_layer = fmt::layer().compact().with_writer(std::io::stderr);
        Registry::default()
            .with(env_filter)
            .with(file_layer)
            .with(stdout_layer)
            .try_init()
            .err()
            .map(|e| e.to_string())
    };

    // Ignore double-init Err (`SetGlobalDefaultError`) — это нормально в тестах.
    // В проде main() гарантирует один вызов; первый успех держит subscriber.
    if let Some(e) = init_err {
        eprintln!("logging::init: subscriber already set (likely test re-init): {e}");
    }

    Ok(guard)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tempfile::TempDir;

    fn tempdir_paths() -> (Paths, TempDir) {
        let dir = TempDir::new().expect("tempdir");
        let paths = Paths::resolve_for_exe_dir(dir.path().to_path_buf()).expect("paths");
        (paths, dir)
    }

    /// Найти trackly.log.* файл в logs_dir (имя зависит от today's date).
    fn find_log_file(paths: &Paths) -> Option<std::path::PathBuf> {
        std::fs::read_dir(paths.logs_dir())
            .ok()?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .find(|p| {
                p.file_name()
                    .and_then(|s| s.to_str())
                    .is_some_and(|s| s.starts_with("trackly.log"))
            })
    }

    #[test]
    fn init_creates_logs_dir_and_returns_guard() {
        let (paths, _dir) = tempdir_paths();
        let config = AppConfig::default();
        assert!(!paths.logs_dir().exists(), "precondition: logs dir absent");
        let _guard = init(&paths, &config).expect("init");
        assert!(paths.logs_dir().exists(), "logs dir created by init");
    }

    #[test]
    fn init_writes_to_daily_rolling_file() {
        let (paths, _dir) = tempdir_paths();
        let config = AppConfig::default();
        let guard = init(&paths, &config).expect("init");
        // Излучить событие. Note: первый init в test-binary установит
        // глобальный subscriber, последующие init'ы — no-op (т.к. в одном
        // test-process subscriber только один). Мы зависим от того, что
        // ТЕКУЩИЙ init выиграл гонку. Если у нас параллельные тесты,
        // событие может уйти в чужой sink — поэтому ассертим best-effort:
        // только что файл создан (а не его содержимое).
        tracing::info!(target: "trackly_app::logging::tests", "test event");
        drop(guard);
        std::thread::sleep(Duration::from_millis(50)); // flush
        let log = find_log_file(&paths).expect("daily rolling log file created");
        assert!(log.exists(), "log file at {} exists", log.display());
    }

    #[test]
    fn secret_leak_guard_compile_time_no_serialize() {
        // Compile-time гарантия в trackly-core/tests/secret_zeroize.rs
        // (`assert_not_impl_all!(Secret<String>: serde::Serialize)`).
        // Здесь подтверждаем runtime-форму: Secret::Debug = "***", оригинал
        // не виден. Это копирует логику из trackly-core, но эта проверка
        // ВАЖНА в контексте logging.rs — Plan 05 threat T-05-03 требует
        // явного gate перед любой реализацией tracing.
        use trackly_core::primitives::Secret;
        let s = Secret::new(String::from("hunter2"));
        let dbg = format!("{s:?}");
        assert_eq!(dbg, "***");
        assert!(!dbg.contains("hunter2"));
    }

    #[test]
    fn secret_through_tracing_writes_redacted_marker_to_file() {
        // End-to-end T-05-03 mitigation: эмитим Secret через tracing с
        // ?-форматтером (Debug), читаем .log файл, ассертим "***" есть и
        // "hunter2" — нет.
        //
        // Замечание: тесты cargo-test работают в одном processе, и
        // глобальный subscriber устанавливается только первый раз. Для
        // надёжности используем `tracing::subscriber::with_default` —
        // scoped subscriber, который не зависит от глобального.
        use tracing_subscriber::{layer::SubscriberExt, Registry};
        use trackly_core::primitives::Secret;

        let (paths, _dir) = tempdir_paths();
        fs::create_dir_all(paths.logs_dir()).expect("logs dir");
        let file_appender = tracing_appender::rolling::daily(paths.logs_dir(), "trackly.log");
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
        let subscriber = Registry::default().with(
            fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)
                .with_target(false),
        );

        let secret = Secret::new(String::from("hunter2"));
        tracing::subscriber::with_default(subscriber, || {
            tracing::info!(secret = ?secret, "log line with secret");
        });
        drop(guard); // flush non-blocking aппендер.
        std::thread::sleep(Duration::from_millis(100));

        let log = find_log_file(&paths).expect("log file present");
        let contents = std::fs::read_to_string(&log).expect("read log");
        assert!(
            contents.contains("***"),
            "log file should contain redaction marker '***', got: {contents}"
        );
        assert!(
            !contents.contains("hunter2"),
            "log file LEAKED secret 'hunter2': {contents}"
        );
    }
}
