---
phase: 44-link
plan: 09
subsystem: link-client
tags: [link-app, tray, autostart, window-policy]
requires: [44-05]
provides:
  - window_policy.rs (close_action, skip_taskbar, detect_tray, TrayProbe, apply_*_policy)
  - autostart.rs (run_value, AutostartBackend, ensure_enabled, AutoLaunchBackend)
  - tray.rs (menu_model, tooltip_for, icon_kind_for, build_tray, update_tray, toggle_main_window)
  - 6 ассетов иконок трея в crates/link-app/icons/tray
affects: [44-17, 44-20]
key-files:
  created:
    - crates/link-app/icons/tray/ (6 файлов)
  modified:
    - crates/link-app/src/window_policy.rs
    - crates/link-app/src/autostart.rs
    - crates/link-app/src/tray.rs
    - crates/link-app/Cargo.toml
    - Cargo.lock
decisions:
  - "menu_model(state, last_user): LinkState::Offline не несёт ФИО, его передаёт ядро"
  - "Кавычки вокруг пути автозапуска на Windows и Linux, на macOS без кавычек (plist хранит путь отдельным элементом)"
requirements-completed: [LNK-02, LNK-03, LNK-04, LNK-11]
completed: 2026-11-28
---

# Phase 44 Plan 09: Оболочка Link Summary

Чистые решения для трея, автозапуска и политики окна (покрыты тестами на macOS) плюс тонкие обёртки Tauri; реальное поведение оболочки ОС ждёт живой проверки в плане 20.

## Коммиты

- ef2596ff: window_policy.rs
- bf931293: autostart.rs
- a8f27916: tray.rs и иконки

## Проверки

- `cargo test -p link-app --lib window_policy`: 5 зелёных; `autostart`: 6 зелёных; `tray`: 7 зелёных (включая декодирование ICO/PNG через Tauri)
- `cargo build -p link-app`, `cargo clippy -p link-app --all-targets -- -D warnings`, `cargo fmt --check -p link-app`: чисто
- Грепы приёмки: `Dynamic`, `fn disable`, `.disable(`, `tauri-plugin-autostart` в Cargo.toml, `Автозапуск` в tray.rs: пусто; `WindowsEnableMode::CurrentUser` найден

## Отклонения от плана

1. **[Rule 3] Cargo.toml:** `zbus = "5"` добавлен напрямую под `cfg(target_os = "linux")` (план разрешал zbus::blocking; в графе версия 5.19 уже есть от secret-service-хранилища, Cargo.lock получил одну строку). Также к tauri добавлена фича `image-ico`, чтобы Windows-иконка читалась как ICO.
2. **Сигнатура `menu_model`:** `menu_model(state, last_user: Option<&str>)` вместо `(state, tray_click_supported: bool)`: «Иванов И.И. · нет связи» в `Offline` нельзя собрать без имени, а булев параметр модель не менял (на Linux без клика просто остаётся меню).
3. **Иконки:** исходный `icon.png` непрозрачный (белый фон), прозрачность восстановлена по расстоянию до белого, затем уменьшение через `sips`; серый вариант: `#8a8f99` с той же альфой; macOS template: чёрный силуэт с альфой (серый: 40 %). ICO собран с PNG-кадрами 16/20/24/32/40/48. Скрипт генерации не коммитился (одноразовый, лежал в scratchpad); кадры PNG 24 для Linux не делались (в плане обязательны только 22).
4. Линукс-проба (`LinuxDbusProbe`) под `cfg(linux)` на macOS не компилируется: не проверена сборкой.

## Не проверено

- Живое поведение трея на Windows/macOS/Linux, перезапуск Explorer, GNOME без AppIndicator, запись Run с путём с пробелом, LaunchAgent и XDG-запись: план 20.
- Компиляция `LinuxDbusProbe` (zbus 5.19 blocking API): требует сборки на Linux (CI).
- Применение политики (`apply_close_policy`, `apply_taskbar_policy`, `ensure_enabled`, `build_tray`) к реальному окну подключает план 17.

## Угрозы

T-44-30 (кавычки, CurrentUser): закрыто тестом `run_value_quotes_path_with_space`. T-44-31 (потерянный Link): `detect_tray` + `Minimize`/`skip_taskbar=false`. T-44-32 (accept): Диспетчер задач может переопределить StartupApproved, Link восстанавливает запись только при отсутствии; это следствие D-17.

## Авто-одобренные чекпоинты

Нет (в плане нет чекпоинтов).

## Self-Check: PASSED
