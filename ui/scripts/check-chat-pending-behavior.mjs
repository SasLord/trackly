#!/usr/bin/env node
// [check-chat-pending-behavior] Поведенческий гейт очереди неотправленных сообщений чата
// (фаза 43, план 10; D-11, D-12, DLV-03).
//
// Исполняет НАСТОЯЩИЕ pendingStore.svelte.ts + sendWithRetry.ts + userScoped.svelte.ts через
// стенд _rune-harness.mjs; заглушён только ./api (chat.send — управляемая очередь ответов).
// sleep инжектируется, реальных секунд не ждём.
//
// Кейсы:
//   [P1] успех: запись снята, 'sent', один вызов с валидным clientMsgId;
//   [P2] сетевой сбой на всех 4 попытках: запись 'failed', 4 вызова, РОВНО ОДИН clientMsgId,
//        text и draftIds сохранены;
//   [P3] retryMessage при исправной сети: тот же clientMsgId, запись снята, 'sent';
//   [P4] повторный сбой на retry: запись остаётся 'failed' с тем же id;
//   [P5] доменный отказ (rejectCode): запись снята, 'rejected', один вызов;
//   [P6] брошенная AppError FORBIDDEN: запись снята, 'error', один вызов;
//   [P7] retryMessage по записи в 'sending' игнорируется (null, без второго запроса);
//   [P8] pendingFor разделяет заявки; resetUserScoped очищает стор.
//
// Usage: node scripts/check-chat-pending-behavior.mjs [--selftest]

import { gateMain, installHarness, srcUrl, pkgUrl } from './_rune-harness.mjs';

const TAG = '[check-chat-pending-behavior]';
const PS = 'features/requests/chat/pendingStore.svelte.ts';
const ID_RE = /^[A-Za-z0-9_-]{1,64}$/;

const STUBS = {
  './api': `export const chat = { send: (...a) => globalThis.__h.send(...a) };`,
};

