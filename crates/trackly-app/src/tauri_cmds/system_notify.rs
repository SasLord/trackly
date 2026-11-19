//! Desktop OS notification with a click handler - Phase 41.5 gaps R2-1 and R3-1.
//!
//! tauri-plugin-notification 2.4.0 shows the toast via notify-rust and drops the
//! `NotificationHandle`, so a click is lost. On Windows we build the toast ourselves with
//! tauri-winrt-notification (`Toast::on_activated`): a click focuses the `main` window and
//! emits `trackly-notify-activated` with a numeric token to the webview.
//!
//! R3-1: `Toast::show()` returns `Ok` as soon as Windows accepts the call, but for an AUMID
//! unknown to the shell the toast is silently dropped. So the AUMID is chosen by run kind:
//! an installed NSIS build (an uninstaller next to the exe; the Start-menu shortcut carries
//! AUMID = identifier) uses the identifier; portable and dev runs use POWERSHELL_APP_ID,
//! which needs no registration (D-R3-1: the "Windows PowerShell" caption is accepted).
//! `Paths::is_portable()` is not usable here: it is also true when a config file lies next
//! to an installed exe. Before showing, `ToastNotifier::Setting()` is read (read-only):
//! a probe error (AUMID unknown) falls back to POWERSHELL_APP_ID; a successful probe that
//! is not Enabled (user or AD policy) means the toast is NOT shown under any AUMID and the
//! outcome is `suppressed` (the frontend must not use the plugin path either).
//! Portable: no registry writes, no AUMID registration, no shortcuts, no app-data dirs.
//! Logs (`notify_desktop:` lines) never contain notification text or file paths.

use serde::Serialize;
use specta::Type;
use tauri::{Emitter, Manager};
use trackly_core::error::AppError;

/// Event delivered to the webview when the user clicks the OS notification.
#[cfg_attr(not(windows), allow(dead_code))]
pub const NOTIFY_ACTIVATED_EVENT: &str = "trackly-notify-activated";

const MAX_TEXT_CHARS: usize = 256;

#[cfg_attr(not(windows), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastAppIdChoice {
    Identifier,
    PowershellFallback,
}

#[cfg_attr(not(windows), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastRunKind {
    Dev,
    Installed,
    Portable,
}

/// Result of the read-only `Setting()` probe. `Unknown` = the probe returned an error
/// (the AUMID is not known to the shell).
#[cfg_attr(not(windows), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeKind {
    Enabled,
    Disabled,
    Unknown,
}

#[cfg_attr(not(windows), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeDecision {
    Show,
    FallbackToPowershell,
    Suppress,
}

/// Outcome of `notify_desktop` as seen by the frontend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum NotifyDesktopOutcome {
    /// Toast accepted by Windows; a click will be delivered.
    Shown,
    /// Notifications are disabled in Windows (user or policy): show nothing at all.
    Suppressed,
    /// Not Windows / no exe dir / show failed: the frontend uses the plugin path.
    Unavailable,
}

/// NSIS template of Tauri writes the uninstaller into $INSTDIR and gives the Start-menu
/// shortcut AUMID = identifier; the portable ZIP contains no uninstaller.
#[cfg_attr(not(windows), allow(dead_code))]
const NSIS_UNINSTALLER: &str = "uninstall.exe";

/// D-R3-1: AUMID for portable. The single switch point if the user revisits the decision.
#[cfg_attr(not(windows), allow(dead_code))]
const PORTABLE_AUMID: ToastAppIdChoice = ToastAppIdChoice::PowershellFallback;

/// Parity with tauri-plugin-notification 2.4.0 desktop.rs: the two last components
/// `target` + (`debug` | `release`) mean a dev/unbundled run.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn classify_run(exe_dir: &std::path::Path, has_uninstaller: bool) -> ToastRunKind {
    let comps: Vec<_> = exe_dir.components().collect();
    if comps.len() >= 2 {
        let last = comps[comps.len() - 1].as_os_str();
        let prev = comps[comps.len() - 2].as_os_str();
        if prev == "target" && (last == "debug" || last == "release") {
            return ToastRunKind::Dev;
        }
    }
    if has_uninstaller {
        ToastRunKind::Installed
    } else {
        ToastRunKind::Portable
    }
}

