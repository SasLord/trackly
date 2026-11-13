#!/usr/bin/env node
// [check-user-scoped-reset] Гейт D-39 (фаза 41.4, план 42): клиентское состояние
// не переживает смену пользователя в той же вкладке.
//
// Структурный: рантайм (смена входа без перезагрузки) он не доказывает —
// svelte-check и build его тоже не видят; живая проверка в плане 46.
//
// Правила:
//  [A] Любой модуль (.ts, .svelte.ts, а в .svelte — блок <script module>) с
//      модульным изменяемым состоянием (let/var, const = new Map|Set|[]|{}|$state)
//      И обращением к API обязан вызывать registerUserScopedReset(), либо стоять в
//      таблице EXEMPT с причиной. Правило распространяется и на обычные .ts
//      (repairLocks.ts: кэш места группы), не только на .svelte.ts.
//  [B] deviceStatuses и deviceTypes: регистрация сброса, обнуление state в 'idle',
//      страховка employee ДО запроса, проверка поколения перед записью в items
//      и в catch. placeContentEvents: регистрация сброса.
//  [C] App.svelte: $effect.pre читает authStore.user, зовёт resetUserScoped( внутри
//      себя; сброс хеша ?id= и очистка тостов стоят под условием «прежний
//      пользователь не null» (восстановление сессии null -> id хеш сохраняет).
//  [D] Счётный минимум: не меньше трёх модулей с регистрацией (против зелёного по
//      пустоте).
//
// Usage:
//   node scripts/check-user-scoped-reset.mjs
//   node scripts/check-user-scoped-reset.mjs --selftest

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { stripComments } from './_strip.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const SRC = path.resolve(__dirname, '../src');
const TAG = '[check-user-scoped-reset]';

const REGISTRY = 'lib/stores/userScoped.svelte.ts';
const STATUSES = 'lib/stores/deviceStatuses.svelte.ts';
const TYPES = 'lib/stores/deviceTypes.svelte.ts';
const EVENTS = 'lib/stores/placeContentEvents.svelte.ts';
const APP = 'App.svelte';

// Освобождения от правила [A]: файл -> причина (не короче 20 символов).
// Запись обязана совпасть с реальным файлом (иначе гейт красный).
export const EXEMPT = {
  'lib/stores/theme.svelte.ts': 'настройка браузера, не данные пользователя системы',
  'features/layout/layout-state.svelte.ts': 'состояние выдвижной панели, не данные с сервера',
  'lib/stores/toast.svelte.ts': 'не кэш данных; очищается отдельно (clearToasts) в App.svelte',
  'lib/stores/auth.svelte.ts': 'сам источник смены пользователя; обнуляется входом и выходом',
  'lib/api/ws.ts': 'refCount и обработчики соединения: их закрывают размонтируемые Layout',
  'features/attachments/attachmentEvents.svelte.ts':
    'фабрика без модульного состояния: всё внутри createAttachmentEvents',
  [REGISTRY]: 'сам реестр сбросов; хранит функции, а не данные пользователя',
};

/** Модульное состояние: верхний уровень (отступ indent пробелов). */
function hasModuleState(src, indent) {
  const sp = ` {${indent}}`;
  const reLet = new RegExp(`^${sp}(?:export )?(?:let|var)\\s+\\w+`, 'm');
  const reConst = new RegExp(
    `^${sp}(?:export )?const\\s+\\w+(?::[^=\\n]+)?\\s*=\\s*(?:new (?:Map|Set|WeakMap)|\\[\\]|\\{\\}|\\$state)`,
    'm',
  );
  return reLet.test(src) || reConst.test(src);
}

function usesApi(src) {
  return /from\s+['"](?:\$lib\/api\/[^'"]*|(?:\.{1,2}\/)+(?:[\w/]*\/)?api)['"]/.test(src);
}

/** Тело первого вызова `head(` ... парной `)`; null, если не нашли. */
function callBody(src, head) {
  const i = src.indexOf(head);
  if (i < 0) return null;
  let depth = 0;
  for (let k = i + head.length - 1; k < src.length; k++) {
    if (src[k] === '(') depth++;
    else if (src[k] === ')') {
      depth--;
      if (depth === 0) return src.slice(i, k + 1);
    }
  }
  return null;
}

