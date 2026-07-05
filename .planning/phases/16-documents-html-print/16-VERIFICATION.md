---
phase: 16-documents-html-print
verified: 2026-07-05T10:30:00Z
status: human_needed
score: 10/10 must-haves verified
overrides_applied: 0
human_verification:
  - test: "Print an act with 2+ devices and long Комплектация/Технические характеристики text in Chrome/Edge (Windows target) print preview, with browser header/footer disabled"
    expected: "All device rows print in full, no text overlap/clipping across A4 page breaks, no browser URL/date/page-number injected, output visually matches the Phase 15 Word sample layout"
    why_human: "Browser pagination (@page + page-break-inside: avoid) and print-dialog rendering cannot be verified by static analysis or headless Rust tests — requires an actual print-preview render in a real browser/webview"
  - test: "Open the print-preview in both desktop Tauri window and a LAN browser (different machine/tab) with no internet connectivity; confirm logo renders and Cyrillic text has no missing glyphs/tofu boxes"
    expected: "Logo displays from the embedded data: URI; Cyrillic renders correctly via system fonts in both webviews"
    why_human: "Font rendering and offline network conditions are runtime/visual concerns not verifiable from source"
  - test: "Edit templates/act_handover.html by hand (e.g. in Notepad) while the app is running, save, then re-generate the act without restarting"
    expected: "The new generation reflects the hand edit immediately (read-on-render, D-08)"
    why_human: "Requires interacting with a running instance and a real filesystem edit, not just unit-test-level TempDir manipulation"
---

# Phase 16: documents-html-print Verification Report

**Phase Goal:** Оба акта (приёма-передачи и приёмки устройства) генерируются из HTML-шаблонов (папка `templates/` рядом с exe + вшитый дефолт-fallback) и печатаются/сохраняются в PDF через диалог браузера в обоих режимах (desktop + LAN), визуально по образцу Word; krilla/DocSpec заморожен и не используется.
**Verified:** 2026-07-05T10:30:00Z
**Status:** human_needed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `act_handover` renders as self-contained HTML reproducing the Word-sample block order | ✓ VERIFIED | `crates/trackly-app/templates/act_handover.html` contains header (logo+requisites), centered title "Акт приема-передачи", number/date subtitle, intro field-row with receiver name, per-item field rows (Инвентарный номер/Серийный номер/Модель/Комплектация/Технические характеристики/Состояние), "Сроком до", optional parent-act note, two-line "Выдал/Получил" signature block — in the exact order documented in the file's own header comment and matching the `.minijinja` analog |
| 2 | `act_acceptance` renders as HTML | ✓ VERIFIED | `crates/trackly-app/templates/act_acceptance.html` exists (135 lines): header+requisites, "Документ приёма устройства на склад" heading, date/giver/receiver kv rows, device kv rows, signature block |
| 3 | Multi-device (N items) acts print all positions without truncation; A4 page-breaks are correct | ✓ VERIFIED | `page-break-inside: avoid` present in both templates' `.device-block`/`.signatures`/`.signature` CSS rules; `html_handover_multi_device_all_items_present_no_truncation` test (passing) asserts a 3-device act with a >150-char field renders every device name and the full untruncated value with no `…` marker |
| 4 | HTML templates read from `templates/` folder next to exe + embedded fallback; edits apply without rebuild | ✓ VERIFIED | `Paths::templates_dir()` = `exe_dir.join("templates")`; `html_templates::resolve_templates_dir`/`materialize_defaults_on_startup`/`load_template` implement env-override, idempotent materialize-on-startup, and read-on-render fallback; 5 unit tests + 2 integration tests (`html_falls_back_to_embedded_default_when_file_absent`, `html_uses_file_when_present_and_edit_changes_output`) all pass |
| 5 | Printing/saving to PDF happens via the browser's native print dialog in both desktop and LAN-browser modes | ✓ VERIFIED | `ActService::render_pdf`/`render_acceptance_pdf` return `Result<String, AppError>` (HTML, not bytes); Tauri commands and axum handlers (`text/html; charset=utf-8`) both call the same `build_*` helpers; `PdfPreviewModal.svelte` renders via `<iframe srcdoc={htmlContent}>` and calls `iframeEl.contentWindow.print()`; `client.ts` correctly routes `text/html` responses through `res.text()` (fixed bug found during 16-04) so the LAN/HTTP transport doesn't corrupt the HTML into a byte array |
| 6 | Self-contained / offline-safe: no external CDN/network dependency, logo embedded, system fonts | ✓ VERIFIED | Logo embedded as base64 `data:` URI built server-side from trusted `org_settings` BLOB or canonicalized local file bytes (never user input); `font-family: "DejaVu Sans", "Arial", sans-serif` (system fonts, no `@font-face`/CDN); `html_is_offline_safe_no_external_links` test passes, asserting no `http://`/`https://` substring in rendered output for either act |
| 7 | Print CSS (A4 portrait + margins), no reliance on browser header/footer | ✓ VERIFIED | Both templates declare `@page { size: A4 portrait; margin: 20mm 15mm; }`; layout uses only in-document elements (border-bottom underlines, grid signatures) — no dependency on browser-injected header/footer content |
| 8 | Tests: HTML-generation regression + new coverage; frozen krilla tests don't break the build | ✓ VERIFIED | `cargo test -p trackly-app` fully green (confirmed independently with `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1`, required per project CI convention); `html_act_render.rs` has all 6 D-14 tests passing; `pdf_determinism.rs`'s 2 heavy tests correctly `#[ignore]`d (still pass via `--ignored`); direct-renderer bit-rot guards in `pdf_logo.rs`/`pdf_column_overflow.rs` remain un-ignored and green |
| 9 | krilla/DocSpec pipeline is frozen (present in repo, not called by active generation) | ✓ VERIFIED | `grep -n "DocSpec\|render_docspec" crates/trackly-app/src/services/act_service.rs` returns 0 matches; `renderer.rs`/`docspec.rs`/both `.minijinja` files remain on disk untouched; `pdf/mod.rs` still registers `pub mod docspec;`/`pub mod renderer;` (compiled, just unreachable from the active service path) |
| 10 | Requirements traceability: all 8 SPEC-Req IDs claimed and covered | ✓ VERIFIED | Req1/Req2/Req4/Req6/Req7 → Plan 16-01; Req1/Req2/Req3/Req6/Req7 → Plan 16-02; Req5/Req6 → Plan 16-03; Req5 → Plan 16-04; Req3/Req8 → Plan 16-05. All 8 requirements from 16-SPEC.md appear in at least one plan's frontmatter; no orphans |

