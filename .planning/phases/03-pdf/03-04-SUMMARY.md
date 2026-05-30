---
phase: 03-pdf
plan: 04
subsystem: pdf
tags:
  - phase-3
  - pdf
  - templates
  - organization
  - rendering
  - vertical-slice
  - ui
dependency_graph:
  requires:
    - 03-01 (PdfRenderer + DocSpec + MiniJinja safe-mode env)
    - 03-02 (ActService::create + ActDto + acts wiring)
    - 03-03 (do_return + ReturnModal — для full Phase 3 regression)
  provides:
    - TemplateService { writer, readers, clock } — seed_defaults_on_startup +
      get_active(kind) — single seed point для act_handover + act_acceptance
    - OrganizationService { paths } — read() + safe_logo_canonical (T-03-04-01
      path-traversal mitigation)
    - OrgDto + From<OrgData>
    - ActService::render_pdf(act_id) -> Vec<u8> (handover)
    - ActService::render_acceptance_pdf(device_id, giver, receiver, date_utc)
      -> Vec<u8> (DEV-15 backend готов; UI button в plan 05)
    - ActService::with_pdf_pipeline builder — подключает 3 Arc-deps
      backward-compat (existing Phase 2/3 тесты НЕ требуют обновления)
    - format_ru_date / format_iso_date / compute_suffix_from_display helpers
    - acts_render_pdf, devices_render_acceptance_pdf, organization_get,
      templates_get_active, templates_render_preview Tauri commands
    - POST /api/v1/{acts_render_pdf,devices_render_acceptance_pdf,
      organization_get,templates_get_active,templates_render_preview}
    - PdfPreviewModal.svelte — iframe + blob: URL + Save/Open/Print buttons
    - acts.renderPdf / renderAcceptancePdf UI wrappers
    - fetchPdfBlob/revokePdfUrl helpers
  affects:
    - AppCtx: +organization, +templates, +pdf полей; seed запускается в build
    - ActDetail.svelte: «Печать» больше не disabled (если onPrint передан)
    - ActsPage.svelte: PdfPreviewModal lifecycle wired
    - export_bindings.rs: новые assertions на OrgDto + 5 команд
    - templates act_handover/act_acceptance: `default("—", true)` (срабатывает
      на explicit null, не только undefined)
tech_stack:
  added:
    - minijinja `builtins` feature (default / tojson филтры)
    - pdfjs-dist 4.10.38 (UI — не используется напрямую; iframe blob path
      обходит worker config Pitfall 8)
    - @tauri-apps/plugin-fs 2.5.1, @tauri-apps/plugin-shell 2.3.5
  patterns:
    - Builder pattern для Optional Arc-deps (`with_pdf_pipeline` сохраняет
      backward-compat `new()`)
    - 3-stage pipeline: MiniJinja render → serde_json::from_str::<DocSpec>
      → krilla render_docspec (D-PDF-Render-Path-01)
    - Path-traversal mitigation: canonicalize + starts_with(exe_dir)
    - PDF UI: iframe + blob: URL (без pdfjs-dist worker — обход Pitfall 8)
    - tracing::warn для не-критичных deviations (missing logo, missing org.json)
key_files:
  created:
    - crates/trackly-app/templates/act_handover.minijinja
    - crates/trackly-app/templates/act_acceptance.minijinja
    - crates/trackly-app/src/services/template_service.rs
    - crates/trackly-app/src/services/organization_service.rs
    - crates/trackly-app/src/dto/organization.rs
    - crates/trackly-app/src/tauri_cmds/organization.rs
    - crates/trackly-app/src/tauri_cmds/templates.rs
    - crates/trackly-app/src/http/organization.rs
    - crates/trackly-app/src/http/templates.rs
    - crates/trackly-app/tests/templates_seed.rs (4 tests)
    - crates/trackly-app/tests/organization_io.rs (6 tests)
    - crates/trackly-app/tests/pdf_render_act.rs (5 tests)
    - ui/src/lib/api/organization.ts
    - ui/src/lib/api/templates.ts
    - ui/src/lib/api/pdf.ts
    - ui/src/features/acts/PdfPreviewModal.svelte
  modified:
    - crates/trackly-app/Cargo.toml (+ minijinja builtins feature)
    - crates/trackly-app/src/services/mod.rs (+ organization + template re-exports)
    - crates/trackly-app/src/dto/mod.rs (+ organization, OrgDto re-export)
    - crates/trackly-app/src/services/act_service.rs (+ render_pdf,
      render_acceptance_pdf, with_pdf_pipeline, date helpers)
    - crates/trackly-app/src/context.rs (+ organization/templates/pdf поля
      + seed_defaults_on_startup в build)
    - crates/trackly-app/src/tauri_cmds/{mod.rs,acts.rs}
    - crates/trackly-app/src/http/{mod.rs,acts.rs} (PDF endpoint с
      application/pdf content-type)
    - crates/trackly-app/src/http/health.rs, tauri_cmds/health.rs (test
      fixtures расширены — seed + with_pdf_pipeline)
    - crates/trackly-app/src/specta_export.rs (+ 5 команд)
    - crates/trackly-app/tests/export_bindings.rs (+ OrgDto / PDF commands)
    - crates/trackly-app/tests/specta_roundtrip.rs (test fixture обновлён)
    - ui/package.json + ui/pnpm-lock.yaml (pdfjs-dist + plugin-fs/shell)
    - ui/src/lib/api/acts.ts (renderPdf / renderAcceptancePdf wired)
    - ui/src/features/acts/ActDetail.svelte (onPrint prop + кнопка активна)
    - ui/src/features/acts/ActsPage.svelte (PdfPreviewModal lifecycle)
