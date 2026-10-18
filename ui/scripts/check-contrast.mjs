#!/usr/bin/env node
// [check-contrast] Постоянный CI-гейт WCAG AA-контраста (Phase 30, план 01, D-01; Phase 41.11, план 03).
//
// Читает ЕДИНСТВЕННЫЙ файл — ui/src/styles/_tokens.scss — и делает две независимые вещи.
//
// (1) Каноническая таблица: 51 пара (foreground token / background token / порог) × 2 темы =
//     102 проверки за прогон на «голых» фонах. Формула контраста WCAG 2.x реализована с нуля:
//     в проекте нет npm-зависимости для WCAG-математики. Порог 4.5:1 — обычный текст
//     (45 пар, включая приглушённые --tr-warning-muted-text и --tr-danger-muted-text на четырёх
//     фонах). Порог 3.0:1 — документированное исключение для --tr-text-tertiary (роль
//     «плейсхолдеры/мета/счётчики», не основной текст — 6 пар).
//
// (2) R1 — композит. Чип рисуется не на голом фоне, а на полупрозрачной подложке *-soft, поэтому
//     контраст текста чипа меряется на КОМПОЗИТЕ: source-over round(fg*alpha + bg*(1-alpha)) по
//     каналам в закодированном sRGB (так рисует браузер), затем те же relativeLuminance и
//     contrastRatio. Набор фонов — шесть: --tr-bg, --tr-surface, --tr-surface-raised,
//     --tr-surface-sunken, --tr-row-hover, --tr-group. Набор тонов — все пять мягких
//     (accent, success, warning, danger, info) в варианте active (*-text на *-soft) плюс
//     warning и danger в варианте muted (*-muted-text на *-muted-soft):
//     (5 + 2) x 6 фонов x 2 темы = 60 + 24 = 84 композитные проверки за прогон
//     (60 active + 24 muted). Порог — COMPOSITE_MIN = 4.5. Требование порядка «muted тише
//     active» снято (D-15): различие внутри пары несёт нецветовая обводка (план 09, D-16),
//     цвет его не гарантирует.
//
// (3) R2: альфа *-muted-soft не больше MAX_ALPHA_RATIO (0.65) от альфы активного *-soft.
//
// ГРАНИЦА ОХВАТА (D-33): --tr-row-selected (rgba) в набор фонов НЕ входит и остаётся ВНЕ
// ПЕРИМЕТРА этого гейта — чип поверх выбранной строки здесь не измеряется; остаточный риск
// закрывается ручной проверкой (план 41.11-11), а не молчанием. Остальные rgba/hsl-токены
// (--tr-focus-ring, --tr-danger-ring, --tr-overlay) в таблицу и в композит тоже не входят.
// --tr-text-disabled не входит — WCAG 1.4.3 исключает неактивные UI-элементы.
//
// Zero-dependency: только node:fs/node:path/node:url.
//
// Usage: node scripts/check-contrast.mjs [--tokens=<путь к копии _tokens.scss>] | --selftest [--dump-mutants=<каталог>]

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');

// Порог R1 на композите подложки (D-15). Токены держатся с запасом ≥ 4.6 (D-29), чтобы
// округление канала (≈0.02) не отправляло значение под границу.
const COMPOSITE_MIN = 4.5;
// Потолок альфы приглушённой подложки относительно активной (R2, D-17).
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

function rgbToHex({ r, g, b }) {
  return `#${[r, g, b].map((c) => c.toString(16).padStart(2, '0')).join('')}`;
}

// Source-over композит fg(alpha) поверх непрозрачного bgHex: round(fg*alpha + bg*(1-alpha))
// на канал в закодированном sRGB. Округление до целого канала — осознанно (так рисует
// браузер); оно сдвигает контраст до ≈0.02, что значимо у границы 4.5 (D-14).
function alphaComposite({ r, g, b }, alpha, bgHex) {
  const bg = hexToRgb(bgHex);
  const mix = (f, k) => Math.round(f * alpha + k * (1 - alpha));
  return { r: mix(r, bg.r), g: mix(g, bg.g), b: mix(b, bg.b) };
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

const RGBA_RE = /^rgba\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*,\s*([0-9.]+)\s*\)$/;

// Композитные тона (D-33): все пять мягких. variants — active всегда, muted у warning/danger.
const COMPOSITE_TONES = [
  { name: 'accent', active: ['--tr-accent-text', '--tr-accent-soft'] },
  { name: 'success', active: ['--tr-success-text', '--tr-success-soft'] },
  {
    name: 'warning',
    active: ['--tr-warning-text', '--tr-warning-soft'],
    muted: ['--tr-warning-muted-text', '--tr-warning-muted-soft'],
  },
  {
    name: 'danger',
    active: ['--tr-danger-text', '--tr-danger-soft'],
    muted: ['--tr-danger-muted-text', '--tr-danger-muted-soft'],
  },
  { name: 'info', active: ['--tr-info-text', '--tr-info-soft'] },
];

