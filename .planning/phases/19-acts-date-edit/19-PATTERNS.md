# Phase 19: Акты — дата и редактирование - Pattern Map

**Mapped:** 2026-07-11
**Files analyzed:** 14 (5 backend new/modified, 2 backend-adjacent helper additions, 7 frontend)
**Analogs found:** 14 / 14 (all files have a strong in-repo analog — this phase is a pure "extract + adapt" job per RESEARCH.md)

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `crates/trackly-core/src/domain/acts.rs` (extend `ActPatch`) | model | CRUD | same file, `ActNew`/`ActReturnNew` structs | exact (sibling struct in same file) |
| `crates/trackly-app/src/dto/act.rs` (`ActDto.handover_date_utc` + new `ActUpdateDto`/`ActUpdateItemDto`) | model/DTO | CRUD | same file, `ActCreateDto`/`ActReturnDto`/`ActReturnItemDto` | exact (sibling DTOs in same file) |
| `crates/trackly-infra/src/repos/acts_sqlite.rs` (`update_act_header_in_tx` + `ORDER BY` fix ×2) | service (repo helper) | CRUD (CAS write) | `soft_delete_in_tx` (lines 325-362) for CAS shape; `insert_act_in_tx` (92-116) for column list | exact |
| `crates/trackly-infra/src/repos/audit_log_sqlite.rs` (new `select_latest_device_mutation`) | service (repo helper) | CRUD (read) | `select_device_mutations_for_act` (66-90) | exact (near-identical query, narrower filter) |
| `crates/trackly-app/src/services/act_service.rs` (`ActService::update` + `populate_outstanding_device_ids_in_tx` twin + date-source switch in `render_pdf`) | service | CRUD (multi-step tx) | `create` (200-512) for add-loop + validation shape; `do_return` (582-923) for tx orchestration + guards; `delete_soft` (1203-1327) for CAS + audit shape; `undo_device_mutations_for_act` (1707-1744) for restore-from-snapshot pattern; `populate_outstanding_device_ids` (1846-1873) for D-08 predicate | exact (service already has 3 sibling mutation methods with the exact same shape) |
| `crates/trackly-app/src/tauri_cmds/acts.rs` (`build_acts_update` + `#[tauri::command] acts_update`) | controller (Tauri command) | request-response | `build_acts_delete`/`acts_delete` (75-83, 192-201) for CAS-mutation shape; `build_acts_return`/`acts_return` (64-72, 181-190) for payload+id shape | exact |
| `crates/trackly-app/src/http/acts.rs` (`UpdatePayload` + `handler_update` + router entry) | controller (axum handler) | request-response | `DeletePayload`/`handler_delete` (54-59, 167-179); `ReturnPayload`/`handler_return` (61-66, 152-165) | exact |
| `crates/trackly-app/tests/acts_update.rs` (new) | test | CRUD | `crates/trackly-app/tests/acts_undo.rs`, `acts_crud.rs`, `acts_returns.rs` (not read in full — referenced by RESEARCH.md as the test-convention source) | exact (naming/fixture convention) |
| `crates/trackly-app/tests/role_endpoint_matrix.rs` (extend, add `acts_update` case) | test | request-response | Case 4 "Employee → acts_create → 403" (lines 488-503) | exact |
| `ui/src/lib/api/acts.ts` (`acts.update(...)`) | service (API client) | request-response | `acts.doReturn` (30-31) / `acts.delete` (33) | exact |
| `ui/src/features/acts/ActFormBody.svelte` (`mode: 'create'\|'edit'` prop, prefill, branch submit) | component (form) | CRUD (form submit) | same file's `handleSubmit`/create-payload-building (82-148); `ActFormItemsTable`'s `FormItemRow` shape (19-39) for position-row state | exact (extend in place, not a new file) |
| `ui/src/features/acts/ActFormModal.svelte` (`mode`/`initialAct` props, title switch) | component (modal shell) | request-response | same file (extend in place) | exact |
| `ui/src/features/acts/ActDetail.svelte` (`headerDate` source switch, wire `onEdit` call-site) | component | CRUD (display) | same file — `headerDate` derivation (37-43) and existing (currently-dead) `onEdit` prop plumbing (14-20, 70) | exact |
| `ui/src/features/acts/ActListRow.svelte` (`dateLabel` source switch) | component | CRUD (display) | same file — `dateLabel` derivation (31-36) | exact |
| `ui/src/features/acts/ActsPage.svelte` (`editModalOpen`/`editTargetAct` state + `onEdit` handler) | component (orchestration) | CRUD (orchestration) | same file — `handleReturn`/`returnTargetAct`/`returnModalOpen` (34-35, 134-137) and `createModalOpen`/`openCreate`/`handleSaved` (33, 119-127) | exact |

## Pattern Assignments

### `crates/trackly-core/src/domain/acts.rs` — extend `ActPatch` (model)

**Analog:** same file, `ActNew`/`ActReturnNew` (domain structs, no serde) — `crates/trackly-core/src/domain/acts.rs:49-107`

