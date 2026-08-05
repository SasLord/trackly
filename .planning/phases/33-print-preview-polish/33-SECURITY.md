---
phase: 33
slug: print-preview-polish
status: verified
threats_open: 0
asvs_level: 1
created: 2026-08-05
---

# Phase 33 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.
> Verify-mitigations mode: register authored at plan time (STRIDE, per-plan: 33-01…33-04).
> No new threat scanning — each declared mitigation confirmed present in the CURRENT
> working tree (`HEAD` = `6060d2a`, tree clean), i.e. **after** the post-phase quick fixes
> `c77ab6c..1f868ad` that touched this same print/preview code.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| npm registry → `ui/node_modules` | New third-party dependency `pagedjs` enters the build | Library bytes that become part of the CSP-hashed inline script |
| `bootstrapScript.js` (frozen static text) → CSP hash constant (`http/mod.rs`) | Byte-exact coupling between a UI source file and a Rust security control two plans away | SHA-256 digest |
| LAN client browser ↔ axum server | `content-security-policy` response header (`crates/trackly-app/src/http/mod.rs:219`) | CSP directive string |
| User-editable HTML act/report template (Phase 16 D-03) ↔ preview `<iframe srcdoc>` | Semi-trusted content now allowed to execute script (D-05) | Rendered act/report HTML |
| Preview `<iframe>` (opaque origin) ↔ parent `PdfPreviewModal.svelte` | New bidirectional `postMessage` channel | Page count, pixel height, CSS color hex, Paged.js error string |
| Desktop temp file (`BaseDirectory.Temp`) ↔ OS default browser | Pre-existing Phase 16 `file://` print path, now embedding the Paged.js payload | Act/report HTML + static scripts |
| App top-level document ↔ dynamically imported `pagedjs` ESM chunk | LAN print path runs Paged.js unsandboxed in the app's own origin | Same-origin module code (`script-src 'self'`) |

---

## Threat Register

All 15 plan-time threats verified. Evidence is `file:line` in the current working tree.