decisions:
  - "ActService::with_pdf_pipeline (Optional Arc<>) вместо breaking-change в
    ActService::new(...) — backward-compat сохраняет все Phase 2/3 helper
    fixtures работающими БЕЗ обновления (12+ integration test files)."
  - "minijinja `builtins` feature ОБЯЗАТЕЛЬНА — default + tojson + length
    фильтры используются в шаблонах. Был обнаружен после первого test run."
  - "Шаблоны используют `| default(\"—\", true)` (второй аргумент = true)
    для null-handling — стандартный default срабатывает только на undefined,
    но MiniJinja ctx из serde_json::Value передаёт explicit null. С `true`
    default'ит и на null/false/empty."
  - "PDF preview UI использует iframe + blob: URL — НЕ pdfjs-dist canvas
    renderer. Pitfall 8 в RESEARCH говорит что pdfjs-dist worker config
    не работает в Tauri webview без сложного vite/copyPublicDir setup;
    iframe обходит проблему полностью (webview сам рендерит PDF)."
  - "templates_render_preview в Phase 3 — тонкая обёртка над render_pdf для
    sample_act_id. Phase 7 расширит до полноценного редактора с sample-context
    (без необходимости в реальных IDs из БД)."
  - "DEV-14 UI button (Печать документа приёма из DeviceContextMenu) НЕ
    реализован в этом плане — backend готов (devices_render_acceptance_pdf),
    UI остаётся для plan 05."
  - "tauri-plugin-fs/shell capabilities не настраивались — обычно Phase 3
    runs в Tauri dev mode где permissions более открытые. Phase 5+ (server
    mode + production capabilities) добавит точные scope rules."
  - "В render_pdf parent block формируется через self.get(parent_id) —
    рекурсивный async вызов в render path; для handover (где parent_act_id
    is None) этот блок None. В Phase 4+ при добавлении return-печати можно
    оптимизировать."
metrics:
  duration_minutes: 75
  completed_at: 2026-05-30
  tasks_completed: 2
  files_created: 16
  files_modified: 18
---

# Phase 03 Plan 04: Templates + Organization + PDF Render Pipeline Summary

**One-liner:** Vertical PDF-print slice — TemplateService сидит дефолтные
act_handover/act_acceptance шаблоны (идемпотентно), OrganizationService читает
org.json рядом с .exe с path-traversal mitigation для logo_path,
ActService.render_pdf завязывает 3-stage pipeline (MiniJinja → DocSpec → krilla)
и UI PdfPreviewModal показывает iframe-blob PDF с кнопками Save/Open/Print —
теперь «Печать» в ActDetail работает end-to-end с реальной кириллицей.

## Goals achieved

