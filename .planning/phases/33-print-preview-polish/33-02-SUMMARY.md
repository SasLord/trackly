---
phase: 33-print-preview-polish
plan: 02
subsystem: security
tags: [csp, sha256, hash-drift-gate, axum, paged-js, structural-test]

# Dependency graph
requires:
  - "PAGED_PREVIEW_INLINE_SCRIPT frozen concatenation formula from Plan 33-01 (ui/src/lib/pdfPreview/pagedPreviewBootstrap.ts)"
provides:
  - "'sha256-<digest>' hash-source in LAN-mode axum CSP script-src, permitting the frozen Paged.js bootstrap script to execute inside the srcdoc preview iframe (D-14)"
  - "ui/scripts/check-pagedjs-csp-hash.mjs — build-time hash recompute + drift check, wired into pnpm lint"
  - "crates/trackly-app/tests/html_page_parity.rs — D-13 @page-parity structural regression guard"
affects: [33-03-preview-wiring, 33-04-print-paths]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Zero-dependency Node drift-check script (node:crypto/fs/path/url only) that recomputes a security-relevant hash and compares it against a hardcoded Rust constant — follows check-tokens.mjs's existing CI-gate convention"
    - "include_str! (compile-time, CWD-independent) for a Rust test that reads shipped template files as read-only structural fixtures"

key-files:
  created:
    - ui/scripts/check-pagedjs-csp-hash.mjs
    - crates/trackly-app/tests/html_page_parity.rs
  modified:
    - ui/package.json
    - crates/trackly-app/src/http/mod.rs
    - crates/trackly-app/tests/security_headers.rs

key-decisions:
  - "CSP hash-source value computed once via `node ui/scripts/check-pagedjs-csp-hash.mjs --print` (sha256-5ZDjul5PEiak1qhxbmi9Rx3W4tYmf4sQbt9wgef8vQY=) and hardcoded as a literal in http/mod.rs's HeaderValue::from_static string, per RESEARCH.md's compile-time-constant requirement (the header is a &'static str literal, not built at runtime)"
  - "script-src segment isolated in security_headers.rs's assertion (split on ';', find segment starting with 'script-src') rather than checking the whole CSP string, since style-src legitimately keeps 'unsafe-inline' and a whole-string check would give a false pass/fail either way"

patterns-established:
  - "PRV-CSP-tagged comment block in http/mod.rs, following the file's existing WR-07/T-06-12-I/PDF-CSP/GAP-16-01 dated-comment convention for every prior CSP-string addition"

requirements-completed: [PRV-01, PRV-02]

duration: ~50min
completed: 2026-08-04
---

# Phase 33 Plan 02: CSP hash-source for Paged.js preview + D-13 template-parity guard Summary

**Adds exactly one `sha256-<digest>'` hash-source to the LAN-mode axum CSP's `script-src` directive so the frozen Paged.js bootstrap script (Plan 33-01) can execute inside the preview `<iframe srcdoc>` in server mode, with an independent Node-side drift-detection gate wired into `pnpm lint`; also lands a structural regression test guarding that all three print templates declare identical `@page` blocks (D-13).**

## Performance

- **Duration:** ~50 min (dominated by two full `cargo test -p trackly-app` invocations — one for `security_headers`, one for the newly-added `html_page_parity` — each requiring a fresh `trackly-app` crate compile, ~3m17s for the second)
- **Tasks:** 3/3
- **Files modified:** 5 (2 created, 3 modified)

## Accomplishments

- Created `ui/scripts/check-pagedjs-csp-hash.mjs` — zero-dependency Node script that reads `node_modules/pagedjs/dist/paged.min.js` + `src/lib/pdfPreview/bootstrapScript.js`, recomputes `sha256-<base64 digest>` over the exact `libraryText + ';\n' + bootstrapText` formula, and either prints it (`--print`) or diffs it against the hardcoded token in `crates/trackly-app/src/http/mod.rs`, failing loudly on drift or absence
- Wired the script into `ui/package.json`'s `lint` chain (after `check-focus-outline.mjs`)
- Added `'sha256-5ZDjul5PEiak1qhxbmi9Rx3W4tYmf4sQbt9wgef8vQY='` to `script-src` in `http/mod.rs`'s CSP `HeaderValue::from_static(...)` string — every other directive byte-identical (verified via `git diff`, single-token addition), with a new `PRV-CSP (Phase 33, D-14)` comment block following the file's existing named/dated-comment convention
- Extended `security_headers.rs`'s `security_headers_present` test with a `script-src`-segment-isolated assertion (`contains("sha256-")` and NOT `contains("unsafe-inline")`)
- Created `crates/trackly-app/tests/html_page_parity.rs` — `include_str!`-based structural test asserting `act_handover.html`, `act_acceptance.html`, and `report.html` all declare the same `@page { size; margin }` block (D-13); templates read-only, never modified (D-01)

