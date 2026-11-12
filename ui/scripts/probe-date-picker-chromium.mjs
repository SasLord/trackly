#!/usr/bin/env node
// [probe-date-picker-chromium] РУЧНАЯ проба (НЕ в lint и не в CI: нужен установленный
// Google Chrome и macOS `sips`). Проверяет положение иконки календаря у DatePicker
// в настоящем Chromium по стилям СОБРАННОГО бандла ui/dist.
//
// Что это и чего это не доказывает. Страница синтетическая: один и тот же CSS
// бандла, но не само приложение. Chromium (headless) — это движок WebView2, но не
// сам WebView2; WKWebView (macOS) — другой движок. Вердикт пробы НЕ заменяет
// живую проверку на Windows/macOS.
//
// Прибор самопроверяется: строка (д) — КОНТРОЛЬ с прежним дефектом (display:flex;
// align-items:center на input). Если контроль проходит критерий — прибор сломан,
// проба падает.
//
// Шаги: vite build -> node scripts/probe-date-picker-chromium.mjs <каталог вне репо>
//   env CHROME — путь к Chrome (по умолчанию macOS-приложение).
//
// Usage: node scripts/probe-date-picker-chromium.mjs <out-dir>

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const ASSETS = path.resolve(__dirname, '../dist/assets');
const TAG = '[probe-date-picker-chromium]';
const out = process.argv[2];
if (!out) {
  console.error(`${TAG} usage: node probe-date-picker-chromium.mjs <out-dir>`);
  process.exit(2);
}
fs.mkdirSync(out, { recursive: true });

function findChrome() {
  const cands = [
    process.env.CHROME,
    '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
  ].filter(Boolean);
  const pw = path.join(os.homedir(), 'Library/Caches/ms-playwright');
  if (fs.existsSync(pw)) {
    for (const d of fs.readdirSync(pw).filter((x) => x.startsWith('chromium_headless_shell-'))) {
      cands.push(path.join(pw, d, 'chrome-headless-shell-mac-arm64/chrome-headless-shell'));
    }
  }
  const c = cands.find((p) => fs.existsSync(p));
  if (!c) {
    console.error(`${TAG} не найден Chrome (задайте CHROME)`);
    process.exit(2);
  }
  return c;
}
const CHROME = findChrome();
const version = execFileSync(CHROME, ['--version']).toString().trim();

const cssFiles = fs.existsSync(ASSETS)
  ? fs.readdirSync(ASSETS).filter((f) => /^index-.*\.css$/.test(f))
  : [];
if (cssFiles.length === 0) {
  console.error(`${TAG} нет ui/dist/assets/index-*.css — сначала pnpm exec vite build`);
  process.exit(2);
}
const cssPath = path.join(ASSETS, cssFiles[0]);
const scope = fs.readFileSync(cssPath, 'utf8').match(/\.date-picker\.(svelte-[a-z0-9]+)/)?.[1];
if (!scope) {
  console.error(`${TAG} в бандле нет правила .date-picker.svelte-XXXX`);
  process.exit(2);
}

const ROWS = [
  { id: 'a', label: 'обычное поле 460px', style: '', attrs: 'value="2026-11-12"', expect: 'right' },
  {
    id: 'b',
    label: 'PeriodSelector 28px / min-width 130px',
    style: 'height:28px;width:auto;min-width:130px;',
    attrs: 'value="2026-11-12"',
    expect: 'right',
  },
  { id: 'c', label: 'пустое значение', style: '', attrs: 'value=""', expect: 'right' },
  {
    id: 'd',
    label: 'disabled',
    style: '',
    attrs: 'value="2026-11-12" disabled',
    expect: 'disabled',
  },
  {
    id: 'x',
    label: 'КОНТРОЛЬ: display:flex;flex-direction:row;align-items:center (прежний дефект)',
    style: 'display:flex;flex-direction:row;align-items:center;',
    attrs: 'value="2026-11-12"',
    expect: 'control',
  },
];
const ROW_H = 60;
const TOP0 = 30;
const LEFT = 20;
const W = 460;

function html(theme) {
  const rows = ROWS.map(
    (r, i) =>
      `<div class="row" style="top:${TOP0 + i * ROW_H}px"><input id="r-${r.id}" type="date" lang="ru" class="date-picker ${scope}" style="${r.style}" ${r.attrs}></div>`,
  ).join('\n');
  return `<!doctype html><html lang="ru" data-theme="${theme}"><head><meta charset="utf-8">
<link rel="stylesheet" href="file://${cssPath}">
<style>html,body{margin:0;padding:0;background:var(--tr-surface-base,#fff)}
.row{position:absolute;left:${LEFT}px;width:${W}px}</style></head><body>
${rows}
<pre id="rects" style="display:none"></pre>
<script>window.addEventListener('load',()=>{const o={};document.querySelectorAll('input').forEach(i=>{const r=i.getBoundingClientRect();o[i.id]=[r.left,r.top,r.width,r.height]});document.getElementById('rects').textContent=JSON.stringify(o);document.body.setAttribute('data-done','1')})</script>
</body></html>`;
}

