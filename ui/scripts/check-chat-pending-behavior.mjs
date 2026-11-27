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
//   [P8] pendingFor разделяет заявки; resetUserScoped очищает стор;
//   [P9/P10] отказ/ошибка на ПОВТОРЕ оставляют запись 'failed' (CR-02);
//   [P11] discardUnheldDrafts не чистит черновики, удерживаемые очередью;
//   [P12] returnToDraft; [P13] restoreText; [P14] первичный отказ снимает запись;
//   [C1-C4] структурные правила ChatComposer/ChatPendingMessage.
//
// Usage: node scripts/check-chat-pending-behavior.mjs [--selftest]

import fs from 'node:fs';
import path from 'node:path';
import { stripComments } from './_strip.mjs';
import { gateMain, SRC, installHarness, srcUrl, pkgUrl } from './_rune-harness.mjs';

const TAG = '[check-chat-pending-behavior]';
const PS = 'features/requests/chat/pendingStore.svelte.ts';
const DP = 'features/requests/chat/draftPolicy.ts';
const CHAT = 'features/requests/chat/';
const ID_RE = /^[A-Za-z0-9_-]{1,64}$/;

const STUBS = {
  './api': `export const chat = { send: (...a) => globalThis.__h.send(...a), discardDrafts: async (...a) => { globalThis.__h.discards.push(a); } };`,
};

