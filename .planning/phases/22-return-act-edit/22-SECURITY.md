---
phase: 22
slug: return-act-edit
status: verified
threats_open: 0
asvs_level: 1
created: 2026-07-15
---

# Phase 22 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.
> Register origin: `register_authored_at_plan_time: true` — this audit verifies each
> `mitigate`-disposition threat's claimed code-level mitigation against the actual
> implementation; it does not scan for brand-new threats.

---

## Trust Boundaries

| Boundary | Description |
|----------|--------------|
| Frontend → `ActUpdateReturnDto` | Untrusted client JSON deserializes into this struct; validated in `ActService::validate_update_return` + `update_return`'s in-tx guards |
| Migration V034 → `acts` table | One-time, server-controlled backfill; no user input crosses this boundary |
| `ActService::get()` → `compute_archived_at_utc` query | Server-computed, read-only aggregate over server-controlled data |
| Payload → `ActService::update_return` | Untrusted `ActUpdateReturnDto` (device_ids, version, item values) crosses into the single-writer transaction; ALL business-rule validation (D-10/D-11/type-guard/CAS/WR-01/WR-02/WR-03) is enforced inside the transaction, independent of caller |
| LAN browser / Tauri webview → `acts_update_return` | Untrusted caller identity resolved via session cookie (HTTP, `session_identity`) or OS-process trust (Tauri invoke); both converge on the same `authorize(caller, &Action::MutateActs)` call before touching `ActService` |
| Svelte UI → `acts.updateReturn` client call | UI only sends what the user edited; all authoritative validation lives server-side — no new server-trusted logic added by the UI plan |

---

## Threat Register