#[cfg_attr(not(windows), allow(dead_code))]
pub fn aumid_for(kind: ToastRunKind) -> ToastAppIdChoice {
    match kind {
        ToastRunKind::Dev => ToastAppIdChoice::PowershellFallback,
        ToastRunKind::Installed => ToastAppIdChoice::Identifier,
        ToastRunKind::Portable => PORTABLE_AUMID,
    }
}

#[cfg_attr(not(windows), allow(dead_code))]
pub fn probe_decision(choice: ToastAppIdChoice, probe: ProbeKind) -> ProbeDecision {
    match (choice, probe) {
        (_, ProbeKind::Enabled) => ProbeDecision::Show,
        // A prohibition by the user or AD policy is respected: neither swap the AUMID
        // nor go through the plugin.
        (_, ProbeKind::Disabled) => ProbeDecision::Suppress,
        (ToastAppIdChoice::Identifier, ProbeKind::Unknown) => ProbeDecision::FallbackToPowershell,
        (ToastAppIdChoice::PowershellFallback, ProbeKind::Unknown) => ProbeDecision::Show,
    }
}

/// Read-only probe of the notification setting for an AUMID.
#[cfg(windows)]
fn probe(app_id: &str) -> (ProbeKind, String) {
    use windows::UI::Notifications::{NotificationSetting, ToastNotificationManager};
    let res =
        ToastNotificationManager::CreateToastNotifierWithId(&windows::core::HSTRING::from(app_id))
            .and_then(|n| n.Setting());
    match res {
        Ok(s) if s == NotificationSetting::Enabled => (ProbeKind::Enabled, format!("{s:?}")),
        Ok(s) => (ProbeKind::Disabled, format!("{s:?}")),
        Err(e) => (ProbeKind::Unknown, format!("{:?}", e.code())),
    }
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
#[tauri::command]
#[specta::specta]
pub async fn notify_desktop(
    app: tauri::AppHandle,
    title: String,
    body: String,
    token: u32,
) -> Result<NotifyDesktopOutcome, AppError> {
    tracing::info!(token, "notify_desktop: called");
    validate_notify_text(&title, &body)?;

    #[cfg(windows)]
    {
        let exe_dir = match tauri::utils::platform::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        {
            Some(d) => d,
            None => {
                tracing::warn!(token, "notify_desktop: exe dir unavailable");
                return Ok(NotifyDesktopOutcome::Unavailable);
            }
        };
        let has_uninstaller = exe_dir.join(NSIS_UNINSTALLER).is_file();
        let kind = classify_run(&exe_dir, has_uninstaller);
        let mut choice = aumid_for(kind);
        let powershell_id = tauri_winrt_notification::Toast::POWERSHELL_APP_ID;
        let id_of = |c: ToastAppIdChoice| match c {
            ToastAppIdChoice::Identifier => app.config().identifier.clone(),
            ToastAppIdChoice::PowershellFallback => powershell_id.to_string(),
        };
        let mut aumid = id_of(choice);
        let (mut probe_kind, mut probe_desc) = probe(&aumid);
        let mut decision = probe_decision(choice, probe_kind);
        tracing::info!(token, run = ?kind, aumid = ?choice, probe = %probe_desc, decision = ?decision, "notify_desktop: probe");
        if decision == ProbeDecision::FallbackToPowershell {
            choice = ToastAppIdChoice::PowershellFallback;
            aumid = id_of(choice);
            (probe_kind, probe_desc) = probe(&aumid);
            decision = probe_decision(choice, probe_kind);
            tracing::info!(token, run = ?kind, aumid = ?choice, probe = %probe_desc, decision = ?decision, "notify_desktop: probe");
        }
        if decision == ProbeDecision::Suppress {
            tracing::info!(token, "notify_desktop: suppressed by windows settings");
            return Ok(NotifyDesktopOutcome::Suppressed);
        }
        let app_clone = app.clone();
        // Same shape as tauri-plugin-notification 2.4.0 / notify-rust 4.18.2.
        let res = tauri::async_runtime::spawn(async move {
            tauri_winrt_notification::Toast::new(&aumid)
                .title(&title)
                .text1("")
                .text2(&body)
                .sound(None)
                .duration(tauri_winrt_notification::Duration::Short)
                .on_activated(move |_action| {
                    tracing::info!(token, "notify_desktop: toast activated");
                    focus_main_and_emit(&app_clone, token);
                    Ok(())
                })
                .on_dismissed(move |reason| {
                    tracing::info!(token, reason = ?reason, "notify_desktop: toast dismissed");
                    Ok(())
                })
                .show()
        })
        .await;
        match res {
            Ok(Ok(())) => {
                tracing::info!(token, "notify_desktop: show ok");
                Ok(NotifyDesktopOutcome::Shown)
            }
            Ok(Err(e)) => {
                tracing::warn!(token, error = ?e, "notify_desktop: show failed");
                Ok(NotifyDesktopOutcome::Unavailable)
            }
            Err(_) => {
                tracing::warn!(token, "notify_desktop: show task failed");
                Ok(NotifyDesktopOutcome::Unavailable)
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = &app;
        tracing::debug!(token, "notify_desktop: not windows");
        Ok(NotifyDesktopOutcome::Unavailable)
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
    fn classify_run_kinds() {
        assert_eq!(
            classify_run(&p(&["a", "target", "debug"]), false),
            ToastRunKind::Dev
        );
        assert_eq!(
            classify_run(&p(&["a", "target", "release"]), true),
            ToastRunKind::Dev
        );
        assert_eq!(
            classify_run(&p(&["Programs", "Trackly"]), true),
            ToastRunKind::Installed
        );
        assert_eq!(
            classify_run(&p(&["Programs", "Trackly"]), false),
            ToastRunKind::Portable
        );
        assert_eq!(
            classify_run(&p(&["a", "target", "debug", "deps"]), false),
            ToastRunKind::Portable
        );
    }

    #[test]
    fn aumid_choice() {
        assert_eq!(
            aumid_for(ToastRunKind::Dev),
            ToastAppIdChoice::PowershellFallback
        );
        assert_eq!(
            aumid_for(ToastRunKind::Installed),
            ToastAppIdChoice::Identifier
        );
        assert_eq!(
            aumid_for(ToastRunKind::Portable),
            ToastAppIdChoice::PowershellFallback
        );
    }

    #[test]
    fn probe_table() {
        use ProbeDecision as D;
        use ProbeKind as K;
        use ToastAppIdChoice as C;
        assert_eq!(probe_decision(C::Identifier, K::Enabled), D::Show);
        assert_eq!(probe_decision(C::Identifier, K::Disabled), D::Suppress);
        assert_eq!(
            probe_decision(C::Identifier, K::Unknown),
            D::FallbackToPowershell
        );
        assert_eq!(probe_decision(C::PowershellFallback, K::Enabled), D::Show);
        assert_eq!(probe_decision(C::PowershellFallback, K::Unknown), D::Show);
        assert_eq!(
            probe_decision(C::PowershellFallback, K::Disabled),
            D::Suppress
        );
    }

    #[test]
    fn outcome_serialization() {
        let j = |o| serde_json::to_string(&o).unwrap();
        assert_eq!(j(NotifyDesktopOutcome::Shown), "\"shown\"");
        assert_eq!(j(NotifyDesktopOutcome::Suppressed), "\"suppressed\"");
        assert_eq!(j(NotifyDesktopOutcome::Unavailable), "\"unavailable\"");
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