| Threat ID | Category | Component | Disposition | Evidence | Status |
|-----------|----------|-----------|-------------|----------|--------|
| T-33-01-01 | Tampering | `pagedjs` npm dependency ingestion | accept | `ui/package.json:25` literal `"pagedjs": "0.4.3"` (EXACT pin, no caret); `ui/pnpm-lock.yaml:26-28` `specifier: 0.4.3 / version: 0.4.3`; legitimacy audit `33-RESEARCH.md:149-161` (npm registry `0.4.3`/MIT, `slopcheck … → [OK]`, disposition Approved). See AR-33-01 | closed |
| T-33-01-02 | Tampering | `ui/src/lib/pdfPreview/bootstrapScript.js` (frozen inline-script source of truth) | mitigate | Whole file (60 LoC) is hand-authored static text — `grep '\`\|\${'` matches ONLY comment lines 5/8/21-24; zero template literals and zero interpolation in executable code. Per-render variance (backdrop/shadow) lives exclusively in the `<style>` chrome block `pagedPreviewBootstrap.ts:56-76`, outside the hashed `<script>` built at `pagedPreviewBootstrap.ts:81`. Unchanged since the hash was regenerated: `git diff 9a66ff8..HEAD -- ui/src/lib/pdfPreview` is empty | closed |
| T-33-01-03 | Info Disclosure | `pagedPreviewBridge.ts` message contract | accept | Outbound payloads are only `{pages}` (`bootstrapScript.js:33`), `{total, height}` (`:55`), `{message: String(err)}` (`:58`); inbound only `{backdrop}` (`:44-48`). No user/session data. Residual nuance recorded in AR-33-03 | closed |
| T-33-02-01 | Tampering | CSP `script-src` (`crates/trackly-app/src/http/mod.rs`) | mitigate | `http/mod.rs:219` → `script-src 'self' 'sha256-1nG6ajqUxHpGqTH1xMQEfH1DAoyP3C8xrIMr3PNVhPQ='`; exactly ONE hash source; `'unsafe-inline'` present only in `style-src` (pre-existing WR-07 decision, comment `:191-194`). Regression-asserted `crates/trackly-app/tests/security_headers.rs:104-116` | closed |
| T-33-02-02 | Tampering (control erosion) | Hash-drift guard | mitigate | `ui/scripts/check-pagedjs-csp-hash.mjs:40` reproduces the identical formula `libraryText + ';\n' + bootstrapText` (matches `pagedPreviewBootstrap.ts:29`), reads the constant from `http/mod.rs` (`:55-57`), exits 1 on mismatch (`:73-82`). Wired into the gate: `ui/package.json:16` `"lint": … && node scripts/check-pagedjs-csp-hash.mjs`; enforced in CI `.github/workflows/ci-fast.yml:111-113` and `ci-full.yml:126-129`. **Executed this audit:** `node ui/scripts/check-pagedjs-csp-hash.mjs` → `OK` (exit 0); full `pnpm --dir ui lint` → green. No drift from the post-phase quick fixes | closed |
| T-33-02-03 | Elevation of Privilege (scope) | CSP diff confined to `script-src` | accept | `git show e49bf04 -- crates/trackly-app/src/http/mod.rs` shows a single-line change adding only the hash to `script-src`; `default-src`/`style-src`/`img-src`/`connect-src`/`frame-src`/`object-src` byte-identical. `9a66ff8` changed only the digest. `security_headers.rs:104-108` isolates the `script-src` segment via `split(';').find(starts_with("script-src"))` before asserting. See AR-33-04 | closed |
| T-33-02-04 | Elevation of Privilege | Stacked control: CSP hash + iframe sandbox | accept | Both layers present and independent: hash `http/mod.rs:219`; sandbox `PdfPreviewModal.svelte:529` `sandbox="allow-scripts"` with no `allow-same-origin`. See AR-33-05 | closed |
| T-33-02-SC | Tampering (supply chain) | Package installs in plan 33-02 | accept | `check-pagedjs-csp-hash.mjs:21-24` imports only `node:crypto`/`node:fs`/`node:path`/`node:url`; no dependency added by plan 33-02 (`pagedjs` was already added and audited in 33-01). See AR-33-02 | closed |
| T-33-03-01 | Elevation of Privilege | Preview `<iframe>` sandbox posture | mitigate | Paginated branch `PdfPreviewModal.svelte:528-535` → `sandbox="allow-scripts"`, `allow-same-origin` absent (`grep sandbox` over the file returns exactly two hits, neither containing `allow-same-origin`). Degraded branch `:505` reverts to `sandbox=""` with the raw `htmlContent`, so the widened capability exists only while Paged.js is confirmed running | closed |
| T-33-03-02 | Spoofing | `postMessage` sender validation | mitigate | `pagedPreviewBridge.ts:17` `if (e.source !== iframeEl.contentWindow) return;` — object identity; `grep 'e.origin\|event.origin'` finds no code-level comparison anywhere in `ui/src/lib/pdfPreview/**` or `PdfPreviewModal.svelte` (matches are comments only). Wired at `PdfPreviewModal.svelte:233` with teardown returned from the `$effect` | closed |
| T-33-03-03 | Info Disclosure | Parent→iframe `trackly-theme-update` uses `targetOrigin '*'` | accept | `PdfPreviewModal.svelte:270-277` posts `{type, backdrop: THEME_CHROME[...].backdrop}` — a literal hex from `pagedPreviewBootstrap.ts:37-46`; no other field. See AR-33-06 | closed |
| T-33-03-04 | DoS (soft) | Pagination hang | mitigate | `PdfPreviewModal.svelte:115` `PAGINATION_TIMEOUT_MS = 8000`; armed `:206-210` on `srcdoc` assignment; `enterDegraded()` `:127-134` sets `paginationStatus = 'degraded'` and emits `console.warn`; degraded render path `:503-507` (`sandbox=""`, unpaginated). Also fires on `trackly-pagedjs-error` `:256-259`. Timer is cleared on first progress/done `:244`/`:253` (silence detector, per D-02) | closed |
| T-33-04-01 | Tampering | Desktop temp file embeds the inline script | accept | `PdfPreviewModal.svelte:308-336`: `trackly-print-${Date.now()}.html` written to `BaseDirectory.Temp` (`:331-332`), content = server-rendered HTML + frozen `PAGED_PREVIEW_INLINE_SCRIPT` (`:312`) + a 1-line print-trigger script (`:315`, itself guarded by `e.source!==window`, plan-registered at `33-04-PLAN.md:101-102,121`). No secrets/session data; `file://`, no CSP. See AR-33-07 | closed |
| T-33-04-02 | Elevation of Privilege (scope minimization) | LAN print path `printViaTopLevel` | mitigate | `PdfPreviewModal.svelte:465` `const { Previewer } = await import('pagedjs');` — dynamic same-origin ESM, covered by existing `script-src 'self'`. Repo-wide grep for `PAGED_PREVIEW_INLINE_SCRIPT` finds only two uses: `buildSrcdoc` (opaque-origin srcdoc) and `printViaSystemBrowser` (Tauri-only `file://`); the LAN path injects **no** inline `<script>`. CSP `script-src` still carries exactly one hash source — no second inline dependency introduced | closed |
| T-33-04-03 | Tampering (regression risk) | Dual Paged.js runs may diverge in page breaks | accept | Not covered by automated tests (no frontend rendering harness). Manual-UAT item recorded in `33-VALIDATION.md:78` ("Превью совпадает с бумагой" — сверить число страниц и точки разрыва). See AR-33-08, incl. residual coverage note | closed |

