#!/usr/bin/env node
// [check-contrast] Постоянный CI-гейт WCAG AA-контраста (Phase 30, план 01, D-01).
//
// Читает ЕДИНСТВЕННЫЙ файл — ui/src/styles/_tokens.scss — и проверяет закрытую
// каноническую таблицу из 51 пары (foreground token / background token / порог) × 2 темы
// (102 проверки за прогон) на соответствие WCAG 2.x. Формула контраста реализована с нуля:
// в проекте нет npm-зависимости для WCAG-математики (30-PATTERNS.md → «No Analog Found»).
//
// Порог 4.5:1 — обычный текст (45 пар, в том числе приглушённые --tr-warning-muted-text и
// --tr-danger-muted-text на четырёх фонах). Порог 3.0:1 — документированное исключение
// для --tr-text-tertiary (роль «плейсхолдеры/мета/счётчики» по 23-UI-SPEC.md:81, не
// основной текст — 6 пар).
//
// Приглушённые soft-фоны (--tr-warning-muted-soft, --tr-danger-muted-soft) — rgba и в таблицу
// контраста НЕ входят, но участвуют в относительном правиле R2 по альфе (см. ниже).
//
// Относительные правила (UI-SPEC §10.2, решение 2026-10-17: «приглушённее» — различие ВНУТРИ
// пары, а не уход ниже порога контраста), для warning и danger в каждой теме:
//   R1: контраст *-muted-text к --tr-surface не больше (контраст активного *-text к --tr-surface
//       минус MIN_CONTRAST_GAP);
//   R2: альфа *-muted-soft не больше MAX_ALPHA_RATIO от альфы активного *-soft.
// Правила падают на нарушающих значениях, а не описывают подобранные.
//
// Rgba/rgb/hsl/hsla-токены (--tr-*-soft, --tr-focus-ring, --tr-danger-ring, --tr-overlay,
// --tr-row-selected) сознательно НЕ входят в каноническую таблицу — альфа-композитинг
// поверх фактического фона вне периметра этого скрипта; остаточный риск закрывается
// финальным ручным UAT (план 30-03). --tr-text-disabled тоже не входит — WCAG 1.4.3
// явно исключает неактивные UI-элементы.
//
// Zero-dependency: только node:fs/node:path/node:url.
//
// Usage: node scripts/check-contrast.mjs [--tokens=<путь к копии _tokens.scss>] | --selftest

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');

// Окно светлой warning-пары к --tr-surface: от ≈5.53 (порог 4.5 к --tr-surface-sunken при
// отношении 0.815) до ≈5.73 (активный 5.83 минус зазор) — шириной ≈0.21; зазор больше 0.1
// сделал бы пару нерешаемой. Текст почти неразличим, поэтому потолок альфы 0.65 заставляет
// различие быть видимым, а не выводиться расчётом.
const MIN_CONTRAST_GAP = 0.1;
const MAX_ALPHA_RATIO = 0.65;
const EPS = 1e-9;

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

// ── Canonical pair table (51 pairs) — hardcoded, no CLI parameters ─────────────────────
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
  ...BG_4.map((bg) => ({ fg: '--tr-warning-muted-text', bg, threshold: 4.5 })),
  ...BG_4.map((bg) => ({ fg: '--tr-danger-muted-text', bg, threshold: 4.5 })),
  ...BG_4.map((bg) => ({ fg: '--tr-info-text', bg, threshold: 4.5 })),
  ...ON_ACCENT_BGS.map((bg) => ({ fg: '--tr-on-accent', bg, threshold: 4.5 })),
  ...BG_6.map((bg) => ({ fg: '--tr-text-tertiary', bg, threshold: 3.0 })),
];

const RGBA_RE = /^rgba\(\s*\d+\s*,\s*\d+\s*,\s*\d+\s*,\s*([0-9.]+)\s*\)$/;

