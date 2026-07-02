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

## dashboard-consumption-chart-422 — Consumption chart 422 (snake vs camelCase) + empty single-bucket render
- **Date:** 2026-07-02
- **Error patterns:** HTTP 422, Unprocessable Entity, dashboard_get_consumption_chart, window_months, windowMonths, rename_all camelCase, serde deserialize, empty chart, no plotted data, single month bucket, polyline draws nothing, Динамика расхода картриджей, both transports, Tauri + LAN browser
- **Root cause (cycle 1 — 422):** Frontend `DashboardPage.svelte` sent the request payload with snake_case key `window_months`, but the shared request contract expects camelCase `windowMonths` — the axum `GetConsumptionChartPayload` has `#[serde(rename_all = "camelCase")]` over a *required* (non-Option) field, and the tauri-specta generated binding also uses `windowMonths`. Missing required field → serde deserialize failure → axum `Json` extractor returns **422 on both transports** (Tauri invoke and /api/v1 share the same DTO). Reproduced everywhere because it's shared validation, not transport code. `dashboard_get_all_widgets` was unaffected because its only field `period` is single-word (camel==snake) AND Option. The deployed `ui/dist` also still carried the old key, so a rebuild was required.
- **Root cause (cycle 2 — empty render, progression after 422 fix):** Request now succeeds and returns data, but the dev DB had all cartridge installs in the current month, so the backend `GROUP BY strftime('%Y-%m', ...)` collapsed everything into a single `month_key` → `uniqueMonths.length === 1`. `ChartWidget.svelte` rendered each model series only as an SVG `<polyline>`, and `toPoints()` returned `''` whenever `series.length < 2`. A single-point series produced an empty polyline that drew nothing — chart looked empty (only the legend + one centered «Июн.» x-tick were visible) despite data being present. NOT the `data.length === 0` empty-state branch.
- **Fix:** (1) Changed the payload key in `DashboardPage.svelte` from `window_months` to `windowMonths`. (2) In `ChartWidget.svelte`, split `toPoints()` into coord + polyline helpers and added a `<circle>` marker per data point for every series, so single-bucket (one-month) series render as visible dots (polylines still draw for ≥2 months); centered the lone x-tick. Rebuilt ui/dist for server mode.
- **Files changed:** ui/src/features/dashboard/DashboardPage.svelte, ui/src/features/dashboard/ChartWidget.svelte (ui/dist rebuilt, gitignored)
- **Lesson:** When a Rust DTO uses `#[serde(rename_all = "camelCase")]`, every hand-written frontend `apiCall(name, {...})` must use camelCase keys (the tauri-specta bindings.ts is the source of truth); a snake_case key on a required field surfaces as an opaque 422 on both transports. Separately: a line/polyline chart must have a point-marker fallback, or single-time-bucket data renders invisibly.
---
