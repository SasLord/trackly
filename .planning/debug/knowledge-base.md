# GSD Debug Knowledge Base

Resolved debug sessions. Used by `gsd-debugger` to surface known-pattern hypotheses at the start of new investigations.

---

## pdf-json-parse-lan-browser — PDFs broken in LAN browser (JSON-parse of binary + CSP blob framing)
- **Date:** 2026-07-02
- **Error patterns:** Unexpected token '%', %PDF-1.7, is not valid JSON, application/pdf, Refused to load blob, frame-src, default-src, Content-Security-Policy, blank preview, LAN browser, server mode
- **Root cause:** Two chained causes, LAN-browser-only (desktop Tauri fine). (1) Shared `apiCall` fetch helper (ui/src/lib/api/client.ts) called `res.json()` on all successful browser responses; the /api/v1 PDF endpoints return raw `application/pdf` bytes, so JSON.parse choked on `%PDF-1.7`. (2) After that fix produced a blob: URL, the axum CSP had no `frame-src`, so `<iframe src=blob:...>` previews were refused (fell back to default-src 'self') and rendered blank.
- **Fix:** (1) Made `apiCall` content-type-aware: non-JSON success → arrayBuffer → `Array.from(new Uint8Array(...))` as number[], matching Tauri Vec<u8>→number[]; rebuilt ui/dist. (2) Added `frame-src 'self' blob:; object-src 'self' blob:` to CSP in http/mod.rs + regression assertion in security_headers.rs.
- **Files changed:** ui/src/lib/api/client.ts, crates/trackly-app/src/http/mod.rs, crates/trackly-app/tests/security_headers.rs (ui/dist rebuilt, gitignored)
---