*Status: open · closed*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-33-01 | T-33-01-01 | Third-party `pagedjs` in the build. Mitigated by an EXACT version pin (`0.4.3`, no caret) so the CSP hash-source cannot drift on a routine `pnpm install`, plus a registry + `slopcheck [OK]` legitimacy audit (33-RESEARCH.md). Residual supply-chain exposure of a single MIT, 7-year-old, Coko-Foundation-maintained package is accepted. | Plan 33-01 | 2026-08-04 |
| AR-33-02 | T-33-02-SC | No new package-manager installs in plan 33-02; the hash-drift gate is zero-dependency (`node:` builtins only). No new supply-chain surface to audit. | Plan 33-02 | 2026-08-04 |
| AR-33-03 | T-33-01-03 | Bridge payloads are non-secret by design (page count, pixel height, CSS color hex). **Nuance recorded at audit time:** the register's enumeration omits the fourth payload, `trackly-pagedjs-error { message: String(err) }` (`bootstrapScript.js:58`). That string flows inward — from the opaque-origin iframe to the parent, which already holds the source HTML — and is consumed only by `console.warn` (`PdfPreviewModal.svelte:129-134`). No privilege or confidentiality boundary is crossed; accepted as within the declared "no user/session data" contract. | Plan 33-01 + auditor | 2026-08-05 |
| AR-33-04 | T-33-02-03 | The CSP change is additive to `script-src` only; every other directive is byte-identical to the pre-phase header. Verified by commit diff, not by intent. | Plan 33-02 | 2026-08-04 |
| AR-33-05 | T-33-02-04 | A CSP hash source grants permission to run, not capability. The residual capability of the granted script is bounded independently by `sandbox="allow-scripts"` without `allow-same-origin` (opaque origin: no cookies, no `localStorage`, no parent DOM). Two stacked, independently verified controls for the user-editable-template boundary. | Plan 33-02 | 2026-08-04 |
| AR-33-06 | T-33-03-03 | Parent→iframe `postMessage` must use `targetOrigin: '*'` because the opaque-origin iframe has no addressable origin. Payload is a literal CSS color hex from a compile-time constant map — no user or session data. | Plan 33-03 | 2026-08-04 |
| AR-33-07 | T-33-04-01 | Desktop print writes a timestamp-named, single-use temp file under the OS temp dir and opens it in the default browser (pre-existing Phase 16 trust model). It now embeds a larger static script payload; the file still contains only server-rendered act/report HTML plus static scripts — no secrets, no session data. Size changed, risk profile did not. | Plan 33-04 | 2026-08-04 |
| AR-33-08 | T-33-04-03 | Preview-vs-print page-break divergence is not provable by any automated test in this repo (no headless rendering harness; building one is out of phase scope). Recorded as a manual-UAT item (`33-VALIDATION.md:78`). **Residual note at audit time:** commit `9a66ff8` states "Multi-page preview remains unobserved end-to-end", so this row's multi-page case may still be unexercised — the risk is accepted, but the manual check should be run against a document with N≥2 pages before the release tag. | Plan 33-04 + auditor | 2026-08-05 |

