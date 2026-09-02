---
phase: 40-movement-history
plan: 12
subsystem: api
tags: [rust, axum, tauri, rbac, reports, movement-history]

# Dependency graph
requires:
  - phase: 40-movement-history (plan 11)
    provides: "ReportFilter.{from_place_id,to_place_id}, ReportRow.{from_place_path,from_place_path_short,actor_name,reason,entity_type_label,is_deleted}, ReportService::list_movements + query_movements_inner"
  - phase: 40-movement-history (plan 10)
    provides: "paired Tauri/axum endpoint shape (identity resolve → build_* delegate → Json/map error), Action::ReadPlaces gate precedent"
provides:
  - "build_reports_list_movements (tauri_cmds/reports.rs) — gates on Action::ReadPlaces, NOT Action::ReadData like the other 12 reports (D-12)"
  - "#[tauri::command] reports_list_movements — registered in specta_export.rs, invokable from the desktop transport (not explicitly named in the plan's action text, added because Plan 40-18's UI wiring references it as an existing command)"
  - "handler_list_movements (http/reports.rs) + POST /api/v1/reports_list_movements — same gate, same authorize call, via the shared build_* function"
  - "\"movements\" arm in columns_for/column_labels_for/report_display_name, index-aligned (D-23), extending the existing column_labels_for_is_index_aligned_with_columns_for regression test"
  - "\"movements\" added to PERIOD_BASED_REPORT_TYPES + fetch_report's dispatch arm, so CSV/PDF export resolves the report type with zero changes to report_service.rs (D-26)"
  - "crates/trackly-app/tests/report_movements.rs — 5 integration tests exercising the gate and both export formats through the authorize-gated adapter layer"
affects: [40-14, 40-18]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Gate divergence is enforced structurally, not just by convention: build_reports_list_movements is the ONLY build_reports_list_* function calling Action::ReadPlaces; every other sibling calls Action::ReadData. Both axum and Tauri delegate to this single function, so the gate lives once (T-40-25 mitigation)."
    - "CSV export header row is the raw column KEY list (e.g. \"handover_date_utc;device_name;...\"), not the Russian labels — confirmed pre-existing pipeline behavior via report_csv_export.rs, not something this plan changed. PDF/HTML export headers ARE the Russian labels (column_labels_for), a genuinely different row than CSV's."

key-files:
  created:
    - crates/trackly-app/tests/report_movements.rs
  modified:
    - crates/trackly-app/src/tauri_cmds/reports.rs
    - crates/trackly-app/src/http/reports.rs
    - crates/trackly-app/src/specta_export.rs

key-decisions:
  - "columns_for(\"movements\")'s date column key is \"handover_date_utc\", NOT \"created_at_utc\" as PATTERNS.md's interface snippet literally showed — Plan 40-11 reused the existing ReportRow.handover_date_utc field to carry pm.created_at_utc (documented in its own SUMMARY as the D-23 place_path reuse pattern), and row_field has no \"created_at_utc\" match arm. Using the literal plan text would have compiled cleanly but silently rendered an empty «Дата» column in both the table and every export. Fixed inline (Rule 1) before any test caught it, since none of the plan's own acceptance criteria greps for this key."
  - "Added the actual #[tauri::command] reports_list_movements wrapper and its specta_export.rs registration, even though Task 1's action text only asked for the build_reports_list_movements adapter function. Plan 40-18 (UI, depends_on: [\"40-12\"]) references cmd: 'reports_list_movements' as an already-existing invokable command in its own interfaces section — without this addition the desktop transport would have no way to reach list_movements at all, only the HTTP transport would. Rule 2 (auto-add missing critical functionality): the plan's own stated success criterion \"registered on both transports\" would otherwise be false for the Tauri side."
  - "Reused the existing ListWithPeriodPayload HTTP payload shape for handler_list_movements instead of a new ListMovementsPayload struct (the plan's action text named the latter generically) — list_movements takes exactly {filter, period} like device_acts, so a new struct would be a pure duplicate."
  - "Did not touch report_service.rs, export_csv, or export_pdf — build_reports_export_csv/build_reports_export_pdf still gate on Action::ReadData at their own top level for ALL report types including movements. This satisfies D-26 (zero new export code) and still correctly excludes Employee, because Action::ReadData and Action::ReadPlaces currently authorize the identical role set (Admin | Manager) per authorize()'s permission matrix — verified explicitly in a test rather than assumed."

requirements-completed: []  # Bookkeeping constraint: HST-04 closed by the orchestrator at phase end, not by this plan.

