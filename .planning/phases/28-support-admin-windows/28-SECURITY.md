---
phase: 28
slug: support-admin-windows
status: verified
threats_open: 0
asvs_level: 1
created: 2026-07-23
---

# Phase 28 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.
> Verify-mitigations mode: register authored at plan time (STRIDE, per-plan).
> No new threat scanning — each declared mitigation confirmed present in code.

---

## Trust Boundaries

Phase 28 is a UI re-tokenization phase. It introduces **no new** network endpoints,
auth paths, write paths, or trust boundaries. Existing boundaries are unchanged:

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| Browser/webview → Tauri invoke / axum `/api/v1/*` | Existing authenticated command surface (reused verbatim) | Requests, reports, settings, user records |
| Session middleware (`tower-sessions`) | Gates `/api/*` except login (unchanged) | Session cookie |
| Role gate (admin/manager/employee) | `ad_register` lifecycle actions remain admin-only (T-09-21) | Request lifecycle transitions |
| SQLite read path (`query_acts_inner`) | Authorized report read; type-coercion only change | Act rows for reports |

---

## Threat Register

All threats `mitigate` unless marked `accept`. No `high`-severity threats introduced.
Verification base commit `83a0fb0` → `HEAD`. Implementation files read-only (not modified).

| Threat ID | Category | Component | Disposition | Evidence | Status |
|-----------|----------|-----------|-------------|----------|--------|
| T-28-01-01 | Tampering(regression) | RequestsSearchAndTabs null status filter | mitigate | RequestsSearchAndTabs.svelte:62 `key === 'null' ? null : (key as StatusTab)` round-trip | closed |
| T-28-01-02 | Tampering(regression) | RequestsList → Table | mitigate | RequestsList.svelte:59 `<Table columns={4}>`; 4 headers (Автор/Тип/…/Статус) preserved | closed |
| T-28-02-01 | Elevation of Privilege | RequestDetail lifecycle + admin gate | mitigate | RequestDetail.svelte:92 `isAdmin`, :447/:500/:556 admin gates; 7 actions present; T-09-21 preserved | closed |
| T-28-02-02 | Tampering(regression) | 4 confirm-Modals | mitigate | RequestDetail.svelte reject/delete/cancel/approve Modal blocks (:631/:651/:664/:679) — count 4 | closed |
| T-28-03-01 | Tampering(regression) | ReportSubNav count-fallback | accept | Known minor visual diff (`'–'`→`0`); see Accepted Risks Log | closed |
| T-28-03-02 | Tampering(regression) | PeriodSelector month/year adapter | mitigate | PeriodSelector.svelte:93/:98 `Number(v)`; onMonthChange/onYearChange unchanged | closed |
| T-28-04-01 | Tampering(regression) | ReportTable dynamic columns/separator | mitigate | ReportTable.svelte:71 `formatCellValue`, :83 `grouped` derived intact | closed |
| T-28-04-02 | Information Disclosure | ReportTable state texts | mitigate | ReportTable.svelte:116 error text, :123 emptyTitle, :124 emptyBody — all 3 verbatim | closed |
| T-28-05-01 | Tampering(regression) | ThresholdSettings autosave-on-blur | mitigate | ThresholdSettings.svelte:48 `onfocusout={saveThreshold}` (not onblur); pushToast imported :3 | closed |
| T-28-05-02 | Tampering(regression) | StorageSettings DB-move gate | accept | Audit-only; proceedWithMove/isTauri untouched. See Accepted Risks Log | closed |
| T-28-06-01 | Tampering(regression) | NetworkSettings disabled conditions | mitigate | NetworkSettings.svelte:203/:217/:241 `disabled={saving \|\| serverRunning}` ported | closed |
| T-28-06-02 | Tampering(regression) | NetworkSettings desktop-lock checkbox | mitigate | NetworkSettings.svelte:268 `onchange={(checked) => toggleDesktopLock(checked)}` ported | closed |
| T-28-07-01 | Tampering(regression) | AD regMode ↔ auto_accept adapter | mitigate | ActiveDirectorySettings.svelte:37 `regMode = auto_accept ? 'auto' : 'confirm'`, :40 `auto_accept = regMode === 'auto'` — non-inverted round-trip | closed |
| T-28-07-02 | Tampering(regression) | OrgSettings 10 requisite fields | mitigate | OrgSettings.svelte:237–295 exactly 10 `<Input bind:value>` (name/inn/kpp/address/addr2/phone/fax/email/okpo/ogrn) | closed |
| T-28-08-01 | Tampering(HIGH-if-boundary-broken) | TemplateEditor textarea/preview/save (D-08) | mitigate | Diff touches only kind-select→Dropdown + `.form-select` CSS; textarea (:285), `sandbox=""` (:306), saveTemplate (:190), resetTemplate (:212), `apiCall('templates_validate_preview')` (:175) unchanged | closed |
| T-28-08-02 | Tampering(regression) | template-kind Select adapter | mitigate | TemplateEditor.svelte selectedKind read/write preserved; onPickGroup `selectedKind = o.id` | closed |
| T-28-09-01 | Information Disclosure | UserFormModal password field | mitigate | UserFormModal.svelte grep `type="password"` count==1 (:191); raw `<input>`, NOT Input primitive; passwordErr length validation (:111) intact | closed |
| T-28-09-02 | Tampering(regression) | UsersList/UserListRow inline delete | mitigate | UserListRow.svelte:26 `confirmDelete`, :56 `"Удалить?"` inline (not modal) | closed |
| T-28-11-01 | Tampering(regression) | RequestDetail approveRole picker | mitigate | RequestDetail.svelte:62 ROLE_OPTIONS, :698 groups, :708 `onPickGroup={(o) => (approveRole = o.id)}` same write target | closed |
| T-28-11-02 | Tampering(regression) | RequestFormModal categoryId picker | mitigate | RequestFormModal.svelte:56 `NONE_CATEGORY_ID`, :280 `categoryId = o.id === NONE_CATEGORY_ID ? null : parseInt(o.id, 10)` round-trip | closed |
| T-28-12-01 | Tampering(regression) | Backup/Network Select→Dropdown | mitigate | BackupSettings.svelte:34/:114 schedule Dropdown + disabled sentinel; NetworkSettings bind-addr Dropdown + disabled preserved | closed |
| T-28-12-02 | Tampering(scope-creep, D-08) | TemplateEditor.svelte | mitigate | Diff has zero changes to textarea/editor-wrapper/`apiCall('templates_*')` (see T-28-08-01) | closed |
| T-28-13-01 | Tampering(regression) | UserFormModal role picker | mitigate | UserFormModal.svelte:224 `onPickGroup={(o) => (form.role = o.value)}` same target; roleErr invalid state (:70) preserved | closed |
| T-28-13-02 | Tampering(regression) | PeriodSelector pickers + recalc | mitigate | PeriodSelector.svelte:156/:177/:201 wire to unchanged onMonthChange/onYearChange | closed |
| T-28-13-03 | Information Disclosure(dead CSS) | PeriodSelector :global() overrides | mitigate | PeriodSelector.svelte only `:global(.tr-dropdown*)` remain (:256/:261); no stale `.select-wrapper`/`.select` (line 254 is a comment) | closed |
| T-28-14-01/02 | Tampering(regression) | RequestsPage/RequestListRow | mitigate | RequestsPage CSS-only; RequestListRow removals limited to `role="button"`/`tabindex="0"` (moved to TableRow) — no script logic removed | closed |
| T-28-15-01 | Tampering(unverified fix) | report_service.rs sub_number CAST | mitigate | report_service.rs:749 single-line `CAST(a.sub_number AS TEXT)`; no WHERE/filter/authz change; regression test `tests/report_returns_sub_number.rs` present | closed |
| T-28-15-02 | Tampering(regression sibling) | list_device_acts | mitigate | report_service.rs:251/:270 list_device_acts calls shared query_acts_inner; type-coercion only, handover/return paths unaffected | closed |
| T-28-16-01/02 | Tampering(regression) | ReportsPage/ReportTable framed | mitigate | ReportsPage.svelte diff CSS-only (overflow/padding/min-height); no script change | closed |
| T-28-\*-SC | Tampering(supply-chain) | npm/cargo installs | accept | Dependency diff `83a0fb0..HEAD` for package.json/Cargo.toml/lockfiles is empty — no new deps. See Accepted Risks Log | closed |

*Status: open · closed*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-28-01 | T-28-03-01 | ReportSubNav count-fallback renders `0` instead of legacy `–` when a subsection count is absent. Cosmetic-only; no data loss or auth impact. Documented at plan time. | Plan 28-03 | 2026-07-23 |
| AR-28-02 | T-28-05-02 | StorageSettings desktop-only DB-move gate (`proceedWithMove`/`isTauri`) is audit-only in this phase — logic left untouched, no behavioral change to verify. | Plan 28-05 | 2026-07-23 |
| AR-28-03 | T-28-\*-SC | No new npm or cargo dependencies were added in the entire phase. Supply-chain surface unchanged; verified by empty dependency/lockfile diff across `83a0fb0..HEAD`. | Plan 28 (all) | 2026-07-23 |

*Accepted risks do not resurface in future audit runs.*

---

## Unregistered Flags

None. All SUMMARY.md `## Threat Flags` sections (28-02, 28-07, 28-08, 28-12) report
"None"; no new network surface, auth path, write path, or dependency was introduced.
No unmapped attack surface detected.

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-07-23 | 30 | 30 | 0 | gsd-security-auditor (Claude Opus 4.8) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-07-23
