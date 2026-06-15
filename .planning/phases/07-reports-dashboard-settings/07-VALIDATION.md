---
phase: 7
slug: reports-dashboard-settings
status: draft
nyquist_compliant: true
wave_0_complete: true
created: 2026-06-15
---

# Phase 7 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (Rust, workspace data layer) + `vitest` (Svelte, if present) |
| **Config file** | `Cargo.toml` workspace members; `vitest.config.ts` (frontend, if present) |
| **Quick run command** | `cargo test -p <crate-touched>` (single crate) |
| **Full suite command** | `cargo test --workspace` |
| **Estimated runtime** | ~60–120 seconds (workspace; one `cargo test` at a time — target/ lock) |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p <crate-touched>`
- **After every plan wave:** Run `cargo test --workspace`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 120 seconds

> Constraint: never run two `cargo test` concurrently — they contend on the `target/` lock and look like a multi-minute hang.

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 07-01-T1 | 07-01 | 1 | SET-01, RPT-01–07, DASH-01–05 | T-07-01-a, T-07-01-b | V026 CHECK(id=1) enforces single-row org_settings; sequential schema_version prevents downgrade | integration | `cargo test -p trackly-infra --test downgrade_protection -- --nocapture 2>&1 \| tail -5` | ❌ W0 | ⬜ pending |
| 07-01-T2 | 07-01 | 1 | SET-01–07, RPT-01–08, DASH-01–05, SET-09 | T-07-01-SC | DTO structs compile with serde+specta; no user input yet | compile | `cargo check -p trackly-app 2>&1 \| grep -E "error\|warning" \| head -20` | ❌ W0 | ⬜ pending |
| 07-02-T1 | 07-02 | 2 | SET-01, SET-02, SET-03, SET-04, SET-07, SET-09 | T-07-02-01, T-07-02-02, T-07-02-04 | OrgDbService.save_logo enforces 512KB + MIME allowlist; BackupService uses rusqlite::backup::Backup (not fs::copy); backup_to_path runs integrity_check before returning Ok | integration | `cargo test -p trackly-app --test org_settings -- --nocapture 2>&1 \| tail -10` | ❌ W0 | ⬜ pending |
| 07-02-T2 | 07-02 | 2 | SET-05, SET-06, SET-03 | T-07-02-03, T-07-02-05 | Supervisor: atomic UPDATE WHERE status!='running' prevents duplicate job claims; MiniJinja safe env with fuel limit; logo_bytes precedence over logo_path | integration | `cargo test -p trackly-app --test supervisor -- --nocapture 2>&1 \| tail -10` | ❌ W0 | ⬜ pending |
| 07-03-T1 | 07-03 | 2 | RPT-01, RPT-02, RPT-03, RPT-04, RPT-05, RPT-06, RPT-07 | T-07-03-01, T-07-03-05 | All filter values via params_from_iter; CSV formula injection prevention (csv_safe); action='custom:install' not 'install' | integration | `cargo test -p trackly-app --test report_acts -- --nocapture 2>&1 \| tail -10` | ❌ W0 | ⬜ pending |
| 07-03-T2 | 07-03 | 2 | DASH-01, DASH-02, DASH-03, DASH-04, DASH-05, RPT-08 | T-07-03-02, T-07-03-03 | Period math server-side via time::UtcOffset; no chrono; all dashboard SQL parameterized | integration | `cargo test -p trackly-app --test dashboard_widgets -- --nocapture 2>&1 \| tail -10` | ❌ W0 | ⬜ pending |
| 07-04-T1 | 07-04 | 3 | SET-01, SET-02, SET-05 | T-07-04-01, T-07-04-05 | Frontend logo size check <= 512KB before send (defense in depth); SVG rendered as img src data: URL (not raw innerHTML) | svelte-check | `pnpm svelte-check --tsconfig ui/tsconfig.json 2>&1 \| grep -E "Error\|error" \| head -20` | ❌ W0 | ⬜ pending |
| 07-04-T2 | 07-04 | 3 | SET-03, SET-04, SET-06, SET-07, SET-09 | T-07-04-02, T-07-04-03, T-07-04-04 | Template body sent to backend for validation (never eval'd in browser); DB move gated to Tauri context with confirmation modal | svelte-check | `pnpm svelte-check --tsconfig ui/tsconfig.json 2>&1 \| grep -E "Error\|error" \| head -20` | ❌ W0 | ⬜ pending |
| 07-05-T1 | 07-05 | 3 | DASH-01, DASH-02, DASH-03, DASH-04, DASH-05 | T-07-05-01 | SVG content computed from numeric data only; Svelte auto-escapes text nodes; no chart library | svelte-check | `pnpm svelte-check --tsconfig ui/tsconfig.json 2>&1 \| grep -E "Error\|error" \| head -20` | ❌ W0 | ⬜ pending |
| 07-05-T2 | 07-05 | 3 | DASH-01, DASH-02, DASH-03, DASH-04, DASH-05 | T-07-05-02, T-07-05-03 | Widgets load independently; read-only display page; auth enforced in plan 07 backend handlers | svelte-check | `pnpm svelte-check --tsconfig ui/tsconfig.json 2>&1 \| grep -E "Error\|error" \| head -20` | ❌ W0 | ⬜ pending |
| 07-06-T1 | 07-06 | 3 | RPT-01, RPT-02, RPT-03, RPT-04, RPT-05, RPT-06, RPT-07, RPT-08 | T-07-06-03 | PeriodSelector blocks onPeriodChange when start > end (date range validation) | svelte-check | `pnpm svelte-check --tsconfig ui/tsconfig.json 2>&1 \| grep -E "Error\|error" \| head -20` | ❌ W0 | ⬜ pending |
| 07-06-T2 | 07-06 | 3 | RPT-01, RPT-02, RPT-03, RPT-04, RPT-05, RPT-06, RPT-07, RPT-08 | T-07-06-01, T-07-06-02 | Filter values sent to backend (not SQL-constructed in UI); formula injection prevention on backend | svelte-check | `pnpm svelte-check --tsconfig ui/tsconfig.json 2>&1 \| grep -E "Error\|error" \| head -20` | ❌ W0 | ⬜ pending |
| 07-07-T1a | 07-07 | 4 | RPT-01–08, DASH-01–05, SET-01–07, SET-09 | T-07-07-01, T-07-07-02, T-07-07-03 | settings_move_db is Tauri-only (not in HTTP router per T-07-07-03); settings mutation commands require ManageSettings | compile | `cargo check -p trackly-app 2>&1 \| grep -E "^error" \| head -20` | ❌ W0 | ⬜ pending |
| 07-07-T1b | 07-07 | 4 | RPT-01–08, DASH-01–05, SET-01–07, SET-09 | T-07-07-01, T-07-07-02, T-07-07-04 | HTTP mutation handlers call authorize(&caller, &Action::ManageSettings)?; report filter handlers call same parameterized build_* helpers as Tauri | compile | `cargo check -p trackly-app 2>&1 \| grep -E "^error" \| head -20` | ❌ W0 | ⬜ pending |
| 07-07-T2 | 07-07 | 4 | RPT-01–08, DASH-01–05, SET-01–07, SET-09 | T-07-07-04, T-07-07-05 | All integration tests green; supervisor duplicate-claim prevented by atomic UPDATE WHERE status!='running' | integration | `cargo test -p trackly-app 2>&1 \| grep -E "^test result" \| tail -5` | ❌ W0 | ⬜ pending |
| 07-07-T3 | 07-07 | 4 | RPT-01–08, DASH-01–05, SET-01–07, SET-09 | T-07-07-01–05 | Human verifies end-to-end: dashboard widgets, reports export, settings save, logo in PDF, template preview | human-verify | (manual — checkpoint:human-verify gate=blocking) | n/a | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [x] Integration test scaffolding for report SQL period-boundary math (UTC-in-DB → Moscow offset month grouping) — created in plan 07-01 Task 2 as RED stubs (report_period_bounds.rs, report_acts.rs, report_cartridges.rs, report_csv_export.rs); turned GREEN by plan 07-03 Task 1
- [x] Integration test for `rusqlite::backup::Backup` from reader connection + post-write `integrity_check` — created in plan 07-01 Task 2 as RED stub (backup_service.rs); turned GREEN by plan 07-02 Task 1
- [x] Unit test stubs for MiniJinja template validation (Act, Acceptance Document) — created in plan 07-01 Task 2 as RED stub (template_edit.rs); turned GREEN by plan 07-02 Task 1
- [x] Fixtures: seeded device/cartridge/request/printer rows spanning multiple months for dashboard aggregation + consumption time-series — created as part of dashboard_widgets.rs RED stub in plan 07-01 Task 2; turned GREEN by plan 07-03 Task 2

*All Wave 0 requirements are addressed by plan 07-01 test scaffold tasks. If existing data-layer test infrastructure from Phases 1–6 already covers a requirement, reuse it instead of adding Wave 0 stubs.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| PDF export renders Cyrillic + logo correctly | RPT-04, SET-02 | Visual fidelity of krilla PDF output not assertable in unit test | Generate report PDF, open, confirm Cyrillic text + org logo in header |
| System print dialog opens | RPT-05 | OS-level dialog, not automatable in CI | Trigger print, confirm native dialog appears |
| DB path change rejects SMB share | SET-05 | Windows-only SMB path; dev box is macOS | On Windows, attempt to set DB path to `\\server\share`, confirm rejection |
| CSV opens in Excel with correct Cyrillic + delimiter | RPT-03 | UTF-8 BOM + `;` rendering is an Excel-locale behavior | Open exported CSV in Excel, confirm columns split and Cyrillic intact |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 120s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
