#!/usr/bin/env node
// [check-date-picker-layout] Гейт: у <input type="date"> класса .date-picker нет
// строчного flex-контейнера, который сжимает внутренности поля.
//
// Почему: display:flex + align-items:center на самом input (фаза 27, ради
// вертикального центрирования в WKWebView) делает внутренний контейнер Chromium
// (-webkit-date-and-time-container) flex-элементом шириной по содержимому —
// иконка календаря встаёт сразу за датой, а не у правого края (UAT 41.4, раунд 2).
// svelte-check и build этого не видят.
//
// Допустимо: display:block; display:grid без justify-items кроме stretch/normal;
// display:flex ТОЛЬКО с flex-direction:column и без align-items кроме stretch/normal
// (в колонке align-items:center снова сжимает контейнер по ширине).
// Переопределения :global(.date-picker) в любом .svelte не должны включать
// flex/grid (размеры — можно).
//
// Вторая часть гейта (раунд 4, план 48): значение даты прижато влево
// (::-webkit-date-and-time-value), собственная кнопка календаря для WebKit стоит
// ТОЛЬКО под {#if} от needsOwnCalendarIcon (не скрыта CSS — иначе дубль в Chromium),
// showPicker вызывается в try под typeof-охраной с фоллбэком focus, цвет иконки — токен,
// таблица User-Agent детектора. Проверяется вложенность, а не число вхождений.
//
// Гейт структурный (чтение исходников). Рантайм раскладки доказывает только
// ручная проба scripts/probe-date-picker-chromium.mjs (настоящий Chromium).
//
// Usage:
//   node scripts/check-date-picker-layout.mjs
//   node scripts/check-date-picker-layout.mjs --selftest

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { stripComments } from './_strip.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const SRC = path.resolve(__dirname, '../src');
const COMPONENT = path.join(SRC, 'lib/components/DatePicker.svelte');
const TAG = '[check-date-picker-layout]';
const WHY =
  'строчный flex на input сжимает внутренний контейнер Chromium ' +
  '(-webkit-date-and-time-container) по содержимому — иконка календаря встаёт за датой';

/** Тело блока `{ ... }`, открывающегося после позиции open (индекс `{`). */
function blockBody(src, open) {
  let depth = 0;
  for (let i = open; i < src.length; i++) {
    if (src[i] === '{') depth++;
    else if (src[i] === '}') {
      depth--;
      if (depth === 0) return src.slice(open + 1, i);
    }
  }
  return src.slice(open + 1);
}

/** Объявления верхнего уровня тела блока (без вложенных блоков). */
function topLevelDecls(body) {
  let depth = 0;
  let flat = '';
  for (const ch of body) {
    if (ch === '{') {
      if (depth === 0) flat = flat.slice(0, flat.lastIndexOf(';') + 1); // заголовок вложенного селектора
      depth++;
      continue;
    }
    if (ch === '}') {
      depth--;
      if (depth === 0) flat += ';';
      continue;
    }
    if (depth === 0) flat += ch;
  }
  const decls = {};
  for (const part of flat.split(';')) {
    const m = part.match(/^\s*([a-z-]+)\s*:\s*([^]*?)\s*$/);
    if (m) decls[m[1]] = m[2].trim();
  }
  return decls;
}

/** Тело `{...}` вложенного/верхнего блока по заголовку-регэкспу, либо null. */
function findBlock(css, headerRe) {
  const m = headerRe.exec(css);
  if (!m) return null;
  return blockBody(css, m.index + m[0].length - 1);
}

const NEUTRAL = new Set(['stretch', 'normal']);

