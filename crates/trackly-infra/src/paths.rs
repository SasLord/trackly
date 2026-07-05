//! `Paths` — portable-mode path discipline.
//!
//! ВСЁ корнится на `std::env::current_exe()?.parent()?`. БД, конфиг,
//! `data/webview` (для WEBVIEW2_USER_DATA_FOLDER) и `logs/` лежат рядом
//! с .exe. Никаких `dirs::*_dir()` — это запрещено через `clippy.toml`
//! (`disallowed-methods`).
//!
//! Portable detection — sentinel-based: наличие `portable.txt` ИЛИ
//! `trackly.config.toml` рядом с .exe (D-Config-01, ARCHITECTURE.md).
//! Writability-probe не используем — sentinel-based стабильнее и
//! предсказуемее.
//!
//! Windows-only: UNC / SMB пути (`\\server\share\...`) отвергаются с
//! `AppError::Validation` — SQLite WAL не работает на сетевых шарах
//! (Security V8 control; RESEARCH §Common Pitfalls).

use std::path::{Path, PathBuf};

use trackly_core::error::AppError;

/// Все пути, относительно которых работает приложение в portable mode.
#[derive(Debug, Clone)]
pub struct Paths {
    exe_dir: PathBuf,
    db_path: PathBuf,
    config_file: PathBuf,
    webview_data_dir: PathBuf,
    logs_dir: PathBuf,
    templates_dir: PathBuf,
    is_portable: bool,
}

impl Paths {
    /// Резолвит пути относительно `std::env::current_exe()?.parent()?`.
    ///
    /// Возвращает `AppError::Internal`, если `current_exe()` фейлится или
    /// родительская директория не существует. `AppError::Validation` —
    /// при UNC/SMB пути на Windows.
    pub fn resolve() -> Result<Self, AppError> {
        let exe = std::env::current_exe().map_err(|e| AppError::Internal {
            source_chain: format!("current_exe failed: {e}"),
        })?;
        let exe_dir = exe
            .parent()
            .ok_or_else(|| AppError::Internal {
                source_chain: format!("current_exe has no parent dir (got {})", exe.display()),
            })?
            .to_path_buf();
        Self::resolve_for_exe_dir(exe_dir)
    }

    /// Тестовый seam: задаёт exe_dir вручную, минуя `current_exe()`.
    /// Используется в `tests/paths_test.rs`.
    pub fn resolve_for_exe_dir(exe_dir: PathBuf) -> Result<Self, AppError> {
        // V8 control: отвергаем UNC / SMB пути на Windows. SQLite WAL не работает
        // на сетевых шарах — silent corruption через несколько часов.
        #[cfg(windows)]
        {
            let s = exe_dir.to_string_lossy();
            if s.starts_with(r"\\") {
                return Err(AppError::Validation {
                    field: "exe_dir".to_string(),
                    message: format!(
                        "UNC/SMB path rejected: SQLite WAL does not support network shares (got {s})"
                    ),
                });
            }
        }

        let db_path = exe_dir.join("trackly.db");
        let config_file = exe_dir.join("trackly.config.toml");
        let webview_data_dir = exe_dir.join("data").join("webview");
        let logs_dir = exe_dir.join("logs");
        let templates_dir = exe_dir.join("templates");

        // Sentinel rule (D-Config-01): portable.txt ИЛИ trackly.config.toml.
        let is_portable = exe_dir.join("portable.txt").exists() || config_file.exists();

        Ok(Self {
            exe_dir,
            db_path,
            config_file,
            webview_data_dir,
            logs_dir,
            templates_dir,
            is_portable,
        })
    }

    /// Корневая директория, где лежит исполняемый файл.
    pub fn exe_dir(&self) -> &Path {
        &self.exe_dir
    }

    /// Путь к SQLite-файлу (по умолчанию `<exe_dir>/trackly.db`; может быть
    /// переопределён через `[paths].db_path` в `trackly.config.toml` — это
    /// делает caller, не сам `Paths`).
    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    /// Путь к `trackly.config.toml` (sentinel-кандидат и источник конфига).
    pub fn config_file(&self) -> &Path {
        &self.config_file
    }

    /// Директория для WebView2 user data — `<exe_dir>/data/webview`.
    /// Передаётся в `WEBVIEW2_USER_DATA_FOLDER` ДО любого вызова Tauri
    /// (FOUND-05, Pitfall #1).
    pub fn webview_data_dir(&self) -> &Path {
        &self.webview_data_dir
    }

    /// Директория для логов — `<exe_dir>/logs`. Реальное создание —
    /// в Plan 05 (`tracing_appender::rolling::daily`).
    pub fn logs_dir(&self) -> &Path {
        &self.logs_dir
    }

    /// Директория для шаблонов документов — `<exe_dir>/templates`.
    /// Читается `pdf::html_templates` (Phase 16); может быть переопределена
    /// через `TRACKLY_TEMPLATES_DIR` — это делает caller, не сам `Paths`,
    /// см. D-07.
    pub fn templates_dir(&self) -> &Path {
        &self.templates_dir
    }

    /// True, если рядом с .exe лежит `portable.txt` или `trackly.config.toml`.
    pub fn is_portable(&self) -> bool {
        self.is_portable
    }
}