**Current `ActPatch`** (lines 109-117, currently unused anywhere — confirmed by RESEARCH.md grep):
```rust
/// Partial update for an act (used by Phase 7 admin UI; minimal usage in Phase 3).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ActPatch {
    pub giver_name: Option<String>,
    pub receiver_name: Option<String>,
    pub location_id: Option<Option<i64>>,
    pub notes: Option<Option<String>>,
    pub deadline_utc: Option<Option<i64>>,
}
```
**Extend with** (per RESEARCH.md Open Question 2 recommendation): `handover_date_utc: Option<i64>`, `number: Option<i64>`, `expected_version: i64` (not `Option` — always required for CAS). Domain stays serde-free — `items: Vec<...>` for D-06 stays in the DTO layer only (`trackly-app::dto::act::ActUpdateDto`), destructured by the service into `(ActPatch, Vec<i64> new_device_ids)` — mirrors how `ActCreateDto` → `ActNew`-shaped fields are consumed directly by `create` without a domain `ActNew` round-trip today (note: `create` currently reads straight off `ActCreateDto`, not `ActNew` — `ActNew` is a currently-unused parallel domain type; follow the *same* convention `create` actually uses, i.e., let `ActService::update` read fields straight off `ActUpdateDto`, not `ActPatch`, if that turns out simpler — planner's call, `ActPatch` extension is the RESEARCH.md-recommended starting point but not mandatory).

---

### `crates/trackly-app/src/dto/act.rs` — `ActDto.handover_date_utc` + `ActUpdateDto`/`ActUpdateItemDto`

**Analog:** same file, `ActCreateDto`/`ActItemNewDto` (170-215) for the create-payload shape; `ActReturnDto`/`ActReturnItemDto` (116-168) for the "id + expected mutation state" shape.

**Add to `ActDto`** (insert as a sibling of `created_at_utc` at `dto/act.rs:71-74` — Pitfall 1 in RESEARCH.md: this MUST happen before any frontend date-source switch):
```rust
#[specta(type = i32)]
pub created_at_utc: i64,
#[specta(type = i32)]
pub updated_at_utc: i64,
// NEW:
#[specta(type = i32)]
pub handover_date_utc: i64,
```
Then update `act_dto_from_row` (295-323) to copy `row.handover_date_utc` through — `ActRow` already carries this field (`domain/acts.rs:142`), only the DTO mapping is missing.

**New `ActUpdateDto`** — model on `ActCreateDto` (172-198) header fields + `ActReturnDto`'s id/expected-version idea:
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
pub struct ActUpdateDto {
    #[specta(type = i32)]
    pub id: i64,
    #[specta(type = i32)]
    pub expected_version: i64,
    #[specta(type = Option<i32>)]
    pub number_override: Option<i64>,   // only when caller wants to change №; None = no-op (A3 uniqueness re-check only fires if Some)
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
    pub items: Vec<ActUpdateItemDto>,   // full replacement set — service diffs against current act_items
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
pub struct ActUpdateItemDto {
    #[specta(type = i32)]
    pub device_id: i64,
}
```
Snake_case JSON invariant test convention to mirror (`dto/act.rs:368-387`, `snake_case_json_invariant` test) — add an equivalent test asserting `ActUpdateDto` serializes `expected_version`/`number_override` in snake_case, not camelCase.

---

### `crates/trackly-infra/src/repos/acts_sqlite.rs` — `update_act_header_in_tx` (CAS UPDATE)

**Analog:** `soft_delete_in_tx` — `crates/trackly-infra/src/repos/acts_sqlite.rs:325-362` (CAS shape to copy verbatim); `insert_act_in_tx` — `acts_sqlite.rs:92-116` (column list to mirror).

**CAS pattern to copy exactly** (this IS Pattern 1 from RESEARCH.md — do not deviate):
```rust
pub fn soft_delete_in_tx(
    &self,
    tx: &Transaction<'_>,
    id: i64,
    version: i64,
    now_utc: i64,
) -> Result<(), AppError> {
    let affected = tx
        .execute(
            "UPDATE acts SET deleted_at_utc = ?1, version = version + 1, \
             updated_at_utc = ?1 \
             WHERE id = ?2 AND version = ?3 AND deleted_at_utc IS NULL",
            params![now_utc, id, version],
        )
        .map_err(map_rusqlite)?;

    if affected == 0 {
        let actual: Option<i64> = tx
            .query_row("SELECT version FROM acts WHERE id = ?1", params![id], |r| {
                r.get(0)
            })
            .optional()
            .map_err(map_rusqlite)?;
        return match actual {
            None => Err(AppError::NotFound { entity: "act", id }),
            Some(actual) => Err(AppError::OptimisticLockMismatch {
                entity: "act", id, expected: version, actual,
            }),
        };
    }
    // ... (soft_delete_in_tx's own DELETE act_items follow-up — update_act_header_in_tx
    // has NO equivalent follow-up statement; it's a pure header UPDATE)
    Ok(())
}
```
`update_act_header_in_tx(tx, id, patch: &ActPatch, now_utc)` should build the same `UPDATE acts SET <fields...>, version = version + 1, updated_at_utc = ?now WHERE id = ? AND version = ?expected_version` single statement — column list to touch per RESEARCH.md Code Examples section: `giver_name, receiver_name, location_id, notes, deadline_utc, handover_date_utc, number (if Some)` — explicitly NOT `sub_number`/`parent_act_id`/`act_type` (immutable identity fields) and NOT `created_at_utc` (D-02: purely internal).

**`ORDER BY` date-source fix** — two call sites, both currently `a.created_at_utc DESC, a.id DESC`:
```rust
// list() — acts_sqlite.rs:537
ORDER BY a.created_at_utc DESC, a.id DESC
// → ORDER BY a.handover_date_utc DESC, a.id DESC

// search_acts() — acts_sqlite.rs:295
ORDER BY a.created_at_utc DESC, a.id DESC
// → ORDER BY a.handover_date_utc DESC, a.id DESC
```
`SELECT_ACTS` (30-44) already selects `a.handover_date_utc` as its last column (line 40) and `from_row` (47-83) already maps it into `ActRow.handover_date_utc` (line 81) — no SELECT/mapping change needed, only the two `ORDER BY` clauses.

---

### `crates/trackly-infra/src/repos/audit_log_sqlite.rs` — `select_latest_device_mutation` (single-device, most-recent)

**Analog:** `select_device_mutations_for_act` — `crates/trackly-infra/src/repos/audit_log_sqlite.rs:66-90` (same table, near-identical query — only the filter and ORDER direction differ).

```rust
// EXISTING — bulk, chronological ASC (used by full-act LIFO undo):
pub fn select_device_mutations_for_act(
    &self,
    tx: &Transaction<'_>,
    act_id: i64,
) -> Result<Vec<(i64, String)>, AppError> {
    let mut stmt = tx
        .prepare(
            "SELECT entity_id, before_json FROM audit_log \
             WHERE entity_type = 'device' \
               AND json_extract(payload_json, '$.act_id') = ?1 \
               AND before_json IS NOT NULL \
             ORDER BY created_at_utc ASC, id ASC",
        )
        .map_err(map_rusqlite)?;
    let rows = stmt
        .query_map(params![act_id], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
        })
        .map_err(map_rusqlite)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(map_rusqlite)?);
    }
    Ok(out)
}
```
**New method — copy the shape, narrow the filter to `entity_id = device_id`, flip ORDER + LIMIT 1** (this is Pattern 2 / Pitfall 2 from RESEARCH.md — MOST RECENT, not first):
```rust
pub fn select_latest_device_mutation(
    &self,
    tx: &Transaction<'_>,
    act_id: i64,
    device_id: i64,
) -> Result<Option<String>, AppError> {
    tx.query_row(
        "SELECT before_json FROM audit_log \
         WHERE entity_type = 'device' AND entity_id = ?2 \
           AND json_extract(payload_json, '$.act_id') = ?1 \
           AND before_json IS NOT NULL \
         ORDER BY created_at_utc DESC, id DESC \
         LIMIT 1",
        params![act_id, device_id],
        |r| r.get::<_, String>(0),
    )
    .optional()
    .map_err(map_rusqlite)
}
```
(`OptionalExtension`/`.optional()` already imported in `acts_sqlite.rs`; import into `audit_log_sqlite.rs` too.)

---

### `crates/trackly-app/src/services/act_service.rs` — `ActService::update`

This is the core of ACT-02. Four analog sub-patterns apply, each already proven in this exact file.

**Analog A — CAS + guard shape, from `delete_soft`** (`act_service.rs:1203-1327`):
```rust
pub async fn delete_soft(&self, id: i64, version: i64) -> Result<(), AppError> {
    let now = self.clock.unix_seconds();
    let acts_repo = self.acts_repo.clone();
    let audit_repo = self.audit_repo.clone();
    let devices_repo = self.devices_repo.clone();
    let user_id_opt: Option<i64> = None;

    self.writer
        .execute(move |conn| {
            let tx = conn.transaction().map_err(map_rusqlite)?;

            // Optimistic-lock check + load row (включая deleted_at_utc).
            let act = acts_repo.fetch_full_in_tx(&tx, id)?;
            if act.deleted_at_utc.is_some() {
                return Err(AppError::NotFound { entity: "act", id });
            }
            if act.version != version {
                return Err(AppError::OptimisticLockMismatch {
                    entity: "act", id, expected: version, actual: act.version,
                });
            }
            // ... act_type-specific branch, mutation, soft_delete_in_tx, audit insert ...
            tx.commit().map_err(map_rusqlite)?;
            Ok(())
        })
        .await
}
```
`update` should follow the identical shape: `writer.execute(move |conn| { let tx = conn.transaction()...; fetch_full_in_tx; act.deleted_at_utc check; act.version != expected check (redundant defense-in-depth on top of the CAS UPDATE's own WHERE clause — this codebase does BOTH, see `delete_soft`); act_type guard (`ActType::Handover` only — mirrors `do_return`'s `parent.act_type != ActType::Handover` guard below); ... tx.commit(); Ok(()) }).await` then `self.get(id).await` for the fresh `ActDto` return (mirrors `do_return`'s tail: `self.get(return_act_id).await` at line 922).

**Analog B — return-act-type guard, from `do_return`** (`act_service.rs:603-608`):
```rust
if parent.act_type != ActType::Handover {
    return Err(AppError::Validation {
        field: "act_id".into(),
        message: "Возврат можно оформить только по handover-акту".into(),
    });
}
```
D-07 server-side enforcement in `update`: same shape, reject `ActType::Return` acts with a message like `"Редактировать можно только акты выдачи (handover)"`.

**Analog C — device "add" loop, from `create`** (`act_service.rs:429-471`, the per-device UPDATE + audit body — status guard on `на_складе` is above it at 344-359):
```rust
for &dev_id in &effective_device_ids {
    let before = devices_repo.get_in_tx(&tx, dev_id)?;
    let before_json = device_snapshot_json(&before).map_err(|e| AppError::Internal {
        source_chain: format!("before_json: {e}"),
    })?;
    let after = devices_repo.update_status_and_location_in_tx(
        &tx, dev_id, in_work_status_id, resolved_location_id, now,
    )?;
    let after_json = device_snapshot_json(&after).map_err(|e| AppError::Internal {
        source_chain: format!("after_json: {e}"),
    })?;
    let payload_json = serde_json::json!({
        "act_id": act_id,
        "kind": "handover",
    })
    .to_string();
    audit_repo.insert(
        &tx,
        AuditEntry {
            entity_type: "device",
            entity_id: dev_id,
            action: "update",
            user_id: user_id_opt,
            before_json: Some(before_json),
            after_json: Some(after_json),
            payload_json: Some(payload_json),
            created_at_utc: now,
        },
    )?;
}
```
Copy this loop body verbatim for `added` device_ids in `update` (canonical `device_ids[]`-only, per RESEARCH.md Pattern 3 recommendation — no legacy clone-on-handover support needed for edit-added positions). The status guard above it (344-359, `if d.status_id != on_warehouse_status_id { return Err(AppError::Conflict {...}) }`) must run first for each added device — same anti-pattern warning as RESEARCH.md's "Applying the D-06 device-add loop to devices not in на_складе status."

**Analog D — device "remove/restore" loop, from `undo_device_mutations_for_act`** (`act_service.rs:1707-1744`):
```rust
fn undo_device_mutations_for_act(
    tx: &rusqlite::Transaction<'_>,
    devices_repo: &SqliteDeviceRepository,
    audit_repo: &SqliteAuditLogRepository,
    act_id: i64,
    user_id_opt: Option<i64>,
    now: i64,
) -> Result<(), AppError> {
    let rows = audit_repo.select_device_mutations_for_act(tx, act_id)?;
    for (device_id, before_json) in rows.into_iter().rev() {
        let snapshot: serde_json::Value = serde_json::from_str(&before_json)
            .map_err(|e| AppError::Internal {
                source_chain: format!("undo: corrupt before_json for device {device_id}: {e}"),
            })?;
        let restored = devices_repo.restore_from_snapshot_in_tx(tx, device_id, &snapshot, now)?;
        let after_json = device_snapshot_json(&restored).map_err(|e| AppError::Internal {
            source_chain: format!("undo after_json: {e}"),
        })?;
        let payload_json = serde_json::json!({ "undo_of_act_id": act_id }).to_string();
        audit_repo.insert(
            tx,
            AuditEntry {
                entity_type: "device",
                entity_id: device_id,
                action: "custom:undo",
                user_id: user_id_opt,
                before_json: Some(before_json),
                after_json: Some(after_json),
                payload_json: Some(payload_json),
                created_at_utc: now,
            },
        )?;
    }
    Ok(())
}
```
For a **single removed device** during `update` (NOT a full-act undo), use the new `select_latest_device_mutation(tx, act_id, device_id)` helper (single row, already MOST-RECENT-ordered — no `.rev()` needed since it's a single row, not a Vec) instead of `select_device_mutations_for_act`+`.rev()` iteration; everything else in this loop body (parse snapshot → `restore_from_snapshot_in_tx` → audit insert) is identical. Use a distinct `action` string, e.g. `"custom:update_remove"` (per RESEARCH.md Pattern 2: "pick a name distinct from `custom:undo` so audit history is legible, but it must still carry `before_json`/`after_json` so a LATER full-act delete can still find and unwind it via `select_device_mutations_for_act`" — this means the removal's audit row's `payload_json` must still carry `{"act_id": act_id}` so it stays discoverable by the bulk query).

**Analog E — D-08 outstanding-device guard, from `populate_outstanding_device_ids`** (`act_service.rs:1846-1873`, read-path version using `&Connection`):
```rust
fn populate_outstanding_device_ids(
    conn: &rusqlite::Connection,
    act_id: i64,
    items: &mut [ActItemDto],
) -> Result<(), AppError> {
    let mut stmt = conn
        .prepare(
            "SELECT device_id FROM act_items WHERE act_id = ?1 \
             EXCEPT \
             SELECT rai.device_id FROM act_items rai \
               JOIN acts ra ON ra.id = rai.act_id \
              WHERE ra.parent_act_id = ?1 AND ra.deleted_at_utc IS NULL",
        )
        .map_err(map_rusqlite)?;
    let outstanding: std::collections::HashSet<i64> = stmt
        .query_map(params![act_id], |r| r.get::<_, i64>(0))
        .map_err(map_rusqlite)?
        .collect::<rusqlite::Result<_>>()
        .map_err(map_rusqlite)?;
    for item in items.iter_mut() {
        if outstanding.contains(&item.device_id) {
            item.outstanding_device_ids = vec![item.device_id];
        } else {
            item.outstanding_device_ids = Vec::new();
        }
    }
    Ok(())
}
```
Build a `_in_tx` twin (same SQL, `&Transaction` instead of `&Connection`, returning `HashSet<i64>` directly rather than mutating `items`) — this is Pattern 4 from RESEARCH.md. In `update`'s writer closure, run this BEFORE any mutation; for every device_id in `removed` (or replaced), if it's NOT in the outstanding set → `AppError::Conflict` and abort the whole transaction before any `tx.execute` side effect runs (validate-then-commit style, matching `validate_return`'s pre-tx validation at 515-580 combined with in-tx guards like the `handover_qty`/`already_returned` check at 781-810).

**Date-source switch inside `render_pdf`** (`act_service.rs:1447-1451`, act block) and the `parent_block` (`1402-1408`):
```rust
"act": {
    "number": act.number_raw,
    "suffix": suffix,
    "date": format_iso_date(act.created_at_utc),
    "date_human": format_ru_date(act.created_at_utc),
    ...
```
→ both lines switch to `act.handover_date_utc`. Same for `parent_block`:
```rust
Some(serde_json::json!({
    "number": parent.number,
    "date_human": format_ru_date(parent.created_at_utc),
    "date": format_iso_date(parent.created_at_utc),
}))
```
→ `parent.handover_date_utc` (both lines). **Do NOT touch** `render_acceptance_pdf` — it takes `date_utc: i64` as an explicit caller param, unrelated to `handover_date_utc` (confirmed out-of-scope by RESEARCH.md).

**Validation function to add** — `Self::validate_update(&payload: &ActUpdateDto)`, modeled on `validate_create` (115-193) and `validate_return` (515-580): non-empty giver/receiver, non-empty items (or is empty-items even valid for an update that only touches header fields? — decide: `items` in `ActUpdateDto` should be the FULL replacement set per the RESEARCH.md diagram, so empty items list is likely still invalid, matching `create`'s "Добавьте хотя бы одну позицию" rule), dedup device_ids within `items` (same `HashSet` pattern as `validate_create:145-183`), and — per RESEARCH.md Anti-Patterns/A3 — if `number_override.is_some()`, re-run the exact uniqueness check `create` does at lines 249-260 (`SELECT EXISTS(SELECT 1 FROM acts WHERE number=?1 LIMIT 1)`) plus the `custom:act_number_override`-style audit row (lines 303-323).

---

### `crates/trackly-app/src/tauri_cmds/acts.rs` — `build_acts_update` + `acts_update`

**Analog:** `build_acts_return`/`acts_return` — `tauri_cmds/acts.rs:64-72, 181-190` (id + payload shape); `build_acts_delete`/`acts_delete` — `64-72, 192-201` (id + version CAS shape).

```rust
/// Мутация: требует `caller` с правом `MutateActs`.
pub async fn build_acts_return(
    ctx: &AppCtx,
    caller: &Identity,
    act_id: i64,
    payload: ActReturnDto,
) -> Result<ActDto, AppError> {
    authorize(caller, &Action::MutateActs)?;
    ctx.acts.do_return(act_id, payload).await
}
```
```rust
#[tauri::command]
#[specta::specta]
pub async fn acts_return(
    state: tauri::State<'_, AppCtx>,
    act_id: i32,
    payload: ActReturnDto,
) -> Result<ActDto, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_acts_return(state.inner(), &caller, act_id as i64, payload).await
}
```
New `build_acts_update(ctx, caller, payload: ActUpdateDto) -> Result<ActDto, AppError>` (single-DTO shape like `build_acts_create`, since `id`/`expected_version` live inside `ActUpdateDto`, not as separate args — matches RESEARCH.md's diagram `ActUpdateDto { id, expected_version, ... }`); `authorize(caller, &Action::MutateActs)?` then `ctx.acts.update(payload).await`. Thin `#[tauri::command] acts_update(state, payload: ActUpdateDto)` wrapper follows `acts_create`'s single-payload shape (173-179) exactly, not `acts_return`'s split-args shape.

