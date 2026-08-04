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

function main() {
  const printOnly = process.argv.includes('--print');
  const hash = computeHash();

  if (printOnly) {
    console.log(hash);
    process.exit(0);
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
