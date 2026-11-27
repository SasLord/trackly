#!/usr/bin/env node
// [check-send-retry] Поведенческий гейт автоповторов и клиентского id (фаза 43, план 09; D-12, DLV-03).
//
// Зачем: повтор отправки обязан уходить с ТЕМ ЖЕ id, доменные отказы не повторяются, а id не
// пропадает в контексте без crypto.randomUUID (LAN по http). svelte-check/build этого не видят.
//
// Исполняет НАСТОЯЩИЙ packages/transport/src/sendWithRetry.ts через стенд _rune-harness.mjs;
// sleep инжектируется, реального ожидания нет.
//
// Кейсы:
//   [S1] успех с первой попытки: 1 вызов, пауз нет;
//   [S2] TypeError дважды, затем успех: 3 вызова, паузы [1000, 2000], одно замыкание/один id;
//   [S3] постоянный TypeError: 4 вызова, паузы [1000, 2000, 4000], бросается последняя ошибка;
//   [S4] доменный отказ (FORBIDDEN, VALIDATION): 1 вызов, без повторов, ошибка проброшена;
//   [S7] HTTP_422/HTTP_413 (ответ без формы AppError): 1 вызов, без повторов (WR-07);
//   [S8] HTTP_502/503/504: повторяются, паузы [1000, 2000];
//   [S9] программный TypeError не повторяется; «Failed to fetch»/«Load failed» повторяются;
//   [S10] {code:'NETWORK_ERROR'} (отказ fetch в apiCall) повторяется;
//   [S5] newClientId проходит ^[A-Za-z0-9_-]{1,64}$ при randomUUID, без него и без crypto;
//   [S6] два вызова newClientId различны.
//
// Usage: node scripts/check-send-retry.mjs [--selftest]

import { gateMain, installHarness, pkgUrl } from './_rune-harness.mjs';

const TAG = '[check-send-retry]';
const FILE = '../../packages/transport/src/sendWithRetry.ts';
const ID_RE = /^[A-Za-z0-9_-]{1,64}$/;

function withCrypto(value, fn) {
  const d = Object.getOwnPropertyDescriptor(globalThis, 'crypto');
  Object.defineProperty(globalThis, 'crypto', { value, configurable: true, writable: true });
  try {
    return fn();
  } finally {
    if (d) Object.defineProperty(globalThis, 'crypto', d);
    else delete globalThis.crypto;
  }
}