*Accepted risks do not resurface in future audit runs.*

---

## Unregistered Flags

None (no BLOCKER, no WARNING).

- `33-02-SUMMARY.md:100-102` is the only summary carrying a `## Threat Flags` section; it reports "None".
- Summaries 33-01, 33-03, 33-04 omit the section entirely. Compensating check performed instead of assuming completeness: repo-wide grep for `pagedjs` / `PAGED_PREVIEW_INLINE_SCRIPT` / `printViaTopLevel` / `act-print-root` / `Previewer` across `ui/src`, `crates/*/src`, `crates/*/tests`, `ui/scripts` returned only `pagedPreviewBootstrap.ts`, `pagedPreviewBridge.ts`, `bootstrapScript.js`, `pagedjs.d.ts` (types only), `PdfPreviewModal.svelte`, `check-pagedjs-csp-hash.mjs` and `http/mod.rs` — i.e. no unmapped print/preview entry point, no second inline-script site, no new network endpoint or auth path.
- The desktop print-trigger script (`PdfPreviewModal.svelte:315`) is an inline script beyond `PAGED_PREVIEW_INLINE_SCRIPT` and is **not** separately named in the T-33-04-01 row; it is however specified verbatim in `33-04-PLAN.md:101-102` with the `e.source !== window` guard required at `:121`. Mapped to T-33-04-01, not an unregistered flag.

---

## Auditor Observations (non-blocking, not in the register)

1. `check-pagedjs-csp-hash.mjs:33` matches the FIRST `'sha256-…'` token anywhere in `http/mod.rs`, not scoped to the `script-src` directive. It currently resolves correctly (no `sha256-` literal appears in the comment block `:191-217`), but a future comment or a second hash source would make the gate check the wrong token. Hardening idea only — no action taken (implementation is read-only in this audit).
2. `security_headers.rs` asserts the *presence* of a `sha256-` source, not its *value*; digest equality is enforced solely by the `pnpm lint` gate. Both are wired into CI, so the pair is sufficient — noted so a future removal of the lint gate is understood to remove the only value check.

---

## Verification Commands Run (2026-08-05, read-only)

| Command | Result |
|---------|--------|
| `node ui/scripts/check-pagedjs-csp-hash.mjs` | `[check-pagedjs-csp-hash] OK` (exit 0) — no CSP hash drift after the post-phase quick fixes |
| `node ui/scripts/check-pagedjs-csp-hash.mjs --print` | `sha256-1nG6ajqUxHpGqTH1xMQEfH1DAoyP3C8xrIMr3PNVhPQ=` — byte-equal to `http/mod.rs:219` |
| `pnpm --dir ui lint` | green (eslint, prettier, check-tokens, check-contrast, check-focus-outline, check-pagedjs-csp-hash) — proves the drift gate is actually wired into the gate command |
| `git diff 9a66ff8..HEAD -- ui/src/lib/pdfPreview ui/scripts/check-pagedjs-csp-hash.mjs crates/trackly-app/src/http/mod.rs ui/package.json crates/trackly-app/tests/security_headers.rs` | empty — the six post-phase quick fixes (`c77ab6c..1f868ad`) touched only `PdfPreviewModal.svelte`, never a hash-source or CSP file |

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-08-05 | 15 | 15 | 0 | gsd-security-auditor (Claude Opus 5) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-08-05
