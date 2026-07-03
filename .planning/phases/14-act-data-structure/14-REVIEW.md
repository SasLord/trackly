---
phase: 14-act-data-structure
reviewed: 2026-07-03T00:00:00Z
depth: standard
files_reviewed: 15
files_reviewed_list:
  - crates/trackly-app/src/context.rs
  - crates/trackly-app/src/dto/act.rs
  - crates/trackly-app/src/dto/reports.rs
  - crates/trackly-app/src/pdf/docspec.rs
  - crates/trackly-app/src/pdf/renderer.rs
  - crates/trackly-app/src/services/act_service.rs
  - crates/trackly-app/src/services/org_db_service.rs
  - crates/trackly-app/src/services/report_service.rs
  - crates/trackly-app/src/services/template_service.rs
  - crates/trackly-app/tests/org_settings.rs
  - crates/trackly-app/tests/pdf_column_overflow.rs
  - crates/trackly-app/tests/pdf_logo.rs
  - crates/trackly-app/tests/pdf_render_act.rs
  - migrations/V033__org_settings_requisites.sql
  - ui/src/features/settings/OrgSettings.svelte
findings:
  critical: 0
  warning: 5
  info: 4
  total: 9
status: issues_found
---

# Phase 14: Code Review Report

**Reviewed:** 2026-07-03
**Depth:** standard
**Files Reviewed:** 15
**Status:** issues_found

## Summary

Phase 14 adds migration V033 (5 requisite columns: phone/fax/email/okpo/ogrn), extends the OrgPatch/OrgSettingsDto DTOs and HeaderBlock, sources act-render org requisites from `org_settings` (with an org.json fallback), and pipes live `device.notes` into act item `specs`.

The mechanics reviewed cleanly on the axes flagged in the brief:
- **SQL ordinal correctness:** All `org_settings` sites use explicit column lists (never `SELECT *`), and the `.get(N)` ordinals in `OrgDbService::get` and `get_for_pdf` correctly match their own SELECT ordering. V033 appends columns, so existing statements are unaffected. Verified against V026 schema.
- **serde(default) backward-compat:** `HeaderBlock`'s new `org_*` fields carry `#[serde(default)]`; old template JSON that omits them deserializes to empty strings. `Default` derive is present.
- **Single-writer discipline:** All mutations go through `WriterHandle::execute`.
- **org_db fallback path in act_service:** `render_pdf` degrades to an empty `OrgSettingsDto` when `org_db` is `None` (helper fixtures), returning empty requisites rather than erroring.

However, the central deliverable of the phase — making the extended requisites and live specs *appear in the produced document* — is not actually wired end-to-end. The data reaches the `HeaderBlock`/context struct but is dropped before it is drawn. See WR-01 and WR-02, which are the load-bearing findings. The remaining findings concern a logo-source split, a non-transactional multi-statement write, and CSV/error-handling robustness.

## Warnings

### WR-01: Extended requisites (phone/fax/email/okpo/ogrn) are stored and plumbed but never rendered into the act PDF

**File:** `crates/trackly-app/src/pdf/renderer.rs:130-165`, `crates/trackly-app/templates/act_handover.minijinja:17-25`
**Issue:** The whole point of V033 + the DTO/HeaderBlock extension is to put the org requisites on the printed act ("Word-fidelity sample", PDFA-03). The data now flows: DB → `get_for_pdf` → `render_pdf` context (`act_service.rs:1410-1421` emits `org.phone/fax/email/okpo/ogrn`) → template. But two links are missing:

1. **The default `act_handover.minijinja` `header` block does not emit `org_phone/org_fax/org_email/org_okpo/org_ogrn`.** It only emits `org_name/org_inn/org_kpp/org_address/logo_path/act_label/date_label`. So the render context's `org.phone` etc. are silently ignored — the template never reads them into the `HeaderBlock` JSON.
2. **`renderer.rs::render_docspec` never draws the new `HeaderBlock` fields.** The header band (lines 130-165) draws only `org_name`, `org_address`, `ИНН/КПП`, and `date_label`. Even if the template populated `header.org_phone` etc., krilla would not paint them.

