---
phase: 19
slug: acts-date-edit
status: approved
nyquist_compliant: true
wave_0_complete: false
created: 2026-07-11
---

# Phase 19 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Derived from 19-RESEARCH.md § Validation Architecture.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `cargo test` — rusqlite integration tests against `trackly_infra::test_support::test_writer_and_readers` (tempfile-backed real SQLite, WAL + migrations). No frontend unit-test framework (no vitest/jest); frontend correctness = `svelte-check` + human-verify checkpoints. |
| **Config file** | none — pattern lives in existing `crates/trackly-app/tests/acts_*.rs` |
| **Quick run command** | `cargo test -p trackly-app --test acts_update` |
| **Full suite command** | `cargo test --workspace` (ONE `cargo test` at a time — concurrent runs contend on the `target/` lock) |
| **Estimated runtime** | ~60–120 seconds (workspace) |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p trackly-app --test acts_update` plus whichever existing file is extended for ACT-01 (`acts_display_rule.rs` / `html_act_render.rs`)
- **After every plan wave:** Run `cargo test --workspace` (single invocation — no concurrent `cargo test`)
- **Before `/gsd-verify-work`:** Full suite green **and** `pnpm --dir ui build` (LAN/browser mode serves `ui/dist`, not HMR)
- **Max feedback latency:** ~120 seconds

---

## Per-Task Verification Map

| Req | Behavior | Test Type | Automated Command | File Exists |
|-----|----------|-----------|-------------------|-------------|
| ACT-01 | List/detail/PDF/sort key off `handover_date_utc`, not `created_at_utc` | integration | `cargo test -p trackly-app --test acts_display_rule` (extend) or new `acts_date_source.rs` | ❌ W0 |
| ACT-01 | HTML act render shows `handover_date_utc`-derived date string | integration | `cargo test -p trackly-app --test html_act_render` (extend) | ✅ extend |
| ACT-02 | Header-only edit, device state untouched (D-05) | integration | `cargo test -p trackly-app --test acts_update -- header_only_edit_does_not_touch_devices` | ❌ W0 |
| ACT-02 | Add position (device на_складе → в_работе) | integration | `cargo test -p trackly-app --test acts_update -- add_position_transitions_device` | ❌ W0 |
| ACT-02 | Remove position restores device to prior state (D-06) | integration | `cargo test -p trackly-app --test acts_update -- remove_position_restores_prior_state` | ❌ W0 |
| ACT-02 | Restore to MOST RECENT prior state, not original (Pitfall 2) | integration | `cargo test -p trackly-app --test acts_update -- double_edit_restores_most_recent_snapshot` | ❌ W0 |
| ACT-02 | Version mismatch → `OptimisticLockMismatch` (409) | integration | `cargo test -p trackly-app --test acts_update -- version_mismatch_returns_conflict` | ❌ W0 |
| ACT-02 | D-08: removing a returned-bound device → rejected | integration | `cargo test -p trackly-app --test acts_update -- reject_removal_of_returned_device` | ❌ W0 |
| ACT-02 | D-08: header edit on act with existing return still succeeds | integration | `cargo test -p trackly-app --test acts_update -- header_edit_free_even_with_existing_return` | ❌ W0 |
| ACT-02 | D-07: return-act update rejected server-side | integration | `cargo test -p trackly-app --test acts_update -- reject_update_on_return_act` | ❌ W0 |
| ACT-02 | Act-number edit re-validates uniqueness (A3) | integration | `cargo test -p trackly-app --test acts_update -- number_change_rejects_duplicate` | ❌ W0 |
| ACT-02 | RBAC: `acts_update` gated by `Action::MutateActs` (Employee rejected) | integration | extend `crates/trackly-app/tests/role_endpoint_matrix.rs` | ❌ W0 |
| ACT-02 (UI) | Edit form prefilled from `acts.get(id)`, not stale `list()` row (Pitfall 5) | manual / human-verify | N/A — checkpoint-gated (`human_verify_mode: end-of-phase`) | N/A |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/trackly-app/tests/acts_update.rs` — new file, covers all ACT-02 rows above
- [ ] Extend `crates/trackly-app/tests/acts_display_rule.rs` (or add `acts_date_source.rs`) — ACT-01 list/sort assertions
- [ ] Extend `crates/trackly-app/tests/html_act_render.rs` — ACT-01 PDF/HTML date assertions
- [ ] Extend `crates/trackly-app/tests/role_endpoint_matrix.rs` — `acts_update` RBAC case (mirror `acts_delete`)
- [ ] Extend `crates/trackly-app/tests/export_bindings.rs` — assert `ActDto.handover_date_utc` + `ActUpdateDto`/`acts_update` presence (pattern at lines 193–242)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Edit form prefills from `acts.get(id)` and saves without error; date shows «Когда отдали» value | ACT-02, ACT-01 | UI behavior, no frontend unit-test framework | Open an existing handover act → «Редактировать» → verify all header + position fields prefilled → change header + add/remove a position → save → verify success + list/detail «Дата» = handover date |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies (checker confirmed 8a–8d)
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 120s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-07-11
