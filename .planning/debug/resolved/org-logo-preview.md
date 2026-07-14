---
slug: org-logo-preview
status: resolved
trigger: |
  В Настройках, раздел Организация, если загрузить svg логотип то он не отображается в превью.
  А в Акте при Печати — вставляется правильно. Если подключиться через web-браузер, то там в
  Настройках, Организация: превью вообще не отображает загруженный логотип, даже png.
created: 2026-07-14
updated: 2026-07-14
---

# Debug: Org logo preview not rendering

## Symptoms

- **Expected:** Uploaded org logo (PNG or SVG) shows in the Settings → Organization preview, in both desktop and web-browser (server mode).
- **Actual:**
  1. Desktop app: **SVG** logo does NOT show in preview. PNG shows fine.
  2. Desktop app: SVG (and PNG) print CORRECTLY in the Act (печать) — so storage/backend is fine.
  3. Web browser (server mode): preview shows NOTHING, even for PNG.
- **Error messages:** none reported by user (browser console likely has a CSP "Refused to load blob:" message — to confirm).
- **Timeline:** not specified.
- **Reproduction:** Settings → Organization → upload logo → observe preview area.

## Current Focus

status: resolved

reasoning_checkpoint:
  hypothesis: "Settings preview <img> gets a typeless blob: URL. (A) SVG needs an explicit image/svg+xml type or the browser won't render it → blank SVG on desktop; (B) server-mode CSP img-src is 'self' data: (no blob:) so ANY blob: URL is blocked in the LAN browser → blank PNG and SVG there."
  confirming_evidence:
    - "OrgSettings.svelte:71 builds `new Blob([ua])` with no type; OrgSettingsDto/settings_get_org_logo expose bytes only, no mime — client can't type the blob."
    - "http/mod.rs CSP img-src = 'self' data: (no blob:); act path embeds logo as data:image URI server-side and prints fine, proving bytes/storage are correct."
    - "org_db_service already stores logo_mime (used by get_for_pdf) and a purpose-built OrgLogoDto{logo_bytes,logo_mime} already exists in dto/reports.rs."
  falsification_test: "Render preview as data:${mime};base64 URL carrying stored mime. If SVG still blank on desktop OR PNG still blank in browser, hypothesis is wrong."
  fix_rationale: "Return stored mime alongside bytes (OrgLogoDto) and build a data: URL. data: is already allowed by CSP img-src → fixes browser case with no CSP edit; correct mime → fixes SVG case. Addresses root cause (missing mime typing), not a symptom."
  blind_spots: "Not exercising a live browser here; verifying via unit/integration tests + code. btoa on large byte arrays is fine for ≤512KiB logos (size-capped server-side)."

test: Return stored MIME via OrgLogoDto from settings_get_org_logo; render preview as data: URL (allowed by existing img-src data:).
expecting: PNG + SVG preview render in both desktop and web browser.
next_action: Apply Rust DTO/service/handler change + Svelte loadLogo change, regenerate bindings via cargo test, verify.

## Evidence

- timestamp: 2026-07-14 — `ui/src/features/settings/OrgSettings.svelte:71` — `loadLogo()` does `const blob = new Blob([ua]);` with NO type argument → typeless blob → `URL.createObjectURL` yields a blob: URL with no content-type. Browsers byte-sniff PNG in <img> but require an explicit `image/svg+xml` for SVG, so SVG preview is blank on desktop.
- timestamp: 2026-07-14 — `OrgSettingsDto` (OrgSettings.svelte:8-21) has NO `logo_mime` field; `settings_get_org_logo` returns raw `Vec<u8>` only. Client cannot know the MIME on reload, so it can't set the blob type.
- timestamp: 2026-07-14 — Act printing works because the act HTML embeds the logo as a proper `data:image` URI server-side (per comment at `crates/trackly-app/src/http/mod.rs:148-150`, GAP-16-01), where the MIME is known. Confirms bytes/storage are correct; the bug is purely in the settings PREVIEW path.
- timestamp: 2026-07-14 — Server-mode CSP at `crates/trackly-app/src/http/mod.rs:155`: `... img-src 'self' data:; ... frame-src 'self' blob:; object-src 'self' blob:`. `img-src` allows `'self'` and `data:` but NOT `blob:`. The preview `<img src={logoObjectUrl}>` uses a `blob:` object URL → blocked by CSP in the browser → no preview at all (PNG and SVG). Desktop (Tauri) does not apply this axum CSP, so PNG works there.
- timestamp: 2026-07-14 — `settings_headers` test (`crates/trackly-app/tests/security_headers.rs:92-97`) already asserts `img-src` contains `data:` for the act logo — reinforces that `data:` is the sanctioned image scheme in this app; switching the preview to a `data:` URL aligns with existing policy and needs no CSP change.

