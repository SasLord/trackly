---
phase: 05-auth-server-mode
plan: "06"
subsystem: auth-server-settings
tags: [gap-closure, settings, http-route, tauri-command, migration-test]
dependency_graph:
  requires: ["05-01", "05-02", "05-03", "05-04", "05-05"]
  provides: ["settings_set_network-http", "settings_set_network-tauri", "migration-idempotency-verified"]
  affects: ["ui/src/bindings.ts", "NetworkSettings.svelte saveSettings()"]
tech_stack:
  added: []
  patterns:
    - "NetworkPatch DTO (Debug/Clone/Serialize/Deserialize/Type) в http/settings.rs — shared between HTTP и Tauri transports"
    - "build_settings_set_network: authorize(ManageSettings) + port validation + three upserts в app_settings"
    - "build_settings_set_network_tauri: resolve_tauri_identity + authorize(ManageSettings) — CR-01 pattern"
key_files:
  created: []
  modified:
    - crates/trackly-app/src/http/settings.rs
    - crates/trackly-app/src/tauri_cmds/auth.rs
    - crates/trackly-app/src/specta_export.rs
decisions:
  - "NetworkPatch объявлен в http/settings.rs (pub) и импортирован в tauri_cmds/auth.rs — один источник истины для DTO, не дублировать в dto/auth.rs"
  - "Три отдельных upsert-запроса в app_settings (server_host, server_port, server_cert_path) вместо одной JSON-строки — согласованно с паттерном set_desktop_lock_enabled"
  - "Task 2 — verified no-op: migration_idempotency.rs уже содержал == 19 (коммит 7c26288); тест зелёный без дополнительных изменений"
metrics:
  duration: "~20 min"
  completed: "2026-06-13T20:27:16Z"
  tasks_completed: 3
  files_modified: 3
---

# Phase 05 Plan 06: Gap Closure — settings_set_network + migration idempotency Summary

Закрыты два BLOCKER-gap'а из 05-VERIFICATION.md (score 12/14 → 14/14): реализован `settings_set_network` на HTTP и Tauri транспортах с authorize(ManageSettings), подтверждён зелёный migration_idempotency тест.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Реализовать settings_set_network (HTTP + Tauri + specta_export) | 2a88cd9 | http/settings.rs, tauri_cmds/auth.rs, specta_export.rs |
| 2 | Верифицировать migration_idempotency тест | — (no-op) | crates/trackly-infra/tests/migration_idempotency.rs |
| 3 | Финальная проверка: тесты + export_bindings + svelte-check | — (verify only) | ui/src/bindings.ts (gitignored, generated) |

## What Was Built

### Gap 1: settings_set_network (HTTP + Tauri)

**`crates/trackly-app/src/http/settings.rs`:**
- Добавлен `NetworkPatch` (pub struct, Debug/Clone/Serialize/Deserialize/Type, `#[specta(type = i32)]` на поле port)
- Добавлен `SetNetworkPayload { patch: NetworkPatch }`
- Добавлен `build_settings_set_network(ctx, session, patch)`: session_identity → authorize(ManageSettings) → port validation (1..=65535) → три upsert в app_settings (server_host, server_port, server_cert_path)
- Добавлен `handler_set_network` handler
- Зарегистрирован маршрут `POST /api/v1/settings_set_network` в `router()`
- Удалён TODO-комментарий "Phase 5+" из doc-comment модуля

**`crates/trackly-app/src/tauri_cmds/auth.rs`:**
- Добавлен `build_settings_set_network_tauri(ctx, patch)`: resolve_tauri_identity → authorize(ManageSettings) → port validation → три upsert (идентичная логика)
- Добавлена Tauri-команда `settings_set_network` с правильным порядком атрибутов `#[tauri::command]` перед `#[specta::specta]`

**`crates/trackly-app/src/specta_export.rs`:**
- Зарегистрирована `crate::tauri_cmds::auth::settings_set_network` как 14-я команда Phase 5

### Gap 2: migration_idempotency

Коммит 7c26288 уже исправил все assertion'ы на `== 19`. Тест запущен — зелёный. Файл содержит корректные значения:
- `report.applied_count == 19` (первый run)
- `report.schema_version == 19` (все три точки)
- `report2.applied_count == 0` (no-op run — правильно, не тронуто)
- `report3.applied_count == 0` (reopened DB — правильно, не тронуто)

## Verification Results

```
cargo check -p trackly-app              → OK (0 errors)
cargo test -p trackly-app               → OK (56+12+8+... все суиты зелёные)
cargo test -p trackly-infra             → OK (5/5 passed, migration_idempotency включён)
grep "settings_set_network" specta_export.rs   → 1 вхождение
grep '/api/v1/settings_set_network' http/settings.rs → маршрут присутствует
grep "settings_set_network" ui/src/bindings.ts → 1 вхождение (функция-обёртка TAURI_INVOKE)
pnpm svelte-check (ui/)                → 0 errors, 30 pre-existing warnings
```

## Deviations from Plan

### Auto-applied

**[Rule 2 - Security] authorize(ManageSettings) в build_settings_set_network_tauri**
- Found during: Task 1
- Issue: план требовал authorize только в HTTP-хелпере; Tauri-хелпер тоже должен проверять права (threat register T-05-SN-01 + T-05-SN-02: unlocked desktop yields trusted_admin, но locked desktop должен проверять роль)
- Fix: добавлен `trackly_core::auth::authorize(&caller, &Action::ManageSettings)?` в `build_settings_set_network_tauri`
- Files modified: crates/trackly-app/src/tauri_cmds/auth.rs

**[Rule 3 - No-change] Task 2 verified as no-op**
- Found during: Task 2
- Issue: план допускал, что migration_idempotency.rs может требовать исправления
- Fix: тест запущен → зелёный; файл уже содержит == 19 во всех нужных местах (коммит 7c26288). Коммит для Task 2 не создавался (нет изменений файлов)

### None — plan executed cleanly

## Known Stubs

Нет. `settings_set_network` реализован полностью: записывает server_host/server_port/server_cert_path в app_settings. `build_settings_get_network` читает из `ctx.config.server` (файл конфига), а не из app_settings — это известное ограничение, задокументированное в плане как scope boundary ("изменение build_settings_get_network — опционально и в рамках данного плана не обязательно").

## Threat Flags

Нет новых security-поверхностей сверх threat model плана: `POST /api/v1/settings_set_network` покрыт T-05-SN-01 (authorize), T-05-SN-02 (Tauri identity), T-05-SN-03 (port validation).

## Self-Check

- [x] `crates/trackly-app/src/http/settings.rs` — exists and contains `settings_set_network`
- [x] `crates/trackly-app/src/tauri_cmds/auth.rs` — exists and contains `settings_set_network`
- [x] `crates/trackly-app/src/specta_export.rs` — exists and contains `settings_set_network`
- [x] Commit 2a88cd9 — exists (feat(05-06))
- [x] `ui/src/bindings.ts` — gitignored but exists on disk with `settings_set_network`

## Self-Check: PASSED
