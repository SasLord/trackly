# Phase 18: Автокомплит и дропдауны — Pattern Map

**Mapped:** 2026-07-09
**Files analyzed:** 14 (8 frontend components/utils, 6 backend/generated)
**Analogs found:** 12 / 14 (2 native-`<select>` components have no applicable overlay-dropdown analog — see note)

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `ui/src/lib/utils/portal.ts` (+ anchoring layer) | utility | event-driven (DOM position sync) | `ui/src/features/devices/DeviceContextMenu.svelte` (inline anchor logic) | role-match, exact behavior |
| `ui/src/lib/components/LocationAutocomplete.svelte` | component | request-response | itself (canonical focus-open) + `DeviceContextMenu.svelte` (portal usage) | exact (focus-open) / role-match (portal) |
| `ui/src/lib/components/PersonAutocomplete.svelte` | component | request-response | `LocationAutocomplete.svelte` (dropdown shell, near-identical CSS) | exact |
| `ui/src/features/devices/DeviceAutocompleteField.svelte` | component | request-response | `LocationAutocomplete.svelte` (focus-open) / itself (header/grouped rendering) | exact |
| `ui/src/lib/components/Select.svelte` | component | request-response (native) | **none — native `<select>`, no custom overlay** | no analog needed |
| `ui/src/lib/components/CartridgeSelect.svelte` | component | request-response (native) | **none — native `<select>`** | no analog needed |
| `ui/src/lib/components/GroupedPrinterSelect.svelte` | component | request-response (native) | **none — native `<select>`** | no analog needed |
| `ui/src/lib/components/PrinterSelect.svelte` | component | request-response (native) | **none — native `<select>`** | no analog needed |
| `ui/src/features/acts/ActFormItemsTable.svelte` | component | request-response + CRUD (row state) | `LocationAutocomplete.svelte` (focus-open), `DeviceAutocompleteField.svelte` (grouped/header dropdown), `DeviceContextMenu.svelte` (portal+anchor), itself (qty/clone semantics to preserve) | role-match, composite |
| `crates/trackly-core/src/domain/devices.rs` (`DeviceFilter`, `DeviceGroupRow`) | model | CRUD | itself (struct already has unused `name_prefix`) | exact |
| `crates/trackly-app/src/dto/device.rs` (`DeviceGroup`, `DeviceFilter`) | model/DTO | CRUD | itself | exact |
| `crates/trackly-infra/src/repos/devices_sqlite.rs::list_grouped` | service (repo) | CRUD (read/aggregate) | `::list` (dynamic optional-filter WHERE), `::search_fts` (FTS5 multi-field name+inv+serial match), `::autocomplete` (`Box<dyn ToSql>` dynamic clause builder) | role-match, strong |
| `crates/trackly-app/src/services/device_service.rs::list_grouped` | service | CRUD | itself + `::search` (FTS5 service wrapper, sibling method) | exact |
| `ui/src/bindings.ts` | config (generated) | transform | itself — **generated file, do not hand-edit** (regenerate via `cargo test export_bindings` / specta export test) | exact |
| `crates/trackly-app/tests/devices_grouping.rs` | test | CRUD | itself (existing suite, extend in place) | exact |

**Important existing capability — do not rebuild:** per-instance drill-in member details (D-06/D-07 needs) can be sourced with **zero backend changes** via the already-exported `devices.listByIds(ids)` (`ui/src/lib/api/devices.ts:53`, bindings `devicesListByIds`, service `DeviceService::list_by_ids`, repo `SqliteDeviceRepository::list_by_ids`). Call this on-demand with `g.ids` when a group is drilled into. This resolves the "Claude's Discretion" question about extending `DeviceGroup` vs. dozagruzka — dozagruzka is already wired end-to-end.

---

## Pattern Assignments

### `ui/src/lib/utils/portal.ts` (utility) + new anchoring layer

**Analog:** `ui/src/features/devices/DeviceContextMenu.svelte` (lines 24–43, 90–142, 202–216) — the only place in the codebase that already does fixed-position, portal-rendered, anchor-following UI. `CartridgeContextMenu.svelte` duplicates the same pattern (secondary reference, do not read twice — same shape).

