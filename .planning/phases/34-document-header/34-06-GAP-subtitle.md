# 34-06 Gap Closure: Report print-form subtitle used raw English period discriminator

**Status:** Fixed
**Reported during:** 34-06 human-verification checkpoint (UAT)
**Commit:** `fix(34-06): render Russian period label in report print-form subtitle`

## Defect

The report export print form's `.subtitle` (`crates/trackly-app/templates/report.html:115`,
`{{ period_label }}`) rendered an untranslated, partly wrong period string. Root cause was in
`crates/trackly-app/src/tauri_cmds/reports.rs` inside `build_reports_export_pdf`:

```rust
let period_label = period
    .as_ref()
    .map(|p| format!("{} {}", p.mode, p.year.unwrap_or(0)))
    .unwrap_or_default();
```

`p.mode` is the raw discriminator string `"month" | "year" | "range"`, so the subtitle
literally read `year 2026`. Three distinct problems:

1. `mode` is English and unlocalized — UI and all printed documents are Russian-only (v1
   constraint).
2. For `mode == "month"`, `p.month` was never used — the month was silently lost, subtitle
   showed only the year.
3. For `mode == "range"`, `p.year` is always `None`, so the subtitle read `range 0`;
   `date_from`/`date_to` were ignored entirely.

This defect predates phase 34 but lives in the `.subtitle` of the print forms this phase
owns, and the user explicitly scoped its fix into phase 34.

## Fix

Added `ReportService::format_period_label(&PeriodDto) -> String` in
`crates/trackly-app/src/services/report_service.rs` (reuses the existing `MONTH_NAMES_RU`
table already used by `month_key_to_russian`):

- `mode == "month"` + year/month → `"Сентябрь 2026"` (falls back to bare year if month
  missing or out of range; empty string if year missing).
- `mode == "year"` + year → `"2026 год"` (empty string if year missing).
- `mode == "range"` + `date_from`/`date_to` (ISO `YYYY-MM-DD`) → `"01.01.2026 — 31.03.2026"`
  (dd.mm.yyyy, em dash separator — new local helper `format_ru_short_date`, no existing
  ISO→dd.mm.yyyy formatter was found in the codebase to reuse).
- Unknown/malformed input degrades to an empty string; no `unwrap()` on `Option`/parse
  results anywhere in the new code; no English discriminator can reach the template.

Wired into `build_reports_export_pdf` in `crates/trackly-app/src/tauri_cmds/reports.rs`,
replacing the old inline `format!`. Verified `crates/trackly-app/src/http/reports.rs`
`handler_export_pdf` calls this same `build_reports_export_pdf` builder, so the single fix
site covers both the Tauri command and the LAN HTTP transport.

## Tests

13 new unit tests added to `crates/trackly-app/src/services/report_service.rs`
(`#[cfg(test)] mod tests`), covering all three modes plus missing-field and
malformed-input degradation cases. All exact-match Russian output strings.

Foreground run (targeted, not `--workspace` per repo constraint):

```
cargo test -p trackly-app --lib services::report_service::tests -- --test-threads=1
```

Result: `22 passed; 0 failed` (13 new `format_period_label_*` tests + 9 pre-existing
`report_service` tests, all green).

`cargo build -p trackly-app`: clean.
`cargo clippy -p trackly-app --lib -- -D warnings`: clean.
`cargo fmt --check` on the two touched files: clean (repo-wide drift is pre-existing in
unrelated files, per project constraint — not touched).

## Files changed

- `crates/trackly-app/src/services/report_service.rs` — added `format_period_label`,
  `format_ru_short_date`, 13 unit tests.
- `crates/trackly-app/src/tauri_cmds/reports.rs` — wired `format_period_label` into
  `build_reports_export_pdf`, removed the old inline `format!`.

## Privacy check

No real organization or personal data introduced — test fixtures use synthetic dates only
(`2026-01-01`, `2026-09`, etc.), no names or org identifiers.
