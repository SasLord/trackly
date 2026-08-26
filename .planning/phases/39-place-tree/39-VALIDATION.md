---
phase: 39
slug: place-tree
status: approved
nyquist_compliant: true
wave_0_complete: true
created: 2026-08-22
approved: 2026-08-22
audited: 2026-08-26
---

# Phase 39 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Derived from `39-RESEARCH.md` § Validation Architecture.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (Rust integration tests, workspace-standard) + `svelte-check` / `eslint` / `pnpm build` (frontend static gates) |
| **Config file** | none dedicated — standard `Cargo.toml` auto-discovery in `crates/trackly-app/tests/` and `crates/trackly-infra/tests/` |
| **Quick run command** | `cargo test -p trackly-infra --test migration_idempotency` |
| **Full suite command** | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app -- --skip login_remember_persistent_cookie --test-threads=1` |
| **Estimated runtime** | ~5s quick / ~3–5 min full |

### Known Test Constraints (repo-specific — MANDATORY)

- **Never run two `cargo test` invocations concurrently** — they contend on the `target/` lock and
  present as a multi-minute apparent hang.
- **`login_remember_persistent_cookie`** (`crates/trackly-app/tests/auth_remember_cookie.rs`) is a
  pre-existing hanging test unrelated to this phase. Every `-p trackly-app` run MUST pass
  `-- --skip login_remember_persistent_cookie`.
- **`TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1`** must be set for any full `trackly-app` run.
- **`pnpm --dir ui build`** must be re-run after any frontend change before LAN-browser
  verification — `cargo tauri dev` only HMRs the desktop webview, not the axum-served bundle.
  Relevant here because this phase adds the `/places` route.

---

## Sampling Rate

- **After every task commit:** targeted `cargo test -p <crate> --test <test_file>` for the file just
  touched (<30s).
- **After every plan wave:** `cargo test -p trackly-infra --test migration_idempotency` +
  full-package `trackly-app` run with the documented skip.
- **Before `/gsd-verify-work`:** full suite green + `cargo clippy --all-targets -- -D warnings` +
  `svelte-check` 0 errors + `pnpm --dir ui build` succeeds.
- **Max feedback latency:** 30 seconds per task, ~5 min per wave.

---

## Per-Task Verification Map

*Populated by `gsd-planner` — each task's `<automated>` verify command lands here. The requirement
→ test mapping below is the contract the task map must satisfy.*

| Req / Decision | Behavior | Test Type | Automated Command | File Exists | Status |
|---------|----------|-----------|-------------------|-------------|--------|
| PLC-01 | Build tree, rename, move subtree without losing device FK bindings | integration | `cargo test -p trackly-infra --test places_crud` | ✅ | ✅ green (8) |
| PLC-01 | Cycle rejection on move (node cannot become its own descendant) | integration | `cargo test -p trackly-app --test places_move_cycle` | ✅ | ✅ green (2) |
| PLC-02 | Floor `level` accepts 0 and negatives; siblings sort by level, not name | unit | `cargo test -p trackly-core places` | ✅ | ✅ green (11, incl. D-20 auth) |
| PLC-03 | Full-path search, incl. Cyrillic case-insensitivity (Rust-side lowercase, not SQL `LIKE`) | integration | `cargo test -p trackly-app --test places_search` | ✅ | ✅ green (5) |
| PLC-04 | `locations` table and all `location_id` / free-text location columns removed post-migration | integration (schema assertion) | `cargo test -p trackly-infra --test migration_idempotency` | ✅ extended | ✅ green (2) |
| PLC-05 | Rename/move instantly reflected in search + all lists, no manual reindex step | integration | `cargo test -p trackly-infra --test devices_place_search --test cartridges_place_search` | ✅ **path deviation** | ✅ green (5+4) |
| PLC-06 | Place-contents screen returns nested items by default; toggle limits to direct children only | integration | `cargo test -p trackly-app --test places_contents` | ✅ | ✅ green (3) |
| D-14 | Delete blocked with exact counts when place is non-empty | integration | `cargo test -p trackly-app --test places_delete_blocked` | ✅ | ✅ green (6, incl. CR-01 act-ref counting) |
| D-16 | Act stores `place_id` + frozen path snapshot; later rename does not alter a printed act | integration | `cargo test -p trackly-app --test acts_place_snapshot` | ✅ | ✅ green (4) |
| D-20 | Manager blocked from `MutatePlaces`; Employee blocked from `ReadPlaces` — on BOTH transports | integration | `cargo test -p trackly-app --test role_endpoint_matrix` | ✅ extended | ✅ green (Cases 45–48) |
| D-28 | Place filter in reports is subtree-inclusive: filtering by an ancestor returns rows nested any number of levels below it, and does NOT leak rows from a sibling subtree. Covers all four independent CTE builders in `report_service.rs` (`query_acts_inner`, `query_device_snapshot`, `count_acts_inner`, `count_device_snapshot`) via the public `ReportService` API. | integration | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test report_place_subtree -- --test-threads=1` | ✅ new (Nyquist gap-fill) | ✅ green |
| D-27 / PLC-01 | Devices CSV export emits the «Место» column carrying the device's full tree path (`place_full_paths`, `' / '` separator), empty when the device has no place | integration | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test devices_csv_export -- --test-threads=1` | ✅ extended (Nyquist gap-fill) | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [x] `crates/trackly-infra/tests/places_crud.rs` — PLC-01 (create/rename/move/archive/delete,
      uniqueness constraint, FK survival across a move)
