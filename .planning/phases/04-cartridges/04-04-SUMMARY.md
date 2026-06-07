---
phase: "04-cartridges"
plan: "04"
subsystem: cartridges-ui-skeleton
tags: [cartridges, svelte, ui, master-detail, switch-bar, filters, history]
dependency_graph:
  requires:
    - "04-03 (CartridgeService + Tauri commands + bindings.ts)"
  provides:
    - CartridgesPage.svelte (root component, two tabs Картриджи/Модели)
    - CartridgesSearchAndTabs.svelte (search + tab switcher)
    - CartridgesMasterDetail.svelte (35/65 grid layout)
    - CartridgeFilters.svelte (status switch-bar + kind/model extra filters)
    - CartridgesList.svelte (list with empty/loading states)
    - CartridgeListRow.svelte (two-line row with Badge, kebab stub)
    - CartridgeDetail.svelte (detail panel + history rendering)
    - api.ts (full cartridges API wrapper)
    - sidebar /cartridges route live (phase:4 marker removed)
  affects:
    - "04-05 (CRUD + lifecycle modals wire into CartridgesPage stubs)"
tech_stack:
  added: []
  patterns:
    - "Svelte 5 runes: $state/$derived/$effect throughout all components"
    - "master-detail 35/65 grid pattern from ActsMasterDetail"
    - "switch-bar with count-badge pattern from DeviceFilters"
    - "AuditEntryDto action→label mapping for history display"
    - "hasFilter derived bool drives empty state text"
key_files:
  created:
    - ui/src/features/cartridges/api.ts
    - ui/src/features/cartridges/CartridgesPage.svelte
    - ui/src/features/cartridges/CartridgesSearchAndTabs.svelte
    - ui/src/features/cartridges/CartridgesMasterDetail.svelte
    - ui/src/features/cartridges/CartridgeFilters.svelte
    - ui/src/features/cartridges/CartridgesList.svelte
    - ui/src/features/cartridges/CartridgeListRow.svelte
    - ui/src/features/cartridges/CartridgeDetail.svelte
  modified:
    - ui/src/features/layout/sidebar-config.ts
    - ui/src/pages/CartridgesPage.svelte
decisions:
  - "pages/CartridgesPage.svelte updated to delegate to features/cartridges/CartridgesPage.svelte — no SvelteKit routing needed, spa-router handles /cartridges directly"
  - "sidebar-config.ts phase:4 removed from /cartridges entry — route is now live"
  - "CartridgeDetail action buttons wired as disabled stubs — onClick handlers deferred to plan 04-05 per plan spec"
  - "kebab button in CartridgeListRow is a stub compatible with 04-05 (calls onMenuAction prop)"
  - "Pre-existing prettier formatting failures in src/features/acts/*.svelte out of scope — logged as deferred"
metrics:
  duration_minutes: 28
  completed_date: "2026-06-08"
  tasks_completed: 2
  files_created: 8
  files_modified: 2
---

# Phase 04 Plan 04: Cartridges UI Skeleton Summary

Eight Svelte components + api.ts wrapper + sidebar activation for the Картриджи section. Switch-bar with 5 status tabs and counts, master-detail layout, list with empty states, detail panel with history, all wired to CartridgeService via Tauri commands from plan 04-03.

## Tasks Completed

| Task | Description | Commit | Files |
|------|-------------|--------|-------|
| 1 | api.ts + CartridgesPage + CartridgesSearchAndTabs + CartridgesMasterDetail + sidebar | eb87ed8 | api.ts, CartridgesPage.svelte, CartridgesSearchAndTabs.svelte, CartridgesMasterDetail.svelte, sidebar-config.ts, pages/CartridgesPage.svelte |
| 2 | CartridgeFilters + CartridgesList + CartridgeListRow + CartridgeDetail | 077cd47 | CartridgeFilters.svelte, CartridgesList.svelte, CartridgeListRow.svelte, CartridgeDetail.svelte |

## What Was Built

### Task 1: API + Page + Layout

**`ui/src/features/cartridges/api.ts`**
- Full wrapper around all 19 Tauri cartridge commands: list/get/create/update/delete/transition/statusCounts/getHistory/lowStock/search/modelsList/modelsGet/modelsCreate/modelsUpdate/modelsDelete/suggestBrand/suggestModel/suggestCompatPrinter/suggestLocation
- Imports types from `../../bindings` (generated from specta in plan 04-03)

**`ui/src/features/cartridges/CartridgesPage.svelte`**
- Root component: two-tab layout (Картриджи / Модели)
- State management: `$state` for items/counts/models/lowStock/filters + `$effect` hooks for reactive refresh
- LowStockBanner inline (warning color, SVG icon, per UI-SPEC §LowStockBanner)
- Models tab: placeholder for plan 04-05
- Stubs: `openCreate()` and `handleMenuAction()` are no-ops — wired in 04-05

**`ui/src/features/cartridges/CartridgesSearchAndTabs.svelte`**
- Search input with debounce 250ms
- Two tabs (Картриджи/Модели) with count badge on Картриджи tab
- Search hidden on Модели tab (search-spacer preserves height)

**`ui/src/features/cartridges/CartridgesMasterDetail.svelte`**
- Identical to ActsMasterDetail: 35/65 grid, responsive 380px breakpoint at <1100px

