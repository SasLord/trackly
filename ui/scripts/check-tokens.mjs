#!/usr/bin/env node
// [check-tokens] Постоянный CI-гейт дизайн-токенов (Phase 23, план 02, D-04).
//
// Три независимо запускаемые проверки над `ui/src`:
//   Rule 1 (old-name gate)      — старые семейства --color-*/--space-*/--radius-*/
//                                  --font-size-*/--font-weight-*/--line-height-*/--shadow-*
//                                  где-либо в файле (не только <style>).
//   Rule 2 (hex-in-style gate)  — hex-литерал внутри <style>-блока .svelte-файла.
//   Rule 3 (closed-world gate)  — var(--tr-*) ссылается на имя, реально определённое
//                                  в ui/src/styles/_tokens.scss.
//
// Zero-dependency: только node:fs/node:path. `fs.readdirSync(dir, { recursive: true })`
// требует Node >= 20.1 (CI пинит node-version: '20' через actions/setup-node@v4, что тянет
// последний патч 20.x — recursive readdir доступен; если CI когда-нибудь зафиксируют ровно
// на 20.0.0, этот вызов бросит TypeError — не проблема этого скрипта, а версии CI-раннера).
//
// Usage: node scripts/check-tokens.mjs [--rules=1,2,3] [--src=<dir>]

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');

function parseArgs(argv) {
  const args = { rules: [1, 2, 3], src: path.join(UI_ROOT, 'src') };
  for (const arg of argv) {
    if (arg.startsWith('--rules=')) {
      args.rules = arg
        .slice('--rules='.length)
        .split(',')
        .map((s) => Number.parseInt(s.trim(), 10))
        .filter((n) => Number.isInteger(n));
    } else if (arg.startsWith('--src=')) {
      args.src = path.resolve(arg.slice('--src='.length));
    } else if (arg === '--help' || arg === '-h') {
      args.help = true;
    }
  }
  return args;
}

function printHelp() {
  console.error(
    '[check-tokens] Usage: node scripts/check-tokens.mjs [--rules=1,2,3] [--src=<dir>]\n' +
      '  Rule 1: old token-family names (--color-*/--space-*/--radius-*/--font-size-*/\n' +
      '          --font-weight-*/--line-height-*/--shadow-*) anywhere in the file.\n' +
      '  Rule 2: hex literals inside <style> blocks of .svelte files.\n' +
      '  Rule 3: var(--tr-*) references not defined in ui/src/styles/_tokens.scss.',
  );
}

/** Собирает список .svelte/.scss файлов рекурсивно. Устойчив к отсутствующей директории. */
function collectSourceFiles(srcDir) {
  if (!fs.existsSync(srcDir)) return [];
  let entries;
  try {
    entries = fs.readdirSync(srcDir, { recursive: true });
  } catch {
    return [];
  }
  const files = [];
  for (const entry of entries) {
    const rel = typeof entry === 'string' ? entry : String(entry);
    if (!rel.endsWith('.svelte') && !rel.endsWith('.scss')) continue;
    const full = path.join(srcDir, rel);
    try {
      if (fs.statSync(full).isFile()) files.push(full);
    } catch {
      // race/broken symlink — skip
    }
  }
  return files;
}

function readFileSafe(filePath) {
  try {
    return fs.readFileSync(filePath, 'utf8');
  } catch {
    return null;
  }
}

// ── Rule 1: old-name gate ──────────────────────────────────────────────────────
const OLD_FAMILY_RE =
  /--(?:color|space|radius|font-size|font-weight|line-height|shadow)-[a-z0-9-]+/gi;

function lineNumberAt(content, index) {
  let line = 1;
  for (let i = 0; i < index && i < content.length; i++) {
    if (content[i] === '\n') line++;
  }
  return line;
}

function checkOldNames(files) {
  const violations = [];
  for (const filePath of files) {
    const content = readFileSafe(filePath);
    if (content == null) continue;
    for (const m of content.matchAll(OLD_FAMILY_RE)) {
      violations.push({
        file: filePath,
        line: lineNumberAt(content, m.index),
        token: m[0],
      });
    }
  }
  return violations;
}

