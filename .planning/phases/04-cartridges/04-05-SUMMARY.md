---
phase: "04-cartridges"
plan: "05"
subsystem: cartridges-lifecycle-ui
tags: [cartridges, svelte, ui, lifecycle, context-menu, modal, portal, form]
dependency_graph:
  requires:
    - "04-03 (CartridgeService + Tauri commands + CartridgeTransitionPayload bindings)"
    - "04-04 (UI skeleton: CartridgesPage stubs, api.ts, CartridgeListRow kebab stub)"
  provides:
    - CartridgeContextMenu.svelte (status-dependent portal menu, use:portal, mousedown-outside)
    - LowStockBanner.svelte (warning banner, hidden when items=[])
    - OperationModal.svelte (5 lifecycle ops, op-dependent fields, D-Op-Fields-01 defaults)
    - CartridgeFormModal.svelte + CartridgeFormBody.svelte (CRUD modals, openInstanceCounter reset)
    - CartridgesPage.svelte fully wired (handleMenuAction dispatches to all modals)
    - CartridgeListRow.svelte kebab wired to CartridgeContextMenu (replacing stub)
    - CartridgeDetail.svelte action buttons wired (replacing disabled stubs)
  affects:
    - "04-06 (phase verification — all lifecycle operations now functional)"
tech_stack:
  added: []
  patterns:
    - "CartridgeContextMenu: use:portal + menuX/menuY from getBoundingClientRect() + mousedown-outside via svelte:window"
    - "OperationModal: op-param discriminated UI — field set switches on op string"
    - "D-Op-Fields-01 default: op=from_refill→stateId=1(Полный), others→stateId=3(Пустой)"
    - "CartridgeFormModal: openInstanceCounter + {#key} remount pattern from DeviceFormModal"
    - "CartridgeFormBody: onRegisterSubmit(fn) from onMount — no reactive trigger, no race"
    - "Conflict code error: inline field error below code Input (not toast)"
key_files:
  created:
    - ui/src/features/cartridges/CartridgeContextMenu.svelte
    - ui/src/features/cartridges/LowStockBanner.svelte
    - ui/src/features/cartridges/OperationModal.svelte
    - ui/src/features/cartridges/CartridgeFormModal.svelte
    - ui/src/features/cartridges/CartridgeFormBody.svelte
  modified:
    - ui/src/features/cartridges/CartridgesPage.svelte
    - ui/src/features/cartridges/CartridgeListRow.svelte
    - ui/src/features/cartridges/CartridgeDetail.svelte
decisions:
  - "CartridgeFormBody extracted as separate component (not inline) to work correctly with {#key openInstanceCounter} remount pattern — Svelte 5 does not support snippet-based form state reset"
  - "OperationModal uses $derived(op === 'from_refill' ? 1 : 3) for stateId default — correctly resets on each open via $effect"
  - "CartridgeListRow kebab stub replaced with CartridgeContextMenu directly in the row — no intermediate event bus; op string propagated up to CartridgesPage via onMenuAction prop"
  - "CartridgeDetail action buttons wired via new optional onMenuAction prop — backward compat (optional = no errors if parent doesn't pass it)"
metrics:
  duration_minutes: 7
  completed_date: "2026-06-08"
  tasks_completed: 2
  files_created: 5
  files_modified: 3
  tests_added: 0
  tests_fixed: 0
---

# Phase 04 Plan 05: Cartridges Lifecycle UI Summary

Four Svelte components (CartridgeContextMenu, LowStockBanner, OperationModal, CartridgeFormModal) + CartridgeFormBody + full wire-up of CartridgesPage/CartridgeListRow/CartridgeDetail for complete lifecycle interaction.

## Tasks Completed

| Task | Description | Commit | Files |
|------|-------------|--------|-------|
| 1 | CartridgeContextMenu + LowStockBanner | 9ec2ae1 | CartridgeContextMenu.svelte, LowStockBanner.svelte |
| 2 | OperationModal + CartridgeFormModal + full wire-up | 566046a | OperationModal.svelte, CartridgeFormModal.svelte, CartridgeFormBody.svelte, CartridgesPage.svelte, CartridgeListRow.svelte, CartridgeDetail.svelte |

