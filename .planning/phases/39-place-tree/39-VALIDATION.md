---
phase: 39
slug: place-tree
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-08-22
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
| PLC-01 | Build tree, rename, move subtree without losing device FK bindings | integration | `cargo test -p trackly-infra --test places_crud` | ❌ W0 | ⬜ pending |
| PLC-01 | Cycle rejection on move (node cannot become its own descendant) | integration | `cargo test -p trackly-app --test places_move_cycle` | ❌ W0 | ⬜ pending |
| PLC-02 | Floor `level` accepts 0 and negatives; siblings sort by level, not name | unit | `cargo test -p trackly-core --lib places` | ❌ W0 | ⬜ pending |
| PLC-03 | Full-path search, incl. Cyrillic case-insensitivity (Rust-side lowercase, not SQL `LIKE`) | integration | `cargo test -p trackly-app --test places_search` | ❌ W0 | ⬜ pending |
| PLC-04 | `locations` table and all `location_id` / free-text location columns removed post-migration | integration (schema assertion) | `cargo test -p trackly-infra --test migration_idempotency` | ✅ extend | ⬜ pending |
| PLC-05 | Rename/move instantly reflected in search + all lists, no manual reindex step | integration | `cargo test -p trackly-app --test places_search_live_reflect` | ❌ W0 | ⬜ pending |
| PLC-06 | Place-contents screen returns nested items by default; toggle limits to direct children only | integration | `cargo test -p trackly-app --test places_contents` | ❌ W0 | ⬜ pending |
| D-14 | Delete blocked with exact counts when place is non-empty | integration | `cargo test -p trackly-app --test places_delete_blocked` | ❌ W0 | ⬜ pending |
| D-16 | Act stores `place_id` + frozen path snapshot; later rename does not alter a printed act | integration | `cargo test -p trackly-app --test acts_place_snapshot` | ❌ W0 | ⬜ pending |
| D-20 | Manager blocked from `MutatePlaces`; Employee blocked from `ReadPlaces` — on BOTH transports | integration | `cargo test -p trackly-app --test role_endpoint_matrix` | ✅ extend | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/trackly-infra/tests/places_crud.rs` — PLC-01 (create/rename/move/archive/delete,
      uniqueness constraint, FK survival across a move)
- [ ] `crates/trackly-app/tests/places_move_cycle.rs` — cycle rejection
- [ ] `crates/trackly-core/src/domain/places.rs` unit tests — PLC-02 level/sort comparator
- [ ] `crates/trackly-app/tests/places_search.rs` — PLC-03, **including a Cyrillic case-fold
      regression test** (highest-value new test in this phase: it is the one most likely to pass
      silently under ASCII input and fail in the RU-only production UI)
- [ ] `crates/trackly-app/tests/places_search_live_reflect.rs` — PLC-05 rename/move cascade
- [ ] `crates/trackly-app/tests/places_contents.rs` — PLC-06 nested-vs-direct toggle
- [ ] `crates/trackly-app/tests/places_delete_blocked.rs` — D-14 exact-count error
- [ ] `crates/trackly-app/tests/acts_place_snapshot.rs` — D-16 (may extend existing `acts_*`
      test files instead of a new file if simpler)
- [ ] Extend `crates/trackly-app/tests/role_endpoint_matrix.rs` — D-20 non-standard
      Admin-only-mutate / Admin+Manager-read split
- [ ] Extend `crates/trackly-infra/tests/migration_idempotency.rs` — assert `locations` and old
      columns are gone (PLC-04)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| `PlaceTree` / `PlacePicker` keyboard navigation + ARIA treeview contract (UI-SPEC §8.5 / §10.5) | PLC-01, PLC-03 | a11y interaction in the real webview; the project has an explicit rule that a synthetic Playwright/Chromium harness is NOT verification — the app runs in WKWebView/WebView2 | Run the app (`cargo tauri dev`), open `/places`; verify arrow-key expand/collapse, Home/End, type-ahead, and focus ring. Then `pnpm --dir ui build` and repeat in a LAN browser. |
| Place shown correctly in printed act (frozen snapshot path) | D-16 | Print layout / page-break fidelity is not visible to text-extraction assertions (documented project lesson) | Render a real act to PDF/print preview in both desktop and LAN-browser mode; confirm the place path prints and does not overflow. |
| Migration on an **existing** portable DB (not a fresh one) | PLC-04 | Documented project trap: fresh-DB tests masked a DB-backed upgrade bug for two phases | Copy a pre-phase-39 DB file, launch the built app against it, confirm existing device placements survived into the tree and no data was silently dropped. |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or a Wave 0 dependency
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all ❌ MISSING references above
- [ ] No watch-mode flags in any command
- [ ] Full-suite commands carry `--skip login_remember_persistent_cookie` and the mock env vars
- [ ] Feedback latency < 30s per task
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