---

### `crates/trackly-app/src/http/acts.rs` — `UpdatePayload` + `handler_update` + router entry

**Analog:** `CreatePayload`/`handler_create` — `http/acts.rs:48-52, 137-150` (single-payload wrapper shape).

```rust
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePayload {
    pub payload: ActCreateDto,
}
...
pub async fn handler_create(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<CreatePayload>,
) -> Result<Json<ActDto>, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_acts_create(&ctx, &identity, p.payload)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}
```
New `UpdatePayload { pub payload: ActUpdateDto }` + `handler_update` mirroring this exactly (single `payload` field, `build_acts_update` call). Router entry pattern (`router()`, lines 271-290):
```rust
.route("/api/v1/acts_delete", post(handler_delete))
```
→ add `.route("/api/v1/acts_update", post(handler_update))` alongside the existing `acts_create`/`acts_return`/`acts_delete` entries.

---

### `crates/trackly-app/tests/role_endpoint_matrix.rs` — RBAC case for `acts_update`

**Analog:** Case 4 "Employee session → POST /api/v1/acts_create → 403 Forbidden" — `role_endpoint_matrix.rs:487-503`:
```rust
// =====================================================================
// Case 4: Employee session → POST /api/v1/acts_create → 403 Forbidden
// =====================================================================
{
    let status = post_with_cookie(
        new_app!(),
        "/api/v1/acts_create",
        act_payload.clone(),
        Some(&employee_cookie),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "Case 4: Employee → acts_create → expected 403, got {status}"
    );
}
```
Add an equivalent case posting a minimal `ActUpdateDto` JSON body to `/api/v1/acts_update` with the Employee cookie, asserting `403 FORBIDDEN` — same `post_with_cookie`/`new_app!()` helper macros already in scope in this file.

