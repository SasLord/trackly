//! Трей Link (UI-SPEC §7): модель меню, подсказки, выбор иконки и тонкая обёртка Tauri.
//!
//! Решения (`menu_model`, `tooltip_for`, `icon_kind_for`) чистые и тестируются на macOS.
//! Живое поведение (клик, перезапуск Explorer, AppIndicator) проверяется в плане 20.
//! Серая иконка при `Offline` выставляется вызывающим после задержки ~5 с (D-22):
//! задержку считает ядро, здесь состояние уже «окончательное».

use std::sync::Arc;

use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};

use crate::state::LinkState;

/// Идентификатор значка в трее.
pub const TRAY_ID: &str = "main";
const ID_OPEN: &str = "open";
const ID_LOGOUT: &str = "logout";
const ID_QUIT: &str = "quit";

/// Цвет значка: обычный или серый (нет связи / нужно действие сотрудника).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconKind {
    Normal,
    Gray,
}

/// Пункт меню трея. Пункта про запуск при входе в систему нет (D-17).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuItemModel {
    Open,
    Separator,
    /// Неактивная строка состояния.
    Status(String),
    Logout,
    Quit,
}

/// Подсказка значка по состоянию (UI-SPEC §7.1).
pub fn tooltip_for(state: &LinkState) -> &'static str {
    match state {
        LinkState::Connected { .. } | LinkState::Starting => "Trackly Link",
        LinkState::Offline { .. } => "Trackly Link — нет связи",
        LinkState::PinMismatch { .. } => "Trackly Link — сертификат сервера изменился",
        LinkState::NeedsSetup | LinkState::NeedsLogin { .. } | LinkState::Revoked { .. } => {
            "Trackly Link — требуется вход"
        }
    }
}

/// Какой иконкой рисовать состояние (UI-SPEC §7.1).
pub fn icon_kind_for(state: &LinkState) -> IconKind {
    match state {
        LinkState::Connected { .. } | LinkState::Starting => IconKind::Normal,
        _ => IconKind::Gray,
    }
}

/// Есть ли вход (виден ли «Выйти из учётной записи»).
fn has_login(state: &LinkState) -> bool {
    matches!(
        state,
        LinkState::Connected { .. } | LinkState::Offline { .. }
    )
}

/// Строка состояния меню. `last_user` — ФИО последнего вошедшего: в `Offline` состояние
/// имени не несёт, его хранит ядро.
fn status_text(state: &LinkState, last_user: Option<&str>) -> String {
    match state {
        LinkState::Connected { user, .. } => format!("{} · в сети", user.display_name),
        LinkState::Offline { .. } => match last_user {
            Some(name) => format!("{name} · нет связи"),
            None => "Нет связи".to_string(),
        },
        LinkState::PinMismatch { .. } => "Сертификат сервера изменился".to_string(),
        LinkState::Starting => "Запуск…".to_string(),
        LinkState::NeedsSetup | LinkState::NeedsLogin { .. } | LinkState::Revoked { .. } => {
            "Вход не выполнен".to_string()
        }
    }
}

/// Модель меню (UI-SPEC §7.2): Открыть, строка состояния, при входе «Выйти из учётной записи», Закрыть.
pub fn menu_model(state: &LinkState, last_user: Option<&str>) -> Vec<MenuItemModel> {
    let mut items = vec![
        MenuItemModel::Open,
        MenuItemModel::Separator,
        MenuItemModel::Status(status_text(state, last_user)),
        MenuItemModel::Separator,
    ];
    if has_login(state) {
        items.push(MenuItemModel::Logout);
        items.push(MenuItemModel::Separator);
    }
    items.push(MenuItemModel::Quit);
    items
}

/// Что делают пункты меню. Реализует ядро (план 17).
#[derive(Clone)]
pub struct TrayHandlers {
    pub on_open: Arc<dyn Fn(&AppHandle) + Send + Sync>,
    pub on_logout: Arc<dyn Fn(&AppHandle) + Send + Sync>,
    pub on_quit: Arc<dyn Fn(&AppHandle) + Send + Sync>,
}

// Ассеты зашиты в бинарник: Link ничего не ищет рядом с exe (ADR 0005).
const ICO_NORMAL: &[u8] = include_bytes!("../icons/tray/tray-normal.ico");
const ICO_GRAY: &[u8] = include_bytes!("../icons/tray/tray-gray.ico");
const PNG_NORMAL_22: &[u8] = include_bytes!("../icons/tray/tray-normal-22.png");
const PNG_GRAY_22: &[u8] = include_bytes!("../icons/tray/tray-gray-22.png");
const PNG_TEMPLATE: &[u8] = include_bytes!("../icons/tray/tray-template.png");
const PNG_TEMPLATE_GRAY: &[u8] = include_bytes!("../icons/tray/tray-template-gray.png");

