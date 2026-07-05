# Phase 16: Documents HTML Print - Pattern Map

**Mapped:** 2026-07-05
**Files analyzed:** 9 (2 new backend, 2 new templates, 3 modified backend, 2 modified frontend, 1 test suite additions)
**Analogs found:** 9 / 9

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|--------------------|------|-----------|-----------------|----------------|
| `crates/trackly-app/src/pdf/html_templates.rs` (NEW) | utility / config (file-resolver + seed) | file-I/O | `crates/trackly-infra/src/paths.rs` (`Paths::resolve`/`resolve_for_exe_dir`) + `crates/trackly-app/src/services/template_service.rs` (`seed_defaults_on_startup`, `DEFAULT_TEMPLATES`) | role-match (composite of two exact analogs) |
| `crates/trackly-app/templates/act_handover.html` (NEW) | template asset | transform (render) | `crates/trackly-app/templates/act_handover.minijinja` | exact (content/variables ported 1:1) |
| `crates/trackly-app/templates/act_acceptance.html` (NEW) | template asset | transform (render) | `crates/trackly-app/templates/act_acceptance.minijinja` | exact |
| `crates/trackly-app/src/services/act_service.rs::render_pdf` (MODIFIED) | service | request-response / transform | itself (same function, lines 1342-1472) + `pdf::minijinja_env::render_with_timeout` + `org_db_service.rs::get_for_pdf` | exact (in-place rewrite of render path) |
| `crates/trackly-app/src/services/act_service.rs::render_acceptance_pdf` (MODIFIED) | service | request-response / transform | itself (lines 1478-1562) | exact |
| `crates/trackly-app/src/tauri_cmds/acts.rs` (MODIFIED) | controller (Tauri command) | request-response | itself — `build_acts_render_pdf`/`acts_render_pdf`/`build_devices_render_acceptance_pdf`/`devices_render_acceptance_pdf` (lines 97-104, 116-129, 212-220, 281-300); `acts_open_pdf_in_system` (lines 240-279) removed | exact (signature-only change: `Vec<u8>` → `String`) |
| `crates/trackly-app/src/http/acts.rs` (MODIFIED) | route (axum handler) | request-response | itself — `handler_render_pdf`/`handler_render_acceptance_pdf` (lines 209-250); content-type precedent from `crates/trackly-app/src/http/mod.rs:184-189` (`text/html; charset=utf-8`) | exact |
| `ui/src/features/acts/PdfPreviewModal.svelte` (MODIFIED) | component | streaming→display (was blob fetch, becomes direct string assign) | itself (lines 1-307) | exact (blob-URL section replaced by `srcdoc`) |
| `ui/src/lib/api/acts.ts` / `ui/src/lib/api/pdf.ts` (MODIFIED) | hook / API client | request-response | itself — `acts.renderPdf`/`acts.renderAcceptancePdf` (acts.ts:39-49); `fetchPdfBlob`/`revokePdfUrl` (pdf.ts, entire file) — blob helpers become obsolete/simplified | exact |
| `crates/trackly-app/src/context.rs` (MODIFIED — startup wiring) | provider / startup wiring | event-driven (once at boot) | itself — `templates.seed_defaults_on_startup().await?;` (line 208) | exact |

## Pattern Assignments

### `crates/trackly-app/src/pdf/html_templates.rs` (NEW module — file-resolver + seed + read-on-render)

**Analog 1 (path resolution):** `crates/trackly-infra/src/paths.rs`

**Portable path-resolve pattern** (lines 32-49, `Paths::resolve` / `resolve_for_exe_dir`):
```rust
pub fn resolve() -> Result<Self, AppError> {
    let exe = std::env::current_exe().map_err(|e| AppError::Internal {
        source_chain: format!("current_exe failed: {e}"),
    })?;
    let exe_dir = exe
        .parent()
        .ok_or_else(|| AppError::Internal {
            source_chain: format!("current_exe has no parent dir (got {})", exe.display()),
        })?
        .to_path_buf();
    Self::resolve_for_exe_dir(exe_dir)
}

/// Тестовый seam: задаёт exe_dir вручную, минуя `current_exe()`.
/// Используется в `tests/paths_test.rs`.
pub fn resolve_for_exe_dir(exe_dir: PathBuf) -> Result<Self, AppError> {
    ...
    let db_path = exe_dir.join("trackly.db");
    ...
}
```
**Apply this shape to `html_templates.rs`:** a `resolve_templates_dir() -> PathBuf` (or a method taking `exe_dir: &Path`) that joins `exe_dir.join("templates")`, with a test/dev seam. D-07 asks for an **ENV-override** rather than a param — mirror the mock-env pattern below instead of (or in addition to) `resolve_for_exe_dir`-style DI, since CONTEXT.md explicitly calls out `TRACKLY_AD_MOCK`/`TRACKLY_SNMP_MOCK` as the precedent:

