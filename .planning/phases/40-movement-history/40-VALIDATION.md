---
phase: 40
slug: movement-history
status: approved
nyquist_compliant: true
wave_0_complete: true
created: 2026-09-01
verified: 2026-09-02
validation_audit: 2026-09-17
validation_audit_result: compliant
---

# Phase 40 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Derived from `40-RESEARCH.md` § Validation Architecture.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (workspace, plain `#[test]` / `#[tokio::test]`); no `nextest` in CI |
| **Config file** | none — tests live in `crates/*/tests/*.rs` and inline `#[cfg(test)] mod tests` |
| **Quick run command** | `cargo test -p trackly-app <substring> -- --test-threads=1` |
| **Full suite command** | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test --workspace --no-fail-fast -- --test-threads=1` |
| **Frontend gates** | `pnpm --dir ui svelte-check` + `pnpm --dir ui lint` (chains eslint, prettier --check, 7 `check-*.mjs` scripts) |
| **Estimated runtime** | ~180-300 seconds (full workspace suite) |

**Hazards (project memory, must be respected during execution):**
- Never run two `cargo test` invocations concurrently — they contend on the `target/` lock and
  look like a multi-minute hang.
- `login_remember_persistent_cookie` is a pre-existing hang; run whole-package with `--skip`
  when it bites, and never treat "my files are green" as a package gate.
- Rebuild `ui/dist` (`pnpm --dir ui build`) before any test or manual check that serves the
  embedded SPA — `cargo tauri dev` only HMRs the desktop webview.

---

## Sampling Rate

- **After every task commit:** targeted `cargo test -p trackly-app <substring> -- --test-threads=1`
  for the touched write site, plus `pnpm --dir ui svelte-check` when `.svelte` files changed.
- **After every plan wave:** full workspace suite + `pnpm --dir ui lint` + `pnpm --dir ui build`.
- **Before `/gsd-verify-work`:** full suite green, including the new `role_endpoint_matrix.rs` cases.
- **Max feedback latency:** ~60 seconds for the targeted run.

---

## Per-Task Verification Map

Populated by `gsd-planner` — each task's `<automated>` verify command must map to one row below.
Requirement -> test mapping the planner must honor:

| Req ID | Behavior | Test Type | Automated Command | File Exists |
|--------|----------|-----------|-------------------|-------------|
| HST-01 | Manual device place change (D-27) writes one row, `source='manual'` | integration | `cargo test -p trackly-app place_movements_manual_device -- --test-threads=1` | W0 |
| HST-01 | Save with unchanged place (D-04) writes zero rows | integration | same file | W0 |
| HST-01 | Cartridge transition into printer writes one row with an operation-derived `note` (D-05); `source` stays 'manual' per D-07 | integration | `cargo test -p trackly-app place_movements_cartridge_transition -- --test-threads=1` | W0 |
| HST-01 | Nested auto-return inside `transition_in_tx` writes a SECOND row (Pitfall 3) | integration | same file | W0 |
| HST-01 | place -> NULL (return act, no override) writes ZERO rows (D-06, Pitfall 4) | integration | `cargo test -p trackly-app place_movements_null_place_skip -- --test-threads=1` | W0 |
| HST-01 | NULL -> place (first assignment) writes ZERO rows (D-06) | integration | same file | W0 |
| HST-01 | `user_id IS NULL` row surfaces as «система» (D-11) | unit + integration | `cargo test -p trackly-app place_movements_system_actor` | W0 |
| HST-01 | Unknown `source` token degrades softly, no crash (Pitfall 6 / IN-01) | integration | `cargo test -p trackly-app place_movements_unknown_source_degrades` | W0 |
| HST-02 | Timeline query returns newest-first, unpaginated (D-20) | unit | `cargo test -p trackly-infra place_movements_history_order` | W0 |
| HST-02 | Printer timeline reads the SAME rows as its device (D-21) | integration | `cargo test -p trackly-app place_movements_printer_is_device` | W0 |
| HST-03 | Handover writes a row with `act_id` set | integration | `cargo test -p trackly-app place_movements_act_link` | W0 |
| HST-03 | Deleting a handover act deletes its movement rows (D-03) | integration | `cargo test -p trackly-app place_movements_act_undo_deletes` | W0 |
| HST-03 | Nested-return cascade deletes each act's own rows, correctly scoped (Pitfall 5) | integration | same file | W0 |
| HST-04 | Both place filters set -> AND semantics (D-24 «со склада в Здание Б») | integration | `cargo test -p trackly-app report_movements_place_filters` | W0 |
| HST-04 | Both filters subtree-inclusive (D-24 <- Phase 39 D-28) | integration | same file | W0 |
| HST-04 | Soft-deleted item still appears, marked «удалено» (D-25) | integration | `cargo test -p trackly-app report_movements_deleted_item_marker` | W0 |
| HST-04 | `columns_for` / `column_labels_for` index alignment holds for the new report type | unit | `cargo test -p trackly-app column_labels_for_is_index_aligned_with_columns_for` | extend existing |
| HST-01..04 | Role matrix: Manager allowed / Employee 403 on BOTH transports for every new endpoint | integration | `cargo test -p trackly-app role_endpoint_matrix` | extend (Case 45-48 shape) |
| HST-01 / UAT3-01 | `OperationModal.validate()` — «Место» для install обязательно только на легаси-пути (`effectivePrinterId === undefined`); для to_refill — безусловно | structural gate | `node ui/scripts/check-operation-modal-validate.mjs` | audit 2026-09-17 |
| UAT3-01, UAT4-02/03, UAT5-01 | Автозаполнение: раздельные `$effect` на op, per-field WR-01 guard в момент разрешения промиса, сброс `placeAutofilled` при ручном выборе, install-очистка гейтована `op === 'install'` | structural gate | `node ui/scripts/check-operation-modal-autofill.mjs` | audit 2026-09-17 |
| UAT3-03 | Инвалидация `statsCache` в `PlaceTree`: чтение через `untrack()`, УСЛОВНАЯ обратная запись, охват предков | structural gate | `node ui/scripts/check-place-tree-invalidation.mjs` | audit 2026-09-17 |
| Privacy | New fixtures use invented ФИО only; gate stays green | gate | `node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256` | existing |

---

## Wave 0 Requirements

> **Verified 2026-09-02 by `gsd-plan-checker`:** every item below is covered by a plan, but the
> test files are distributed across more (and differently named) files than originally suggested
> here. Actual mapping: `place_movements_write_sites.rs` became
> `place_movements_write_sites_devices.rs` (40-07) + `place_movements_write_sites_cartridges.rs`
> (40-08); system-actor and unknown-source coverage lives in `place_movements_timeline.rs` (40-10);
> NULL-skip coverage lives in `place_movements_act_link.rs` (40-09), which 40-20 then extends with
> the undo-deletion tests. Functional coverage is complete — only the filenames below are stale.

- [x] `crates/trackly-infra/tests/place_movements_migration.rs` (or extend `migration_idempotency.rs`)
      — new migration idempotency + fresh-DB check, mirroring the V037-V039 pattern.
- [x] `crates/trackly-app/tests/place_movements_write_sites.rs` — the six write-site integration
      tests (manual device, manual cartridge, transition, nested auto-return, null-skip both
      directions, system actor).
- [x] `crates/trackly-app/tests/place_movements_act_link.rs` — HST-03 act link + D-03 undo,
      including the nested-cascade case.
- [x] `crates/trackly-app/tests/report_movements.rs` — HST-04 filters, subtree inclusion,
      soft-deleted marker, CSV/PDF export.
- [x] Extend `crates/trackly-app/tests/role_endpoint_matrix.rs` with new Cases for the movements
      read endpoints and the new report endpoint (Manager allow / Employee 403, both transports).
- [x] Test ФИО fixtures use invented names only («Иванов И.И.», «Петров П.П.») — first time a
      table other than `users`/`acts` stores a ФИО snapshot, so state it explicitly in the plan.
- [x] No new JS mirror of the path-shortening formula. If one is added anyway, it needs a shared
      golden fixture + `check-*.mjs` gate — the preferred outcome is a single server-side owner.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Timeline renders in the device modal without rune runtime errors | HST-02 | `svelte-check` / eslint do not see Svelte 5 rune runtime errors | Run the app, open a device with >=2 movements, confirm timeline renders and console is clean |
| Timeline renders in the cartridge modal alongside the existing history section (D-16) | HST-02 | Same — plus visual check that neither section was lost | Open a cartridge with both operation history and movements |
| Shortened path + full-path tooltip (D-17/D-18) reads correctly at real widths | HST-02 | Layout/overflow is not visible to text-level assertions | Hover a long stored path in the timeline; confirm tooltip shows the full snapshot |
| Movements report PDF export renders (Cyrillic, pagination, no template preview breakage) | HST-04 | Template-preview strict-undefined and PDF overlap are invisible to extraction tests | Export the report to PDF and open it; then open the template editor and confirm preview still renders |
| LAN-browser parity for the report and the timeline | HST-02, HST-04 | Print/DOM cascade leakage is desktop-vs-browser asymmetric | `pnpm --dir ui build`, then repeat both checks in a LAN browser session |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or a Wave 0 dependency — 1:1 with task count across all 20 plans
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all missing references above (see the filename note in Wave 0 Requirements)
- [x] No watch-mode flags
- [x] Feedback latency < 60s for targeted runs
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-09-02 — `gsd-plan-checker` iteration 2/3 returned VERIFICATION PASSED
(0 blockers) and confirmed every test-function name in the Per-Task Verification Map is created by
a plan and traced verbatim in that plan's grep-based acceptance criteria.

---

## Validation Audit 2026-09-17

| Metric | Count |
|--------|-------|
| Gaps found | 3 |
| Resolved | 3 |
| Escalated | 0 |

**Scope of this audit.** This document was approved 2026-09-02 against the phase's first 20 plans;
the phase then grew to 35. The audit re-checked every row of the Per-Task Verification Map against
HEAD and swept plans 21-35 for uncovered work.

**Backend: no gaps.** Every test-function name in the map above resolves to a real, green test at
HEAD (`place_movements_write_sites_devices.rs`, `place_movements_write_sites_cartridges.rs`,
`place_movements_act_link.rs`, `place_movements_timeline.rs`, `place_movements_bulk_move.rs`,
`report_movements.rs`, `place_movements_migration.rs`, `place_movements_repo.rs`,
`role_endpoint_matrix.rs` Cases 52-55, 60-63). Plans 21-22, 24, 26, 28-30, 33-34 each landed
additional Rust tests of their own.

**Frontend: 3 gaps, all closed.** Plans 40-23, 40-31, 40-32 and 40-35 changed interlocking Svelte 5
rune logic and were verified only by a one-time live browser check — three of the four had shipped a
live defect (UAT3-01b/UAT5-01 cross-effect clobber, UAT3-03a infinite effect loop) that no compile
gate could see. This project has no UI test runner; the established closure pattern is a structural
gate in `ui/scripts/` wired into `pnpm lint` (the pattern plans 40-25 and 40-27 used in this very
phase). Three gates added:

| Gate | Locks | Mutations caught |
|------|-------|------------------|
| `check-operation-modal-validate.mjs` | install place requirement scoped to the legacy path; to_refill unconditional | 3/3 |
| `check-operation-modal-autofill.mjs` | split effects per op; distinct per-field WR-01 guards inside `.then` (no hoist, no combined guard); manual pick resets `placeAutofilled`; install-cleanup gated on `op === 'install'` | 7/7 |
| `check-place-tree-invalidation.mjs` | `untrack()`-ed reads; conditional write-back; ancestor expansion; store reads stay reactive; neighbouring lazy-fetch gate intact; producer bumps `seq` | 7/7 |

Each anchor was verified unique (`grep -c` = 1) before trusting its mutation result — a non-unique
anchor silently disarms a neighbouring gate and makes the mutation "pass" in a vacuum.
Three mutations were additionally re-run independently by the orchestrator on a fresh scratch copy.

**Limit of the guarantee — stated plainly.** These gates are structural: they prove the invariant's
shape survives refactoring, not that the runes behave correctly at runtime. They execute no Svelte.
Runtime behavior (no infinite loop, no cross-effect clobber, correct prefill) stays a live-verification
item under § Manual-Only Verifications — the gates shorten the feedback loop for regressions, they do
not replace that check.

**Verified this audit:** `pnpm --dir ui lint` — full 14-gate chain green. `pnpm --dir ui svelte-check`
— 285 files, 0 errors. `node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256` — 0
violations. No implementation file was modified; only `ui/package.json`'s `lint` script was extended,
additively, with every pre-existing entry preserved verbatim.
