---
slug: pdf-json-parse-lan-browser
status: resolved
trigger: |
  Toast: "Не удалось сгенерировать PDF"
  JS error: Unexpected token '%', "%PDF-1.7 %"... is not valid JSON
created: 2026-07-02
updated: 2026-07-02
---

# Debug Session: pdf-json-parse-lan-browser

## Symptoms

- **Expected:** PDF generation succeeds and the document downloads/opens.
- **Actual:** Toast "Не удалось сгенерировать PDF"; console shows
  `Unexpected token '%', "%PDF-1.7 %"... is not valid JSON`.
- **Error message:** The literal PDF header (`%PDF-1.7`) is being passed to
  `JSON.parse`/`response.json()` — the binary PDF body is being decoded as JSON.
- **Context:** Occurs in **LAN browser mode** (server mode, `/api/v1/...`), NOT
  reported for desktop Tauri.
- **Scope:** Affects EVERY PDF path — акт приёма-передачи, акт возврата,
  печать документа приёма, отчёты (Reports). Additionally, in Settings the
  document **templates do not preview** (likely same shared fetch layer).
- **Timeline / reproduction:** Trigger any print/PDF action from the LAN browser.

## Current Focus

reasoning_checkpoint:
  hypothesis: >
    The shared `apiCall` helper (ui/src/lib/api/client.ts) unconditionally calls
    `res.json()` on the browser fetch path. The four PDF endpoints
    (acts_render_pdf, devices_render_acceptance_pdf, reports_export_pdf,
    templates_render_preview) return raw `application/pdf` bytes from axum, so
    `res.json()` throws `Unexpected token '%', "%PDF-1.7 %"...`. Desktop is
    unaffected because it uses Tauri `invoke`, which returns the `Vec<u8>` as a
    `number[]`. This is why the failure is LAN-browser-only and hits every PDF path.
  confirming_evidence:
    - "client.ts line 48: `return res.json();` runs on every browser response, no content-type check"
    - "acts.rs handler_render_pdf + handler_render_acceptance_pdf return `[(CONTENT_TYPE, application/pdf)], bytes` (raw, not Json)"
    - "reports.rs:216 and templates.rs:44 both return `application/pdf` raw bytes"
    - "All four call sites use `apiCall<number[]>(...)` → the dual-path helper, not the desktop-only `commands.*`/TAURI_INVOKE"
    - "fetchPdfBlob (pdf.ts) + callers expect `number[]`, wrapped via `new Uint8Array(bytes)`"
  falsification_test: >
    If the browser Network tab showed these endpoints returning JSON
    (`application/json`) rather than `application/pdf`, the hypothesis would be
    wrong. It does not — handlers hard-code `application/pdf`.
  fix_rationale: >
    Make `apiCall` content-type-aware: on a successful non-JSON response, read
    `arrayBuffer` and return `Array.from(new Uint8Array(...))` as `number[]`,
    matching the Tauri shape exactly. Single shared helper covers all four
    endpoints; callers and fetchPdfBlob are unchanged.
  blind_spots: >
    Have not confirmed error responses from PDF endpoints are still JSON (they
    go through AppErrorResponse, which is Json) — the fix only branches on the
    success (res.ok) path, so error handling is untouched. Not yet rebuilt
    ui/dist for LAN-browser verification.

- next_action: apply content-type branch in apiCall, rebuild ui/dist, verify in LAN browser.

## Evidence

- timestamp: 2026-07-02
  checked: ui/src/lib/api/client.ts (apiCall shared helper)
  found: >
    Browser path does `const res = await fetch('/api/v1/${name}', ...)` then
    unconditionally `return res.json();` (line 48). No Content-Type inspection.
  implication: Any endpoint returning non-JSON bytes breaks with a JSON.parse error.

- timestamp: 2026-07-02
  checked: crates/trackly-app/src/http/acts.rs handler_render_pdf / handler_render_acceptance_pdf
  found: >
    Both return `(StatusCode::OK, [(header::CONTENT_TYPE, "application/pdf")], bytes)`
    — raw PDF bytes, not `Json`.
  implication: The `%PDF-1.7` body is exactly what apiCall feeds to res.json().

- timestamp: 2026-07-02
  checked: reports.rs:211-217, templates.rs:39-45
  found: Both return raw `application/pdf` bytes (reports_export_pdf, templates_render_preview).
  implication: Explains full scope — reports export AND Settings template preview also break.

- timestamp: 2026-07-02
  checked: call sites — lib/api/acts.ts, features/reports/ReportsPage.svelte, lib/api/templates.ts
  found: All use `apiCall<number[]>(...)`; fetchPdfBlob (pdf.ts) wraps result in `new Uint8Array(bytes)`.
  implication: >
    Contract expects `number[]`. Fix must make the browser path yield `number[]`
    from the PDF bytes to match Tauri (Vec<u8> → number[]).

- timestamp: 2026-07-02
  checked: current working-tree ui/src/lib/api/client.ts (58 lines, not the committed 49-line version)
  found: >
    The content-type branch fix ALREADY EXISTS in the working tree (lines 48-56):
    on a successful non-JSON response it does
    `const buf = new Uint8Array(await res.arrayBuffer()); return Array.from(buf) as R;`.
    `git status` shows client.ts as ` M` (modified, uncommitted). Last commit
    (f482291) did NOT contain this branch.
  implication: >
    The SOURCE fix is present but uncommitted. The code-level root cause is
    already addressed; what remains is delivery.

