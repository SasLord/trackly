//! Desktop OS notification with a click handler — Phase 41.5 gap R2-1.
//!
//! Findings: tauri-plugin-notification 2.4.0 shows the toast via notify-rust and drops
//! the `NotificationHandle`, so a click is lost; its JS API has no desktop click event.
//! On Windows we therefore build the toast ourselves with tauri-winrt-notification
//! (`Toast::on_activated`): a click focuses the `main` window and emits
//! `trackly-notify-activated` with a numeric token to the webview. The AUMID choice mirrors
//! the plugin (identifier, or POWERSHELL_APP_ID when running from target/debug|release).
//! Portable: NO registry writes, NO AUMID registration, NO shortcuts, NO app-data dirs.
//! On other platforms the command returns `false` (the frontend falls back to the plugin).
//! Notification text is never logged.

use tauri::{Emitter, Manager};
use trackly_core::error::AppError;

/// Event delivered to the webview when the user clicks the OS notification.
#[cfg_attr(not(windows), allow(dead_code))]
pub const NOTIFY_ACTIVATED_EVENT: &str = "trackly-notify-activated";

const MAX_TEXT_CHARS: usize = 256;

#[cfg_attr(not(windows), allow(dead_code))]
#[derive(Debug, PartialEq, Eq)]
pub enum ToastAppIdChoice {
    Identifier,
    PowershellFallback,
}

/// Parity with tauri-plugin-notification 2.4.0 desktop.rs: the two last components
/// `target` + (`debug` | `release`) mean a dev/unbundled run.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn toast_app_id_choice(exe_dir: &std::path::Path) -> ToastAppIdChoice {
    let comps: Vec<_> = exe_dir.components().collect();
    if comps.len() >= 2 {
        let last = comps[comps.len() - 1].as_os_str();
        let prev = comps[comps.len() - 2].as_os_str();
        if prev == "target" && (last == "debug" || last == "release") {
            return ToastAppIdChoice::PowershellFallback;
        }
    }
    ToastAppIdChoice::Identifier
}

fn validate_notify_text(title: &str, body: &str) -> Result<(), AppError> {
    if title.trim().is_empty() {
        return Err(AppError::Validation {
            field: "title".to_string(),
            message: "Заголовок уведомления не может быть пустым".to_string(),
        });
    }
    if title.chars().count() > MAX_TEXT_CHARS {
        return Err(AppError::Validation {
            field: "title".to_string(),
            message: format!("Заголовок уведомления длиннее {MAX_TEXT_CHARS} символов"),
        });
    }
    if body.chars().count() > MAX_TEXT_CHARS {
        return Err(AppError::Validation {
            field: "body".to_string(),
            message: format!("Текст уведомления длиннее {MAX_TEXT_CHARS} символов"),
        });
    }
    Ok(())
}

#[cfg_attr(not(windows), allow(dead_code))]
fn focus_main_and_emit<R: tauri::Runtime>(app: &tauri::AppHandle<R>, token: u32) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
    let _ = app.emit(NOTIFY_ACTIVATED_EVENT, token);
}

/// Show an OS notification whose click is routed back to the app.
/// Returns `true` only when a click will be delivered (Windows, shown OK).
#[tauri::command]
#[specta::specta]
pub async fn notify_desktop(
    app: tauri::AppHandle,
    title: String,
    body: String,
    token: u32,
) -> Result<bool, AppError> {
    validate_notify_text(&title, &body)?;

    #[cfg(windows)]
    {
        let exe_dir = match tauri::utils::platform::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        {
            Some(d) => d,
            None => return Ok(false),
        };
        let app_id = match toast_app_id_choice(&exe_dir) {
            ToastAppIdChoice::Identifier => app.config().identifier.clone(),
            ToastAppIdChoice::PowershellFallback => {
                tauri_winrt_notification::Toast::POWERSHELL_APP_ID.to_string()
            }
        };
        let app_clone = app.clone();
        let shown = tauri::async_runtime::spawn_blocking(move || {
            tauri_winrt_notification::Toast::new(&app_id)
                .title(&title)
                .text1(&body)
                .on_activated(move |_action| {
                    focus_main_and_emit(&app_clone, token);
                    Ok(())
                })
                .show()
                .is_ok()
        })
        .await
        .unwrap_or(false);
        Ok(shown)
    }
    #[cfg(not(windows))]
    {
        let _ = (&app, token);
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn p(parts: &[&str]) -> PathBuf {
        PathBuf::from_iter(parts.iter())
    }

    #[test]
    fn debug_and_release_fall_back() {
        assert_eq!(
            toast_app_id_choice(&p(&["a", "target", "debug"])),
            ToastAppIdChoice::PowershellFallback
        );
        assert_eq!(
            toast_app_id_choice(&p(&["a", "target", "release"])),
            ToastAppIdChoice::PowershellFallback
        );
    }

    #[test]
    fn installed_and_deps_use_identifier() {
        assert_eq!(
            toast_app_id_choice(&p(&["Programs", "Trackly"])),
            ToastAppIdChoice::Identifier
        );
        assert_eq!(
            toast_app_id_choice(&p(&["a", "target", "debug", "deps"])),
            ToastAppIdChoice::Identifier
        );
    }

    #[test]
    fn text_validation() {
        assert!(validate_notify_text("", "x").is_err());
        assert!(validate_notify_text("  ", "x").is_err());
        assert!(validate_notify_text(&"a".repeat(257), "x").is_err());
        assert!(validate_notify_text("t", &"a".repeat(257)).is_err());
        assert!(validate_notify_text("Новое сообщение в заявке №7 · Иванов И.И.", "").is_ok());
    }
}
