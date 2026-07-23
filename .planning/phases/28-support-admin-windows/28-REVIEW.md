---
phase: 28-support-admin-windows
reviewed: 2026-07-23T00:00:00Z
depth: standard
files_reviewed: 25
files_reviewed_list:
  - crates/trackly-app/src/services/report_service.rs
  - crates/trackly-app/tests/report_returns_sub_number.rs
  - ui/src/features/reports/PeriodSelector.svelte
  - ui/src/features/reports/ReportSubNav.svelte
  - ui/src/features/reports/ReportTable.svelte
  - ui/src/features/reports/ReportsPage.svelte
  - ui/src/features/requests/RequestDetail.svelte
  - ui/src/features/requests/RequestFormModal.svelte
  - ui/src/features/requests/RequestListRow.svelte
  - ui/src/features/requests/RequestsList.svelte
  - ui/src/features/requests/RequestsMasterDetail.svelte
  - ui/src/features/requests/RequestsPage.svelte
  - ui/src/features/requests/RequestsSearchAndTabs.svelte
  - ui/src/features/settings/ActiveDirectorySettings.svelte
  - ui/src/features/settings/BackupSettings.svelte
  - ui/src/features/settings/NetworkSettings.svelte
  - ui/src/features/settings/OrgSettings.svelte
  - ui/src/features/settings/SettingsSubNav.svelte
  - ui/src/features/settings/TemplateEditor.svelte
  - ui/src/features/settings/ThresholdSettings.svelte
  - ui/src/features/users/UserFormModal.svelte
  - ui/src/features/users/UserListRow.svelte
  - ui/src/features/users/UsersList.svelte
  - ui/src/features/users/UsersPage.svelte
  - ui/src/pages/SettingsPage.svelte
findings:
  critical: 0
  warning: 2
  info: 4
  total: 6
status: issues_found
---

# Phase 28: Code Review Report

**Reviewed:** 2026-07-23
**Depth:** standard
**Files Reviewed:** 25
**Status:** issues_found

## Summary

Phase 28 is a visual re-tokenization: four support/admin windows (Requests / Reports / Settings / Users) migrated onto the shared design-system primitives (Table/TableRow, DetailPanel, Tabs, Dropdown) plus one behavioral fix in `report_service.rs`. I reviewed the migration for injected regressions, focusing on the two security-sensitive areas called out (password masking in `UserFormModal`, TemplateEditor D-08 preview boundary).

**Security checks passed:**
- `UserFormModal.svelte` correctly keeps the password field as a raw `<input type="password">` (the D-04 primitive-migration exception is respected — `Input.svelte` has no `password` type, so migrating it would have unmasked the field). Masking is intact. Not a regression.
- `TemplateEditor.svelte` preview iframe uses `sandbox=""` + `srcdoc` (no `allow-scripts`, no blob/object URL); template body is validated server-side, never `eval`'d client-side. Boundary intact.
- `report_service.rs` GAP-4 fix (`CAST(a.sub_number AS TEXT)`) is correct and matches the pre-existing `CAST(a.number AS TEXT)` treatment; the SQL remains fully parameterised; the accompanying regression test genuinely exercises a non-NULL integer `sub_number`. The logo-mime allowlist in `export_pdf` is enforced before data-URI interpolation. No injection surface introduced.

No BLOCKER-severity defects found. The two WARNINGs are latent correctness issues present in the reviewed files (one pre-dates the phase but sits squarely in a reviewed file and actively misleads an operator); the INFO items are dead props / minor UX/a11y.

## Warnings

### WR-01: Edit-mode "Новый пароль" field is collected, validated, then silently discarded

**Status:** RESOLVED 2026-07-23 — quick task `260723-syw` (commits `a30b360`, `c4df18c`). Instead of dropping the field or wiring a separate reset command, threaded the password through the shared update path: `UserPatch` gained `#[serde(default)] password: Option<String>`; `AuthService::update_user` validates a non-empty new password (`len>=8`) and hashes it via argon2id in `spawn_blocking`, writing `password_hash = COALESCE(?, password_hash)` in the same atomic version-bumping UPDATE (None/empty ⇒ no change). `handleSave` now forwards `data.password` when non-empty. Covered by `users_update_password_change`. Fixes both Tauri and HTTP transports (they share the service).