## Eliminated

- hypothesis: Backend loses/corrupts the logo bytes on save. — ELIMINATED: act printing renders the exact same stored logo correctly.

## Proposed fix (for debugger to apply + verify)

Make the settings preview use a MIME-typed source instead of a typeless blob: URL. Recommended minimal approach:
1. Expose the stored MIME to the client — either add `logo_mime: String` to `OrgSettingsDto` (and populate it in `settings_get_org`), or change `settings_get_org_logo` to return `{ bytes, mime }`. Prefer whichever matches existing DTO/service patterns; the service already stores the mime (used by the act data: URI path).
2. In `loadLogo()`, build the preview as a `data:${mime};base64,...` URL (or `new Blob([ua], { type: mime })`). A `data:` URL is preferred because it is already permitted by the server-mode CSP `img-src 'self' data:` — fixing the web-browser case WITHOUT a CSP change, and fixing the SVG case by carrying the correct MIME.
   - If a blob: URL is kept instead, the CSP at http/mod.rs:155 MUST also add `blob:` to `img-src`, and the blob must be typed. The data: approach avoids the CSP edit.

## Resolution

root_cause: |
  The Settings → Organization logo preview rendered the logo through a typeless
  object URL: `settings_get_org_logo` returned raw `Vec<u8>` (no MIME), and
  `loadLogo()` built `new Blob([ua])` with no `type`. Two consequences:
  (A) SVG needs an explicit `image/svg+xml` MIME to render in an <img>; a typeless
      blob left the desktop SVG preview blank (PNG survived via byte-sniffing).
  (B) The blob: object URL was blocked in server mode because the axum CSP
      `img-src 'self' data:` (crates/trackly-app/src/http/mod.rs) does not allow
      `blob:` — so the browser preview was blank for PNG and SVG alike. Desktop
      (Tauri) does not apply that CSP, which is why PNG worked there.
  Storage/bytes were always correct — the act print path embeds the logo as a
  server-side data:image URI (MIME known) and printed fine, proving the bug was
  purely in the settings PREVIEW path.
fix: |
  Carry the stored MIME to the client and render the preview as a data: URL
  (already permitted by the existing CSP `img-src 'self' data:` — no CSP edit,
  security_headers test stays green).
  - Backend: `settings_get_org_logo` (Tauri cmd + axum handler) now returns the
    existing purpose-built `OrgLogoDto { logo_bytes, logo_mime }` instead of
    `Vec<u8>`. New `OrgDbService::get_logo()` reads logo_blob + logo_mime in one
    query. Regenerated ui/src/bindings.ts via export_bindings.
  - Frontend: `loadLogo()` consumes OrgLogoDto and builds
    `data:${mime};base64,<b64>` (chunked base64 encode). Renamed the reactive
    var `logoObjectUrl` → `logoSrc` and dropped the now-unnecessary
    URL.createObjectURL/revokeObjectURL bookkeeping.
verification: |
  - cargo check -p trackly-app: clean.
  - cargo test export_bindings: ok (bindings regenerated; settingsGetOrgLogo now
    returns OrgLogoDto).
  - cargo test org_settings: 4 passed (round-trip, save/delete, size limit,
    invalid mime — get_logo_bytes retained, no breakage).
  - cargo test security_headers: 4 passed — CSP unchanged, img-src still 'self' data:.
  - svelte-check: 0 errors (48 pre-existing warnings, none in OrgSettings.svelte).
  - pnpm --dir ui build: ok (ui/dist refreshed for server-mode/LAN-browser test).
  - PENDING human verification: live PNG + SVG preview in desktop AND web browser.
files_changed:
  - crates/trackly-app/src/services/org_db_service.rs (new get_logo() -> OrgLogoDto; import OrgLogoDto)
  - crates/trackly-app/src/tauri_cmds/settings_org.rs (settings_get_org_logo returns OrgLogoDto)
  - crates/trackly-app/src/http/settings_org.rs (handler_get_org_logo returns Json<OrgLogoDto>)
  - ui/src/features/settings/OrgSettings.svelte (loadLogo builds data: URL; logoObjectUrl -> logoSrc)
  - ui/src/bindings.ts (regenerated: settingsGetOrgLogo -> OrgLogoDto)