| Threat ID | Category | Component | Disposition | Mitigation | Status |
|-----------|----------|-----------|-------------|------------|--------|
| T-22-01-01 | Tampering | `ActReturnDto` new optional fields | mitigate | `#[serde(default)]` on `giver_name`/`receiver_name`/`handover_date_utc`. Verified `crates/trackly-app/src/dto/act.rs:160-170`. Test `dto::act::tests::act_return_dto_back_compat_omitted_giver_receiver_date` (act.rs:624) passes — confirms old client omitting all three fields does not panic deserialization. | closed |
| T-22-01-02 | Info Disclosure | `ActItemDto.device_location` | accept | Same RBAC-gated data already visible elsewhere; documented in PLAN.md at plan-authoring time. | closed |
| T-22-01-03 | Tampering | Migration V034 backfill correctness | mitigate | `migrations/V034__return_handover_date_backfill.sql` is a single deterministic `UPDATE acts SET handover_date_utc = created_at_utc WHERE act_type = 'return'` with no external input. Refinery's schema-history guarantees no re-run (verified generically by `crates/trackly-infra/tests/migration_idempotency.rs::migrations_are_idempotent_and_wal_persists_across_reopens`, which runs the full migration set including V034 twice against the same DB). | closed |
| T-22-01-04 | Info Disclosure | `ActDto.archived_at_utc` | accept | Derived from data already visible under the same RBAC gate as `get()`; documented in PLAN.md at plan-authoring time. | closed |
| T-22-02-01 | Tampering | `update_return` CAS check | mitigate | Defense-in-depth pre-check `act.version != payload.expected_version` → `AppError::OptimisticLockMismatch` at `act_service.rs:1573-1580`; structural enforcement via `update_act_header_in_tx`'s `WHERE id = ?9 AND version = ?10` at `crates/trackly-infra/src/repos/acts_sqlite.rs:391`. Test `version_mismatch_returns_conflict` (acts_update_return.rs:660) passes. | closed |
| T-22-02-02 | Tampering / business-rule bypass | D-11 device-drift guard | mitigate | 3-field snapshot compare (`status_id`/`location_id`/`state`) against `select_latest_device_mutation_pair`'s `after_json`, run for every `removed`/`retained_with_change` device BEFORE mutation; `AppError::Conflict` on mismatch, no partial apply. Verified `act_service.rs:1827-1859`. Tests `reject_un_return_after_reissue` (acts_update_return.rs:424) and `reject_edit_after_manual_device_relocation` (acts_update_return.rs:470) pass. | closed |
| T-22-02-03 | Tampering (type confusion) | `update_return` act_type guard | mitigate | `if act.act_type != ActType::Return { return Err(AppError::Validation ...) }` at `act_service.rs:1564-1569`. | closed |
| T-22-02-04 | Tampering (business-rule bypass) | D-10 empty-item-set guard | mitigate | `validate_update_return` rejects `p.items.is_empty()` before the tx opens, `act_service.rs:1470-1475`. Test `reject_empty_item_set` (acts_update_return.rs:390) passes. | closed |
| T-22-02-05 | Repudiation | `audit_log` inserts per delta | mitigate | Per-device audit rows for removed (`act_service.rs:1890-1904`), added (`:1944-1959`), retained-edit (`:2013-2028`), plus a final act-header audit row (`:2093-2105`) — all inside the same `BEGIN IMMEDIATE` transaction. Covered by all `acts_update_return.rs` tests, which assert post-mutation device/act state derived from these rows. | closed |
| T-22-03-01 | Elevation of Privilege | `build_acts_update_return` | mitigate | `authorize(caller, &Action::MutateActs)` at `crates/trackly-app/src/tauri_cmds/acts.rs:107`; Employee excluded from `Action::MutateActs`. Named test `Case 43: Employee → acts_update_return → expected 403` in `crates/trackly-app/tests/role_endpoint_matrix.rs:1473` — run and passed (`cargo test -p trackly-app --test role_endpoint_matrix`). | closed |
| T-22-03-02 | Tampering | Transport parity (HTTP vs Tauri) | mitigate | Both `crates/trackly-app/src/http/acts.rs:210-223` (`handler_update_return`, resolves `session_identity` then calls `build_acts_update_return`) and the Tauri command `acts_update_return` (`tauri_cmds/acts.rs:241-246`) converge on the single `build_acts_update_return` → `ActService::update_return` path — all Plan 22-02 validation applies unconditionally. | closed |
| T-22-04-01 | Tampering (business-rule bypass) | No proactive UI hint for D-11-unsafe rows | accept | Authoritative check lives server-side (`ActService::update_return`), cannot be bypassed via raw HTTP; documented in PLAN.md at plan-authoring time, precedent from Phase 19 D-08. | closed |
| T-22-04-02 | Info Disclosure | `ActsPage` fetching parent act for edit-mode prefill | accept | Reuses existing RBAC-gated `acts.get` endpoint; no new data-access surface; documented in PLAN.md at plan-authoring time. | closed |
| T-22-04-03 | Info Disclosure | `ActDetail.svelte` displaying `archived_at_utc` | accept | Client-side rendering of a value already returned by the existing `acts.get()` response; documented in PLAN.md at plan-authoring time. | closed |
| T-22-05-01 | Tampering / Data Integrity | `update_return` added/retained device loops (CR-01) | mitigate | `location.or(before.location_id)` in both the "added" loop (`act_service.rs:1931`) and the "retained_with_change" loop (`act_service.rs:2000`) — an `apply_to_all=true` submit with no bulk location can no longer NULL a device's stored location. Tests `add_outstanding_device_without_bulk_location_preserves_current_location` (acts_update_return.rs:780) and `retained_edit_condition_only_preserves_location` (acts_update_return.rs:736) pass. | closed |
| T-22-05-02 | Repudiation / Data Integrity | `update_return` un-return restore + audit tagging (CR-02) | mitigate | Retained-edit audit rows tagged `action: "custom:return_item_edit"` at `act_service.rs:2018`; `select_latest_device_mutation` (used by the un-return restore path) excludes this tag via `AND action != 'custom:return_item_edit'` at `crates/trackly-infra/src/repos/audit_log_sqlite.rs:126`. Test `un_return_after_retained_edit_restores_original_pre_return_state` (acts_update_return.rs:817) passes. | closed |
| T-22-05-03 | Tampering | `select_latest_device_mutation` shared helper — inertness for handover-edit caller | accept | The exclusion clause is inert for `update()`'s handover-edit un-return path, which never writes `custom:return_item_edit` rows — verified by reading both call sites (`act_service.rs` `update()`'s removed-device restore and `update_return()`'s step 9); documented in PLAN.md and in the repo helper's own doc comment (`audit_log_sqlite.rs:105-113`). | closed |
| T-22-06-01 | Tampering / business-rule bypass | `validate_update_return` (WR-01) | mitigate | Dedup (`HashSet` insert check), non-empty `device_ids`, and per-item override requirement when `apply_to_all=false`, mirroring `validate_return`. Verified `act_service.rs:1485-1516`. Tests `reject_update_return_duplicate_device_id_across_items` (acts_update_return.rs:872) and `reject_update_return_missing_override_when_apply_to_all_false` (acts_update_return.rs:906) pass. | closed |
| T-22-06-02 | Tampering / business-rule bypass | `update_return` step 8a "added" loop (WR-03) | mitigate | `already_returned + per_device_qty > handover_qty` bound ported from `do_return`, rejects with `AppError::Validation`. Verified `act_service.rs:1739-1773`. Test `reject_add_when_device_already_returned_elsewhere_under_parent` (acts_update_return.rs:937) passes. | closed |
| T-22-06-03 | Denial of Service | `update_return` writer closure `parent_act_id` resolution (WR-02) | mitigate | `act.parent_act_id.ok_or_else(|| AppError::Internal { ... })?` at `act_service.rs:1589-1594` — no `.expect()`/panic. Test `update_return_null_parent_act_id_returns_error_not_panic` (acts_update_return.rs:988) passes — directly corrupts `parent_act_id` to NULL via SQL and asserts `AppError::Internal` is returned, not a panic. | closed |
| T-22-06-04 | Info Disclosure (operational) | `migrations/V034__return_handover_date_backfill.sql` comment (WR-04) | mitigate | Comment explicitly states the UPDATE is "NOT safe to run manually after Phase 22 ships" and explains why (would silently clobber user-edited «Дата возврата» values). Verified in the migration file body. | closed |

