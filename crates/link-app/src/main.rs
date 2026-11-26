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

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .invoke_handler(tauri::generate_handler![link_info])
        .setup(|app| {
            // Страховка первого запуска: окно visible:false, а плагин мог не восстановить состояние.
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.set_focus();
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("ошибка запуска Trackly Link");
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
