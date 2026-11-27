#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! Trackly Link: каркас окна без сети, входа, трея и чата (Фаза 42).

use serde::Serialize;
use tauri::Manager;

/// Данные служебной строки окна (D-16).
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct LinkInfo {
    pub version: String,
    pub webview: Option<String>,
    pub install_path: String,
}

/// Чистая сборка `LinkInfo`: каталог установки — родитель exe.
pub fn build_link_info(
    version: &str,
    webview: Option<String>,
    exe: Option<std::path::PathBuf>,
) -> LinkInfo {
    let install_path = exe
        .as_deref()
        .and_then(|p| p.parent())
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    LinkInfo {
        version: version.to_string(),
        webview,
        install_path,
    }
}

#[tauri::command]
fn link_info(app: tauri::AppHandle) -> LinkInfo {
    build_link_info(
        &app.package_info().version.to_string(),
        tauri::webview_version().ok(),
        std::env::current_exe().ok(),
    )
}

/// Ссылки на команды: `generate_handler!` не принимает макросы-обёртки, список сверяется
/// с `link_commands.json` тестом `commands_contract`.
fn main() {
    use link_app::autostart::{ensure_enabled, AutoLaunchBackend};
    use link_app::runtime::{show_and_focus, tray_handlers, Runtime, MAIN_WINDOW};
    use link_app::state::LinkState;
    use link_app::tray::build_tray;
    use link_app::window_policy::{apply_close_policy, default_probe, detect_tray, skip_taskbar};
    use tauri_plugin_window_state::StateFlags;

    let app = tauri::Builder::default()
        // Единственный экземпляр на сеанс пользователя (D-19, без Global\ у мьютекса плагина):
        // второй запуск показывает окно первого.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_and_focus(app);
        }))
        // Всё состояние окна, кроме видимости: ею управляет только решение автомата (D-18).
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(StateFlags::all() - StateFlags::VISIBLE)
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            link_info,
            link_app::commands::link_get_state,
            link_app::commands::link_submit_invite,
            link_app::commands::link_login,
            link_app::commands::link_retry_sso,
            link_app::commands::link_logout,
            link_app::commands::link_recheck_pin,
            link_app::commands::link_retry_now,
            link_app::commands::link_quit
        ])
        .setup(|app| {
            // macOS: без значка в Dock, Link живёт в трее (D-20).
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let handle = app.handle().clone();
            let mut tray_available = detect_tray(&*default_probe());
            if tray_available {
                if let Err(e) = build_tray(&handle, &LinkState::Starting, None, tray_handlers()) {
                    tracing::warn!("трей не создан, окно обычное: {e}");
                    tray_available = false;
                }
            }

            // Для обеих веток: значок в панели задач только при живом трее (D-20).
            if let Some(w) = app.get_webview_window(MAIN_WINDOW) {
                let _ = w.set_skip_taskbar(skip_taskbar(tray_available));
            }

            // D-17: автозапуск восстанавливается при каждом старте, пункта отключения нет.
            match AutoLaunchBackend::for_current_exe() {
                Ok(backend) => {
                    let outcome = ensure_enabled(&backend);
                    tracing::info!("автозапуск: {outcome:?}");
                }
                Err(e) => tracing::warn!("автозапуск недоступен: {e}"),
            }

            let runtime = Runtime::start(&handle, tray_available)?;
            app.manage(runtime);

            // Без трея окно единственная точка доступа: показываем всегда (D-20).
            if !tray_available {
                show_and_focus(&handle);
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let tray_available = window
                    .app_handle()
                    .try_state::<Runtime>()
                    .map(|rt| rt.meta.tray_available)
                    .unwrap_or(true);
                if let Some(w) = window.app_handle().get_webview_window(window.label()) {
                    let _ = apply_close_policy(&w, api, tray_available);
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("ошибка запуска Trackly Link");

    app.run(|_app, event| {
        // Явный выход (`app.exit(0)` из «Закрыть» и link_quit) несёт код и проходит;
        // неявный (закрылось последнее окно) блокируется: Link живёт в трее.
        if let tauri::RunEvent::ExitRequested { api, code, .. } = event {
            if code.is_none() {
                api.prevent_exit();
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn info_with_all_fields() {
        let i = build_link_info(
            "0.1.0",
            Some("123.0".into()),
            Some(PathBuf::from("/opt/link/trackly-link")),
        );
        assert_eq!(i.version, "0.1.0");
        assert_eq!(i.webview.as_deref(), Some("123.0"));
        assert_eq!(i.install_path, "/opt/link");
    }

    #[test]
    fn info_without_optionals() {
        let i = build_link_info("0.1.0", None, None);
        assert_eq!(i.webview, None);
        assert_eq!(i.install_path, "");
    }
}