- [x] `crates/trackly-app/tests/places_move_cycle.rs` — cycle rejection
- [x] `crates/trackly-core/src/domain/places.rs` unit tests — PLC-02 level/sort comparator
- [x] `crates/trackly-app/tests/places_search.rs` — PLC-03, **including a Cyrillic case-fold
      regression test** (highest-value new test in this phase: it is the one most likely to pass
      silently under ASCII input and fail in the RU-only production UI)
- [x] ~~`crates/trackly-app/tests/places_search_live_reflect.rs`~~ — PLC-05 rename/move cascade.
      **Path deviation (accepted):** the behavior landed as two repository-level regression tests
      instead of one app-level file — `crates/trackly-infra/tests/devices_place_search.rs::search_fts_reflects_place_rename_without_reindex`
      and `crates/trackly-infra/tests/cartridges_place_search.rs::search_reflects_place_rename_without_reindex`,
      plus `places_crud.rs::rename_updates_descendant_full_path_without_separate_reindex_call` for
      the tree itself. All three green. Coverage is equivalent or better (both FTS surfaces of D-29
      are proven independently); no file needs to be created.
- [x] `crates/trackly-app/tests/places_contents.rs` — PLC-06 nested-vs-direct toggle
- [x] `crates/trackly-app/tests/places_delete_blocked.rs` — D-14 exact-count error
- [x] `crates/trackly-app/tests/acts_place_snapshot.rs` — D-16 (may extend existing `acts_*`
      test files instead of a new file if simpler)
- [x] Extend `crates/trackly-app/tests/role_endpoint_matrix.rs` — D-20 non-standard
      Admin-only-mutate / Admin+Manager-read split
- [x] Extend `crates/trackly-infra/tests/migration_idempotency.rs` — assert `locations` and old
      columns are gone (PLC-04)

### Post-phase gap closure (Nyquist audit, 2026-08-26)

- [x] `crates/trackly-app/tests/report_place_subtree.rs` — **D-28** subtree-inclusive place filter
      (6 tests, green). Was previously untested anywhere despite four independent recursive-CTE
      implementations in `report_service.rs`.
- [x] Extend `crates/trackly-app/tests/devices_csv_export.rs` — **«Место» column / full tree path**
      (3 new tests, 10 total, green). Every pre-existing export test seeded `place_id: None`, so
      the column was never exercised.