- timestamp: 2026-07-02
  checked: ui/dist/assets/index-BPtGZDDO.js (built Jun 30 22:53 — before the client.ts edit)
  found: >
    The shipped bundle's apiCall HTTP path reads
    `fetch('/api/v1/${a}'...); if(!r.ok){...}` then returns `r.json()` with NO
    content-type branch. The one `arrayBuffer` occurrence in the bundle is the
    OrgSettings file-upload (`Array.from(new Uint8Array(le))`), not the client.ts fix.
  implication: >
    The LAN browser serves ui/dist, which still runs the OLD apiCall that calls
    res.json() on the %PDF-1.7 body. The fix was written but never rebuilt into
    dist. Per project memory: `cargo tauri dev` only HMRs the desktop webview;
    LAN browser needs `pnpm --dir ui build`.

- timestamp: 2026-07-02
  checked: >
    Human verification of fix #1 (LAN browser https://192.168.1.2:8443) — reported
    by coordinator relay.
  found: >
    JSON.parse error GONE; blob URL is now created. But PDF preview shows BLANK
    WHITE area. New console error, repeated per blob:
    "Refused to load blob:https://192.168.1.2:8443/... because it appears in
    neither the frame-src directive nor the default-src directive of the CSP."
  implication: >
    Fix #1 confirmed working. SECOND root cause: the axum CSP header blocks
    framing of blob: PDFs. PdfPreviewModal and TemplateEditor render the PDF in
    an <iframe src={blobUrl}>; CSP had no frame-src, so blob: fell back to
    default-src 'self' → refused.

- timestamp: 2026-07-02
  checked: crates/trackly-app/src/http/mod.rs:137-147 (security_headers middleware)
  found: >
    CSP was `default-src 'self'; script-src 'self'; style-src 'self'
    'unsafe-inline'; connect-src 'self' wss:` — no frame-src, no object-src.
  implication: blob: iframe framing is disallowed → blank preview. Root cause #2 confirmed.

- timestamp: 2026-07-02
  checked: PdfPreviewModal.svelte + TemplateEditor.svelte render tags
  found: Both use `<iframe src={blobUrl}>`. No <embed>/<object> anywhere.
  implication: >
    `frame-src 'self' blob:` is the directive that governs the preview. Added
    `object-src 'self' blob:` defensively (harmless; covers any future <object> fallback).

- timestamp: 2026-07-02
  checked: crates/trackly-app/tests/security_headers.rs
  found: >
    Test asserts only x-frame-options and x-content-type-options — NOT the CSP
    string. Adding frame-src does not break it. Added a new assertion
    (`csp.contains("frame-src") && csp.contains("blob:")`) to lock in the fix.
  implication: CI security_headers test stays green and now guards the PDF-CSP fix.

- timestamp: 2026-07-02
  checked: cargo test -p trackly-app --test security_headers (AD/SNMP mock env)
  found: 4 passed, 0 failed — including updated security_headers_present with the CSP assertion.
  implication: Fix #2 compiles and passes at the integration-test level.

## Out-of-scope (noted, NOT fixed this session)

- `dashboard_get_consumption_chart` returns 422 Unprocessable Entity in the LAN
  browser. Pre-existing, unrelated to the PDF/CSP chain. Likely a payload-shape
  mismatch between the frontend fetch body and the axum handler's expected DTO
  (compare with the `req`-wrapper regression noted in security_headers.rs). Flag
  for a separate debug session.

## Eliminated

## Resolution

root_cause: >
  TWO root causes in the LAN-browser PDF chain:
  (1) JSON-parse of binary PDF — shared `apiCall` browser-fetch helper
  (ui/src/lib/api/client.ts) called `res.json()` on all successful responses;
  the four PDF endpoints return raw `application/pdf` bytes from axum, so
  JSON.parse failed on `%PDF-1.7`. Desktop unaffected (Tauri invoke returns
  Vec<u8> as number[]).
  (2) CSP blocked blob: framing — after fix #1 produced a blob URL, the axum
  Content-Security-Policy had no frame-src, so the <iframe src={blob:...}> in
  PdfPreviewModal/TemplateEditor was refused (fell back to default-src 'self'),
  yielding a blank preview.
fix: >
  Fix #1: Made `apiCall` (ui/src/lib/api/client.ts) content-type-aware on the
  browser fetch path — after `res.ok`, if content-type is not application/json,
  read arrayBuffer and return `Array.from(new Uint8Array(...))` as number[]
  (matches Tauri Vec<u8> → number[]). Rebuilt ui/dist via `pnpm --dir ui build`.
  Fix #2: Added `frame-src 'self' blob:; object-src 'self' blob:` to the axum CSP
  in crates/trackly-app/src/http/mod.rs so the <iframe> PDF previews can render
  their blob: URLs. Added a CSP assertion to tests/security_headers.rs (frame-src
  + blob:) — existing header test untouched (only asserts x-frame-options /
  x-content-type-options). Server-side change → needs `cargo build`/restart +
  browser hard-refresh, no UI rebuild for fix #2.
verification: >
  Fix #1: human-verified in LAN browser — %PDF JSON error gone, blob URL created.
  svelte-check 0 errors; fresh bundle index-CVXnPDXN.js contains the branch.
  Fix #2: `cargo test -p trackly-app --test security_headers` → 4 passed incl.
  updated CSP assertion.
  FULLY HUMAN-VERIFIED (2026-07-02): user confirmed in the LAN browser that PDF
  previews now render correctly across all paths — акт приёма-передачи / возврата
  / печать документа приёма / Reports PDF / Settings template preview — no blank
  page, no CSP blob error, no %PDF JSON error.
files_changed:
  - ui/src/lib/api/client.ts
  - ui/dist/* (rebuilt)
  - crates/trackly-app/src/http/mod.rs
  - crates/trackly-app/tests/security_headers.rs