**File:** `ui/src/features/users/UsersPage.svelte:62-75` (and `ui/src/features/users/UserFormModal.svelte:177-200`)
**Issue:** In edit mode `UserFormModal` presents a field labelled *«Новый пароль (оставьте пустым, чтобы не менять)»* and validates it (`passwordErr` when `< 8` chars, `UserFormModal.svelte:111-114`). But `UsersPage.handleSave` builds the `UserPatch` from only `full_name`, `role`, `email`, `is_active` — `data.password` is never forwarded, and `UserPatch` (bindings.ts:2322) has no password field. So an admin who types a new password, passes validation, and sees the *«Пользователь обновлён»* success toast has NOT changed the password. The UI actively asserts a security action succeeded when it did nothing. (Root cause pre-dates Phase 28 — `handleSave` is byte-identical to the pre-migration version — but it lives in a file under review and the inert field is real.)
**Fix:** Either remove the password field from edit mode, or wire an admin password-reset path. If a reset endpoint exists, call it when `editTarget && data.password`:
```ts
if (editTarget) {
  const patch: UserPatch = { full_name: data.full_name || null, role: data.role || null,
    email: data.email ? data.email : null, is_active: data.is_active };
  await apiCall<UserDto>('users_update', { id: editTarget.id, version: editTarget.version, patch });
  if (data.password) {
    // route to the admin reset command (add one if absent) — do NOT drop this silently
    await apiCall<void>('users_admin_reset_password', { id: editTarget.id, newPassword: data.password });
  }
  ...
}
```
If no such backend command exists, drop the password input in edit mode so the form cannot promise a change it can't make.

### WR-02: `handleAccept` has no in-flight guard — double-click double-fires the transition

**File:** `ui/src/features/requests/RequestDetail.svelte:210-231`
**Issue:** Every other lifecycle handler in this component guards against re-entrancy (`if (!request || completeSubmitting) return; completeSubmitting = true;` — see `handleComplete`, `handleRejectConfirm`, `handleDeleteConfirm`, `handleCancelConfirm`, `handleApproveConfirm`). `handleAccept` alone has no `submitting` flag and no `loading` binding on its «Принять в работу» button (`RequestDetail.svelte:461`). A fast double-click sends two `accept` transitions with the same `version`; the second loses the optimistic-lock race and surfaces a spurious error toast to the operator after the first already succeeded.
**Fix:** Add a guard mirroring the sibling handlers:
```ts
let acceptSubmitting = $state(false);
async function handleAccept() {
  if (!request || acceptSubmitting) return;
  acceptSubmitting = true;
  try { /* ...existing... */ } finally { acceptSubmitting = false; }
}
```
and bind `loading={acceptSubmitting}` on the button.

## Info

### IN-01: `ReportTable` declares a `reportType` prop it never consumes

**File:** `ui/src/features/reports/ReportTable.svelte:45,49`
**Issue:** `reportType: string` is declared in `Props` and passed by `ReportsPage.svelte:481`, but the `$props()` destructure (`const { rows, columns, loading, error, isSnapshot } = $props()`) omits it. Dead prop — harmless but misleading (implies per-report-type rendering that doesn't exist).
**Fix:** Remove `reportType` from the `Props` interface and the parent call site, or consume it.

### IN-02: New-request WS toast mislabels any non-cartridge/free_form request as «Свободная форма»

**File:** `ui/src/features/requests/RequestsPage.svelte:108-113`
**Issue:** The `new_request` toast is a two-way ternary: `requestType === 'cartridge_replace' ? 'Замена картриджа' : 'Свободная форма'`. Any other type (notably `ad_register`, which the detail/list views handle explicitly elsewhere) is announced to specialists/admins as «Свободная форма».
**Fix:** Reuse the same `typeLabel` mapping used in `RequestListRow`/`RequestDetail` (add an `ad_register → 'Регистрация AD'` arm) instead of a binary ternary.

### IN-03: Cartridge consumption/refills rows show raw ISO `month_key` in the «Месяц» cell

**File:** `ui/src/features/reports/ReportTable.svelte:71-80,105-156` (config in `ReportsPage.svelte:132-143`)
**Issue:** For `consumption`/`refills`, `month_key` is both the grouping key (rendered humanized as «Сентябрь 2026» in the separator row) and a table column. `formatCellValue` has no `month_key` branch, so the cell prints the raw `"2026-09"` string next to the humanized separator — inconsistent formatting for the same value.
**Fix:** Add a `month_key` case to `formatCellValue` that routes through `formatMonthKey`, or drop the redundant column now that the separator already labels the month.

### IN-04: Month/location separator row is `aria-hidden`, dropping grouping context for screen readers

**File:** `ui/src/features/reports/ReportTable.svelte:130`
**Issue:** The separator `<tr class="report-separator" aria-hidden="true">` hides the month/location grouping label from assistive tech, so a screen-reader user hears an ungrouped flat list. Purely cosmetic `<td>`s are normally hidden, but here the `<td>` carries the only grouping label.
**Fix:** Drop `aria-hidden` (or expose the label via a visually-hidden caption/`role="rowheader"`) so the grouping is announced.

---

_Reviewed: 2026-07-23_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
