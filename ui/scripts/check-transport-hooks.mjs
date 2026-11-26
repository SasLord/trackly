#!/usr/bin/env node
// [check-transport-hooks] Поведенческий гейт хуков 401/403 транспорта (фаза 42, план 15; D-17, T-42-15-01).
//
// Зачем: реакции на 401/403 раньше жили внутри apiCall (authStore/pushToast), при выносе транспорта
// в packages/transport они стали внедряемыми хуками. Потеря вызова хука или регистрации в main.ts
// молча ломает выход из сессии и тост 403; svelte-check/build этого не видят (хуки необязательные).
//
// Исполняет НАСТОЯЩИЙ packages/transport/src/client.ts через стенд _rune-harness.mjs; подменён
// только глобальный fetch. Ветка HTTP (LAN-браузер). Tauri-ветка invoke этим гейтом НЕ покрыта:
// признак isTauri читается при загрузке модуля, второй загрузки в одном процессе нет (UNVERIFIED).
//
// Кейсы:
//   [T1] HTTP 401 -> onUnauthorized ровно один раз, onForbidden не вызван, apiCall бросает AppError;
//   [T2] HTTP 403 -> onForbidden ровно один раз, onUnauthorized не вызван, ошибка брошена;
//   [T3] HTTP 500 и успех -> ни один хук не вызван;
//   [T4] без зарегистрированных хуков 401/403 просто бросают ошибку (безопасное значение по умолчанию);
//   [T5] main.ts регистрирует setApiHooks с onUnauthorized и onForbidden ДО mount(...).
//
// Usage: node scripts/check-transport-hooks.mjs [--selftest]

import fs from 'node:fs';
import path from 'node:path';
import { gateMain, installHarness, pkgUrl, SRC } from './_rune-harness.mjs';

const TAG = '[check-transport-hooks]';
const CLIENT = '../../packages/transport/src/client.ts';

async function runCases(overrides) {
  // transport.svelte.ts без рун, но `.svelte.ts` идёт через compileModule и тянет svelte, которого у пакета нет.
  installHarness({ stubs: { './transport.svelte': 'export const isTauri = false;' }, overrides });
  globalThis.window = {};
  const bad = [];
  const expect = (code, cond, msg) => {
    if (!cond) bad.push(`[${code}] ${msg}`);
  };
  const mod = await import(pkgUrl('transport', 'client.ts'));
  const mkRes = (status, body, ct = 'application/json') => ({
    ok: status >= 200 && status < 300,
    status,
    headers: { get: () => ct },
    json: async () => body,
  });
  const call = async (res) => {
    globalThis.fetch = async () => res;
    try {
      await mod.apiCall('x_cmd');
      return null;
    } catch (e) {
      return e;
    }
  };
  const calls = { un: 0, fb: 0 };
  const reset = () => {
    calls.un = 0;
    calls.fb = 0;
  };
  mod.setApiHooks({
    onUnauthorized: () => {
      calls.un += 1;
    },
    onForbidden: () => {
      calls.fb += 1;
    },
  });

  let err = await call(mkRes(401, { code: 'UNAUTHORIZED', message: 'm' }));
  expect('T1', calls.un === 1 && calls.fb === 0, `401: un=${calls.un} fb=${calls.fb}`);
  expect('T1', err && err.code === 'UNAUTHORIZED', `401: ошибка ${JSON.stringify(err)}`);
  reset();
  err = await call(mkRes(403, { code: 'FORBIDDEN', message: 'm' }));
  expect('T2', calls.fb === 1 && calls.un === 0, `403: un=${calls.un} fb=${calls.fb}`);
  expect('T2', err && err.code === 'FORBIDDEN', `403: ошибка ${JSON.stringify(err)}`);
  reset();
  err = await call(mkRes(500, { code: 'X', message: 'm' }));
  expect('T3', calls.un === 0 && calls.fb === 0 && err, `500: un=${calls.un} fb=${calls.fb}`);
  const ok = await call(mkRes(200, { a: 1 }));
  expect('T3', ok === null && calls.un === 0 && calls.fb === 0, 'успех вызвал хук или бросил');
  mod.setApiHooks({});
  err = await call(mkRes(401, { code: 'UNAUTHORIZED', message: 'm' }));
  expect('T4', err && err.code === 'UNAUTHORIZED', '401 без хуков: ошибка не брошена');
  err = await call(mkRes(403, { code: 'FORBIDDEN', message: 'm' }));
  expect('T4', err && err.code === 'FORBIDDEN', '403 без хуков: ошибка не брошена');

  let main = fs.readFileSync(path.join(SRC, 'main.ts'), 'utf8');
  // main.ts читается с диска, а не импортируется: мутации стенда применяем вручную.
  for (const [from, to] of overrides[path.join(SRC, 'main.ts')] ?? [])
    main = main.replace(from, to);
  const reg = main.indexOf('setApiHooks(');
  const mount = main.indexOf('mount(App');
  const block = reg >= 0 ? main.slice(reg, mount) : '';
  expect(
    'T5',
    reg >= 0 && mount > reg && /onUnauthorized/.test(block) && /onForbidden/.test(block),
    'main.ts не регистрирует onUnauthorized/onForbidden до mount',
  );
  return bad;
}

const MUTANTS = [
  {
    name: 'HTTP 401 не зовёт onUnauthorized',
    file: CLIENT,
    from: '      hooks.onUnauthorized?.(err);\n    }\n    // 403',
    to: '    }\n    // 403',
    expect: 'T1',
  },
  {
    name: 'HTTP 403 зовёт onUnauthorized вместо onForbidden',
    file: CLIENT,
    from: '      hooks.onForbidden?.(err);\n    }\n    throw err;',
    to: '      hooks.onUnauthorized?.(err);\n    }\n    throw err;',
    expect: 'T2',
  },
  {
    name: 'хук зовётся на любой ошибке',
    file: CLIENT,
    from: '    if (res.status === 401) {',
    to: '    if (res.status >= 400) {',
    expect: 'T3',
  },
  {
    name: 'без хуков ошибка глотается',
    file: CLIENT,
    from: '    throw err;\n  }\n  // Binary',
    to: '    if (hooks.onForbidden) throw err;\n  }\n  // Binary',
    expect: 'T4',
  },
  {
    name: 'main.ts: регистрация хуков снята',
    file: 'main.ts',
    from: 'setApiHooks({',
    to: 'void ({',
    expect: 'T5',
  },
];

await gateMain({ tag: TAG, scriptUrl: import.meta.url, mutants: MUTANTS, runCases });
