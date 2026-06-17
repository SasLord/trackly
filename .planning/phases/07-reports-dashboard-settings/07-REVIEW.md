---
phase: 07-reports-dashboard-settings
reviewed: 2026-06-17T00:00:00Z
depth: standard
files_reviewed: 11
files_reviewed_list:
  - crates/trackly-app/src/services/dashboard_service.rs
  - crates/trackly-app/src/services/template_service.rs
  - ui/src/features/reports/PeriodSelector.svelte
  - ui/src/features/reports/ReportFilters.svelte
  - ui/src/features/reports/ReportSubNav.svelte
  - ui/src/features/reports/ReportsPage.svelte
  - ui/src/features/settings/BackupSettings.svelte
  - ui/src/features/settings/SettingsSubNav.svelte
  - ui/src/features/settings/StorageSettings.svelte
  - ui/src/features/settings/ThresholdSettings.svelte
  - ui/src/pages/SettingsPage.svelte
findings:
  critical: 2
  warning: 3
  info: 3
  total: 8
status: issues_found
---

# Phase 07: Code Review Report

**Reviewed:** 2026-06-17T00:00:00Z
**Depth:** standard
**Files Reviewed:** 11
**Status:** issues_found

## Summary

Reviewed 11 files from the gap-closure run (phase 07 — reports, dashboard, settings). The Rust
services follow project conventions cleanly (rusqlite single-writer, parameterised queries,
proper `spawn_blocking` boundaries). The Svelte frontend correctly uses `__TAURI_INTERNALS__`
for environment detection and Svelte 5 runes throughout.

Two blockers were found: a hardcoded UTC+3 offset in the consumption-chart SQL that ignores the
user-configured timezone, and a silent success return from `update_body` / `reset_to_default`
when zero rows are updated (template not found). Three warnings cover a semantic inaccuracy in
the printer-online counter, a stale hardcoded date in the preview fallback path, and dead code
in `currentCmd()`. Three info items cover minor UX/maintainability gaps.

## Narrative Findings (AI reviewer)

## Critical Issues

### CR-01: `get_consumption_chart` ignores configured timezone — hardcodes UTC+3

**File:** `crates/trackly-app/src/services/dashboard_service.rs:299`
**Issue:** The consumption-chart SQL uses `'+3 hours'` as a literal offset string inside the
SQLite `datetime()` call. `get_consumption_chart` never reads `self.config.organization.timezone`
at all (the field exists on `DashboardService` specifically for this purpose). `get_all_widgets`
on the same service correctly resolves the offset from config (lines 56–63), but
`get_consumption_chart` does not. For any organisation configured with `timezone =
"Europe/Moscow"` this is accidentally correct today, but the logic is wrong for any other
timezone and will break silently when the setting changes.

**Fix:**

```rust
pub async fn get_consumption_chart(
    &self,
    window_months: u8,
) -> Result<Vec<ConsumptionPoint>, AppError> {
    let now = self.clock.unix_seconds();
    let start_utc = now - (window_months as i64 * 30 * 86400);
    let readers = self.readers.clone();
    // Resolve offset the same way get_all_widgets does:
    let tz_offset_hours: i64 = {
        let tz_name = &self.config.organization.timezone;
        if tz_name == "Europe/Moscow" { 3 } else { 0 }
    };
    let tz_modifier = format!("+{tz_offset_hours} hours");

    tokio::task::spawn_blocking(move || {
        let conn = readers.acquire();
        let mut stmt = conn
            .prepare(
                "SELECT m.brand || ' ' || m.model AS model_label, \
                       strftime('%Y-%m', datetime(al.created_at_utc, 'unixepoch', ?2)) AS month_key, \
                       COUNT(*) AS installs \
                 FROM audit_log al \
                 JOIN cartridges c ON c.id = al.entity_id \
                 JOIN cartridge_models m ON m.id = c.model_id \
                 WHERE al.entity_type = 'cartridge' \
                   AND al.action = 'custom:install' \
                   AND al.created_at_utc IS NOT NULL \
                   AND al.created_at_utc >= ?1 \
                 GROUP BY model_label, month_key \
                 ORDER BY month_key ASC, model_label ASC",
            )
            .map_err(map_rusqlite)?;
        // pass both params: start_utc (?1) and tz_modifier (?2)
        let rows = stmt
            .query_map(params![start_utc, tz_modifier], |r| { ... })
            ...
    })
    ...
}
```