/** Нарушения для DatePicker.svelte: список строк. */
export function check(raw) {
  const m = raw.match(/<style\b[^>]*>([^]*?)<\/style>/);
  if (!m) return ['нет блока <style>'];
  const css = stripComments(m[1]);
  const re = /(^|[\s}])\.date-picker\s*\{/g;
  const bad = [];
  let found = false;
  for (const hit of css.matchAll(re)) {
    found = true;
    const open = hit.index + hit[0].length - 1;
    const d = topLevelDecls(blockBody(css, open));
    const display = d.display ?? '';
    const ai = d['align-items'];
    if (/^inline-(flex|grid)$/.test(display)) {
      bad.push(`display: ${display} — ${WHY}`);
    } else if (display === 'flex') {
      if (d['flex-direction'] !== 'column') {
        bad.push(`display: flex без flex-direction: column — ${WHY}`);
      } else if (ai && !NEUTRAL.has(ai)) {
        bad.push(`display: flex column с align-items: ${ai} — ${WHY}`);
      }
    } else if (display === 'grid') {
      const ji = d['justify-items'];
      if (ji && !NEUTRAL.has(ji)) bad.push(`display: grid с justify-items: ${ji} — ${WHY}`);
    }
  }
  if (!found) bad.push('не найден блок .date-picker');
  return bad;
}

const ICON_WHY_DUP =
  'кнопка календаря должна быть под {#if} от needsOwnCalendarIcon, а не скрыта CSS (дубль иконки в Chromium/WebView2)';

