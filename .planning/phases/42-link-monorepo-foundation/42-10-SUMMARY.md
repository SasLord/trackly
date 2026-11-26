---
phase: 42-link-monorepo-foundation
plan: 10
subsystem: link-app
tags: [tauri, link, LNK-01, LNK-10]
requires: [42-08]
provides:
  - крейт crates/link-app (bin trackly-link) с литеральной версией 0.1.0
  - tauri.conf.json Link (NSIS currentUser, окно 1000x700, min 720x500, строгая CSP)
  - capabilities минимальны (core:default, window-state:default)
  - иконки знака Trackly с меткой связи
key-files:
  created: [crates/link-app/Cargo.toml, build.rs, src/main.rs, tauri.conf.json, capabilities/main.json, icons/]
  modified: [Cargo.toml, Cargo.lock, THIRD-PARTY-NOTICES.md, scripts/check-privacy.mjs]
metrics:
  tasks: 2
  completed: 2026-11-26
---

# Phase 42 Plan 10: крейт Trackly Link Summary

Tauri-2-крейт Link с независимой версией, окном с запоминанием размера (tauri-plugin-window-state 2.5.0), командой `link_info` и минимальными правами. Коммиты: 89c30f8c, 57906f73.

## Что сделано

- `link-app` добавлен в workspace; `version = "0.1.0"` литералом (D-10), bin `trackly-link`; `link_info` берёт версию из `package_info()`, WebView2 из `tauri::webview_version`, каталог — родитель `current_exe`. Окно `visible:false` + явный `show()`/`set_focus()` в setup. Два юнит-теста `build_link_info`.
- `tauri.conf.json`: productName «Trackly Link», mainBinaryName `trackly-link`, identifier `org.trackly.link`, NSIS x64 `currentUser`, русский язык, devUrl :1421, без shell и signingIdentity.
- Иконки: знак Trackly + векторное звено цепи (`icons/source/link-icon.svg`, рендер qlmanage, набор через `tauri icon`; android/ios удалены). Финальная прорисовка — Фаза 46.
- Запись о window-state в THIRD-PARTY-NOTICES.md.

## Проверка

`cargo fmt --check`, `cargo clippy -p link-app --all-targets -- -D warnings`, `cargo test -p link-app` (2/2) зелёные; `pnpm --dir ui lint` и `svelte-check` (0/0) зелёные; check-privacy и его selftest PASS. `cargo deny` не запускался.

## Отклонения

**[Rule 3 - Blocker] Гейт приватности класса D** блокировал коммит PNG в `crates/link-app/icons/`. В `BINARY_ALLOWLIST_PREFIXES` (scripts/check-privacy.mjs) добавлен префикс `crates/link-app/icons/` по образцу иконок trackly-app; selftest зелёный.

## К проверке позже

- CSP (допущение A10) подтвердится первой Windows-сборкой/пилотом.
- Визуальный запуск окна Link и читаемость иконки при 16 px — человеком, мной не проверялось (план 09 не требовался).
- `link-ui/dist/index.html` — локальная заглушка (каталог в .gitignore).

## Self-Check: PASSED
