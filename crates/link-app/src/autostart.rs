//! Автозапуск Link (D-17): всегда включён, пункта отключения нет.
//!
//! При старте запись проверяется и при отсутствии восстанавливается (GPO или Defender ASR
//! могут её удалять). Модуль намеренно не содержит операции «выключить». Диспетчер задач
//! Windows может переопределить `StartupApproved`: это следствие решения D-17 (T-44-32).

use std::path::Path;

/// Имя записи автозапуска.
pub const APP_NAME: &str = "Trackly Link";
/// Аргумент автозапуска: стартовать свёрнутым в трей.
pub const MINIMIZED_ARG: &str = "--minimized";

/// Значение записи Run: путь в кавычках (в пути есть пробел «Trackly Link», T-44-30),
/// затем аргументы. Уже заключённый в кавычки путь повторно не оборачивается.
pub fn run_value(exe: &Path, args: &[&str]) -> String {
    let mut out = quoted_path(exe);
    for a in args {
        out.push(' ');
        out.push_str(a);
    }
    out
}

fn quoted_path(exe: &Path) -> String {
    let raw = exe.to_string_lossy();
    let raw = raw.as_ref();
    if raw.len() >= 2 && raw.starts_with('"') && raw.ends_with('"') {
        raw.to_string()
    } else {
        format!("\"{raw}\"")
    }
}

/// Результат проверки записи при старте.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AutostartOutcome {
    /// Запись уже была.
    AlreadyOk,
    /// Записи не было, создана заново.
    Restored,
    /// Не вышло (политика, антивирус): без паники и без повторов в этом запуске.
    Failed(String),
}

/// Хранилище записи автозапуска. Намеренно без `disable` (D-17).
pub trait AutostartBackend {
    fn is_enabled(&self) -> Result<bool, String>;
    fn enable(&self) -> Result<(), String>;
}

/// Проверить запись и восстановить при необходимости. Один вызов `enable` за запуск.
pub fn ensure_enabled(backend: &dyn AutostartBackend) -> AutostartOutcome {
    match backend.is_enabled() {
        Ok(true) => AutostartOutcome::AlreadyOk,
        // Ошибка чтения трактуется как «записи нет»: пробуем записать один раз.
        Ok(false) | Err(_) => match backend.enable() {
            Ok(()) => AutostartOutcome::Restored,
            Err(reason) => {
                tracing::warn!("автозапуск не восстановлен: {reason}");
                AutostartOutcome::Failed(reason)
            }
        },
    }
}

/// Настоящее хранилище на `auto-launch` (реестр Windows, LaunchAgent, XDG autostart).
pub struct AutoLaunchBackend {
    inner: auto_launch::AutoLaunch,
}

impl AutoLaunchBackend {
    /// Собрать для исполняемого файла `exe`.
    pub fn new(exe: &Path) -> Result<Self, String> {
        // На Windows и Linux путь содержит пробел: нужны кавычки (Run / Exec).
        // LaunchAgent на macOS хранит путь отдельным элементом plist: кавычки сломали бы запуск.
        let app_path = if cfg!(target_os = "macos") {
            exe.to_string_lossy().into_owned()
        } else {
            quoted_path(exe)
        };
        let inner = auto_launch::AutoLaunchBuilder::new()
            .set_app_name(APP_NAME)
            .set_app_path(&app_path)
            .set_args(&[MINIMIZED_ARG])
            // Только HKCU (CurrentUser): режим с выбором пробует HKLM и требует прав администратора.
            .set_windows_enable_mode(auto_launch::WindowsEnableMode::CurrentUser)
            .set_macos_launch_mode(auto_launch::MacOSLaunchMode::LaunchAgent)
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self { inner })
    }

    /// Бэкенд для текущего исполняемого файла.
    pub fn for_current_exe() -> Result<Self, String> {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        Self::new(&exe)
    }
}

impl AutostartBackend for AutoLaunchBackend {
    fn is_enabled(&self) -> Result<bool, String> {
        self.inner.is_enabled().map_err(|e| e.to_string())
    }

    fn enable(&self) -> Result<(), String> {
        self.inner.enable().map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    struct Fake {
        enabled: bool,
        enable_fails: bool,
        enable_calls: Cell<u32>,
    }

    impl Fake {
        fn new(enabled: bool, enable_fails: bool) -> Self {
            Self {
                enabled,
                enable_fails,
                enable_calls: Cell::new(0),
            }
        }
    }

    impl AutostartBackend for Fake {
        fn is_enabled(&self) -> Result<bool, String> {
            Ok(self.enabled)
        }
        fn enable(&self) -> Result<(), String> {
            self.enable_calls.set(self.enable_calls.get() + 1);
            if self.enable_fails {
                Err("доступ запрещён политикой".into())
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn run_value_quotes_path_with_space() {
        let exe = Path::new(r"C:\Users\x\AppData\Local\Programs\Trackly Link\trackly-link.exe");
        assert_eq!(
            run_value(exe, &["--minimized"]),
            "\"C:\\Users\\x\\AppData\\Local\\Programs\\Trackly Link\\trackly-link.exe\" --minimized"
        );
    }

    #[test]
    fn run_value_does_not_double_quote() {
        let exe = Path::new("\"C:\\Program Files\\Trackly Link\\trackly-link.exe\"");
        assert_eq!(
            run_value(exe, &["--minimized"]),
            "\"C:\\Program Files\\Trackly Link\\trackly-link.exe\" --minimized"
        );
    }

    #[test]
    fn ensure_enabled_restores_missing_entry() {
        let b = Fake::new(false, false);
        assert_eq!(ensure_enabled(&b), AutostartOutcome::Restored);
        assert_eq!(b.enable_calls.get(), 1);
    }

    #[test]
    fn ensure_enabled_leaves_existing_entry_alone() {
        let b = Fake::new(true, false);
        assert_eq!(ensure_enabled(&b), AutostartOutcome::AlreadyOk);
        assert_eq!(b.enable_calls.get(), 0);
    }

    #[test]
    fn enable_failure_is_reported_once_without_retry() {
        let b = Fake::new(false, true);
        assert!(matches!(ensure_enabled(&b), AutostartOutcome::Failed(_)));
        assert_eq!(b.enable_calls.get(), 1);
    }

    #[test]
    fn module_exposes_no_disable() {
        // D-17: дублирует grep-гейт приёмки по исходнику модуля.
        let src = include_str!("autostart.rs");
        let needle = ["fn ", "disable"].concat();
        let call = [".", "disable", "("].concat();
        let body = src.split("#[cfg(test)]").next().unwrap();
        assert!(!body.contains(&needle));
        assert!(!body.contains(&call));
    }
}