// ── Rule 2: hex-in-<style> gate ────────────────────────────────────────────────
const STYLE_BLOCK_RE = /<style[^>]*>([\s\S]*?)<\/style>/g;
const HEX_RE = /#[0-9a-fA-F]{3,4}\b|#[0-9a-fA-F]{6}\b|#[0-9a-fA-F]{8}\b/g;

function checkHexInStyle(files) {
  const violations = [];
  for (const filePath of files) {
    if (!filePath.endsWith('.svelte')) continue;
    const content = readFileSafe(filePath);
    if (content == null) continue;
    for (const styleMatch of content.matchAll(STYLE_BLOCK_RE)) {
      const block = styleMatch[1];
      const blockStart = styleMatch.index + styleMatch[0].indexOf(block);
      for (const hexMatch of block.matchAll(HEX_RE)) {
        violations.push({
          file: filePath,
          line: lineNumberAt(content, blockStart + hexMatch.index),
          hex: hexMatch[0],
        });
      }
    }
  }
  return violations;
}

// ── Rule 3: closed-world --tr-* existence gate ─────────────────────────────────
const DEFINE_RE = /(--tr-[a-z0-9-]+)\s*:/gi;
const USE_RE = /var\((--tr-[a-z0-9-]+)/gi;

/**
 * Strips comments before Rule 3 matching. Documentation prose in _tokens.scss legitimately
 * describes token-name patterns with a `{role}` placeholder (e.g. `var(--tr-text-{role})`),
 * which truncates the char-class match at `--tr-text-` (no `{` in [a-z0-9-]) and would
 * otherwise produce a false "undefined token" violation from a comment, not real code.
 * Block comments are removed first, then `//` line comments — guarded so a URL scheme
 * prefix (http://, https://, file://) inside markup/strings is not truncated: only strips
 * `//` when NOT immediately preceded by `:`. Pragmatic heuristic (grep, not a parser),
 * matching this script's overall design philosophy.
 */
function stripCommentsForRule3(content) {
  let stripped = content.replace(/\/\*[\s\S]*?\*\//g, '');
  stripped = stripped.replace(/(^|[^:])\/\/.*$/gm, '$1');
  return stripped;
}

function checkClosedWorld(files, tokensScssPath) {
  const tokensContent = readFileSafe(tokensScssPath);
  const defined = new Set();
  if (tokensContent != null) {
    const stripped = stripCommentsForRule3(tokensContent);
    for (const m of stripped.matchAll(DEFINE_RE)) defined.add(m[1]);
  }
  const violations = [];
  for (const filePath of files) {
    const rawContent = readFileSafe(filePath);
    if (rawContent == null) continue;
    const content = stripCommentsForRule3(rawContent);
    for (const m of content.matchAll(USE_RE)) {
      const name = m[1];
      if (!defined.has(name)) {
        violations.push({
          file: filePath,
          line: lineNumberAt(content, m.index),
          token: name,
        });
      }
    }
  }
  return violations;
}

function relPath(filePath) {
  return path.relative(UI_ROOT, filePath);
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.help) {
    printHelp();
    process.exit(0);
  }

  const files = collectSourceFiles(args.src);
  let totalViolations = 0;

  if (args.rules.includes(1)) {
    const violations = checkOldNames(files);
    for (const v of violations) {
      console.error(`[check-tokens] ${relPath(v.file)}:${v.line} — old token name ${v.token}`);
    }
    totalViolations += violations.length;
  }

  if (args.rules.includes(2)) {
    const violations = checkHexInStyle(files);
    for (const v of violations) {
      console.error(`[check-tokens] ${relPath(v.file)}:${v.line} — hex literal ${v.hex}`);
    }
    totalViolations += violations.length;
  }

  if (args.rules.includes(3)) {
    const tokensScssPath = path.join(args.src, 'styles', '_tokens.scss');
    const violations = checkClosedWorld(files, tokensScssPath);
    for (const v of violations) {
      console.error(
        `[check-tokens] ${relPath(v.file)}:${v.line} — undefined token reference ${v.token}`,
      );
    }
    totalViolations += violations.length;
  }

  if (totalViolations > 0) {
    console.error(`[check-tokens] FAIL — ${totalViolations} нарушений`);
    process.exit(1);
  }

  console.error('[check-tokens] PASS — 0 нарушений');
  process.exit(0);
}

main();
