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
// Гейт структурный (чтение исходников). Рантайм раскладки доказывает только
// ручная проба scripts/probe-date-picker-chromium.mjs (настоящий Chromium).
//
// Usage:
//   node scripts/check-date-picker-layout.mjs
//   node scripts/check-date-picker-layout.mjs --selftest

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
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
  const bad = check(fs.readFileSync(file, 'utf8')).map((b) => `${path.basename(file)}: ${b}`);
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
