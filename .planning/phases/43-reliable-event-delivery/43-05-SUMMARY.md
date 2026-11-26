---
phase: 43-reliable-event-delivery
plan: 05
subsystem: ui-gates
tags: [websocket, resync, registry-gate, svelte]
requires: [43-03]
provides:
  - "check-ws-resync-registry.mjs: реестровый гейт onWsResync для подписчиков onWsEvent (MIN_SUBSCRIBERS=13, EXEMPTIONS, selftest M1-M7)"
  - "onWsResync в Layout, EmployeeLayout, PrintersPage, EmployeeDevicesPage"
affects: [43-07]
key-files:
  created:
    - ui/scripts/check-ws-resync-registry.mjs
  modified:
    - ui/src/features/layout/Layout.svelte
    - ui/src/features/layout/EmployeeLayout.svelte
    - ui/src/features/printers/PrintersPage.svelte
    - ui/src/features/employee/EmployeeDevicesPage.svelte
key-decisions:
  - "D-08: Layout на ресинк зовёт notifyEntitiesChanged([]) (пустой список = перечитать всё)"
  - "D-09: EmployeeLayout на ресинк зовёт refreshUnread() — тихо, без pushToast и ОС-уведомлений"
  - "D-10: PrintersPage — refresh(), EmployeeDevicesPage — scheduleRefresh(); формы, фильтры и выбор не трогаются"
requirements-completed: [DLV-01]
completed: 2026-11-27
---

# Phase 43 Plan 05: реестровый гейт ресинка и первые 4 подписчика Summary

Гейт не пропускает подписчика onWsEvent без onWsResync; четыре экрана перечитывают данные на ресинк тихо и снимают регистрацию в cleanup.

## Задачи

1. Реестровый гейт check-ws-resync-registry.mjs с selftest M1-M7 — `9b26f439`
2. onWsResync в оболочках и страницах — `a5f6cae3`

## Проверки

- `node ui/scripts/check-ws-resync-registry.mjs --selftest` — OK (M1-M7 пойманы).
- Боевой запуск — exit 1, перечисляет ровно 9 непокрытых файлов из 13 найденных (Attachments x2, RequestDetail, RequestsPage, RequestChat, chatNotifications, unreadStore, OrgSettings, NumberTemplateField); ожидаемо до Плана 07. Четыре файла этого плана в списке отсутствуют.
- `pnpm svelte-check` — 0 ошибок, 0 предупреждений; prettier и eslint по затронутым файлам чисты; check-employee-tabs и check-user-scoped-reset зелёные.
- Гейт в `pnpm lint` не подключён (Plan 07), полный lint не запускался.

## Отклонения от плана

Нет.

## Known Stubs

Нет.

## Self-Check: PASSED

Файл гейта и коммиты 9b26f439, a5f6cae3 существуют.