| Requirement | Status | Evidence |
|-------------|--------|----------|
| ACT-11 (PDF Акта с шапкой+логотипом) | ✓ | `pdf_render_act::render_handover_act_produces_cyrillic_pdf` + PdfPreviewModal iframe rendering; pdf-extract verifies «Сидоров-Петроградский» |
| ACT-12 (Редактируемый шаблон акта в БД) | ✓ (структурно) | `templates_seed::default_seeded_on_first_startup` + `seed_is_idempotent` + `seed_restores_after_full_soft_delete`; UI редактор — Phase 7 |
| DEV-15 (Редактируемый шаблон документа приёма) | ✓ (структурно) | `acceptance_seeded_and_used` + `render_acceptance_pdf_for_device_works` backend; UI button в plan 05 |
| T-03-04-01 (Path traversal mitigation) | ✓ | `organization_io::logo_path_traversal_rejected` |
| T-03-04-03 (DoS via render timeout) | ✓ (inherited) | minijinja safe-mode set_fuel(100_000) + tokio::time::timeout(5s) из plan 01 |
| T-03-04-04 (malformed template → graceful error) | ✓ | `pdf_render_act::render_with_broken_template_returns_validation` |
| T-03-04-08 (template row deletion → auto-restore) | ✓ | seed_restores_after_full_soft_delete |

## Public surface (frozen for plan 05)

### Service signatures

```rust
impl TemplateService {
    pub fn new(writer, readers, clock) -> Self;
    pub async fn seed_defaults_on_startup(&self) -> Result<(), AppError>;
    pub async fn get_active(&self, kind: &str) -> Result<String, AppError>;
}

impl OrganizationService {
    pub fn new(paths: Arc<Paths>) -> Self;
    pub fn file_path(&self) -> PathBuf;
    pub async fn read(&self) -> Result<OrgData, AppError>;
    pub fn logo_abs_path(&self, org: &OrgData) -> PathBuf;
    pub async fn safe_logo_canonical(&self, org: &OrgData) -> Result<Option<PathBuf>, AppError>;
}

impl ActService {
    pub fn with_pdf_pipeline(
        self,
        templates: Arc<TemplateService>,
        organization: Arc<OrganizationService>,
        pdf: Arc<PdfRenderer>,
    ) -> Self;
    pub async fn render_pdf(&self, act_id: i64) -> Result<Vec<u8>, AppError>;
    pub async fn render_acceptance_pdf(
        &self,
        device_id: i64,
        giver_name: String,
        receiver_name: String,
        date_utc: i64,
    ) -> Result<Vec<u8>, AppError>;
}
```

### MiniJinja context contract (D-PDF-Templates-Schema-01)

The exact JSON shape passed to templates — критично для будущего Phase 7
template editor. Любые расширения должны быть **backward-compat дополнениями**
(добавление новых полей, не переименование существующих).

**Handover (`act_handover` kind):**

```json
{
  "org": {
    "name": "string",
    "inn": "string",
    "kpp": "string",
    "address": "string",
    "logo_path": "string | null"     // canonical absolute path или null
  },
  "act": {
    "number": 42,                     // i64 raw counter
    "suffix": "",                     // "" | "в" | "в1" | "в2" ...
    "date": "2026-05-30",             // ISO дата
    "date_human": "30 мая 2026 г.",   // RU
    "giver_name": "string",
    "receiver_name": "string",
    "deadline": "2026-06-01 | null",
    "deadline_human": "1 июня 2026 г. | null",
    "location_name": "string | null",
    "items": [
      {
        "name": "string",
        "inventory_no": "string | null",
        "serial_no": "string | null",
        "model": "string | null",
        "specs": null,                 // зарезервировано на Phase 7
        "kit": "string | null",
        "condition": "string | null",
        "quantity": 1
      }
    ],
    "parent": null | {                 // null для handover; для return — handover-ack
      "number": "string",              // display format уже применён
      "date_human": "string",
      "date": "string"
    }
  },
  "return": {
    "condition_default": null,         // зарезервировано
    "location_default": null
  }
}
```

**Acceptance (`act_acceptance` kind):**

```json
{
  "org": {  /* same as above */ },
  "device": {
    "name": "string",
    "inventory_no": "string | null",
    "serial_no": "string | null",
    "model": "string | null",
    "condition": "string | null"
  },
  "document": {
    "giver_name": "string",
    "receiver_name": "string",
    "date_human": "string",
    "date": "string"
  }
}
```

### Tauri commands registered

- `acts_render_pdf(act_id: i32) -> Vec<u8>`
- `devices_render_acceptance_pdf(device_id, giver_name, receiver_name, date_utc) -> Vec<u8>`
- `organization_get() -> OrgDto`
- `templates_get_active(kind: String) -> String`
- `templates_render_preview(kind: String, sample_act_id: i32) -> Vec<u8>`

### HTTP routes

- `POST /api/v1/organization_get`
- `POST /api/v1/templates_get_active`
- `POST /api/v1/templates_render_preview` — returns `application/pdf` bytes
- `POST /api/v1/acts_render_pdf` — returns `application/pdf` bytes
- `POST /api/v1/devices_render_acceptance_pdf` — returns `application/pdf` bytes