---

### CR-02: `update_body` and `reset_to_default` silently return `Ok(())` when 0 rows updated

**File:** `crates/trackly-app/src/services/template_service.rs:159–167` and `195–203`
**Issue:** Both methods run an `UPDATE … WHERE kind=?1 AND is_active=1 AND deleted_at_utc IS NULL`
and then call `.map(|_| ())` on the `rusqlite::execute` result, discarding the `usize`
rows-affected count. If no active template exists for the requested `kind` (e.g., all rows are
soft-deleted, or an invalid `kind` is passed), the database silently updates nothing and the
method returns `Ok(())`. The caller has no way to distinguish a successful update from a no-op.
This also means a typo in `kind` (e.g., `"act_handove"`) passes silently.

**Fix:**

```rust
// In update_body (and mirror in reset_to_default):
self.writer
    .execute(move |conn| {
        let affected = conn.execute(
            "UPDATE document_templates \
             SET body_minijinja=?2, is_default=0, \
                 updated_at_utc=?3, version=version+1 \
             WHERE kind=?1 AND is_active=1 AND deleted_at_utc IS NULL",
            params![kind_owned, body, now],
        )
        .map_err(map_rusqlite)?;
        if affected == 0 {
            return Err(AppError::NotFound {
                entity: "document_template",
                id: 0,
            });
        }
        Ok(())
    })
    .await
```

---

## Warnings

### WR-01: `printer_online` count includes printers with `error`-type alerts

**File:** `crates/trackly-app/src/services/dashboard_service.rs:260`
**Issue:** `printer_online` is computed as `printer_total - printer_offline`, where
`printer_offline` only counts rows in `printer_alerts` with `alert_type = 'offline'`. A printer
that has an `alert_type = 'error'` row is not offline, so it is counted in `printer_online` —
even though it is actively alerting. A dashboard user will see an inflated "online" count that
includes erroring printers.

Since `printer_alerts` enforces `UNIQUE(printer_id)`, a printer can have at most one alert of
either type at a time, which makes this tractable to fix.

**Fix:**

```rust
// Change the online calculation to exclude ALL alerted printers:
let printer_alerted: i64 = conn
    .query_row(
        "SELECT COUNT(*) FROM printer_alerts",
        [],
        |r| r.get(0),
    )
    .map_err(map_rusqlite)?;
let printer_online = printer_total - printer_alerted;
```

Or alternatively keep the current fields and add a clear comment that `printer_online` means
"not-offline" (and update the dashboard widget label to match). The label must match the
calculation — as-is they diverge.

---

### WR-02: Fallback `DocSpec` in `validate_preview` uses a hardcoded stale date

**File:** `crates/trackly-app/src/services/template_service.rs:305`
**Issue:** When the rendered template output cannot be parsed as a `DocSpec` JSON struct, the
code builds a fallback `DocSpec` with `date_label: "16.06.2026".to_string()`. This date is
hardcoded and will be wrong on any day other than 16 June 2026. The fallback path is reached
when a user is editing a template that renders plain text (not JSON), which is a real scenario
during template authoring.

**Fix:**

```rust
use time::OffsetDateTime;
let today = OffsetDateTime::now_utc();
let date_label = format!("{:02}.{:02}.{}", today.day(), today.month() as u8, today.year());
// ...
HeaderBlock {
    // ...
    date_label,
    // ...
}
```

---

### WR-03: `currentCmd()` in `ReportsPage.svelte` contains unreachable dead code