**Current `portal.ts`** (lines 11–32) — move-only, no positioning:
```typescript
export function portal(
  node: HTMLElement,
  target: HTMLElement | string = 'body',
): { destroy(): void } {
  let targetEl: HTMLElement | null;
  if (typeof target === 'string') {
    targetEl = document.querySelector(target);
  } else {
    targetEl = target;
  }
  if (targetEl) {
    targetEl.appendChild(node);
  }
  return {
    destroy() {
      node.parentNode?.removeChild(node);
    },
  };
}
```

**Anchor computation pattern to copy** (`DeviceContextMenu.svelte` lines 24–43):
```typescript
let menuX = $state(0);
let menuY = $state(0);
let triggerEl = $state<HTMLButtonElement | null>(null);

function toggleMenu() {
  if (menuOpen) { menuOpen = false; return; }
  if (triggerEl) {
    const rect = triggerEl.getBoundingClientRect();
    menuX = rect.right - 160;
    menuY = rect.bottom + 4;
  }
  menuOpen = true;
}
```

**Reposition-on-scroll/resize pattern to copy (and DIFFER from):** `DeviceContextMenu.svelte` lines 90–94 currently **closes** the menu on scroll/resize:
```typescript
function handleScrollOrResize() {
  if (menuOpen) menuOpen = false;
}
```
```svelte
<svelte:window onmousedown={handleBodyMousedown} onscroll={handleScrollOrResize} onresize={handleScrollOrResize} />
```
**AUTO-01 (D-02) explicitly wants the opposite:** recompute `menuX`/`menuY` (or `top`/`left`/`bottom`) from `getBoundingClientRect()` on the same `scroll`/`resize` events instead of closing. Use `capture: true` on the scroll listener so scrolling *any* ancestor container (not just window) triggers reposition — plain `<svelte:window onscroll>` only fires for window-level scroll, not inner `overflow:auto` containers common in modals. Use `window.addEventListener('scroll', reposition, true)` for capture-phase.

**Portal + fixed-position CSS pattern to copy** (`DeviceContextMenu.svelte` lines 120–142, 207–216):
```svelte
{#if menuOpen}
  <div use:portal class="ctx-menu-portal" role="menu" tabindex="-1"
       style="left:{menuX}px; top:{menuY}px;">
    ...
  </div>
{/if}
```
```scss
:global(.ctx-menu-portal) {
  position: fixed;
  z-index: 2000;
  background: var(--color-surface-raised);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  box-shadow: var(--shadow-elev-1);
}
```
Note the `:global()` requirement — once a node moves into `<body>` via `use:portal`, Svelte's scoped-CSS attribute no longer applies, so styles must be `:global`.

**AUTO-01 z-index note:** UI-SPEC specifies `z-index: 1000` for autocomplete dropdowns; `DeviceContextMenu` uses `2000` for its context menu — keep dropdown z-index below context-menu z-index if both can ever be open simultaneously (unlikely but worth a comment).

---

### `ui/src/lib/components/LocationAutocomplete.svelte` (component, request-response)

**Analog:** itself — already the canonical focus-open source (per CONTEXT.md D-03). No behavior change, only swap the `.dropdown` from `position: absolute` (wrapper-relative) to portal+anchor.

**Focus-open pattern (canonical, lines 59–63) — replicate verbatim into the device picker:**
```typescript
function handleFocus() {
  // UAT-fix #3: открываем dropdown сразу на focus (empty prefix → top 20).
  suppress = false;
  scheduleFetch(value, 0);
}
```

**ArrowDown-while-closed pattern (lines 80–85):**
```typescript
if (e.key === 'ArrowDown' && !open) {
  e.preventDefault();
  suppress = false;
  scheduleFetch(value, 0);
  return;
}
```

**Current dropdown CSS to replace with portal+fixed** (lines 186–198):
```scss
.dropdown {
  position: absolute;
  top: calc(100% + 2px);
  left: 0;
  right: 0;
  z-index: 50;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  box-shadow: var(--shadow-md);
  max-height: 240px;
  overflow-y: auto;
}
```
Replace `position: absolute` / `top: calc(100% + 2px)` / `left: 0; right: 0` with `position: fixed` and JS-computed `top`/`left`/`width` per UI-SPEC AUTO-01 contract (§Позиционирование). Keep `max-height: 240px; overflow-y: auto`, `border`, `border-radius`, `box-shadow` — UI-SPEC confirms no visual change, only positioning mechanism.

---

