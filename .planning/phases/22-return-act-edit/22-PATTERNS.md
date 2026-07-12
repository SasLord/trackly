# Phase 22: Правка возвратов (return-act-edit) - Pattern Map

**Mapped:** 2026-07-12
**Files analyzed:** 13 (7 backend, 6 frontend) + 1 migration + 2 extended test files + 1 new test file
**Analogs found:** 13 / 13 (100% — every new/modified file has a direct, already-committed sibling to copy from; this phase is a structural clone of Phase 19)

All file:line citations below were verified against the current repo state this session
(not copied blindly from CONTEXT.md — two drifts were caught, see Notes at the bottom of
each section). RESEARCH.md's own analog map is confirmed correct; this document adds the
concrete excerpts + exact ranges the planner can paste into PLAN.md action sections.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `act_service.rs::update_return()` (new fn) | service | CRUD (delta-recompute) | `act_service.rs::update()` (:578-1027) | exact — same shape, same act, different `act_type` guard |
| `act_service.rs::do_return()` (write-site edit) | service | CRUD | itself, `:1214-1235` construction block | exact — surgical field-swap, not a new function |
| `act_service.rs::delete_soft()` `ActType::Return` branch (reference only, no edit needed) | service | CRUD (rollback) | itself, `:1811-1839` | exact — un-return restore mechanism to reuse per-device |
| `dto/act.rs::ActUpdateReturnDto` (new struct) | model/DTO | request-response | `dto/act.rs::ActUpdateDto` (:228-255) | exact — same id+expected_version+items shape |
| `dto/act.rs::ActReturnDto` (extend) | model/DTO | request-response | itself, `:126-136` (extend, don't replace) | exact |
| `dto/act.rs::ActItemDto` (extend, location fields) | model/DTO | request-response | itself, `:94-116` (extend) | exact |
| `audit_log_sqlite.rs::select_latest_device_mutation_pair` (new fn) | service/repo helper | CRUD (audit read) | `audit_log_sqlite.rs::select_latest_device_mutation` (:104-122) | exact — literal sibling, one extra column |
| `tauri_cmds/acts.rs::build_acts_update_return` + `#[tauri::command] acts_update_return` (new) | controller | request-response | `build_acts_update` / `#[tauri::command] acts_update` (:89-96, :217-224) | exact |
| `http/acts.rs::UpdateReturnPayload` + `handler_update_return` + router entry (new) | controller/route | request-response | `UpdatePayload` / `handler_update` / router `"/api/v1/acts_update"` (:62-66, :188-201, :301) | exact |
| `ui/src/lib/api/acts.ts::acts.updateReturn` (new) | service (client) | request-response | `acts.update` (:30-31) | exact |
| `migrations/V034__return_handover_date_backfill.sql` (new) | migration | batch | any single-`UPDATE` migration, e.g. `V031__requests_status_add_cancelled.sql` style | role-match (content is fully specified in RESEARCH.md, not restated here) |
| `ui/src/features/acts/ReturnModal.svelte` (extend: mode prop, date-picker, un-swap) | component | request-response | `ActFormBody.svelte` edit-mode (:63-96, :120-159, :260-262) + itself (create mode, unchanged parts) | exact |
| `ui/src/features/acts/ActDetail.svelte` (edit-gate line 70) | component | request-response | itself, line 70 (`{#if onEdit && act.act_type === 'handover' && !act.archived}`) | exact — one-line condition change |
| `ui/src/features/acts/ActsPage.svelte` (`handleEdit` branch on act_type) | component/orchestration | request-response | itself, `handleEdit` (:145-148), `handleEditSaved` (:150-162) | exact — extend existing function |
| `crates/trackly-app/tests/acts_update_return.rs` (new file) | test | CRUD (integration) | `crates/trackly-app/tests/acts_update.rs` (full file — helpers + 13 test fns) | exact — same helper scaffolding, same test shape |
| `crates/trackly-app/tests/acts_returns.rs` (extend) | test | CRUD (integration) | itself | exact |
| `crates/trackly-app/tests/acts_date_source.rs` (extend) | test | CRUD (integration) | itself | exact |
| `crates/trackly-app/tests/role_endpoint_matrix.rs` (extend, new RBAC case) | test | request-response | itself, Case 42 (`acts_update`, `:1413-1432`) | exact |

## Pattern Assignments

### `act_service.rs::update_return()` (service, CRUD/delta-recompute)

**Analog:** `ActService::update()` — full function at
`crates/trackly-app/src/services/act_service.rs:578-1027` (verified this session; CONTEXT.md's
citation of `:578` for `update()`'s start is correct).

**Structure to copy (verbatim skeleton), by numbered step — map each to the return-specific
substitution described in RESEARCH.md's flow diagram:**

1. **Validate + load + CAS pre-check** (`update()` lines 578-620):
```rust
pub async fn update(&self, payload: ActUpdateDto) -> Result<ActDto, AppError> {
    Self::validate_update(&payload)?;
    let now = self.clock.unix_seconds();
    let acts_repo = self.acts_repo.clone();
    let audit_repo = self.audit_repo.clone();
    let devices_repo = self.devices_repo.clone();
    let user_id_opt: Option<i64> = None;

    let act_id = self
        .writer
        .execute(move |conn| {
            let tx = conn.transaction().map_err(map_rusqlite)?;
            let act = acts_repo.fetch_full_in_tx(&tx, payload.id)?;
            if act.deleted_at_utc.is_some() {
                return Err(AppError::NotFound { entity: "act", id: payload.id });
            }
            // type-guard — for update_return(), invert: require Return, reject Handover
            if act.act_type != ActType::Handover {
                return Err(AppError::Validation {
                    field: "id".into(),
                    message: "Редактировать можно только акты выдачи (handover)".into(),
                });
            }
            if act.version != payload.expected_version {
                return Err(AppError::OptimisticLockMismatch {
                    entity: "act", id: payload.id,
                    expected: payload.expected_version, actual: act.version,
                });
            }
            // ... (continues below)
```
For `update_return`, the type-guard (step 2) becomes:
```rust
if act.act_type != ActType::Return {
    return Err(AppError::Validation {
        field: "id".into(),
        message: "Редактировать можно только акты возврата".into(),
    });
}
```

2. **Delta computation** (lines 654-668) — copy verbatim, `payload.id` is the RETURN's id here,
   not the parent's:
```rust
let d_old: std::collections::HashSet<i64> = { /* SELECT device_id FROM act_items WHERE act_id = ?1 */ };
let d_new: std::collections::HashSet<i64> = payload.items.iter().map(|i| i.device_id).collect();
let added: Vec<i64> = d_new.difference(&d_old).copied().collect();
let unchanged: Vec<i64> = d_old.intersection(&d_new).copied().collect();
let removed: Vec<i64> = d_old.difference(&d_new).copied().collect();
```
Note: for return-edit, `d_new` should flatten `ActReturnItemDto.device_ids[]` (via
`effective_device_ids`, see below) rather than a flat `device_id` field per `ActUpdateItemDto`
— the DTO shape differs from `ActUpdateDto` here (return items carry `device_ids: Vec<i64>`).

3. **Added-devices guard + mutation loop** (lines 679-754) — reuse the two-pass shape
   (status guard pass, then mutate+audit+insert pass), but substitute `do_return`'s own
   guard/mutation calls (see Pattern 2 below) instead of `update()`'s
   `update_status_and_location_in_tx` (that helper transitions на_складе→в_работе, which is
   the WRONG direction for a return; return-add must transition в_работе→на_складе via
   `update_full_in_tx`, exactly like `do_return`'s loop at `:1371-1378`).

4. **D-08-equivalent guard before mutating `removed`** (lines 812-830) — the return-edit
   equivalent is D-11 (Pattern 4 in RESEARCH.md), structurally identical validate-then-mutate
   placement: run the guard for ALL removed/changed devices BEFORE any of their mutations.

5. **Removed-devices restore** (lines 851-901) — copy verbatim, this is un-return:
```rust
for &removed_id in &removed {
    let before_json = audit_repo
        .select_latest_device_mutation(&tx, payload.id, removed_id)?
        .ok_or_else(|| AppError::Internal { source_chain: format!(
            "update: no audit trail for outstanding device {removed_id} on act {}", payload.id) })?;
    let snapshot: serde_json::Value = serde_json::from_str(&before_json)
        .map_err(|e| AppError::Internal { source_chain: format!("update: corrupt before_json for device {removed_id}: {e}") })?;
    let restored = devices_repo.restore_from_snapshot_in_tx(&tx, removed_id, &snapshot, now)?;
    let after_json = device_snapshot_json(&restored).map_err(|e| AppError::Internal { source_chain: format!("update remove after_json: {e}") })?;
    audit_repo.insert(&tx, AuditEntry {
        entity_type: "device", entity_id: removed_id, action: "custom:update_remove",
        user_id: user_id_opt, before_json: Some(before_json), after_json: Some(after_json),
        payload_json: Some(serde_json::json!({ "act_id": payload.id }).to_string()),
        created_at_utc: now,
    })?;
    tx.execute("DELETE FROM act_items WHERE act_id = ?1 AND device_id = ?2",
        params![payload.id, removed_id]).map_err(map_rusqlite)?;
}
```
IMPORTANT: for `update_return`, pass `act_id = return_act_id` (the return's own id, NOT the
parent) to `select_latest_device_mutation` — this is what makes the D-11 3-field snapshot
compare meaningful (Pattern 4 needs the pair-variant scoped to the return's own mutation, same
`act_id` argument).

6. **Header CAS write via `update_act_header_in_tx`** (lines 903-918) — reuse UNCHANGED:
```rust
let patch = ActPatch {
    giver_name: Some(payload.giver_name.clone()),
    receiver_name: Some(payload.receiver_name.clone()),
    location_id: Some(resolved_location_id),
    notes: Some(None),           // return acts have no notes field in the edit form
    deadline_utc: Some(None),    // return acts have no deadline
    handover_date_utc: payload.handover_date_utc,   // «Дата возврата», D-05
    number: None,                // returns never rename (out of scope)
    expected_version: payload.expected_version,
};
acts_repo.update_act_header_in_tx(&tx, payload.id, &patch, now)?;
```

7. **`recompute_parent_archived` gate** (lines 920-936) — for `update_return`, target the
   PARENT act (`act.parent_act_id`, not `payload.id`) and gate identically:
```rust
if !added.is_empty() || !removed.is_empty() {
    recompute_parent_archived(&tx, act.parent_act_id.expect("return act always has parent"), now)?;
}
```

**D-10 empty-item-set validation** — mirror `validate_update`'s items-non-empty check
(`act_service.rs:528-533`):
```rust
if p.items.is_empty() {
    return Err(AppError::Validation {
        field: "items".into(),
        message: "Добавьте хотя бы одну позицию".into(),
    });
}
```

**Notes on drift vs CONTEXT.md:** `update()` starts at line 578 (CONTEXT.md said `:578` too —
confirmed exact). `delete_soft`'s `ActType::Return` branch starts at line **1811**, not `:1746+`
as CONTEXT.md's citation implied (that line number is the top of the enclosing `match act.act_type`
statement, not the `Return` arm itself — RESEARCH.md already caught and documented this drift).

---

### `act_service.rs::do_return()` write-site edits (service, CRUD)

**Analog:** itself — this is a surgical 3-field edit inside the existing function, not a new
function. Exact block, `act_service.rs:1214-1235`:
```rust
let return_row = ActRow {
    id: 0,
    number: parent.number,
    sub_number: Some(sub_number),
    parent_act_id: Some(act_id),
    act_type: ActType::Return,
    giver_name: parent.giver_name.clone(),       // → payload.giver_name.unwrap_or_else(|| parent.receiver_name.clone())
    receiver_name: parent.receiver_name.clone(), // → payload.receiver_name.unwrap_or_else(|| parent.giver_name.clone())
    location_id: resolved_bulk_location_id,
    location: None,
    notes: None,
    deadline_utc: None,
    archived: false,
    created_at_utc: now,
    updated_at_utc: now,
    deleted_at_utc: None,
    version: 1,
    handover_date_utc: parent.handover_date_utc, // → payload.handover_date_utc.unwrap_or(now)
    parent_number: None,
    sibling_return_count: None,
};
```
This is Pitfall 1 + D-05's write-site fix — see RESEARCH.md for the full rationale (giver/
receiver were NEVER wired from the create-form despite the UI collecting them, `ReturnModal.svelte`
`handleSubmit` payload construction at lines 112-118 confirmed to never reference them).

**Validation to add to `validate_return`** (`act_service.rs:1033-1098`, extend, don't replace):
new optional `giver_name: Option<String>` / `receiver_name: Option<String>` / `handover_date_utc:
Option<i64>` fields on `ActReturnDto` need no NEW validation rule beyond the existing DB
`NOT NULL` constraint (`acts.giver_name`/`receiver_name`) already enforced by
`update_act_header_in_tx`'s and `insert_act_in_tx`'s SQL — the fallback-to-parent-swap-default
when `None` guarantees a non-null value always reaches the INSERT.

---

### `dto/act.rs::ActUpdateReturnDto` (new DTO, model, request-response)

**Analog:** `ActUpdateDto` — full struct, `crates/trackly-app/src/dto/act.rs:219-255`:
```rust
/// Payload sent by the UI when editing an existing act's header + item set
/// (Phase 19, ACT-02). ...
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
pub struct ActUpdateDto {
    #[specta(type = i32)]
    pub id: i64,
    #[specta(type = i32)]
    pub expected_version: i64,
    #[specta(type = Option<i32>)]
    pub number_override: Option<i64>,
    pub giver_name: String,
    pub receiver_name: String,
    #[specta(type = Option<i32>)]
    pub location_id: Option<i64>,
    #[serde(default)]
    pub location_name: Option<String>,
    pub notes: Option<String>,
    #[specta(type = Option<i32>)]
    pub deadline_utc: Option<i64>,
    #[serde(default)]
    #[specta(type = Option<i32>)]
    pub handover_date_utc: Option<i64>,
    pub items: Vec<ActUpdateItemDto>,
}
```
`ActUpdateReturnDto` mirrors this shape 1:1 EXCEPT:
- drop `number_override` (returns never rename, out of scope — RESEARCH.md Alternatives
  Considered table, "Reuse `update_act_header_in_tx` unchanged" row)
- add `bulk_condition: Option<String>` / `bulk_location_id: Option<i64>` /
  `bulk_location_name: Option<String>` / `apply_to_all: bool` (mirrors `ActReturnDto`'s own
  fields, `dto/act.rs:126-134`)
- `items: Vec<ActReturnItemDto>` (REUSE the existing return-item type, not a new
  `ActUpdateReturnItemDto` — RESEARCH.md's "Alternatives Considered" table explicitly
  recommends this: `ActReturnItemDto`'s `device_ids[]`/`condition_override`/
  `location_name_override` shape is already exactly what a full-replacement-set needs, and the
  frontend's `buildReturnItems()` helper works unchanged for both create and edit)
- `handover_date_utc: i64` (NOT `Option` — unlike `ActUpdateDto`'s optional
  "no change requested" semantics, D-04 requires the edit form to ALWAYS show + submit a
  populated «Дата возврата» value, mirroring `ActReturnDto`'s eventual required date field)

**Snake-case JSON invariant test** — copy the existing test pattern verbatim
(`dto/act.rs:454-481`, `act_update_dto_snake_case_json_invariant`) for the new
`ActUpdateReturnDto`.

---

### `dto/act.rs::ActReturnDto` + `ActItemDto` (extend, model, request-response)

**Analog:** itself. `ActReturnDto` full struct at `dto/act.rs:125-136`:
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
pub struct ActReturnDto {
    pub bulk_condition: Option<String>,
    #[specta(type = Option<i32>)]
    pub bulk_location_id: Option<i64>,
    pub bulk_location_name: Option<String>,
    pub apply_to_all: bool,
    pub items: Vec<ActReturnItemDto>,
}
```
Add (Pitfall 1 fix + D-05):
```rust
    #[serde(default)]
    pub giver_name: Option<String>,
    #[serde(default)]
    pub receiver_name: Option<String>,
    #[serde(default)]
    #[specta(type = Option<i32>)]
    pub handover_date_utc: Option<i64>,
```
All three `Option`-with-`#[serde(default)]` for back-compat with any not-yet-updated client
(mirrors `ActCreateDto.handover_date_utc`'s exact pattern at `dto/act.rs:196-198`).

`ActItemDto` full struct at `dto/act.rs:93-116` — add for Pitfall 2 (per-row location prefill):
```rust
    #[specta(type = Option<i32>)]
    pub device_location_id: Option<i64>,
    pub device_location: Option<String>,
```
Populate these via a `LEFT JOIN locations dl ON d.location_id = dl.id` added to
`load_items_for_act`'s SQL (`act_service.rs:2302-2343`) — mirrors the existing
`LEFT JOIN locations l ON a.location_id = l.id` pattern already used for the act-level
location in `acts_sqlite.rs`'s `SELECT_ACTS` (per RESEARCH.md Pitfall 2, exact line `:42`).

---

### `audit_log_sqlite.rs::select_latest_device_mutation_pair` (new fn, repo helper, CRUD)

**Analog:** `select_latest_device_mutation`, full fn at
`crates/trackly-infra/src/repos/audit_log_sqlite.rs:104-122`:
```rust
pub fn select_latest_device_mutation(
    &self,
    tx: &Transaction<'_>,
    act_id: i64,
    device_id: i64,
) -> Result<Option<String>, AppError> {
    tx.query_row(
        "SELECT before_json FROM audit_log \
         WHERE entity_type = 'device' \
           AND entity_id = ?2 \
           AND json_extract(payload_json, '$.act_id') = ?1 \
           AND before_json IS NOT NULL \
         ORDER BY created_at_utc DESC, id DESC LIMIT 1",
        params![act_id, device_id],
        |r| r.get::<_, String>(0),
    )
    .optional()
    .map_err(map_rusqlite)
}
```
New sibling — same query shape, select both columns, return a tuple:
```rust
pub fn select_latest_device_mutation_pair(
    &self,
    tx: &Transaction<'_>,
    act_id: i64,
    device_id: i64,
) -> Result<Option<(String, String)>, AppError> {
    tx.query_row(
        "SELECT before_json, after_json FROM audit_log \
         WHERE entity_type = 'device' \
           AND entity_id = ?2 \
           AND json_extract(payload_json, '$.act_id') = ?1 \
           AND before_json IS NOT NULL \
         ORDER BY created_at_utc DESC, id DESC LIMIT 1",
        params![act_id, device_id],
        |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
    )
    .optional()
    .map_err(map_rusqlite)
}
```
Add a unit test to the same `#[cfg(test)] mod tests` block (`audit_log_sqlite.rs:129-192`),
mirroring `round_trip_insert_and_select_by_act_id` (:146-191) with two rows for the same
device (different `act_id`) to assert only the newest row's `(before_json, after_json)` pair
comes back.

---

### `tauri_cmds/acts.rs::build_acts_update_return` + `acts_update_return` (controller, request-response)

**Analog:** `build_acts_update` + `#[tauri::command] acts_update`, both exact:
```rust
// build_acts_update_return.rs:89-96 pattern
pub async fn build_acts_update(
    ctx: &AppCtx,
    caller: &Identity,
    payload: ActUpdateDto,
) -> Result<ActDto, AppError> {
    authorize(caller, &Action::MutateActs)?;
    ctx.acts.update(payload).await
}
```
```rust
// tauri command wrapper, acts.rs:217-224
#[tauri::command]
#[specta::specta]
pub async fn acts_update(
    state: tauri::State<'_, AppCtx>,
    payload: ActUpdateDto,
) -> Result<ActDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_acts_update(state.inner(), &caller, payload).await
}
```
New pair: `build_acts_update_return(ctx, caller, payload: ActUpdateReturnDto) -> Result<ActDto, AppError>`
calling `authorize(caller, &Action::MutateActs)` (same `Action` variant — no new RBAC surface)
then `ctx.acts.update_return(payload).await`; `#[tauri::command] acts_update_return` wraps it
identically.

---

### `http/acts.rs::UpdateReturnPayload` + `handler_update_return` + router entry (controller/route)

**Analog:** exact triple at `crates/trackly-app/src/http/acts.rs`:
```rust
// Payload struct, lines 62-66
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePayload {
    pub payload: ActUpdateDto,
}
```
```rust
// Handler, lines 188-201
pub async fn handler_update(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<UpdatePayload>,
) -> Result<Json<ActDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_acts_update(&ctx, &identity, p.payload)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}
```
```rust
// Router entry, line 301, inside fn router() -> Router<AppCtx> (lines 293-313)
.route("/api/v1/acts_update", post(handler_update))
```
New: `UpdateReturnPayload { pub payload: ActUpdateReturnDto }`, `handler_update_return` (identical
shape, calls `build_acts_update_return`), router entry
`.route("/api/v1/acts_update_return", post(handler_update_return))` inserted next to the
existing `acts_update`/`acts_return` lines (:299-301) for discoverability.

---

### `ui/src/lib/api/acts.ts::acts.updateReturn` (client, request-response)

**Analog:** `acts.update`, exact, `ui/src/lib/api/acts.ts:30-31`:
```typescript
/** Phase 19 Plan 04 — редактирование существующего акта (ACT-02). */
update: (payload: ActUpdateDto) => apiCall<ActDto>('acts_update', { payload }),
```
New:
```typescript
/** Phase 22 — редактирование существующего возврата (ACT-03). */
updateReturn: (payload: ActUpdateReturnDto) => apiCall<ActDto>('acts_update_return', { payload }),
```
Add `ActUpdateReturnDto` to the `import type { ... } from '../../bindings'` block (line 11-20) —
`bindings.ts` regenerates automatically once the Rust `#[derive(Type)]` struct + `#[specta::specta]`
command exist (`export_bindings.rs` test, see Wave-0 gap list in RESEARCH.md).

---

### `migrations/V034__return_handover_date_backfill.sql` (migration, batch)

**Content is fully specified in RESEARCH.md** (verbatim SQL + rationale) — no additional pattern
extraction needed beyond confirming the next-free-version number. Confirmed via directory
listing this session: last migration is `V033__org_settings_requisites.sql`; **V034** is free.
No schema-defining migration to copy structurally from (this is a pure data-backfill `UPDATE`,
simplest possible migration shape — any single-statement migration in the directory, e.g.
`V031__requests_status_add_cancelled.sql`, demonstrates the one-file-one-statement convention).

```sql
UPDATE acts SET handover_date_utc = created_at_utc WHERE act_type = 'return';
```

---

### `ui/src/features/acts/ReturnModal.svelte` (component, extend for edit mode)

**Two analogs, blended:**

1. **`ActFormBody.svelte`'s edit-mode pattern** (mode prop, date-picker prefill/submit) —
   `crates/../ui/src/features/acts/ActFormBody.svelte:18-34` (Props interface with `mode`/
   `initialAct`), `:44-61` (`todayISO`/`unixToIso` helpers), `:82-96` (state init keyed off
   `isEditPrefill`), `:120-124` (`isoToUnix`), `:260-262` (DatePicker JSX usage):
```typescript
function todayISO(): string {
  const d = new Date();
  const y = d.getUTCFullYear();
  const m = String(d.getUTCMonth() + 1).padStart(2, '0');
  const day = String(d.getUTCDate()).padStart(2, '0');
  return `${y}-${m}-${day}`;
}
function unixToIso(unixSeconds: number | null | undefined): string {
  if (unixSeconds === null || unixSeconds === undefined) return '';
  const d = new Date(unixSeconds * 1000);
  const y = d.getUTCFullYear();
  const m = String(d.getUTCMonth() + 1).padStart(2, '0');
  const day = String(d.getUTCDate()).padStart(2, '0');
  return `${y}-${m}-${day}`;
}
function isoToUnix(iso: string): number | null {
  if (!iso) return null;
  const t = Date.parse(iso + 'T00:00:00Z');
  return Number.isFinite(t) ? Math.floor(t / 1000) : null;
}
```
```svelte
<DatePicker id="act-handover-date" bind:value={handoverDateISO} required />
```
Apply verbatim in `ReturnModal.svelte`: add a `mode?: 'create' | 'edit'` + `editTarget?: ActDto
| null` prop pair (mirrors `ActFormModal.svelte`'s own `mode`/`initialAct` props, confirmed at
`ActFormModal.svelte:12-13,18,44-45,61`), a `returnDateISO` state initialized to
`isEditPrefill ? unixToIso(editTarget!.handover_date_utc) : todayISO()`, submit
`handover_date_utc: isoToUnix(returnDateISO)`.

2. **`ReturnModal.svelte` itself (create-mode parts stay unchanged)** — the `$effect` rebuilding
   `rows` on `act` prop change (:45-70) must branch: in `mode==='edit'`, seed rows from BOTH
   `editTarget.items[]` (checked=true, pre-filled condition/location from the item's own
   `condition_at_time`/new `device_location` field) AND the parent's outstanding items
   (checked=false, addable) — this requires the caller (`ActsPage.svelte`) to also fetch
   `parent = await acts.get(act.parent_act_id)` before opening the modal (see `ActsPage.svelte`
   pattern below). The giver/receiver **swap logic** (:59-64) must be SKIPPED in edit mode —
   prefill directly from `editTarget.giver_name`/`editTarget.receiver_name` (D-12: these are the
   return's own saved values now that Pitfall 1 is fixed, not the parent's).

**`handleSubmit`'s payload construction** (:104-118) branches on mode:
```typescript
// create mode — unchanged, calls acts.doReturn(act.id, payload)
// edit mode — new:
const updatePayload: ActUpdateReturnDto = {
  id: editTarget!.id,
  expected_version: editTarget!.version,
  giver_name: giverName.trim(),
  receiver_name: receiverName.trim(),
  handover_date_utc: isoToUnix(returnDateISO)!,
  bulk_condition: bulkCondition.trim().length > 0 ? bulkCondition.trim() : null,
  bulk_location_id: null,
  bulk_location_name: bulkLocationName.trim().length > 0 ? bulkLocationName.trim() : null,
  apply_to_all: applyToAll,
  items, // buildReturnItems(rows, applyToAll) — UNCHANGED helper, reused verbatim
};
const saved = await acts.updateReturn(updatePayload);
```
`buildReturnItems()` from `returnPayload.ts` is reused UNCHANGED for both create and edit
submission (confirmed by RESEARCH.md's Alternatives Considered table — the `ActReturnItemDto`
shape already matches what a full-replacement-set needs).

**D-10 empty-set guard on the Save button** — extend `canSubmit`'s `$derived.by` (:86-98): add
`if (checkedRows.length === 0) return false;` (already present, line 88) — this already
satisfies D-10 for BOTH create and edit modes without additional code, since un-checking every
row in edit mode naturally drives `checkedRows.length` to 0.

---

### `ui/src/features/acts/ActDetail.svelte` (component, one-line gate)

**Analog:** itself, exact line 70:
```svelte
{#if onEdit && act.act_type === 'handover' && !act.archived}
  <Button variant="secondary" size="sm" onclick={() => onEdit(act)}>Редактировать</Button>
{/if}
```
Change to:
```svelte
{#if onEdit && (act.act_type === 'handover' || act.act_type === 'return') && !act.archived}
```
Note: the `!act.archived` guard on a return-act refers to the RETURN row's own `archived` field
(always `false` — only handover/parent rows carry a meaningful `archived` flag; V004's schema
has the column on every `acts` row but `recompute_parent_archived` only ever writes it for
parent rows). Confirm this doesn't block return-edit unexpectedly — return rows' `archived` is
always `false` by construction (never written by `do_return`'s INSERT, `:1226`), so the gate is
a no-op restriction for returns and safe to leave as-is.

---

### `ui/src/features/acts/ActsPage.svelte` (orchestration, extend `handleEdit`)

**Analog:** itself, exact block `:141-148`:
```typescript
// Plan 19-05 (ACT-02): reuse the `act` argument directly (no acts.get(act.id)
// re-fetch) — onEdit is only ever invoked from ActDetail where act === selectedAct...
function handleEdit(act: ActDto) {
  editTargetAct = act;
  editModalOpen = true;
}
```
Extend to branch on `act.act_type`:
```typescript
async function handleEdit(act: ActDto) {
  if (act.act_type === 'return') {
    // Return-edit needs the parent's outstanding items too (addable rows) —
    // ReturnModal in edit mode needs BOTH act.items (already returned, checked)
    // AND parent.items[].outstanding_device_ids (addable, unchecked).
    try {
      const parent = await acts.get(act.parent_act_id!);
      returnEditParentAct = parent;
      returnEditTargetAct = act;
      returnModalOpen = true; // reuse ReturnModal in mode="edit"
    } catch (e: unknown) {
      const msg = e && typeof e === 'object' && 'message' in e
        ? String((e as { message: unknown }).message)
        : 'Не удалось загрузить родительский акт';
      pushToast('error', msg);
    }
    return;
  }
  editTargetAct = act;
  editModalOpen = true;
}
```
`handleEditSaved` (`:150-162`) reactive-refresh pattern must be mirrored for the return-edit
success callback (reuse or extend `handleReturnSuccess`, `:169-183`) — the same D-11 (Phase 19)
"assign `selectedAct` directly from the fresh server response, don't rely on the
`selectedActId`-keyed `$effect`" fix applies verbatim to return-edit, since `ReturnModal`'s
`onSuccess` callback already receives the fresh `ActDto`:
```typescript
function handleEditSaved(act: ActDto) {
  editModalOpen = false;
  editTargetAct = null;
  // D-11: selectedActId = act.id is a no-op when the edited act is already
  // selected ... Assign selectedAct directly...
  selectedActId = act.id;
  selectedAct = act;
  refresh();
  refreshCounts();
}
```

---

### `crates/trackly-app/tests/acts_update_return.rs` (new test file)

**Analog:** `crates/trackly-app/tests/acts_update.rs`, full file. Copy the helper scaffolding
verbatim:
```rust
fn make_acts_service() -> (ActService, tempfile::TempDir) { /* :39-45 */ }
async fn seed_devices_with_state(/* :46-75 */) { }
async fn seed_location(/* :76-97 */) { }
async fn create_handover_with_location(/* :98-131 */) { }
async fn read_device_snap(svc: &ActService, device_id: i64) -> DeviceSnap { /* :132-154 */ }
fn update_dto_from(act: &ActDto, device_ids: &[i64]) -> ActUpdateDto { /* :155-181 */ }
```
For `acts_update_return.rs`, adapt `create_handover_with_location` to ALSO call `do_return`
once (seeding an initial return act to edit), and replace `update_dto_from` with an
`update_return_dto_from(return_act: &ActDto, device_ids: &[i64]) -> ActUpdateReturnDto` helper
of the same shape.

**Test-fn-per-behavior convention** — one `#[tokio::test] async fn snake_case_description()`
per row of RESEARCH.md's Phase Requirements → Test Map table (11 new fns listed there):
`retained_edit_changes_device_condition_location`, `un_return_restores_prior_state`,
`add_outstanding_device_to_return`, `reject_empty_item_set`,
`reject_un_return_after_reissue`, `reject_edit_after_manual_device_relocation`,
`allow_edit_when_device_untouched`, `add_last_device_archives_parent`,
`un_return_unarchives_parent`, `version_mismatch_returns_conflict`. Mirror the exact assertion
style of existing analogs `version_mismatch_returns_conflict` (`acts_update.rs:271-315`),
`remove_position_restores_prior_state` (`:363-431`), and `remove_last_outstanding_archives_act`
(`:694-759`) — these three are the closest 1:1 behavioral matches for the return-edit
equivalents (version-mismatch check, un-return restore, archive-flip-on-last-item).

---

### `crates/trackly-app/tests/role_endpoint_matrix.rs` (extend, RBAC test)

**Analog:** Case 42 (`acts_update` Employee-403 case), exact, `:1413-1432`:
```rust
// Phase 19 Plan 04 (ACT-02): acts_update — id/expected_version don't
// need to reference a real act, RBAC must reject before any lookup.
let act_update_payload = json!({
    "payload": {
        "id": 1, "expected_version": 1, "number_override": null,
        "giver_name": "Тест Тестов", "receiver_name": "Тест2 Тестов",
        "location_id": null, "location_name": null, "notes": null,
        "deadline_utc": null, "handover_date_utc": null, "items": []
    }
});
// ...
{
    let status = post_with_cookie(
        new_app!(), "/api/v1/acts_update", act_update_payload.clone(), Some(&employee_cookie),
    ).await;
    assert_eq!(status, StatusCode::FORBIDDEN,
        "Case 42: Employee → acts_update → expected 403, got {status}");
}
```
New "Case 4X" (next free case number — check current max case number in the file before
assigning) uses the same shape, POSTing to `/api/v1/acts_update_return` with a synthetic
`ActUpdateReturnDto` JSON body (id/expected_version don't need to be real — RBAC rejects
before lookup, same as Case 42).

## Shared Patterns

### Single-writer transaction + CAS optimistic lock
**Source:** `crates/trackly-infra/src/repos/acts_sqlite.rs:377-426` (`update_act_header_in_tx`)
**Apply to:** `update_return`'s header write — reused UNCHANGED, zero code changes needed. The
function has no `act_type` branching at all.

### Audit-log-as-source-of-truth for device restore
**Source:** `crates/trackly-infra/src/repos/audit_log_sqlite.rs:104-122`
(`select_latest_device_mutation`) + `crates/trackly-infra/src/services/act_service.rs`'s
`restore_from_snapshot_in_tx` calls (e.g. `:871-876`, `:1813-1820` via
`undo_device_mutations_for_act`)
**Apply to:** un-return restore in `update_return` — same DESC-LIMIT-1 lookup pattern, scoped
to the RETURN's own `act_id`, not the parent's.

### `recompute_parent_archived` — idempotent, unconditional, item-count-driven
**Source:** `crates/trackly-infra/src/repos/acts_sqlite.rs:502-539`
**Apply to:** `update_return` (call on parent id, gated by `added`/`removed` non-empty, same as
`update()`'s own gate at `act_service.rs:934`), and is ALREADY called correctly by `do_return`
(`:1406`) and `delete_soft`'s Return branch (`:1823`) — no changes needed to those two call
sites, only awareness that ordering relative to the return's own CAS header write doesn't
matter (different row, see RESEARCH.md Pitfall 4).

### `authorize(caller, &Action::MutateActs)` — no new RBAC surface
**Source:** every existing act-mutation controller (`build_acts_update`, `build_acts_return`,
`build_acts_delete`, all in `tauri_cmds/acts.rs`)
**Apply to:** `build_acts_update_return` — identical call, same `Action` variant, no new enum
case needed.

### `#[serde(default)]` + `Option<T>` for back-compat new DTO fields
**Source:** `dto/act.rs:196-198` (`ActCreateDto.handover_date_utc`), `:243-244`
(`ActUpdateDto.location_name`)
**Apply to:** all new fields added to `ActReturnDto` (giver_name/receiver_name/
handover_date_utc) — ensures any not-yet-updated client (or an older test payload) still
deserializes without the new fields present.

## No Analog Found

None — every file in this phase's scope has a direct, already-committed sibling to copy from
(this phase is architecturally a near-exact structural clone of Phase 19's handover-edit work,
per RESEARCH.md's own framing).

## Metadata

**Analog search scope:** `crates/trackly-app/src/services/act_service.rs`,
`crates/trackly-app/src/dto/act.rs`, `crates/trackly-infra/src/repos/{acts_sqlite,
audit_log_sqlite}.rs`, `crates/trackly-app/src/tauri_cmds/acts.rs`,
`crates/trackly-app/src/http/acts.rs`, `ui/src/lib/api/acts.ts`,
`ui/src/features/acts/{ReturnModal,ActFormBody,ActFormModal,ActDetail,ActsPage,
ActListRow}.svelte`, `crates/trackly-app/tests/{acts_update,role_endpoint_matrix}.rs`,
`migrations/*.sql` (directory listing only).
**Files scanned (full or targeted read this session):** 13
**Pattern extraction date:** 2026-07-12
**Line-number verification:** All citations above were re-verified against the current file
state this session (matches RESEARCH.md's own verification pass — no additional drift found
beyond the two RESEARCH.md already documented: `ActRow.handover_date_utc` at
`domain/acts.rs:155` not `:141`, and `delete_soft`'s `Return` arm at `act_service.rs:1811` not
`:1746`).
