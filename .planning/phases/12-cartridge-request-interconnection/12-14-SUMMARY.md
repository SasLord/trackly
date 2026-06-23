---
phase: 12-cartridge-request-interconnection
plan: 14
subsystem: api
tags: [rust, axum, tauri, rbac, sqlite, soft-delete, state-machine]

# Dependency graph
requires:
  - phase: 12-cartridge-request-interconnection
    provides: "RequestService transition() dispatcher, dual-transport (Tauri+axum) pattern, role_endpoint_matrix RBAC regression harness (Case 1-35)"
provides:
  - "RequestTransitionOp::Reject валиден из open ИЛИ in_progress (не только open)"
  - "RequestTransitionOp::Cancel — новый state-machine переход (open → cancelled, custom:cancel)"
  - "Action::DeleteRequests (Admin|Manager) и Action::CancelOwnRequest (все роли, owner-check в сервисе)"
  - "RequestService::delete() — soft-delete заявки в ЛЮБОМ статусе, Admin|Manager"
  - "RequestService::cancel() — self-cancel собственной open-заявки, Employee, BOLA-safe"
  - "Dual-transport эндпоинты requests_delete/requests_cancel (Tauri + axum POST /api/v1/...)"
  - "migrations/V031 — requests.status CHECK расширен на 'cancelled'"
  - "role_endpoint_matrix Case 36-39 — RBAC regression для новых эндпоинтов"
affects: ["12-15 (UI-кнопки delete/cancel + confirmation-модалки, зависит от этого плана)"]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "SQLite CHECK-constraint widening через 12-шаговый table-rebuild (V031, по образцу V030) — нет ALTER TABLE DROP CONSTRAINT в SQLite"
    - "Self-cancel как отдельный сервисный метод вне общего transition()-диспетчера, когда авторизация для разных операций над одной FSM принципиально различается (Admin|Manager vs all-roles+ownership)"

key-files:
  created:
    - crates/trackly-app/tests/request_lifecycle.rs
    - migrations/V031__requests_status_add_cancelled.sql
  modified:
    - crates/trackly-core/src/domain/printers.rs
    - crates/trackly-core/src/auth.rs
    - crates/trackly-app/src/services/request_service.rs
    - crates/trackly-app/src/tauri_cmds/requests.rs
    - crates/trackly-app/src/http/requests.rs
    - crates/trackly-app/src/specta_export.rs
    - crates/trackly-app/tests/role_endpoint_matrix.rs

key-decisions:
  - "cancel() реализован как полностью отдельный сервисный метод/эндпоинт, а не как вариант RequestTransitionPayload — избегает протаскивания Employee через transition()'s безусловный authorize(&Action::TransitionRequests)"
  - "delete()'s NotFound/OptimisticLockMismatch disambiguation скопирована 1:1 с CartridgeService::delete() — повторный delete уже-удалённой (но физически существующей) строки даёт OptimisticLockMismatch, не NotFound"
  - "V031 миграция добавлена как Rule 2 auto-fix (не было в плане явно) — без неё cancel() падал с CHECK constraint failed, т.к. 'cancelled' не входил в исходный CHECK requests.status"

requirements-completed: [GAP-12-07]

# Metrics
duration: ~45min
completed: 2026-06-24
---

# Phase 12 Plan 14: Управление жизненным циклом заявки — delete/cancel Summary

**Admin/Manager soft-delete заявки в любом статусе + Employee self-cancel собственной open-заявки, оба через dual-transport (Tauri+axum) эндпоинты с полным RBAC-покрытием**

## Performance

- **Duration:** ~45 мин (эта сессия — Task 1 был завершён и закоммичен в прошлой сессии)
- **Completed:** 2026-06-24
- **Tasks:** 3/3
- **Files modified:** 7 modified, 2 created

## Accomplishments
- `RequestTransitionOp::Reject` теперь валиден из `"open"` ИЛИ `"in_progress"` — специалист может отклонить заявку, которую сам же принял в работу
- Новый переход `RequestTransitionOp::Cancel` (open → cancelled) с собственными `validate_from_status`/`target_status`/`audit_action`
- `RequestService::delete()` — Admin/Manager может удалить заявку в ЛЮБОМ статусе (soft-delete, optimistic lock, audit-запись)
- `RequestService::cancel()` — Employee может отменить СОБСТВЕННУЮ заявку, только пока статус "open"; BOLA-safe (чужая заявка → Forbidden), статус-gate (in_progress → Validation)
- Dual-transport эндпоинты `requests_delete`/`requests_cancel` зарегистрированы в Tauri commands, axum routes и specta export
- 11 новых тестов: 7 в `request_lifecycle.rs` (сервисный уровень) + 4 новых RBAC-кейса (Case 36-39) в `role_endpoint_matrix.rs`

## Task Commits

1. **Task 1: Domain — Reject из in_progress, новый Cancel, новые Action** - `ce6bf20` (feat) — выполнен и закоммичен в предыдущей сессии
2. **Task 2: Сервис — RequestService::delete()/cancel()** - `b39d89a` (feat)
3. **Task 3: Dual-transport эндпоинты + specta + RBAC-тесты** - `bbec042` (feat)

**Plan metadata:** _(будет добавлен после этого SUMMARY)_