**Env-override precedent** (`crates/trackly-app/src/context.rs:270`):
```rust
let use_ad_mock = config.ad.use_mock || std::env::var("TRACKLY_AD_MOCK").is_ok();
```
```rust
// crates/trackly-app/src/context.rs:306
let use_mock = std::env::var("TRACKLY_SNMP_MOCK").is_ok();
```
Apply the same `std::env::var("TRACKLY_TEMPLATES_DIR")` check ahead of the `current_exe()`-derived default — env wins when set (dev/tests), falls back to `exe_dir.join("templates")` in prod. Every existing PDF-pipeline test already carries a `Paths::resolve_for_exe_dir(dir.path().to_path_buf())` seam (see `tests/pdf_render_act.rs:43`) — the new module should offer an equivalent test-friendly constructor (e.g. `TemplatesDir::resolve(exe_dir: &Path)` or read the env var directly) so tests don't need to touch real `current_exe()`.

**Analog 2 (embedded-default + seed-on-startup):** `crates/trackly-app/src/services/template_service.rs`

**Embedded defaults array** (lines 41-54):
```rust
pub const DEFAULT_TEMPLATES: &[(&str, &str, &str)] = &[
    (
        "act_handover",
        "Дефолтный шаблон акта приёма-передачи",
        include_str!("../../templates/act_handover.minijinja"),
    ),
    (
        "act_acceptance",
        "Дефолтный шаблон документа приёма",
        include_str!("../../templates/act_acceptance.minijinja"),
    ),
];
```
Mirror this exact shape with `include_str!("../../templates/act_handover.html")` / `act_acceptance.html"` as the fallback constants for D-06.

