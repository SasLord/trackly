//! Fail-soft recovery for `trackly.config.toml` load failures (quick task 260804-lk0).
//!
//! Root cause of the "GUI silently exits with code 1" bug: `main()` used to call
//! `AppConfig::load_or_default(...)?` at step 4, BEFORE `logging::init` (step 5). Any parse
//! error propagated straight out of `fn main() -> anyhow::Result<()>`; under
//! `#[cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` (release builds)
//! there is no console attached, so the printed `Error: ...` was invisible — the app just
//! vanished.
//!
//! Fix: config load never propagates fatally anymore. `load_or_recover` always returns a
//! usable `AppConfig` (real or `AppConfig::default()`), plus an optional human-readable
//! error string. `main.rs` inits logging with whatever config comes back, THEN surfaces the
//! captured error through every channel available: the log file (always), a
//! portable-mode-safe `config-error.txt` next to the exe (always — works headless too), and
//! a best-effort native dialog (desktop/interactive only — `main.rs` skips this for
//! `--self-test` so CI never blocks on a dialog nobody can dismiss).

use std::path::{Path, PathBuf};

use time::format_description::well_known::Rfc3339;
use trackly_infra::{AppConfig, Paths};

/// Loads `trackly.config.toml`, never fatally. On any load/parse error, falls back to
/// `AppConfig::default()` and returns the error's `Display` string as `Some(..)` for the
/// caller to surface. A successful load — including "file absent", which is normal
/// portable-mode operation, not an error — returns `None`.
pub fn load_or_recover(config_path: &Path) -> (AppConfig, Option<String>) {
    match AppConfig::load_or_default(config_path) {
        Ok(cfg) => (cfg, None),
        Err(e) => (AppConfig::default(), Some(e.to_string())),
    }
}

/// Writes `<exe_dir>/config-error.txt` with a timestamped, Russian, human-readable
/// explanation of the failure — the portable-mode-safe fallback that works even when no
/// console/dialog is available (headless / no display server). Best-effort: callers should
/// log (not propagate) an `Err`.
pub fn write_config_error_file(paths: &Paths, message: &str) -> std::io::Result<PathBuf> {
    let path = paths.exe_dir().join("config-error.txt");
    let ts = time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "unknown-time".to_string());
    let contents = format!(
        "Trackly: не удалось загрузить trackly.config.toml — используются настройки по \
         умолчанию.\n\
         Время: {ts}\n\
         Файл: {}\n\
         Ошибка: {message}\n\n\
         Сверьтесь с trackly.config.toml.example рядом с исполняемым файлом, исправьте \
         конфигурацию и перезапустите Trackly.\n",
        paths.config_file().display()
    );
    std::fs::write(&path, contents)?;
    Ok(path)
}

/// Removes a stale `config-error.txt` after a successful config load, so a leftover error
/// file from a previous broken run doesn't confuse the user once they've fixed the config.
/// Best-effort: `NotFound` and any other I/O error are swallowed — there is nothing
/// actionable to do about a failed best-effort cleanup.
pub fn clear_config_error_file(paths: &Paths) {
    let _ = std::fs::remove_file(paths.exe_dir().join("config-error.txt"));
}

/// Best-effort native message dialog surfacing the failure to an interactive desktop user.
/// Runs on a detached OS thread (never blocks startup) and wraps the call in
/// `catch_unwind` — a missing display server / headless environment must never crash
/// startup. Callers MUST NOT invoke this for `--self-test` / CI runs (see `main.rs`) — a
/// blocking dialog nobody can dismiss would hang the process indefinitely.
pub fn show_best_effort_dialog(message: &str) {
    let message = message.to_string();
    std::thread::spawn(move || {
        let _ = std::panic::catch_unwind(|| {
            rfd::MessageDialog::new()
                .set_level(rfd::MessageLevel::Warning)
                .set_title("Trackly — ошибка конфигурации")
                .set_description(format!(
                    "trackly.config.toml не удалось загрузить, используются настройки по \
                     умолчанию.\n\nПодробности записаны в config-error.txt рядом с \
                     trackly.exe.\n\n{message}"
                ))
                .set_buttons(rfd::MessageButtons::Ok)
                .show();
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tempdir_paths() -> (Paths, tempfile::TempDir) {
        let dir = tempfile::TempDir::new().expect("tempdir");
        let paths = Paths::resolve_for_exe_dir(dir.path().to_path_buf()).expect("paths");
        (paths, dir)
    }

    #[test]
    fn load_or_recover_missing_file_returns_defaults_and_no_error() {
        let (paths, _dir) = tempdir_paths();
        let (cfg, err) = load_or_recover(paths.config_file());
        assert!(err.is_none(), "missing file is not an error condition");
        assert_eq!(cfg.server.host, AppConfig::default().server.host);
    }

    #[test]
    fn load_or_recover_malformed_toml_returns_defaults_and_error_message() {
        let (paths, _dir) = tempdir_paths();
        std::fs::write(paths.config_file(), "[server\nenabled = true\n").unwrap();
        let (cfg, err) = load_or_recover(paths.config_file());
        let err = err.expect("malformed TOML must surface an error message");
        assert!(!err.is_empty());
        assert_eq!(
            cfg.server.host,
            AppConfig::default().server.host,
            "must fall back to defaults, not panic or propagate"
        );
    }

    #[test]
    fn write_config_error_file_creates_file_with_message_and_path() {
        let (paths, _dir) = tempdir_paths();
        let written = write_config_error_file(&paths, "boom: unexpected end of input")
            .expect("write must succeed in a writable tempdir");
        assert_eq!(written, paths.exe_dir().join("config-error.txt"));
        let contents = std::fs::read_to_string(&written).unwrap();
        assert!(contents.contains("boom: unexpected end of input"));
        assert!(contents.contains(&paths.config_file().display().to_string()));
    }

    #[test]
    fn clear_config_error_file_removes_existing_file() {
        let (paths, _dir) = tempdir_paths();
        write_config_error_file(&paths, "x").unwrap();
        assert!(paths.exe_dir().join("config-error.txt").exists());
        clear_config_error_file(&paths);
        assert!(!paths.exe_dir().join("config-error.txt").exists());
    }

    #[test]
    fn clear_config_error_file_is_noop_when_absent() {
        let (paths, _dir) = tempdir_paths();
        // Must not panic when there is nothing to remove.
        clear_config_error_file(&paths);
    }
}