/** Нарушения иконки/выравнивания в DatePicker.svelte (весь файл, не только style). */
export function checkIcon(raw) {
  const bad = [];
  const clean = stripComments(raw);
  const styleM = clean.match(/<style\b[^>]*>([^]*?)<\/style>/);
  const css = styleM ? styleM[1] : '';
  const scriptM = clean.match(/<script\b[^>]*>([^]*?)<\/script>/);
  const script = scriptM ? scriptM[1] : '';
  const markup = clean
    .replace(/<style\b[^>]*>[^]*?<\/style>/g, '')
    .replace(/<script\b[^>]*>[^]*?<\/script>/g, '');

  // (а) значение слева в WebKit
  const dp = findBlock(css, /(^|[\s}])\.date-picker\s*\{/);
  const val = dp ? findBlock(dp, /&?::-webkit-date-and-time-value\s*\{/) : null;
  const ta = val === null ? undefined : topLevelDecls(val)['text-align'];
  if (ta !== 'left' && ta !== 'start') {
    bad.push(
      '.date-picker без ::-webkit-date-and-time-value { text-align: left } — WKWebView центрирует значение даты',
    );
  }

  // (е) импорт и ownIcon
  if (
    !/import\s*\{[^}]*\bneedsOwnCalendarIcon\b[^}]*\}\s*from\s*['"][^'"]*calendarIcon(?:\.js)?['"]/.test(
      script,
    )
  ) {
    bad.push('нет импорта needsOwnCalendarIcon из calendarIcon');
  }

  // (б) кнопка
  const btnM = /<button\b[^>]*\bclass="date-picker-btn"[^>]*>/.exec(markup);
  if (!btnM) {
    bad.push('нет button.date-picker-btn — в WKWebView нет иконки календаря');
  } else {
    if (!/\btype="button"/.test(btnM[0])) bad.push('button.date-picker-btn без type="button"');
    if (!/\baria-label="[^"]+"/.test(btnM[0])) bad.push('button.date-picker-btn без aria-label');

    // (в) кнопка внутри {#if ...needsOwnCalendarIcon|ownIcon...} без {:else} до неё
    const stack = [];
    let guarded;
    const tok = /\{#if\b([^}]*)\}|\{:else[^}]*\}|\{\/if\}/g;
    for (const t of markup.matchAll(tok)) {
      if (t.index >= btnM.index) break;
      if (t[0].startsWith('{#if')) stack.push({ cond: t[1], inElse: false });
      else if (t[0].startsWith('{:else')) {
        if (stack.length) stack[stack.length - 1].inElse = true;
      } else stack.pop();
    }
    guarded = stack.some((f) => !f.inElse && /\b(ownIcon|needsOwnCalendarIcon)\b/.test(f.cond));
    if (!guarded) bad.push(ICON_WHY_DUP);
    if (guarded && /\bownIcon\b/.test(stack.map((f) => f.cond).join(' '))) {
      if (!/\bownIcon\s*=\s*needsOwnCalendarIcon\s*\(/.test(script)) {
        bad.push('ownIcon не вычисляется через needsOwnCalendarIcon(...)');
      }
    }

    // (г) обработчик клика
    const hm = /\bonclick=\{\s*([A-Za-z_$][\w$]*)\s*\}/.exec(btnM[0]);
    const fnRe = hm ? new RegExp('function\\s+' + hm[1] + '\\s*\\([^)]*\\)\\s*\\{') : null;
    const fnBody = fnRe ? findBlock(script, fnRe) : null;
    if (fnBody === null) {
      bad.push('у button.date-picker-btn нет onclick={функция} с объявленной функцией');
    } else {
      const tryM = /\btry\s*\{/.exec(fnBody);
      const tryBody = tryM ? blockBody(fnBody, tryM.index + tryM[0].length - 1) : '';
      const rest = tryM
        ? fnBody.slice(0, tryM.index) +
          fnBody.slice(tryM.index + tryM[0].length + tryBody.length + 1)
        : fnBody;
      if (!/typeof\s+[\w$.]+\.showPicker\s*===\s*['"]function['"]/.test(fnBody)) {
        bad.push("обработчик без охраны typeof …showPicker === 'function'");
      }
      if (!/\.showPicker\s*\(/.test(tryBody)) {
        bad.push('вызов showPicker() вне try — NotAllowedError/SecurityError уронят обработчик');
      } else if (/\.showPicker\s*\(/.test(rest)) {
        bad.push('вызов showPicker() и вне try');
      }
      if (!/\.focus\s*\(\s*\)/.test(rest)) {
        bad.push('нет фоллбэка .focus() при отказе/отсутствии showPicker');
      }
    }
  }

  // (д) цвет иконки — токен
  const btnCss = findBlock(css, /(^|[\s}])\.date-picker-btn\s*\{/);
  if (btnCss !== null) {
    const color = topLevelDecls(btnCss).color ?? '';
    if (!/^var\(--tr-[\w-]+\)$/.test(color)) {
      bad.push(`.date-picker-btn color: ${color || '(нет)'} — нужен токен var(--tr-…)`);
    }
    if (/#[0-9a-fA-F]{3,8}\b|\brgba?\(|\bhsla?\(/.test(btnCss)) {
      bad.push('.date-picker-btn: хардкод цвета (hex/rgb/hsl) вместо токена');
    }
    if (/\bdisplay\s*:\s*none\b/.test(btnCss)) {
      bad.push(`.date-picker-btn скрыт через CSS — ${ICON_WHY_DUP}`);
    }
  } else if (btnM) {
    bad.push('нет стилей .date-picker-btn');
  }
  return bad;
}

export const UA_TABLE = [
  [
    'Chrome Windows',
    'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36',
    false,
  ],
  [
    'Chrome macOS',
    'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36',
    false,
  ],
  [
    'Edge / WebView2',
    'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36 Edg/130.0.0.0',
    false,
  ],
  [
    'Firefox',
    'Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:132.0) Gecko/20100101 Firefox/132.0',
    false,
  ],
  [
    'Opera',
    'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36 OPR/115.0.0.0',
    false,
  ],
  [
    'Safari macOS',
    'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Safari/605.1.15',
    true,
  ],
  [
    'Tauri WKWebView macOS',
    'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko)',
    true,
  ],
  [
    'Safari iPhone',
    'Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Mobile/15E148 Safari/604.1',
    false,
  ],
  [
    'Chrome iOS',
    'Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) CriOS/130.0.0.0 Mobile/15E148 Safari/604.1',
    false,
  ],
  [
    'Chrome Android',
    'Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Mobile Safari/537.36',
    false,
  ],
  ['пустая строка', '', false],
  ['undefined', undefined, false],
];

/** Расхождения детектора с таблицей UA. */
export function checkDetector(fn) {
  const bad = [];
  for (const [name, ua, want] of UA_TABLE) {
    const got = fn(ua);
    if (got !== want)
      bad.push(`needsOwnCalendarIcon(${name}: ${JSON.stringify(ua)}) = ${got}, ожидалось ${want}`);
  }
  return bad;
}

/** Нарушения переопределений :global(.date-picker) в одном .svelte. */
export function checkOverrides(raw, file = '?') {
  const bad = [];
  for (const sm of raw.matchAll(/<style\b[^>]*>([^]*?)<\/style>/g)) {
    const css = stripComments(sm[1]);
    for (const hit of css.matchAll(/:global\(\s*\.date-picker\s*\)[^{;]*\{/g)) {
      const open = hit.index + hit[0].length - 1;
      const d = topLevelDecls(blockBody(css, open));
      if (/^(inline-)?(flex|grid)$/.test(d.display ?? '')) {
        bad.push(`${file}: :global(.date-picker) с display: ${d.display} — ${WHY}`);
      }
    }
  }
  return bad;
}

function walk(dir, out = []) {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, e.name);
    if (e.isDirectory()) walk(p, out);
    else if (e.name.endsWith('.svelte')) out.push(p);
  }
  return out;
}

const wrap = (decls) =>
  `<input class="date-picker"/><style lang="scss">.date-picker { ${decls} }</style>`;

export const CLEAN = `<script lang="ts">
  import { needsOwnCalendarIcon } from '$lib/utils/calendarIcon.js';
  const ownIcon = needsOwnCalendarIcon(navigator.userAgent);
  function openPicker() {
    if (typeof inputEl.showPicker === 'function') {
      try {
        inputEl.showPicker();
        return;
      } catch {
        // отказ
      }
    }
    inputEl.focus();
  }
</script>
<span class="date-picker-field">
  <input type="date" class="date-picker" />
  {#if ownIcon && !disabled}
    <button type="button" class="date-picker-btn" aria-label="Выбрать дату" onclick={openPicker}></button>
  {/if}
</span>
<style lang="scss">
  .date-picker-btn { color: var(--tr-text-secondary); &:hover { color: var(--tr-text-primary); } }
  .date-picker {
    display: flex; flex-direction: column;
    &::-webkit-date-and-time-value { text-align: left; }
  }
</style>`;

export function iconFixtures() {
  const rep = (a, b) => {
    if (CLEAN.split(a).length !== 2) throw new Error('якорь фикстуры не уникален: ' + a);
    return CLEAN.replace(a, b);
  };
  const bad = {
    'нет text-align': rep('text-align: left;', 'color: red;'),
    'text-align: center': rep('text-align: left;', 'text-align: center;'),
    'text-align только в комментарии': rep('text-align: left;', '/* text-align: left; */'),
    'нет кнопки': rep(/<button[^]*?<\/button>/.exec(CLEAN)[0], ''),
    'кнопка только в HTML-комментарии': rep(
      /<button[^]*?<\/button>/.exec(CLEAN)[0],
      '<!-- <button type="button" class="date-picker-btn" aria-label="x"></button> -->',
    ),
    'без type=button': rep('<button type="button" class', '<button class'),
    'без aria-label': rep(' aria-label="Выбрать дату"', ''),
    'кнопка безусловна': rep('{#if ownIcon && !disabled}', '{#if !disabled}'),
    'кнопка перед {#if}': rep(
      '{#if ownIcon && !disabled}\n    <button',
      '<button type="button" class="date-picker-btn" aria-label="x" onclick={openPicker}></button>\n  {#if ownIcon}\n  {/if}\n  {#if ownIcon}\n    <button',
    ).replace(
      /\{#if ownIcon\}\n\s*\{\/if\}\n\s*\{#if ownIcon\}\n\s*<button type="button" class="date-picker-btn" aria-label="Выбрать дату" onclick=\{openPicker\}><\/button>\n\s*\{\/if\}/,
      '{#if ownIcon}\n  {/if}',
    ),
    'кнопка в {:else}': rep('{/if}\n</span>', '{/if}\n</span>').replace(
      '{#if ownIcon && !disabled}',
      '{#if ownIcon}{:else}',
    ),
    'ownIcon не от детектора': rep(
      'const ownIcon = needsOwnCalendarIcon(navigator.userAgent);',
      'const ownIcon = true;',
    ),
    'скрыта через CSS': rep('.date-picker-btn { color', '.date-picker-btn { display: none; color'),
    'showPicker вне try': rep(
      'try {\n        inputEl.showPicker();\n        return;\n      } catch {\n        // отказ\n      }',
      'inputEl.showPicker();\n      return;',
    ),
    'нет typeof-охраны': rep("typeof inputEl.showPicker === 'function'", 'inputEl'),
    'нет focus-фоллбэка': rep('inputEl.focus();', 'noop();'),
    'хардкод цвета': rep('color: var(--tr-text-secondary);', 'color: #888;'),
    'нет импорта': rep("import { needsOwnCalendarIcon } from '$lib/utils/calendarIcon.js';", ''),
  };
  return bad;
}

function selftest() {
  const bad = [
    wrap('display: flex; align-items: center;'),
    wrap('display: flex;'),
    wrap('display: inline-flex; flex-direction: column;'),
    wrap('display: flex; flex-direction: column; align-items: center;'),
    wrap('display: flex; flex-direction: row; justify-content: center;'),
    wrap('display: grid; justify-items: start;'),
    wrap('display: inline-grid;'),
  ];
  const good = [
    wrap('display: flex; flex-direction: column; justify-content: center;'),
    wrap('display: flex; flex-direction: column; align-items: stretch;'),
    wrap('display: grid; align-items: center;'),
    wrap('display: block;'),
    wrap('width: 100%;'),
    // комментарий с плохим кодом не считается
    wrap('/* display: flex; align-items: center; */ display: block;'),
    // вложенный блок не считается объявлением верхнего уровня
    wrap('display: block; &:focus-visible { display: flex; } width: 100%;'),
  ];
  const ovBad = [
    `<style>.r :global(.date-picker) { display: flex; }</style>`,
    `<style lang="scss">.r :global(.date-picker) { display: inline-flex; height: 28px; }</style>`,
    `<style>:global(.date-picker) { display: grid; }</style>`,
  ];
  const ovGood = [
    `<style>.r :global(.date-picker) { height: 28px; width: auto; min-width: 130px; }</style>`,
    `<style>.r { display: flex; }</style>`,
    `<style>/* :global(.date-picker) { display: flex; } */ .r { color: red; }</style>`,
  ];
  const errs = [];
  if (checkIcon(CLEAN).length !== 0)
    errs.push('чистая фикстура иконки ложно красная: ' + checkIcon(CLEAN).join('; '));
  for (const [name, src] of Object.entries(iconFixtures())) {
    if (checkIcon(src).length === 0) errs.push(`плохая фикстура иконки не поймана: ${name}`);
  }
  // таблица UA красная на заглушках (всегда true / всегда false)
  if (checkDetector(() => true).length === 0)
    errs.push('таблица UA не поймала заглушку «всегда true»');
  if (checkDetector(() => false).length === 0)
    errs.push('таблица UA не поймала заглушку «всегда false»');
  bad.forEach((s, i) => check(s).length === 0 && errs.push(`bad[${i}] не пойман`));
  good.forEach((s, i) => check(s).length !== 0 && errs.push(`good[${i}] ложно красный`));
  ovBad.forEach((s, i) => checkOverrides(s).length === 0 && errs.push(`ovBad[${i}] не пойман`));
  ovGood.forEach(
    (s, i) => checkOverrides(s).length !== 0 && errs.push(`ovGood[${i}] ложно красный`),
  );
  if (errs.length > 0) {
    for (const e of errs) console.error(`${TAG} FAIL selftest: ${e}`);
    process.exit(1);
  }
  console.log(`${TAG} selftest OK`);
}

if (process.argv.includes('--selftest')) {
  selftest();
} else {
  const file = process.argv.find((a) => a.endsWith('.svelte')) ?? COMPONENT;
  const src = fs.readFileSync(file, 'utf8');
  const bad = [...check(src), ...checkIcon(src)].map((b) => `${path.basename(file)}: ${b}`);
  const { needsOwnCalendarIcon } = await import(
    pathToFileURL(path.join(SRC, 'lib/utils/calendarIcon.js')).href
  );
  bad.push(...checkDetector(needsOwnCalendarIcon).map((b) => `calendarIcon.js: ${b}`));
  const files = walk(SRC);
  for (const f of files) {
    bad.push(...checkOverrides(fs.readFileSync(f, 'utf8'), path.relative(SRC, f)));
  }
  if (bad.length > 0) {
    for (const b of bad) console.error(`${TAG} FAIL: ${b}`);
    process.exit(1);
  }
  console.log(`${TAG} OK (${files.length} файлов)`);
}
