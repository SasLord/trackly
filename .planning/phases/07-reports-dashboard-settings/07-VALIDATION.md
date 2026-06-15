---
phase: 7
slug: reports-dashboard-settings
status: draft
nyquist_compliant: false
wave_0_complete: false
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

> Filled by the planner from PLAN.md task IDs once plans exist. Each row maps a task to its automated check or Wave 0 dependency. The gsd-planner and gsd-nyquist-auditor own population of this table during/after planning.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| TBD | TBD | TBD | RPT/DASH/SET | TBD | TBD | unit/integration | `cargo test ...` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] Integration test scaffolding for report SQL period-boundary math (UTC-in-DB → Moscow offset month grouping)
- [ ] Integration test for `rusqlite::backup::Backup` from reader connection + post-write `integrity_check`
- [ ] Unit test stubs for MiniJinja template validation (Act, Acceptance Document)
- [ ] Fixtures: seeded device/cartridge/request/printer rows spanning multiple months for dashboard aggregation + consumption time-series

*If existing data-layer test infrastructure from Phases 1–6 already covers a requirement, reuse it instead of adding Wave 0 stubs.*

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

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 120s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