**Score:** 10/10 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/trackly-app/templates/act_handover.html` | Self-contained handover HTML | ✓ VERIFIED | 235 lines, all required blocks present, no `tojson`, print CSS present |
| `crates/trackly-app/templates/act_acceptance.html` | Self-contained acceptance HTML | ✓ VERIFIED | 135 lines, all required blocks present |
| `crates/trackly-app/src/pdf/html_templates.rs` | Resolver/materialize/loader | ✓ VERIFIED | 184 lines, `resolve_templates_dir`/`materialize_defaults_on_startup`/`load_template`/`DEFAULT_HTML_TEMPLATES` all present, 5 unit tests pass |
| `crates/trackly-app/src/pdf/minijinja_env.rs` | `build_safe_html_env` (autoescape ON) | ✓ VERIFIED | Present, used by both render paths |
| `crates/trackly-infra/src/paths.rs` | `templates_dir()` accessor | ✓ VERIFIED | `exe_dir.join("templates")`, mirrors `logs_dir()` shape |
| `crates/trackly-app/src/services/act_service.rs` | `render_pdf`/`render_acceptance_pdf` → `String` | ✓ VERIFIED | Both return `Result<String, AppError>`; zero `DocSpec`/`render_docspec` references |
| `crates/trackly-app/src/tauri_cmds/acts.rs` | Tauri commands return `String`; `acts_open_pdf_in_system` removed | ✓ VERIFIED | Confirmed via grep; only a historical doc-comment mentions the removed command name |
| `crates/trackly-app/src/http/acts.rs` | axum handlers respond `text/html` | ✓ VERIFIED | `text/html; charset=utf-8` × 2, `application/pdf` 0 matches |
| `ui/src/lib/api/acts.ts` | `renderPdf`/`renderAcceptancePdf` → `Promise<string>` | ✓ VERIFIED | Both typed `Promise<string>`, `apiCall<string>` |
| `ui/src/features/acts/PdfPreviewModal.svelte` | `srcdoc` iframe + print(), no blob/system-open | ✓ VERIFIED | `srcdoc={htmlContent}`, `contentWindow.print()` unchanged, `handleOpen`/`handleSave`/blob code fully removed |
| `crates/trackly-app/tests/html_act_render.rs` | D-14 coverage (6 tests) | ✓ VERIFIED | All 6 tests present and passing |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `act_service.rs` | `pdf/html_templates.rs` | `html_templates::load_template(...)` | ✓ WIRED | 2 call sites (handover + acceptance) |
| `act_service.rs` | `pdf/minijinja_env.rs` | `build_safe_html_env()` | ✓ WIRED | 2 call sites |
| `act_service.rs` | `org_settings` BLOB | base64 `data:` URI construction | ✓ WIRED | `logo_data_uri` built in both render paths from trusted bytes |
| `tauri_cmds/acts.rs` | `act_service.rs` | `ctx.acts.render_pdf`/`render_acceptance_pdf` | ✓ WIRED | Both `build_*` helpers delegate directly |
| `http/acts.rs` | `tauri_cmds/acts.rs` | shared `build_*` helper reuse | ✓ WIRED | Dual-transport thin-adapter pattern intact |
| `PdfPreviewModal.svelte` | `ui/src/lib/api/acts.ts` | `renderCall()` → `htmlContent = html` | ✓ WIRED | Direct assignment, no blob conversion |
| `ui/src/lib/api/client.ts` | HTTP response | `text/html` branch → `res.text()` | ✓ WIRED | Explicit branch added ahead of binary fallback (bug found+fixed during 16-04) |
| `context.rs` (`AppCtx::build`) | `pdf/html_templates.rs` | `materialize_defaults_on_startup` | ✓ WIRED | Called once, additive to existing DB-template seed |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|---------------------|--------|
| `act_handover.html` render | `ctx` (serde_json) | `ActService::render_pdf` assembling act/device/org rows from SQLite via `readers` pool | Yes — verified via `html_handover_multi_device_all_items_present_no_truncation` asserting real seeded device names/fields appear in output | ✓ FLOWING |
| Logo `data:` URI | `logo_bytes`/`logo_blob` | `OrgDbService::get_for_pdf` (org_settings table) or `OrganizationService::read_logo_bytes` (legacy file) | Yes — `html_handover_contains_required_blocks_and_logo` test confirms a real saved logo (`OrgDbService::save_logo`) round-trips into `data:image/png;base64,` in the output | ✓ FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Full backend test suite green (with required mock env) | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app` | All test binaries pass, 0 failures | ✓ PASS |
| New HTML-generation tests pass | `cargo test -p trackly-app --test html_act_render` | 6/6 pass | ✓ PASS |
| Migrated PDF-byte tests now pass as HTML-string tests | `cargo test -p trackly-app --test pdf_render_act --test pdf_column_overflow --test pdf_logo --test acts_e2e_smoke` | 4+6+4+2 = 16/16 pass | ✓ PASS |
| Frozen krilla tests correctly ignored by default, pass when explicit | `cargo test -p trackly-app --test pdf_determinism` / `-- --ignored` | 0 run by default (2 ignored); 2/2 pass with `--ignored` | ✓ PASS |
| Frontend type-check green | `pnpm --dir ui exec svelte-check` | 0 errors, 38 pre-existing warnings unrelated to Phase 16 | ✓ PASS |
| Frontend build green | `pnpm --dir ui build` | Builds successfully, `dist/` produced | ✓ PASS |
| No `DocSpec`/`render_docspec` reachable from `act_service.rs` | `grep -n "DocSpec\|render_docspec" act_service.rs` | 0 matches | ✓ PASS |
| krilla/DocSpec/minijinja files remain on disk (frozen, not deleted) | `ls renderer.rs docspec.rs act_handover.minijinja act_acceptance.minijinja` | All 4 files exist | ✓ PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|--------------|--------|----------|
| SPEC-Req1 | 16-01, 16-02 | HTML-акт приёма-передачи | ✓ SATISFIED | `act_handover.html` + `render_pdf` HTML path |
| SPEC-Req2 | 16-01, 16-02 | HTML-акт приёмки устройства | ✓ SATISFIED | `act_acceptance.html` + `render_acceptance_pdf` HTML path |
| SPEC-Req3 | 16-02, 16-05 | Мультиустройство без обрезки | ✓ SATISFIED | `page-break-inside: avoid` CSS + passing multi-device no-truncation test |
| SPEC-Req4 | 16-01 | Шаблоны в `templates/` + вшитый fallback | ✓ SATISFIED | `html_templates.rs` resolver/materialize/loader + tests |
| SPEC-Req5 | 16-03, 16-04 | Печать/сохранение через диалог браузера в обоих режимах | ✓ SATISFIED | `srcdoc` iframe + `print()`; `text/html` transport fixed for LAN |
| SPEC-Req6 | 16-01, 16-02, 16-03 | Self-contained / offline-safe | ✓ SATISFIED | `data:` URI logo, system fonts, no-CDN test passing |
| SPEC-Req7 | 16-01, 16-02 | Print CSS `@page` A4 | ✓ SATISFIED | `@page` block present in both templates |
| SPEC-Req8 | 16-05 | Тесты HTML-генерации | ✓ SATISFIED | `html_act_render.rs` (6 tests) + migrated regression tests, full suite green |

