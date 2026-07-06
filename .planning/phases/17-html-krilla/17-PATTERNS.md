# Phase 17: html-krilla - Pattern Map

**Mapped:** 2026-07-06
**Files analyzed:** 11 (to create/modify)
**Analogs found:** 11 / 11 (all via Phase 16 in-tree implementation)

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `crates/trackly-app/templates/report.html` (new) | config/template | transform (render) | `crates/trackly-app/templates/act_handover.html` | exact |
| `crates/trackly-app/src/pdf/html_templates.rs` (modify: add tuple) | config/utility | file-I/O | itself (Phase 16, unchanged mechanism) | exact |
| `crates/trackly-app/src/services/report_service.rs::export_pdf` (modify) | service | transform (CRUD read → HTML render) | `crates/trackly-app/src/services/act_service.rs::render_pdf` | exact |
| `crates/trackly-app/src/services/report_service.rs::row_field` (keep, reuse) | utility | transform | itself — unchanged, reused as cell-value source | exact (no change) |
| `crates/trackly-app/src/tauri_cmds/reports.rs::reports_export_pdf` / `build_reports_export_pdf` (modify) | controller (Tauri cmd) | request-response | `crates/trackly-app/src/tauri_cmds/acts.rs::acts_render_pdf` / `build_acts_render_pdf` | exact |
| `crates/trackly-app/src/http/reports.rs::handler_export_pdf` (modify) | controller (HTTP handler) | request-response | `crates/trackly-app/src/http/acts.rs::handler_render_pdf` | exact |
| `ui/src/features/reports/ReportsPage.svelte::exportPdf` (modify) + wiring to modal | component (Svelte, event handler) | request-response | `ui/src/features/acts/ActsPage.svelte` (pdfModalOpen/pdfModalAct pattern, §252) | exact |
| `ui/src/features/acts/PdfPreviewModal.svelte` (modify: add `mode='report'`) | component (modal) | request-response (self-fetch) | itself (Phase 16, extend `renderCall()`) | exact (extend, don't rewrite) |
| `ui/src/lib/api/reports.ts` (new, or inline apiCall in ReportsPage) | service (API wrapper) | request-response | `ui/src/lib/api/acts.ts::renderPdf`/`renderAcceptancePdf` | exact |
| `crates/trackly-app/src/services/template_service.rs::validate_preview` (modify: krilla → HTML) | service | transform (render) | `crates/trackly-app/src/services/act_service.rs::render_pdf` (template load + `build_safe_html_env` + `render_with_timeout`) | exact |
| `crates/trackly-app/src/services/template_service.rs::list_all_for_editor` / `update_body` / `reset_to_default` (modify: DB → file I/O) | service | file-I/O (CRUD on disk file, not DB row) | `crates/trackly-app/src/pdf/html_templates.rs` (`resolve_templates_dir`, `load_template`, `materialize_defaults_on_startup`) | exact |
| `ui/src/features/settings/TemplateEditor.svelte` (modify: kind-select, variables panel, preview) | component | request-response | itself (Phase 16-era `PdfPreviewModal.svelte` for the new HTML preview rendering approach) | role-match |

## Pattern Assignments

### `crates/trackly-app/templates/report.html` (new template file)

**Analog:** `crates/trackly-app/templates/act_handover.html` (full file read above)

**Doc-comment / header-block convention** (lines 1-30 of act_handover.html):
```html
<!DOCTYPE html>
{#- Default HTML template for ... (explain context vars + autoescape rationale) -#}
<html lang="ru">
<head>
<meta charset="UTF-8">
<title>...</title>
<style>
  @page { size: A4 portrait; margin: 20mm 15mm; }
  body { font-family: "DejaVu Sans", "Arial", sans-serif; font-size: 11pt; color: #000; margin: 0; padding: 0; }
  .header { display: grid; grid-template-columns: auto 1fr; gap: 12pt; align-items: flex-start; margin-bottom: 16pt; }
  .header .logo img { max-height: 60pt; max-width: 120pt; }
  .header .requisites { font-size: 9pt; line-height: 1.35; }
</style>
</head>
<body>
```

**Org header block to copy verbatim (D-02)** (lines 119-141):
```html
  <div class="header">
    <div class="logo">
      {%- if org.logo_data_uri %}
      {#- logo_data_uri is server-constructed exclusively from base64 output
        (RFC 4648 alphabet [A-Za-z0-9+/=], never user-controlled HTML) plus a
        hardcoded mime whitelist — `| safe` here does not reopen the XSS
        mitigation, it only prevents autoescape from HTML-entity-encoding the
        `/` in "data:image/png;base64,..." which would corrupt the URI. -#}
      <img src="{{ org.logo_data_uri | safe }}" alt="Логотип">
      {%- endif %}
    </div>
    <div class="requisites">
      {%- if org.name %}<div>{{ org.name }}</div>{%- endif %}
      {%- if org.inn %}<div>ИНН {{ org.inn }}{% if org.kpp %} / КПП {{ org.kpp }}{% endif %}</div>{%- endif %}
      {%- if org.address %}<div>{{ org.address }}</div>{%- endif %}
      {%- if org.phone %}<div>Тел.: {{ org.phone }}</div>{%- endif %}
      {%- if org.fax %}<div>Факс: {{ org.fax }}</div>{%- endif %}
      {%- if org.email %}<div>E-mail: {{ org.email }}</div>{%- endif %}
      {%- if org.okpo %}<div>ОКПО {{ org.okpo }}</div>{%- endif %}
      {%- if org.ogrn %}<div>ОГРН {{ org.ogrn }}</div>{%- endif %}
    </div>
  </div>
```
Copy this block into `report.html` unchanged (same `org.*` field names). Only the `.title`/`.subtitle` text below it and the body content differ (report title + period label instead of act number/date).

**New body content (D-01/D-03/D-04/D-05/D-07) — not ported from any existing template, build fresh per spec:**
- Title: `<div class="title">{{ report_name }}</div>` + `<div class="subtitle">{{ period_label }}</div>` (report_name/period_label passed as plain strings from Rust, same role as `act.number`/`act.date_human` above).
- Iterate `groups` (built by Rust, D-04): `{% for group in groups %}` → month-separator heading (`<h3>{{ group.month_label }}</h3>` style, zebra `<table>` with `<thead><tr>{% for col in columns %}<th>{{ col }}</th>{% endfor %}</tr></thead>` and `<tbody>{% for row in group.rows %}<tr>{% for cell in row %}<td>{{ cell }}</td>{% endfor %}</tr>{% endfor %}</tbody>`).
- Empty case (D-07): `{% if groups is not defined or groups | length == 0 %}<p>Нет данных за указанный период.</p>{% endif %}` — same string already used in `report_service.rs` (see below).
- All interpolation stays plain `{{ var }}` (autoescape ON, matching D-08) — the only `| safe` exception is `org.logo_data_uri`, exactly mirroring the act template's inline comment.

---

### `crates/trackly-app/src/pdf/html_templates.rs` (modify — add one tuple)

**Analog:** itself, unchanged mechanism (Phase 16). No new functions — only extend the const.

**Exact line to add** (extends `DEFAULT_HTML_TEMPLATES`, lines 30-39):
```rust
pub const DEFAULT_HTML_TEMPLATES: &[(&str, &str)] = &[
    (
        "act_handover.html",
        include_str!("../../templates/act_handover.html"),
    ),
    (
        "act_acceptance.html",
        include_str!("../../templates/act_acceptance.html"),
    ),
    (
        "report.html",
        include_str!("../../templates/report.html"),
    ),
];
```
`resolve_templates_dir`, `materialize_defaults_on_startup`, `load_template` require **zero changes** — they iterate `DEFAULT_HTML_TEMPLATES` generically. Existing test `materialize_creates_both_defaults_in_empty_dir` (line 112) iterates the const too — it will automatically cover the third entry; rename mentally to "all defaults" but no code change forced.

---

### `crates/trackly-app/src/services/report_service.rs::export_pdf` (modify — DocSpec/krilla → HTML)

**Analog:** `crates/trackly-app/src/services/act_service.rs::render_pdf` (lines 1341-1475) for the HTML-render pipeline shape; **current `export_pdf` body itself** (lines 512-584) for the report-specific data (org header params, per-row/month grouping) that must be preserved.

**Current signature to keep (org/logo params already match act's needs — do NOT change the public signature apart from return type):**
```rust
// report_service.rs:512-521 (current)
#[allow(clippy::too_many_arguments)]
pub async fn export_pdf(
    &self,
    rows: &ReportResponse,
    report_name: &str,
    period_label: &str,
    org: &OrgSettingsDto,
    logo_bytes: Option<Vec<u8>>,
    logo_mime: Option<String>,
    columns: &[&str],
) -> Result<Vec<u8>, AppError> {   // ← change return type to Result<String, AppError>
```

**Logo data-URI construction — copy from act_service.rs (lines 1373-1383):**
```rust
let logo_data_uri: Option<String> = logo_bytes.map(|bytes| {
    use base64::Engine;
    let mime = logo_mime.as_deref().unwrap_or("image/png");
    format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    )
});
```

**Template load — copy from act_service.rs (lines 1384-1398), swap filename to `report.html`:**
```rust
let templates_dir =
    crate::pdf::html_templates::resolve_templates_dir(&pipeline.organization.paths);
let embedded_default = crate::pdf::html_templates::DEFAULT_HTML_TEMPLATES
    .iter()
    .find(|(f, _)| *f == "report.html")
    .map(|(_, body)| *body)
    .unwrap_or("");
let template_src = crate::pdf::html_templates::load_template(
    &templates_dir,
    "report.html",
    embedded_default,
);
```
Note: `ReportService` has no `pipeline`/`organization` field today (its struct only holds `writer, readers, clock, config, pdf` — see lines 184-190). The templates-dir resolution needs a `Paths` handle; the planner must decide whether to thread `Arc<Paths>` (or `Arc<OrganizationService>` for the `.paths` accessor already used by acts) into `ReportService::new`, mirroring how `ActService` holds `organization: Option<Arc<OrganizationService>>` for exactly this purpose. This is a required wiring change — flag explicitly in the plan.

**Month-grouping (D-04) — keep the existing loop shape from current `export_pdf` (lines 539-568), but build `groups: Vec<serde_json::Value>` instead of `Vec<Section>`:**
```rust
// current (krilla) — lines 539-568, reuse the SAME grouping algorithm,
// replacing `sections.push(Section::ItemsTable{..})`/`Section::Heading{..}`
// with pushing `serde_json::json!({ "month_label": ..., "rows": [...] })`
// into a `groups: Vec<Value>` accumulator (D-04/D-05).
let mut current_month: Option<String> = None;
let mut table_rows: Vec<Vec<String>> = Vec::new();
for row in &rows.rows {
    let month_key = row.month_key.as_deref().unwrap_or("");
    if !month_key.is_empty() && Some(month_key) != current_month.as_deref() {
        if !table_rows.is_empty() {
            groups.push(serde_json::json!({
                "month_label": month_key_to_russian(current_month.as_deref().unwrap_or("")),
                "rows": std::mem::take(&mut table_rows),
            }));
        }
        current_month = Some(month_key.to_string());
    }
    table_rows.push(columns.iter().map(|col| row_field(row, col)).collect());
}
if !table_rows.is_empty() {
    groups.push(serde_json::json!({
        "month_label": month_key_to_russian(current_month.as_deref().unwrap_or("")),
        "rows": table_rows,
    }));
}
```
`row_field` (lines 591-611) and `month_key_to_russian` (line 1182) are **unchanged, reused as-is** — they already produce plain `String` cell values (D-05/D-06).

**MiniJinja render call — copy from act_service.rs (lines 1466-1474):**
```rust
let ctx = serde_json::json!({
    "org": { "name": org.org_name, "inn": org.inn, "kpp": org.kpp, "address": org.address,
              "phone": org.phone, "fax": org.fax, "email": org.email,
              "okpo": org.okpo, "ogrn": org.ogrn, "logo_data_uri": logo_data_uri },
    "report_name": report_name,
    "period_label": period_label,
    "columns": columns,
    "groups": groups,
});
let rendered = crate::pdf::minijinja_env::render_with_timeout(
    &crate::pdf::minijinja_env::build_safe_html_env(),
    "report_html",
    &template_src,
    ctx,
)
.await?;
Ok(rendered)
```
The `DocSpec`/`HeaderBlock`/`Section` construction (current lines 522-581) and the trailing `self.pdf.render_docspec(&spec)` call are **removed entirely** from this path (Req 6 — krilla frozen out of active path). Remove the `use crate::pdf::{docspec::{...}, PdfRenderer}` import at the top of the file (lines 26-29) if nothing else in the file still needs it — check `TemplateService` doesn't share the import (it doesn't; different file).

---

### `crates/trackly-app/src/tauri_cmds/reports.rs::reports_export_pdf` / `build_reports_export_pdf` (modify)

**Analog:** `crates/trackly-app/src/tauri_cmds/acts.rs::acts_render_pdf` / `build_acts_render_pdf` (lines 101-108, 220-226)

**Return-type change only — signature/body otherwise identical shape to today's:**
```rust
// tauri_cmds/acts.rs:101-108 (target shape)
pub async fn build_acts_render_pdf(
    ctx: &AppCtx,
    caller: &Identity,
    act_id: i64,
) -> Result<String, AppError> {
    authorize(caller, &Action::MutateActs)?;
    ctx.acts.render_pdf(act_id).await
}
```
Apply the same `Result<Vec<u8>, AppError>` → `Result<String, AppError>` change to `build_reports_export_pdf` (currently lines 153-198 of `reports.rs`) — the body's `.export_pdf(&rows, report_name, &period_label, &org, logo_bytes, logo_mime, &cols).await` call site is unchanged, only the return type it produces (now `String`) propagates through. `#[tauri::command] pub async fn reports_export_pdf(...) -> Result<Vec<u8>, AppError>` (line 345-350) changes its return annotation to `Result<String, AppError>` — thin wrapper body (`build_reports_export_pdf(...).await`) is untouched.

---

### `crates/trackly-app/src/http/reports.rs::handler_export_pdf` (modify)

**Analog:** `crates/trackly-app/src/http/acts.rs::handler_render_pdf` (lines 209-229)

**Copy this exact response shape, swapping the payload/builder call:**
```rust
// http/acts.rs:213-229 (target shape)
pub async fn handler_render_pdf(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<RenderPdfPayload>,
) -> Result<impl IntoResponse, AppErrorResponse> {
    let identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    let html = build_acts_render_pdf(&ctx, &identity, p.act_id)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        html,
    ))
}
```
Current `handler_export_pdf` (lines 203-219 of `http/reports.rs`) keeps its `ExportPayload` deserialize + `build_reports_export_pdf(...)` call, only the response tuple's Content-Type header changes from `"application/pdf"` to `"text/html; charset=utf-8"` and the body var changes from `bytes: Vec<u8>` to `html: String`. `handler_export_csv` (lines ~180-200, `text/csv` + `Content-Disposition: attachment`) is untouched — CSV stays a file download, only PDF export becomes inline HTML.

---

### `ui/src/features/reports/ReportsPage.svelte::exportPdf` (modify) + modal wiring

**Analog:** `ui/src/features/acts/ActsPage.svelte` (state + modal invocation pattern, line 252) and the `PdfPreviewModal` it drives.

**State + modal-open pattern to copy (ActsPage.svelte, conceptual — adapt var names):**
```svelte
<PdfPreviewModal
  open={pdfModalOpen}
  actId={pdfModalAct ? pdfModalAct.id : null}
  title={pdfModalAct ? `Печать акта №${pdfModalAct.number}` : 'Печать акта'}
  onClose={() => { pdfModalOpen = false; pdfModalAct = null; }}
/>
```
For Reports (D-09/D-10), add local state `let reportModalOpen = $state(false);` and replace the entire body of `exportPdf()` (current lines 380-430 of `ReportsPage.svelte` — the blob/tauri-plugin-fs/download logic) with simply:
```js
function exportPdf() {
  reportModalOpen = true; // PdfPreviewModal(mode='report') self-fetches on open
}
```
Then render `<PdfPreviewModal open={reportModalOpen} mode="report" reportParams={{ reportType: reportTypeKey(), filter, period: isSnapshot() ? undefined : period }} title="Печать отчёта" onClose={() => (reportModalOpen = false)} />` near the bottom of the markup, mirroring ActsPage's placement at line 252. The old `printReport()` function (lines 432-463, duplicate PDF-generation + save-dialog logic) becomes redundant once print goes through the modal — the "Печать" button (`onPrint={printReport}` at line 542 via `ReportFilters.svelte`) should be pointed at the same `reportModalOpen = true` trigger as `exportPdf`, per D-10 ("кнопка полностью заменить печатью"). `exportCsv()` (lines 356-378) and its `onExportCsv` wiring are **unchanged**.

---

### `ui/src/features/acts/PdfPreviewModal.svelte` (modify — add `mode='report'`)

**Analog:** itself (Phase 16) — extend, do not rewrite.

**`renderCall()` dispatcher to extend (current lines 79-95):**
```svelte
function renderCall(): Promise<string> {
  if (mode === 'acceptance') {
    if (!acceptancePayload) {
      return Promise.reject(new Error('acceptancePayload required for mode="acceptance"'));
    }
    return acts.renderAcceptancePdf(/* ... */);
  }
  if (mode === 'report') {
    if (!reportParams) {
      return Promise.reject(new Error('reportParams required for mode="report"'));
    }
    return apiCall<string>('reports_export_pdf', {
      reportType: reportParams.reportType,
      filter: reportParams.filter,
      period: reportParams.period,
    });
  }
  if (actId === null) {
    return Promise.reject(new Error('actId required for mode="handover"'));
  }
  return acts.renderPdf(actId);
}
```
**Props interface to extend (current lines 54-73):**
```svelte
interface ReportParams {
  reportType: string;
  filter: unknown; // ReportFilter shape
  period?: unknown; // PeriodDto | undefined for snapshot reports
}
interface Props {
  open: boolean;
  actId: number | null;
  title: string;
  onClose: () => void;
  mode?: 'handover' | 'acceptance' | 'report';
  acceptancePayload?: AcceptancePayload | null;
  reportParams?: ReportParams | null;
}
```
`ready` derivation (current lines 97-99) needs a third branch: `mode === 'report' ? reportParams !== null : ...`. **Everything else — `printViaSystemBrowser`, `printViaTopLevel`, `handlePrint`, the iframe/srcdoc markup, loading/error states, footer buttons — is untouched.** This is a pure extension, not a rewrite; the whole point of D-09 is that the existing print machinery (GAP-16-01 desktop/LAN branching) already generalizes to any HTML string regardless of `mode`.

---

### `ui/src/lib/api/reports.ts` (new — optional; or inline `apiCall`)

**Analog:** `ui/src/lib/api/acts.ts` (full file above) — same `apiCall<T>('cmd_name', {...})` wrapper convention with camelCase args.

```typescript
import { apiCall } from './client';

export const reports = {
  exportPdf: (reportType: string, filter: ReportFilter, period?: PeriodDto): Promise<string> =>
    apiCall<string>('reports_export_pdf', { reportType, filter, period }),
};
```
The planner may instead keep `PdfPreviewModal`'s `mode='report'` branch calling `apiCall` directly (as sketched above) if a dedicated `reports.ts` module is judged unnecessary churn — `ReportsPage.svelte` currently calls `apiCall` inline (no `reports.ts` exists today), so either approach is consistent with existing conventions. `acts.ts` is the reference for the wrapper style IF a file is created.

---

### `crates/trackly-app/src/services/template_service.rs::validate_preview` (modify — krilla → HTML)

**Analog:** `crates/trackly-app/src/services/act_service.rs::render_pdf` for the render pipeline (template load + `build_safe_html_env` + `render_with_timeout`); **current `validate_preview`'s demo_ctx** (lines 304-358) as the source of sample-data shape to keep/extend (D-11).

**Current signature (krilla) — lines 295-296, 391:**
```rust
pub async fn validate_preview(&self, body: &str) -> Result<Vec<u8>, AppError> {
    let env = build_safe_env();          // ← MiniJinja text env
    // ... demo_ctx ...
    let rendered_json = render_with_timeout(&env, "_preview", body, demo_ctx).await?;
    let spec = serde_json::from_str::<DocSpec>(&rendered_json)...;
    self.pdf.render_docspec(&spec)        // ← krilla call, REMOVE
}
```
**Target shape — swap engine + demo context per kind, return the rendered HTML string directly (no DocSpec/krilla round-trip):**
```rust
pub async fn validate_preview(&self, kind: &str, body: &str) -> Result<String, AppError> {
    let demo_ctx = demo_context_for_kind(kind); // D-11/D-12: per-kind sample data
    crate::pdf::minijinja_env::render_with_timeout(
        &crate::pdf::minijinja_env::build_safe_html_env(),
        "_preview",
        body,
        demo_ctx,
    )
    .await
}
```
This mirrors `act_service.rs::render_pdf`'s call to `render_with_timeout(&build_safe_html_env(), name, &template_src, ctx)` (lines 1466-1472) exactly — same helper, same env constructor, just fed the in-editor `body` string instead of a file read from disk. The existing `demo_ctx` JSON literal (lines 304-358) is a good starting skeleton for the `act_handover` case; a `report` case must be added with `report_name`, `period_label`, `columns`, `groups` (matching the new `report.html` context) per D-11/D-12. Note the caller signature change (`kind` param added) — `build_templates_validate_preview` in `tauri_cmds/settings_org.rs` (line 282-291) currently discards `_kind` (`ctx.templates.validate_preview(&body).await`) — that discard must be removed since `kind` now selects which template file + demo context to use.

---

### `crates/trackly-app/src/services/template_service.rs::list_all_for_editor` / `update_body` / `reset_to_default` (modify — DB → file I/O)

**Analog:** `crates/trackly-app/src/pdf/html_templates.rs` (`resolve_templates_dir`, `load_template`, `materialize_defaults_on_startup`) — these are the file-I/O primitives to call from the service layer instead of the current `rusqlite`/`document_templates` queries.

**Current DB-backed shape to REPLACE (lines 145-174 `list_all_for_editor`, 179-219 `update_body`, 222-258 `reset_to_default`)** — these query/UPDATE the `document_templates` table via `self.readers`/`self.writer`. **D-13 freezes this table and its seed** (`seed_defaults_on_startup`, lines 88-142, `DEFAULT_TEMPLATES` const, lines 43-54) — do not touch those; they remain compiled and running, just no longer wired to the editor UI.

**Target shape — mirror `html_templates::load_template` + filesystem write, keyed by kind → filename mapping (`act_handover` → `act_handover.html`, `act_acceptance` → `act_acceptance.html`, `report` → `report.html`):**
```rust
// list_all_for_editor — read each of the 3 known kinds from disk via load_template
pub async fn list_all_for_editor(&self) -> Result<Vec<TemplateEditorItem>, AppError> {
    let templates_dir = crate::pdf::html_templates::resolve_templates_dir(&self.paths);
    Ok(crate::pdf::html_templates::DEFAULT_HTML_TEMPLATES
        .iter()
        .map(|(filename, default_body)| {
            let kind = filename.trim_end_matches(".html");
            let body = crate::pdf::html_templates::load_template(&templates_dir, filename, default_body);
            let is_default = body == *default_body;
            TemplateEditorItem { id: 0, kind: kind.to_string(), body, is_default }
        })
        .collect())
}

// update_body — write kind's file to disk (portable path, D-13)
pub async fn update_body(&self, caller: &Identity, kind: &str, body: String) -> Result<(), AppError> {
    authorize(caller, &Action::ManageSettings)?;
    // MiniJinja syntax validation unchanged (lines 187-195 current impl)
    let templates_dir = crate::pdf::html_templates::resolve_templates_dir(&self.paths);
    let filename = format!("{kind}.html");
    tokio::fs::write(templates_dir.join(&filename), body)
        .await
        .map_err(|e| AppError::Internal { source_chain: format!("write {filename}: {e}") })
}

// reset_to_default — delete file (or overwrite with embedded default) so next load_template call falls back / matches default
pub async fn reset_to_default(&self, caller: &Identity, kind: &str) -> Result<(), AppError> {
    authorize(caller, &Action::ManageSettings)?;
    let filename = format!("{kind}.html");
    let default_body = crate::pdf::html_templates::DEFAULT_HTML_TEMPLATES
        .iter()
        .find(|(f, _)| *f == filename)
        .map(|(_, body)| *body)
        .ok_or(AppError::NotFound { entity: "default_template", id: 0 })?;
    let templates_dir = crate::pdf::html_templates::resolve_templates_dir(&self.paths);
    tokio::fs::write(templates_dir.join(&filename), default_body)
        .await
        .map_err(|e| AppError::Internal { source_chain: format!("reset write {filename}: {e}") })
}
```
`TemplateService` needs a `paths: Arc<Paths>` (or equivalent) field added to its struct (currently `writer, readers, clock, pdf` only, lines 57-62) — same wiring concern flagged for `ReportService` above; both services now need filesystem access via `Paths`/`resolve_templates_dir`, which they don't currently hold. `get_active(kind)` (lines 260-289, DB-backed) is used by the render path historically but **acts no longer call it** (they call `html_templates::load_template` directly per Phase 16) — leave `get_active` as dead/frozen code alongside the DB table, do not delete (mirrors D-13 hygiene).

**`TemplateEditorItem` DTO (unchanged struct, `crates/trackly-app/src/dto/reports.rs:273-282`)** — `id: i64` becomes meaningless for file-backed kinds; planner may hardcode `0` or drop the field if `bindings.ts` churn is acceptable — Claude's Discretion per CONTEXT.md.

---

### `ui/src/features/settings/TemplateEditor.svelte` (modify)

**Analog:** itself (current DB-backed editor, full file read above) for structure/layout; `PdfPreviewModal.svelte`'s `srcdoc` iframe pattern for the new HTML preview (replacing the `blobUrl`/`application/pdf` iframe at lines 98-102, 222-225).

**Kind-select (line 164) — unchanged markup, just add `report` to `KIND_LABELS` (line 15-18):**
```svelte
const KIND_LABELS: Record<string, string> = {
  act_handover: 'Акт приёма-передачи',
  act_acceptance: 'Документ приёмки товара',
  report: 'Отчёт',
};
```

**Preview call — replace krilla blob-preview (lines 90-112) with HTML srcdoc, mirroring `PdfPreviewModal`'s iframe:**
```svelte
// current (krilla, REMOVE):
const bytes = await apiCall<number[]>('templates_validate_preview', { kind: selectedKind, body });
const blob = new Blob([new Uint8Array(bytes)], { type: 'application/pdf' });
blobUrl = URL.createObjectURL(blob);
// ...
// target (HTML, D-11):
const html = await apiCall<string>('templates_validate_preview', { kind: selectedKind, body });
previewHtml = html; // $state<string | null>
```
Preview markup (lines 222-225) becomes an `<iframe srcdoc={previewHtml} title="Превью" class="pdf-iframe">` — same `iframe`/`class="pdf-iframe"` convention already used by `PdfPreviewModal.svelte` (line 261: `<iframe srcdoc={htmlContent} title="Document Preview" class="pdf-iframe">`), just without print buttons (this is a validate-preview, not a print flow) — remove the `blobUrl` state var and its `URL.revokeObjectURL` cleanup (lines 24, 61-69, 82-86) since there's no object URL anymore with `srcdoc`.

**Save/reset (lines 114-154) — API calls unchanged (`templates_update_body`, `templates_reset_to_default`), only the backend semantics change (DB row → file write) which is transparent to this component.**

**Variables panel (lines 173-195, D-12) — currently a single static hardcoded block; must become per-kind (`$derived` on `selectedKind`):**
```svelte
const VARIABLES_BY_KIND: Record<string, { col: string; items: {code: string; desc: string}[] }[]> = {
  act_handover: [ /* org.*, act.* fields per act_handover.html doc-comment */ ],
  act_acceptance: [ /* device.*, document.* fields */ ],
  report: [ /* org.*, report_name, period_label, columns, groups[].month_label, groups[].rows[] */ ],
};
const currentVariables = $derived(VARIABLES_BY_KIND[selectedKind] ?? []);
```
Render `currentVariables` instead of the hardcoded `.var-col` blocks (lines 176-193) — same `<code>`/`<p class="var-item">` markup, just data-driven per kind.

---

## Shared Patterns

### File-first + embedded fallback + materialize-on-startup
**Source:** `crates/trackly-app/src/pdf/html_templates.rs` (whole file)
**Apply to:** `report_service.rs::export_pdf`, `template_service.rs::{list_all_for_editor, update_body, reset_to_default, validate_preview}`
```rust
pub fn resolve_templates_dir(paths: &Paths) -> PathBuf { /* TRACKLY_TEMPLATES_DIR env override, else paths.templates_dir() */ }
pub fn materialize_defaults_on_startup(templates_dir: &Path) -> Result<(), AppError> { /* idempotent insert-only */ }
pub fn load_template(templates_dir: &Path, filename: &str, embedded_default: &str) -> String { /* read-on-render, never fails */ }
```

### MiniJinja safe-HTML render pipeline
**Source:** `crates/trackly-app/src/pdf/minijinja_env.rs` (`build_safe_html_env`, lines 53-61; `render_with_timeout`, lines 68-~95)
**Apply to:** `report_service.rs::export_pdf`, `template_service.rs::validate_preview`
```rust
pub fn build_safe_html_env() -> Environment<'static> {
    let mut env = Environment::new();
    env.set_undefined_behavior(UndefinedBehavior::Strict);
    env.set_auto_escape_callback(|_| AutoEscape::Html);
    env.set_recursion_limit(64);
    env.set_fuel(Some(100_000));
    env // no loader — no filesystem includes (T-16-02)
}
```
`render_with_timeout(env, name, template_src, ctx)` clones the env, adds the template owned, and enforces a 5s wall-clock timeout inside `spawn_blocking`.

### Logo data-URI construction
**Source:** `crates/trackly-app/src/services/act_service.rs` lines 1373-1383 (usage); byte source is `OrgDbService::get_for_pdf` (`crates/trackly-app/src/services/org_db_service.rs:363-`) or `OrganizationService::read_logo_bytes` (`crates/trackly-app/src/services/organization_service.rs:153-177`) depending on call site — `report_service.rs::export_pdf` already receives `logo_bytes`/`logo_mime` as params from its Tauri/HTTP caller (`tauri_cmds/reports.rs` lines 162-180 call `ctx.org_db.get_logo_bytes()` + a separate `logo_mime` query) — reuse those params, just add the `data:` URI formatting step inside `export_pdf` itself (mirroring act_service, not duplicating the byte-fetch).
```rust
let logo_data_uri: Option<String> = logo_bytes.map(|bytes| {
    use base64::Engine;
    let mime = logo_mime.as_deref().unwrap_or("image/png");
    format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes))
});
```

### HTTP text/html response tuple
**Source:** `crates/trackly-app/src/http/acts.rs` lines 213-229 (`handler_render_pdf`)
**Apply to:** `http/reports.rs::handler_export_pdf`
```rust
Ok((
    StatusCode::OK,
    [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
    html, // String
))
```

### Self-fetch preview modal (frontend)
**Source:** `ui/src/features/acts/PdfPreviewModal.svelte` (whole file, esp. `renderCall()` lines 79-95, `handlePrint`/`printViaSystemBrowser`/`printViaTopLevel` lines 154-244)
**Apply to:** `ReportsPage.svelte` (via `mode='report'` on the same component — no new modal component)
No excerpt duplication needed — this is a single shared component extended with a third `mode`, not copied.

### Tauri thin-wrapper + `build_*` free-function split
**Source:** `crates/trackly-app/src/tauri_cmds/acts.rs` lines 101-108 (`build_acts_render_pdf`) + lines 220-226 (`#[tauri::command] acts_render_pdf`)
**Apply to:** `reports.rs::{build_reports_export_pdf, reports_export_pdf}`, `settings_org.rs::{build_templates_validate_preview, templates_validate_preview}`
```rust
pub async fn build_X(ctx: &AppCtx, caller: &Identity, /* args */) -> Result<T, AppError> {
    authorize(caller, &Action::Whatever)?;
    ctx.service.method(/* args */).await
}
#[tauri::command]
#[specta::specta]
pub async fn X(state: tauri::State<'_, AppCtx>, /* args */) -> Result<T, AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_X(state.inner(), &caller, /* args */).await
}
```
Both the Tauri command AND the HTTP handler (`http/reports.rs`) call the same `build_*` function — this is the "dual access path" architecture the project CLAUDE.md mandates (Tauri invoke + HTTP share business logic).

## No Analog Found

None. Every file in scope for Phase 17 has a direct, already-shipped Phase-16 analog in the same codebase (acts' HTML-print migration is the literal template for this phase). The only genuinely new artifact is the **body content** of `report.html` (D-01 "new clean table design" — zebra rows, thead, month separators) — there is no existing zebra-table HTML in the codebase to copy; this must be authored fresh per the SPEC/CONTEXT constraints (A4 print, autoescape-safe interpolation), using `act_handover.html`'s `<style>`/`@page` conventions as the only structural precedent.

## Metadata

**Analog search scope:** `crates/trackly-app/src/{pdf,services,tauri_cmds,http}/`, `crates/trackly-app/templates/`, `ui/src/features/{acts,reports,settings}/`, `ui/src/lib/api/`
**Files scanned:** ~20 (act_service.rs, html_templates.rs, act_handover.html, act_acceptance.html, minijinja_env.rs, org_db_service.rs, organization_service.rs, report_service.rs, template_service.rs, tauri_cmds/{acts,reports,settings_org}.rs, http/{acts,reports}.rs, ui PdfPreviewModal.svelte, ActsPage.svelte, ReportsPage.svelte, ReportFilters.svelte, TemplateEditor.svelte, acts.ts, dto/reports.rs)
**Pattern extraction date:** 2026-07-06
