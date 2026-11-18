#!/usr/bin/env node
// [check-chat-effects] Структурный гейт эффектов чата заявки (фаза 41.5, план 18).
//
// Дефект UAT 41.5 (тест 8): эффект сброса черновика в RequestChat вызывал
// clearDraft() без untrack. clearDraft читает и пишет draft.chips, эффект
// подписывался на них и перезапускался бесконечно (effect_update_depth_exceeded).
// svelte-check, eslint и build рантайм-ошибок рун не видят, поэтому проверка
// структурная: вызов любой функции записи из draftStore внутри $effect / $effect.pre
// обязан лежать внутри untrack( ... ) в том же теле эффекта (позиционно).
//
// Правила:
//   [E0] обязательные файлы присутствуют (отсутствие = красный, не «пропуск»);
//   [E1] список функций записи берётся из draftStore.svelte.ts (export function),
//        обязаны быть clearDraft, useDraftFor, addFiles, removeChip; пусто = красный;
//   [E2] в .svelte из features/requests/ и features/requests/chat/ вызов функции
//        записи внутри тела $effect вне untrack( = красный (кроме EXEMPT);
//   [E3] счётный минимум: найдено не меньше двух эффектов с безопасным вызовом
//        (RequestChat — сброс черновика, RequestDetail — useDraftFor);
//   [E4] запись EXEMPT обязана соответствовать реальному голому вызову (иначе устарела).
//
// Ограничение: вызов через промежуточную функцию, объявленную вне эффекта, гейт не видит.
//
// Usage: node scripts/check-chat-effects.mjs [--selftest]

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { stripComments } from './_strip.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const TAG = '[check-chat-effects]';
const SRC = path.resolve(__dirname, '../src');
const REQ = 'features/requests/';
const CHAT = `${REQ}chat/`;
const STORE = `${CHAT}draftStore.svelte.ts`;
const REQUIRED = [
  `${CHAT}RequestChat.svelte`,
  `${CHAT}ChatComposer.svelte`,
  `${REQ}RequestDetail.svelte`,
  STORE,
];
const MUST_WRITERS = ['clearDraft', 'useDraftFor', 'addFiles', 'removeChip'];

// Освобождения: файл -> причина. ChatComposer: эффект охраняется условием
// «черновик непуст», а clearDraft идемпотентен, поэтому цикла нет (запись
// происходит один раз и условие гаснет). Правка ChatComposer — в плане 21/27.
export const EXEMPT = {
  [`${CHAT}ChatComposer.svelte`]:
    'эффект под условием непустого черновика, clearDraft идемпотентен — сходится за один проход',
};

function readFiles() {
  const files = {};
  for (const rel of REQUIRED) {
    const p = path.join(SRC, rel);
    if (fs.existsSync(p)) files[rel] = fs.readFileSync(p, 'utf8');
  }
  for (const d of [REQ, CHAT]) {
    const dir = path.join(SRC, d);
    if (!fs.existsSync(dir)) continue;
    for (const f of fs.readdirSync(dir)) {
      if (f.endsWith('.svelte')) files[d + f] = fs.readFileSync(path.join(dir, f), 'utf8');
    }
  }
  return files;
}

/** Индекс парной закрывающей `)` для `(` на позиции open. */
function matchParen(src, open) {
  let depth = 0;
  for (let i = open; i < src.length; i++) {
    if (src[i] === '(') depth++;
    else if (src[i] === ')' && --depth === 0) return i;
  }
  return -1;
}

