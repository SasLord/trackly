#!/usr/bin/env node
// [check-pagedjs-csp-hash] Phase 33 Plan 02 (D-14): CSP hash-drift detection gate.
//
// Recomputes the sha256 CSP hash-source over the exact frozen concatenation
// formula from `ui/src/lib/pdfPreview/pagedPreviewBootstrap.ts`
// (`pagedjsLibraryText + ';\n' + bootstrapText`) and compares it against the
// hardcoded `'sha256-<digest>'` token in `crates/trackly-app/src/http/mod.rs`'s
// `script-src` directive. If a `pagedjs` version bump or a hand-edit to
// `bootstrapScript.js` changes the combined bytes without the Rust constant
// being regenerated, LAN-mode preview pagination silently breaks (the inline
// bootstrap <script> gets blocked by CSP) — this gate turns that into a loud
// `pnpm lint` failure instead.
//
// Zero-dependency: only node:crypto/node:fs/node:path/node:url, matching the
// existing check-tokens.mjs convention.
//
// Usage:
//   node scripts/check-pagedjs-csp-hash.mjs           # verify against http/mod.rs
//   node scripts/check-pagedjs-csp-hash.mjs --print    # print the computed hash and exit 0

import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');

const PAGEDJS_BUNDLE_PATH = path.join(UI_ROOT, 'node_modules/pagedjs/dist/paged.min.js');
const BOOTSTRAP_SCRIPT_PATH = path.join(UI_ROOT, 'src/lib/pdfPreview/bootstrapScript.js');
const HTTP_MOD_RS_PATH = path.resolve(UI_ROOT, '../crates/trackly-app/src/http/mod.rs');

const SHA256_TOKEN_RE = /'sha256-[A-Za-z0-9+/=]+'/;

function computeHash() {
  const libraryText = fs.readFileSync(PAGEDJS_BUNDLE_PATH, 'utf8');
  const bootstrapText = fs.readFileSync(BOOTSTRAP_SCRIPT_PATH, 'utf8');
  // Must match pagedPreviewBootstrap.ts's PAGED_PREVIEW_INLINE_SCRIPT formula
  // byte for byte — do not change the concatenation order or add characters.
  const combined = libraryText + ';\n' + bootstrapText;
  // CSP hash-sources use base64 of the raw SHA-256 digest, NOT hex.
  const digest = crypto.createHash('sha256').update(combined, 'utf8').digest('base64');
  return `sha256-${digest}`;
}

// [Plan 36-04 gap-closure] Structural guard against ES5-pseudo-inheritance
// regressing the RepeatTableHeadHandler back in. `window.PagedModule.Handler`
// is a native ES6 class in the bundled paged.min.js UMD build; invoking it via
// `Handler.call(this, ...)` throws `TypeError: Cannot call a class
// constructor ... without |new|` at Previewer-construction time, which the
// D-02 degrade path silently swallows into an unpaginated fallback (no page
// chrome at all — this exact regression shipped once already and was only
// caught by live desktop UAT, not by the hash check above, which only proves
// the bytes are IN SYNC, not that they're CORRECT). Cheap structural check,
// reuses the bootstrapText already read for the hash above — deliberately
// not a separate script/lint-step to avoid duplicating the file-read wiring.
function checkHandlerIsNativeClass(bootstrapText) {
  const violations = [];
  if (/Handler\s*\.\s*call\s*\(/.test(bootstrapText)) {
    violations.push(
      'found `Handler.call(...)` — a native ES6 class constructor cannot be invoked via ' +
        '.call(), that throws at runtime (see comment above RepeatTableHeadHandler in ' +
        'bootstrapScript.js for the full incident writeup)',
    );
  }
  if (
    /Object\s*\.\s*create\s*\(\s*window\.PagedModule\.Handler\.prototype\s*\)/.test(bootstrapText)
  ) {
    violations.push(
      'found `Object.create(window.PagedModule.Handler.prototype)` — ES5 pseudo-inheritance ' +
        'cannot extend a native ES6 class',
    );
  }
  if (
    !/class\s+RepeatTableHeadHandler\s+extends\s+window\.PagedModule\.Handler\b/.test(bootstrapText)
  ) {
    violations.push(
      'did not find `class RepeatTableHeadHandler extends window.PagedModule.Handler` — the ' +
        'handler must be a native ES6 class (native class syntax is safe in this file: it is ' +
        'imported with ?raw and never transpiled, see pagedPreviewBootstrap.ts)',
    );
  }
  return violations;
}

function main() {
  const printOnly = process.argv.includes('--print');
  const hash = computeHash();

  if (printOnly) {
    console.log(hash);
    process.exit(0);
  }

  const bootstrapText = fs.readFileSync(BOOTSTRAP_SCRIPT_PATH, 'utf8');
  const classViolations = checkHandlerIsNativeClass(bootstrapText);
  if (classViolations.length > 0) {
    console.error(
      '[check-pagedjs-csp-hash] FAIL — RepeatTableHeadHandler is not a native ES6 class:',
    );
    for (const v of classViolations) {
      console.error(`[check-pagedjs-csp-hash]   - ${v}`);
    }
    process.exit(1);
  }

  const httpModRsContent = fs.readFileSync(HTTP_MOD_RS_PATH, 'utf8');
  const match = httpModRsContent.match(SHA256_TOKEN_RE);
  const currentToken = match ? match[0].slice(1, -1) : null; // strip surrounding quotes

  if (currentToken === null) {
    console.error(
      '[check-pagedjs-csp-hash] FAIL — no sha256- token found in ' +
        `${path.relative(UI_ROOT, HTTP_MOD_RS_PATH)}'s script-src directive.`,
    );
    console.error(`[check-pagedjs-csp-hash] Computed hash: ${hash}`);
    console.error(
      '[check-pagedjs-csp-hash] Run `node scripts/check-pagedjs-csp-hash.mjs --print` and add ' +
        `the printed value as a 'sha256-<digest>' source in script-src in ` +
        `${path.relative(UI_ROOT, HTTP_MOD_RS_PATH)}.`,
    );
    process.exit(1);
  }

  if (currentToken !== hash) {
    console.error('[check-pagedjs-csp-hash] FAIL — CSP hash drift detected.');
    console.error(`[check-pagedjs-csp-hash]   Computed: ${hash}`);
    console.error(`[check-pagedjs-csp-hash]   In http/mod.rs: ${currentToken}`);
    console.error(
      '[check-pagedjs-csp-hash] Run `node scripts/check-pagedjs-csp-hash.mjs --print` and update ' +
        `the constant in ${path.relative(UI_ROOT, HTTP_MOD_RS_PATH)}.`,
    );
    process.exit(1);
  }

  console.error('[check-pagedjs-csp-hash] OK');
  process.exit(0);
}

main();