async function runCases(overrides) {
  const H = { calls: [], queue: [], send: null, gate: null };
  // queue: элементы 'net' | 'ok' | {reject} | {throw}; пустая очередь = ok
  H.send = async (requestId, text, draftIds, clientMsgId) => {
    H.calls.push({ requestId, text, draftIds, clientMsgId });
    if (H.gate) await H.gate;
    const next = H.queue.length > 0 ? H.queue.shift() : 'ok';
    if (next === 'net') throw new TypeError('Failed to fetch');
    if (next && next.throw) throw next.throw;
    if (next && next.reject) return { rejectCode: next.reject, message: null };
    return { rejectCode: null, message: { id: 100 + H.calls.length, text } };
  };
  globalThis.__h = H;
  installHarness({ stubs: STUBS, overrides });
  const scoped = await import(pkgUrl('kit', 'stores/userScoped.svelte.ts'));
  const st = await import(srcUrl(PS));

  const bad = [];
  const expect = (code, cond, msg) => {
    if (!cond) bad.push(`[${code}] ${msg}`);
  };
  const noSleep = { sleep: async () => {} };
  const ids = () => [...new Set(H.calls.map((c) => c.clientMsgId))];
  const reset = () => {
    H.calls = [];
    H.queue = [];
    H.gate = null;
    scoped.resetUserScoped();
  };
  const guard = async (code, f) => {
    try {
      await f();
    } catch (e) {
      bad.push(`[${code}] кейс бросил ${e && e.stack ? e.stack : e}`);
    }
  };

  // P1
  await guard('P1', async () => {
    reset();
    const r = await st.submitMessage(1, 'привет', [], noSleep);
    expect(
      'P1',
      r.kind === 'sent' && r.message.text === 'привет',
      `P1: результат ${JSON.stringify(r)}`,
    );
    expect('P1', st.pending.items.length === 0, 'P1: запись не снята');
    expect(
      'P1',
      H.calls.length === 1 && ID_RE.test(H.calls[0].clientMsgId ?? ''),
      `P1: вызовы ${JSON.stringify(H.calls)}`,
    );
  });

  // P2
  await guard('P2', async () => {
    reset();
    H.queue = ['net', 'net', 'net', 'net'];
    const r = await st.submitMessage(1, 'текст', [7, 8], noSleep);
    expect('P2', r.kind === 'failed', `P2: результат ${JSON.stringify(r)}`);
    expect('P2', H.calls.length === 4, `P2: вызовов ${H.calls.length}`);
    expect('P2', ids().length === 1, `P2: разных clientMsgId ${ids().length}`);
    const it = st.pending.items[0];
    expect(
      'P2',
      st.pending.items.length === 1 &&
        it.status === 'failed' &&
        it.text === 'текст' &&
        JSON.stringify(it.draftIds) === '[7,8]',
      `P2: запись ${JSON.stringify(st.pending.items)}`,
    );
  });

  // P3 / P4
  await guard('P3', async () => {
    reset();
    H.queue = ['net', 'net', 'net', 'net'];
    await st.submitMessage(1, 'текст', [], noSleep);
    const id = st.pending.items[0].clientMsgId;
    const before = H.calls.length;
    H.queue = ['net', 'net', 'net', 'net'];
    const r2 = await st.retryMessage(id, noSleep);
    expect('P4', r2 && r2.kind === 'failed', `P4: результат ${JSON.stringify(r2)}`);
    expect(
      'P4',
      st.pending.items.length === 1 &&
        st.pending.items[0].status === 'failed' &&
        st.pending.items[0].clientMsgId === id,
      `P4: запись ${JSON.stringify(st.pending.items)}`,
    );
    H.queue = [];
    const r3 = await st.retryMessage(id, noSleep);
    expect('P3', r3 && r3.kind === 'sent', `P3: результат ${JSON.stringify(r3)}`);
    expect('P3', st.pending.items.length === 0, 'P3: запись не снята');
    const after = H.calls.slice(before);
    expect(
      'P3',
      after.length > 0 && after.every((c) => c.clientMsgId === id) && ids().length === 1,
      `P3: id менялся ${JSON.stringify(ids())}`,
    );
  });

  // P5
  for (const code of ['closed', 'too_long', 'no_chat']) {
    await guard('P5', async () => {
      reset();
      H.queue = [{ reject: code }];
      const r = await st.submitMessage(1, 'x', [], noSleep);
      expect(
        'P5',
        r.kind === 'rejected' && r.code === code,
        `P5 ${code}: результат ${JSON.stringify(r)}`,
      );
      expect('P5', st.pending.items.length === 0, `P5 ${code}: запись осталась`);
      expect('P5', H.calls.length === 1, `P5 ${code}: вызовов ${H.calls.length}`);
    });
  }

  // P6
  await guard('P6', async () => {
    reset();
    H.queue = [{ throw: { code: 'FORBIDDEN', message: 'нет доступа' } }];
    const r = await st.submitMessage(1, 'x', [], noSleep);
    expect('P6', r.kind === 'error', `P6: результат ${JSON.stringify(r)}`);
    expect('P6', st.pending.items.length === 0, 'P6: запись осталась');
    expect('P6', H.calls.length === 1, `P6: вызовов ${H.calls.length}`);
  });

  // P7: второй retry, пока первый в полёте
  await guard('P7', async () => {
    reset();
    H.queue = ['net', 'net', 'net', 'net'];
    await st.submitMessage(1, 'x', [], noSleep);
    const id = st.pending.items[0].clientMsgId;
    const before = H.calls.length;
    let release;
    H.gate = new Promise((res) => (release = res));
    H.queue = [];
    const p1 = st.retryMessage(id, noSleep);
    const p2 = st.retryMessage(id, noSleep);
    release();
    const [, r2] = await Promise.all([p1, p2]);
    expect('P7', r2 === null, `P7: второй retry вернул ${JSON.stringify(r2)}`);
    expect('P7', H.calls.length - before === 1, `P7: запросов в полёте ${H.calls.length - before}`);
  });

  // P8
  await guard('P8', async () => {
    reset();
    H.queue = ['net', 'net', 'net', 'net', 'net', 'net', 'net', 'net'];
    await st.submitMessage(1, 'a', [], noSleep);
    await st.submitMessage(2, 'b', [], noSleep);
    expect(
      'P8',
      st.pendingFor(2).length === 1 &&
        st.pendingFor(2)[0].text === 'b' &&
        st.pendingFor(1).length === 1 &&
        st.pendingFor(1)[0].text === 'a',
      `P8: pendingFor ${JSON.stringify(st.pending.items)}`,
    );
    scoped.resetUserScoped();
    expect('P8', st.pending.items.length === 0, 'P8: resetUserScoped не очистил стор');
  });

  return bad;
}

const MUTANTS = [
  {
    name: 'новый id на каждую попытку',
    file: PS,
    from: 'chat.send(requestId, text, draftIds, clientMsgId)',
    to: 'chat.send(requestId, text, draftIds, newClientId())',
    expect: 'P2',
  },
  {
    name: 'запись снимается при сетевом сбое',
    file: PS,
    from: "setStatus(clientMsgId, 'failed');",
    to: 'removeItem(clientMsgId);',
    expect: 'P2',
  },
  {
    name: 'retry генерирует новый id',
    file: PS,
    from: "return dispatch({ ...item, status: 'sending' }, opts);",
    to: "return dispatch({ ...item, clientMsgId: newClientId(), status: 'sending' }, opts);",
    expect: 'P3',
  },
  {
    name: 'доменный отказ оставляет запись',
    file: PS,
    from: '    removeItem(clientMsgId);\n    if (outcome.rejectCode !== null',
    to: "    if (outcome.rejectCode !== null && outcome.rejectCode !== 'x') setStatus(clientMsgId, 'failed'); else removeItem(clientMsgId);\n    if (outcome.rejectCode !== null",
    expect: 'P5',
  },
  {
    name: 'resetUserScoped не очищает',
    file: PS,
    from: '  pending.items = [];\n});',
    to: '});',
    expect: 'P8',
  },
  {
    name: 'нет защиты от двойного retry',
    file: PS,
    from: "if (!item || item.status !== 'failed') return null;",
    to: 'if (!item) return null;',
    expect: 'P7',
  },
];

await gateMain({ tag: TAG, scriptUrl: import.meta.url, mutants: MUTANTS, runCases });
