---
phase: 06-snmp
plan: "04"
subsystem: ui-printers
tags: [svelte5, ui, printers, snmp, websocket, dual-transport]
dependency_graph:
  requires: [06-03]
  provides: [printers-ui, ws-client]
  affects: [ui-routing, phase6-requirements]
tech_stack:
  added: []
  patterns:
    - Svelte 5 runes master-detail page (PrintersPage pattern)
    - Dual-transport WS client (browser WebSocket + Tauri events + exponential backoff)
    - TonerGauge progressbar with threshold-based color coding
    - DiscoveryModal 2-step flow (scan → review → admit)
    - Phase 6 type definitions in bindings-phase6.ts (gitignored bindings workaround)
key_files:
  created:
    - ui/src/bindings-phase6.ts
    - ui/src/features/printers/api.ts
    - ui/src/lib/api/ws.ts
    - ui/src/features/printers/TonerGauge.svelte
    - ui/src/features/printers/PrinterAlertBanner.svelte
    - ui/src/features/printers/PrinterListRow.svelte
    - ui/src/features/printers/PrintersList.svelte
    - ui/src/features/printers/PrintersSearchAndTabs.svelte
    - ui/src/features/printers/PrintersMasterDetail.svelte
    - ui/src/features/printers/PrintersPage.svelte
    - ui/src/features/printers/PrinterDetail.svelte
    - ui/src/features/printers/DiscoveryModal.svelte
    - ui/src/features/printers/DiscoveryResultsTable.svelte
  modified:
    - ui/src/pages/PrintersPage.svelte
decisions:
  - "bindings-phase6.ts: типы Phase 6 вынесены в отдельный файл (не генерируемый bindings.ts, который gitignored) для хранения в git без использования git add -f"
  - "PrinterDetail.getReadings: вызывается в $effect при смене printer, а не через отдельный пропс — данные загружаются компонентом самостоятельно"
  - "TonerGauge.encoding: параметр 'percent' | 'level_over_max' позволяет поддерживать оба формата SNMP (Pantum = percent, остальные = level_over_max)"
metrics:
  duration: "8 min"
  completed: "2026-06-15"
  tasks: 3
  files: 14
---

# Phase 06 Plan 04: Printer UI Vertical Slice Summary

**One-liner:** Svelte 5 master-detail раздел «Принтеры» с TonerGauge, AlertBanner, DiscoveryModal и dual-transport WS-клиентом (browser WebSocket + Tauri events + exponential backoff reconnect).

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | api.ts + ws.ts + bindings-phase6.ts | 5c79b54 | bindings-phase6.ts, printers/api.ts, ws.ts |
| 2a | Base printer UI components | e5bb0fe | TonerGauge, PrinterAlertBanner, PrinterListRow, PrintersList, PrintersSearchAndTabs, PrintersMasterDetail, PrintersPage |
| 2b | PrinterDetail + Discovery + page routing | 8804b47 | PrinterDetail, DiscoveryModal, DiscoveryResultsTable, pages/PrintersPage.svelte |

## Artifacts Delivered

### ui/src/features/printers/api.ts
Dual-transport обёртки команд: `list`, `get`, `create`, `delete`, `discover`, `admit`, `refresh`, `acknowledgeAlert`, `getReadings`.

### ui/src/lib/api/ws.ts
WS-клиент с dual-transport: браузерный `WebSocket` → `/api/v1/ws` с exponential backoff (1s→2s→4s→...→30s); Tauri → `@tauri-apps/api/event listen('trackly-event')`. Экспортирует `connectWs`, `onWsEvent`, `disconnectWs`. `WsEvent` variant `'request_status_changed'` (не `'request_updated'`) — синхронизировано с Rust.

### ui/src/features/printers/TonerGauge.svelte
`role="progressbar"`, `aria-valuenow/min/max`, высота 8px. Цвет: accent ≥25%, warning 10–24%, destructive <10%, surface-sunken при null/unknown.

### ui/src/features/printers/PrinterAlertBanner.svelte
`role="alert"`, `aria-live="polite"`, по паттерну LowStockBanner. Текст по alertType: offline/error. Фон `color-mix(warning 10%)`, граница усиливается до destructive для error.