**Seed-on-startup pattern** (lines 88-140, `seed_defaults_on_startup`): materializes each default row into the DB if missing; **for D-05 the new module materializes into the filesystem instead of a DB row** — write `include_str!` default to `templates_dir.join("act_handover.html")` via `std::fs::write` (or `tokio::fs::write` if called from an async context) **only if the file does not already exist** (do NOT overwrite user edits — unlike `template_service`'s auto-upgrade branch (lines 119-131) which DOES overwrite `is_default=1` rows, D-05/D-06 in CONTEXT.md calls only for "if missing, materialize" — no auto-upgrade-in-place requirement was locked for the file path, so the simpler idempotent-insert-only branch (lines 108-118) is the right shape to imitate, not the auto-upgrade branch):
```rust
match existing {
    None => {
        tx.execute(
            "INSERT INTO document_templates ...",
            params![kind, name, body, now],
        )
        .map_err(map_rusqlite)?;
        tracing::info!("Seeded default template kind={kind} name={name}");
    }
    ...
}
```
Translate to: `if !path.exists() { std::fs::write(&path, default_html)?; tracing::info!("Materialized default HTML template at {}", path.display()); }`.

**Read-on-render + fallback pattern (D-06/D-08):** no direct DB analog exists (DB path always reads from `document_templates` via `pipeline.templates.get_active("act_handover")` — see `act_service.rs:1378` / `:1488`) — this is new logic. Shape it as:
```rust
pub fn load_template(templates_dir: &Path, filename: &str, embedded_default: &str) -> String {
    std::fs::read_to_string(templates_dir.join(filename))
        .unwrap_or_else(|_| embedded_default.to_string())
}
```
This directly satisfies D-06 ("генерация не падает") and D-08 (read-on-render, no watch).

---

### `crates/trackly-app/templates/act_handover.html` / `act_acceptance.html` (NEW)

**Analog:** `crates/trackly-app/templates/act_handover.minijinja` (full file read above, 162 lines)

**Variables/loops to port as-is** (same MiniJinja syntax, same `ctx` shape — D-04 says context is unchanged except `logo_path` → `logo` data-URI):
```jinja
{{ act.number }}{{ act.suffix | default('') }}
{{ org.name | tojson }}  →  becomes plain {{ org.name }} in HTML (no tojson; HTML autoescape handles it)
{%- for item in act.items %}
  ...
  {%- if item.inventory_no %} ... {%- endif -%}
{%- endfor %}
{%- if act.deadline_human %} ... {%- elif act.deadline %} ... {%- endif %}
{%- if act.parent %} ... {%- endif %}
```
**Key porting note:** the `.minijinja` files emit **DocSpec JSON** (`| tojson` filters everywhere, `AutoEscape::None` per `minijinja_env.rs:34`). The new `.html` templates must emit **raw HTML** — drop all `| tojson` filters (use plain `{{ var }}` — autoescape must be ON, see Shared Patterns below), replace `field_row`/`centered_text`/`signature`/`spacer` JSON section objects with actual `<div>`/`<table>` HTML+CSS matching the Word-sample block order documented in the template's own header comment (lines 1-21): header (logo+reqs) → centered title → number/date → intro `field_row` → per-item `field_row`s (label | underlined value, no "Устройство №N") → "Сроком до" → signatures "Выдал/Получил".

**Self-contained requirement (D-11, Req 6):** logo must be `<img src="{{ org.logo_data_uri }}">` with a `data:image/...;base64,...` value built in Rust (see `act_service.rs` changes below) — NOT `org.logo_path` (a filesystem path, useless in a browser-rendered HTML string).

---

### `crates/trackly-app/src/services/act_service.rs::render_pdf` (MODIFIED, currently lines 1342-1472)

**Analog:** itself — same function, adapted in place. Current 3-stage pipeline call to imitate the *shape* of but change the *target*:

**Current org/logo assembly to keep** (lines 1351-1377, `get_for_pdf` gives `logo_bytes: Option<Vec<u8>>` + `logo_mime: Option<String>` directly):
```rust
let (org_dto, logo_bytes, logo_mime) = match pipeline.org_db {
    Some(org_db) => {
        let (dto, logo_bytes, logo_mime) = org_db.get_for_pdf().await?;
        (dto, logo_bytes, logo_mime)
    }
    None => ( /* empty OrgSettingsDto fallback */ None, None ),
};
```
**New step — build `data:` URI (D-11) instead of propagating `logo_bytes` into a DocSpec struct** (replaces lines 1460-1469's `spec.header.logo_bytes = Some(bytes)` post-parse injection):
```rust
let logo_data_uri: Option<String> = logo_bytes.map(|bytes| {
    use base64::Engine;
    let mime = logo_mime.as_deref().unwrap_or("image/png");
    format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes))
});
```
(Confirm `base64` crate is already a dependency — grep `Cargo.toml`; krilla/rest of stack likely pulls it transitively, but add explicitly if not present as a direct dep.)

**Context `ctx` json — reuse as-is per D-04** (lines 1414-1444), only change `"logo_path"` key to `"logo_data_uri"` (or add alongside, executor's call):
```rust
let ctx = serde_json::json!({
    "org": {
        "name": org_dto.org_name,
        ...
        "logo_data_uri": logo_data_uri,
    },
    "act": { ... same as before ... },
    "return": { ... same as before ... },
});
```

**Render call — same helper, new template + template source lookup replaces `pipeline.templates.get_active(...)`** (old line 1378 `let template_src = pipeline.templates.get_active("act_handover").await?;` → new: read from `html_templates::load_template(...)`):
```rust
let rendered_html = crate::pdf::minijinja_env::render_with_timeout(
    &pipeline.pdf.minijinja_env,   // or a new HTML-mode env — see Shared Patterns
    "act_handover_html",
    &template_src,   // now HTML source from templates/ or include_str! fallback
    ctx,
)
.await?;
```

**Remove:** the DocSpec parse+patch+krilla-render tail (old lines 1454-1471: `serde_json::from_str::<DocSpec>`, `spec.header.logo_bytes = ...`, `pipeline.pdf.render_docspec(&spec)`). Return type changes `Result<Vec<u8>, AppError>` → `Result<String, AppError>` — `rendered_html` is the final return value directly.

---

### `crates/trackly-app/src/services/act_service.rs::render_acceptance_pdf` (MODIFIED, currently lines 1478-1562+)

**Analog:** itself, same restructuring as `render_pdf` above — the acceptance path is structurally identical but simpler (single device, `pipeline.organization.safe_logo_canonical(&org)` for the legacy org.json logo path at line 1487). Apply the same `data:` URI substitution; `org_db`-based org isn't wired here (uses legacy `organization.read()`), so check whether `OrganizationService` exposes raw logo bytes (`safe_logo_canonical` only returns a `PathBuf`) — if it only returns a path, read the file via `std::fs::read` + `base64` encode inline, or extend `OrganizationService` with a bytes-returning helper mirroring `OrgDbService::get_for_pdf` (lines 363-401 of `org_db_service.rs`) as the analog for "read logo bytes + mime for embedding."

---

### `crates/trackly-app/src/tauri_cmds/acts.rs` (MODIFIED)

**Analog:** itself — signature-only change, structure fully preserved.

**build_* / thin-wrapper pattern to keep unchanged** (lines 96-104):
```rust
pub async fn build_acts_render_pdf(
    ctx: &AppCtx,
    caller: &Identity,
    act_id: i64,
) -> Result<Vec<u8>, AppError> {   // → change to Result<String, AppError>
    authorize(caller, &Action::MutateActs)?;
    ctx.acts.render_pdf(act_id).await
}
```
```rust
#[tauri::command]
#[specta::specta]
pub async fn acts_render_pdf(
    state: tauri::State<'_, AppCtx>,
    act_id: i32,
) -> Result<Vec<u8>, AppError> {   // → Result<String, AppError>
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_acts_render_pdf(state.inner(), &caller, act_id as i64).await
}
```
Same for `build_devices_render_acceptance_pdf` / `devices_render_acceptance_pdf` (lines 116-129, 281-300).

**Remove entirely (D-10):** `acts_open_pdf_in_system` (lines 240-279) — the whole command including its path-canonicalization guard (`tauri_plugin_shell::ShellExt` import at line 20 becomes unused — remove import too if nothing else in the file uses it).

**Bindings drift check (Shared Pattern):** per CONTEXT.md "Специта-биндинги: смена возврата `Vec<u8>`→`String` требует `export_bindings`" — run whatever `cargo test --test specta_bindings` / `export_bindings` CI gate exists after the type change; grep for the test name before executing.

---

### `crates/trackly-app/src/http/acts.rs` (MODIFIED)

**Analog:** itself — `handler_render_pdf` (lines 209-225) and `handler_render_acceptance_pdf` (lines 227-250).

**Current `application/pdf` response to replace:**
```rust
pub async fn handler_render_pdf(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<RenderPdfPayload>,
) -> Result<impl IntoResponse, AppErrorResponse> {
    let identity = session_identity(&session).await.map_err(AppErrorResponse::from)?;
    let bytes = build_acts_render_pdf(&ctx, &identity, p.act_id)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/pdf")],
        bytes,
    ))
}
```
**New shape** — same skeleton, `bytes: Vec<u8>` → `html: String`, content-type `application/pdf` → `text/html; charset=utf-8` (exact string precedent at `crates/trackly-app/src/http/mod.rs:186`):
```rust
Ok((
    StatusCode::OK,
    [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
    html,
))
```
Same substitution in `handler_render_acceptance_pdf` (lines 227-250). `router()` (lines 267-286) is unchanged — same routes, same paths, only response bodies differ.

---

### `ui/src/features/acts/PdfPreviewModal.svelte` (MODIFIED)

**Analog:** itself, entire file (307 lines) — this is a **blob-to-srcdoc conversion**, not a rewrite.

**State to remove/replace** (lines 56-61):
```ts
let blobUrl = $state<string | null>(null);
let pdfBytes = $state<number[] | null>(null);
```
→
```ts
let htmlContent = $state<string | null>(null);
```

**Render-call fetch pattern to keep the *shape* of** (lines 98-114, `renderCall()`):
```ts
function renderCall(): Promise<number[]> {   // → Promise<string>
  if (mode === 'acceptance') {
    ...
    return acts.renderAcceptancePdf(...);
  }
  ...
  return acts.renderPdf(actId);
}
```
(Both `acts.renderPdf`/`acts.renderAcceptancePdf` change their declared return type in `acts.ts` — see next section — `renderCall`'s body is untouched, only its type annotation.)

**`$effect` fetch-and-assign pattern** (lines 120-169) — replace the `fetchPdfBlob` + blob-URL creation/cleanup with direct string assignment (no URL lifecycle needed for `srcdoc`):
```ts
$effect(() => {
  if (!ready) {
    htmlContent = null;
    errorMsg = null;
    return;
  }
  loading = true;
  errorMsg = null;
  let cancelled = false;
  (async () => {
    try {
      const html = await renderCall();
      if (cancelled) return;
      htmlContent = html;
    } catch (e: unknown) {
      if (cancelled) return;
      errorMsg = /* same error-shape extraction as lines 152-156 */;
    } finally {
      if (!cancelled) loading = false;
    }
  })();
  return () => { cancelled = true; };
});
```

**Iframe render — `src={blobUrl}` → `srcdoc={htmlContent}`** (line 250):
```svelte
{:else if blobUrl !== null}
  <iframe bind:this={iframeEl} src={blobUrl} title="PDF Preview" class="pdf-iframe"></iframe>
```
→
```svelte
{:else if htmlContent !== null}
  <iframe bind:this={iframeEl} srcdoc={htmlContent} title="Document Preview" class="pdf-iframe"></iframe>
```

**`handlePrint` — keep exactly as-is** (lines 224-234, this was already the target UX per D-09):
```ts
function handlePrint() {
  if (!ready) return;
  if (iframeEl?.contentWindow) {
    try {
      iframeEl.contentWindow.focus();
      iframeEl.contentWindow.print();
    } catch {
      pushToast('error', 'Не удалось вызвать диалог печати');
    }
  }
}
```

**`handleSave` (lines 171-193) and `handleOpen` (lines 195-222) — remove or replace.** D-10 removes `acts_open_pdf_in_system` entirely, so `handleOpen` and its footer button must be deleted. `handleSave` currently does `writeFile(path, new Uint8Array(bytes))` (binary PDF write) — since there is no backend PDF anymore, either (a) drop "Сохранить как PDF" as a distinct action (printing IS the save-as-PDF path via the browser dialog per Req 5), or (b) repurpose it to save the raw `.html` file via `writeFile(path, htmlContent, {..text})`. This is Claude's Discretion per CONTEXT.md ("Механика передачи HTML-строки во фронт... в деталях") — flag for planner/executor decision; **do not silently keep the old binary-write code**, it will corrupt data since `htmlContent` is now a `string`, not `number[]`.

**Footer buttons (lines 258-269)** — remove "Открыть в системном просмотрщике" button (tied to deleted `acts_open_pdf_in_system`); keep "Закрыть", "Печать"; resolve "Сохранить как PDF" per above.

---

### `ui/src/lib/api/acts.ts` + `ui/src/lib/api/pdf.ts` (MODIFIED)

**Analog:** itself.

**Current typed API calls to change return type on** (`acts.ts:39-49`):
```ts
/** Plan 04 — PDF render handover акта (возвращает PDF bytes как number[]). */
renderPdf: (actId: number): Promise<number[]> => apiCall<number[]>('acts_render_pdf', { actId }),

renderAcceptancePdf: (
  deviceId: number,
  giverName: string,
  receiverName: string,
  dateUtc: number,
): Promise<number[]> =>
  apiCall<number[]>('devices_render_acceptance_pdf', { ... }),
```
→ change `Promise<number[]>` / `apiCall<number[]>` to `Promise<string>` / `apiCall<string>` (Tauri now serializes a `String`, not a byte array). Update the doc comment ("возвращает PDF bytes как number[]" → "возвращает HTML-документ строкой").

**`pdf.ts` — entire file is now obsolete** (`fetchPdfBlob`/`revokePdfUrl` existed solely to convert `number[]` → `Blob` → object URL for `<iframe src>`). Since `PdfPreviewModal.svelte` moves to `srcdoc` with a plain string, this file's helpers have no callers left — delete the file (or leave an empty/deprecated stub if another module still imports it; grep first: `grep -rn "api/pdf'" ui/src`).

---

### `crates/trackly-app/src/context.rs` (MODIFIED — startup wiring, ~line 200-208)

**Analog:** itself — `templates.seed_defaults_on_startup().await?;` (line 208), inserted right after `TemplateService::new(...)` (lines 200-204) and before `PdfRenderer::new()` usage.

**Pattern to mirror for the new file-based materialization step (D-05):**
```rust
let templates = Arc::new(TemplateService::new(
    writer.clone(),
    readers.clone(),
    clock.clone(),
));
let pdf = Arc::new(PdfRenderer::new());

// Seed default templates on first run (idempotent).
templates.seed_defaults_on_startup().await?;
```
Add a parallel call, e.g.:
```rust
// Phase 16: materialize embedded HTML defaults into templates/ (D-05), idempotent.
crate::pdf::html_templates::materialize_defaults_on_startup(&templates_dir)?;
```
placed at the same point in `AppCtx::build` (after `paths_arc` is available, since the templates dir is derived from `paths_arc.exe_dir()` — see `paths_arc` construction at context.rs:198). This keeps startup wiring for both the DB-templates path (frozen, still seeded — SPEC out-of-scope says no migration, but D-13/frozen krilla path still exists so `seed_defaults_on_startup` for `document_templates` should NOT be removed) and the new file-templates path side by side.

---

## Shared Patterns

### MiniJinja safe-mode environment — needs an HTML-mode variant
**Source:** `crates/trackly-app/src/pdf/minijinja_env.rs:31-39` (`build_safe_env`)
**Apply to:** the new HTML render call site in `act_service.rs`.

Current env is JSON-mode (autoescape OFF):
```rust
pub fn build_safe_env() -> Environment<'static> {
    let mut env = Environment::new();
    env.set_undefined_behavior(UndefinedBehavior::Strict);
    env.set_auto_escape_callback(|_| AutoEscape::None);
    env.set_recursion_limit(64);
    env.set_fuel(Some(100_000));
    env
}
```
**HTML output requires autoescape ON** (so `{{ act.receiver_name }}` etc. don't get injected as raw markup) — either (a) add a second constructor `build_safe_html_env()` that swaps `set_auto_escape_callback(|_| AutoEscape::None)` for `set_auto_escape_callback(|name| if name.ends_with(".html") { AutoEscape::Html } else { AutoEscape::None })`, or (b) since `render_with_timeout` takes an `&Environment<'static>` built once outside the call (see `pipeline.pdf.minijinja_env` usage in `act_service.rs:1447`), thread a second `Arc<Environment<'static>>` / field through `PdfRenderer`/`AppCtx` for the HTML path. Keep `UndefinedBehavior::Strict`, `set_recursion_limit(64)`, `set_fuel(Some(100_000))`, and **no loader** unchanged — those safe-mode invariants apply equally to HTML templates (Req/D-02 constraints don't relax the sandbox just because the output format changed). `render_with_timeout` itself (lines 46-85) needs zero changes — it's already generic over template source and only maps errors to `AppError::Validation{field:"template",...}`.

### Dual-transport thin-adapter pattern
**Source:** `crates/trackly-app/src/tauri_cmds/acts.rs` (`build_acts_render_pdf` + `acts_render_pdf`) / `crates/trackly-app/src/http/acts.rs` (`handler_render_pdf`)
**Apply to:** All modified Tauri commands and axum handlers in this phase.
Both transports call the identical `build_acts_render_pdf` helper — the change from `Vec<u8>` to `String` propagates through exactly one shared function per operation; do not duplicate the type change independently in the Tauri wrapper and the axum handler.

### Portable path resolution (CLAUDE.md constraint)
**Source:** `crates/trackly-infra/src/paths.rs:38-49`
**Apply to:** `html_templates.rs` templates-dir resolution.
All new path logic MUST derive from `std::env::current_exe()?.parent()?` (mirrored via the already-injected `Paths`/`paths_arc` in `AppCtx::build`, not a fresh `current_exe()` call) — never `dirs::*`. `Paths` doesn't currently expose a `templates_dir()` accessor; either add one (`exe_dir.join("templates")`, same shape as `db_path()`/`logs_dir()` at `paths.rs:95-97/113-115`) or compute it locally from `paths.exe_dir()` in the new module — adding the accessor to `Paths` is the more consistent choice given every other portable path lives there.

### Env-override for dev/test (mirrors AD/SNMP mock gates)
**Source:** `crates/trackly-app/src/context.rs:270` (`TRACKLY_AD_MOCK`), `:306` (`TRACKLY_SNMP_MOCK`)
**Apply to:** `html_templates.rs` dev-override (D-07), name TBD by executor (`TRACKLY_TEMPLATES_DIR` suggested in CONTEXT.md).
```rust
let use_ad_mock = config.ad.use_mock || std::env::var("TRACKLY_AD_MOCK").is_ok();
```
Pattern: env var presence (`.is_ok()`), no value inspection needed for boolean mocks; for `TRACKLY_TEMPLATES_DIR` the value itself matters (it's a path override, not a boolean), so use `std::env::var("TRACKLY_TEMPLATES_DIR").map(PathBuf::from).ok()` and fall through to the `current_exe()`-derived default when absent/unset — same "env wins, else compute" control flow.

### Test seam: `Paths::resolve_for_exe_dir` + `TempDir`
**Source:** `crates/trackly-app/tests/pdf_render_act.rs:39-66` (`make_full_pipeline`)
**Apply to:** New HTML-generation tests (D-14 / Req 8).
```rust
let (writer, readers, dir) = test_writer_and_readers();
let paths = Arc::new(Paths::resolve_for_exe_dir(dir.path().to_path_buf()).expect("paths"));
...
templates.seed_defaults_on_startup().await.expect("seed");
```
New tests should follow the same `TempDir`-per-test isolation pattern: create a temp dir, either set `TRACKLY_TEMPLATES_DIR` to a subpath of it or pass `paths.exe_dir()` into the new templates-dir resolver, write/omit `templates/act_handover.html` inside it per test case (fallback vs file-present, D-14 item 3), then call the render path and assert on the returned HTML string (`assert!(html.contains(...))`, `assert!(!html.contains("http://"))` for D-14 item 4).

### krilla test hygiene (D-13) — do not touch these files' pass/fail behavior beyond `#[ignore]`
**Source:** `crates/trackly-app/tests/pdf_determinism.rs`, `pdf_render_act.rs`, `pdf_column_overflow.rs`, `pdf_text_extract.rs`, `pdf_logo.rs`, `pdf_logo_aspect.rs`, `templates_seed.rs`, `template_edit.rs`
Per D-13: fast unit tests (`pdf_logo`, `pdf_logo_aspect`) stay green as bit-rot guards; heavy/slow ones (`pdf_determinism`, full-render tests in `pdf_render_act.rs`) get `#[ignore]`. These tests call `renderer`/`PdfRenderer::render_docspec` directly and are **decoupled** from `act_service::render_pdf`'s new HTML path — once `render_pdf` stops calling `pipeline.pdf.render_docspec`, these test files' *own* internal pipeline construction (they build their own `PdfRenderer`/`DocSpec` fixtures, not through `ActService`) keeps working unmodified. Do not delete or "fix" krilla-path tests to match new act_service behavior — they test frozen code, not the active path.

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| New HTML-generation test module(s) (D-14 items 1-4: block-presence, 1-vs-N, fallback-vs-file, offline/no-CDN) | test | assertion/transform | No existing test asserts on raw HTML string content (existing PDF tests use `pdf-extract`/krilla-specific tooling — see `pdf_text_extract.rs`); closest structural analog is `templates_seed.rs`/`template_edit.rs` for the "seed + read-back" shape, but the assertions themselves (substring checks on HTML, `data:` URI presence, absence of `http(s)://`) are new logic with no direct precedent in this codebase. |

## Metadata

**Analog search scope:** `crates/trackly-app/src/pdf/`, `crates/trackly-app/src/services/act_service.rs`, `crates/trackly-app/src/services/template_service.rs`, `crates/trackly-app/src/services/org_db_service.rs`, `crates/trackly-app/src/services/organization_service.rs`, `crates/trackly-app/src/tauri_cmds/acts.rs`, `crates/trackly-app/src/http/acts.rs`, `crates/trackly-app/src/http/mod.rs`, `crates/trackly-infra/src/paths.rs`, `crates/trackly-app/src/context.rs`, `ui/src/features/acts/*.svelte`, `ui/src/lib/api/{acts,pdf}.ts`, `crates/trackly-app/templates/*.minijinja`, `crates/trackly-app/tests/{pdf_render_act,pdf_logo,paths_test}.rs`
**Files scanned:** ~20 (read in full or targeted sections; no file exceeded 2,000 lines requiring chunked Grep+Read)
**Pattern extraction date:** 2026-07-05