/** Тела эффектов: [{start, end}] — содержимое скобок $effect( / $effect.pre(. */
function effectBodies(src) {
  const out = [];
  const re = /\$effect(?:\.pre)?\s*\(/g;
  let m;
  while ((m = re.exec(src))) {
    const open = m.index + m[0].length - 1;
    const close = matchParen(src, open);
    if (close > 0) out.push({ start: open + 1, end: close });
  }
  return out;
}

function untrackRanges(src, start, end) {
  const out = [];
  const re = /\buntrack\s*\(/g;
  re.lastIndex = start;
  let m;
  while ((m = re.exec(src)) && m.index < end) {
    const open = m.index + m[0].length - 1;
    const close = matchParen(src, open);
    if (close > 0) out.push({ start: open, end: close });
  }
  return out;
}

export function writers(storeSrc) {
  const s = stripComments(storeSrc);
  return [...s.matchAll(/export\s+function\s+(\w+)/g)].map((m) => m[1]);
}

const count = (s, sub) => s.split(sub).length - 1;

export function check(files) {
  const v = [];
  const fail = (code, msg) => v.push({ code, msg });
  for (const rel of REQUIRED) {
    if (files[rel] === undefined) fail('E0', `обязательный файл отсутствует: ${rel}`);
  }
  const names = files[STORE] === undefined ? [] : writers(files[STORE]);
  if (names.length === 0) fail('E1', 'не найдено ни одной функции записи в draftStore');
  for (const n of MUST_WRITERS) {
    if (files[STORE] !== undefined && !names.includes(n))
      fail('E1', `draftStore не экспортирует ${n}`);
  }
  let safe = 0;
  const exemptUsed = new Set();
  for (const [rel, raw] of Object.entries(files)) {
    if (!rel.endsWith('.svelte')) continue;
    if (!rel.startsWith(REQ)) continue;
    const src = stripComments(raw);
    // Локальные псевдонимы импорта: `addFiles as storeAddFiles`.
    const callNames = new Set(names);
    for (const m of src.matchAll(/\b(\w+)\s+as\s+(\w+)/g)) {
      if (names.includes(m[1])) callNames.add(m[2]);
    }
    for (const body of effectBodies(src)) {
      const ranges = untrackRanges(src, body.start, body.end);
      for (const n of callNames) {
        const re = new RegExp(`(?<![\\w.])${n}\\s*\\(`, 'g');
        re.lastIndex = body.start;
        let m;
        while ((m = re.exec(src)) && m.index < body.end) {
          const inside = ranges.some((r) => m.index > r.start && m.index < r.end);
          if (inside) {
            safe++;
          } else if (EXEMPT[rel]) {
            exemptUsed.add(rel);
          } else {
            fail('E2', `${rel}: ${n}( внутри $effect вне untrack( — риск цикла эффекта`);
          }
        }
      }
    }
  }
  for (const rel of Object.keys(EXEMPT)) {
    if (!exemptUsed.has(rel)) fail('E4', `EXEMPT устарел: в ${rel} нет голого вызова в эффекте`);
  }
  if (safe < 2) fail('E3', `безопасных эффектов с вызовом записи найдено ${safe}, нужно >= 2`);
  return v;
}

function selftest(files) {
  const real = check(files);
  if (real.length > 0) {
    for (const x of real) console.error(`${TAG} FAIL — [${x.code}] ${x.msg}`);
    console.error(`${TAG} SELFTEST FAIL — реальные файлы не зелёные`);
    process.exit(1);
  }
  const swap = (rel, from, to) => () => {
    const n = count(files[rel], from);
    if (n !== 1) {
      console.error(`${TAG} SELFTEST FAIL — якорь мутации не уникален (${n}): ${from}`);
      process.exit(1);
    }
    return { ...files, [rel]: files[rel].replace(from, to) };
  };
  const RC = `${CHAT}RequestChat.svelte`;
  const RD = `${REQ}RequestDetail.svelte`;
  const mutants = [
    {
      name: 'снят untrack в эффекте сброса RequestChat',
      expect: 'E2',
      make: swap(RC, 'untrack(() => clearDraft())', 'clearDraft()'),
    },
    {
      name: 'перестановка: clearDraft перед untrack в том же эффекте',
      expect: 'E2',
      make: swap(RC, 'untrack(() => clearDraft())', 'clearDraft(); untrack(() => void 0)'),
    },
    {
      name: 'снят untrack у useDraftFor в RequestDetail',
      expect: 'E2',
      make: swap(RD, 'untrack(() => useDraftFor(id))', 'useDraftFor(id)'),
    },
    {
      name: 'пустой список экспортов draftStore',
      expect: 'E1',
      make: () => ({ ...files, [STORE]: '// пусто\n' }),
    },
  ];
  let bad = 0;
  for (const m of mutants) {
    const codes = check(m.make()).map((x) => x.code);
    const ok = codes.includes(m.expect);
    console.log(`${TAG} мутант «${m.name}»: ${ok ? 'обнаружен' : 'НЕ ОБНАРУЖЕН'} [${m.expect}]`);
    if (!ok) bad++;
  }
  if (bad > 0) process.exit(1);
  console.log(`${TAG} SELFTEST PASS — реальные файлы зелёные, ${mutants.length} мутантов красные`);
}

function main() {
  const files = readFiles();
  if (process.argv.includes('--selftest')) return selftest(files);
  const violations = check(files);
  if (violations.length > 0) {
    for (const x of violations) console.error(`${TAG} FAIL — [${x.code}] ${x.msg}`);
    process.exit(1);
  }
  console.log(`${TAG} PASS — 0 нарушений`);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) main();