/**
 * @param {Record<string,string>} files относительный путь от src -> исходник
 * @returns {string[]} нарушения
 */
export function check(files) {
  const bad = [];
  const strip = (p) => (files[p] === undefined ? null : stripComments(files[p]));

  // освобождения: причина и существование файла
  for (const [f, why] of Object.entries(EXEMPT)) {
    if (why.length < 20) bad.push(`[A] освобождение ${f} без внятной причины`);
    if (files[f] === undefined) bad.push(`[A] освобождение ${f} не совпало с реальным файлом`);
  }

  // [A]
  let registered = 0;
  for (const [p, raw] of Object.entries(files)) {
    if (p.endsWith('.test.ts') || p.startsWith('bindings')) continue;
    let scope = null;
    let indent = 0;
    if (p.endsWith('.svelte')) {
      const m = /<script\s[^>]*(?:module|context="module")[^>]*>([\s\S]*?)<\/script>/.exec(raw);
      if (m) {
        scope = stripComments(m[1]);
        indent = 2;
      }
    } else if (p.endsWith('.ts')) {
      scope = stripComments(raw);
    }
    if (scope === null) continue;
    const reg = /registerUserScopedReset\(/.test(scope);
    if (reg && p !== REGISTRY) registered++;
    if (EXEMPT[p] !== undefined) continue;
    if (hasModuleState(scope, indent) && usesApi(scope) && !reg)
      bad.push(`[A] ${p}: модульное состояние с обращением к API без registerUserScopedReset`);
  }

  // [B]
  for (const [p, store] of [
    [STATUSES, 'deviceStatusesStore'],
    [TYPES, 'deviceTypesStore'],
  ]) {
    const s = strip(p);
    if (s === null) {
      bad.push(`[B] нет файла ${p}`);
      continue;
    }
    const regBody = callBody(s, 'registerUserScopedReset(');
    if (!regBody) bad.push(`[B] ${p}: нет registerUserScopedReset(`);
    else if (!new RegExp(`${store}\\.state\\s*=\\s*'idle'`).test(regBody))
      bad.push(`[B] ${p}: сброс не возвращает state в 'idle'`);
    else if (!/generation\s*\+=\s*1/.test(regBody))
      bad.push(`[B] ${p}: сброс не увеличивает поколение`);
    const guard = s.indexOf("authStore.user?.role === 'employee'");
    const req = s.indexOf('inflight = devices');
    if (guard < 0 || req < 0 || guard > req)
      bad.push(`[B] ${p}: нет страховки employee до запроса (D-38)`);
    const gens = s.split(/gen\s*!==\s*generation/).length - 1;
    const write = s.indexOf(`${store}.items = rows`);
    const firstGuard = s.search(/gen\s*!==\s*generation/);
    if (write < 0 || firstGuard < 0 || firstGuard > write)
      bad.push(`[B] ${p}: запись в items без проверки поколения перед ней`);
    if (gens < 2) bad.push(`[B] ${p}: нет проверки поколения в catch`);
  }
  const ev = strip(EVENTS);
  if (ev === null || !/registerUserScopedReset\(/.test(ev))
    bad.push(`[B] ${EVENTS}: нет registerUserScopedReset(`);

  // [C]
  const app = strip(APP);
  if (app === null) bad.push(`[C] нет ${APP}`);
  else {
    const eff = callBody(app, '$effect.pre(');
    if (!eff) bad.push('[C] App.svelte: нет $effect.pre(...)');
    else {
      if (!/authStore\.user/.test(eff)) bad.push('[C] эффект не читает authStore.user');
      if (!/resetUserScoped\(\)/.test(eff))
        bad.push('[C] resetUserScoped() не вызывается внутри $effect.pre');
      if (!/prevUserId\s*!==\s*undefined/.test(eff))
        bad.push('[C] первый прогон эффекта не исключён из смены (prevUserId !== undefined)');
      const guarded = /prevUserId\s*!==\s*null\s*\)\s*\{([\s\S]*)\}/.exec(eff);
      if (!guarded) bad.push('[C] нет условия «прежний пользователь не null» для хеша и тостов');
      else {
        if (!/location\.hash\s*=\s*'#\/'/.test(guarded[1]))
          bad.push('[C] нет сброса хеша ?id= под условием prevUserId !== null');
        if (!/clearToasts\(\)/.test(guarded[1]))
          bad.push('[C] нет clearToasts() под условием prevUserId !== null');
      }
      // сброс хеша не должен стоять вне охранного условия
      const hashAt = eff.search(/location\.hash\s*=/);
      const guardAt = eff.search(/prevUserId\s*!==\s*null/);
      if (hashAt >= 0 && (guardAt < 0 || hashAt < guardAt))
        bad.push('[C] сброс хеша стоит вне условия prevUserId !== null (сломает перезагрузку)');
    }
  }

  // [D]
  if (registered < 3)
    bad.push(`[D] модулей с регистрацией сброса ${registered}, нужно не меньше 3`);
  return bad;
}

function walk(dir, out = []) {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, e.name);
    if (e.isDirectory()) walk(p, out);
    else if (/\.(ts|svelte)$/.test(e.name)) out.push(p);
  }
  return out;
}

