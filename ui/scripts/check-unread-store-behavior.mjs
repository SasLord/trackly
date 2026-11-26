#!/usr/bin/env node
// [check-unread-store-behavior] Поведенческий гейт счётчиков непрочитанного
// (фаза 41.5, планы 11/25; Nyquist 41.5 G5; MSG-04, D-18/D-19/D-24, D-G2/D-G3).
//
// Исполняет НАСТОЯЩИЕ unreadStore.svelte.ts + toast.svelte.ts + userScoped.svelte.ts через
// стенд _rune-harness.mjs; заглушены только WS (onWsEvent) и API чата (unreadSummary/setMuted).
//
// Кейсы:
//   [U1] сводка с сервера: byRequest только с count > 0, requestsWithUnread = число таких заявок,
//        mutedIds из mutedRequestIds, loaded = true;
//   [U2] setActiveChat(id) обнуляет счётчик заявки и пересчитывает сайдбар; при открытом чате
//        перечитанная сводка эту заявку не показывает;
//   [U3] WS chat_message ЧУЖОЙ заявки → перечитывание сводки; по ОТКРЫТОЙ заявке — нет;
//   [U4] request_status_changed / attachments_changed / new_request / chat_read_changed → перечитывание;
//        неизвестное событие — нет; после stop подписка снята;
//   [U5] toggleMute оптимистичен; при ошибке откат + тост «Не удалось изменить оповещения»;
//        повторный вызов во время полёта → false и без второго запроса;
//   [U6] resetUserScoped обнуляет стор, а запоздавший ответ прежнего пользователя его не перезаписывает;
//   [U7] битый ответ (HTML-строка spa_fallback, нечисловые поля) прежние числа не трогает.
//
// Usage: node scripts/check-unread-store-behavior.mjs [--selftest]

import { gateMain, installHarness, pkgUrl, srcUrl, until } from './_rune-harness.mjs';

const TAG = '[check-unread-store-behavior]';
const US = 'features/requests/chat/unreadStore.svelte.ts';

const STUBS = {
  '$lib/api/ws': `export function onWsEvent(h) { const s = globalThis.__h.ws; s.add(h); return () => s.delete(h); }`,
  './api': `export const chat = {
    unreadSummary: (...a) => globalThis.__h.api.unreadSummary(...a),
    setMuted: (...a) => globalThis.__h.api.setMuted(...a),
  };`,
};

