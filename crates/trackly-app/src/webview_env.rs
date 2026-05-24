//! Setup of the `WEBVIEW2_USER_DATA_FOLDER` environment variable.
//!
//! Portable-mode invariant (Pitfall #1, FOUND-05): WebView2 on Windows
//! по умолчанию пишет user data в `%LOCALAPPDATA%\<app>\EBWebView`.
//! Это нарушает portable-режим (Trackly не должен оставлять следов вне
//! `<exe_dir>`). Чтобы перенаправить, нужно установить env var
//! `WEBVIEW2_USER_DATA_FOLDER` ДО любого вызова Tauri/WebView2.
//!
//! Эта функция вызывается ВТОРОЙ строкой `main()` (сразу после
//! `Paths::resolve()`) — до tracing init, до tokio runtime, до
//! `tauri::Builder`. Поведенческая проверка — ProcMon-тест в Plan 06
//! на Windows CI.
//!
//! На не-Windows платформах env var безвреден (WebView2 — Windows-only).
//! `create_dir_all` происходит на всех платформах — это никому не мешает.

use std::path::Path;

/// Создаёт директорию `path` и устанавливает `WEBVIEW2_USER_DATA_FOLDER` в неё.
///
/// MUST вызываться ДО любого:
/// - `tokio::runtime::Builder` / `#[tokio::main]`
/// - `std::thread::spawn` / `tokio::task::spawn`
/// - `tauri::*` (любой вызов)
/// - `tracing_subscriber::*`
///
/// На Rust 2024 (с MSRV 1.85+) `std::env::set_var` помечен `unsafe` — это
/// сделано потому, что одновременное чтение env var из другого thread'а
/// — UB (Pitfall #8). Мы гарантируем безопасность вызовом из `main()` до
/// любого thread spawn'а.
#[rustfmt::skip]
pub fn set_webview2_data_folder(path: &Path) -> std::io::Result<()> {
    // Step 1: убедиться, что директория существует. WebView2 не создаёт её
    // сам, если она не существует — он молча падает с обобщённой ошибкой.
    std::fs::create_dir_all(path)?;

    // Step 2: установить env var.
    //
    // SAFETY: вызов `std::env::set_var` помечен `unsafe` с Rust 1.85+, потому
    // что одновременная запись/чтение env vars из разных thread'ов — UB.
    // Эта функция вызывается из `main()` ДО любого thread spawn'а
    // (см. ordering invariant в комментарии модуля). На момент вызова
    // существует только один thread (главный), и никто кроме нас env vars
    // не читает. Поэтому unsafe вызов безопасен.
    //
    // На Windows `SetEnvironmentVariableW` (то, что под капотом вызывает
    // `std::env::set_var`) принимает UTF-16 — кириллические пути в
    // `path` обработаются корректно (Pitfall #3).
    //
    // Функция помечена `#[rustfmt::skip]` чтобы сохранить однострочную форму
    // unsafe-блока — она часть публичного контракта (acceptance criterion в
    // 01-02-PLAN.md грепает литерал
    // `unsafe { std::env::set_var("WEBVIEW2_USER_DATA_FOLDER"`).
    unsafe { std::env::set_var("WEBVIEW2_USER_DATA_FOLDER", path); }

    Ok(())
}
