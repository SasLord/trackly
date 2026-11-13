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
// Проба собирает НАСТОЯЩИЙ DatePicker.svelte (vite lib-сборка во временный каталог
// вне репо) и монтирует его — не статичную копию разметки. Два прогона на тему:
//   1) настоящий UA Chrome: кнопки .date-picker-btn в DOM быть НЕ должно, зазоры у
//      правого края прежние (колоночный flex плана 38);
//   2) подставленный UA WebKit (--user-agent, как у Tauri WKWebView): кнопка есть,
//      лежит у правого края внутри поля, иконка не перекрывает дату, контраст >= 3:1,
//      клик не бросает исключение и (при отказе showPicker в headless) фокусирует input.
// В прогоне (2) нативный индикатор Chromium скрыт (в WebKit его нет) — иначе иконки
// накладывались бы. Это Chromium под подставленным UA, НЕ WKWebView.
//
// Шаги: node scripts/probe-date-picker-chromium.mjs <каталог вне репо>
//   env CHROME — путь к Chrome (по умолчанию macOS-приложение).
//
// Usage: node scripts/probe-date-picker-chromium.mjs <out-dir>

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI = path.resolve(__dirname, '..');
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

const WK_UA =
  'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko)';

// Сборка harness: настоящий DatePicker + глобальные стили приложения.
const { build } = await import('vite');
const { svelte, vitePreprocess } = await import('@sveltejs/vite-plugin-svelte');
const hdir = path.join(out, 'harness-src');
fs.mkdirSync(hdir, { recursive: true });
const nm = path.join(hdir, 'node_modules');
if (!fs.existsSync(nm)) fs.symlinkSync(path.join(UI, 'node_modules'), nm); // резолв svelte из каталога вне репо
const entry = path.join(hdir, 'entry.js');
fs.writeFileSync(
  entry,
  `import { mount } from 'svelte';
import DatePicker from ${JSON.stringify(path.join(UI, 'src/lib/components/DatePicker.svelte'))};
import ${JSON.stringify(path.join(UI, 'src/styles/global.scss'))};
window.__mount = (target, props) => mount(DatePicker, { target, props });
`,
);
const hout = path.join(out, 'harness');
await build({
  root: hdir,
  configFile: false,
  logLevel: 'error',
  plugins: [svelte({ configFile: false, preprocess: vitePreprocess() })],
  resolve: { alias: { $lib: path.join(UI, 'src/lib') } },
  build: {
    outDir: hout,
    emptyOutDir: true,
    lib: { entry, formats: ['iife'], name: 'H', fileName: () => 'h.js', cssFileName: 'h' },
  },
});
const hjs = path.join(hout, 'h.js');
const hcss = fs.readdirSync(hout).find((f) => f.endsWith('.css'));
if (!hcss) {
  console.error(`${TAG} harness: нет css`);
  process.exit(2);
}

const ROWS = [
  { id: 'a', label: 'обычное поле 460px', wrap: '', value: '2026-11-12', expect: 'right' },
  {
    id: 'b',
    label: 'PeriodSelector: flex-строка .range-label (28px / min-width 130px)',
    wrap: 'display:inline-block;',
    value: '2026-11-12',
    expect: 'right',
    period: true,
  },
  { id: 'c', label: 'пустое значение', wrap: '', value: '', expect: 'right' },
  { id: 'd', label: 'disabled', wrap: '', value: '2026-11-12', expect: 'disabled', disabled: true },
  {
    id: 'x',
    label: 'КОНТРОЛЬ: display:flex;flex-direction:row;align-items:center (прежний дефект)',
    wrap: '',
    value: '2026-11-12',
    expect: 'control',
  },
];
const ROW_H = 60;
const TOP0 = 30;
const LEFT = 20;
const W = 460;

