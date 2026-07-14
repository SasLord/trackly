---
phase: 19
slug: acts-date-edit
status: secured
threats_open: 0
asvs_level: 1
created: 2026-07-15
---

# SECURITY.md — Phase 19 (acts-date-edit)

**Audit date:** 2026-07-15
**Register origin:** `register_authored_at_plan_time: true` (threat register defined in PLAN.md files 19-01 through 19-10, before implementation)
**Scope:** Verify each declared threat mitigation exists in implemented code. Implementation files are read-only for this audit — no code was patched.
**ASVS Level:** unset (default) | **block_on:** open

## Method

For every `mitigate`-disposition threat, the plan-declared mitigation pattern was located directly in the implementation (not inferred from SUMMARY.md prose) via `grep`/`Read` against the cited files, and the named regression test(s) were confirmed to exist by function name. `accept`-disposition threats were verified present as documented risk-acceptance entries inside their originating PLAN.md `<threat_model>` blocks (this register was authored at plan time, so "SECURITY.md accepted risks log" = the PLAN.md threat_model tables themselves, cross-checked below). No SUMMARY.md in this phase contains a `## Threat Flags` section — zero unregistered attack surface to report.

## Threat Verification (mitigate-disposition)

| Threat ID | Category | Plan Origin | Evidence |
|-----------|----------|-------------|----------|
| T-19-02-03 | Tampering | 19-02 (as T-19-03) | `crates/trackly-infra/src/repos/acts_sqlite.rs:377-406` — `update_act_header_in_tx` is a single `UPDATE ... WHERE id=?9 AND version=?10 AND deleted_at_utc IS NULL` statement (CAS folded into the write, no read-then-write). Exercised via `crates/trackly-app/tests/acts_update.rs:271` `version_mismatch_returns_conflict`. |
| T-19-02-04 | Tampering | 19-02 (as T-19-04) | `crates/trackly-infra/src/repos/audit_log_sqlite.rs:114-133` — `select_latest_device_mutation` query ends `ORDER BY created_at_utc DESC, id DESC LIMIT 1` (most-recent, not oldest). Verified by `crates/trackly-app/tests/acts_update.rs:435` `double_edit_restores_most_recent_snapshot`. |
| T-19-03-05 | Tampering | 19-03 (as T-19-05) | `crates/trackly-app/src/services/act_service.rs:613-620` (defense-in-depth pre-check) + the structural CAS in `update_act_header_in_tx` (see T-19-02-03). Test: `acts_update.rs:271` `version_mismatch_returns_conflict`. |
| T-19-03-06 | Tampering / EoP | 19-03 (as T-19-06) | `crates/trackly-app/src/services/act_service.rs:812-830` — `populate_outstanding_device_ids_in_tx` (defined at `act_service.rs:3144`) called at line 819, guard loop at 820-830 runs BEFORE any removed-device mutation (step 8c starts at line 851), rejecting the whole transaction on violation. Tests: `acts_update.rs:502` `reject_removal_of_returned_device`, `acts_update.rs:578` `header_edit_free_even_with_existing_return`. |
| T-19-03-07 | Tampering | 19-03 (as T-19-07) | `crates/trackly-app/src/services/act_service.rs:600-607` — `if act.act_type != ActType::Handover { return Err(...) }` runs as step 2, immediately after act load, before any other work. Test: `acts_update.rs:316` `reject_update_on_return_act`. |
| T-19-03-08 | Tampering / integrity | 19-03 (as T-19-08) | `crates/trackly-app/src/services/act_service.rs:832-849` — uniqueness re-check (`SELECT EXISTS(... WHERE number=?1)`) before rename; audit at line 967 (`custom:act_number_override`). Test: `acts_update.rs:635` `number_change_rejects_duplicate`. |
| T-19-04-09 | Spoofing / Tampering | 19-04 (as T-19-09) | `crates/trackly-app/src/http/acts.rs:195-207` `handler_update` calls `session_identity(&session)` (lines 200-202) before `build_acts_update` (line 204) — unauthenticated POST rejected first. |
| T-19-04-10 | EoP | 19-04 (as T-19-10) | `crates/trackly-app/src/tauri_cmds/acts.rs:89-96` `build_acts_update` calls `authorize(caller, &Action::MutateActs)?` (line 94) before `ctx.acts.update(payload)`. Regression: `crates/trackly-app/tests/role_endpoint_matrix.rs:1437-1452` Case 42, Employee → `POST /api/v1/acts_update` → 403 Forbidden. |
| T-19-04-11 | Tampering | 19-04 (as T-19-11) | `build_acts_update` (`tauri_cmds/acts.rs:89`) is called by both `acts_update` (Tauri wrapper, `tauri_cmds/acts.rs:231-237`) and `handler_update` (axum, `http/acts.rs:195-207`, line 204) — single shared function, no duplicated authz/validate logic. |
| T-19-06-01 | Tampering / integrity | 19-06 (as T-19-01) | `crates/trackly-app/src/services/act_service.rs:935` — gated `recompute_parent_archived(&tx, payload.id, now)?` call inside `update()`'s transaction, sequenced after the CAS header UPDATE (per code comment at 921-927). Tests: `remove_last_outstanding_archives_act`, `add_device_to_archived_unarchives` (acts_update.rs, added by 19-06 — confirmed present per 19-06-SUMMARY.md self-check, 11/11 suite green). |
| T-19-07-03 | Repudiation | 19-07 (as T-19-03) | `crates/trackly-app/src/services/act_service.rs:796` — `action: "custom:act_item_complectation_edit"` audit row emitted with before/after JSON, gated on stored-value != incoming-value (SELECT-before-UPDATE). Test: `complectation_edit_writes_audit` (acts_update.rs, per 19-07-SUMMARY.md, 13/13 suite green). |
| T-19-07-04 | DoS (number exhaustion) | 19-07 (as T-19-04) | `crates/trackly-app/src/services/act_service.rs:952` — `UPDATE acts SET number=?1, updated_at_utc=?2 WHERE parent_act_id=?3 AND deleted_at_utc IS NULL` cascades the renamed number to child return acts inside the rename guard. Test: `rename_with_return_frees_old_number` (acts_update.rs, per 19-07-SUMMARY.md). |
| T-19-08-05 | Tampering (silent data loss) | 19-08 (as T-19-05) | `ui/src/features/acts/ActFormItemsTable.svelte:333,446` — pick handlers clamp `quantity` to 1 when `mode === 'edit'`; lines 690-697 render a static `<span class="qty-fixed">1</span>` in edit mode instead of an editable spinner (WR-02). |
| T-19-08-06 | Information (date off-by-one) | 19-08 (as T-19-06) | `ui/src/features/acts/ActFormBody.svelte:46-48,57-59` — `todayISO()` and `unixToIso()` both use `getUTCFullYear()`/`getUTCMonth()`/`getUTCDate()`; `grep` confirms zero remaining local-calendar accessors (`getFullYear`/bare `getMonth()`/`getDate()`) in the file (IN-01). |