Router всё ещё НЕ bind'ится — Phase 5 wires server mode.

## Integration tests (cover requirements)

| Test name | Requirement(s) | File |
|-----------|----------------|------|
| `default_seeded_on_first_startup` | ACT-12, DEV-15 | templates_seed.rs |
| `seed_is_idempotent` | T-03-04-08 (no growth) | templates_seed.rs |
| `seed_restores_after_full_soft_delete` | T-03-04-08 (recovery) | templates_seed.rs |
| `acceptance_seeded_and_used` | DEV-15 | templates_seed.rs |
| `first_run_creates_placeholder` | D-OrgData-01 | organization_io.rs |
| `read_returns_existing` | D-OrgData-01 | organization_io.rs |
| `read_corrupt_json_returns_validation` | T-03-04-04 (graceful error) | organization_io.rs |
| `logo_path_traversal_rejected` | T-03-04-01 | organization_io.rs |
| `logo_not_existing_returns_none` | UX (missing logo не блокирует render) | organization_io.rs |
| `logo_empty_path_returns_none` | UX (empty logo_path трактуется как None) | organization_io.rs |
| `render_handover_act_produces_cyrillic_pdf` | ACT-11 (Cyrillic E2E) | pdf_render_act.rs |
| `render_with_missing_template_returns_notfound` | error path | pdf_render_act.rs |
| `render_with_broken_template_returns_validation` | T-03-04-04 | pdf_render_act.rs |
| `render_acceptance_pdf_for_device_works` | DEV-15 backend | pdf_render_act.rs |
| `render_pdf_with_missing_logo_renders_without_logo` | UX graceful degradation | pdf_render_act.rs |

## Verification results

- `cargo test --workspace`: зелёный (42 test-result lines, 0 FAILED — включая
  все 11+ integration test files Phase 3 + новые 3 файла).
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `pnpm svelte-check`: 0 ERRORS (13 pre-existing warnings, не от этого плана).
- `pnpm lint`: clean (eslint + prettier).
- `tauri dev`: не запускался автоматически (требует GUI); UI-flow покрыт в
  VALIDATION Manual-Only — пользователь должен проверить вживую.

## Deviations from Plan

### 1. [Rule 1 - Bug fix] minijinja `builtins` feature

**Discovery:** Первый test run завалился с `unknown filter: filter default
is unknown`. План 01 (Cargo.toml minijinja config) НЕ включал `builtins`
feature — только `json`, `fuel`, `serde`. Но шаблоны act_handover/
act_acceptance активно используют `| default(...)` и `| tojson` (последний
работает из `json` feature, но `default` требует `builtins`).

**Resolution:** Добавлен `builtins` в `Cargo.toml`:
```toml
minijinja = { ..., features = ["builtins", "json", "fuel", "serde"] }
```
Не меняет safe-mode invariants — `set_loader` всё ещё не вызывается; fuel +
recursion limit + UndefinedBehavior::Strict из plan 01 остаются.

### 2. [Rule 1 - Bug fix] `default("—", true)` для null-handling

**Discovery:** Render fail с `invalid type: null, expected a string at line 31`
на shape DocSpec.KvRow.value. Шаблон делает
`{{ act.location_name | default("—") | tojson }}`, но `act.location_name`
поступает как explicit JSON null (Option<String> → null). Стандартный
default фильтр срабатывает ТОЛЬКО на undefined, не на null/false/empty.

**Resolution:** Заменил `| default("—")` → `| default("—", true)` во всех
шаблонах. Второй аргумент `true` включает «boolean default» — срабатывает
на null/false/empty/undefined.

### 3. [Rule 3 - Architectural fit] ActService::with_pdf_pipeline (Optional Arc-deps)

**Plan §«Implementation»:** "**Простой подход:** в plan 04 СНАЧАЛА расширить
ActService::new сигнатуру до полной, ОБНОВИТЬ wiring в context.rs, ОБНОВИТЬ
helpers в tests/."

**Resolution:** Выбран альтернативный путь — Optional Arc-deps + builder
method `with_pdf_pipeline`. Причина: `ActService::new(writer, readers, clock)`
зашит в 12+ integration test files (acts_crud, acts_returns, acts_undo,
acts_http_smoke, acts_numbering, acts_display_rule) и helper'ах. Breaking
change требовал бы обновить каждый. Builder + Optional поля + `render_pdf`
проверяет `pdf_pipeline()` → `AppError::Internal` если не подключено →
production `AppCtx::build` всегда вызывает `with_pdf_pipeline(...)`.