// Пары альф для R2 (muted не плотнее 0.65 от active).
const MUTED_TONES = [
  { name: 'warning', mutedSoft: '--tr-warning-muted-soft', activeSoft: '--tr-warning-soft' },
  { name: 'danger', mutedSoft: '--tr-danger-muted-soft', activeSoft: '--tr-danger-soft' },
];

// Возвращает список строк-нарушений для содержимого _tokens.scss (без побочных эффектов).
// stats (необязательный) получает: compositeCells, minComposite (+ где).
function checkTokens(content, stats = {}) {
  const violations = [];
  stats.compositeCells = 0;
  stats.minComposite = Infinity;
  stats.minWhere = '';
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

    // R1: контраст текста чипа на КОМПОЗИТЕ подложки поверх каждого из шести фонов.
    for (const t of COMPOSITE_TONES) {
      for (const variant of ['active', 'muted']) {
        const pair = t[variant];
        if (!pair) continue;
        const [textTok, softTok] = pair;
        const textHex = map.get(textTok);
        const softM = RGBA_RE.exec(map.get(softTok) ?? '');
        if (textHex == null || !HEX6_RE.test(textHex) || !softM) {
          violations.push(
            `${theme} R1 composite ${t.name} ${variant}: token ${textTok} not found or not hex, or ${softTok} not found or not rgba()`,
          );
          continue;
        }
        const fg = { r: Number(softM[1]), g: Number(softM[2]), b: Number(softM[3]) };
        const alpha = Number.parseFloat(softM[4]);
        for (const bgTok of BG_6) {
          const bgHex = map.get(bgTok);
          if (bgHex == null || !HEX6_RE.test(bgHex)) {
            violations.push(
              `${theme} R1 composite ${t.name} ${variant} over ${bgTok}: background token not found or not hex`,
            );
            continue;
          }
          const comp = rgbToHex(alphaComposite(fg, alpha, bgHex));
          const ratio = contrastRatio(textHex, comp);
          stats.compositeCells++;
          if (ratio < stats.minComposite) {
            stats.minComposite = ratio;
            stats.minWhere = `${theme} ${t.name} ${variant} over ${bgTok}`;
          }
          if (ratio < COMPOSITE_MIN) {
            violations.push(
              `${theme} R1 composite ${t.name} ${variant} over ${bgTok} (${textTok} on ${softTok}): ratio ${ratio.toFixed(2)} < ${COMPOSITE_MIN}`,
            );
          }
        }
      }
    }

    // R2: альфа приглушённой подложки не больше MAX_ALPHA_RATIO от активной.
    for (const t of MUTED_TONES) {
      const mutedSoft = RGBA_RE.exec(map.get(t.mutedSoft) ?? '');
      const activeSoft = RGBA_RE.exec(map.get(t.activeSoft) ?? '');
      if (!mutedSoft || !activeSoft) {
        violations.push(
          `${theme} R2 ${t.mutedSoft}: token ${t.mutedSoft} or ${t.activeSoft} not found or not rgba()`,
        );
      } else {
        const mutedA = Number.parseFloat(mutedSoft[4]);
        const activeA = Number.parseFloat(activeSoft[4]);
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

function report(violations, stats) {
  for (const v of violations) console.error(`[check-contrast] ${v}`);
  const minC = Number.isFinite(stats.minComposite) ? stats.minComposite.toFixed(2) : 'n/a';
  if (violations.length > 0) {
    console.error(
      `[check-contrast] FAIL — ${violations.length} нарушений (минимум композита ${minC}: ${stats.minWhere})`,
    );
    return 1;
  }
  console.error(
    `[check-contrast] PASS — 0 нарушений: 102 проверки таблицы + ${stats.compositeCells} композитных, минимум композита ${minC} (${stats.minWhere})`,
  );
  return 0;
}

// ── Самотест: мутанты КОПИИ реального _tokens.scss; якорь обязан встречаться ровно один раз ──
// Якорь включает ЗНАЧЕНИЕ токена: правишь токен — обновляй якорь вместе с ним (D-26).
function mutate(content, anchor, replacement) {
  const count = content.split(anchor).length - 1;
  if (count !== 1) {
    console.error(
      `[check-contrast] selftest: якорь ${JSON.stringify(anchor)} встречается ${count} раз(а), ожидалось ровно 1 — обнови якорь вместе с токеном (D-26), а не ослабляй проверку`,
    );
    process.exit(1);
  }
  return content.replace(anchor, () => replacement);
}

function selftest(content, dumpDir) {
  const real = checkTokens(content);
  if (real.length > 0) {
    console.error('[check-contrast] selftest: реальные токены не проходят гейт:');
    for (const v of real) console.error(`  ${v}`);
    process.exit(1);
  }
  console.error('[check-contrast] selftest ok (контроль) немутированные токены — 0 нарушений');
  // Мутанты ПОРЧАТ композит (или другое правило) при сохранённом контрасте на голом фоне.
  // onlyBg: каждое композитное нарушение мутанта обязано называть именно этот фон.
  const mutants = [
    {
      name: '(b) светлый danger-muted-text низкоконтрастный',
      make: (c) =>
        mutate(c, '--tr-danger-muted-text: #af3737;', '--tr-danger-muted-text: #c8cfdb;'),
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
      name: '(g) альфа тёмного warning-muted-soft = 0.7 от активной',
      make: (c) =>
        mutate(
          c,
          '--tr-warning-muted-soft: rgba(229, 161, 58, 0.08);',
          '--tr-warning-muted-soft: rgba(229, 161, 58, 0.112);',
        ),
      expect: 'R2',
    },
    {
      name: '(1) светлая тема: плотнее warning-soft — композит warning active падает',
      make: (c) =>
        mutate(
          c,
          '--tr-warning-soft: rgba(185, 114, 12, 0.14);',
          '--tr-warning-soft: rgba(185, 114, 12, 0.3);',
        ),
      expect: 'light R1 composite warning active over --tr-surface-sunken',
    },
    {
      name: '(2) тёмная тема: danger-muted-text возвращён к #e07171 (композит на row-hover ≈4.10)',
      make: (c) =>
        mutate(c, '--tr-danger-muted-text: #e37f7f;', '--tr-danger-muted-text: #e07171;'),
      expect: 'dark R1 composite danger muted over --tr-row-hover',
    },
    {
      name: '(3) светлая тема: тёмный --tr-group — краснеет только шестой фон',
      make: (c) =>
        // --tr-text-tertiary темнеет вместе с фоном, чтобы голая пара (порог 3.0) осталась зелёной
        mutate(
          mutate(c, '--tr-group: #e9edf5;', '--tr-group: #dde2ec;'),
          '--tr-text-tertiary: #7a849a;',
          '--tr-text-tertiary: #6a7488;',
        ),
      expect: 'over --tr-group',
      onlyBg: '--tr-group',
    },
    {
      name: '(4) D-33: светлый success-text возвращён к #0f7343 (композит ≈4.20 на sunken)',
      make: (c) => mutate(c, '--tr-success-text: #0e6c3f;', '--tr-success-text: #0f7343;'),
      expect: 'light R1 composite success active over --tr-surface-sunken',
    },
    {
      name: '(5a) D-33: плотнее accent-soft — accent реально измеряется',
      make: (c) =>
        mutate(
          c,
          '--tr-accent-soft: rgba(43, 95, 217, 0.1);',
          '--tr-accent-soft: rgba(43, 95, 217, 0.4);',
        ),
      expect: 'light R1 composite accent active over',
    },
    {
      name: '(5b) D-33: плотнее info-soft — info реально измеряется',
      make: (c) =>
        mutate(
          c,
          '--tr-info-soft: rgba(43, 95, 217, 0.1);',
          '--tr-info-soft: rgba(43, 95, 217, 0.4);',
        ),
      expect: 'light R1 composite info active over',
    },
  ];
  let failed = 0;
  for (const [i, m] of mutants.entries()) {
    const mutated = m.make(content);
    if (dumpDir) fs.writeFileSync(path.join(dumpDir, `mutant-${i}.scss`), mutated);
    const v = checkTokens(mutated);
    const hit = v.filter((line) => line.includes(m.expect));
    let ok = hit.length > 0;
    if (ok && m.onlyBg) {
      const composite = v.filter((line) => line.includes('R1 composite'));
      ok = composite.length > 0 && composite.every((line) => line.includes(`over ${m.onlyBg} `));
    }
    if (!ok) {
      console.error(
        `[check-contrast] selftest FAIL ${m.name}: гейт не покраснел на «${m.expect}»${m.onlyBg ? ` (только ${m.onlyBg})` : ''}`,
      );
      failed++;
    } else {
      console.error(`[check-contrast] selftest ok ${m.name} → «${m.expect}»`);
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
  if (args.includes('--selftest')) {
    const dump = args.find((a) => a.startsWith('--dump-mutants='));
    selftest(content, dump ? path.resolve(dump.slice('--dump-mutants='.length)) : null);
  }
  const stats = {};
  const violations = checkTokens(content, stats);
  process.exit(report(violations, stats));
}

main();