// Относительные пары: приглушённый токен текста / активный токен текста / фон / активный soft.
const MUTED_TONES = [
  {
    name: 'warning',
    mutedText: '--tr-warning-muted-text',
    activeText: '--tr-warning-text',
    mutedSoft: '--tr-warning-muted-soft',
    activeSoft: '--tr-warning-soft',
  },
  {
    name: 'danger',
    mutedText: '--tr-danger-muted-text',
    activeText: '--tr-danger-text',
    mutedSoft: '--tr-danger-muted-soft',
    activeSoft: '--tr-danger-soft',
  },
];

// Возвращает список строк-нарушений для содержимого _tokens.scss (без побочных эффектов).
function checkTokens(content) {
  const violations = [];
  const themeBlocks = {
    light: extractThemeBlock(content, "[data-theme='light']"),
    dark: extractThemeBlock(content, "[data-theme='dark']"),
  };

  for (const theme of ['light', 'dark']) {
    const map = parseTokenBlock(themeBlocks[theme]);
    for (const pair of PAIRS) {
      const fgValue = map.get(pair.fg);
      const bgValue = map.get(pair.bg);

      if (fgValue == null || bgValue == null) {
        const missing = fgValue == null ? pair.fg : pair.bg;
        violations.push(
          `${theme} ${pair.fg} on ${pair.bg}: token ${missing} not found in _tokens.scss`,
        );
        continue;
      }

      if (!HEX6_RE.test(fgValue) || !HEX6_RE.test(bgValue)) {
        violations.push(
          `${theme} ${pair.fg} on ${pair.bg}: non-hex value not supported by contrast checker`,
        );
        continue;
      }

      const ratio = contrastRatio(fgValue, bgValue);
      if (ratio < pair.threshold) {
        violations.push(
          `${theme} ${pair.fg} on ${pair.bg}: ratio ${ratio.toFixed(2)} < threshold ${pair.threshold}`,
        );
      }
    }

    // R1 / R2: приглушённое строго «тише» активного.
    const surface = map.get('--tr-surface');
    for (const t of MUTED_TONES) {
      const mutedText = map.get(t.mutedText);
      const activeText = map.get(t.activeText);
      if (
        surface == null ||
        mutedText == null ||
        activeText == null ||
        !HEX6_RE.test(surface) ||
        !HEX6_RE.test(mutedText) ||
        !HEX6_RE.test(activeText)
      ) {
        violations.push(
          `${theme} R1 ${t.mutedText}: token ${t.mutedText}, ${t.activeText} or --tr-surface not found or not hex`,
        );
      } else {
        const mutedC = contrastRatio(mutedText, surface);
        const activeC = contrastRatio(activeText, surface);
        if (mutedC > activeC - MIN_CONTRAST_GAP + EPS) {
          violations.push(
            `${theme} R1 ${t.mutedText} (${mutedC.toFixed(2)}) vs ${t.activeText} (${activeC.toFixed(2)}) on --tr-surface: muted must be lower by >= ${MIN_CONTRAST_GAP}`,
          );
        }
      }

      const mutedSoft = RGBA_RE.exec(map.get(t.mutedSoft) ?? '');
      const activeSoft = RGBA_RE.exec(map.get(t.activeSoft) ?? '');
      if (!mutedSoft || !activeSoft) {
        violations.push(
          `${theme} R2 ${t.mutedSoft}: token ${t.mutedSoft} or ${t.activeSoft} not found or not rgba()`,
        );
      } else {
        const mutedA = Number.parseFloat(mutedSoft[1]);
        const activeA = Number.parseFloat(activeSoft[1]);
        if (!(activeA > 0) || mutedA > MAX_ALPHA_RATIO * activeA + EPS) {
          violations.push(
            `${theme} R2 ${t.mutedSoft} alpha ${mutedA} vs ${t.activeSoft} alpha ${activeA}: ratio must be <= ${MAX_ALPHA_RATIO}`,
          );
        }
      }
    }
  }
  return violations;
}

function report(violations) {
  for (const v of violations) console.error(`[check-contrast] ${v}`);
  if (violations.length > 0) {
    console.error(`[check-contrast] FAIL — ${violations.length} нарушений`);
    return 1;
  }
  console.error('[check-contrast] PASS — 0 нарушений');
  return 0;
}