fn icon_bytes(kind: IconKind) -> &'static [u8] {
    match (kind, std::env::consts::OS) {
        (IconKind::Normal, "windows") => ICO_NORMAL,
        (IconKind::Gray, "windows") => ICO_GRAY,
        (IconKind::Normal, "macos") => PNG_TEMPLATE,
        (IconKind::Gray, "macos") => PNG_TEMPLATE_GRAY,
        (IconKind::Normal, _) => PNG_NORMAL_22,
        (IconKind::Gray, _) => PNG_GRAY_22,
    }
}

fn load_icon(kind: IconKind) -> tauri::Result<Image<'static>> {
    Image::from_bytes(icon_bytes(kind))
}

fn build_menu(
    app: &AppHandle,
    state: &LinkState,
    last_user: Option<&str>,
) -> tauri::Result<Menu<Wry>> {
    let menu = Menu::new(app)?;
    for item in menu_model(state, last_user) {
        match item {
            MenuItemModel::Open => menu.append(&MenuItem::with_id(
                app,
                ID_OPEN,
                "Открыть Link",
                true,
                None::<&str>,
            )?)?,
            MenuItemModel::Separator => menu.append(&PredefinedMenuItem::separator(app)?)?,
            MenuItemModel::Status(text) => menu.append(&MenuItem::with_id(
                app,
                "status",
                text,
                false,
                None::<&str>,
            )?)?,
            MenuItemModel::Logout => menu.append(&MenuItem::with_id(
                app,
                ID_LOGOUT,
                "Выйти из учётной записи",
                true,
                None::<&str>,
            )?)?,
            MenuItemModel::Quit => menu.append(&MenuItem::with_id(
                app,
                ID_QUIT,
                "Закрыть",
                true,
                None::<&str>,
            )?)?,
        }
    }
    Ok(menu)
}

/// Показать окно, если скрыто; скрыть, если видно и в фокусе (левый клик, UI-SPEC §7.2).
pub fn toggle_main_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let visible = window.is_visible().unwrap_or(false);
    let focused = window.is_focused().unwrap_or(false);
    let minimized = window.is_minimized().unwrap_or(false);
    if visible && focused && !minimized {
        let _ = window.hide();
    } else {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// Собрать значок трея. Ошибка (Linux без AppIndicator) возвращается вызывающему,
/// он трактует её как «трея нет»; паники нет (Pitfall 5).
pub fn build_tray(
    app: &AppHandle,
    state: &LinkState,
    last_user: Option<&str>,
    handlers: TrayHandlers,
) -> tauri::Result<TrayIcon> {
    let menu = build_menu(app, state, last_user)?;
    let on_open = handlers.on_open.clone();
    let builder = TrayIconBuilder::with_id(TRAY_ID)
        .icon(load_icon(icon_kind_for(state))?)
        .tooltip(tooltip_for(state))
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            ID_OPEN => (on_open)(app),
            ID_LOGOUT => (handlers.on_logout)(app),
            ID_QUIT => (handlers.on_quit)(app),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            // На Linux события клика нет: остаётся только меню.
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_main_window(tray.app_handle());
            }
        });
    #[cfg(target_os = "macos")]
    let builder = builder.icon_as_template(true);
    builder.build(app)
}

