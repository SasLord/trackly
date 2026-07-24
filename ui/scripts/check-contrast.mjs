#!/usr/bin/env node
// [check-contrast] Постоянный CI-гейт WCAG AA-контраста (Phase 30, план 01, D-01).
//
// Читает ЕДИНСТВЕННЫЙ файл — ui/src/styles/_tokens.scss — и проверяет закрытую
// каноническую таблицу из 43 пар (foreground token / background token / порог) × 2 темы
// (86 проверок за прогон) на соответствие WCAG 2.x. Формула контраста реализована с нуля:
// в проекте нет npm-зависимости для WCAG-математики (30-PATTERNS.md → «No Analog Found»).
//
// Порог 4.5:1 — обычный текст (37 пар). Порог 3.0:1 — документированное исключение
// для --tr-text-tertiary (роль «плейсхолдеры/мета/счётчики» по 23-UI-SPEC.md:81, не
// основной текст — 6 пар).
//
// Rgba/rgb/hsl/hsla-токены (--tr-*-soft, --tr-focus-ring, --tr-danger-ring, --tr-overlay,
// --tr-row-selected) сознательно НЕ входят в каноническую таблицу — альфа-композитинг
// поверх фактического фона вне периметра этого скрипта; остаточный риск закрывается
// финальным ручным UAT (план 30-03). --tr-text-disabled тоже не входит — WCAG 1.4.3
// явно исключает неактивные UI-элементы.
//
// Zero-dependency: только node:fs/node:path/node:url.
//
// Usage: node scripts/check-contrast.mjs

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');

function readFileSafe(filePath) {
  try {
    return fs.readFileSync(filePath, 'utf8');
  } catch {
    return null;
  }
}

function relPath(filePath) {
  return path.relative(UI_ROOT, filePath);
}

// ── Theme-block extraction (brace-depth counting — defensive against future nesting) ───
function extractThemeBlock(content, themeMarker) {
  const markerIdx = content.indexOf(themeMarker);
  if (markerIdx === -1) return null;
  const openIdx = content.indexOf('{', markerIdx);
  if (openIdx === -1) return null;
  let depth = 0;
  let i = openIdx;
  for (; i < content.length; i++) {
    if (content[i] === '{') depth++;
    else if (content[i] === '}') {
      depth--;
      if (depth === 0) break;
    }
  }
  return content.slice(openIdx + 1, i);
}

const TOKEN_DEF_RE = /(--tr-[a-z0-9-]+)\s*:\s*([^;]+);/g;

function parseTokenBlock(blockText) {
  const map = new Map();
  if (blockText == null) return map;
  for (const m of blockText.matchAll(TOKEN_DEF_RE)) {
    map.set(m[1], m[2].trim());
  }
  return map;
}

// ── WCAG 2.x relative luminance + contrast ratio, from scratch ─────────────────────────
const HEX6_RE = /^#[0-9a-fA-F]{6}$/;

function hexToRgb(hex) {
  const n = Number.parseInt(hex.slice(1), 16);
  return { r: (n >> 16) & 255, g: (n >> 8) & 255, b: n & 255 };
}

function srgbChannelToLinear(c) {
  const cs = c / 255;
  return cs <= 0.03928 ? cs / 12.92 : ((cs + 0.055) / 1.055) ** 2.4;
}

function relativeLuminance({ r, g, b }) {
  const R = srgbChannelToLinear(r);
  const G = srgbChannelToLinear(g);
  const B = srgbChannelToLinear(b);
  return 0.2126 * R + 0.7152 * G + 0.0722 * B;
}

function contrastRatio(hexA, hexB) {
  const lA = relativeLuminance(hexToRgb(hexA));
  const lB = relativeLuminance(hexToRgb(hexB));
  const lighter = Math.max(lA, lB);
  const darker = Math.min(lA, lB);
  return (lighter + 0.05) / (darker + 0.05);
}

// ── Canonical pair table (43 pairs) — hardcoded, no CLI parameters ─────────────────────
const BG_6 = [
  '--tr-bg',
  '--tr-surface',
  '--tr-surface-raised',
  '--tr-surface-sunken',
  '--tr-row-hover',
  '--tr-group',
];
const BG_4 = ['--tr-bg', '--tr-surface', '--tr-surface-raised', '--tr-surface-sunken'];
const ON_ACCENT_BGS = ['--tr-accent', '--tr-danger', '--tr-success', '--tr-warning', '--tr-info'];

const PAIRS = [
  ...BG_6.map((bg) => ({ fg: '--tr-text-primary', bg, threshold: 4.5 })),
  ...BG_6.map((bg) => ({ fg: '--tr-text-secondary', bg, threshold: 4.5 })),
  ...BG_4.map((bg) => ({ fg: '--tr-accent-text', bg, threshold: 4.5 })),
  ...BG_4.map((bg) => ({ fg: '--tr-success-text', bg, threshold: 4.5 })),
  ...BG_4.map((bg) => ({ fg: '--tr-warning-text', bg, threshold: 4.5 })),
  ...BG_4.map((bg) => ({ fg: '--tr-danger-text', bg, threshold: 4.5 })),
  ...BG_4.map((bg) => ({ fg: '--tr-info-text', bg, threshold: 4.5 })),
  ...ON_ACCENT_BGS.map((bg) => ({ fg: '--tr-on-accent', bg, threshold: 4.5 })),
  ...BG_6.map((bg) => ({ fg: '--tr-text-tertiary', bg, threshold: 3.0 })),
];

function main() {
  const tokensScssPath = path.join(UI_ROOT, 'src', 'styles', '_tokens.scss');
  const content = readFileSafe(tokensScssPath);
  if (content == null) {
    console.error(`[check-contrast] cannot read ${relPath(tokensScssPath)}`);
    process.exit(1);
  }

  const themeBlocks = {
    light: extractThemeBlock(content, "[data-theme='light']"),
    dark: extractThemeBlock(content, "[data-theme='dark']"),
  };

  let totalViolations = 0;

  for (const theme of ['light', 'dark']) {
    const map = parseTokenBlock(themeBlocks[theme]);
    for (const pair of PAIRS) {
      const fgValue = map.get(pair.fg);
      const bgValue = map.get(pair.bg);

      if (fgValue == null || bgValue == null) {
        const missing = fgValue == null ? pair.fg : pair.bg;
        console.error(
          `[check-contrast] ${theme} ${pair.fg} on ${pair.bg}: token ${missing} not found in _tokens.scss`,
        );
        totalViolations++;
        continue;
      }

      if (!HEX6_RE.test(fgValue) || !HEX6_RE.test(bgValue)) {
        console.error(
          `[check-contrast] ${theme} ${pair.fg} on ${pair.bg}: non-hex value not supported by contrast checker`,
        );
        totalViolations++;
        continue;
      }

      const ratio = contrastRatio(fgValue, bgValue);
      if (ratio < pair.threshold) {
        console.error(
          `[check-contrast] ${theme} ${pair.fg} on ${pair.bg}: ratio ${ratio.toFixed(2)} < threshold ${pair.threshold}`,
        );
        totalViolations++;
      }
    }
  }

  if (totalViolations > 0) {
    console.error(`[check-contrast] FAIL — ${totalViolations} нарушений`);
    process.exit(1);
  }

  console.error('[check-contrast] PASS — 0 нарушений');
  process.exit(0);
}

main();
