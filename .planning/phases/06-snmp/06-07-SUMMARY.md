---
phase: 06-snmp
plan: "07"
subsystem: requests
tags: [gap-closure, requests, history, a11y, frontend, backend]
dependency_graph:
  requires: []
  provides:
    - requests_get_history (REQ-07)
    - arg-key fix requests_create (REQ-01, REQ-02)
    - requests_counts command name fix
    - a11y div[role=tablist] in SearchAndTabs
  affects:
    - ui/src/features/requests/api.ts
    - crates/trackly-app/src/http/requests.rs
    - crates/trackly-app/src/tauri_cmds/requests.rs
    - crates/trackly-app/src/services/request_service.rs
    - crates/trackly-infra/src/repos/requests_sqlite.rs
    - crates/trackly-app/src/specta_export.rs
    - ui/src/features/requests/RequestsSearchAndTabs.svelte
    - ui/src/features/printers/PrintersSearchAndTabs.svelte
tech_stack:
  added: []
  patterns:
    - "get_history inherent impl pattern (not trait) — mirrors cartridges_sqlite"
    - "AuditEntryRow/AuditEntryDto reuse across request and cartridge history"
key_files:
  created: []
  modified:
    - crates/trackly-infra/src/repos/requests_sqlite.rs
    - crates/trackly-app/src/services/request_service.rs
    - crates/trackly-app/src/tauri_cmds/requests.rs
    - crates/trackly-app/src/http/requests.rs
    - crates/trackly-app/src/specta_export.rs
    - ui/src/features/requests/api.ts
    - ui/src/features/requests/RequestsSearchAndTabs.svelte
    - ui/src/features/printers/PrintersSearchAndTabs.svelte
decisions:
  - "AuditEntryDto reused from dto/cartridge.rs — no new DTO created"
  - "get_history as inherent impl on SqliteRequestRepository (not trait method) — matches cartridges pattern"
  - "statusCounts wrapper kept with same name, signature simplified to () — zero impact on callers"
metrics:
  duration: "~25 min"
  completed: "2026-06-15T04:54:46Z"
  tasks_completed: 3
  files_modified: 8
---

# Phase 6 Plan 07: Requests Gap-Closure Summary

**One-liner:** Закрыты три критических дефекта портала заявок: arg-key `dto`, имя команды `requests_counts`, новая `requests_get_history` (REQ-07); устранён a11y-конфликт `<nav role="tablist">`.

## What Was Built

### Task 1: requests_get_history end-to-end (REQ-07)

- **`crates/trackly-infra/src/repos/requests_sqlite.rs`** — добавлен `get_history(conn, request_id)` как inherent impl (не trait) с SQL-фильтром `entity_type='request' AND action NOT IN ('list','get') ORDER BY created_at_utc DESC`. Переиспользует `AuditEntryRow` из cartridges_sqlite.
- **`crates/trackly-app/src/services/request_service.rs`** — добавлен `async fn get_history(request_id)` через `spawn_blocking` + `ReaderPool`, маппинг в `AuditEntryDto`. Переиспользует DTO из `dto/cartridge.rs`.
- **`crates/trackly-app/src/tauri_cmds/requests.rs`** — `build_requests_get_history` + `#[tauri::command] requests_get_history(id: i32)`.
- **`crates/trackly-app/src/http/requests.rs`** — `handler_get_history` + route `/api/v1/requests_get_history`.
- **`crates/trackly-app/src/specta_export.rs`** — `requests_get_history` зарегистрирована в `collect_commands!`.
- **Тест:** `test_request_get_history_returns_create_entry` — после вставки заявки + audit записи `get_history()` возвращает ≥1 строку с action='create'. Зелёный.

### Task 2: Исправление arg-key и имён команд в api.ts

- **`ui/src/features/requests/api.ts`**:
  - `create`: `{ payload }` → `{ dto: payload }` — согласовано с Rust arg `dto`.
  - `statusCounts`: `'requests_status_counts'` → `'requests_counts'`, убран аргумент `{ filter }` (бэкенд не принимает фильтр).
  - `getHistory`: уже вызывал `requests_get_history` с `{ id }` — без изменений.
