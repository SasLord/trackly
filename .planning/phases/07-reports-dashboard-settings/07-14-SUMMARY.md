---
phase: "07-reports-dashboard-settings"
plan: 14
subsystem: reports-frontend
tags: [gap-closure, reports, frontend, backend, tauri-command, specta, svelte5]
dependency_graph:
  requires: ["07-12"]
  provides: ["reports_get_report_counts Tauri command", "real per-tab report counts", "G2-5a controls-row alignment"]
  affects: ["ui/src/features/reports/ReportsPage.svelte", "ui/src/features/reports/ReportSubNav.svelte"]
tech_stack:
  added: []
  patterns:
    - "COUNT(*)-only SQL helpers mirroring WHERE clauses of list_* siblings (no row collection)"
    - "Vec<ReportCountEntry> DTO pattern — Array in TypeScript, no HashMap, consistent with all reports.rs DTOs"
    - "Single spawn_blocking task for 4 COUNT queries (one reader pool acquisition)"
    - "for-of iteration over result.counts array (not Object.entries / Record cast)"
key_files:
  created: []
  modified:
    - crates/trackly-app/src/dto/reports.rs
    - crates/trackly-app/src/services/report_service.rs
    - crates/trackly-app/src/tauri_cmds/reports.rs
    - crates/trackly-app/src/specta_export.rs
    - ui/src/features/reports/ReportSubNav.svelte
    - ui/src/features/reports/ReportsPage.svelte
decisions:
  - "Vec<ReportCountEntry> not HashMap — consistent with all existing DTOs in reports.rs; specta::Type derives cleanly; TypeScript gets Array not Record"
  - "COUNT(DISTINCT a.id) for acts/returns (JOINs can multiply rows); COUNT(*) for snapshot tables"
  - "loadStatusCounts() called alongside loadReport() in the same $effect — same triggers (domain, period, filter), no separate reactive dependency needed"
  - "countsLoading guard prevents concurrent count fetches when $effect fires rapidly during domain switch"
metrics:
  duration: "~15 min"
  completed: "2026-06-18"
  tasks_completed: 2
  files_modified: 6
---

# Phase 07 Plan 14: Reports Real Tab Counts + Controls-Row Alignment Summary

Gap closure G2-5 (both G2-5a and G2-5b) — adds real per-tab row counts from a new `reports_get_report_counts` backend command and right-aligns the export/print block via `justify-content: space-between` on `.controls-row`.

## What Was Built

**G2-5a — Controls-row alignment fix:**
`.controls-row` in `ReportsPage.svelte` had `align-items: flex-start` and no `justify-content`. Changed to `align-items: center` + `justify-content: space-between` so PeriodSelector stays flush-left and the ReportFilters (export/print block) is flush-right, both vertically centered on the same baseline.

**G2-5b — Real per-tab counts:**

Backend:
- `ReportCountEntry { key: String, count: i64 }` and `ReportCountsDto { counts: Vec<ReportCountEntry> }` added to `dto/reports.rs` — Vec-based, no HashMap, derives `specta::Type` without feature flags, TypeScript binding types `counts` as `ReportCountEntry[]`
- 4 private COUNT-only SQL helpers in `report_service.rs`: `count_acts_inner`, `count_device_snapshot`, `count_cartridge_audit_inner`, `count_cartridge_snapshot_inner` — each mirrors the WHERE clauses of its `list_*` sibling but runs `SELECT COUNT(*)` (no row collection). `count_acts_inner` uses `COUNT(DISTINCT a.id)` because JOIN with act_items multiplies rows
- `ReportService::get_report_counts(domain, filter, period)` runs all 4 counts in a single `spawn_blocking` task; individual failures return 0 (non-fatal)
- `reports_get_report_counts` Tauri command + `build_reports_get_report_counts` helper in `tauri_cmds/reports.rs`
- Registered in `specta_export.rs` `collect_commands!`
- `ui/src/bindings.ts` regenerated via `cargo test --test export_bindings`

Frontend:
- `ReportSubNav.svelte`: added `statusCounts?: Record<string, number>` prop; badge rendering now uses `statusCounts[r.key] ?? 0` for all tabs when `statusCounts` is provided; falls back to `rowCount` / `'–'` when absent (backwards compat)
- `ReportsPage.svelte`: added `statusCounts` + `countsLoading` state; `loadStatusCounts()` calls `reports_get_report_counts`, iterates `result.counts` array with `for (const entry of result.counts)` (Vec iteration, not Object.entries), builds `Record<string,number>` map; called alongside `loadReport()` in the reactive `$effect`; `{statusCounts}` prop passed to `ReportSubNav`

## Deviations from Plan

None — plan executed exactly as written.

## Verification Results

- `cargo build --workspace` → 0 errors
- `cargo test --test export_bindings` → ok (1 passed)
- `grep -c "reports_get_report_counts" ui/src/bindings.ts` → 2 (declaration + type)
- `grep -c "reports_get_report_counts" specta_export.rs` → 1
- `grep -c "ReportCountEntry" dto/reports.rs` → 3 (struct + field + doc comment)
- `grep -c "HashMap" dto/reports.rs` → 2 (doc comments only, no actual usage)
- `grep "ReportCountsDto" ui/src/bindings.ts` → `counts: ReportCountEntry[]` (Array, not Record)
- `grep -c "statusCounts" ReportSubNav.svelte` → 5 (prop decl, destructure, JSDoc, badge condition, badge value)
- `grep -c "justify-content: space-between" ReportsPage.svelte` → 1
- `grep -c "for (const entry of result.counts" ReportsPage.svelte` → 1
- `pnpm svelte-check` → 0 errors, 36 warnings (pre-existing)

## Self-Check

### Files exist:
- [x] `crates/trackly-app/src/dto/reports.rs` — ReportCountEntry + ReportCountsDto added
- [x] `crates/trackly-app/src/services/report_service.rs` — get_report_counts + count helpers added
- [x] `crates/trackly-app/src/tauri_cmds/reports.rs` — reports_get_report_counts command added
- [x] `crates/trackly-app/src/specta_export.rs` — command registered
- [x] `ui/src/features/reports/ReportSubNav.svelte` — statusCounts prop + badge rendering updated
- [x] `ui/src/features/reports/ReportsPage.svelte` — loadStatusCounts, statusCounts state, controls-row CSS

### Commits:
- ed78479: feat(07-14): add reports_get_report_counts backend command (G2-5b)
- dcb08c3: feat(07-14): wire real per-tab counts + fix controls-row alignment (G2-5a + G2-5b)

## Self-Check: PASSED
