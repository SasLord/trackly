//! Политика окна Link: что делать при закрытии и нужен ли значок на панели задач.
//!
//! Решения чистые и тестируются на macOS. Реальный трей и панель задач проверяются живьём
//! (план 20). LNK-04: закрытие прячет окно в трей; D-20: без системного трея окно остаётся
//! на панели задач и сворачивается, чтобы Link не потерялся.

use tauri::WebviewWindow;

/// Что делает крестик окна.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseAction {
    /// Спрятать окно в трей (LNK-04).
    Hide,
    /// Свернуть на панель задач (трея нет, D-20).
    Minimize,
}

/// Решение по закрытию окна.
pub fn close_action(tray_available: bool) -> CloseAction {
    if tray_available {
        CloseAction::Hide
    } else {
        CloseAction::Minimize
    }
}

/// Прятать ли окно с панели задач. Без трея окно обязано оставаться видимым в панели.
pub fn skip_taskbar(tray_available: bool) -> bool {
    tray_available
}

/// D-19: повторный запуск всегда показывает окно уже работающего экземпляра.
pub fn should_show_window_on_second_launch() -> bool {
    true
}

/// Ошибка опроса шины сообщений (D-Bus недоступна и т.п.).
#[derive(Debug, thiserror::Error)]
#[error("не удалось опросить шину: {0}")]
pub struct ProbeError(pub String);

/// Проба наличия системного трея (Linux: `org.kde.StatusNotifierWatcher`).
pub trait TrayProbe {
    fn status_notifier_watcher_present(&self) -> Result<bool, ProbeError>;
}

/// Есть ли системный трей. Windows и macOS: всегда да, без пробы. Linux: результат пробы;
/// ошибка шины трактуется как «трея нет» (Pitfall 5), паники нет.
#[cfg_attr(not(target_os = "linux"), allow(unused_variables))]
pub fn detect_tray(probe: &dyn TrayProbe) -> bool {
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    {
        true
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        match probe.status_notifier_watcher_present() {
            Ok(present) => present,
            Err(err) => {
                tracing::warn!("проба трея не удалась, считаем что трея нет: {err}");
                false
            }
        }
    }
}

/// Проба через сессионную шину D-Bus: `NameHasOwner("org.kde.StatusNotifierWatcher")`.
#[cfg(target_os = "linux")]
pub struct LinuxDbusProbe;

#[cfg(target_os = "linux")]
impl TrayProbe for LinuxDbusProbe {
    fn status_notifier_watcher_present(&self) -> Result<bool, ProbeError> {
        let conn = zbus::blocking::Connection::session().map_err(|e| ProbeError(e.to_string()))?;
        let proxy =
            zbus::blocking::fdo::DBusProxy::new(&conn).map_err(|e| ProbeError(e.to_string()))?;
        let name = zbus::names::BusName::try_from("org.kde.StatusNotifierWatcher")
            .map_err(|e| ProbeError(e.to_string()))?;
        proxy
            .name_has_owner(name)
            .map_err(|e| ProbeError(e.to_string()))
    }
}

/// Проба для платформ без D-Bus-пробы: на Windows и macOS не вызывается.
#[cfg(not(target_os = "linux"))]
pub struct NoProbe;

#[cfg(not(target_os = "linux"))]
impl TrayProbe for NoProbe {
    fn status_notifier_watcher_present(&self) -> Result<bool, ProbeError> {
        Ok(true)
    }
}

/// Проба по умолчанию для текущей платформы.
pub fn default_probe() -> Box<dyn TrayProbe> {
    #[cfg(target_os = "linux")]
    {
        Box::new(LinuxDbusProbe)
    }
    #[cfg(not(target_os = "linux"))]
    {
        Box::new(NoProbe)
    }
}

/// Применить политику к окну: значок на панели задач по `skip_taskbar`.
/// Вызов подключает план 17.
pub fn apply_taskbar_policy(window: &WebviewWindow, tray_available: bool) -> tauri::Result<()> {
    window.set_skip_taskbar(skip_taskbar(tray_available))
}

/// Обработчик закрытия: отменить закрытие и спрятать/свернуть окно (LNK-04, D-20).
/// Единственный выход из Link: пункт «Закрыть» меню трея. Вызов подключает план 17.
pub fn apply_close_policy(
    window: &WebviewWindow,
    api: &tauri::CloseRequestApi,
    tray_available: bool,
) -> tauri::Result<()> {
    api.prevent_close();
    match close_action(tray_available) {
        CloseAction::Hide => window.hide(),
        CloseAction::Minimize => window.minimize(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeProbe(Result<bool, &'static str>);
    impl TrayProbe for FakeProbe {
        fn status_notifier_watcher_present(&self) -> Result<bool, ProbeError> {
            self.0.map_err(|e| ProbeError(e.to_string()))
        }
    }

    #[test]
    fn close_action_hide_with_tray_minimize_without() {
        assert_eq!(close_action(true), CloseAction::Hide);
        assert_eq!(close_action(false), CloseAction::Minimize);
    }

    #[test]
    fn skip_taskbar_only_with_tray() {
        assert!(skip_taskbar(true));
        assert!(!skip_taskbar(false));
    }

    #[test]
    fn detect_tray_follows_probe_on_linux_and_is_true_elsewhere() {
        let yes = detect_tray(&FakeProbe(Ok(true)));
        let no = detect_tray(&FakeProbe(Ok(false)));
        assert!(yes);
        if cfg!(target_os = "linux") {
            assert!(!no);
        } else {
            assert!(no, "Windows и macOS: трей всегда есть");
        }
    }

    #[test]
    fn probe_error_means_no_tray_on_linux_without_panic() {
        let got = detect_tray(&FakeProbe(Err("нет шины")));
        if cfg!(target_os = "linux") {
            assert!(!got);
        } else {
            assert!(got);
        }
    }

    #[test]
    fn second_launch_always_shows_window() {
        assert!(should_show_window_on_second_launch());
    }
}
