#!/usr/bin/env node
// [check-transport-tauri] Поведенческий гейт Tauri-ветки транспорта (фаза 42, ревью WR-02/WR-03).
//
// Зачем: check-transport-hooks исполняет только HTTP-ветку (isTauri=false читается при загрузке
// модуля). Здесь isTauri=true, исполняются НАСТОЯЩИЕ packages/transport/src/{client,ws}.ts, а
// подменены только `@tauri-apps/api/core` (invoke) и `@tauri-apps/api/event` (listen).
//
// Кейсы:
//   [C1] invoke отклонён {code:'UNAUTHORIZED'} -> onUnauthorized ровно раз, onForbidden нет, ошибка брошена;
//   [C2] то же в иных регистрах ('Unauthorized') и для FORBIDDEN/'Forbidden' -> свой хук (нормализация);
//   [C3] прочие коды, строка-отказ, успех -> ни один хук не вызван; без хуков ошибка просто бросается;
//   [C4] коды `UNAUTHORIZED`/`FORBIDDEN` в crates/trackly-core/src/error.rs совпадают со строками
//        клиента (звено «Rust -> строка», иначе хуки на десктопе молча не сработают);
//   [W1] disconnectWs() пока listen() ещё ждёт: когда listen() завершится, подписка снята
//        (гонка WR-02: подписка жила вечно);
//   [W2] после такой гонки новый connectWs() даёт ровно одну живую подписку (событие -> один вызов);
//   [W3] обычный путь: два потребителя делят одну подписку, снимается только последним release();
//   [W4] listen() отклонён -> connectWs бросает, счётчик возвращён (следующий connectWs зовёт listen снова);
//   [W5] событие `trackly-resync` (мост десктопа при Lagged, фаза 43) -> ровно один вызов onWsResync-обработчика.
// Соединение Tauri = ДВЕ подписки: `trackly-event` и `trackly-resync` (фаза 43, D-08).
//
// Usage: node scripts/check-transport-tauri.mjs [--selftest]

import fs from 'node:fs';
import path from 'node:path';
import { gateMain, installHarness, pkgUrl, SRC, until } from './_rune-harness.mjs';

const TAG = '[check-transport-tauri]';
const CLIENT = '../../packages/transport/src/client.ts';
const WS = '../../packages/transport/src/ws.ts';
const ERROR_RS = '../../crates/trackly-core/src/error.rs';

const STUBS = {
  './transport.svelte': 'export const isTauri = true;',
  '@tauri-apps/api/core': 'export const invoke = (...a) => globalThis.__invoke(...a);',
  '@tauri-apps/api/event': 'export const listen = (...a) => globalThis.__listen(...a);',
};