async function runCases(overrides) {
  const H = {
    ws: new Set(),
    calls: 0,
    summary: { items: [], mutedRequestIds: [] },
    api: {
      unreadSummary: async () => {
        H.calls += 1;
        return H.summary;
      },
      setMuted: async () => {},
    },
  };
  globalThis.__h = H;
  installHarness({ stubs: STUBS, overrides });
  const toast = await import(pkgUrl('kit', 'stores/toast.svelte.ts'));
  const scoped = await import(pkgUrl('kit', 'stores/userScoped.svelte.ts'));
  const u = await import(srcUrl(US));
  const s = u.unreadStore;

  const bad = [];
  const expect = (code, cond, msg) => {
    if (!cond) bad.push(`[${code}] ${msg}`);
  };
  const emit = (ev) => {
    for (const h of [...H.ws]) h(ev);
  };
  const snap = () => JSON.stringify({ b: s.byRequest, n: s.requestsWithUnread });

  // U1
  H.summary = {
    items: [
      { requestId: 1, count: 3 },
      { requestId: 2, count: 0 },
      { requestId: 3, count: 1 },
    ],
    mutedRequestIds: [3, 'x'],
  };
  await u.refreshUnread();
  expect('U1', u.unreadFor(1) === 3 && u.unreadFor(3) === 1, `счётчики: ${snap()}`);
  expect('U1', !(2 in s.byRequest) && u.unreadFor(2) === 0, `нулевой счётчик в карте: ${snap()}`);
  expect(
    'U1',
    s.requestsWithUnread === 2,
    `requestsWithUnread = ${s.requestsWithUnread}, ожидалось 2`,
  );
  expect('U1', u.isMuted(3) && !u.isMuted(1), `mutedIds: ${JSON.stringify(s.mutedIds)}`);
  expect('U1', s.loaded === true, 'loaded не выставлен');

  // U2
  u.setActiveChat(1);
  expect('U2', u.unreadFor(1) === 0, `счётчик открытой заявки: ${u.unreadFor(1)}`);
  expect('U2', s.requestsWithUnread === 1, `сайдбар после открытия: ${s.requestsWithUnread}`);
  await u.refreshUnread();
  expect(
    'U2',
    u.unreadFor(1) === 0 && s.requestsWithUnread === 1,
    `сводка вернула открытую заявку: ${snap()}`,
  );
  u.setActiveChat(null);
  await u.refreshUnread();
  expect('U2', u.unreadFor(1) === 3, 'после закрытия чата счётчик не вернулся');

  // U3, U4: подписка
  const stop = u.startUnreadRealtime();
  await until(() => H.calls > 0);
  const waitCall = async (code, ev, want) => {
    const before = H.calls;
    emit(ev);
    // Дебаунс стора — 150 мс: 500 мс хватает и на «было», и на доказательство «не было».
    const got = await until(() => H.calls > before, 500);
    expect(
      code,
      got === want,
      `${ev.type} (заявка ${ev.requestId ?? ev.parentId ?? '-'}): перечитывание ${got ? 'было' : 'не было'}`,
    );
  };
  u.setActiveChat(7);
  await waitCall('U3', { type: 'chat_message', requestId: 8, actorUserId: 2 }, true);
  await waitCall('U3', { type: 'chat_message', requestId: 7, actorUserId: 2 }, false);
  u.setActiveChat(null);
  await waitCall('U4', { type: 'request_status_changed', requestId: 8 }, true);
  await waitCall('U4', { type: 'attachments_changed', parentKind: 'request', parentId: 8 }, true);
  await waitCall('U4', { type: 'new_request', requestId: 9 }, true);
  await waitCall('U4', { type: 'chat_read_changed', requestId: 8, userId: 1 }, true);
  await waitCall('U4', { type: 'printer_status', printerId: 1 }, false);
  stop();
  expect('U4', H.ws.size === 0, `после stop подписок WS: ${H.ws.size}`);

  // U5
  toast.clearToasts();
  let release;
  H.api.setMuted = () =>
    new Promise((_, rej) => {
      release = () => rej(new Error('сеть'));
    });
  const p1 = u.toggleMute(1, true);
  expect('U5', u.isMuted(1), 'оптимистичное включение не применено');
  const p2 = await u.toggleMute(1, false);
  expect('U5', p2 === false, 'повторный toggleMute во время полёта не вернул false');
  expect('U5', u.isMuted(1), 'повторный вызов во время полёта изменил состояние');
  release();
  const r1 = await p1;
  expect(
    'U5',
    r1 === false && !u.isMuted(1),
    `ошибка сервера: результат ${r1}, muted=${u.isMuted(1)} (ожидался откат)`,
  );
  expect(
    'U5',
    toast.toastStore.items.some(
      (t) => t.kind === 'error' && t.message === 'Не удалось изменить оповещения',
    ),
    'нет тоста об ошибке',
  );
  H.api.setMuted = async () => {};
  expect(
    'U5',
    (await u.toggleMute(1, true)) === true && u.isMuted(1),
    'успешный toggleMute не включил',
  );

  // U6
  let late;
  H.api.unreadSummary = () =>
    new Promise((res) => {
      late = () => res({ items: [{ requestId: 42, count: 9 }], mutedRequestIds: [42] });
    });
  const pending = u.refreshUnread();
  u.setActiveChat(5);
  scoped.resetUserScoped();
  expect(
    'U6',
    Object.keys(s.byRequest).length === 0 &&
      s.requestsWithUnread === 0 &&
      s.loaded === false &&
      s.activeChatRequestId === null &&
      Object.keys(s.mutedIds).length === 0,
    `стор после смены пользователя: ${JSON.stringify(s)}`,
  );
  late();
  await pending;
  expect(
    'U6',
    u.unreadFor(42) === 0 && !u.isMuted(42),
    'запоздавший ответ прежнего пользователя перезаписал стор',
  );

  // U7
  H.api.unreadSummary = async () => ({ items: [{ requestId: 1, count: 2 }], mutedRequestIds: [] });
  await u.refreshUnread();
  const before = snap();
  for (const broken of [
    '<!doctype html><html>',
    { items: [{ requestId: '1', count: 5 }] },
    { items: null },
    null,
  ]) {
    H.api.unreadSummary = async () => broken;
    await u.refreshUnread();
    expect(
      'U7',
      snap() === before,
      `битый ответ ${JSON.stringify(broken)} изменил счётчики: ${snap()}`,
    );
  }
  return bad;
}

const MUTANTS = [
  {
    name: 'setActiveChat не пересчитывает сайдбар',
    file: US,
    from: '    unreadStore.byRequest = next;\n    recount();\n',
    to: '    unreadStore.byRequest = next;\n',
    expect: 'U2',
  },
  {
    name: 'инверсия: перечитывание только по ОТКРЫТОЙ заявке',
    file: US,
    from: 'if (unreadStore.activeChatRequestId === ev.requestId) return;',
    to: 'if (unreadStore.activeChatRequestId !== ev.requestId) return;',
    expect: 'U3',
  },
  {
    name: 'снята защита от запоздавшего ответа (token)',
    file: US,
    from: '    if (mine !== token) return;\n',
    to: '',
    expect: 'U6',
  },
  {
    name: 'откат toggleMute к новому значению вместо прежнего',
    file: US,
    from: '    unreadStore.mutedIds = withMuted(requestId, prev);',
    to: '    unreadStore.mutedIds = withMuted(requestId, muted);',
    expect: 'U5',
  },
  {
    name: 'нулевые счётчики попадают в карту',
    file: US,
    from: '      if (i.count > 0) map[i.requestId] = i.count;',
    to: '      if (i.count >= 0) map[i.requestId] = i.count;',
    expect: 'U1',
  },
  {
    name: 'перестановка: открытая заявка удаляется из сводки ПОСЛЕ присваивания',
    file: US,
    edits: [
      [
        '    if (unreadStore.activeChatRequestId !== null) {\n      delete map[unreadStore.activeChatRequestId];\n    }\n    unreadStore.byRequest = map;\n',
        '    unreadStore.byRequest = { ...map };\n    if (unreadStore.activeChatRequestId !== null) {\n      delete map[unreadStore.activeChatRequestId];\n    }\n',
      ],
    ],
    expect: 'U2',
  },
  {
    name: 'chat_read_changed не перечитывает сводку',
    file: US,
    from: "      case 'chat_read_changed':\n",
    to: '',
    expect: 'U4',
  },
  {
    name: 'сброс пользователя не чистит mutedIds',
    file: US,
    from: '  unreadStore.mutedIds = {};\n  muteInFlight.clear();',
    to: '  muteInFlight.clear();',
    expect: 'U6',
  },
];

await gateMain({ tag: TAG, scriptUrl: import.meta.url, mutants: MUTANTS, runCases });