// ── Самотест: мутанты КОПИИ реального _tokens.scss; якорь обязан встречаться ровно один раз ──
function mutate(content, anchor, replacement) {
  const count = content.split(anchor).length - 1;
  if (count !== 1) {
    console.error(
      `[check-contrast] selftest: якорь ${JSON.stringify(anchor)} встречается ${count} раз(а), ожидалось ровно 1`,
    );
    process.exit(1);
  }
  return content.replace(anchor, () => replacement);
}

function selftest(content) {
  const real = checkTokens(content);
  if (real.length > 0) {
    console.error('[check-contrast] selftest: реальные токены не проходят гейт:');
    for (const v of real) console.error(`  ${v}`);
    process.exit(1);
  }
  const mutants = [
    {
      name: '(a) тёмный warning-muted-text равен активному',
      make: (c) =>
        mutate(c, '--tr-warning-muted-text: #d39e4f;', '--tr-warning-muted-text: #f0b45a;'),
      expect: 'R1',
    },
    {
      name: '(b) светлый danger-muted-text низкоконтрастный',
      make: (c) =>
        mutate(c, '--tr-danger-muted-text: #b43939;', '--tr-danger-muted-text: #c8cfdb;'),
      expect: 'threshold 4.5',
    },
    {
      name: '(c) в тёмном блоке нет warning-muted-text',
      make: (c) => mutate(c, '  --tr-warning-muted-text: #d39e4f;\n', ''),
      expect: 'not found',
    },
    {
      name: '(d) альфа тёмного danger-muted-soft равна активной',
      make: (c) =>
        mutate(
          c,
          '--tr-danger-muted-soft: rgba(242, 101, 101, 0.08);',
          '--tr-danger-muted-soft: rgba(242, 101, 101, 0.16);',
        ),
      expect: 'R2',
    },
    {
      name: '(e) светлый warning-muted-text контрастнее активного (перестановка)',
      make: (c) =>
        mutate(
          mutate(c, '--tr-warning-muted-text: #915c0f;', '--tr-warning-muted-text: #8f590a;'),
          '--tr-warning-text: #8f590a;',
          '--tr-warning-text: #915c0f;',
        ),
      expect: 'R1',
    },
    {
      name: '(f) тёмный danger-muted-text ниже активного лишь на 0.05',
      make: (c) =>
        mutate(c, '--tr-danger-muted-text: #e07171;', '--tr-danger-muted-text: #ff7f7f;'),
      expect: 'R1',
    },
    {
      name: '(g) альфа тёмного warning-muted-soft = 0.7 от активной',
      make: (c) =>
        mutate(
          c,
          '--tr-warning-muted-soft: rgba(229, 161, 58, 0.08);',
          '--tr-warning-muted-soft: rgba(229, 161, 58, 0.112);',
        ),
      expect: 'R2',
    },
  ];
  let failed = 0;
  for (const m of mutants) {
    const v = checkTokens(m.make(content));
    if (!v.some((line) => line.includes(m.expect))) {
      console.error(`[check-contrast] selftest FAIL ${m.name}: гейт не покраснел на «${m.expect}»`);
      failed++;
    } else {
      console.error(`[check-contrast] selftest ok ${m.name}`);
    }
  }
  if (failed > 0) process.exit(1);
  console.error(`[check-contrast] selftest PASS — ${mutants.length} мутантов поймано`);
  process.exit(0);
}

function main() {
  const args = process.argv.slice(2);
  const tokensArg = args.find((a) => a.startsWith('--tokens='));
  const tokensScssPath = tokensArg
    ? path.resolve(tokensArg.slice('--tokens='.length))
    : path.join(UI_ROOT, 'src', 'styles', '_tokens.scss');
  const content = readFileSafe(tokensScssPath);
  if (content == null) {
    console.error(`[check-contrast] cannot read ${relPath(tokensScssPath)}`);
    process.exit(1);
  }
  if (args.includes('--selftest')) selftest(content);
  process.exit(report(checkTokens(content)));
}

main();