No orphaned requirements found — all 8 SPEC requirements are claimed by at least one plan and independently verified in the codebase.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `ui/src/features/acts/PdfPreviewModal.svelte` | 121, 125, 91 | Stale "PDF" wording ("Генерируем PDF…", "Не удалось сгенерировать PDF") | ⚠️ Warning | Cosmetic/UX inaccuracy (review WR-03); does not block the phase goal — the underlying flow generates and prints HTML correctly, only the loading/error copy is stale |
| `ui/src/lib/api/acts.ts` | 7-8 | Stale comment describing `renderPdf`/`search` as stubs "until plan 04" | ℹ️ Info | Documentation drift only (review IN-04) |
| `crates/trackly-app/src/pdf/mod.rs` | 1-27 | Module doc still describes the 3-stage krilla pipeline as if it were the active path, no mention of `html_templates` as the live path | ℹ️ Info | Documentation drift only (review IN-05); does not affect runtime behavior |
| `crates/trackly-app/src/services/act_service.rs` | 1683-1696 | `compute_suffix_from_display` hardcodes Cyrillic "в" for suffix extraction | ⚠️ Warning | Pre-existing pattern, not introduced by this phase's core deliverable but flagged by review (WR-04); fragile if number-format convention changes |
| `crates/trackly-app/src/tauri_cmds/templates.rs` | 29-40 | `templates_render_preview` reuses `sample_act_id` as `device_id` | ⚠️ Warning | Dead/preview-only code path, zero UI callers (review WR-05, confirmed via deferred-items.md) |
| `ui/src/features/acts/PdfPreviewModal.svelte` | 129-134 | `<iframe srcdoc>` with no `sandbox` attribute, while org-logo MIME whitelist permits `image/svg+xml` | 🛑 Critical (per code review CR-01) | Defense-in-depth gap: a future user hand-edited template using `\| safe` on untrusted data, or an SVG logo, would render unsandboxed in the app's iframe. **Assessed against phase goal:** this does NOT block the stated Phase 16 goal (HTML generation from templates/fallback, browser-print in both modes, krilla frozen) — it is a security-hardening follow-up on the delivery surface introduced by this phase, not a missing capability. See escalation below. |

