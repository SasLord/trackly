---
phase: 22
slug: return-act-edit
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-07-12
---

# Phase 22 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (rusqlite integration tests via `trackly_infra::test_support`, tempfile-backed real SQLite, WAL + migrations applied). No frontend unit-test framework in repo — frontend correctness via `svelte-check` + human-verify checkpoints. |
| **Config file** | none — pattern lives in existing test files (`crates/trackly-app/tests/acts_update.rs`, `acts_returns.rs`, `acts_undo.rs`, `acts_date_source.rs`) |
| **Quick run command** | `cargo test -p trackly-app --test acts_update_return` |
| **Full suite command** | `cargo test --workspace` (never run two `cargo test` invocations concurrently — they contend on the `target/` lock) |
| **Estimated runtime** | ~60–120 seconds (workspace); ~10–20s for the single new test file |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p trackly-app --test acts_update_return` (once created) plus whichever existing file is extended (`acts_returns.rs`, `acts_date_source.rs`, `role_endpoint_matrix.rs`)
- **After every plan wave:** Run `cargo test --workspace` (single invocation)
- **Before `/gsd-verify-work`:** Full suite green + `pnpm --dir ui build` (LAN/browser mode serves `ui/dist`, not HMR)
- **Max feedback latency:** ~20 seconds (single test file)

---

## Per-Task Verification Map

| Behavior | Requirement | Test Type | Automated Command | File Exists |
|----------|-------------|-----------|-------------------|-------------|
| Return-edit happy path: change condition/location on a retained returned device | ACT-03 | integration | `cargo test -p trackly-app --test acts_update_return -- retained_edit_changes_device_condition_location` | ❌ W0 |
| Un-return (remove device) restores device to prior в_работе state (D-09.1) | ACT-03 | integration | `cargo test -p trackly-app --test acts_update_return -- un_return_restores_prior_state` | ❌ W0 |
| Add outstanding device to an existing return (D-09.3) | ACT-03 | integration | `cargo test -p trackly-app --test acts_update_return -- add_outstanding_device_to_return` | ❌ W0 |
| D-10: empty item set (all unchecked) rejected | ACT-03 | integration | `cargo test -p trackly-app --test acts_update_return -- reject_empty_item_set` | ❌ W0 |
| D-11: un-return of device re-issued by a later handover rejected | ACT-03 | integration | `cargo test -p trackly-app --test acts_update_return -- reject_un_return_after_reissue` | ❌ W0 |
| D-11: edit of retained device relocated via manual device-page edit rejected | ACT-03 | integration | `cargo test -p trackly-app --test acts_update_return -- reject_edit_after_manual_device_relocation` | ❌ W0 |
| D-11: edit/un-return when device untouched since return succeeds | ACT-03 | integration | `cargo test -p trackly-app --test acts_update_return -- allow_edit_when_device_untouched` | ❌ W0 |
| archived flips false→true when edit adds the last outstanding device | ACT-03 | integration | `cargo test -p trackly-app --test acts_update_return -- add_last_device_archives_parent` | ❌ W0 |
| archived flips true→false when un-return removes device from fully-returned parent | ACT-03 | integration | `cargo test -p trackly-app --test acts_update_return -- un_return_unarchives_parent` | ❌ W0 |
| Version mismatch → `OptimisticLockMismatch` (409) | ACT-03 | integration | `cargo test -p trackly-app --test acts_update_return -- version_mismatch_returns_conflict` | ❌ W0 |
| D-12: giver_name/receiver_name persist as submitted (create AND edit) | ACT-03 | integration | `cargo test -p trackly-app --test acts_returns -- create_persists_giver_receiver_from_payload` + `acts_update_return -- edit_persists_giver_receiver` | ❌ W0 |
| D-05/D-08: `handover_date_utc` write-site uses payload date; migration backfills existing rows to created_at_utc | ACT-03 | integration | `cargo test -p trackly-app --test acts_date_source -- do_return_persists_own_date` + migration backfill test | ❌ W0 |
| RBAC: `acts_update_return` gated by `Action::MutateActs` (Employee rejected) | ACT-03 | integration | extend `crates/trackly-app/tests/role_endpoint_matrix.rs` (mirror `acts_update` case) | ❌ W0 |
| Edit form prefilled from return's own items AND parent's outstanding items (not stale `list()` row) | ACT-03 (UI) | manual / human-verify | N/A — checkpoint-gated (`human_verify_mode: end-of-phase`) | N/A |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/trackly-app/tests/acts_update_return.rs` — new file, covers all ACT-03 backend rows above
- [ ] Extend `crates/trackly-app/tests/acts_returns.rs` — add `create_persists_giver_receiver_from_payload` (Pitfall 1 regression guard)
- [ ] Extend `crates/trackly-app/tests/acts_date_source.rs` — assert `do_return` persists payload's own `handover_date_utc`, not parent's
- [ ] Migration test for `V034__return_handover_date_backfill.sql` idempotency (apply-twice-safe; confirm project migration suite auto-picks V034)
- [ ] Extend `crates/trackly-app/tests/role_endpoint_matrix.rs` — add `acts_update_return` RBAC case
- [ ] Regenerate `ui/src/bindings.ts` via `cargo test -p trackly-app --test export_bindings` once `ActUpdateReturnDto` / `acts_update_return` / extended `ActReturnDto` / `ActItemDto` exist — extend `export_bindings.rs` assertions

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Edit form prefill (return items + outstanding), «Дата возврата» date-picker, ФИО prefill, per-row + apply_to_all | ACT-03 | Svelte UI behavior; no frontend unit-test framework | Open a return card → «Редактировать» → verify dialog «Возврат по акту №XXX» prefilled with saved values; change composition/state/date; save; confirm device effects + parent archived flag update in detail view |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 20s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