function html(theme, webkit) {
  const rows = ROWS.map(
    (r, i) =>
      `<div class="row ${r.period ? 'pr' : ''}" id="w-${r.id}" style="top:${TOP0 + i * ROW_H}px;${r.period ? 'display:flex;align-items:center;width:max-content;' : ''}"></div>`,
  ).join('\n');
  const mounts = ROWS.map(
    (r) =>
      `window.__mount(document.getElementById('w-${r.id}'), {value:${JSON.stringify(r.value)}, id:'r-${r.id}'${r.disabled ? ', disabled:true' : ''}});`,
  ).join('\n');
  return `<!doctype html><html lang="ru" data-theme="${theme}"><head><meta charset="utf-8">
<link rel="stylesheet" href="file://${path.join(hout, hcss)}">
<style>html,body{margin:0;padding:0;background:var(--tr-surface-base,#fff)}
.row{position:absolute;left:${LEFT}px;width:${W}px}
/* как PeriodSelector: :global(.date-picker-field) и :global(.date-picker) */
.pr .date-picker-field{width:auto;min-width:130px}
.pr .date-picker{height:28px;width:auto;min-width:130px;font-size:var(--tr-font-size-label)}
#w-x .date-picker{display:flex;flex-direction:row;align-items:center}
${webkit ? '.date-picker::-webkit-calendar-picker-indicator{display:none}' : ''}</style></head><body>
${rows}
<pre id="rects" style="display:none"></pre>
<script src="file://${hjs}"></script>
<script>
${mounts}
window.addEventListener('load',()=>{
 const o={};
 document.querySelectorAll('input').forEach(i=>{const r=i.getBoundingClientRect();o[i.id]=[r.left,r.top,r.width,r.height]});
 const info={ua:navigator.userAgent, btns:{}};
 document.querySelectorAll('.date-picker-btn').forEach(b=>{
  const w=b.closest('.row'); const inp=w.querySelector('input'); const r=b.getBoundingClientRect();
  const cs=getComputedStyle(b), is=getComputedStyle(inp);
  info.btns[inp.id]={rect:[r.left,r.top,r.width,r.height],color:cs.color,bg:is.backgroundColor,padR:is.paddingRight,label:b.getAttribute('aria-label'),type:b.getAttribute('type')};
 });
 info.btnCount=document.querySelectorAll('.date-picker-btn').length;
 let err=null;
 const a=document.querySelector('#r-a'); const ba=document.querySelector('#w-a .date-picker-btn');
 window.onerror=(m)=>{err=String(m)};
 if(ba){ try{ ba.click(); }catch(e){err=String(e)} info.click={err, activeIsInput: document.activeElement===a}; }
 info.fieldRects={};
 document.querySelectorAll('.date-picker-field').forEach(f=>{const r=f.getBoundingClientRect(); info.fieldRects[f.closest('.row').id]=[r.left,r.top,r.width,r.height]});
 document.getElementById('rects').textContent=JSON.stringify({rects:o,info});
 document.body.setAttribute('data-done','1');
});
</script>
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
const rgb = (s) =>
  s
    .match(/[\d.]+/g)
    .slice(0, 3)
    .map(Number);
const relLum = (c) => {
  const f = (v) => {
    v /= 255;
    return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * f(c[0]) + 0.7152 * f(c[1]) + 0.0722 * f(c[2]);
};
const contrast = (a, b) => {
  const [x, y] = [relLum(a), relLum(b)].sort((p, q) => q - p);
  return (x + 0.05) / (y + 0.05);
};

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

const result = { chrome: version, runs: {}, ok: true };
const fail = (why) => {
  result.ok = false;
  (result.failures ??= []).push(why);
};
for (const mode of ['chrome-ua', 'webkit-ua']) {
  const webkit = mode === 'webkit-ua';
  for (const theme of ['light', 'dark']) {
    const key = `${mode}/${theme}`;
    const file = path.join(out, `probe-${mode}-${theme}.html`);
    fs.writeFileSync(file, html(theme, webkit));
    const url = `file://${file}`;
    const uaArg = webkit ? [`--user-agent=${WK_UA}`] : [];
    const dom = chrome([...uaArg, '--virtual-time-budget=2000', '--dump-dom', url]);
    const payload = JSON.parse(
      dom.match(/<pre id="rects"[^>]*>([^<]*)<\/pre>/)[1].replace(/&quot;/g, '"'),
    );
    const { rects, info } = payload;
    const png = path.join(out, `probe-${mode}-${theme}.png`);
    const winH = TOP0 + ROWS.length * ROW_H + 20;
    chrome([...uaArg, `--window-size=${LEFT * 2 + W},${winH}`, `--screenshot=${png}`, url]);
    const bmp = path.join(out, `probe-${mode}-${theme}.bmp`);
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
      const row = { label: r.label, rect: { l, t, w, h }, inkMin, inkMax, gap, expect: r.expect };
      if (!webkit) {
        // настоящий UA Chrome: раскладка плана 38 без изменений
        let pass;
        if (inkMax < 0) pass = false;
        else if (r.expect === 'disabled') pass = inkMin - l <= 20;
        else if (r.expect === 'control') pass = gap > 60;
        else pass = gap <= 20;
        row.pass = pass;
        if (!pass) fail(`${key} ${r.id}: раскладка`);
      } else if (r.expect === 'right' || r.expect === 'disabled') {
        const b = info.btns[`r-${r.id}`];
        if (r.expect === 'disabled') {
          row.pass = !b;
          if (b) fail(`${key} ${r.id}: кнопка у disabled`);
        } else if (!b) {
          row.pass = false;
          fail(`${key} ${r.id}: нет кнопки под UA WebKit`);
        } else {
          const [bl, bt, bw, bh] = b.rect.map(Math.round);
          const rightGap = l + w - (bl + bw);
          // дата не уходит под иконку: padding-right поля >= ширине кнопки + отступ
          const padR = parseFloat(b.padR);
          const dateInkMax = (() => {
            let m = -1;
            for (let x = l + 3; x < bl - 1; x++) {
              for (let y = y0; y < y1; y++) {
                if (Math.abs(lum(img.px(x, y)) - bg) > 60) {
                  m = x;
                  break;
                }
              }
            }
            return m;
          })();
          // иконка видна: чернила внутри прямоугольника кнопки
          let iconInk = 0;
          for (let x = bl; x < bl + bw; x++)
            for (let y = bt; y < bt + bh; y++) if (Math.abs(lum(img.px(x, y)) - bg) > 60) iconInk++;
          const cr = contrast(rgb(b.color), rgb(b.bg));
          row.button = {
            rect: { bl, bt, bw, bh },
            rightGap,
            padR,
            iconInk,
            colorContrast: Math.round(cr * 100) / 100,
            label: b.label,
            type: b.type,
          };
          row.pass =
            rightGap <= 12 &&
            rightGap >= 0 &&
            bl >= l &&
            padR >= bw + 6 &&
            iconInk > 10 &&
            cr >= 3 &&
            b.label === 'Выбрать дату' &&
            b.type === 'button' &&
            (inkMax < 0 || dateInkMax < bl);
          if (!row.pass) fail(`${key} ${r.id}: кнопка WebKit`);
        }
      }
      rows[r.id] = row;
    }
    if (!webkit) {
      if (info.btnCount !== 0)
        fail(`${key}: в DOM под UA Chrome есть .date-picker-btn (${info.btnCount})`);
      if (rows.x.gap <= 20) {
        fail(`${key}: прибор сломан, контроль прошёл критерий`);
        result.instrumentBroken = true;
      }
    } else {
      if (!info.click || info.click.err || !info.click.activeIsInput)
        fail(`${key}: клик по кнопке: ${JSON.stringify(info.click)}`);
    }
    result.runs[key] = { png, ua: info.ua, btnCount: info.btnCount, click: info.click, rows };
  }
}
console.log(JSON.stringify(result, null, 2));
if (!result.ok) {
  console.error(
    `${TAG} FAIL${result.instrumentBroken ? ' (прибор сломан: контроль прошёл критерий)' : ''}`,
  );
  process.exit(1);
}
console.log(
  `${TAG} OK (Chromium headless; под подставленным UA WebKit это НЕ WKWebView; WebView2/WKWebView/Safari — UNVERIFIED)`,
);