*Status: open · closed*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Unregistered Flags (new attack surface without threat mapping)

None. All 6 plan SUMMARY.md files (`22-01` through `22-06`) were checked for a `## Threat Flags` section; only `22-03-SUMMARY.md` mentions threat coverage at all, and it references the already-registered T-22-03-01 (informational note confirming no new `Action` variant was introduced). No new attack surface was flagged by any plan's executor without a corresponding threat register entry.

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-22-01 | T-22-01-02 | `ActItemDto.device_location` exposes location names already visible elsewhere to any authenticated caller with ReadData/MutateActs; same data, new field, no new disclosure surface. | Phase 22 owner (Alexander Platov) | 2026-07-15 |
| AR-22-02 | T-22-01-04 | `ActDto.archived_at_utc` is derived from `handover_date_utc` values already visible under the same RBAC gate as `get()`'s own detail view; a new read-time aggregate of existing visible dates, not new data. | Phase 22 owner (Alexander Platov) | 2026-07-15 |
| AR-22-03 | T-22-04-01 | No proactive UI hint for D-11-unsafe rows — authoritative check is server-side-only (matches Phase 19 D-08 precedent); a client-side hint would give a false sense of protection without adding real safety. | Phase 22 owner (Alexander Platov) | 2026-07-15 |
| AR-22-04 | T-22-04-02 | `ActsPage` parent-act fetch for edit-mode prefill reuses the existing RBAC-gated `acts.get` endpoint; no new data-access surface beyond existing detail-view navigation. | Phase 22 owner (Alexander Platov) | 2026-07-15 |
| AR-22-05 | T-22-04-03 | `ActDetail.svelte` rendering `archived_at_utc` is a pure client-side render of a value the backend already returns on the same `acts.get()` response; no new API call or RBAC surface. | Phase 22 owner (Alexander Platov) | 2026-07-15 |
| AR-22-06 | T-22-05-03 | `select_latest_device_mutation`'s new `custom:return_item_edit` exclusion clause is inert for `update()`'s handover-edit un-return caller, which never writes that action tag — verified by reading both call sites before modifying the shared query. | Phase 22 owner (Alexander Platov) | 2026-07-15 |

*Accepted risks do not resurface in future audit runs.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-07-15 | 20 | 20 | 0 | gsd-secure-phase (register verified against implementation; all `mitigate` dispositions confirmed present via grep + `cargo test -p trackly-app --test acts_update_return --test role_endpoint_matrix` (19 tests passed) and `cargo test -p trackly-app --lib act_return_dto_back_compat_omitted_giver_receiver_date` (1 test passed); `accept` dispositions confirmed documented in this file's Accepted Risks Log) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-07-15