---

### `ui/src/lib/api/acts.ts` — `acts.update(...)`

**Analog:** `acts.doReturn` / `acts.delete` — `ui/src/lib/api/acts.ts:30-33`:
```typescript
/** Plan 03-03 — оформление возврата по handover-акту. */
doReturn: (actId: number, payload: ActReturnDto) =>
  apiCall<ActDto>('acts_return', { actId, payload }),

delete: (id: number, version: number) => apiCall<null>('acts_delete', { id, version }),
```
Add:
```typescript
/** Phase 19 — редактирование handover-акта (шапка + позиции, CAS через expected_version). */
update: (payload: ActUpdateDto) => apiCall<ActDto>('acts_update', { payload }),
```
Import `ActUpdateDto` from `'../../bindings'` alongside the existing `ActCreateDto`/`ActDto`/... imports (lines 12-19) — will appear automatically once `cargo test -p trackly-app --test export_bindings` regenerates `bindings.ts` after the Rust DTO exists (Pitfall 1 sequencing: Rust DTO → regenerate bindings → THEN write this TS).

---

### `ui/src/features/acts/ActFormBody.svelte` — edit-mode prop + prefill + branch submit

**Analog:** same file's create-mode `handleSubmit` (82-148) and its state declarations (30-56) — this file is extended in place, not cloned into a new file (per CONTEXT.md's discretion note recommending reuse "для консистентности").

