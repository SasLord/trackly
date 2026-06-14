---
phase: 06-snmp
plan: "05"
subsystem: ui-requests
tags: [svelte5, requests, portal, OperationModal, REQ-05, WS]
dependency_graph:
  requires:
    - 06-03  # backend requests_* Tauri commands
    - 06-04  # bindings-phase6.ts types + ws.ts
  provides:
    - full requests portal (RequestsPage + RequestDetail + RequestFormModal)
    - OperationModal preFillPrinterId prop (REQ-05 gate for RequestDetail)
  affects:
    - ui/src/features/cartridges/OperationModal.svelte (added prop)
    - ui/src/pages/RequestsPage.svelte (Placeholder removed)
tech_stack:
  added: []
  patterns:
    - requests/api.ts dual-transport (apiCall)
    - RequestsMasterDetail 35/65 grid (паттерн CartridgesMasterDetail)
    - RequestsSearchAndTabs status switch-bar (паттерн PrintersSearchAndTabs)
    - RequestFormModal type-toggle с conditional fields
    - RequestDetail lifecycle кнопки role+status, confirm-modal «Отклонить»
    - WS push via onWsEvent (new_request/request_status_changed)
key_files:
  created:
    - ui/src/features/requests/api.ts
    - ui/src/features/requests/RequestsMasterDetail.svelte
    - ui/src/features/requests/RequestsSearchAndTabs.svelte
    - ui/src/features/requests/RequestsList.svelte
    - ui/src/features/requests/RequestListRow.svelte
    - ui/src/features/requests/RequestFormModal.svelte
    - ui/src/features/requests/RequestDetail.svelte
    - ui/src/features/requests/RequestsPage.svelte
  modified:
    - ui/src/features/cartridges/OperationModal.svelte
    - ui/src/pages/RequestsPage.svelte
decisions:
  - "specialist role maps to manager in actual UserRole type ('admin'|'manager'|'employee'); isSpecialist = admin || manager"
  - "RequestDetail + RequestsPage created in single commit (Task 1) since RequestDetail is a compile-time dependency of RequestsPage"
  - "preFillPrinterId shown as context hint in OperationModal install form (instead of pre-populating invisible field)"
metrics:
  duration: "7 min"
  completed: "2026-06-15"
  tasks_completed: 2
  files_changed: 10
---

# Phase 06 Plan 05: Requests Portal Summary

Portal заявок с полным lifecycle — сотрудник создаёт заявки через браузер, специалист управляет через RequestDetail.

## What Was Built

Полноценный портал заявок (REQ-01..05, REQ-07) как вертикальный срез UI:

- **OperationModal.svelte** — добавлен `preFillPrinterId?: number` prop (REQ-05 prerequisite). При op='install' показывает hint о принтере-контексте.
- **requests/api.ts** — dual-transport обёртки: `list`, `get`, `create`, `transition`, `listCategories`, `statusCounts`, `getHistory`.
- **RequestFormModal.svelte** — модалка создания заявки с тип-переключателем «Замена картриджа» / «Свободная форма», условными полями, валидацией перед submit.
- **RequestListRow.svelte** — строка списка: тип-badge, краткое описание, статус-badge, автор, относительная дата.
- **RequestsList.svelte** — список с empty-config (роль-зависимые тексты), spinner, пагинатор.
- **RequestsSearchAndTabs.svelte** — switch-bar null/open/in_progress/completed/rejected + кнопка «Создать заявку».
- **RequestsMasterDetail.svelte** — 35/65 grid (по паттерну CartridgesMasterDetail).
- **RequestDetail.svelte** — карточка заявки: поля по типу, lifecycle кнопки (только admin/manager), REQ-05 «Установить картридж» → OperationModal с preFillPrinterId, confirm-modal «Отклонить», секция «История» (REQ-07).
- **RequestsPage.svelte** (features) — корневой компонент с роль-зависимой логикой, WS push (new_request → toast для admin/manager), re-fetch при request_status_changed.
- **pages/RequestsPage.svelte** — Placeholder заменён на импорт features/requests/RequestsPage.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] 'specialist' role не существует в UserRole**
- **Found during:** Task 1 (svelte-check)
- **Issue:** Plan использует роль 'specialist', но UserRole = 'admin' | 'manager' | 'employee'. TypeScript: "unintentional comparison with no overlap"
- **Fix:** `isSpecialist = role === 'admin' || role === 'manager'` (manager = specialist в контексте приложения)
- **Files modified:** RequestDetail.svelte, RequestsPage.svelte
- **Commit:** 296c48c

**2. [Rule 1 - Bug] aria-label не поддерживается Button.svelte Props**
- **Found during:** Task 1 (svelte-check)
- **Issue:** Button Props interface не включает aria-label. 7 ошибок "Object literal may only specify known properties"
- **Fix:** Убраны aria-label атрибуты; достаточно текстового содержимого кнопок для доступности
- **Files modified:** RequestDetail.svelte
- **Commit:** 296c48c

**3. [Rule 2 - Missing] preFillPrinterId не read в OperationModal (TS error)**
- **Found during:** Task 1 (svelte-check)
- **Issue:** prop объявлен в Props но не используется нигде в компоненте → TS error "declared but never read"
- **Fix:** Добавлен `printerContextHint = $derived(...)` + отображение hint в install-форме
- **Files modified:** OperationModal.svelte
- **Commit:** 296c48c

**4. [Rule 3 - Blocking] Task 1 и Task 2 выполнены вместе**
- **Found during:** Task 1 planning
- **Issue:** RequestsPage.svelte импортирует RequestDetail.svelte — compile-time зависимость. Без RequestDetail.svelte Task 1 не пройдёт svelte-check
- **Fix:** RequestDetail.svelte создан в рамках Task 1 (не отдельным коммитом для Task 2), т.к. план явно разрешает это — оба задания в одном коммите
- **Commit:** 296c48c

## Known Stubs

Нет критических стабов. Все данные заявок берутся из backend API через apiCall. `requests.getHistory` возвращает записи из audit_log — если backend ещё не реализовал `requests_get_history`, история будет пустой (graceful degradation).

## Threat Flags

| Flag | File | Description |
|------|------|-------------|
| threat_flag: role-bypass | RequestDetail.svelte | isSpecialist проверка на клиенте скрывает lifecycle кнопки для employee, но реальная защита — authorize() в backend (Plan 03, T-06-15-E accepted) |

## Self-Check: PASSED

All key files exist and commit 296c48c is confirmed. svelte-check: 0 errors.