# Metrics
duration: ~35min
completed: 2026-09-02
---

# Phase 40 Plan 12: Movements Report Adapter Layer Summary

**`build_reports_list_movements`/`handler_list_movements`/`reports_list_movements` wired on both transports, gated on `Action::ReadPlaces` (verified distinct from every other report's `Action::ReadData` via grep + a live Forbidden-identity test), with CSV/PDF export proven at parity through the unmodified existing pipeline.**

## Performance

- **Duration:** ~35 min
- **Completed:** 2026-09-02
- **Tasks:** 2/2
- **Files modified:** 3 (1 created)

## Accomplishments

- `columns_for`/`column_labels_for`/`report_display_name` gain an index-aligned `"movements"` arm (D-23: Дата/Предмет/Тип/Откуда/Куда/Кем/Причина) — extended, not duplicated, the existing `column_labels_for_is_index_aligned_with_columns_for` regression test
- `build_reports_list_movements` authorizes on `Action::ReadPlaces` exclusively — confirmed by grep (`Action::ReadPlaces` present, `Action::ReadData` absent in the function body) AND by a live integration test (`report_movements_gate_denies_employee`) round-tripping an Employee identity to `Err(AppError::Forbidden)`
- `"movements"` added to `PERIOD_BASED_REPORT_TYPES` and `fetch_report`'s dispatch arm so `build_reports_export_csv`/`build_reports_export_pdf` (which route through `fetch_report`, unchanged) can resolve the new report type
- `handler_list_movements` (axum) + `POST /api/v1/reports_list_movements`, delegating to the SAME `build_reports_list_movements` the Tauri command calls — the gate lives in exactly one place, not duplicated per transport (T-40-25)
- Added the actual `#[tauri::command] reports_list_movements` wrapper + `specta_export.rs` registration (Rule 2 addition — see Decisions) so the desktop transport can reach it too, not just HTTP
- 5 new integration tests in `crates/trackly-app/tests/report_movements.rs`: D-24 AND-filtered dual subtree-inclusive place filter (through the gated adapter, not the raw service method Plan 40-11 already unit-tested), D-25 soft-deleted-item marker, an explicit Employee-Forbidden gate test, and CSV/PDF export smoke tests confirming zero new export code (D-26)

## Task Commits

Each task was committed atomically:

1. **Task 1: columns_for/column_labels_for/report_display_name + build_reports_list_movements (ReadPlaces gate)** - `b2746ebf` (feat)
2. **Task 2: handler_list_movements (HTTP) + report_movements.rs test suite** - `f99d2e26` (test)

**Plan metadata:** (this commit) `docs: complete plan`

## Files Created/Modified

- `crates/trackly-app/src/tauri_cmds/reports.rs` - `"movements"` arm × 3 matches, `build_reports_list_movements` (`Action::ReadPlaces`), `PERIOD_BASED_REPORT_TYPES`/`fetch_report` extension, `#[tauri::command] reports_list_movements`, extended index-alignment test
- `crates/trackly-app/src/http/reports.rs` - `handler_list_movements` + `POST /api/v1/reports_list_movements` route registration
- `crates/trackly-app/src/specta_export.rs` - registered `reports_list_movements` for bindings generation (`ui/src/bindings.ts` is gitignored, regenerated locally by `cargo test --test export_bindings`, no commit needed)
- `crates/trackly-app/tests/report_movements.rs` - new integration test file, 5 tests

## Decisions Made

- `columns_for("movements")` uses `"handover_date_utc"` as its date key, not the literal `"created_at_utc"` from PATTERNS.md's interface snippet — see key-decisions in frontmatter for the full reasoning (Plan 40-11's field-reuse decision made the literal plan text a latent empty-column bug)
- Added the full `#[tauri::command] reports_list_movements` + specta registration, one layer above what Task 1's action text literally asked for, because Plan 40-18 depends on it existing as a real invokable command
- Reused `ListWithPeriodPayload` for the HTTP payload instead of introducing a new struct
- Left `export_csv`/`export_pdf`'s own `Action::ReadData` gate untouched (D-26) — verified this still denies Employee for the movements report too, since both actions currently authorize the identical Admin|Manager role set

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `columns_for("movements")`'s date key would have resolved to an empty cell**
- **Found during:** Task 1, before writing any test — cross-checked against `row_field`'s existing match arms and Plan 40-11's own SUMMARY
- **Issue:** The plan's `<interfaces>` section (and PATTERNS.md) literally showed `"movements" => vec!["created_at_utc", ...]`, but `row_field` has no `"created_at_utc"` arm — `ReportRow`'s date is carried in the existing `handover_date_utc` field (Plan 40-11's explicit reuse decision, matching the `place_path` reuse convention for "Куда"). Using the literal key would compile and pass every acceptance-criteria grep in the plan, but silently render an empty «Дата» column in the table AND both exports.
- **Fix:** Used `"handover_date_utc"` as the key instead, matching the actually-populated field and its existing `row_field` arm.
- **Files modified:** `crates/trackly-app/src/tauri_cmds/reports.rs`
- **Verification:** `report_movements_export_csv_has_d23_headers` asserts the CSV header row contains `"handover_date_utc"`; manual inspection of a rendered row (`15.11.23, 01:15;Ноутбук ФИО-тест;...`) during test debugging confirmed a real date value, not an empty cell.
- **Committed in:** `b2746ebf`

**2. [Rule 2 - Missing Critical] Tauri command wrapper + specta registration for `reports_list_movements`**
- **Found during:** Task 1, while reading Plan 40-18 (the UI plan) to understand the full downstream contract
- **Issue:** Task 1's action text only asked for the shared `build_reports_list_movements` adapter function, not the actual `#[tauri::command]` entry point. But Plan 40-18 (`depends_on: ["40-12"]`) references `cmd: 'reports_list_movements'` as an already-existing invokable command in its own `<interfaces>` section. Without the wrapper, the desktop transport would have no way to call `list_movements` — only HTTP would work, contradicting this plan's own success criterion ("registered on both transports").
- **Fix:** Added `#[tauri::command] #[specta::specta] pub async fn reports_list_movements(...)` delegating to `build_reports_list_movements`, and registered it in `specta_export.rs`'s command list (mandatory per that file's own doc comment: "Каждое следующее phase, добавляющее `#[tauri::command]`, ОБЯЗАНО зарегистрировать её здесь").
- **Files modified:** `crates/trackly-app/src/tauri_cmds/reports.rs`, `crates/trackly-app/src/specta_export.rs`
- **Verification:** `cargo test --test export_bindings` regenerated `ui/src/bindings.ts` with a `reports_list_movements` entry (file is gitignored, not committed, matches project convention).
- **Committed in:** `b2746ebf`