### `ui/src/lib/components/PersonAutocomplete.svelte` (component, request-response)

**Analog:** `LocationAutocomplete.svelte` — near-identical dropdown CSS (lines 270–282), same portal-migration recipe. This component's `handleFocus` (lines 108–129) already matches the canonical DEF-1 pattern, so only the `.dropdown` CSS block needs the portal+fixed swap — no behavior change.

---

### `ui/src/features/devices/DeviceAutocompleteField.svelte` (component, request-response)

**Analog:** itself — already has `handleFocus` (lines 175–210) matching the canonical pattern, plus a **grouped/header dropdown rendering pattern** (lines 309–362) worth reusing conceptually for the device-picker's drill-in headers:
```svelte
{#if open}
  <div class="dropdown" role="listbox">
    {#if field !== 'name' && suggestions.length > 0 && (contextName || contextStatusId)}
      <header class="dropdown-header">Ранее использовалось с «{contextName}»:</header>
    {/if}
    ...
    {#if field === 'location' && allLocationSuggestions.length > 0}
      <header class="dropdown-header">Все расположения:</header>
      ...
    {/if}
  </div>
{/if}
```
This `dropdown-header` pattern (separator row inside an open list) is the closest existing analog to the AUTO-04 drill-in header (`← Назад` + group name) — same idea of a non-selectable header row above/inside the option list, styled via `.dropdown-header` (lines 423–430):
```scss
.dropdown-header {
  padding: var(--space-xs) var(--space-sm);
  font-size: var(--font-size-label);
  color: var(--color-text-secondary);
  border-bottom: 1px solid var(--color-border);
  background: var(--color-surface-sunken);
  font-style: italic;
}
```
Only the `.dropdown` CSS (lines 409–421) needs the portal+fixed swap; content/logic unchanged per UI-SPEC.

---

### `ui/src/lib/components/Select.svelte`, `CartridgeSelect.svelte`, `GroupedPrinterSelect.svelte`, `PrinterSelect.svelte` — NO ANALOG NEEDED

All four are thin wrappers around a **native `<select>`** element (confirmed by reading `Select.svelte` lines 23–47, `CartridgeSelect.svelte` lines 27–61, `GroupedPrinterSelect.svelte` lines 51–86). There is no custom absolute-positioned overlay list in any of them — the browser renders the native option popup itself, which is not subject to the modal's `overflow: hidden` clipping in the way a custom `<div class="dropdown">` is. **UI-SPEC lists these as AUTO-01 portal-migration targets, but there is nothing to portal** — the only `position: absolute` element in these files is a purely decorative 12×12 caret icon (`.caret`, non-interactive, `pointer-events: none`), not a dropdown. Flag this to the planner: either (a) confirm with the user that native-`<select>` components are out of scope for AUTO-01 (most likely correct reading of D-01's intent — "не обрезается overflow-контейнером" already holds true for native selects), or (b) if a custom listbox replacement is actually wanted here, that is a larger redesign beyond "add a positioning layer" and should be flagged back to `/gsd-discuss-phase` scope, not silently assumed.

---

### `ui/src/features/acts/ActFormItemsTable.svelte` (component, request-response + CRUD row state) — full rewrite

**Analog set:**
1. `LocationAutocomplete.svelte` — focus-open + ArrowDown-when-closed (copy verbatim, see above).
2. `DeviceAutocompleteField.svelte` — header-row-inside-dropdown pattern for drill-in.
3. `DeviceContextMenu.svelte` — portal + anchor + reposition-on-scroll (adapted to reposition, not close).
4. **Itself** — preserve existing business logic exactly:

**Existing early-return to REMOVE** (line 92, blocks AUTO-02/D-03's "empty input opens dropdown"):
```typescript
if (v.trim().length < 1) {
  suggestionsByRow[idx] = [];
  openByRow[idx] = false;
  return;
}
```

**Existing `devices.listGrouped` call to extend** (lines 100–114) — currently `group_by_condition: true` (name+model+condition key per DEF-2B); D-05 requires top-level key = name+model only, with condition as a sub-grouping inside drill-in (D-07) rather than the SQL `GROUP BY`. This is a backend contract change (see `list_grouped` section below), not just a frontend param tweak:
```typescript
const groups = await devices.listGrouped(
  {
    type_id: null,
    location_id: null,
    status_id: 1,
    state: null,
    name_prefix: v.trim(),
    include_deleted: false,
    group_by_condition: true,
  },
  { offset: 0, limit: 20 },
);
```

**DEF-2A dedup pattern to KEEP unchanged** (lines 115–118, 175–185):
```typescript
const selectedIds = getSelectedIds(idx);
const filtered = groups.filter((g) => !g.ids.some((id) => selectedIds.has(id)));
```

**Clone-qty semantics to KEEP unchanged** (lines 39–50, 160–173): `MAX_CLONE_QTY = 1000`, `qtyMax()` capping to `stock_available`, `has_serial` forcing qty=1. Do not regress these when rewriting `pickGroup`/row selection into the drill-in flow.

**Existing dropdown CSS to replace with portal+fixed** (lines 317–332) — note this one currently anchors to `top: 40px` (hardcoded row height) rather than `calc(100% + 2px)`, because it's inside a grid cell, not a wrapper with `position: relative` sized to the input:
```scss
.dropdown {
  position: absolute;
  top: 40px;
  left: 0;
  right: 0;
  max-height: 240px;
  overflow: auto;
  background: var(--color-surface-raised, var(--color-surface));
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  z-index: 10;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.08);
}
```
Replace entirely with portal + `getBoundingClientRect()`-derived fixed coordinates anchored to the row's `<Input>` element (need a per-row `inputEl` ref, currently absent — `Input.svelte` is used without `bind:this`, will need either a ref-forwarding prop or a raw `<input>` swap for this row to get the anchor element).

**New per-row state needed (not in current file):** drill-in requires a "view mode" per row (`'groups' | 'members'`) and a "current group" pointer, plus fetched member details (via `devices.listByIds(g.ids)` — see the "Important existing capability" note above) — none of this exists yet; `suggestionsByRow`/`openByRow`/`loadingByRow` (lines 64–67) are the closest existing per-row-keyed state pattern to extend.

---

### `crates/trackly-core/src/domain/devices.rs` (model)

**Analog:** itself — `DeviceFilter` (lines 46–58) already carries `name_prefix: Option<String>` and `group_by_condition: bool`, but (see repo section) `name_prefix` is **currently dead — parsed into the struct but never referenced in any SQL** in `list_grouped`. This is a pre-existing gap, not a regression to introduce.

```rust
pub struct DeviceFilter {
    pub type_id: Option<i64>,
    pub location_id: Option<i64>,
    pub status_id: Option<i64>,
    pub state: Option<String>,
    pub name_prefix: Option<String>,
    pub include_deleted: bool,
    pub group_by_condition: bool,
}

pub struct DeviceGroupRow {
    pub repr: DeviceRow,
    pub ids: Vec<i64>,
    pub count: i64,
    pub condition_distinct_count: i64,
}
```
D-04 (sort by stock count DESC) needs either a new explicit sort-mode field or simply changing `list_grouped`'s hardcoded `ORDER BY d.name` — no struct change strictly required for sorting. D-04 (filter by inv#+SN in addition to name) does need `name_prefix`'s *semantics* widened (or a new field) — Claude's Discretion per CONTEXT.md; the `AutocompleteField` enum pattern (lines 118–178, same file) shows the project's established way to whitelist/enumerate query targets if a new explicit multi-field filter type is preferred over silently widening `name_prefix`.

---

### `crates/trackly-app/src/dto/device.rs` (model/DTO)

**Analog:** itself (lines 234–249) — `DeviceGroup` DTO mirrors `DeviceGroupRow` 1:1 via specta `#[derive(Type)]`. Any domain-layer field changes must be mirrored here with matching `#[specta(type = ...)]` annotations (note `ids: Vec<i32>` DTO vs `Vec<i64>` domain — existing narrowing convention, follow it for any new fields):
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct DeviceGroup {
    pub repr: DeviceDto,
    #[specta(type = u32)]
    pub count: u64,
    pub ids: Vec<i32>,
    #[specta(type = i32)]
    pub condition_distinct_count: i64,
}
```

---

### `crates/trackly-infra/src/repos/devices_sqlite.rs::list_grouped` (repository, CRUD read/aggregate)

**Analog 1 — dynamic optional-filter WHERE clause, `::list`** (lines 544–592): shows the project's standard `(?N IS NULL OR col = ?N)` pattern for optional equality filters (`status_id`, `type_id`). This is the pattern already used inside `list_grouped`'s own `WHERE` (line 963); D-04's status filter stays as-is.

**Analog 2 — FTS5 multi-field text search, `::search_fts`** (lines 705–760) — **this is the closest existing implementation of "filter by name + inv# + SN"** that AUTO-03 needs, and it already covers `model` too:
```rust
fn search_fts(
    &self,
    conn: &Self::Conn,
    fts_query: &str,
    page: &Pagination,
) -> Result<(Vec<DeviceRow>, u64), AppError> {
    let match_expr = build_fts_query(fts_query);
    if match_expr.is_empty() {
        return Ok((Vec::new(), 0));
    }
    ...
    let mut stmt = conn.prepare(&format!(
        "SELECT ... FROM devices d
         LEFT JOIN locations l ON d.location_id = l.id
         JOIN devices_fts ON d.id = devices_fts.rowid
         WHERE devices_fts MATCH ?1
           AND d.deleted_at_utc IS NULL
         ORDER BY rank
         LIMIT {limit} OFFSET {offset}"
    ))...
}
```
**Sanitizer to reuse** (lines 75–83, `build_fts_query`):
```rust
fn build_fts_query(user_input: &str) -> String {
    user_input
        .split_whitespace()
        .map(|t| t.replace('\0', "").replace('"', "\"\""))
        .filter(|t| !t.is_empty())
        .map(|t| format!("\"{t}\"*"))
        .collect::<Vec<_>>()
        .join(" ")
}
```
The `devices_fts` virtual table (confirm columns via migration, not re-read here) already indexes `name`/`inventory_number`/`serial_number`/`model` per the doc comment on `search_fts` ("FTS5 full-text search по name/inventory_number/serial_number/model" — `device_service.rs` line 323). **Recommended approach for D-04's text filter:** join `devices_fts MATCH` into `list_grouped`'s query (or run FTS5 first to get candidate IDs, then GROUP BY over that ID set) instead of hand-rolling a new multi-column `LIKE` chain — reuses the existing tokenizer/sanitizer instead of duplicating it. Note `search_fts` does **not** currently filter by `status_id` — `list_grouped` will need to AND that in, following Analog 1's `(?N IS NULL OR ...)` pattern.

**Analog 3 — dynamic clause + params builder with `Box<dyn ToSql>`, `::autocomplete`** (lines 762–862): shows the pattern for building a variable-length WHERE clause + parameter vector when the number of optional conditions is not fixed (used there for `ctx_name`/`ctx_status_id`/`status_in`). Useful if the group-by/filter query ends up needing more than the 3 fixed placeholders `list_grouped` currently has.

**Current `list_grouped` group key to change (D-05: name+model, not name+model+condition at top level)** — lines 934–999, both SQL branches key on `GROUP BY d.type_id, d.name[, d.condition]`; need `d.model` added to the `GROUP BY`/key regardless of `group_by_condition`, with `condition` moved to a **secondary** grouping inside the per-group member expansion (handled in the drill-in fetch via `devices.listByIds`, not in this aggregate query) rather than the top-level SQL `GROUP BY`. This likely means `group_by_condition` as a filter param becomes obsolete/renamed once AUTO-04 lands — flag this to the planner as a filter-contract change, not purely additive.

**Current sort to change (D-04: count DESC, not name ASC)** — both branches end `ORDER BY d.name LIMIT ?2 OFFSET ?3` (lines 965–966, 998–999); change to `ORDER BY cnt DESC` (or `ORDER BY cnt DESC, d.name ASC` for stable tie-break).

---

### `crates/trackly-app/src/services/device_service.rs::list_grouped` (service)

**Analog:** itself (lines 492–530) — thin pass-through + pagination-limit validation (`page.limit > 200`) + `spawn_blocking` around the reader-pool `.acquire()` call. Sibling method `::search` (line ~323–onward, referenced but not fully read — same shape) is the FTS5-equivalent service wrapper if the planner adopts the `search_fts`-join approach above; read it directly when implementing (not pre-read here per non-overlap discipline, but it is guaranteed structurally identical to `list_grouped`'s wrapper — same `spawn_blocking`/`readers.clone()`/`repo.clone()` shape).

```rust
pub async fn list_grouped(
    &self,
    filter: DeviceFilter,
    page: Pagination,
) -> Result<Vec<DeviceGroup>, AppError> {
    if page.limit > 200 {
        return Err(AppError::Validation {
            field: "pagination.limit".to_string(),
            message: "Максимальный размер страницы — 200".to_string(),
        });
    }
    let readers = self.readers.clone();
    let repo = self.repo.clone();
    let domain_filter = trackly_core::domain::devices::DeviceFilter { ... };
    let domain_page = trackly_core::domain::devices::Pagination { ... };
    let group_rows = tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        repo.list_grouped(&conn, &domain_filter, &domain_page)
    })
    .await
    .map_err(|e| AppError::Internal { source_chain: format!("spawn_blocking: {e}") })?;
    ...
}
```
No structural change needed here beyond mirroring whatever new/changed `DeviceFilter` fields are added at the domain layer (line ~510–518 field-by-field copy).

---

### `ui/src/bindings.ts` (generated — do not hand-edit)

**Analog:** itself. This file is generated by `tauri-specta` from the Rust DTOs/commands (confirmed by `crates/trackly-app/tests/export_bindings.rs`, which asserts commands like `devices_list_grouped`/`devices_list_by_ids` appear in the generated output). Any `DeviceGroup`/`DeviceFilter` shape change must be made in `crates/trackly-app/src/dto/device.rs` first, then regenerated (existing project convention — checked by `export_bindings.rs`, run via the crate's test suite) — do not hand-patch `bindings.ts`.

Current relevant excerpts (for reference, not to copy-paste as source of truth once regenerated):
```typescript
// bindings.ts:95-102
async devicesListGrouped(filter: DeviceFilter, pagination: Pagination) : Promise<Result<DeviceGroup[], AppError>> { ... }
// bindings.ts:1532-1541
export type DeviceFilter = { type_id: number | null; location_id: number | null; status_id: number | null; state: string | null; name_prefix: string | null;
include_deleted: boolean;
group_by_condition: boolean }
// bindings.ts:1549-1554
export type DeviceGroup = { repr: DeviceDto; count: number; ids: number[];
condition_distinct_count: number }
```

---

### `crates/trackly-app/tests/devices_grouping.rs` (test)

**Analog:** itself — existing suite (17 tests referenced by line numbers 45–773) already covers grouping-collapse, status filter, `list_by_ids`, and a documented regression ("list_grouped SQL previously omitted inventory_number/serial_number", line 299). Follow its established scaffolding exactly for new AUTO-04/AUTO-03/AUTO-04 tests:

```rust
use std::sync::Arc;
use std::time::Duration;
use trackly_app::dto::device::{DeviceFilter, DeviceNew, Pagination};
use trackly_app::services::DeviceService;
use trackly_core::primitives::clock::Clock;
use trackly_infra::clock_impl::SystemClock;
use trackly_infra::test_support::test_writer_and_readers;