async function runCases(overrides) {
  installHarness({ overrides });
  const bad = [];
  const expect = (code, cond, msg) => {
    if (!cond) bad.push(`[${code}] ${msg}`);
  };
  const mod = await import(pkgUrl('transport', 'sendWithRetry.ts'));
  const mk = () => {
    const sleeps = [];
    return { sleeps, sleep: async (ms) => void sleeps.push(ms) };
  };
  const J = (x) => JSON.stringify(x);
  // Исключение внутри кейса — нарушение этого кейса, а не падение стенда.
  const guard = async (code, f) => {
    try {
      await f();
    } catch (e) {
      bad.push(`[${code}] кейс бросил ${e}`);
    }
  };

  // S1
  await guard('S1', async () => {
    const { sleeps, sleep } = mk();
    let n = 0;
    const r = await mod.sendWithRetry(async () => ++n, { sleep });
    expect('S1', n === 1 && r === 1 && sleeps.length === 0, `S1: вызовов ${n}, пауз ${J(sleeps)}`);
  });
  // S2: замыкание держит один id, fn запоминает его на каждой попытке
  await guard('S2', async () => {
    const { sleeps, sleep } = mk();
    const id = mod.newClientId();
    const seen = [];
    let n = 0;
    const r = await mod.sendWithRetry(
      async () => {
        seen.push(id);
        n += 1;
        if (n <= 2) throw new TypeError('Failed to fetch');
        return 'ok';
      },
      { sleep },
    );
    expect('S2', r === 'ok' && n === 3, `S2: результат ${r}, вызовов ${n}`);
    expect('S2', J(sleeps) === J([1000, 2000]), `S2: паузы ${J(sleeps)}`);
    expect('S2', new Set(seen).size === 1 && seen.length === 3, `S2: id менялся ${J(seen)}`);
  });
  // S3
  await guard('S3', async () => {
    const { sleeps, sleep } = mk();
    let n = 0;
    let thrown = null;
    let res;
    try {
      res = await mod.sendWithRetry(
        async () => {
          n += 1;
          throw new TypeError(`Failed to fetch ${n}`);
        },
        { sleep },
      );
    } catch (e) {
      thrown = e;
    }
    expect('S3', n === 4, `S3: вызовов ${n}`);
    expect('S3', J(sleeps) === J([1000, 2000, 4000]), `S3: паузы ${J(sleeps)}`);
    expect(
      'S3',
      thrown && thrown.message === 'Failed to fetch 4' && res === undefined,
      `S3: брошено ${thrown}`,
    );
  });
  // S4
  for (const code of ['FORBIDDEN', 'VALIDATION']) {
    const { sleeps, sleep } = mk();
    let n = 0;
    const err = { code, message: 'отказ' };
    let thrown = null;
    try {
      await mod.sendWithRetry(
        async () => {
          n += 1;
          throw err;
        },
        { sleep },
      );
    } catch (e) {
      thrown = e;
    }
    expect('S4', n === 1 && sleeps.length === 0, `S4 ${code}: вызовов ${n}, пауз ${J(sleeps)}`);
    expect('S4', thrown === err, `S4 ${code}: ошибка не проброшена как есть`);
  }
  // S7: HTTP-ответ без формы AppError — не сетевой сбой
  for (const code of ['HTTP_422', 'HTTP_413']) {
    await guard('S7', async () => {
      const { sleeps, sleep } = mk();
      let n = 0;
      const err = { code, message: 'Сервер вернул ошибку' };
      let thrown = null;
      try {
        await mod.sendWithRetry(
          async () => {
            n += 1;
            throw err;
          },
          { sleep },
        );
      } catch (e) {
        thrown = e;
      }
      expect('S7', n === 1 && sleeps.length === 0, `S7 ${code}: вызовов ${n}, пауз ${J(sleeps)}`);
      expect('S7', thrown === err, `S7 ${code}: ошибка не проброшена как есть`);
    });
  }
  // S8: шлюзовые отказы повторяются
  for (const code of ['HTTP_502', 'HTTP_503', 'HTTP_504']) {
    await guard('S8', async () => {
      const { sleeps, sleep } = mk();
      let n = 0;
      const r = await mod.sendWithRetry(
        async () => {
          n += 1;
          if (n <= 2) throw { code, message: 'шлюз' };
          return 'ok';
        },
        { sleep },
      );
      expect('S8', r === 'ok' && n === 3, `S8 ${code}: результат ${r}, вызовов ${n}`);
      expect('S8', J(sleeps) === J([1000, 2000]), `S8 ${code}: паузы ${J(sleeps)}`);
    });
  }
  // S9: программная ошибка не повторяется, отказы fetch повторяются
  await guard('S9', async () => {
    const { sleeps, sleep } = mk();
    let n = 0;
    let thrown = null;
    try {
      await mod.sendWithRetry(
        async () => {
          n += 1;
          throw new TypeError('x is not a function');
        },
        { sleep },
      );
    } catch (e) {
      thrown = e;
    }
    expect('S9', n === 1 && sleeps.length === 0, `S9: вызовов ${n}, пауз ${J(sleeps)}`);
    expect('S9', thrown instanceof TypeError, `S9: ошибка не проброшена`);
  });
  for (const msg of ['Failed to fetch', 'Load failed']) {
    await guard('S9', async () => {
      const { sleeps, sleep } = mk();
      let n = 0;
      await mod.sendWithRetry(
        async () => {
          n += 1;
          if (n === 1) throw new TypeError(msg);
          return 1;
        },
        { sleep },
      );
      expect('S9', n === 2 && J(sleeps) === J([1000]), `S9 «${msg}»: вызовов ${n}, паузы ${J(sleeps)}`);
    });
  }
  // S10
  await guard('S10', async () => {
    const { sleeps, sleep } = mk();
    let n = 0;
    const r = await mod.sendWithRetry(
      async () => {
        n += 1;
        if (n === 1) throw { code: 'NETWORK_ERROR', message: 'Нет связи с сервером' };
        return 'ok';
      },
      { sleep },
    );
    expect('S10', r === 'ok' && n === 2 && J(sleeps) === J([1000]), `S10: вызовов ${n}, паузы ${J(sleeps)}`);
  });
  // S5 / S6
  const real = globalThis.crypto;
  const variants = {
    randomUUID: real,
    'без randomUUID': { getRandomValues: (a) => real.getRandomValues(a) },
    'без crypto': undefined,
  };
  for (const [name, c] of Object.entries(variants)) {
    let a;
    let b;
    try {
      a = withCrypto(c, () => mod.newClientId());
      b = withCrypto(c, () => mod.newClientId());
    } catch (e) {
      bad.push(`[S5] ${name}: newClientId бросил ${e}`);
      continue;
    }
    expect('S5', typeof a === 'string' && ID_RE.test(a), `S5 ${name}: «${a}» не проходит проверку`);
    expect('S6', a !== b, `S6 ${name}: два вызова дали одно значение «${a}»`);
  }
  return bad;
}

