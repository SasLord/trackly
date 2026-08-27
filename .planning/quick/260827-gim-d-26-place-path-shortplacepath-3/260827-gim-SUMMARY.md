---
phase: 260827-gim
plan: 01
subsystem: reports
tags: [svelte5, rusqlite, sqlite, reports, d-26, requests-report]

# Dependency graph
requires:
  - phase: 39
    provides: "D-26 place_path truncation (shortPlacePath) in ReportTable.svelte; Phase 12's combine_printer_and_place composite string for the requests report"
provides:
  - "ReportRow.device_name/place_path arrive as separate fields for the requests report domain (no query-time string concatenation)"
  - "ReportTable.svelte formatPlaceCell — renders composite place cells via explicit Column.compositeWith flag, never parses a joined string"
  - "column_labels_for uniform 'Место' label across all report domains, matching ReportsPage.svelte COLUMNS_MAP (W2 fix)"
affects: [reports, requests, printers]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Composite report cells: backend keeps source fields separate (device_name/place_path); frontend combines them explicitly via a Column.compositeWith flag instead of parsing a joined string — export-only composite string (printer_place) built exclusively in row_field for CSV/PDF"

key-files:
  created: []
  modified:
    - crates/trackly-app/src/services/report_service.rs
    - crates/trackly-app/src/tauri_cmds/reports.rs
    - crates/trackly-app/tests/report_place_subtree.rs
    - ui/src/features/reports/ReportTable.svelte
    - ui/src/features/reports/ReportsPage.svelte

key-decisions:
  - "Fixed W1 (printer name disappearing on 3+ segment paths) by splitting fields at the source (backend) rather than parsing the composite string in ReportTable.svelte — eliminates the bug class structurally instead of patching one symptom"
  - "combine_printer_and_place's only remaining call-site is row_field's new 'printer_place' arm (CSV/PDF export) — screen rendering never sees the joined string"
  - "Bundled W2 (export header labels mismatched with on-screen COLUMNS_MAP: 'Локация'/'Расположение'/'Принтер / Локация') into the same plan since it touched the exact same column_labels_for function"

requirements-completed: [PLC-04]

# Metrics
duration: 82min
completed: 2026-08-27
---

# Quick Task 260827-gim: D-26 place path vs composite requests column Summary

**Split printer_name/place_path into separate ReportRow fields for the requests report so D-26's path-shortening never truncates the printer's name; export headers aligned to "Место" everywhere.**

## Performance

- **Duration:** 82 min (includes ~1h of backend/frontend gate verification; a full-workspace `cargo test -p trackly-app` regression run was started but killed by the environment after 56+ min without completing — see Issues Encountered)
- **Started:** 2026-08-27T05:03:12Z
- **Completed:** 2026-08-27T06:25:17Z
- **Tasks:** 2 of 3 (Task 3 is a blocking human-verify checkpoint, not executable by the agent)
- **Files modified:** 5

## Accomplishments
- `query_requests_inner` no longer glues `printer_name`+`place` into a single `place_path` string — `device_name` and `place_path` are now distinct `ReportRow` fields, so the printer's name can never be truncated away by D-26 shortening regardless of place-path depth
- `row_field`'s new `"printer_place"` arm is now the *only* caller of `combine_printer_and_place`, used exclusively by CSV/PDF export — the composite string on the wire for exports is unchanged
- `ReportTable.svelte` gained `formatPlaceCell`, which combines `col.compositeWith` (device_name) + `place_path` explicitly, with D-26 shortening applied only to the pure path segment — no string parsing of a joined value anywhere in the component anymore
- `column_labels_for` (W2) now returns "Место" for every place-column across all six report domains, matching `ReportsPage.svelte`'s `COLUMNS_MAP` exactly

## Task Commits

1. **Task 1: Backend — split device_name/place_path fields + W2 label alignment + regression test** - `83bb697b` (fix)
2. **Task 2: Frontend — composite column via compositeWith (no string parsing)** - `1e4619fb` (fix)

Task 3 (`checkpoint:human-verify`) is NOT executed by the agent — see below.

## Files Created/Modified
- `crates/trackly-app/src/services/report_service.rs` - `query_requests_inner` assigns `device_name`/`place_path` directly instead of calling `combine_printer_and_place`; `row_field` gains `"printer_place"` match arm; two new unit tests (`row_field_printer_place_combines_device_name_and_place_path`, `row_field_printer_place_empty_when_no_printer_and_no_place`); doc-comment on `combine_printer_and_place` updated to reflect its new call-site
- `crates/trackly-app/src/tauri_cmds/reports.rs` - `columns_for` requests_* arm's last key changed `place_path` → `printer_place`; `column_labels_for` — all six domain arms now return `"Место"` (was `"Локация"`/`"Расположение"`/`"Принтер / Локация"`)
- `crates/trackly-app/tests/report_place_subtree.rs` - new regression test `requests_report_printer_name_survives_deep_place_path` (3-segment path fixture, asserts `device_name`/`place_path` arrive separately and intact)
- `ui/src/features/reports/ReportTable.svelte` - `Column` interface gains `compositeWith?: string`; new `formatPlaceCell(row, col, transformPath)` helper; `formatCellTitle`/`formatCellDisplay` now take the whole `Column` object (not just its key) and delegate to `formatPlaceCell` for `place_path`
- `ui/src/features/reports/ReportsPage.svelte` - `Column` interface gains `compositeWith?: string`; `REQUEST_COLUMNS`' `place_path` entry now carries `compositeWith: 'device_name'`