**Current props/state shape to extend:**
```typescript
interface Props {
    onSaved: (_act: ActDto) => void;
    onLoading: (_l: boolean) => void;
    onCanSubmitChange: (_c: boolean) => void;
    onRegisterSubmit: (_fn: () => void) => void;
}
const { onSaved, onLoading, onCanSubmitChange, onRegisterSubmit }: Props = $props();

let numberOverride = $state<number | null>(null);
let giverName = $state('');
let receiverName = $state('');
let location = $state('');
let deadlineISO = $state('');
let handoverDateISO = $state(todayISO());
let notes = $state('');
let items = $state<FormItemRow[]>([
  { device_id: null, quantity: 1, device_label: '', query: '', picked: false },
]);
```
Add `mode: 'create' | 'edit'` and `initialAct: ActDto | null` props. On `mode === 'edit'`, initialize state from `initialAct` instead of defaults (`giverName = initialAct.giver_name`, `handoverDateISO` derived from `initialAct.handover_date_utc` via the inverse of `isoToUnix` at line 73-77, `items` built from `initialAct.items` — each existing item becomes a `FormItemRow` with `device_id: it.device_id, quantity: 1, device_label: it.device_name, picked: true` and NO `group_ids` (per RESEARCH.md's noted wrinkle: pre-filled existing-position rows must bypass the live warehouse-search path in `ActFormItemsTable`, since they're `в_работе`, not `на_складе`, so a live re-search would never find them — inject directly as `FormItemRow` state, not via `fetchGroups`)).