/** Минимальный разбор BMP 24/32 бит (строки снизу вверх). */
function readBmp(buf) {
  const off = buf.readUInt32LE(10);
  const w = buf.readInt32LE(18);
  let h = buf.readInt32LE(22);
  const bpp = buf.readUInt16LE(28);
  const topDown = h < 0;
  h = Math.abs(h);
  const bytes = bpp / 8;
  const stride = Math.floor((bpp * w + 31) / 32) * 4;
  return {
    w,
    h,
    px(x, y) {
      const row = topDown ? y : h - 1 - y;
      const p = off + row * stride + x * bytes;
      return [buf[p + 2], buf[p + 1], buf[p]];
    },
  };
}
const lum = ([r, g, b]) => 0.299 * r + 0.587 * g + 0.114 * b;

function chrome(extra) {
  return execFileSync(
    CHROME,
    [
      '--headless=new',
      '--disable-gpu',
      '--hide-scrollbars',
      '--force-device-scale-factor=1',
      ...extra,
    ],
    { stdio: ['ignore', 'pipe', 'pipe'], maxBuffer: 1 << 26 },
  ).toString();
}

const result = { chrome: version, scope, css: path.basename(cssPath), themes: {}, ok: true };
for (const theme of ['light', 'dark']) {
  const file = path.join(out, `probe-${theme}.html`);
  fs.writeFileSync(file, html(theme));
  const url = `file://${file}`;
  const dom = chrome(['--virtual-time-budget=2000', '--dump-dom', url]);
  const rects = JSON.parse(
    dom.match(/<pre id="rects"[^>]*>([^<]*)<\/pre>/)[1].replace(/&quot;/g, '"'),
  );
  const png = path.join(out, `probe-${theme}.png`);
  const winH = TOP0 + ROWS.length * ROW_H + 20;
  chrome([`--window-size=${LEFT * 2 + W},${winH}`, `--screenshot=${png}`, url]);
  const bmp = path.join(out, `probe-${theme}.bmp`);
  execFileSync('sips', ['-s', 'format', 'bmp', png, '--out', bmp], { stdio: 'ignore' });
  const img = readBmp(fs.readFileSync(bmp));
  const rows = {};
  for (const r of ROWS) {
    const [l, t, w, h] = rects[`r-${r.id}`].map(Math.round);
    const midY = t + Math.floor(h / 2);
    const bg = lum(img.px(l + 3, midY));
    const y0 = t + Math.floor(h / 4);
    const y1 = t + h - Math.floor(h / 4);
    let inkMax = -1;
    let inkMin = Infinity;
    for (let x = l + 3; x < l + w - 4; x++) {
      for (let y = y0; y < y1; y++) {
        if (Math.abs(lum(img.px(x, y)) - bg) > (r.expect === 'disabled' ? 25 : 60)) {
          inkMax = Math.max(inkMax, x);
          inkMin = Math.min(inkMin, x);
          break;
        }
      }
    }
    const innerRight = l + w - 1 - 12;
    const gap = innerRight - inkMax;
    let pass;
    if (inkMax < 0) pass = false;
    // disabled: Chromium скрывает иконку календаря — проверяем только, что дата слева и поле цело
    else if (r.expect === 'disabled') pass = inkMin - l <= 20;
    else if (r.expect === 'control')
      pass = gap > 60; // контроль обязан быть «за датой»
    else pass = gap <= 20;
    if (!pass) result.ok = false;
    rows[r.id] = {
      label: r.label,
      rect: { l, t, w, h },
      inkMin,
      inkMax,
      innerRight,
      gap,
      expect: r.expect,
      pass,
    };
  }
  // прибор: контроль не должен проходить критерий «у правого края»
  const control = rows.x;
  if (control.gap <= 20) {
    result.ok = false;
    result.instrumentBroken = true;
  }
  result.themes[theme] = { png, rows };
}
console.log(JSON.stringify(result, null, 2));
if (!result.ok) {
  console.error(
    `${TAG} FAIL${result.instrumentBroken ? ' (прибор сломан: контроль прошёл критерий)' : ''}`,
  );
  process.exit(1);
}
console.log(`${TAG} OK (Chromium headless; WebView2/WKWebView — UNVERIFIED)`);