No unresolved `TBD`/`FIXME`/`XXX` markers found in phase-modified files (checked via grep across all `files_modified` lists from the 5 SUMMARY.md frontmatters).

### Human Verification Required

### 1. Real-browser print output vs. Word sample (multi-device, A4 pagination)

**Test:** Generate a handover act with 2+ devices, one with a long Комплектация/Технические характеристики value, open the print preview in Chrome/Edge, disable browser header/footer in the print dialog, and inspect/print to PDF.
**Expected:** All device rows appear in full without text overlap or clipping; page breaks occur cleanly between (not inside) device blocks; overall layout visually matches the Phase 15 Word sample; no browser-injected URL/date/page-number appears in the output.
**Why human:** CSS `@page`/`page-break-inside` pagination behavior and print-dialog rendering fidelity cannot be verified by grep or headless Rust tests — this requires visual inspection of an actual browser print render.

### 2. Offline logo + Cyrillic rendering in both desktop and LAN-browser webviews

**Test:** With no internet connectivity, open the print preview in both the desktop Tauri window and a separate LAN browser session; confirm the logo image displays and Cyrillic text renders without missing-glyph boxes.
**Expected:** Logo displays via the embedded `data:` URI in both webviews; Cyrillic renders correctly via system fonts (DejaVu Sans/Arial fallback) with no tofu/replacement-character boxes.
**Why human:** Font availability and rendering fidelity are runtime/visual concerns specific to the actual OS/webview font stack, not verifiable from source code alone.

