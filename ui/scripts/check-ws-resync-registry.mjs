#!/usr/bin/env node
// [check-ws-resync-registry] Реестровый гейт ресинка подписчиков WS (фаза 43, D-08).
//
// После обрыва сокета или Lagged события пропущены; клиент зовёт resyncAll(), и
// перечитать данные обязан КАЖДЫЙ, кто слушает onWsEvent. Забытый подписчик
// молча остаётся со старыми данными, поэтому гейт требует: в каждом файле
// (ui/src, packages/*/src, link-ui/src, кроме packages/transport/src/ws.ts)
// вызовов onWsResync( не меньше, чем вызовов onWsEvent(, либо файл записан в
// EXEMPTIONS с причиной. Комментарии вырезаются (stripComments), строка импорта
// без скобки не считается.
//
// Защита от «зелёного по пустоте»: найденных подписчиков не меньше
// MIN_SUBSCRIBERS (инвентарь от репозитория на 2026-11-28; поднимать при росте,
// не опускать). Устаревшее освобождение и мёртвая запись (у файла ресинк есть)
// тоже ошибки.
//
// Режимы: без аргументов — боевой скан; --selftest — мутанты M1-M7.

import fs from 'node:fs';
import path from 'node:path';
import { stripComments } from './_strip.mjs';
import { REPO, walk } from './_roots.mjs';

export const MIN_SUBSCRIBERS = 14;
const MIN_REASON = 20;
const WS_FILE = 'packages/transport/src/ws.ts';

/** путь (от корня репозитория) -> причина освобождения (>= 20 символов). */
export const EXEMPTIONS = {
  'ui/src/features/requests/chat/chatNotifications.svelte.ts':
    'тосты о новых сообщениях; перечитывать нечего, ресинк обязан быть тихим (D-09); счётчики перечитывает unreadStore',
};

const RE_EVENT = /\bonWsEvent\s*\(/g;
const RE_RESYNC = /\bonWsResync\s*\(/g;

const count = (re, s) => (s.match(re) ?? []).length;

/**
 * @param {Map<string,string>} files путь -> исходник
 * @param {Record<string,string>} exemptions
 * @returns {string[]} ошибки
 */
export function evaluate(files, exemptions) {
  const errors = [];
  let subscribers = 0;
  for (const [file, raw] of files) {
    if (file === WS_FILE) continue;
    const src = stripComments(raw);
    const n = count(RE_EVENT, src);
    const m = count(RE_RESYNC, src);
    if (n === 0) continue;
    subscribers++;
    if (file in exemptions) continue;
    if (m < n) errors.push(`${file}: подписчиков ${n}, ресинков ${m}`);
  }
  if (subscribers < MIN_SUBSCRIBERS) {
    errors.push(
      `найдено подписчиков ${subscribers} < MIN_SUBSCRIBERS=${MIN_SUBSCRIBERS} (гейт не видит инвентарь)`,
    );
  }
  for (const [file, reason] of Object.entries(exemptions)) {
    if (typeof reason !== 'string' || reason.trim().length < MIN_REASON) {
      errors.push(`${file}: причина освобождения короче ${MIN_REASON} символов`);
    }
    const raw = files.get(file);
    const src = raw === undefined ? '' : stripComments(raw);
    if (raw === undefined || count(RE_EVENT, src) === 0) {
      errors.push(`${file}: устаревшее освобождение (файла нет или нет onWsEvent)`);
    } else if (count(RE_RESYNC, src) > 0) {
      errors.push(`${file}: мёртвая запись освобождения (onWsResync уже есть)`);
    }
  }
  return errors;
}

function scanRepo() {
  const files = new Map();
  for (const abs of walk({ exts: ['.ts', '.svelte'] })) {
    const rel = path.relative(REPO, abs).split(path.sep).join('/');
    files.set(rel, fs.readFileSync(abs, 'utf8'));
  }
  return files;
}

// ---------- selftest ----------

function goodMap() {
  const files = new Map();
  for (let i = 0; i < MIN_SUBSCRIBERS; i++) {
    files.set(
      `ui/src/f${i}.svelte`,
      `import { onWsEvent, onWsResync } from 'x';\nconst a = onWsEvent(h);\nconst b = onWsResync(r);\n`,
    );
  }
  files.set(WS_FILE, 'export function onWsEvent(cb) {}\n');
  return files;
}

function selftest() {
  let fail = 0;
  const expect = (name, errors, needle) => {
    const ok = needle === null ? errors.length === 0 : errors.some((e) => e.includes(needle));
    if (!ok) {
      console.error(
        `SELFTEST FAIL ${name}: ожидалось ${needle ?? 'чисто'}, получено ${JSON.stringify(errors)}`,
      );
      fail++;
    }
  };
  const ex = { 'ui/src/f0.svelte': 'причина достаточной длины для записи' };

  expect('эталон', evaluate(goodMap(), {}), null);

  let m = goodMap();
  m.set('ui/src/f3.svelte', `const a = onWsEvent(h);\n`);
  expect('M1 нет ресинка', evaluate(m, {}), 'ui/src/f3.svelte');

  m = goodMap();
  m.set('ui/src/f4.svelte', `const a = onWsEvent(h);\n// onWsResync(r)\n/* onWsResync(r) */\n`);
  expect('M2 ресинк в комментарии', evaluate(m, {}), 'ui/src/f4.svelte');

  expect('M3a пустая карта', evaluate(new Map(), {}), 'MIN_SUBSCRIBERS');
  m = goodMap();
  m.delete('ui/src/f0.svelte');
  expect('M3b мало подписчиков', evaluate(m, {}), 'MIN_SUBSCRIBERS');

  m = goodMap();
  m.set('ui/src/f5.svelte', `onWsEvent(a);\nonWsEvent(b);\nonWsResync(r);\n`);
  expect('M4 два события, один ресинк', evaluate(m, {}), 'ui/src/f5.svelte');

  expect(
    'M5 устаревшее освобождение',
    evaluate(goodMap(), { 'ui/src/нет.svelte': 'причина достаточной длины для записи' }),
    'устаревшее',
  );
  m = goodMap();
  m.set('ui/src/f6.svelte', `import { onWsEvent } from 'x';\n`);
  expect(
    'M5b файл без onWsEvent',
    evaluate(m, { 'ui/src/f6.svelte': 'причина достаточной длины для записи' }),
    'устаревшее',
  );

  m = goodMap();
  m.set('ui/src/f7.svelte', `onWsEvent(a);\n`);
  expect('M6 короткая причина', evaluate(m, { 'ui/src/f7.svelte': 'коротко' }), 'короче');

  expect('M7 мёртвая запись', evaluate(goodMap(), ex), 'мёртвая запись');

  m = goodMap();
  m.set('ui/src/f8.svelte', `onWsEvent(a);\n`);
  expect(
    'освобождение работает',
    evaluate(m, { 'ui/src/f8.svelte': 'причина достаточной длины для записи' }),
    null,
  );

  if (fail) process.exit(1);
  console.log('[check-ws-resync-registry] selftest OK: эталон и мутанты M1-M7 пойманы');
}

if (process.argv.includes('--selftest')) {
  selftest();
} else {
  const files = scanRepo();
  const errors = evaluate(files, EXEMPTIONS);
  if (errors.length) {
    console.error('[check-ws-resync-registry] FAIL:');
    for (const e of errors) console.error('  ' + e);
    process.exit(1);
  }
  console.log('[check-ws-resync-registry] OK');
}