Evidence this is a real gap, not an oversight in my reading: the phase's own positive test `render_pdf_with_filled_specs_and_requisites_surfaces_data` (`tests/pdf_render_act.rs:344-412`) fills phone/fax/email/okpo/ogrn via `save_fields`, but only asserts `text.contains("Ромашка")` (the org *name*, which the template does render). It never asserts any of the 5 new requisites appear in the extracted PDF text — because they don't. The feature is effectively a no-op at the output layer.

**Fix:**
- Add the fields to the template `header` block, e.g.:
```jinja
"org_phone": {{ org.phone | default('') | tojson }},
"org_fax":   {{ org.fax   | default('') | tojson }},
"org_email": {{ org.email | default('') | tojson }},
"org_okpo":  {{ org.okpo  | default('') | tojson }},
"org_ogrn":  {{ org.ogrn  | default('') | tojson }},
```
- In `render_docspec`, after the ИНН/КПП line, draw the non-empty requisites (skip empty strings so historic rows degrade to nothing, per D-02). Add a test asserting at least one new requisite value round-trips into `pdf_extract` output.

### WR-02: `specs` (live device.notes) is plumbed into the context but never rendered in the items table

**File:** `crates/trackly-app/src/services/act_service.rs:1393-1408`, `crates/trackly-app/templates/act_handover.minijinja:60-77`
**Issue:** D-01 makes `ActItemDto.specs` carry live `device.notes` (correctly loaded in `load_items_for_act` at `act_service.rs:1745,1765`), and `render_pdf` emits `"specs": it.specs` into each item JSON (line 1402). But the default `act_handover.minijinja` `items_table` columns are `["№", "Наименование", "Инв.№", "Серийный №", "Модель", "Кол-во"]` — there is no «Технические характеристики»/specs column, and `item.specs` is never referenced in the template row body. As with WR-01, the value is computed and passed but dropped before output. `render_pdf_with_filled_specs_and_requisites_surfaces_data` sets `notes = "Intel i5, 8GB ОЗУ"` but never asserts that string appears in the PDF — so the regression is untested.

**Fix:** Add a specs column to the default template's `items_table` (`columns` + per-row `{{ (item.specs | default("—", true)) | tojson }}`), and extend the positive test to assert the specs string surfaces via `pdf_extract`. If specs is intentionally deferred to a later plan, document that explicitly and remove the misleading "carries live device.notes into act specs" framing from the phase scope.

### WR-03: Logo source split — org requisites come from `org_settings`, but logo still comes from legacy `org.json`

**File:** `crates/trackly-app/src/services/act_service.rs:1351-1373, 1421`
**Issue:** `render_pdf` calls `org_db.get_for_pdf()` and destructures `(dto, _logo_bytes, _logo_mime)` — deliberately discarding the BLOB logo bytes that now live in `org_settings`. The `logo_path` put into the render context (line 1421) comes from `pipeline.organization.safe_logo_canonical(&org_legacy)` — i.e. the org.json file path. So after V033, an org that uploaded its logo through the Settings UI (which writes `org_settings.logo_blob` via `OrgDbService::save_logo`) will get correct text requisites but **no logo** on the act, because the act renderer reads the logo from org.json instead of the BLOB it just discarded. This contradicts the D-05 intent ("единый источник ... то, что пишет Settings UI"). The Settings UI (`OrgSettings.svelte`) uploads logos exclusively to the BLOB path — there is no code path that writes org.json's `logo_path` anymore for new installs.

**Fix:** Use the BLOB logo from `get_for_pdf` — pass `logo_bytes`/`logo_mime` into the `HeaderBlock` (the renderer already prioritises `logo_bytes` over `logo_path`, see `renderer.rs:178-183`). Fall back to `safe_logo` (org.json) only when the BLOB is `None`. Otherwise the logo is invisible on acts for the mainline (Settings-UI) configuration path.

### WR-04: `migrate_from_org_json` performs two dependent UPDATEs without a transaction