async function runCases(overrides) {
  installHarness({ stubs: STUBS, overrides });
  globalThis.window = {};
  const bad = [];
  const expect = (code, cond, msg) => {
    if (!cond) bad.push(`[${code}] ${msg}`);
  };

  // ---- client.ts: Tauri-ветка apiCall ----
  const client = await import(pkgUrl('transport', 'client.ts'));
  const calls = { un: 0, fb: 0 };
  const hooks = {
    onUnauthorized: () => {
      calls.un += 1;
    },
    onForbidden: () => {
      calls.fb += 1;
    },
  };
  client.setApiHooks(hooks);
  const rejectWith = async (rejection) => {
    calls.un = 0;
    calls.fb = 0;
    globalThis.__invoke = async () => {
      throw rejection;
    };
    try {
      await client.apiCall('x_cmd');
      return null;
    } catch (e) {
      return e;
    }
  };

  let err = await rejectWith({ code: 'UNAUTHORIZED', message: 'm' });
  expect('C1', calls.un === 1 && calls.fb === 0, `UNAUTHORIZED: un=${calls.un} fb=${calls.fb}`);
  expect('C1', err && err.code === 'UNAUTHORIZED', `ошибка не брошена: ${JSON.stringify(err)}`);

  for (const [code, un, fb] of [
    ['Unauthorized', 1, 0],
    ['unauthorized', 1, 0],
    ['FORBIDDEN', 0, 1],
    ['Forbidden', 0, 1],
  ]) {
    err = await rejectWith({ code, message: 'm' });
    expect('C2', calls.un === un && calls.fb === fb, `${code}: un=${calls.un} fb=${calls.fb}`);
    expect('C2', err && err.code === code, `${code}: ошибка не брошена`);
  }

  for (const rejection of [
    { code: 'NOT_FOUND', message: 'm' },
    { code: 'INTERNAL', message: 'm' },
    'boom',
    null,
    { code: 'UNAUTHORIZED' }, // без message: parseAppError не признаёт AppError
  ]) {
    err = await rejectWith(rejection);
    expect(
      'C3',
      calls.un === 0 && calls.fb === 0 && err && typeof err.code === 'string',
      `${JSON.stringify(rejection)}: un=${calls.un} fb=${calls.fb} err=${JSON.stringify(err)}`,
    );
  }
  calls.un = 0;
  calls.fb = 0;
  globalThis.__invoke = async () => 42;
  const ok = await client.apiCall('x_cmd');
  expect('C3', ok === 42 && calls.un === 0 && calls.fb === 0, 'успех вызвал хук или исказил ответ');
  client.setApiHooks({});
  err = await rejectWith({ code: 'FORBIDDEN', message: 'm' });
  expect('C3', err && err.code === 'FORBIDDEN', 'без хуков ошибка не брошена');
  client.setApiHooks(hooks);

  // [C4] Rust -> строка. error.rs читается с диска; мутации стенда применяем вручную.
  const rsPath = path.join(SRC, ERROR_RS);
  let rs = fs.readFileSync(rsPath, 'utf8');
  for (const [from, to] of overrides[rsPath] ?? []) rs = rs.replace(from, to);
  const rustCode = (variant) =>
    new RegExp(`Self::${variant}\\s*=>\\s*"([A-Z_]+)"`).exec(rs)?.[1] ?? null;
  for (const [variant, want] of [
    ['Unauthorized', 'UNAUTHORIZED'],
    ['Forbidden', 'FORBIDDEN'],
  ]) {
    expect(
      'C4',
      rustCode(variant)?.toUpperCase() === want,
      `error.rs: AppError::${variant} даёт '${rustCode(variant)}', клиент ждёт '${want}'`,
    );
  }

  // ---- ws.ts: подписка Tauri listen() ----
  const subs = new Set();
  let listenCalls = 0;
  let gate = null; // управление моментом завершения listen()
  globalThis.__listen = (name, cb) => {
    listenCalls += 1;
    const sub = { cb, name };
    const done = () => {
      subs.add(sub);
      return () => subs.delete(sub);
    };
    if (gate === 'reject') return Promise.reject(new Error('listen failed'));
    if (gate === 'hold') {
      gate = null; // держим только первый listen(): вторая подписка соединения проходит сразу
      return new Promise((res) => (globalThis.__resolveListen = () => res(done())));
    }
    return Promise.resolve(done());
  };
  const emit = () =>
    subs.forEach((s) => {
      if (s.name === 'trackly-event') s.cb({ payload: { type: 'employee_scope_changed' } });
    });
  const fresh = async (n) => {
    subs.clear();
    listenCalls = 0;
    gate = null;
    return import(pkgUrl('transport', 'ws.ts', `?case=${n}`));
  };

  // [W1] + [W2] гонка: disconnectWs() пока listen() ждёт
  let ws = await fresh('w1');
  gate = 'hold';
  const pending = ws.connectWs();
  expect('W1', await until(() => listenCalls === 1), 'listen() не вызван');
  ws.disconnectWs();
  globalThis.__resolveListen();
  await pending;
  await new Promise((r) => setTimeout(r, 20));
  expect('W1', subs.size === 0, `после гонки осталось подписок: ${subs.size} (утечка listen)`);
  gate = null;
  const rel = await ws.connectWs();
  let seen = 0;
  const off = ws.onWsEvent(() => {
    seen += 1;
  });
  emit();
  expect('W2', subs.size === 2 && seen === 1, `подписок ${subs.size}, доставок ${seen}: ждали 2/1`);
  // [W5] trackly-resync -> один ресинк
  let resyncs = 0;
  const offR = ws.onWsResync(() => {
    resyncs += 1;
  });
  [...subs].filter((x) => x.name === 'trackly-resync').forEach((x) => x.cb({ payload: null }));
  expect('W5', resyncs === 1, `trackly-resync: ресинков ${resyncs}, ждали 1`);
  offR();
  off();
  rel();
  expect('W2', subs.size === 0, 'release() последнего потребителя не снял подписку');

  // [W3] два потребителя, одна подписка
  ws = await fresh('w3');
  const r1 = await ws.connectWs();
  const r2 = await ws.connectWs();
  expect(
    'W3',
    listenCalls === 2 && subs.size === 2,
    `listen зван ${listenCalls}, подписок ${subs.size}`,
  );
  r1();
  expect('W3', subs.size === 2, 'первый release() снял общую подписку');
  r2();
  expect('W3', subs.size === 0, 'последний release() не снял подписку');

  // [W4] listen отклонён
  ws = await fresh('w4');
  gate = 'reject';
  let threw = false;
  try {
    await ws.connectWs();
  } catch {
    threw = true;
  }
  expect('W4', threw, 'connectWs не бросил при отклонённом listen');
  gate = null;
  const rel4 = await ws.connectWs();
  expect(
    'W4',
    listenCalls === 3 && subs.size === 2,
    `после отказа listen зван ${listenCalls}, подписок ${subs.size}`,
  );
  rel4();
  return bad;
}