## Files Created/Modified
- `crates/trackly-core/src/domain/printers.rs` — `RequestTransitionOp::Cancel`, `Reject` из двух статусов
- `crates/trackly-core/src/auth.rs` — `Action::DeleteRequests`, `Action::CancelOwnRequest`
- `crates/trackly-app/src/services/request_service.rs` — `delete()`, `cancel()` методы
- `crates/trackly-app/tests/request_lifecycle.rs` — 7 интеграционных тестов (новый файл)
- `migrations/V031__requests_status_add_cancelled.sql` — расширение CHECK-constraint (новый файл)
- `crates/trackly-app/src/tauri_cmds/requests.rs` — `build_requests_delete`/`build_requests_cancel` + `#[tauri::command]` обёртки
- `crates/trackly-app/src/http/requests.rs` — `handler_delete`/`handler_cancel`, новые routes
- `crates/trackly-app/src/specta_export.rs` — регистрация двух новых команд
- `crates/trackly-app/tests/role_endpoint_matrix.rs` — Case 36-39

## Decisions Made
- `cancel()` НЕ добавлен как вариант `RequestTransitionPayload` — архитектурно это отдельный путь (Employee-доступный), а не часть Admin/Manager-only `transition()`-диспетчера
- Для `delete()` повторное удаление уже-удалённой строки возвращает `OptimisticLockMismatch`, не `NotFound` — сохранена консистентность с established-поведением `CartridgeService::delete()`, не изменена логика, исправлено только понимание/ожидание в тесте

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Добавлена миграция V031 — расширение CHECK requests.status**
- **Found during:** Task 2 (`RequestService::cancel()`)
- **Issue:** Исходный CHECK-constraint на `requests.status` (из V006) допускал только `'open', 'in_progress', 'completed', 'rejected'` — без миграции `cancel()` падал с `CHECK constraint failed` при попытке установить `'cancelled'`. План не указывал явно на необходимость новой миграции (раздел `<read_first>` упоминал только то, что `deleted_at_utc` уже существует и новой миграции для soft-delete не требуется — но это не относилось к новому статусу `Cancel`)
- **Fix:** Создана `migrations/V031__requests_status_add_cancelled.sql` по established table-rebuild паттерну (precedent: V030) — пересборка таблицы `requests` с расширенным CHECK, включающим `'cancelled'`
- **Files modified:** `migrations/V031__requests_status_add_cancelled.sql`
- **Verification:** `cancel_own_open_request_succeeds` тест проходит после миграции; лог подтверждает применение `applying migration: V31__requests_status_add_cancelled`
- **Committed in:** `b39d89a` (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 missing critical — Rule 2)
**Impact on plan:** Необходимое исправление для корректности — без миграции функциональность cancel() была бы полностью неработоспособна. Не выходит за рамки заявленной цели плана (GAP-12-07/A4).

## Issues Encountered
- Borrow-of-moved-value (E0382) в первой версии `cancel()` — `RequestTransitionOp::Cancel` нельзя дешево клонировать (enum содержит `Option<String>` в других вариантах, без `Copy`). Решено повторным конструированием `RequestTransitionOp::Cancel` внутри замыкания транзакции и отдельно после `.await?` для WS-push, вместо удержания одной переменной через границу замыкания.
- Тест `delete_already_deleted_request_returns_not_found` изначально ожидал `NotFound` при повторном delete — неверное ожидание теста, не баг кода. Сравнение с `CartridgeService::delete()` подтвердило established-поведение (soft-deleted строка физически существует → `OptimisticLockMismatch`). Тест переименован в `delete_already_deleted_request_returns_optimistic_lock_mismatch` с explaining doc-comment; добавлен отдельный тест `delete_nonexistent_request_returns_not_found` (id=999999) для покрытия истинного `NotFound`-пути.
- Полный прогон `cargo test -p trackly-app` выявил 2 pre-existing сбоя, НЕ связанных с этим планом: `restore_request_visibility_http.rs` и `settings_ad.rs::ad_test_connection_admin_succeeds_in_mock_mode` — оба ожидают AD mock-режим, но получают 503 "service unavailable: ad" (real AD client выбран вместо mock в текущем dev-окружении). Это out-of-scope согласно SCOPE BOUNDARY (файлы не модифицировались этим планом, проблема — окруженческая, ad_mode конфигурация). Не исправлялось.
- `cargo fmt -p trackly-app` (без указания конкретного файла после `--`) переформатировал весь пакет, затронув несвязанные файлы (`dto/cartridge.rs`, `tests/cartridges_lifecycle.rs`, `tests/request_printer_options.rs`, `tests/ws_http_single_broadcast.rs`). Эти изменения откатены через `git checkout --` перед коммитом — в финальных коммитах только файлы, релевантные плану 12-14.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Backend полностью готов: `requests_delete`/`requests_cancel` доступны через оба транспорта (Tauri invoke + HTTP), TS-биндинги сгенерированы в `ui/src/bindings.ts`
- План 12-15 (UI-кнопки + confirmation-модалки) может начинаться немедленно — зависимость на этот план удовлетворена
- Известные pre-existing AD-mock-режим сбои (`restore_request_visibility_http`, `settings_ad`) не блокируют 12-15, но стоит учесть при следующей фазе работы с AD-настройками

---
*Phase: 12-cartridge-request-interconnection*
*Completed: 2026-06-24*

## Self-Check: PASSED

All created/modified files confirmed present on disk; all 3 task commit hashes (`ce6bf20`, `b39d89a`, `bbec042`) confirmed in `git log`.