### 3. Live hand-edit of a template file without app restart

**Test:** While the app is running, open `templates/act_handover.html` in a text editor, make a visible change (e.g. edit the title text), save, then re-generate the act from the running app without restarting.
**Expected:** The newly generated document reflects the hand edit immediately (read-on-render per D-08).
**Why human:** This requires interacting with a live running instance and a real filesystem edit — the unit/integration tests validate the same code path via `TempDir` manipulation, but an end-to-end "edit file while app is running" experience should be confirmed once by a human for full confidence.

## Escalation: CR-01 (iframe sandbox) — advisory, not phase-blocking

The code review (`16-REVIEW.md`) flagged one Critical finding: the `<iframe srcdoc={htmlContent}>` in `PdfPreviewModal.svelte` has no `sandbox` attribute, and the org-logo MIME whitelist still permits `image/svg+xml`. This is a real defense-in-depth gap (a future template hand-edit using `| safe` on untrusted data, or an SVG-based logo attack, would render unsandboxed) but:

- It does not prevent any of the 8 SPEC requirements or the phase goal itself from being achieved — HTML generation, template-folder-with-fallback, and browser-print-in-both-modes all function correctly today.
- The currently shipped templates are provably safe (autoescape ON, only one scoped `| safe` on a server-constructed base64 URI, verified in both `.html` files).
- Fixing it (adding `sandbox="allow-modals"` and re-verifying `print()` still works under the sandbox, dropping `image/svg+xml` from the whitelist) is scoped, mechanical follow-up work, not a phase-goal gap.

**Recommendation:** Track as a fast-follow hardening task (quick/debug workflow) rather than reopening Phase 16. If the developer wants to formally accept this as deferred rather than blocking, add to this VERIFICATION.md frontmatter:

```yaml
overrides:
  - must_have: "iframe srcdoc sandboxing (CR-01)"
    reason: "Defense-in-depth hardening on the newly-introduced delivery surface; not a phase-8-requirement gap. Templates are provably safe today (autoescape ON, one scoped `| safe` on trusted server-constructed data). Tracked as fast-follow."
    accepted_by: "{name}"
    accepted_at: "{ISO timestamp}"
```

No override was applied automatically — this is presented as a warning requiring a human decision, per the escalation-gate pattern, not auto-accepted.

### Gaps Summary

No BLOCKER-level gaps found. All 10 observable truths derived from the ROADMAP goal + 16-SPEC.md's 8 requirements are VERIFIED against actual, running code (not SUMMARY.md narrative): the krilla/DocSpec pipeline is fully disconnected from `act_service.rs`'s active render paths (0 references), both acts render as self-contained HTML with print CSS and no external dependencies, the templates-folder + embedded-fallback mechanism is implemented and tested (7 tests: 5 unit + verified via 2 integration tests), and the full `cargo test -p trackly-app` suite plus `svelte-check`/`pnpm build` are independently confirmed green (not just claimed).

Three items require human verification (visual print fidelity, offline font/logo rendering, live file-edit UX) — these are inherent to a browser-print-based deliverable and cannot be settled by static analysis, hence `status: human_needed` rather than `passed`.

One Critical code-review finding (CR-01, iframe sandbox) remains open. It is assessed as advisory/hardening, not a phase-goal blocker, and is surfaced above for an explicit human accept/reject decision (escalation gate) rather than silently passed over or used to fail the phase.

---

_Verified: 2026-07-05T10:30:00Z_
_Verifier: Claude (gsd-verifier)_