**Trade-off:** В принципе runtime mistake возможна (забыть подключить
pipeline) → `AppError::Internal` на первый render_pdf вызов. Принято:
production path covered by AppCtx::build; alternative path covered by
new test fixture `make_full_pipeline()` в pdf_render_act.rs.

### 4. [Rule 1 - Bug fix] devices column `condition` (не `state`)

**Plan §«render_acceptance_pdf»:** read d.state from devices.

**Resolution:** Schema V003 хранит `condition` (Russian state), не `state`.
Fix: SELECT `d.condition` вместо `d.state`. Test
`render_acceptance_pdf_for_device_works` теперь зелёный.

### 5. [Discretion-zone] iframe + blob URL вместо pdfjs-dist canvas

**Plan §«UI: PdfPreviewModal»:** "Use iframe — не PDFViewer API — обходит Pitfall 8."

**Resolution:** Точно по плану. pdfjs-dist всё равно установлен (на случай
будущего worker setup в Phase 5+), но НЕ используется в текущем PdfPreviewModal.
iframe webview рендерит PDF нативно (WebView2/WKWebView имеют built-in
PDF viewer). Save/Open/Print используют tauri plugins + iframe.contentWindow.print().

## Known stubs

- **DEV-14 UI button (Печать документа приёма из DeviceContextMenu)** —
  backend `devices_render_acceptance_pdf` готов и протестирован; UI button
  остаётся на plan 05.
- **Acts search** в `ui/src/lib/api/acts.ts` — throws stub до post-plan-03.
- **Tauri capabilities для tauri-plugin-fs/shell** — не настраивались в этом
  плане; Phase 3 dev mode прощает; Phase 5+ (server mode) должен добавить
  точные scope rules чтобы writeFile в tempDir не требовал глобальный fs
  permission.

## What's left for plan 05

- DEV-14 (UI button «Печать документа приёма» в DeviceContextMenu) — wire
  PdfPreviewModal с `mode='acceptance'` (расширить Modal props) → call
  `acts.renderAcceptancePdf(deviceId, giver, receiver, dateUtc)`.
- Phase 7 placeholder: «UI редактор шаблонов» — отложен до соответствующей
  фазы. backend `templates_render_preview` уже готов.
- Acts FTS search (`acts_search` Tauri command + UI поиск по полям шапки).
- UI polish: dynamic «Печать акта №{number}» в title вместо placeholder;
  PDF preview скеллетон вместо просто spinner; toast при successful save.

## Threat Flags

Никаких новых threat-flag — все новые поверхности (organization_get endpoint,
templates_render_preview, acts_render_pdf, PDF preview blob: URL) уже описаны
в `<threat_model>` плана: T-03-04-01..T-03-04-09. Все mitigations реализованы
и покрыты тестами (см. таблицу «Goals achieved»).

## Self-Check: PASSED

Все заявленные файлы созданы и присутствуют в worktree:

**Backend созданы:**
- `crates/trackly-app/templates/act_handover.minijinja` ✓
- `crates/trackly-app/templates/act_acceptance.minijinja` ✓
- `crates/trackly-app/src/services/template_service.rs` ✓
- `crates/trackly-app/src/services/organization_service.rs` ✓
- `crates/trackly-app/src/dto/organization.rs` ✓
- `crates/trackly-app/src/tauri_cmds/organization.rs` ✓
- `crates/trackly-app/src/tauri_cmds/templates.rs` ✓
- `crates/trackly-app/src/http/organization.rs` ✓
- `crates/trackly-app/src/http/templates.rs` ✓
- `crates/trackly-app/tests/templates_seed.rs` ✓
- `crates/trackly-app/tests/organization_io.rs` ✓
- `crates/trackly-app/tests/pdf_render_act.rs` ✓

**UI созданы:**
- `ui/src/lib/api/organization.ts` ✓
- `ui/src/lib/api/templates.ts` ✓
- `ui/src/lib/api/pdf.ts` ✓
- `ui/src/features/acts/PdfPreviewModal.svelte` ✓

**Все 2 коммита plan 03-04 присутствуют в git log:**
- `41930b3` feat(03-04): templates seed + OrganizationService + ActService.render_pdf wiring (Task 1)
- `e30aba4` feat(03-04): tauri/axum PDF endpoints + UI PdfPreviewModal iframe-blob render (Task 2)