### ui/src/features/printers/PrinterDetail.svelte
Секции: Уровни тонера (TonerGauge per entry), Страничные счётчики, **Установленный картридж** (`printer.currentCartridgeId`, PRN-07, D-PRN07-01), История статусов (printers_get_readings), Метаданные. Header: DisplayName (font-size-display, semibold), статус-badge, «Обновить сейчас» (loading state).

### ui/src/features/printers/DiscoveryModal.svelte
size="wide" (960px). 2-step: scan → review → admit. Reset on open. `handleScan` → `printers.discover`. `handleCreate` → `printers.admit`. CTA «Завести выбранные (N)» disabled при 0 выбранных.

### ui/src/features/printers/DiscoveryResultsTable.svelte
Колонки: чекбокс / IP / Производитель / Модель / Имя (sysName) / Статус. Badge «Уже заведён» (default) для isDuplicate=true. Header-чекбокс: выбрать все не-дубликаты.

### ui/src/features/printers/PrintersPage.svelte (pages)
Заменён `<Placeholder section="Принтеры" />` на `<PrintersPage />` из features.

## Verification Results

```
pnpm --dir ui svelte-check: 0 ERRORS, 32 WARNINGS (все warnings — pre-existing в других файлах)
grep role="progressbar" TonerGauge.svelte: FOUND
grep role="alert" PrinterAlertBanner.svelte: FOUND
grep Placeholder pages/PrintersPage.svelte: NOT found (component import — only in comment)
grep connectWs ws.ts: FOUND
grep currentCartridgeId PrinterDetail.svelte: FOUND (PRN-07)
grep request_status_changed ws.ts: FOUND (not request_updated)
```

## Deviations from Plan

### [Rule 1 - Bug] Исправлены TypeScript ошибки в $derived

**Found during:** Task 2a
**Issue:** Svelte 5 `$derived` не принимает функции-обёртки как аргумент (тип `() => T` вместо `T`)
**Fix:** TonerGauge.svelte — переписан derived в ternary-expression; PrinterListRow.svelte — IIFE внутри derived; удалены неиспользованные imports (Badge в PrintersSearchAndTabs, onMount в PrinterDetail)
**Files modified:** TonerGauge.svelte, PrinterListRow.svelte, PrintersSearchAndTabs.svelte, PrinterDetail.svelte

### [Architectural Note] bindings.ts gitignored — создан bindings-phase6.ts

**Context:** `ui/src/bindings.ts` находится в `.gitignore` (генерируется `cargo test -p trackly-app --test export_bindings`). Добавление типов Phase 6 напрямую в bindings.ts потребовало бы `git add -f` (обход gitignore).
**Decision:** Создан `ui/src/bindings-phase6.ts` — файл под git-контролем, содержащий Phase 6 типы. При регенерации bindings.ts (при следующем `cargo test`) Phase 6 типы должны быть включены в сгенерированный файл, а `bindings-phase6.ts` может быть удалён.
**Impact:** api.ts и ws.ts импортируют из `../../bindings-phase6` вместо `../../bindings`.

## Known Stubs

None — все компоненты полностью реализованы согласно плану.

## Threat Flags

| Flag | File | Description |
|------|------|-------------|
| threat_flag: ui_rbac | PrintersSearchAndTabs.svelte | Кнопка «Найти принтеры» скрыта для не-admin (`identity.role === 'admin'`). UI-гейт — только UX-слой; реальная защита на бэкенде (D-RBAC-01, T-06-13-E). |

## Self-Check: PASSED

Files created/verified:
- ui/src/bindings-phase6.ts: FOUND
- ui/src/features/printers/api.ts: FOUND
- ui/src/lib/api/ws.ts: FOUND
- ui/src/features/printers/TonerGauge.svelte: FOUND
- ui/src/features/printers/PrinterAlertBanner.svelte: FOUND
- ui/src/features/printers/PrintersPage.svelte: FOUND
- ui/src/features/printers/PrinterDetail.svelte: FOUND
- ui/src/features/printers/DiscoveryModal.svelte: FOUND
- ui/src/features/printers/DiscoveryResultsTable.svelte: FOUND
- ui/src/pages/PrintersPage.svelte: FOUND (no Placeholder component)

Commits verified:
- 5c79b54: Task 1
- e5bb0fe: Task 2a
- 8804b47: Task 2b