**File:** `ui/src/features/reports/ReportsPage.svelte:214–225`
**Issue:** `currentCmd()` computes `allReports.find(...)` and stores the result in `found` on
line 215. The function then branches on `activeDomain === 'cartridges'` (returns early) and
`activeDomain === 'devices'` (returns early). Both branches exhaust all possible values of
`activeDomain` (`DomainKey = 'devices' | 'cartridges'`), so `found?.cmd ?? 'reports_list_device_acts'`
on line 225 is unreachable. The `allReports` array and `found` variable are computed wastefully
on every call.

**Fix:** Remove the dead preamble:

```typescript
function currentCmd(): string {
  if (activeDomain === 'cartridges') {
    const r = CARTRIDGE_REPORTS.find((r) => r.key === activeReport);
    if (r) return r.cmd;
  } else {
    const r = DEVICE_REPORTS.find((r) => r.key === activeReport);
    if (r) return r.cmd;
  }
  return 'reports_list_device_acts'; // fallback for unknown activeReport
}
```

---

## Info

### IN-01: `list_all_for_editor` silently drops per-row rusqlite errors

**File:** `crates/trackly-app/src/services/template_service.rs:124`
**Issue:** The `query_map` result is collected via `.filter_map(|r| r.ok())`, which silently
discards any row-level deserialization errors (e.g., a `NULL` value in `is_default` that cannot
be coerced to `bool`). A schema migration that leaves inconsistent data would produce a partial
list without any log entry or error surface.

**Fix:** Propagate errors explicitly:

```rust
let mut items = Vec::new();
for row in rows.map_err(map_rusqlite)? {
    items.push(row.map_err(map_rusqlite)?);
}
Ok(items)
```

---

### IN-02: `ThresholdSettings.svelte` — number input allows decimal entry (missing `step="1"`)

**File:** `ui/src/features/settings/ThresholdSettings.svelte:45–48`
**Issue:** The `<input type="number" min="1" max="999" bind:value={threshold}>` has no
`step="1"` attribute. Browsers allow decimal input by default (e.g., `2.5`). The bound
`threshold` state will hold a float. When passed to the Tauri command typed as `i32`, serde
will reject the fractional value with a runtime deserialization error rather than a graceful
validation message. The backend does validate the range (1..=999) after the type assertion, but
the UX failure mode (no feedback until blur → API call → generic error toast) is poor.

**Fix:**

```html
<input
  id="threshold-input"
  class="form-input"
  type="number"
  min="1"
  max="999"
  step="1"
  bind:value={threshold}
  onblur={saveThreshold}
/>
```

---

### IN-03: `PeriodSelector.svelte` — local state does not re-sync when parent `period` prop changes

**File:** `ui/src/features/reports/PeriodSelector.svelte:24–28`
**Issue:** The component initialises `mode`, `selectedMonth`, `selectedYear`, `dateFrom`, and
`dateTo` from `$state(period.xxx ?? ...)` — these are one-time initialisers that read the prop
at mount time. If a parent component ever replaces the `period` object (e.g., after a domain
switch that resets filters), the PeriodSelector UI will not reflect the new value.

Currently `ReportsPage` never externally mutates `period` after mount (period flows only via
`onPeriodChange` callbacks), so this is not a live bug. But the pattern is fragile: any future
parent that resets `period` to `{ mode: 'month', year: ..., month: ... }` after a domain change
will find the PeriodSelector stuck in stale state.

**Fix:** Use a `$derived` or `$effect` to re-sync when the prop changes:

```typescript
// Replace the state initializers with derived sync:
let mode = $state<PeriodMode>((period.mode as PeriodMode) ?? 'month');
$effect(() => {
  mode = (period.mode as PeriodMode) ?? 'month';
  selectedMonth = period.month ?? new Date().getMonth() + 1;
  selectedYear = period.year ?? new Date().getFullYear();
});
```

---

_Reviewed: 2026-06-17T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