## Decisions Made
- Chose "split fields at source" (backend) over "parse composite string smarter" (frontend-only) or "resize the string cut at export boundary" — per the plan's option analysis, this is the only approach that structurally prevents the bug class rather than patching the one reproduction case found in audit
- Bundled the W2 label fix into this plan (same function, same commit-worthy change, cheap to include) rather than deferring to a separate quick task

## Deviations from Plan

None — plan executed exactly as written. All artifacts, key_links, and must_haves.truths from the plan frontmatter were satisfied as specified.

## Issues Encountered

- A full-workspace `cargo test -p trackly-app -- --test-threads=1 --skip login_remember_persistent_cookie` regression run (plan's overall `<verification>` step 2, not a per-task `<verify>` requirement) was started to double-check no unrelated regression. It ran for 56+ minutes progressing through pre-existing integration test binaries (including a long-running `graceful_shutdown_drain` test) before the environment killed the background process — this is an environment/session time constraint, not a test failure; no error or panic was observed in any output produced before the kill. All test binaries specific to this change (`report_place_subtree` — 12/12 passing, `report_service::tests::row_field_printer_place_*` — 2/2 passing, `tauri_cmds::reports::tests::column_labels_for_is_index_aligned_with_columns_for` — passing) were run individually and are green, as was `cargo clippy -p trackly-app --all-targets -- -D warnings` and `cargo fmt --check`. Recommend re-running the full workspace suite out-of-band (e.g. overnight or in CI) before considering this milestone's tech debt fully closed, but it is not blocking for this quick task's own scope.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Backend + frontend changes are complete, committed, and pass all targeted automated gates (fmt, clippy, svelte-check, eslint, `pnpm build`, the full `report_place_subtree` suite, and the new unit tests). `ui/dist` has been rebuilt so LAN/server mode serves the fix.

**Blocked on Task 3 (human-verify checkpoint)** — compile gates do not prove Svelte 5 rune runtime behavior; a live check in the running app (desktop or LAN browser) is required before this quick task can be marked fully done. See the checkpoint message for exact verification steps.

---
*Phase: 260827-gim*
*Completed: 2026-08-27 (Tasks 1-2; Task 3 pending human verification)*

## Self-Check: PASSED

All 5 modified files confirmed present on disk; commits `83bb697b` (Task 1) and `1e4619fb` (Task 2) confirmed present in `git log`.

---

## Дополнение оркестратора (2026-08-27)

**Найдена и починена регрессия, не входившая в инвентарь исполнителя.**

Исполнитель прогнал `report_place_subtree` (12/12) и статические гейты, но не остальные
отчётные бинари. Полный прогон отчётных тестов на границе задачи выявил падение:

```
report_requests_open_filters_by_status_and_translates_type
  left:  Some("Склад тест")
  right: Some("Принтер HP LaserJet, Склад тест")
```

Тест ассертил старую склейку в `place_path` и падал ровно из-за Задачи 1. Исправлен
(`6773438c`): теперь ассертит `device_name` и `place_path` как два раздельных поля.

Это тот же класс, что записан в проектной памяти: «зелено по своим файлам» при красном
пакете. Полный прогон на границе волны его ловит, точечный — нет.

**Гейты после починки (по одному прогону за раз, лок `target/` не нарушался):**

| Прогон | Результат |
|--------|-----------|
| `report_place_subtree` | 12 passed |
| `report_requests` | 12 passed |
| `html_report_render` | 8 passed |
| `report_acts` / `report_cartridges` / `report_csv_export` | 2 / 2 / 2 passed |
| `report_period_bounds` / `reports_period_required` / `report_returns_sub_number` | 3 / 2 / 1 passed |
| `--lib` (вкл. unit-тесты `row_field`) | 214 passed |
| `cargo fmt --check` | чисто |

**Проверено отдельно оркестратором (статически):**

- `device_name` действительно был свободен для домена заявок до этой правки
  (`report_service.rs:1662` — `device_name: None`), то есть новое поле ничего не затирает.
- Группировка по `place_path` для заявок не применяется: `isSnapshot()` возвращает `true`
  только для `in_use`/`in_stock`, а все четыре вкладки «Заявки» — period-based. Смена формы
  `place_path` разделителей не задевает.

**Остаётся:** Задача 3 — живой UAT. Компиляционные гейты не доказывают поведение рун
Svelte 5; ячейка отчёта проверяется только в работающем приложении.