**`handleSubmit`'s payload-building branch** (82-148) — the create path:
```typescript
const payload: ActCreateDto = {
    number_override: numberOverride,
    giver_name: giverName.trim(),
    receiver_name: receiverName.trim(),
    location_id: null,
    location_name: location.trim().length > 0 ? location.trim() : null,
    notes: notes.trim() || null,
    deadline_utc: isoToUnix(deadlineISO),
    handover_date_utc: isoToUnix(handoverDateISO),
    items: payloadItems,
};
const created = await acts.create(payload);
pushToast('success', `Создан акт №${created.number}`);
onSaved(created);
```
Branch on `mode`: `edit` builds `ActUpdateDto` (adding `id: initialAct.id, expected_version: initialAct.version`) and calls `acts.update(payload)` instead of `acts.create(payload)`; toast message becomes `Акт №${saved.number} обновлён`. The `payloadItems` derivation (lines 93-103, filtering + mapping `FormItemRow[]` → `{device_id, device_ids, quantity}[]`) is reused as-is for the `items` field of `ActUpdateDto` (map to `{device_id}[]` shape per the DTO above — quantity/device_ids fields drop away since `update` is canonical-only). Error-handling branch (122-144, `err.code === 'Validation'` / `'Conflict'`) is reused verbatim — add a `err.code === 'OptimisticLockMismatch'` branch (maps to HTTP 409, per RESEARCH.md's `AppError::OptimisticLockMismatch` → HTTP 409 mapping already wired end-to-end) with a toast like "Акт был изменён другим пользователем — обновите и попробуйте снова."

---

### `ui/src/features/acts/ActFormModal.svelte` — `mode`/`initialAct` props, title switch

**Analog:** same file — `Props` interface (10-16) and `<Modal title="Новый акт" ...>` (34).

```typescript
interface Props {
    open: boolean;
    onClose: () => void;
    onSaved: (_act: ActDto) => void;
}
const { open, onClose, onSaved }: Props = $props();
...
<Modal {open} title="Новый акт" size="xwide" {onClose}>
  {#key openInstanceCounter}
    <ActFormBody
      {onSaved}
      onLoading={(l) => (formLoading = l)}
      onCanSubmitChange={(c) => (formCanSubmit = c)}
      onRegisterSubmit={(fn) => (bodySubmitFn = fn)}
    />
  {/key}
  {#snippet footer()}
    <Button variant="secondary" onclick={onClose}>Отмена</Button>
    <Button variant="primary" loading={formLoading} disabled={!formCanSubmit} onclick={() => bodySubmitFn?.()}>
      {#if formLoading}Создание…{:else}Создать акт{/if}
    </Button>
  {/snippet}
</Modal>
```
Add `mode: 'create' | 'edit'` and `initialAct: ActDto | null` props, pass both through to `ActFormBody`. Title becomes `{mode === 'edit' ? `Редактировать акт №${initialAct?.number}` : 'Новый акт'}`; footer button label becomes `{formLoading ? (mode === 'edit' ? 'Сохранение…' : 'Создание…') : (mode === 'edit' ? 'Сохранить' : 'Создать акт')}`. The `{#key openInstanceCounter}` remount-on-open pattern (18-27, 35) is reused as-is — it already guarantees a fresh `ActFormBody` instance (and thus fresh prefill from `initialAct`) each time the modal opens, which matters for Pitfall 5 (edit form must re-fetch via `acts.get(id)`, not reuse a stale `list()` row — see `ActsPage.svelte` orchestration below).

---

### `ui/src/features/acts/ActDetail.svelte` — `headerDate` source switch + `onEdit` wiring

**Analog:** same file — `headerDate` derivation (37-43) and the ALREADY-PRESENT (but currently dead, since no caller passes `onEdit`) button wiring at line 70.

```typescript
function formatDate(utcSeconds: number | null): string | null {
    if (utcSeconds === null) return null;
    const d = new Date(utcSeconds * 1000);
    return `${d.getUTCDate()} ${MONTHS_RU[d.getUTCMonth()]} ${d.getUTCFullYear()}`;
}

const headerDate = $derived(act ? formatDate(act.created_at_utc) : null);
```
→ `const headerDate = $derived(act ? formatDate(act.handover_date_utc) : null);` (requires `ActDto.handover_date_utc` to exist first — Pitfall 1).

**`onEdit` is already fully plumbed in this component** — no change needed here beyond the date switch:
```svelte
<Button variant="secondary" size="sm" onclick={() => onEdit?.(act)} disabled={!onEdit}>
  Редактировать
</Button>
```
The `Props` interface (10-18) already declares `onEdit?: (_act: ActDto) => void;` — the gap is entirely on the caller side (`ActsPage.svelte` never passes `onEdit`, confirmed by CONTEXT.md's diagnosis and RESEARCH.md's Pitfall/Integration Points sections).

---

### `ui/src/features/acts/ActListRow.svelte` — `dateLabel` source switch

**Analog:** same file — `dateLabel` derivation (31-36):
```typescript
function formatDate(utcSeconds: number): string {
    const d = new Date(utcSeconds * 1000);
    return `${d.getUTCDate()} ${MONTHS_RU[d.getUTCMonth()]} ${d.getUTCFullYear()}`;
}

const dateLabel = $derived(formatDate(act.created_at_utc));
```
→ `const dateLabel = $derived(formatDate(act.handover_date_utc));`

---

### `ui/src/features/acts/ActsPage.svelte` — `onEdit` orchestration

**Analog:** same file — the existing `Возврат` orchestration triple (`returnModalOpen`/`returnTargetAct` state at 34-35, `handleReturn` at 134-137, `<ReturnModal>` wiring at 242-250) is the closest sibling pattern (modal-open + target-act state + a handler that sets both), and the existing `createModalOpen`/`openCreate`/`handleSaved` triple (33, 119-127, 236-240) is the closest for "which acts.* API call + which toast + which refresh."

```typescript
let returnModalOpen = $state(false);
let returnTargetAct = $state<ActDto | null>(null);
...
function handleReturn(act: ActDto) {
    returnTargetAct = act;
    returnModalOpen = true;
}
...
<ActDetail
    act={selectedAct}
    loading={detailLoading}
    onCreate={openCreate}
    onDelete={handleDelete}
    onReturn={handleReturn}
    onPrint={handlePrint}
/>
...
<ReturnModal
    open={returnModalOpen}
    act={returnTargetAct}
    onClose={() => {
        returnModalOpen = false;
        returnTargetAct = null;
    }}
    onSuccess={handleReturnSuccess}
/>
```
Add `editModalOpen`/`editTargetAct` state mirroring `returnModalOpen`/`returnTargetAct`; `handleEdit(act)` mirroring `handleReturn`; pass `onEdit={handleEdit}` into `<ActDetail>` (the prop already exists on `ActDetail`, currently just never supplied — see above); render `<ActFormModal mode="edit" initialAct={editTargetAct} open={editModalOpen} onClose={...} onSaved={handleEditSaved} />` as a SEPARATE modal instance from the existing create-mode `<ActFormModal open={createModalOpen} .../>` at lines 236-240 (two `ActFormModal` instances, one per mode, is simpler than threading a shared open/mode state — matches how `ReturnModal` and the create `ActFormModal` already coexist as separate top-level modal instances in this same file). `handleEditSaved(act)` mirrors `handleSaved` (122-127: close modal, `selectedActId = act.id`, `refresh()`, `refreshCounts()`).

**Critical: `editTargetAct` MUST be populated from a fresh `acts.get(id)` call, not directly from a `list()`/`search()` row** (Pitfall 5 in RESEARCH.md — `outstanding_device_ids` is only populated on the `get()` path). `ActsPage.svelte` already does this correctly for `selectedAct` via the `$effect` at lines 89-112 (`acts.get(id).then((a) => { selectedAct = a; })`) — `handleEdit` should reuse `selectedAct` (already the fresh `acts.get()` result, since `onEdit` is only ever invoked from `ActDetail` where `act === selectedAct`) rather than re-fetching, i.e. `editTargetAct = act` inside `handleEdit(act: ActDto)` is safe BECAUSE `act` here is always `selectedAct`, which is guaranteed fresh by the existing effect.

## Shared Patterns

### Optimistic-lock CAS (`version` field)
**Source:** `crates/trackly-infra/src/repos/acts_sqlite.rs:325-362` (`soft_delete_in_tx`)
**Apply to:** `update_act_header_in_tx` (new), `ActService::update`, `ActUpdateDto.expected_version`, frontend `OptimisticLockMismatch` toast handling in `ActFormBody.svelte`.
```rust
let affected = tx.execute(
    "UPDATE acts SET <fields...>, version = version + 1, updated_at_utc = ?now \
     WHERE id = ?id AND version = ?expected_version AND deleted_at_utc IS NULL",
    params![...],
).map_err(map_rusqlite)?;
if affected == 0 {
    // distinguish NotFound vs OptimisticLockMismatch via follow-up SELECT version
}
```
`AppError::OptimisticLockMismatch` already maps to HTTP 409 end-to-end (`error_axum.rs:35`, confirmed by RESEARCH.md) — no new error-mapping work.

### Single-writer transactional orchestration
**Source:** `crates/trackly-app/src/services/act_service.rs` — `create` (200-512), `do_return` (582-923), `delete_soft` (1203-1327) — all three share the exact shape: clone `Arc` repos outside the closure → `self.writer.execute(move |conn| { let tx = conn.transaction()...; ...; tx.commit(); Ok(id_or_unit) }).await` → `self.get(id).await` for the fresh DTO.
**Apply to:** `ActService::update` must follow this identical shape — no direct `SqliteActRepository`/`SqliteDeviceRepository` calls outside a `writer.execute` closure (single-writer discipline, CLAUDE.md).

### Audit trail on every mutation
**Source:** `crates/trackly-infra/src/repos/audit_log_sqlite.rs` — `AuditEntry` struct (26-36) + `insert` (40-58); used identically in `create`/`do_return`/`delete_soft`.
**Apply to:** Every device mutation inside `ActService::update` (add/remove) AND the act-level header change itself must write an `audit_log` row — mirrors the act-level `action: "create"` row at `create`'s tail (474-500) and `do_return`'s tail (890-915); `update` should write `action: "update", entity_type: "act"` with `before_json`/`after_json` snapshots of the header fields that changed.

### `build_*` helper + thin transport wrapper (Tauri + axum share one function)
**Source:** `crates/trackly-app/src/tauri_cmds/acts.rs` (all `build_acts_*` functions) — this is S-1 per the file's own module doc.
**Apply to:** `build_acts_update` must be the single source of truth called by both `acts_update` (`#[tauri::command]`) in `tauri_cmds/acts.rs` and `handler_update` in `http/acts.rs`.

### RBAC via `authorize(caller, &Action::MutateActs)`
**Source:** every mutation `build_*` in `tauri_cmds/acts.rs` (e.g. `build_acts_create:54-61`, `build_acts_return:64-72`, `build_acts_delete:75-83`).
**Apply to:** `build_acts_update` — same `Action::MutateActs` variant, no new `Action` needed (confirmed by RESEARCH.md's Security Domain table).

### DTO snake_case JSON invariant
**Source:** `crates/trackly-app/src/dto/act.rs:368-387` (`snake_case_json_invariant` test).
**Apply to:** Add an equivalent test for `ActUpdateDto` (`expected_version`, `number_override`, `handover_date_utc` — NOT `expectedVersion`/`numberOverride`).

### Master-detail modal orchestration (open-state + target-act state + handler)
**Source:** `ui/src/features/acts/ActsPage.svelte` — the `returnModalOpen`/`returnTargetAct`/`handleReturn` triple (34-35, 134-137) and the `createModalOpen`/`openCreate`/`handleSaved` triple (33, 119-127).
**Apply to:** `editModalOpen`/`editTargetAct`/`handleEdit` — same triple shape, wired into `<ActDetail onEdit={handleEdit}>` and a second `<ActFormModal mode="edit">` instance.

## No Analog Found

None — RESEARCH.md's own conclusion holds: "Every hard part of ACT-02 (concurrency, delta reconciliation, 'is this touchable' guards) already has a canonical, tested implementation elsewhere in `act_service.rs`/`acts_sqlite.rs`/`audit_log_sqlite.rs` for the sibling operations `create`/`do_return`/`delete_soft`." All 14 files-to-touch have an exact or near-exact in-repo analog; no file requires inventing a pattern from RESEARCH.md's external knowledge alone.

## Metadata

**Analog search scope:** `crates/trackly-core/src/domain/acts.rs`, `crates/trackly-core/src/ports/acts.rs`, `crates/trackly-app/src/dto/act.rs`, `crates/trackly-app/src/services/act_service.rs` (full file), `crates/trackly-infra/src/repos/acts_sqlite.rs` (full file), `crates/trackly-infra/src/repos/audit_log_sqlite.rs` (full file), `crates/trackly-infra/src/repos/devices_sqlite.rs:300-440` (targeted), `crates/trackly-app/src/tauri_cmds/acts.rs` (full file), `crates/trackly-app/src/http/acts.rs` (full file), `crates/trackly-app/tests/role_endpoint_matrix.rs:460-503` (targeted), `ui/src/lib/api/acts.ts` (full file), `ui/src/features/acts/ActFormBody.svelte` (full file), `ui/src/features/acts/ActFormModal.svelte` (full file), `ui/src/features/acts/ActDetail.svelte` (full file), `ui/src/features/acts/ActListRow.svelte` (full file), `ui/src/features/acts/ActsPage.svelte` (full file), `ui/src/features/acts/ActFormItemsTable.svelte:1-120` (targeted, for `FormItemRow` shape).
**Files scanned:** 16
**Pattern extraction date:** 2026-07-11