const MUTANTS = [
  {
    name: 'client: регистр кода не нормализуется',
    file: CLIENT,
    from: 'err.code.toUpperCase()',
    to: 'err.code',
    expect: 'C2',
  },
  {
    name: 'client: UNAUTHORIZED не зовёт onUnauthorized',
    file: CLIENT,
    from: "      if (code === 'UNAUTHORIZED') {\n        hooks.onUnauthorized?.(err);",
    to: "      if (code === 'UNAUTHORIZED') {\n        void err;",
    expect: 'C1',
  },
  {
    name: 'client: FORBIDDEN зовёт onUnauthorized',
    file: CLIENT,
    from: "      } else if (code === 'FORBIDDEN') {\n        hooks.onForbidden?.(err);",
    to: "      } else if (code === 'FORBIDDEN') {\n        hooks.onUnauthorized?.(err);",
    expect: 'C2',
  },
  {
    name: 'client: хук на любой ошибке',
    file: CLIENT,
    from: "      } else if (code === 'FORBIDDEN') {",
    to: '      } else if (code) {',
    expect: 'C3',
  },
  {
    name: 'error.rs: код Unauthorized переименован',
    file: ERROR_RS,
    from: 'Self::Unauthorized => "UNAUTHORIZED"',
    to: 'Self::Unauthorized => "NOT_AUTHORIZED"',
    expect: 'C4',
  },
  {
    name: 'ws: подписка не снимается, если соединение снесли во время listen',
    file: WS,
    from: 'if (epoch !== myEpoch) {',
    to: 'if (false) {',
    expect: 'W1',
  },
  {
    name: 'ws: снос не сдвигает эпоху',
    file: WS,
    from: '  epoch += 1;\n',
    to: '',
    expect: 'W1',
  },
  {
    name: 'ws: последний release() не снимает подписку',
    file: WS,
    from: '    if (refCount === 0) teardown();',
    to: '    if (refCount === -1) teardown();',
    expect: 'W3',
  },
  {
    name: 'ws: trackly-resync не вызывает ресинк',
    file: WS,
    from: '          resyncAll();\n        });\n        const unlisten = () => {',
    to: '          void 0;\n        });\n        const unlisten = () => {',
    expect: 'W5',
  },
  {
    name: 'ws: отказ listen не возвращает счётчик',
    file: WS,
    from: '        if (epoch === myEpoch) refCount = Math.max(0, refCount - 1);\n',
    to: '',
    expect: 'W4',
  },
];

await gateMain({ tag: TAG, scriptUrl: import.meta.url, mutants: MUTANTS, runCases });
