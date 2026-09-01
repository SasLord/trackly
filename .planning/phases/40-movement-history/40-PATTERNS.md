# Phase 40: История перемещений - Pattern Map

**Mapped:** 2026-09-01
**Files analyzed:** ~28 (new + modified, Rust + Svelte)
**Analogs found:** 28 / 28 (every file has at least a role-match analog; none in "No Analog Found")

**Privacy check:** all excerpts below are read from the current, already-committed codebase.
None contain real organization data or real personal names. `CartridgeTransitionOp`/act code
excerpts use only field names and placeholder/domain literals (status ids, place ids). Where the
UI-SPEC's own timeline example string appears (`Здание А / 2 эт. / 214 → Склад · Иванов И.И. ·
актом №123`), it is CONTEXT.md's own explicitly-invented example, not real data — reproduced
verbatim here only because it is the canonical row-format contract.

---

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `migrations/V040__place_movements.sql` | migration | CRUD (schema) | `migrations/V039__place_path_display.sql` | exact |
| `crates/trackly-core/src/domain/place_movements.rs` (new, domain types: `MovementSource`, row struct) | model | transform | `crates/trackly-core/src/domain/places.rs` (`PathDisplayVariant`, `shorten_place_path`) | exact |
| `crates/trackly-infra/src/repos/place_movements_sqlite.rs` (new repo) | model/repository | CRUD + streaming-read | `crates/trackly-infra/src/repos/cartridges_sqlite.rs` (`get_history`, `transition_in_tx`) | exact |
| `crates/trackly-app/src/services/place_movement_service.rs` (new, OR folded into existing services — planner's call) | service | CRUD + request-response | `crates/trackly-app/src/services/cartridge_service.rs` (`get_history`) + `place_service.rs` (`caller: &Identity` shape) | exact |
| `crates/trackly-app/src/services/device_service.rs::update` (modified) | service | CRUD | itself (before/after diff already present — this is the "receiver" of new logic, not a fresh clone) | exact (self) |
| `crates/trackly-app/src/services/cartridge_service.rs::update` (modified) | service | CRUD | `device_service.rs::update` (needs the SAME before-fetch this method currently lacks) | role-match, gap noted |
| `crates/trackly-infra/src/repos/cartridges_sqlite.rs::transition_in_tx` (modified, 2 call sites) | repository | CRUD | itself + its own nested auto-return branch (2nd call site) | exact (self, twice) |
| `crates/trackly-app/src/services/act_service.rs::create/update/do_return/update_return` (modified, 6 call sites) | service | CRUD | itself (`update_status_and_place_in_tx` / `update_full_in_tx` call sites already diff before/after) | exact (self) |
| `crates/trackly-app/src/services/act_service.rs::delete_soft` (modified) + `undo_device_mutations_for_act` | service | event-driven (undo/compensation) | itself — `delete_soft`'s existing LIFO cascade loop is the insertion point | exact (self) |
| `compute_place_path_short` — promoted from `act_service.rs` to a new shared module (planner names it, e.g. `crates/trackly-app/src/services/place_path_display.rs`) | utility | transform | `crates/trackly-infra/src/repos/place_path_settings.rs` (single-owner module pattern, WR-08) | role-match |
| `crates/trackly-app/src/services/place_service.rs::move_subtree_contents` (new, D-28 bulk move) | service | batch (CRUD, one tx, N rows) | `place_service.rs::list_subtree_contents` (read side, same scope) + `move_node` (mutation shape) | exact |
| `crates/trackly-app/src/dto/reports.rs::ReportFilter` (add `from_place_id`/`to_place_id`) | model (DTO) | transform | itself — additive fields, same struct 12 reports already share | exact (self) |
| `crates/trackly-app/src/dto/reports.rs::ReportRow` (add `from_place_path[_short]`, `actor_name`, `reason`/`deleted`) | model (DTO) | transform | itself — sparse-struct convention | exact (self) |
| `crates/trackly-app/src/dto/place_movements.rs` (new — timeline DTO, e.g. `MovementEntryDto`) | model (DTO) | transform | `crates/trackly-app/src/dto/cartridge.rs::AuditEntryDto` (explicitly NOT to be reused/extended — new flat DTO) | role-match, explicit divergence noted |
| `crates/trackly-app/src/services/report_service.rs::list_movements` (13th `list_*`) | service | request-response (CRUD read) | `report_service.rs::list_device_acts` + `query_acts_inner` | exact |
| `crates/trackly-app/src/services/report_service.rs::row_field` (new match arms) | utility | transform | itself — existing `match` on column-name strings | exact (self) |
| `crates/trackly-app/src/tauri_cmds/reports.rs::columns_for/column_labels_for/report_display_name` (new `"movements"` arm) | config | transform | itself — 3 parallel index-aligned matches | exact (self) |
| `crates/trackly-app/src/tauri_cmds/reports.rs::build_reports_list_movements` (new) | route/controller | request-response | `build_reports_list_device_acts` — **but gate diverges**, see Shared Patterns | role-match, gate divergence noted |
| `crates/trackly-app/src/http/reports.rs::handler_list_movements` (new) | route/controller | request-response | `handler_list_device_acts` | exact |
| `crates/trackly-app/src/tauri_cmds/place_movements.rs` (new, timeline read commands) | route/controller | request-response | `crates/trackly-app/src/tauri_cmds/places.rs` (`build_places_list_subtree_contents`) | exact |
| `crates/trackly-app/src/http/place_movements.rs` (new, timeline read handlers) | route/controller | request-response | `crates/trackly-app/src/http/devices.rs::handler_update` (session→identity→build_* delegation shape) | exact |
| `crates/trackly-app/tests/role_endpoint_matrix.rs` (new Cases, movements read + report) | test | request-response | Cases 45–48 (`ReadPlaces` four-part shape) | exact |
| `crates/trackly-infra/tests/place_movements_migration.rs` (new, or extend `migration_idempotency.rs`) | test | CRUD (schema) | existing V037–V039 idempotency test pattern | role-match |
| `ui/src/features/places/PlaceEntityViewModal.svelte` (modified — add timeline `DetailSection`) | component | request-response | itself (existing modal shell, single `$effect` fetch) | exact (self) |
| `ui/src/features/cartridges/CartridgeDetail.svelte` (modified — rename + new section) | component | request-response | itself, `DetailSection heading="История перемещений"` at line ~192 | exact (self) |
| `ui/src/features/printers/PrinterDetail.svelte` (modified — reads same timeline by device id) | component | request-response | `PlaceEntityViewModal`'s device-timeline consumption | role-match |
| `ui/src/features/devices/DeviceContextMenu.svelte` (modified — new «Просмотр» item) | component | event-driven (menu action) | itself — existing `ctx-menu-item` + confirm-`Modal` pattern (`openConfirm`/`Удалить`) | exact (self) |
| `ui/src/features/reports/ReportSubNav.svelte` (modified — 4th `DOMAINS` entry) | component/provider | transform (nav config) | itself — `DOMAINS` array + per-domain `ReportConfig[]` | exact (self) |
| `ui/src/features/reports/ReportFilters.svelte` (modified — two `PlacePicker`s) | component | event-driven (filter change) | itself — existing single `.place-filter-group`/`PlacePicker` block | exact (self) |
| `ui/src/features/reports/ReportTable.svelte` (modified — new columns) | component | transform | itself — existing `place_path`/`place_path_short` + `title=` cell convention | exact (self) |
| `ui/src/features/places/PlaceContents.svelte` (modified — bulk-move action + confirm) | component | event-driven | itself (`.crumb` link pattern) + `DeviceContextMenu.svelte` (confirm-`Modal` pattern) | exact (self) + role-match |
| `ui/src/features/acts/ActsPage.svelte` (modified — accept `?id=` hash query, D-19 discretion #6) | component | event-driven (routing) | `DevicesPage.svelte`/`CartridgesPage.svelte`'s existing `parseIdFromHash` consumption | role-match |

---

## Pattern Assignments

### `migrations/V040__place_movements.sql`

**Analog:** `migrations/V039__place_path_display.sql` (full file read: header doc-comment block,
`ALTER TABLE`, `INSERT INTO app_settings` seed, recursive-CTE view) and `migrations/V008__audit_log.sql`
(append-only shape — no `deleted_at_utc`, no `version`, hard-delete only).

**Header doc-comment convention** (V039 lines 1–23) — every migration in this repo opens with a
comment explaining *why*, citing the specific decisions it encodes and which prior migration/module
it mirrors rather than inventing shape:
```sql
-- V039: place-path display format (Phase 39.1, PLC-07/PLC-08).
--
-- Moves the "how much of a place's path to show" choice ... (D-01..D-04).
--
-- Mirrors two existing conventions rather than inventing new ones:
--   - `migrations/V037__places.sql` (`place_full_paths`) for ...
--   - `migrations/V016__cartridges_kind_color_settings.sql` for the
--     `app_settings` key/value seeding pattern ...
```
V040's own header should cite: D-01 (own table, not `audit_log`), D-06/D-07/D-09/D-10 (column
shapes), and explicitly name `migrations/V008__audit_log.sql` as the append-only-shape precedent
(no `deleted_at_utc`/`version` columns — this is a journal, not an editable entity).

**Unconstrained-enum-token convention** (V039 line 18–23, applies verbatim to `source`):
```sql
-- `path_variant_override` is intentionally NOT constrained by a SQLite CHECK
-- clause enumerating 'ends'/'last_two'/'last' — same choice already made for
-- `places.kind` (V037), which validates its token set in Rust
-- (`PlaceKind::from_str`) rather than in SQL.
```
`source` (`manual`/`act`/`map`/`workstone`) should follow this exact precedent: bare `TEXT`
column, Rust-side `MovementSource::from_str` as the single parse point (see Common/Shared
Patterns below re: Pitfall 6 soft-degradation).

**Full RESEARCH-proposed schema** (already vetted against D-01/D-06/D-07/D-09/D-10 and the two
undo-scoping indexes) — reproduce verbatim as the starting point, confirm names with planner:
```sql
CREATE TABLE place_movements (
  id                  INTEGER PRIMARY KEY AUTOINCREMENT,
  entity_type         TEXT    NOT NULL,   -- 'device' | 'cartridge' (D-21: printer is 'device')
  entity_id           INTEGER NOT NULL,
  from_place_id       INTEGER NOT NULL REFERENCES places(id) ON DELETE RESTRICT,
  from_place_path     TEXT    NOT NULL,
  to_place_id         INTEGER NOT NULL REFERENCES places(id) ON DELETE RESTRICT,
  to_place_path       TEXT    NOT NULL,
  source              TEXT    NOT NULL,   -- 'manual' | 'act' | 'map' | 'workstation'
  note                TEXT    NULL,
  act_id              INTEGER NULL REFERENCES acts(id) ON DELETE SET NULL,
  user_id             INTEGER NULL REFERENCES users(id) ON DELETE SET NULL,
  actor_name_snapshot TEXT    NULL,
  created_at_utc      INTEGER NOT NULL
);
CREATE INDEX idx_place_movements_entity ON place_movements(entity_type, entity_id, created_at_utc DESC);
CREATE INDEX idx_place_movements_created ON place_movements(created_at_utc);
CREATE INDEX idx_place_movements_from_place ON place_movements(from_place_id);
CREATE INDEX idx_place_movements_to_place   ON place_movements(to_place_id);
CREATE INDEX idx_place_movements_act        ON place_movements(act_id) WHERE act_id IS NOT NULL;
PRAGMA user_version = 40;
```
**Next free migration number confirmed: `V040`** (`migrations/` directory listing ends at
`V039__place_path_display.sql`).

---

### `crates/trackly-infra/src/repos/place_movements_sqlite.rs` (new repo)

**Analog:** `crates/trackly-infra/src/repos/cartridges_sqlite.rs` — both the read-side `get_history`
and the write-side `transition_in_tx`.

**Read-side SQL to clone verbatim (shape)** — `cartridges_sqlite.rs:1105-1121`:
```rust
/// Cartridge history from audit_log (D-History-01, CART-10).
pub fn get_history(
    &self,
    conn: &Connection,
    cartridge_id: i64,
) -> Result<Vec<AuditEntryRow>, AppError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, entity_type, entity_id, action, user_id, \
                    before_json, after_json, payload_json, created_at_utc \
               FROM audit_log \
              WHERE entity_type = 'cartridge' \
                AND entity_id = ?1 \
                AND action NOT IN ('list', 'get') \
              ORDER BY created_at_utc DESC, id DESC",
        )
        .map_err(map_rusqlite)?;
    let rows = stmt.query_map(params![cartridge_id], |r| { /* ... */ }).map_err(map_rusqlite)?;
    let mut out = Vec::new();
    for row in rows { out.push(row.map_err(map_rusqlite)?); }
    Ok(out)
}
```
For `place_movements`, clone the `ORDER BY created_at_utc DESC, id DESC` (D-20, "newest first, no
pagination") and the `WHERE entity_type = ? AND entity_id = ?1` scoping verbatim — new table has
its own dedicated columns instead of `audit_log`'s JSON blobs, so `SELECT` picks named columns
(`from_place_path`, `to_place_path`, `source`, `note`, `act_id`, `actor_name_snapshot`,
`user_id`, `created_at_utc`) directly, no `payload_json` parsing needed.

**Write-side transaction pattern to clone** — `transition_in_tx` signature and its "insert audit
row via the shared repo, inside the caller's `tx`" convention (`cartridges_sqlite.rs:458-464`,
`674-687`):
```rust
pub fn transition_in_tx(
    &self,
    tx: &Transaction<'_>,
    cartridge_id: i64,
    version: i64,
    op: &CartridgeTransitionOp,
    now_utc: i64,
) -> Result<(), AppError> { /* ... */ }
```
A new `pub fn insert_in_tx(&self, tx: &Transaction<'_>, movement: NewMovement) -> Result<(), AppError>`
on the new `place_movements` repo should take the SAME `&Transaction<'_>` (never opens its own —
D-01 requires it share the mutation's transaction) and be called from inside each of the six
write-site methods below, mirroring how `audit_repo.insert(tx, AuditEntry { ... })` is called
inline from `transition_in_tx`.

---

### The six existing write sites — before/after diff + transaction pattern per site

**1. `device_service.rs::update`** (lines 258-303, already read in full) — before/after diff
ALREADY exists; this is the cleanest insertion point, needs only the new insert call added after
`repo.update_in_tx`:
```rust
// crates/trackly-app/src/services/device_service.rs:258-303 (excerpt)
pub async fn update(&self, id: i64, version: i64, patch: DevicePatch) -> Result<DeviceDto, AppError> {
    let now = self.clock.unix_seconds();
    let repo = self.repo.clone();
    let printer_repo = self.printer_repo.clone();
    let domain_patch: trackly_core::domain::devices::DevicePatch = patch.into();
    let user_id_opt: Option<i64> = None;   // <-- Pitfall 1: must become `caller.user_id`

    let updated_row = self.writer.execute(move |conn| {
        let tx = conn.transaction().map_err(map_rusqlite)?;
        let before = repo.get_in_tx(&tx, id).ok();          // <-- before.place_id available here
        let before_json = /* ... */;
        let after = repo.update_in_tx(&tx, id, version, &domain_patch, now)?;  // <-- after.place_id here
        Self::sync_printer_row_in_tx(&printer_repo, &tx, id, after.type_id, now)?;
        let after_json = /* ... */;
        tx.execute("INSERT INTO audit_log (...) VALUES (...)", /* ... */).map_err(map_rusqlite)?;
        // <-- INSERT INTO place_movements HERE, guarded by:
        //     before.place_id != after.place_id AND both Some (D-06)
        tx.commit().map_err(map_rusqlite)?;
        Ok(after)
    }).await?;
    Ok(DeviceDto::from(updated_row))
}
```

**2. `cartridge_service.rs::update`** (lines 187-224) — **Pitfall 2: no before-fetch exists today**.
Current code (excerpt):
```rust
// crates/trackly-app/src/services/cartridge_service.rs:187-207 (excerpt)
pub async fn update(&self, id: i64, version: i64, place_id: Option<i64>, notes: Option<String>) -> Result<CartridgeDto, AppError> {
    let now = self.clock.unix_seconds();
    let audit_repo = self.audit_repo.clone();
    self.writer.execute(move |conn| {
        let tx = conn.transaction().map_err(map_rusqlite)?;
        let affected = tx.execute(
            "UPDATE cartridges SET place_id=?1, notes=?2, updated_at_utc=?3, version=version+1 \
             WHERE id=?4 AND version=?5 AND deleted_at_utc IS NULL",
            params![place_id, notes, now, id, version],
        ).map_err(map_rusqlite)?;
        // ... optimistic-lock error handling only, NO before-state captured
```
Task must add a `SELECT place_id FROM cartridges WHERE id = ?1` (or `RETURNING`) BEFORE this
`UPDATE`, exactly the shape `device_service.rs` already has via `repo.get_in_tx`.

**3 & 4. `cartridge_service.rs::transition` → `SqliteCartridgeRepository::transition_in_tx`**
(`crates/trackly-infra/src/repos/cartridges_sqlite.rs:458-566` main mutation, `568-680` nested
auto-return) — TWO separate movement-insert call sites in the SAME function:
```rust
// Main mutation — current.place_id (before) vs new_place_id (after) both
// already local variables at this point (cartridges_sqlite.rs:504-566):
let affected = tx.execute(
    "UPDATE cartridges SET status_id=?1, state_id=?2, place_id=?3, ... \
     WHERE id=?7 AND version=?8",
    params![new_status_id, new_state_id, new_place_id, /* ... */],
).map_err(map_rusqlite)?;
// <-- movement insert #1 HERE: current.place_id -> new_place_id

// Nested auto-return branch (cartridges_sqlite.rs:568-680), fires when
// Install finds another cartridge already "В работе" in the target printer:
if let Some((prev_id, prev_version)) = previous {
    let prev_current = self.fetch_in_tx(tx, prev_id)?;   // <-- before, for prev_id
    let prev_affected = tx.execute(
        "UPDATE cartridges SET status_id=1, state_id=?1, place_id=?2, ... \
         WHERE id=?4 AND version=?5",
        params![resolved_state_id, resolved_place_id, /* ... */],
    ).map_err(map_rusqlite)?;
    // <-- movement insert #2 HERE: prev_current.place_id -> resolved_place_id,
    //     entity_id = prev_id (NOT cartridge_id) — separate row, separate entity
    audit_repo.insert(tx, AuditEntry { entity_type: "cartridge", entity_id: prev_id, /* ... */ })?;
}
```
Pitfall 3 (RESEARCH) names this exact spot as the easy-to-miss second site.

**5. `act_service.rs::create`/`update` (handover)** — `update_status_and_place_in_tx` call site
(lines 442-486, `act_id` in scope):
```rust
// crates/trackly-app/src/services/act_service.rs:442-486 (excerpt)
for &dev_id in &effective_device_ids {
    let before = devices_repo.get_in_tx(&tx, dev_id)?;         // <-- before.place_id
    let before_json = device_snapshot_json(&before)?;
    let after = devices_repo.update_status_and_place_in_tx(
        &tx, dev_id, in_work_status_id, resolved_place_id, now,
    )?;                                                          // <-- after.place_id = resolved_place_id
    let after_json = device_snapshot_json(&after)?;
    let payload_json = serde_json::json!({ "act_id": act_id, "kind": "handover" });
    // <-- movement insert HERE: before.place_id -> resolved_place_id, source='act', act_id=act_id
```
`update` (line ~718) has the identical shape at its own `update_status_and_place_in_tx` call.

**6. `act_service.rs::do_return`/`update_return`** — `update_full_in_tx` call sites (lines
~1408, ~1942, ~2011). **Pitfall 4 is load-bearing here**: `effective_location` CAN be `None`
(DEF-3, lines 1307-1312), which per D-06 means "skip the insert, do not write `place_id = NULL`":
```rust
// crates/trackly-app/src/services/act_service.rs:1307-1312 (comment, DEF-3)
// DEF-3: если effective_place=None, update_full_in_tx запишет
// NULL в place_id. Caller обязан передать bulk_place_id или
// place_id_override для восстановления расположения при возврате.
...
// :1395-1420 (excerpt)
for &device_id in &dids {
    let before = devices_repo.get_in_tx(&tx, device_id)?;      // <-- before.place_id
    acts_repo.insert_act_item_in_tx(&tx, return_act_id, device_id, /* ... */)?;
    let after = devices_repo.update_full_in_tx(
        &tx, device_id, on_warehouse_status_id, effective_location, effective_condition.as_deref(), now,
    )?;                                                          // <-- after.place_id = effective_location (maybe None!)
    // <-- movement insert HERE, GUARDED: only if before.place_id.is_some()
    //     AND after.place_id.is_some() AND they differ (D-06)
```

---

### `act_service.rs::delete_soft` + `undo_device_mutations_for_act` — D-03 deletion point

**Analog:** the existing cascade loop itself (`act_service.rs:2422-2470`), which already does a
per-act soft-delete + audit-insert pair for each cascaded return, then the handover:
```rust
// crates/trackly-app/src/services/act_service.rs:2422-2470 (excerpt, current code)
match act.act_type {
    ActType::Handover => {
        let returns = acts_repo.list_returns_for_parent_in_tx(&tx, id)?;
        for ret in returns.iter().rev() {                       // LIFO
            undo_device_mutations_for_act(&tx, &devices_repo, &audit_repo, ret.id, user_id_opt, now)?;
            acts_repo.soft_delete_in_tx(&tx, ret.id, ret.version, now)?;
            // <-- ADD: tx.execute("DELETE FROM place_movements WHERE act_id = ?1", [ret.id])?
            audit_repo.insert(&tx, AuditEntry { entity_type: "act", entity_id: ret.id, action: "delete", .. })?;
        }
        // ... then, after the loop, the handover's own soft-delete:
        // undo_device_mutations_for_act(&tx, ..., id, ...)?;
        // acts_repo.soft_delete_in_tx(&tx, id, version, now)?;
        // <-- ADD: tx.execute("DELETE FROM place_movements WHERE act_id = ?1", [id])?
```
Per Pitfall 5: the `DELETE FROM place_movements WHERE act_id = ?` MUST sit immediately alongside
EACH act's own `soft_delete_in_tx` call inside the existing loop (once per cascaded return, once
for the handover) — not as one blanket delete at the end of `delete_soft`. No new function is
needed; `act_id` being a first-class column (not buried JSON, unlike `audit_log`) is what makes
this a one-line `DELETE`, per D-01's stated rationale.

`undo_device_mutations_for_act` itself (lines 3073-3110) is NOT modified — D-03 explicitly rejects
walking its LIFO restore loop; the plain `act_id`-scoped `DELETE` above is sufficient and simpler.

---

### `caller: &Identity` threading — analog for the six write-site signature changes

**Analog (a method that ALREADY takes `caller: &Identity` end-to-end, both transports):**
`crates/trackly-app/src/services/place_service.rs::create` (and every other `PlaceService`
method) + `crates/trackly-app/src/tauri_cmds/devices.rs::build_devices_update` (currently drops
`caller` — the exact gap to close) + `crates/trackly-app/src/http/devices.rs::handler_update`
(the HTTP side of the SAME gap).

**Service-layer shape to copy** (`place_service.rs:148-160`):
```rust
pub async fn create(&self, caller: &Identity, new: PlaceNew) -> Result<PlaceDto, AppError> {
    authorize(caller, &Action::MutatePlaces)?;
    Self::validate_name(&new.name)?;
    let now = self.clock.unix_seconds();
    let user_id = caller.user_id;          // <-- extracted BEFORE moving into the writer closure
    let repo = self.repo.clone();
    let audit_repo = self.audit_repo.clone();
    let row: PlaceRow = self.writer.execute(move |conn| {
        // ... user_id is moved into the closure, used for AuditEntry.user_id
    }).await?;
    Ok(PlaceDto::from(row))
}
```
This is the exact fix for the `user_id_opt: Option<i64> = None` lines in `device_service.rs:165`
and `:1031` (per RESEARCH Pitfall 1) — extract `caller.user_id` in the async fn body (outside the
`writer.execute` closure, since `Identity` itself is not `Send`-safe to move as-is into a
`'static` closure the same way an already-copied `Option<i64>` is), pass it into the closure by
value.

**Adapter-layer gap being closed** (`crates/trackly-app/src/tauri_cmds/devices.rs:57-66`, current
code — `caller` used ONLY for `authorize`, then dropped):
```rust
/// Мутация: требует `caller` с правом `MutateDevices`.
pub async fn build_devices_update(
    ctx: &AppCtx, caller: &Identity, id: i64, version: i64, patch: DevicePatch,
) -> Result<DeviceDto, AppError> {
    authorize(caller, &Action::MutateDevices)?;
    ctx.devices.update(id, version, patch).await    // <-- caller dropped here; must become
                                                       //     ctx.devices.update(caller, id, version, patch)
}
```
Both transports already resolve a real `Identity` before this point and need NO change beyond the
call-site argument: Tauri via `resolve_tauri_identity` (used throughout `tauri_cmds/devices.rs`),
HTTP via `session_identity(&session)` in `crates/trackly-app/src/http/devices.rs::handler_update`:
```rust
// crates/trackly-app/src/http/devices.rs:176-189 (excerpt, current code)
pub async fn handler_update(
    State(ctx): State<AppCtx>, session: Session, Json(payload): Json<UpdatePayload>,
) -> Result<Json<DeviceDto>, AppErrorResponse> {
    let identity = session_identity(&session).await /* ... */;
    Ok(Json(
        build_devices_update(&ctx, &identity, payload.id, payload.version, payload.patch)
            .await.map_err(AppErrorResponse::from)?,
    ))
}
```
This handler needs ZERO changes — it already passes `&identity` through to `build_devices_update`;
only `build_devices_update` and `DeviceService::update`'s signatures/bodies change. Apply the
identical shape to `cartridge_service.rs::update`/`transition` and all four `ActService` mutation
methods, and their respective `build_cartridges_*`/`build_acts_*` adapters in both
`tauri_cmds/*.rs` and `http/*.rs`.

---

### New "movements" report — end-to-end clone of the 12-report pattern

**Analog chain (all 5 layers read in full for one report, `list_device_acts`):**

**1. `ReportFilter` — additive fields** (`crates/trackly-app/src/dto/reports.rs:22-61`, full
struct read):
```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct ReportFilter {
    #[specta(type = Option<i32>)] pub date_from_utc: Option<i64>,
    #[specta(type = Option<i32>)] pub date_to_utc: Option<i64>,
    #[specta(type = Option<i32>)] pub place_id: Option<i64>,   // existing single-place filter
    // ... status_id, type_id, act_type, model_id, color, search,
    //     request_category_filter, is_storage — all existing, ignorable by this report
    // ADD per D-24: from_place_id: Option<i64>, to_place_id: Option<i64>
}
```

**2. `ReportRow` — additive fields** (`reports.rs:69-113`, full struct read) — sparse, only
relevant fields populated per report type:
```rust
pub struct ReportRow {
    #[specta(type = i32)] pub id: i64,
    pub month_key: Option<String>,
    pub number: Option<String>,
    // ... device_name, code, status_name, etc. — all reused as-is by the movements report
    pub place_path: Option<String>,        // REUSE for "to" side (D-23's «Куда»)
    pub place_path_short: Option<String>,  // REUSE for "to" side, shortened
    // ADD: from_place_path: Option<String>, from_place_path_short: Option<String>
    // ADD: actor_name: Option<String>  (D-11's ФИО/login/«система»)
    // ADD: reason: Option<String> OR (source + note + act_number as 3 separate fields — planner's call)
    // ADD (D-25): deleted marker, e.g. `is_deleted: Option<bool>`
}
```

**3. `report_service.rs::list_movements` — 13th `list_*`** (clone shape verbatim from
`list_device_acts`, lines 432-447):
```rust
pub async fn list_device_acts(&self, filter: ReportFilter, period: PeriodDto) -> Result<ReportResponse, AppError> {
    let tz = self.get_tz_offset();
    let (ts_from, ts_to) = compute_period_utc(&period, tz);
    let readers = self.readers.clone();
    tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        query_acts_inner(&conn, &filter, ts_from, ts_to, "handover")
    }).await.map_err(|e| AppError::Internal { source_chain: format!("spawn_blocking list_device_acts: {e}") })?
}
```
`list_movements` clones this exactly, calling a new `query_movements_inner(&conn, &filter,
ts_from, ts_to)` free function.

**4. The `WITH RECURSIVE subtree` place-filter clause to clone TWICE** (once for `from_place_id`,
once for `to_place_id`) — `report_service.rs::query_acts_inner`, lines 1140-1153:
```rust
// D-28: subtree-inclusive place filter — choosing a place captures it
// and every place nested under it, not just an exact place_id match.
if let Some(place_id) = filter.place_id {
    let idx = next_idx(&owned_params);
    owned_params.push(Box::new(place_id));
    with_prefix.push_str(&format!(
        "WITH RECURSIVE subtree(id) AS ( \
             SELECT id FROM places WHERE id = ?{idx} AND deleted_at_utc IS NULL \
             UNION ALL \
             SELECT p.id FROM places p JOIN subtree s ON p.parent_id = s.id \
             WHERE p.deleted_at_utc IS NULL \
         ) "
    ));
    clauses.push("a.place_id IN (SELECT id FROM subtree)".to_string());
}
```
For two independent filters (D-24, AND semantics per RESEARCH's Open Question 1 recommendation),
this needs TWO separate named CTEs (`from_subtree`, `to_subtree`) or two invocations joined by
`AND`, each parameterized the same way — SQLite supports multiple CTEs in one `WITH RECURSIVE ...,
... AS (...)` clause.

**5. `row_field` — new match arms** (`report_service.rs:1048-1082`, full function read):
```rust
fn row_field(row: &ReportRow, col: &str, tz: UtcOffset, shorten: bool) -> String {
    match col {
        "number" => row.number.as_deref().unwrap_or("").to_string(),
        "place_path" => if shorten { row.place_path_short.as_deref() } else { row.place_path.as_deref() }.unwrap_or("").to_string(),
        // ADD: "from_place_path" => same if/else shape on from_place_path[_short]
        // ADD: "actor_name" => row.actor_name.as_deref().unwrap_or("").to_string()
        // ADD: "reason" => (composed from source/note/act_number, or a pre-composed field)
        _ => String::new(),
    }
}
```

**6. CSV/PDF export — NO new function needed** (`export_csv`, lines 854-884, full read): it is
report-type-agnostic, iterating `columns: &[&str]` and calling `row_field`. Adding the movements
report's columns to `columns_for`/`column_labels_for` (below) is sufficient; `export_csv`/
`export_pdf` require zero changes.

**7. `tauri_cmds/reports.rs::columns_for`/`column_labels_for`/`report_display_name`** — 3
parallel index-aligned `match` statements (full file section read, lines 20-97):
```rust
fn columns_for(report_type: &str) -> Vec<&'static str> {
    match report_type {
        "device_acts" | "device_returns" => vec!["number", "device_name", "giver_name", "receiver_name", "place_path"],
        // ADD: "movements" => vec!["created_at_utc", "device_name", "entity_type_label", "from_place_path", "place_path", "actor_name", "reason"],
        _ => vec!["id"],
    }
}
fn column_labels_for(report_type: &str) -> Vec<&'static str> {
    match report_type {
        "device_acts" | "device_returns" => vec!["Номер", "Устройства", "Сдал", "Принял", "Место"],
        // ADD: "movements" => vec!["Дата", "Предмет", "Тип", "Откуда", "Куда", "Кем", "Причина"],  // D-23
        _ => vec!["ID"],
    }
}
```
An existing regression test (`column_labels_for_is_index_aligned_with_columns_for`, referenced at
`report_service.rs` line ~609 per RESEARCH) already enforces these two arrays stay index-aligned —
extend it, do not duplicate it, for the new `"movements"` arm.

**8. `build_reports_list_movements` — GATE DIVERGES from the other 12** (`tauri_cmds/reports.rs`,
full `build_reports_list_*` block read, lines 107-260): every existing report uses
`Action::ReadData`:
```rust
pub async fn build_reports_list_device_acts(/* ... */) -> Result<ReportResponse, AppError> {
    authorize(caller, &Action::ReadData)?;
    // ...
}
```
**The movements report is the FIRST report to need a DIFFERENT gate** — `Action::ReadPlaces`
(D-12: Admin+Manager only, unlike the other 12 which are `ReadData`-gated and visible to Employee
too). Do not copy-paste `Action::ReadData` for this one arm — this is the single most
copy-paste-error-prone spot in the whole report clone.

**9. `http/reports.rs::handler_list_movements`** — clone `handler_list_device_acts` exactly
(lines 61-75, full read):
```rust
pub async fn handler_list_device_acts(/* State, Session, Json */) -> Result<Json<ReportResponse>, AppErrorResponse> {
    let identity = session_identity(&session).await /* ... */;
    Ok(Json(
        build_reports_list_device_acts(&ctx, &identity, p.filter, p.period).await.map_err(AppErrorResponse::from)?,
    ))
}
```

---

### Timeline read-side (HST-02) — service + Tauri + HTTP pattern

**Analog:** `CartridgeService::get_history` (`cartridge_service.rs:471-491`, full read) — same
`spawn_blocking` + `readers.acquire()` shape, but note RESEARCH's explicit instruction NOT to
reuse `AuditEntryDto` (`crates/trackly-app/src/dto/cartridge.rs:498-508`, full struct read):
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AuditEntryDto {
    #[specta(type = i32)] pub id: i64,
    pub action: String,
    pub payload_json: Option<String>,   // <-- forces JSON.parse on the frontend; has NO user_id field
    pub before_json: Option<String>,
    pub after_json: Option<String>,
    #[specta(type = i32)] pub created_at_utc: i64,
}
```
New DTO must be flat and pre-formatted (server does the shortening/labeling, per D-18's "one
owner" rule and the Don't-Hand-Roll table): fields like `from_place_path_short`,
`to_place_path_short`, `actor_display`, `source`, `note`, `act_id`, `act_number`,
`created_at_utc` — NOT `payload_json`.

**Service method to clone** (`cartridge_service.rs:471-491`):
```rust
pub async fn get_history(&self, cartridge_id: i64) -> Result<Vec<AuditEntryDto>, AppError> {
    let readers = self.readers.clone();
    let repo = self.cart_repo.clone();
    tokio::task::spawn_blocking(move || -> Result<Vec<AuditEntryDto>, AppError> {
        let conn = readers.acquire();
        let rows = repo.get_history(&conn, cartridge_id)?;
        Ok(rows.into_iter().map(|r| AuditEntryDto { /* field-by-field map */ }).collect())
    }).await.map_err(|e| AppError::Internal { source_chain: format!("spawn_blocking: {e}") })?
}
```

**Tauri command / HTTP handler pair (with a `ReadPlaces` gate) to clone:**
`crates/trackly-app/src/tauri_cmds/places.rs`'s `build_places_list_subtree_contents`-style
wrapper (its own doc-comment cites "PLC-06 / D-23") is the closest existing `Action::ReadPlaces`-
gated read; use its `authorize(caller, &Action::ReadPlaces)?` line verbatim for both the new
timeline-read command/handler and the movements-report command/handler.

---

### D-28 bulk move — `PlaceService::move_subtree_contents` (new)

**Analog (read side, same scope):** `place_service.rs::list_subtree_contents` (already read in
full, lines 637-653):
```rust
pub async fn list_subtree_contents(&self, caller: &Identity, root_id: i64, nested: bool) -> Result<Vec<PlaceContentRow>, AppError> {
    authorize(caller, &Action::ReadPlaces)?;
    let readers = self.readers.clone();
    let repo = self.repo.clone();
    tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        repo.list_subtree_contents(&conn, root_id, nested)
    }).await.map_err(|e| AppError::Internal { source_chain: format!("spawn_blocking: {e}") })?
}
```
The new mutation walks this SAME result set (`kind: 'device'|'printer'|'cartridge'`, `id`) inside
ONE `self.writer.execute(move |conn| { let tx = conn.transaction()...` block, calling the SAME
before/after diff + movement-insert logic as write sites #1/#2 per row (`kind='printer'` treated
identically to `kind='device'`, since both mutate `devices.place_id`), authorized with
`Action::MutatePlaces` or the entity-specific mutate action per row — planner's call, but D-13
("mutation permissions unchanged") means it should reuse `MutateDevices`/`MutateCartridges`
per-row, NOT introduce a new blanket permission.

---

### Path-shortening promotion — `compute_place_path_short`

**Current location (private, to be promoted):** `crates/trackly-app/src/services/act_service.rs:3015-3038`
(full function read):
```rust
fn compute_place_path_short(readers: &ReaderPool, place_id: Option<i64>, snapshot: Option<String>) -> Option<String> {
    let snapshot = snapshot?;
    let conn = readers.acquire();
    let variant_token: String = place_id
        .and_then(|pid| conn.query_row(
            "SELECT effective_variant FROM place_effective_variant WHERE place_id = ?1",
            params![pid], |r| r.get::<_, String>(0),
        ).ok())
        .unwrap_or_else(|| read_org_default_variant_token(&conn));
    let variant = PathDisplayVariant::from_str(&variant_token).unwrap_or(PathDisplayVariant::Ends);
    let (sep_ends, sep_last_two) = read_path_display_separators(&conn);
    Some(shorten_place_path(&snapshot, variant, &sep_ends, &sep_last_two))
}
```
**Single-owner target module analog:** `crates/trackly-infra/src/repos/place_path_settings.rs`
(full header doc-comment + `read_path_display_separators`/`read_org_default_variant_token` read).
That module's own doc-comment explains WHY it lives in `trackly-infra` and not `trackly-core`:
```rust
//! # Почему модуль в `trackly-infra`, а не в `trackly-core`
//! Обе функции принимают `rusqlite::Connection`, а гейт
//! `crates/trackly-core/tests/no_io_deps.rs` держит `rusqlite` в списке крейтов,
//! запрещённых в ядре (гексагональная граница, FOUND-01). Чистая доменная часть —
//! `PathDisplayVariant` и `shorten_place_path` — как и прежде живёт в
//! `trackly_core::domain::places`; здесь только чтение настроек из БД.
```
`compute_place_path_short` takes `&ReaderPool` (a `trackly-app`-level type, `.acquire()` returns
a pooled connection), NOT a bare `&Connection` — so it does NOT fit `place_path_settings.rs`'s
own narrower "settings-read-only, `&Connection`" scope without a signature change. RESEARCH's
Open Question 3 recommendation (confirmed consistent with the gate above): promote it into a NEW
small `trackly-app`-level module (e.g. `crates/trackly-app/src/services/place_path_display.rs`)
that internally calls `place_path_settings::read_org_default_variant_token`/
`read_path_display_separators` (both already `pub fn(&Connection)`) — this keeps
`trackly-core`'s `no_io_deps.rs` gate untouched (no `rusqlite`/`ReaderPool` ever enters
`trackly-core`) while still having exactly ONE copy of the resolution algorithm, imported by
`act_service.rs` (existing caller), the new movement read-path, and the new report read-path.

**`no_io_deps.rs` gate itself** (`crates/trackly-core/tests/no_io_deps.rs`, full file read) —
forbidden-crate list includes `rusqlite`/`tokio`/`tauri`/`axum`; this confirms `place_movements`
domain types (`MovementSource` enum, any pure struct) CAN live in `trackly-core` (no I/O), but
`compute_place_path_short` and the repo/service layers CANNOT.

---

### Role-matrix test cases (Cases 45–48 pattern)

**Analog:** `crates/trackly-app/tests/role_endpoint_matrix.rs` Cases 45–48 (all four read in
full) — the canonical four-part shape for a single `Action`:
- Case 45 (HTTP, Manager, MUTATION denied) — not directly reusable (movements introduces no new
  mutation-permission change per D-13), but its STRUCTURE (`post_with_cookie` + `assert_eq!`
  status) is the HTTP-call pattern to copy.
- **Case 46 (HTTP, Manager, READ allowed)** — direct analog for the new timeline/report reads:
```rust
{
    let status = post_with_cookie(new_app!(), "/api/v1/places_list_all", json!({ "includeArchived": false }), Some(&manager_cookie)).await;
    assert_eq!(status, StatusCode::OK, "Case 46: Manager → places_list_all → expected 200, got {status}");
    let status = post_with_cookie(new_app!(), "/api/v1/places_get", json!({ "id": 1 }), Some(&manager_cookie)).await;
    assert!(status != StatusCode::UNAUTHORIZED && status != StatusCode::FORBIDDEN, "Case 46: ...");
}
```
- **Case 47 (HTTP, Employee, READ denied)** — direct analog, same endpoints, `StatusCode::FORBIDDEN`:
```rust
{
    let status = post_with_cookie(new_app!(), "/api/v1/places_list_all", json!({ "includeArchived": false }), Some(&employee_cookie)).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "Case 47: Employee → places_list_all → expected 403, got {status}");
}
```
- **Case 48 (Tauri path, direct `build_*` call, Forbidden variant match)** — direct analog for
  the Tauri-transport half of the SAME new endpoints:
```rust
let manager_id = Identity { user_id: Some(manager_dto.id), role: Role::Manager };
let result = build_places_create(&ctx, &manager_id, new_place).await;
assert!(matches!(result, Err(AppError::Forbidden)), "Case 48: ...");
```
New Cases for Phase 40 should follow this EXACT four-part shape (HTTP-allow for Manager,
HTTP-deny for Employee, Tauri-deny-or-allow for Manager) for: (a) the timeline read endpoint,
(b) the movements report list endpoint, (c) the movements report export endpoints (CSV/PDF), and
(d) D-28's bulk-move endpoint (mutation gate, unchanged permission model per D-13 — likely
`MutateDevices`/`MutateCartridges`, not a new action). The Case 48 comment itself documents WHY
this four-part shape exists (IN-02 lesson: a case that only covers one transport is a real,
previously-shipped gap in this exact codebase) — cite it in the new test's own comment.

---

### Svelte: `PlaceEntityViewModal.svelte` — new timeline `DetailSection`

**Analog:** the file itself (full 226-line file read) — single `$effect` fetch block (lines
83-107), `loading`/`loadError` state, footer with `handleGoTo`/`handleEdit`. The new timeline
section is added as sibling markup inside the SAME `<Modal>` body (after the existing
`DeviceFormBody`/`CartridgeFormBody` readonly render), sharing the SAME `loading`/`loadFailed`
branches — no independent spinner (per UI-SPEC's States table). The `$effect` block's async IIFE
(lines 87-106) needs one more `Promise.all` member (fetch the movement timeline) alongside the
existing `devices.get(r.id)` / `cartridges.get(r.id)` calls.

**`handleGoTo`'s `push()`-before-`onClose()` sequencing** (lines 128-132, full function read) —
this is the EXACT pattern the new timeline row's act-number link (D-19) and place link must copy,
including the file's own documented rationale (GAP-9) for why `push()` is awaited before
`onClose()`:
```ts
async function handleGoTo() {
    const target = `${SECTION_HASH_BY_KIND[row.kind] ?? '#/'}?id=${row.id}`;
    await push(target);
    onClose();
}
```
The timeline's own place-link and act-link handlers should mirror this exactly (`await push(...)`
then `onClose()` if the timeline is inside a modal being closed by the navigation).

---

### Svelte: `CartridgeDetail.svelte` — rename + new section

**Analog:** the file itself (`CartridgeDetail.svelte:75-205`, full section read). Existing
section to rename (line ~192, verbatim current markup):
```svelte
<DetailSection heading="История перемещений">
  {#if history.length === 0}
    <p class="history-empty">История пуста</p>
  {:else}
    <ul class="history-list">
      {#each history as entry (entry.id)}
        <li class="history-row">{formatHistoryEntry(entry)}</li>
      {/each}
    </ul>
  {/if}
</DetailSection>
```
D-16: change `heading="История перемещений"` → `heading="Журнал операций"`, leave everything
else (including `formatHistoryEntry`'s numeric-`place_id` display bug, lines 88-96, explicitly
NOT fixed by this phase) untouched. Add a NEW, separate `DetailSection heading="Перемещения"`
directly below it, rendering the new backend-formatted timeline DTO (no `JSON.parse`, no
`payload_json`) — this is a sibling block, not a modification of the existing one.

**`formatDate` (line ~"formatDate", manual padStart)** — UI-SPEC explicitly names this as the
date-formatting analog for the new timeline row's `ДД.ММ.ГГГГ` display (no `Intl` dependency);
locate and reuse it as-is (same file, same component).

---

### Svelte: `PlaceContents.svelte` — bulk-move action + `.crumb` link CSS

**Analog:** the file itself (full `.crumb`/`.crumb-sep` CSS block read, lines 345-375, and the
breadcrumb markup, lines 221-229):
```svelte
<button type="button" class="crumb" onclick={() => onSelectAncestor(ancestor.id)}>
  {ancestor.name}
</button>
```
```scss
.crumb {
  padding: 0; background: none; border: none; font: inherit; color: inherit; cursor: pointer;
  &:hover { color: var(--tr-text-primary); text-decoration: underline; }
  &:focus-visible { outline: none; box-shadow: 0 0 0 3px var(--tr-focus-ring); border-radius: var(--tr-radius-xs); }
}
```
Per UI-SPEC's "Clickable-link CSS contract," the NEW timeline place/act-number links reuse this
EXACT rule set with `color: inherit` replaced by `color: var(--tr-accent-text)` — nothing else
changes. This CSS block is the literal analog to copy-and-modify (one property) into whichever
component owns the timeline row markup (likely a small new shared snippet/component consumed by
`PlaceEntityViewModal` + `CartridgeDetail` + `PrinterDetail`, per the "3 consumers, 1 formula"
architecture note in RESEARCH).

**Bulk-move confirm-dialog analog:** `ui/src/features/devices/DeviceContextMenu.svelte`'s
delete-confirm `Modal` (full block read, lines 186-216):
```svelte
<Modal open={confirmOpen} title="Удалить устройство?" onClose={() => (confirmOpen = false)}>
  <p class="confirm-body">
    «{device.name}» (инв. № {device.inventory_no ?? '—'}) будет помечено как удалённое. ...
  </p>
  {#snippet footer()}
    <Button variant="secondary" onclick={() => (confirmOpen = false)}>Отмена</Button>
    <Button variant="destructive" loading={deleting} onclick={handleDelete}>Удалить</Button>
  {/snippet}
</Modal>
```
Clone this shape for the new "Перенести всё содержимое в…" confirm dialog on
`PlaceContents.svelte`, with TWO differences per UI-SPEC's Copywriting Contract: (1)
`variant="primary"` on the confirm button, NOT `variant="destructive"` (D-28 is not a deletion),
and (2) the body copy is UI-SPEC's own literal string («Место изменится у {N} предметов...»).

---

### Svelte: `ReportSubNav.svelte` — 4th `DOMAINS` entry

**Analog:** the file itself (full `DOMAINS`/`ReportConfig` structure read, lines 1-71):
```ts
type DomainKey = 'devices' | 'cartridges' | 'requests';   // ADD 'movements'
interface ReportConfig { key: string; label: string; temporal: boolean; cmd: string; }
const REQUEST_REPORTS: ReportConfig[] = [
  { key: 'all', label: 'Все', temporal: true, cmd: 'reports_list_requests_all' },
  // ...
];
const DOMAINS = [
  { key: 'devices' as DomainKey, label: 'Устройства' },
  { key: 'cartridges' as DomainKey, label: 'Картриджи' },
  { key: 'requests' as DomainKey, label: 'Заявки' },
  // ADD: { key: 'movements' as DomainKey, label: 'Перемещения' },
];
```
New single-entry array (mirrors `REQUEST_REPORTS`'s `key: 'all', label: 'Все'` naming
convention, per UI-SPEC's discretion #2):
```ts
const MOVEMENT_REPORTS: ReportConfig[] = [
  { key: 'all', label: 'Все перемещения', temporal: true, cmd: 'reports_list_movements' },
];
```

### Svelte: `ReportFilters.svelte` — two `PlacePicker` instances

**Analog:** the existing single place-filter block (full read, lines 1-40, 95-125, 175-200):
```svelte
<div class="place-filter-group">
  <label class="filter-label" for="report-place-filter">
    <span class="filter-name">Место</span>
  </label>
  <div class="place-filter">
    <PlacePicker id="report-place-filter" value={placeId} onChange={(id) => onFilterChange?.({ place_id: id })} />
  </div>
</div>
```
```scss
.place-filter-group { display: flex; align-items: center; gap: var(--tr-space-2xs); }
.place-filter { display: flex; flex-direction: column; width: 220px; max-width: 100%; }
```
Clone this block TWICE for D-24 («Откуда» / «Куда»), each with its own `id`/`value`/`onChange`
wired to `from_place_id`/`to_place_id` respectively — the movements domain's filter bar fully
replaces the single `place_id`/`is_storage` pair for its own domain (per UI-SPEC), so this is an
ADDITIVE sibling block pattern within `ReportFilters.svelte`, likely branching on
`activeDomain === 'movements'`.

---

## Shared Patterns

### Single-writer transaction discipline (applies to ALL 7 write sites + D-28's new one)

**Source:** every service method above (`device_service.rs::update`, `place_service.rs::create`,
`cartridge_service.rs::transition_in_tx`) — the universal shape:
```rust
self.writer.execute(move |conn| {
    let tx = conn.transaction().map_err(map_rusqlite)?;
    // ... read before-state, mutate, insert audit_log, insert place_movements — ALL inside tx
    tx.commit().map_err(map_rusqlite)?;
    Ok(result)
}).await?
```
**Apply to:** every new `place_movements` INSERT and every new `DELETE FROM place_movements WHERE
act_id = ?` — none may open its own transaction; all must execute against the caller's already-
open `&Transaction<'_>` (D-01's explicit requirement, WR-05 lesson from Phase 39.2).

### `authorize()` gate placement — first line of the service/build_* method

**Source:** `place_service.rs` (every method), `tauri_cmds/reports.rs::build_reports_list_*`.
**Apply to:** the new timeline-read service method, the new report's `build_reports_list_movements`
(using `Action::ReadPlaces`, NOT the other reports' `Action::ReadData` — see divergence note
above), and D-28's bulk-move mutation (reusing the existing per-entity mutate actions, per D-13).
**Both transports** must call the SAME `build_*`/service method — `http/*.rs` handlers never
call the repo/service layer directly, always via the shared `build_*` function
(`crates/trackly-app/src/http/devices.rs::handler_update` calling
`crate::tauri_cmds::devices::build_devices_update` is the concrete precedent).

### Soft-degrading enum parsing (Pitfall 6 / IN-01 lesson)

**Source:** `act_service.rs::compute_place_path_short`'s own doc-comment and body — `.ok()`/
`.unwrap_or()` chained throughout, NEVER `?`/`.expect()` on a cosmetic/display field:
```rust
let variant = PathDisplayVariant::from_str(&variant_token).unwrap_or(PathDisplayVariant::Ends);
```
**Apply to:** every read site for `place_movements.source` — write ONE helper (e.g.
`MovementSource::from_str_lenient(&str) -> MovementSource` with a safe fallback variant/label,
mirroring `PathDisplayVariant::from_str`) and call it from every consumer (timeline row, report
row, any future filter/grouping) — never inline `match` the raw string per call site. This is the
exact "5 copies, 1 forgot to degrade softly" shape that caused IN-01 in Phase 39.2.

### Actor snapshot, never lazy-resolved (D-09)

**Source:** the Don't-Hand-Roll table entry itself, backed by `place_service.rs`'s `user_id =
caller.user_id` extraction pattern shown above. **Apply to:** every write site — resolve
`actor_name_snapshot` via a single `SELECT full_name FROM users WHERE id = ?` INSIDE the same
transaction as the mutation (not a JOIN at read time), storing the snapshot alongside `user_id`.

---

## No Analog Found

None — every file in this phase's scope has at least a role-match analog in the existing
codebase. This is a pure codebase-integration phase (per RESEARCH's own framing); nothing here
requires inventing a new architectural shape.

---

## Metadata

**Analog search scope:** `crates/trackly-core/src/{auth.rs,domain/places.rs}`,
`crates/trackly-infra/src/repos/{cartridges_sqlite.rs,places_sqlite.rs,place_path_settings.rs,devices_sqlite.rs}`,
`crates/trackly-app/src/services/{device_service.rs,cartridge_service.rs,act_service.rs,place_service.rs,report_service.rs}`,
`crates/trackly-app/src/{tauri_cmds,http}/{devices.rs,reports.rs,places.rs}`,
`crates/trackly-app/src/dto/{reports.rs,cartridge.rs}`,
`crates/trackly-app/tests/role_endpoint_matrix.rs`, `migrations/V037-V039*.sql`,
`crates/trackly-core/tests/no_io_deps.rs`,
`ui/src/features/{places,cartridges,printers,devices,reports}/*.svelte`.
**Files scanned (read in full or targeted sections):** ~30.
**Pattern extraction date:** 2026-09-01.