- `svelte-check`: 0 ошибок.

### Task 3: a11y tablist + ролевой рендер (code-review)

- **`RequestsSearchAndTabs.svelte`** и **`PrintersSearchAndTabs.svelte`**: `<nav role="tablist">` → `<div role="tablist" aria-label="...">`. Устранён ARIA-конфликт (non-interactive element with interactive role).
- **Ролевой рендер (code-review):** В `RequestDetail.svelte:288` lifecycle-кнопки обёрнуты в `{#if isSpecialist}` где `isSpecialist = admin || manager`. Сотрудник (role='employee') исключён → кнопки Принять/Отклонить/Выполнить скрыты. Серверная авторизация `authorize(caller, TransitionRequests)` остаётся источником истины.
- `pnpm svelte-check`: 0 ошибок, tablist-предупреждения устранены (31 warnings осталось, все из других компонентов).

## Verification

```
cargo check --workspace       → Finished (0 errors)
cargo test -p trackly-infra   → ok. 51 passed; 0 failed
test_request_get_history_returns_create_entry → ok
pnpm svelte-check             → 0 ERRORS 31 WARNINGS (tablist warnings eliminated)
```

## Deviations from Plan

None — план выполнен точно.

**Observation (Task 1):** `get_history` потребовал создания отдельного `impl SqliteRequestRepository` блока (не в trait impl), так как `RequestRepository` trait не объявляет этот метод. Это корректный паттерн — аналогичен расположению вне trait у других репозиториев с приватными хелперами.

## Known Stubs

None — все три исправления функциональны. История заявки использует реальный audit_log, который уже пишется при create/transition.

## Threat Flags

None — изменения не вводят новых trust boundaries. `requests_get_history` защищена `session_identity` на HTTP и `resolve_tauri_identity` на Tauri (через неявный AppCtx). T-06-07-02 (Information Disclosure) митигирован.

## Pending Human Verification (Task 3 checkpoint)

Следующий шаг — запустить приложение и выполнить ручную проверку:

1. `cd ui && pnpm svelte-check` — убедиться в 0 ошибок (done автоматически).
2. `TRACKLY_SNMP_MOCK=1 cargo tauri dev` — открыть раздел Заявки.
3. Создать заявку «Свободная форма» → должно показать «Заявка отправлена».
4. Открыть созданную заявку → блок «История» должен показать строку создания.
5. Кликнуть по вкладкам статусов — счётчики без ошибок в консоли.
6. Code-review ролевого рендера подтверждён в п.3 выше.

## Self-Check: PASSED

- SUMMARY.md: FOUND at .planning/phases/06-snmp/06-07-SUMMARY.md
- Commit 734e257: FOUND (feat requests_get_history)
- Commit fc9c514: FOUND (fix api.ts arg-keys)
- Commit e73c629: FOUND (fix tablist a11y)

## Post-Checkpoint Fix (human UAT, 2026-06-15)

Human-verify обнаружил баг: блок «История» показывал `NaN.NaN.NaN NaN:NaN`.
Причина — `requests_get_history` отдавал картриджный `AuditEntryDto`
(snake_case `created_at_utc`, без `actorName`/`notes`), фронтенд ждёт camelCase
`createdAtUtc` + `actorName` + `notes` → `new Date(undefined)` = Invalid Date.

Fix (commit 8654f89):
- Новый `RequestHistoryEntryDto` (camelCase) — отдельный от cartridge DTO.
- Repo `get_history`: LEFT JOIN users → `actor_name`; `RequestHistoryRow`.
- `transition()` пишет `notes` в `payload_json` аудита → История показывает
  причину reject/complete (REQ-07).
- 2 теста репозитория (actor_name join, notes payload) — зелёные.
- `svelte-check`: 0 errors. `cargo check`/`clippy` (app+infra): чисто.

Вторая жалоба UAT — «через веб нельзя добавить заявку под Сотрудник/Специалист»
— НЕ баг 06-07: вход по ролям через браузер отложен до Phase 8 (AD), что
зафиксировано в 06-VERIFICATION.md (truth 6). Вне scope этой gap-фазы.