## Task Commits

Each task was committed atomically:

1. **Task 1: CSP hash drift-detection script** - `74e49be` (feat)
2. **Task 2: Apply the CSP hash-source and extend the security-headers regression test** - `e49bf04` (feat)
3. **Task 3: D-13 @page-parity structural test** - `96d7417` (test)

_No TDD tasks in this plan (`tdd_mode: false` in config, plan not marked `type: tdd`)._

## Files Created/Modified

- `ui/scripts/check-pagedjs-csp-hash.mjs` (new) — recomputes and verifies the CSP hash-source; `--print` flag for regenerating the constant
- `ui/package.json` — `lint` script extended with `&& node scripts/check-pagedjs-csp-hash.mjs`
- `crates/trackly-app/src/http/mod.rs` — `script-src` gains `'sha256-<digest>'`; new `PRV-CSP` comment block; all other CSP directives unchanged
- `crates/trackly-app/tests/security_headers.rs` — new `script-src`-segment assertion for `sha256-` presence and `unsafe-inline` absence
- `crates/trackly-app/tests/html_page_parity.rs` (new) — D-13 `@page`-parity structural regression test

## Decisions Made

- Followed the plan's exact interfaces verbatim: hash computed via `sha256(base64(...))` (not hex), the frozen `pagedjsLibraryText + ';\n' + bootstrapText` formula replicated in Node without importing the `.ts` module (direct `fs.readFileSync` of both source files), and the Rust constant applied as a hardcoded literal since `HeaderValue::from_static` requires a `&'static str`.
- Used `regex::Regex::new(r"(?s)@page\s*\{[^}]*\}")` in `html_page_parity.rs` per the plan's exact specified pattern; `regex` is already a direct `trackly-app` dependency (`Cargo.toml:54`) — no new crate added.

## Deviations from Plan

None — plan executed exactly as written. All acceptance criteria and verification commands from the plan passed without needing any Rule 1-4 fixes.

## Verification Results

- `node ui/scripts/check-pagedjs-csp-hash.mjs --print` → `sha256-5ZDjul5PEiak1qhxbmi9Rx3W4tYmf4sQbt9wgef8vQY=` (single line, base64-looking, matches `^sha256-[A-Za-z0-9+/=]+$`)
- `node ui/scripts/check-pagedjs-csp-hash.mjs` (no `--print`, before Task 2) → exited 1 with a clear "no sha256- token found" diagnostic, confirming the comparison branch is reachable
- `node ui/scripts/check-pagedjs-csp-hash.mjs` (after Task 2) → exits 0 (`OK`)
- `pnpm --dir ui build && TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test security_headers` → 4/4 tests pass (`security_headers_present`, `rate_limit_on_login`, `login_reaches_handler_with_connect_info_and_req_wrapper`, `server_serves_embedded_spa_index`)
- `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test html_page_parity` → 1/1 test passes
- `pnpm --dir ui lint` → full chain (eslint, prettier, check-tokens, check-contrast, check-focus-outline, check-pagedjs-csp-hash) passes end-to-end
- `git diff` on `crates/trackly-app/templates/*.html` → empty (D-01 respected: templates read-only, only `include_str!`'d by the new test)

## Threat Flags

None. This plan's core change (the CSP `script-src` hash-source addition) IS the threat-model mitigation for T-33-02-01/02/03/04, already fully documented in the plan's `<threat_model>` section — no new undocumented surface was introduced. `check-pagedjs-csp-hash.mjs` is zero-dependency (no new package installs, per T-33-02-SC).

## Issues Encountered

None. The `security_headers` cargo test required the `pnpm --dir ui build` prerequisite (per `verification_reality`) — ran it before the cargo test and confirmed a real, non-placeholder `ui/dist` was present.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

Plan 33-03 (on-screen preview wiring) can now wire `buildSrcdoc`/`attachBridge` into `PdfPreviewModal.svelte` knowing the LAN-mode CSP will permit the inlined Paged.js bootstrap script to execute. The hash-drift gate (`pnpm lint`) will catch any future desync if `bootstrapScript.js` or the pinned `pagedjs` version changes without the `http/mod.rs` constant being regenerated. No blockers identified.

---
*Phase: 33-print-preview-polish*
*Completed: 2026-08-04*