async function runCases(overrides) {
  const H = { calls: [], queue: [], send: null, gate: null, discards: [] };
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
  const pol = await import(srcUrl(DP));

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
    H.discards = [];
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

  const failOne = async (draftIds) => {
    reset();
    H.queue = ['net', 'net', 'net', 'net'];
    await st.submitMessage(1, 'текст', draftIds, noSleep);
    return st.pending.items[0].clientMsgId;
  };

  // P9: отказ на повторе не снимает запись
  await guard('P9', async () => {
    const id = await failOne([5]);
    H.queue = [{ reject: 'too_long' }];
    const r = await st.retryMessage(id, noSleep);
    expect('P9', r && r.kind === 'rejected', `P9: результат ${JSON.stringify(r)}`);
    const it = st.pending.items[0];
    expect(
      'P9',
      st.pending.items.length === 1 &&
        it.status === 'failed' &&
        it.text === 'текст' &&
        JSON.stringify(it.draftIds) === '[5]',
      `P9: запись ${JSON.stringify(st.pending.items)}`,
    );
  });

  // P10
  await guard('P10', async () => {
    const id = await failOne([5]);
    H.queue = [{ throw: { code: 'FORBIDDEN', message: 'нет доступа' } }];
    const r = await st.retryMessage(id, noSleep);
    expect('P10', r && r.kind === 'error', `P10: результат ${JSON.stringify(r)}`);
    expect(
      'P10',
      st.pending.items.length === 1 && st.pending.items[0].status === 'failed',
      `P10: запись ${JSON.stringify(st.pending.items)}`,
    );
  });

  // P11
  await guard('P11', async () => {
    await failOne([5]);
    await st.discardUnheldDrafts(1);
    expect('P11', H.discards.length === 0, `P11: очистка при удерживаемых черновиках ${H.discards.length}`);
    await st.discardUnheldDrafts(2);
    expect('P11', H.discards.length === 1 && H.discards[0][0] === 2, 'P11: другая заявка не очищена');
    await failOne([]);
    H.discards = [];
    await st.discardUnheldDrafts(1);
    expect('P11', H.discards.length === 1 && H.discards[0][0] === 1, 'P11: запись без черновиков заблокировала очистку');
    reset();
    await st.discardUnheldDrafts(1);
    expect('P11', H.discards.length === 1, 'P11: пустая очередь не очищена');
  });

  // P12
  await guard('P12', async () => {
    const id = await failOne([]);
    const t = st.returnToDraft(id);
    expect('P12', t === 'текст', `P12: вернулось ${JSON.stringify(t)}`);
    expect('P12', st.pending.items.length === 0, 'P12: запись не снята');
    expect('P12', st.returnToDraft('nope') === null, 'P12: неизвестный id не дал null');
  });

  // P13
  await guard('P13', async () => {
    expect('P13', pol.restoreText('привет', '') === 'привет', 'P13: пустое поле');
    expect('P13', pol.restoreText('привет', 'как дела') === 'привет\nкак дела', 'P13: склейка');
    expect('P13', pol.restoreText('', 'x') === 'x', 'P13: пустой возврат');
  });

  // P14: первичная отправка по-прежнему снимает запись
  await guard('P14', async () => {
    reset();
    H.queue = [{ reject: 'closed' }];
    const r1 = await st.submitMessage(1, 'x', [], noSleep);
    H.queue = [{ throw: { code: 'FORBIDDEN', message: 'нет' } }];
    const r2 = await st.submitMessage(1, 'y', [], noSleep);
    expect(
      'P14',
      r1.kind === 'rejected' && r2.kind === 'error' && st.pending.items.length === 0,
      `P14: ${JSON.stringify([r1, r2, st.pending.items])}`,
    );
  });

  // C1-C4: структурные правила компонентов
  const rd = (f) => stripComments(fs.readFileSync(path.join(SRC, CHAT, f), 'utf8'));
  {
    const comp = rd('ChatComposer.svelte');
    expect('C1', !comp.includes('chat.discardDrafts('), 'C1: ChatComposer зовёт chat.discardDrafts напрямую');
    const n = comp.split('discardUnheldDrafts(').length - 1;
    expect('C2', n >= 3, `C2: discardUnheldDrafts( встречается ${n} раз, нужно не менее 3`);
    const i = comp.indexOf('async function send()');
    const j = comp.indexOf('</script>', i);
    const body = i >= 0 ? comp.slice(i, j) : '';
    expect('C3', i >= 0 && !body.includes('clearDraft('), 'C3: send() вызывает clearDraft(');
    expect('C4', rd('ChatPendingMessage.svelte').includes('onToDraft'), 'C4: нет onToDraft в ChatPendingMessage');
  }

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
    from: "if (keepOnDomainFailure) setStatus(clientMsgId, 'failed');\n      else removeItem(clientMsgId);",
    to: "setStatus(clientMsgId, 'failed');",
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
  {
    name: 'повтор снова снимает запись при отказе',
    file: PS,
    from: "if (keepOnDomainFailure) setStatus(clientMsgId, 'failed');\n      else removeItem(clientMsgId);",
    to: 'removeItem(clientMsgId);',
    expect: 'P9',
  },
  {
    name: 'повтор снова снимает запись при ошибке',
    file: PS,
    from: "if (!keepOnDomainFailure) removeItem(clientMsgId);\n    else setStatus(clientMsgId, 'failed');",
    to: 'removeItem(clientMsgId);',
    expect: 'P10',
  },
  {
    name: 'discardUnheldDrafts всегда чистит',
    file: PS,
    from: 'if (holds) return;',
    to: '',
    expect: 'P11',
  },
  {
    name: 'returnToDraft не снимает запись',
    file: PS,
    from: 'removeItem(item.clientMsgId);\n  return item.text;',
    to: 'return item.text;',
    expect: 'P12',
  },
  {
    name: 'restoreText без разделителя',
    file: DP,
    from: 'return `${sent}\\n${current}`;',
    to: 'return `${sent}${current}`;',
    expect: 'P13',
  },
  {
    name: 'композер зовёт chat.discardDrafts',
    file: CHAT + 'ChatComposer.svelte',
    from: 'await discardUnheldDrafts(sentFor);\n          } catch {\n            // Осиротевшие',
    to: 'await chat.discardDrafts(sentFor);\n          } catch {\n            // Осиротевшие',
    expect: 'C1',
  },
  {
    name: 'send() вызывает clearDraft',
    file: CHAT + 'ChatComposer.svelte',
    from: "const sentText = draft.text;",
    to: "const sentText = draft.text; clearDraft();",
    expect: 'C3',
  },
  {
    name: 'нет кнопки onToDraft',
    file: CHAT + 'ChatPendingMessage.svelte',
    from: 'onToDraft: (_clientMsgId: string) => void;',
    to: '',
    expect: 'C4',
  },
];

await gateMain({ tag: TAG, scriptUrl: import.meta.url, mutants: MUTANTS, runCases });