fn make_service() -> (DeviceService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock: Arc<dyn Clock + Send + Sync> = Arc::new(SystemClock);
    let svc = DeviceService::new(writer, readers, clock);
    (svc, dir)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn grouping_collapses_non_unique() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (svc, _dir) = make_service();
        ...
        let groups = svc.list_grouped(filter, page).await.expect("list_grouped");
        ...
    })
    .await
    .expect("... exceeded 30s");
}
```
Note the `tokio::time::timeout(Duration::from_secs(30), ...)` wrapper on every async test — established project convention to avoid hangs, keep it in new tests. New tests needed: name+model grouping (two devices, same name, different model → 2 groups, not 1), sort-by-count-desc ordering, filter matching inv#/SN (not just name prefix).

---

## Shared Patterns

### Focus-open dropdown (DEF-1/D-03)
**Source:** `ui/src/lib/components/LocationAutocomplete.svelte` lines 59–63, 80–85
**Apply to:** `ActFormItemsTable.svelte` device picker (currently missing `onfocus`)
```typescript
function handleFocus() {
  suppress = false;
  scheduleFetch(value, 0);
}
// keydown:
if (e.key === 'ArrowDown' && !open) {
  e.preventDefault();
  suppress = false;
  scheduleFetch(value, 0);
  return;
}
```

### Portal + fixed-position anchoring (AUTO-01/D-01/D-02)
**Source:** `ui/src/lib/utils/portal.ts` (move-only action) + `ui/src/features/devices/DeviceContextMenu.svelte` lines 24–43, 90–142 (anchor math + portal usage — reposition instead of close for D-02)
**Apply to:** `LocationAutocomplete`, `PersonAutocomplete`, `DeviceAutocompleteField`, `ActFormItemsTable` device-picker dropdown (NOT the four native-`<select>` components — see "no analog needed" above)

### Click-outside-to-close
**Source:** `LocationAutocomplete.svelte` lines 107–114 (identical in `PersonAutocomplete.svelte` lines 173–182 and `DeviceAutocompleteField.svelte` lines 262–271)
**Apply to:** all portal-migrated dropdowns — pattern is unaffected by the portal move since it listens on `document`, not the wrapper:
```typescript
function handleClickOutside(e: MouseEvent) {
  if (wrapperEl && !wrapperEl.contains(e.target as Node)) open = false;
}
$effect(() => {
  document.addEventListener('mousedown', handleClickOutside);
  return () => document.removeEventListener('mousedown', handleClickOutside);
});
```
**Caveat for portal migration:** once the dropdown itself moves into `<body>`, `wrapperEl.contains(e.target)` will be `false` for clicks *inside* the portaled dropdown (it's no longer a DOM descendant of `wrapperEl`). Must also check `!dropdownEl.contains(e.target)` (need a second bound element ref) or the dropdown will incorrectly close on its own option clicks that don't call `e.preventDefault()`/`stopPropagation` (existing `onmousedown={(e) => { e.preventDefault(); select(s); }}` on option buttons already prevents focus-loss, but the outside-click handler still needs the second ref for correctness after the DOM move).

### Existing FTS5 multi-field search (name+inv#+SN+model)
**Source:** `crates/trackly-infra/src/repos/devices_sqlite.rs::search_fts` (lines 705–760) + `build_fts_query` sanitizer (lines 75–83)
**Apply to:** `list_grouped`'s new text filter (AUTO-03/D-04) — reuse the sanitizer and `devices_fts MATCH` join rather than writing a new multi-column `LIKE` chain.

### Dynamic optional-WHERE-clause building
**Source:** `crates/trackly-infra/src/repos/devices_sqlite.rs::list` (lines 559–563, `(?N IS NULL OR col = ?N)`) and `::autocomplete` (lines 779–842, `Box<dyn ToSql>` vector for variable-arity clauses)
**Apply to:** `list_grouped`'s status_id + new text-filter combination.

### Per-instance detail fetch without backend change
**Source:** `ui/src/lib/api/devices.ts` line 53 (`listByIds`) → bindings `devicesListByIds` → `DeviceService::list_by_ids` → `SqliteDeviceRepository::list_by_ids`
**Apply to:** AUTO-04 drill-in (D-06/D-07) — call `devices.listByIds(g.ids)` when a group row is clicked, to get full `DeviceDto[]` (serial_no/inventory_no/state per instance) for rendering the drill-in list.

---

## No Analog Found

| File | Role | Data Flow | Reason |
|---|---|---|---|
| `ui/src/lib/components/Select.svelte`, `CartridgeSelect.svelte`, `GroupedPrinterSelect.svelte`, `PrinterSelect.svelte` | component | request-response (native) | These wrap a native `<select>`; there is no custom overlay to portal — UI-SPEC's inclusion of them under AUTO-01 should be re-confirmed with the user/planner before any code change is scoped here (see full note above). |
| New drill-in "view state" (`groups` vs `members` per row) in `ActFormItemsTable.svelte` | component state | event-driven (UI state machine) | No existing multi-level/master-detail dropdown exists anywhere in the codebase to copy from; `DeviceAutocompleteField`'s header-row pattern is the closest partial match (a static, non-navigable header, not a replace-the-list drill-in), noted above but not an exact analog. |

---

## Metadata

**Analog search scope:** `ui/src/lib/components/`, `ui/src/lib/utils/`, `ui/src/features/{acts,devices,cartridges}/`, `crates/trackly-core/src/domain/devices.rs`, `crates/trackly-core/src/ports/devices.rs`, `crates/trackly-infra/src/repos/devices_sqlite.rs`, `crates/trackly-app/src/{dto,services,tauri_cmds,http}/device*.rs`, `crates/trackly-app/tests/devices_grouping.rs`
**Files scanned:** 17 (11 frontend, 6 backend/test)
**Pattern extraction date:** 2026-07-09