function load() {
  const files = {};
  for (const f of walk(SRC))
    files[path.relative(SRC, f).split(path.sep).join('/')] = fs.readFileSync(f, 'utf8');
  return files;
}

function selftest() {
  const real = load();
  const count = (s, n) => s.split(n).length - 1;
  const mut = (key, find, repl) => {
    if (real[key] === undefined || count(real[key], find) !== 1) {
      console.error(`${TAG} FAIL: selftest: якорь «${find}» в ${key} не уникален или файла нет`);
      process.exit(1);
    }
    return { ...real, [key]: real[key].replace(find, repl) };
  };
  const results = [['real', check(real).length === 0, '']];
  const expect = (name, files, tag) => {
    const bad = check(files);
    results.push([name, bad.some((b) => b.startsWith(tag)), bad.join(' | ')]);
  };
  expect(
    'no-registration-statuses',
    mut(STATUSES, 'registerUserScopedReset(() => {', '((() => {'),
    '[B]',
  );
  expect('no-reset-call-in-app', mut(APP, 'resetUserScoped();', ''), '[C]');
  expect(
    'no-generation-check',
    mut(TYPES, '.then((rows) => {\n      if (gen !== generation) return;\n', '.then((rows) => {\n'),
    '[B]',
  );
  expect('hash-reset-outside-guard', mut(APP, 'if (prevUserId !== null) {', 'if (true) {'), '[C]');
  expect(
    'no-employee-guard',
    mut(STATUSES, "if (authStore.user?.role === 'employee') return Promise.resolve();", ''),
    '[B]',
  );
  // новый модуль без регистрации: обычный .ts и .svelte.ts
  expect(
    'new-plain-ts-store',
    {
      ...real,
      'lib/fake.ts':
        "import { x } from '$lib/api/devices';\nconst cache = new Map<number, string>();\nexport function f() { return x(cache); }\n",
    },
    '[A]',
  );
  expect(
    'new-svelte-ts-store',
    {
      ...real,
      'lib/stores/fake.svelte.ts':
        "import { x } from '$lib/api/devices';\nexport const s = $state({ a: 1 });\nexport function ensureLoaded() { return x(); }\n",
    },
    '[A]',
  );
  expect(
    'repairLocks-unregistered',
    mut('features/requests/repairLocks.ts', 'registerUserScopedReset(() => {', '((() => {'),
    '[A]',
  );
  const failed = results.filter(([, ok]) => !ok).map(([n, , d]) => `${n}${d ? ` (${d})` : ''}`);
  if (failed.length > 0) {
    console.error(`${TAG} FAIL: selftest не отличает плохие фикстуры: ${failed.join('; ')}`);
    process.exit(1);
  }
  console.log(`${TAG} selftest OK`);
}

if (process.argv.includes('--selftest')) {
  selftest();
} else {
  const bad = check(load());
  if (bad.length > 0) {
    for (const b of bad) console.error(`${TAG} FAIL: ${b}`);
    process.exit(1);
  }
  console.log(`${TAG} OK`);
}