**`ui/src/features/layout/sidebar-config.ts`**
- Removed `phase: 4` from `/cartridges` entry — item now renders as fully active nav link

**`ui/src/pages/CartridgesPage.svelte`**
- Replaced Placeholder component with delegation to real CartridgesPage feature component

### Task 2: Filters + List + Row + Detail

**`ui/src/features/cartridges/CartridgeFilters.svelte`**
- Status switch-bar: 5 tabs (Все/На складе/В работе/На заправке/Списано) with live counts from CartridgeCountsDto
- Count-badge pattern from DeviceFilters (`.count-badge.count-active` with `--color-accent`)
- Extra filters row: native `<select>` for Тип (Все/Картридж/Фотобарабан) + Модель (from models prop)

**`ui/src/features/cartridges/CartridgesList.svelte`**
- Loading spinner (items.length=0 + loading)
- Empty state «Картриджей пока нет» with «+ Добавить картридж» button (no filter active)
- Empty state «Ничего не найдено» without action button (filter active, per UI-SPEC)
- Rows: `{#each items as c (c.id)} <CartridgeListRow />`
- Pagination footer showing `1–N из total`

**`ui/src/features/cartridges/CartridgeListRow.svelte`**
- Two-line row: code (tabular-nums, semibold) + model label (truncated) + Badge + kebab
- Badge variant mapping: 1→success, 2→accent, 3→warning, 4→default (UI-SPEC §Badge-цвета)
- location on bottom line (dash fallback `'—'`)
- Kebab 28×28 button with `aria-label="Действия с картриджем {code}"` — stub calling onMenuAction, compatible with plan 04-05

**`ui/src/features/cartridges/CartridgeDetail.svelte`**
- Empty state: «Выберите картридж» + body + «+ Добавить картридж» button
- Header: code (font-size-display, tabular-nums), model label, status badge, action buttons (disabled stubs, status-dependent)
- Информация section: расположение, holder_name (status_id=2 only), state_name, notes (2-column grid)
- История перемещений section: `{#each history}` with formatted entries
  - `formatDate(utcSeconds)` → DD.MM.YYYY format
  - `actionLabel(action)` → human-readable Russian labels
  - `parsePayloadDetails(entry)` → extracts given_by_name/given_to_name/location from payload_json (rendered as text, not innerHTML — T-04-04-02 compliant)
  - Empty history: inline «История пуста» text (not empty screen)

## Deviations from Plan

None — plan executed exactly as written.

Pre-existing prettier formatting failures in `src/features/acts/*.svelte` (6 files) existed before this plan and are out of scope per deviation rule scope boundary. Logged below.

## Deferred Items

- `src/features/acts/ActFormBody.svelte`, `ActFormItemsTable.svelte`, `DocumentAcceptanceModal.svelte`, `ReturnItemsTable.svelte`, `ReturnModal.svelte`, `returnPayload.ts` — pre-existing prettier formatting warnings; not introduced by this plan

## Known Stubs

| Stub | File | Reason |
|------|------|--------|
| `openCreate()` no-op | CartridgesPage.svelte:166 | CartridgeFormModal wired in plan 04-05 |
| `handleMenuAction()` no-op | CartridgesPage.svelte:155 | CartridgeContextMenu + CRUD/lifecycle modals in plan 04-05 |
| Action buttons `disabled` | CartridgeDetail.svelte | onClick handlers deferred to plan 04-05 per plan spec |
| Models tab | CartridgesPage.svelte:241 | ModelsList component in plan 04-05 |

These stubs are intentional per plan spec. The core goal (displaying cartridges list + detail + filters) is fully functional.

## Threat Flags

No new network endpoints, auth paths, or trust boundary crossings. AuditEntryDto.payload_json is rendered as text content only (no innerHTML) — T-04-04-02 accepted.

## Verification

- `pnpm svelte-check`: 0 errors, 16 warnings (all pre-existing from Phase 2/3 files)
- `pnpm lint` (eslint part): 0 errors; 6 prettier warnings are pre-existing acts files out of scope
- sidebar-config.ts: `/cartridges` has no `phase` property → route active in sidebar
- CartridgeListRow Badge variant: status_id=1→success, 2→accent, 3→warning, 4→default
- CartridgeDetail history: AuditEntryDto.action mapped to Russian labels, dates formatted DD.MM.YYYY

## Self-Check: PASSED

- `ui/src/features/cartridges/api.ts`: FOUND
- `ui/src/features/cartridges/CartridgesPage.svelte`: FOUND
- `ui/src/features/cartridges/CartridgesSearchAndTabs.svelte`: FOUND
- `ui/src/features/cartridges/CartridgesMasterDetail.svelte`: FOUND
- `ui/src/features/cartridges/CartridgeFilters.svelte`: FOUND
- `ui/src/features/cartridges/CartridgesList.svelte`: FOUND
- `ui/src/features/cartridges/CartridgeListRow.svelte`: FOUND
- `ui/src/features/cartridges/CartridgeDetail.svelte`: FOUND
- `ui/src/features/layout/sidebar-config.ts`: FOUND (modified)
- `ui/src/pages/CartridgesPage.svelte`: FOUND (updated)
- Task 1 commit eb87ed8: FOUND
- Task 2 commit 077cd47: FOUND