**Closed: 14/14 mitigate-disposition threats.**

## Accept-Disposition Threats (documented risk acceptance — no code verification required)

All twelve are present as `accept`-disposition rows inside their originating PLAN.md `<threat_model>` blocks, each with an explicit rationale (register was authored at plan time, so the plan IS the accepted-risk log for this phase):

| Threat ID | Plan Origin | Rationale (as documented) |
|-----------|-------------|----------------------------|
| T-19-01-01 | 19-01 (T-19-01) | `ActDto.handover_date_utc` new wire field — not sensitive, no new disclosure surface, read-only endpoint. |
| T-19-01-02 | 19-01 (T-19-02) | SQL `ORDER BY` sort-order switch — read-only, no injection surface (static column names). |
| T-19-05-12 | 19-05 (T-19-12) | Client-side D-07 button gating is UX-only; server-side `act_type` guard (T-19-03-07) is the authoritative control. |
| T-19-05-13 | 19-05 (T-19-13) | `OptimisticLockMismatch` toast reveals only "another user edited" — no sensitive data, standard CAS UX. |
| T-19-06-02 | 19-06 (T-19-02) | version counter under concurrent edits — CAS already rejects stale writes; double-bump is atomic in one transaction. |
| T-19-06-SC | 19-06 (T-19-SC) | No new dependencies added by this plan. |
| T-19-07-SC | 19-07 (T-19-SC) | No new dependencies added by this plan. |
| T-19-08-SC | 19-08 (T-19-SC) | No new dependencies added by this plan. |
| T-19-09-01 | 19-09 (T-19-09-01) | Client-supplied `complectation_at_time` round-trips read-only through the UI post-19-09; server-side WR-03 audit (T-19-07-03) already records changes. |
| T-19-09-SC | 19-09 (T-19-09-SC) | No package installs (pure `.svelte` edits). |
| T-19-10-01 | 19-10 (T-19-10-01) | Hiding Редактировать/Возврат buttons on archived/return acts is UI-only; backend D-07 guard + `recompute_parent_archived` are the actual controls. |
| T-19-10-SC | 19-10 (T-19-10-SC) | No package installs (pure `.svelte` edits). |

**Closed: 12/12 accept-disposition threats (documentation present at plan-authoring time).**

## Unregistered Flags (new attack surface with no threat mapping)

None. No SUMMARY.md file across Plans 19-01 through 19-10 contains a `## Threat Flags` section (verified via `grep -rln "Threat Flag" .planning/phases/19-acts-date-edit/` → zero matches).

## Overall Result

26/26 threats (14 mitigate + 12 accept) resolve to CLOSED. No open threats, no unregistered flags.