## What Was Built

### Task 1: CartridgeContextMenu + LowStockBanner

**`ui/src/features/cartridges/CartridgeContextMenu.svelte`**
- Props: cartridge, onInstall, onReturnToStock, onToRefill, onFromRefill, onWriteOff, onEdit, onDelete
- Status-dependent menu items via `$derived.by()` — D-Op-Transitions-01:
  - status_id=1 (На складе): Установить в принтер + Отправить на заправку + Редактировать + sep + Списать + Удалить
  - status_id=2 (В работе): Вернуть на склад + Редактировать + sep + Удалить
  - status_id=3 (На заправке): Забрать с заправки + Редактировать + sep + Удалить
  - status_id=4 (Списано): Редактировать + sep + Удалить
- `use:portal` — menu rendered in `<body>` to escape overflow:hidden containers
- menuX/menuY from `triggerEl.getBoundingClientRect()`: right-160, bottom+4
- `svelte:window` onmousedown/onscroll/onresize close handlers
- `aria-label="Действия с картриджем {code}"`, `aria-expanded={menuOpen}`
- z-index: 2000 via `:global(.ctx-menu-portal)` (scoped CSS doesn't reach portaled elements)

**`ui/src/features/cartridges/LowStockBanner.svelte`**
- Props: items: LowStockItemDto[]
- `{#if items.length > 0}` — not rendered when empty (per UI-SPEC §LowStockBanner, CART-12)
- Inline SVG warning icon (triangle + exclamation), color: `--color-warning`
- Background: `color-mix(in srgb, var(--color-warning) 10%, transparent)`
- Border: `1px solid var(--color-warning)`, border-radius: `--radius-md`
- Row format: «{brand} {model} — {count} шт. на складе (порог: {threshold})»

### Task 2: OperationModal + CartridgeFormModal + Wire-up

**`ui/src/features/cartridges/OperationModal.svelte`**
- type Op = 'install' | 'return_to_stock' | 'to_refill' | 'from_refill' | 'write_off'
- Props: open, op, cartridge, onClose, onSuccess
- `$effect` reset on open: dateIso=today, givenByName/givenToName/location/notes='', stateId=defaultStateId
- D-Op-Fields-01: `$derived(op === 'from_refill' ? 1 : 3)` — from_refill→Полный(1), others→Пустой(3)
- Field sets by op:
  - install | to_refill: DatePicker + PersonAutocomplete(Кто выдал) + PersonAutocomplete(Кому выдал) + LocationAutocomplete (hint for install only)
  - return_to_stock | from_refill: Select(Состояние заряда, defaultStateId) + LocationAutocomplete + Textarea (hint for return_to_stock only)
  - write_off: DatePicker + Textarea
- Validation: required fields checked before submit; inline error messages
- handleSubmit: validate → cartridges.transition(buildPayload()) → onSuccess() + onClose() + pushToast success → catch → pushToast error
- Modal titles per UI-SPEC §Заголовки OperationModal; confirm labels per §Primary CTA

**`ui/src/features/cartridges/CartridgeFormModal.svelte`**
- Props: open, target (null=create), models, onClose, onSuccess
- openInstanceCounter pattern: `$effect(() => { if (open && !_wasOpen) openInstanceCounter++; ... })`
- `{#key openInstanceCounter}` remounts CartridgeFormBody on every open
- Footer buttons driven by formLoading/formCanSubmit/bodySubmitFn from CartridgeFormBody callbacks

**`ui/src/features/cartridges/CartridgeFormBody.svelte`**
- Fields: Код (Input, placeholder C-XXXXXX, hint "Будет присвоен автоматически...") + Модель (Select, required) + Состояние заряда (create only) + Расположение (LocationAutocomplete) + Примечания (Textarea)
- isEdit = target !== null; в edit-mode: Состояние заряда скрыто (update принимает только location+notes)
- code_override: `code.trim() || null` — пустая строка → авто-код backend
- Conflict error detection: inline codeError под полем кода (не toast) — UI-SPEC §Ошибочные состояния
- onRegisterSubmit(fn) from onMount — direct call from footer button (no reactive trigger)

**CartridgesPage.svelte wire-up:**
- handleMenuAction dispatches: install/return_to_stock/to_refill/from_refill/write_off → OperationModal; edit → CartridgeFormModal; delete → confirmDeleteModal
- handleOperationSuccess: refresh() + refreshCounts() + refreshLowStock() + reload selected cartridge detail
- handleFormSuccess: refresh() + auto-select created/updated cartridge
- Inline low-stock-banner replaced with `<LowStockBanner items={lowStockItems} />`
- Confirm-delete Modal with destructive button + loading state

**CartridgeListRow.svelte wire-up:**
- Kebab stub replaced with `<CartridgeContextMenu>` — all op callbacks forwarded via onMenuAction

**CartridgeDetail.svelte wire-up:**
- New optional prop `onMenuAction` — action buttons wired to real handlers (no longer disabled)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing] Wire-up of CartridgesPage, CartridgeListRow, CartridgeDetail**
- **Found during:** Task 2 implementation
- **Issue:** Plan 04-05 files_modified listed only 4 new components, but without wiring CartridgesPage/CartridgeListRow/CartridgeDetail, the lifecycle UI would be unreachable — components created but not connected
- **Fix:** Wired all three files in same commit as Task 2; CartridgeFormBody extracted as separate component to support {#key} remount pattern
- **Files modified:** CartridgesPage.svelte, CartridgeListRow.svelte, CartridgeDetail.svelte, CartridgeFormBody.svelte (new)
- **Commit:** 566046a

## Verification

- `pnpm svelte-check`: 0 errors, 21 warnings (same count as 04-04 — all pre-existing Phase 2/3 files)
- `pnpm lint`: 0 new errors; 6 prettier warnings are pre-existing acts files (scope boundary)
- `use:portal` in CartridgeContextMenu: `grep -c "use:portal" CartridgeContextMenu.svelte` → 2 (declaration + usage)
- `op === 'install' || op === 'to_refill'` in OperationModal: field branching present
- `items.length > 0` in LowStockBanner: conditional rendering confirmed
- `op === 'from_refill' ? 1 : 3` in OperationModal: D-Op-Fields-01 default confirmed
- openInstanceCounter in CartridgeFormModal: 5 references (declared + $effect + {#key} + footer logic)

## Known Stubs

| Stub | File | Reason |
|------|------|--------|
| Models tab | CartridgesPage.svelte | ModelsList/ModelFormModal deferred — plan 04-05 does not include model CRUD |
| suggestPerson | OperationModal via PersonAutocomplete | PersonAutocomplete uses acts.suggestPerson (existing endpoint) — no cartridges-specific suggest needed |

The models tab placeholder is intentional: plan 04-05 scope covers instance lifecycle only; model CRUD is a separate feature.

## Threat Flags

No new network endpoints or trust boundary crossings. OperationModal user inputs (givenByName, givenToName, location, notes) are passed as CartridgeTransitionPayload strings — no XSS risk (Svelte renders as text, not innerHTML). Backend validates via AppError::Validation (T-04-05-01 mitigated). MenuItems generated from cartridge.status_id from backend (T-04-05-02 mitigated). code_override: `code.trim() || null` prevents empty string injection (T-04-05-03 mitigated).

## Self-Check: PASSED

- `ui/src/features/cartridges/CartridgeContextMenu.svelte`: FOUND
- `ui/src/features/cartridges/LowStockBanner.svelte`: FOUND
- `ui/src/features/cartridges/OperationModal.svelte`: FOUND
- `ui/src/features/cartridges/CartridgeFormModal.svelte`: FOUND
- `ui/src/features/cartridges/CartridgeFormBody.svelte`: FOUND
- Task 1 commit 9ec2ae1: FOUND
- Task 2 commit 566046a: FOUND