---

**Total deviations:** 2 auto-fixed (1 bug, 1 missing critical)
**Impact on plan:** Both fixes were necessary for the plan's own stated success criteria to actually hold (a working «Дата» column; a working desktop transport). No scope creep beyond what Plan 40-18 already assumes exists.

## Issues Encountered

- My first version of the CSV/PDF export smoke tests asserted the Russian D-23 labels (`"Дата"`, `"Предмет"`, ...) as CSV headers. `export_csv` actually writes the raw column KEYS as its header row (pre-existing pipeline behavior, confirmed against `report_csv_export.rs`'s own assertions like `body.contains("number")`) — only `export_pdf`'s HTML header row uses the Russian labels via `column_labels_for`. Fixed the CSV test's assertions to check for raw keys; no production code was wrong, only my test's expectation.
- My PDF export test initially failed with `Internal { source_chain: "ReportService::export_pdf called without with_organization" }` because my test fixture's `ReportService` (cloned from `reports_period_required.rs`'s `minimal_ctx()`) was missing the `.with_organization(...)` builder call that `export_pdf` requires. Added it to my fixture; not a production bug, `reports_period_required.rs`'s own tests never actually exercise the PDF-render-success path (only the period-validation early-exit), so this gap in that fixture had never been hit before.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- The movements report is fully registered and correctly gated on both transports; Plan 40-14 can add the role-matrix regression test (`role_endpoint_matrix.rs` Cases) on top without any adapter-layer changes.
- Plan 40-18 (UI) can now wire `cmd: 'reports_list_movements'` against a real, working, correctly-gated Tauri command — verified it exists and its signature (`filter: ReportFilter, period: PeriodDto`) matches what that plan's interfaces section expects.
- No blockers identified.

---
*Phase: 40-movement-history*
*Completed: 2026-09-02*

## Self-Check: PASSED

- FOUND: crates/trackly-app/src/tauri_cmds/reports.rs
- FOUND: crates/trackly-app/src/http/reports.rs
- FOUND: crates/trackly-app/src/specta_export.rs
- FOUND: crates/trackly-app/tests/report_movements.rs
- FOUND: .planning/phases/40-movement-history/40-12-SUMMARY.md
- FOUND commit: b2746ebf
- FOUND commit: f99d2e26
- FOUND commit: 9e1c15aa