/// Обновить значок, подсказку и меню под новое состояние. Перезапуск Explorer
/// (TaskbarCreated) обрабатывает сам tray-icon.
pub fn update_tray(
    app: &AppHandle,
    state: &LinkState,
    last_user: Option<&str>,
) -> tauri::Result<()> {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return Ok(());
    };
    tray.set_icon(Some(load_icon(icon_kind_for(state))?))?;
    tray.set_tooltip(Some(tooltip_for(state)))?;
    tray.set_menu(Some(build_menu(app, state, last_user)?))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{LinkUserView, LoginMethod};

    const NAME: &str = "Иванов И.И.";

    fn connected() -> LinkState {
        LinkState::Connected {
            user: LinkUserView {
                display_name: NAME.into(),
                login: "ivanov".into(),
            },
            method: LoginMethod::Sso,
            server: "srv".into(),
        }
    }
    fn offline() -> LinkState {
        LinkState::Offline {
            server: "srv".into(),
        }
    }
    fn needs_login() -> LinkState {
        LinkState::NeedsLogin {
            reason: None,
            can_retry_sso: false,
            after_logout: false,
            server: "srv".into(),
        }
    }
    fn pin_mismatch() -> LinkState {
        LinkState::PinMismatch {
            server: "srv".into(),
            expected: "a".into(),
            got: "b".into(),
        }
    }

    #[test]
    fn connected_menu_matches_ui_spec() {
        use MenuItemModel::*;
        assert_eq!(
            menu_model(&connected(), None),
            vec![
                Open,
                Separator,
                Status(format!("{NAME} · в сети")),
                Separator,
                Logout,
                Separator,
                Quit
            ]
        );
    }

    #[test]
    fn status_line_per_state() {
        let status = |s: &LinkState, u: Option<&str>| {
            menu_model(s, u)
                .into_iter()
                .find_map(|i| match i {
                    MenuItemModel::Status(t) => Some(t),
                    _ => None,
                })
                .unwrap()
        };
        assert_eq!(
            status(&offline(), Some(NAME)),
            format!("{NAME} · нет связи")
        );
        for s in [
            needs_login(),
            LinkState::NeedsSetup,
            LinkState::Revoked {
                server: "srv".into(),
            },
        ] {
            assert_eq!(status(&s, None), "Вход не выполнен");
            assert!(!menu_model(&s, None).contains(&MenuItemModel::Logout));
        }
        assert_eq!(
            status(&pin_mismatch(), None),
            "Сертификат сервера изменился"
        );
        assert!(!menu_model(&pin_mismatch(), None).contains(&MenuItemModel::Logout));
        assert!(menu_model(&offline(), Some(NAME)).contains(&MenuItemModel::Logout));
    }

    #[test]
    fn tooltips_follow_ui_spec() {
        assert_eq!(tooltip_for(&offline()), "Trackly Link — нет связи");
        assert_eq!(tooltip_for(&connected()), "Trackly Link");
        assert_eq!(tooltip_for(&LinkState::Starting), "Trackly Link");
        assert_eq!(tooltip_for(&needs_login()), "Trackly Link — требуется вход");
        assert_eq!(
            tooltip_for(&LinkState::NeedsSetup),
            "Trackly Link — требуется вход"
        );
        assert_eq!(
            tooltip_for(&pin_mismatch()),
            "Trackly Link — сертификат сервера изменился"
        );
    }

    #[test]
    fn icon_kind_table() {
        assert_eq!(icon_kind_for(&connected()), IconKind::Normal);
        assert_eq!(icon_kind_for(&LinkState::Starting), IconKind::Normal);
        for s in [
            offline(),
            needs_login(),
            LinkState::NeedsSetup,
            pin_mismatch(),
        ] {
            assert_eq!(icon_kind_for(&s), IconKind::Gray);
        }
    }

    #[test]
    fn no_autostart_item_in_any_menu() {
        // D-17: в модели нет варианта автозапуска, а тексты не содержат слова.
        for s in [connected(), offline(), needs_login(), pin_mismatch()] {
            for item in menu_model(&s, Some(NAME)) {
                if let MenuItemModel::Status(t) = item {
                    assert!(!t.to_lowercase().contains("автозапуск"));
                }
            }
        }
    }

    fn png_size(b: &[u8]) -> (u32, u32) {
        assert_eq!(&b[..8], b"\x89PNG\r\n\x1a\n", "не PNG");
        (
            u32::from_be_bytes(b[16..20].try_into().unwrap()),
            u32::from_be_bytes(b[20..24].try_into().unwrap()),
        )
    }

    fn ico_sizes(b: &[u8]) -> Vec<u32> {
        assert_eq!(&b[..4], [0, 0, 1, 0], "не ICO");
        let n = u16::from_le_bytes([b[4], b[5]]) as usize;
        (0..n)
            .map(|i| {
                let w = b[6 + 16 * i];
                if w == 0 {
                    256
                } else {
                    u32::from(w)
                }
            })
            .collect()
    }

    #[test]
    fn icon_assets_are_valid() {
        for ico in [ICO_NORMAL, ICO_GRAY] {
            let sizes = ico_sizes(ico);
            for want in [16, 20, 24, 32] {
                assert!(sizes.contains(&want), "в ICO нет кадра {want}: {sizes:?}");
            }
        }
        assert_eq!(png_size(PNG_NORMAL_22), (22, 22));
        assert_eq!(png_size(PNG_GRAY_22), (22, 22));
        assert_eq!(png_size(PNG_TEMPLATE), (36, 36));
        assert_eq!(png_size(PNG_TEMPLATE_GRAY), (36, 36));
    }

    #[test]
    fn icons_decode_with_tauri() {
        // image-png / image-ico: Tauri должен принять ассеты на этой платформе.
        assert!(load_icon(IconKind::Normal).is_ok());
        assert!(load_icon(IconKind::Gray).is_ok());
        // ICO используется только на Windows, но должен читаться везде.
        assert!(Image::from_bytes(ICO_NORMAL).is_ok());
        assert!(Image::from_bytes(ICO_GRAY).is_ok());
    }
}