const MUTANTS = [
  {
    name: 'программный TypeError считается сетевым сбоем (WR-07)',
    file: FILE,
    from: 'e instanceof TypeError && FETCH_FAILURE_RE.test(e.message)',
    to: 'e instanceof TypeError',
    expect: 'S9',
  },
  {
    name: 'любой HTTP_* считается сетевым сбоем (WR-07)',
    file: FILE,
    from: 'return RETRYABLE_CODES.has(code) || (e instanceof TypeError && FETCH_FAILURE_RE.test(e.message));',
    to: "return RETRYABLE_CODES.has(code) || code.startsWith('HTTP_');",
    expect: 'S7',
  },
  {
    name: 'повторы не выполняются',
    file: FILE,
    from: 'if (!retryable(e) || attempt >= delays.length) throw e;',
    to: 'throw e;',
    expect: 'S2',
  },
  {
    name: 'доменный отказ тоже повторяется',
    file: FILE,
    from: 'const retryable = opts.isRetryable ?? isNetworkFailure;',
    to: 'const retryable = opts.isRetryable ?? (() => true);',
    expect: 'S4',
  },
  {
    name: 'паузы нулевые',
    file: FILE,
    from: 'await sleep(delays[attempt]);',
    to: 'await sleep(0);',
    expect: 'S2',
  },
  {
    name: 'после исчерпания возвращается undefined вместо броска',
    file: FILE,
    from: 'attempt >= delays.length) throw e;',
    to: 'attempt >= delays.length) return undefined as T;',
    expect: 'S3',
  },
  {
    name: 'запасной путь getRandomValues снят',
    file: FILE,
    from: "if (c && typeof c.getRandomValues === 'function') {",
    to: "if (c && typeof c.getRandomValues === 'function') {\n    throw new Error('нет запасного пути');",
    expect: 'S5',
  },
  {
    name: 'id не уникален (константа)',
    file: FILE,
    from: "if (c && typeof c.randomUUID === 'function') return c.randomUUID();",
    to: "if (c && typeof c.randomUUID === 'function') return 'const-id';",
    expect: 'S6',
  },
];

await gateMain({ tag: TAG, scriptUrl: import.meta.url, mutants: MUTANTS, runCases });