**File:** `crates/trackly-app/src/services/org_db_service.rs:312-337`
**Issue:** The migration hook issues two separate `conn.execute` UPDATE statements inside a single `writer.execute` closure without opening a transaction: first the text fields, then (conditionally) the logo. If the second UPDATE fails, the first is already committed (autocommit), leaving org_settings partially migrated. Worse, the subsequent `org.json → org.json.migrated` rename (line 345-347) runs in a *separate* `spawn_blocking` after the write future resolves. Only the text-field write result is checked (`if let Err(e) = write_result`); a rename failure logs but the file stays as `org.json`, so on next startup the placeholder check (`org_name == "Ваша организация"`) already fails (name was migrated) and migration will not re-run — but the un-renamed `org.json` is now stale/misleading. Other services in this file correctly wrap multi-statement writes with `conn.transaction()` (cf. `template_service.rs:74`). Best-effort semantics are documented, but a partial-write window is still a data-integrity smell for a one-shot migration.

**Fix:** Wrap the two UPDATEs in a `tx = conn.transaction()?; ...; tx.commit()?` so the logo+text writes are atomic. Consider gating the rename on the write having fully succeeded (it already is) and re-reading state so a failed rename does not leave an orphan `org.json`.

### WR-05: Report CSV/PDF `search` filter is not LIKE-escaped (inconsistent with acts search hardening)

**File:** `crates/trackly-app/src/services/report_service.rs:655-662, 1000-1007`
**Issue:** `query_acts_inner` / `count_acts_inner` build `LIKE '%{search}%'` with the raw user string bound as a parameter but **without** an `ESCAPE` clause. A `%` or `_` in the search term is treated as a wildcard, so results are wrong (not a SQL-injection issue — the value is parameterised — but a correctness bug). The act-search path in `act_service.rs` deliberately strips/escapes `%`/`_` (see `search` at line 986-990 and `escape_like` at line 1598) precisely to avoid this. The report path diverges. This file is not new in Phase 14, but it was in scope and the divergence is a latent defect touching the same requisites/reporting surface.

**Fix:** Either strip `%`/`_`/`\` from `search` (mirror `act_service::search`) or append `ESCAPE '\\'` to the LIKE clauses and escape the value. Apply consistently to all four `*_acts_inner` builders.

## Info

### IN-01: Empty-string org fields written as real values, not "unset"

**File:** `crates/trackly-app/src/services/org_db_service.rs:85-118`
**Issue:** `save_fields` is a replace-all UPDATE (documented in `reports.rs:172-174`). Because the UI always sends all fields, clearing a field to `""` overwrites any prior value. This is intended per D-02 (empty = not filled), but note there is no server-side length/format validation on inn/kpp/ogrn/okpo/phone (e.g. OGRN is 13 digits). For a print-fidelity feature a malformed OGRN silently prints. Consider light validation if the Word sample requires fixed widths.

### IN-02: `render_pdf` ignores `get_for_pdf` `has_logo` and always sets `logo_path` from org.json

**File:** `crates/trackly-app/src/services/act_service.rs:1356-1373`
**Issue:** Related to WR-03: `org_dto.has_logo` is computed by `get_for_pdf` but unused in `render_pdf`. Once WR-03 is fixed, `has_logo`/`logo_bytes` become the driver and `has_logo` can gate the fallback.

### IN-03: `compute_suffix_from_display` uses `String::find('в')` which can mis-split on a name-like display

**File:** `crates/trackly-app/src/services/act_service.rs:1644`
**Issue:** The fallback branch searches for the first Cyrillic 'в' in the formatted number to extract the return suffix. Formatted numbers are constrained to `{parent}в{sub}` so this is safe today, but the heuristic is fragile — any future display format containing 'в' before the suffix would break it. Low risk given `format_act_number`'s tight output contract; noting for maintainability. Prefer deriving the suffix structurally from `sub_number`/`act_type` rather than string-scanning the display form.

### IN-04: Tests assert only org_name reaches the PDF, giving false confidence for the new fields

**File:** `crates/trackly-app/tests/pdf_render_act.rs:404-408`
**Issue:** `render_pdf_with_filled_specs_and_requisites_surfaces_data` is named as if it proves phone/fax/email/okpo/ogrn AND specs "surface," but only asserts `contains("Ромашка")` (the name). This is why WR-01/WR-02 slipped through: the test passes while the feature does nothing. Strengthen the assertions to cover at least one new requisite and the specs string once WR-01/WR-02 are fixed.

---

_Reviewed: 2026-07-03_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