**Known coverage boundary (not a defect — matches Plan 39-10's stated scope):** the cartridges and
requests report domains ignore `ReportFilter.place_id` entirely (only `is_storage` is wired there),
yet `ui/src/features/reports/ReportFilters.svelte` renders the place PlacePicker for **all three**
domains without gating on `reportDomain`. A user selecting a place on the «Картриджи» or «Заявки»
tab therefore gets no filtering and no feedback. Logged as a WARNING for the milestone debt list.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| `PlaceTree` / `PlacePicker` keyboard navigation + ARIA treeview contract (UI-SPEC §8.5 / §10.5) | PLC-01, PLC-03 | a11y interaction in the real webview; the project has an explicit rule that a synthetic Playwright/Chromium harness is NOT verification — the app runs in WKWebView/WebView2 | Run the app (`cargo tauri dev`), open `/places`; verify arrow-key expand/collapse, Home/End, type-ahead, and focus ring. Then `pnpm --dir ui build` and repeat in a LAN browser. |
| Place shown correctly in printed act (frozen snapshot path) | D-16 | Print layout / page-break fidelity is not visible to text-extraction assertions (documented project lesson) | Render a real act to PDF/print preview in both desktop and LAN-browser mode; confirm the place path prints and does not overflow. |
| Migration on an **existing** portable DB (not a fresh one) | PLC-04 | Documented project trap: fresh-DB tests masked a DB-backed upgrade bug for two phases | Copy a pre-phase-39 DB file, launch the built app against it, confirm it opens without crashing and existing device/cartridge placement values are zeroed out (not preserved — no data migration is performed, confirmed twice in CONTEXT.md), while every other table's data survives untouched. |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or a Wave 0 dependency (every auto/tdd task across the
      21-plan set carries a scoped `<automated>` command; checkpoint tasks carry `<how-to-verify>`
      instead, per convention)
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all ❌ MISSING references above (every file listed under "Wave 0 Requirements"
      is created by a task in Plans 01/02/04/05/08/09/12 — see the per-plan `files_modified`)
- [x] No watch-mode flags in any command
- [x] Full-suite commands carry `--skip login_remember_persistent_cookie` and the mock env vars
- [x] Feedback latency < 30s per task (post-revision: per-task frontend verify now uses
      `pnpm --dir ui run svelte-check` instead of a full `pnpm --dir ui build` where a full build
      isn't strictly required — full build retained at wave/plan level and in Plan 21's final gate)
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-08-22

---

## Validation Audit 2026-08-26

Retroactive Nyquist audit (`/gsd-validate-phase 39`) run after phase completion. Input state **A**
(this file existed pre-execution with an all-`⬜ pending` map). Every mapped command was executed —
statuses above are observed runs, not narration.

| Metric | Count |
|--------|-------|
| Requirements / decisions in map | 12 (10 original + 2 added by this audit) |
| Gaps found | 2 |
| Resolved (tests generated) | 2 |
| Escalated to manual-only | 0 |
| Path deviations recorded | 1 (PLC-05) |

**Executed in this pass (all green):**

```
cargo test -p trackly-infra --test places_crud --test devices_place_search \
  --test cartridges_place_search --test migration_idempotency      → 19 passed
cargo test -p trackly-core places                                   → 11 passed
TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app \
  --test places_search --test places_contents --test places_move_cycle \
  --test places_delete_blocked --test places_service_crud \
  --test acts_place_snapshot --test role_endpoint_matrix -- --test-threads=1
                                                                    → 25 passed
TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app \
  --test report_place_subtree -- --test-threads=1                   → 6 passed (new)
TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app \
  --test devices_csv_export -- --test-threads=1                     → 10 passed (7 → 10)
```

**Gap 1 — D-28 (report place filter subtree-inclusivity).** Four independent recursive-CTE
implementations shipped in `report_service.rs` with zero test coverage, reachable from the live
Reports UI. A silent degradation to exact-match filtering would have been invisible. Closed by
`crates/trackly-app/tests/report_place_subtree.rs` — asserts in both directions (ancestor filter
returns the deep row; sibling subtree does not leak; empty root returns 0) over a 4-level tree.

**Gap 2 — «Место» column in devices CSV export.** Every pre-existing export test seeded
`place_id: None`, so the column that renders `place_full_paths` was never exercised even though the
import side was covered. Closed by 3 added tests in `devices_csv_export.rs`.

**Finding raised during gap-fill (not a validation gap — for the debt list):** the place filter is
inert on two of three report tabs. `report_service.rs` reads `ReportFilter.place_id` only in
`query_acts_inner` / `query_device_snapshot` (+ their count pairs) — the cartridge and requests
builders never consult it — while `ui/src/features/reports/ReportFilters.svelte` destructures
`reportDomain` as unused (`_reportDomain`) and renders `PlacePicker` with no `{#if}` guard. On
«Картриджи» and «Заявки» the control silently does nothing. Backend scope is documented as
intentional in Plan 39-10; the ungated UI arrived later in Plan 39-18. Triage as a UI/backend
contract mismatch — not blocking any PLC truth.
