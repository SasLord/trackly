#!/usr/bin/env node
// [check-chat-notify-behavior] Поведенческий гейт уведомлений о сообщениях чата
// (фаза 41.5, планы 11/25/29/33; Nyquist 41.5 G4; MSG-04, D-G1/D-G2, R3-5).
//
// Исполняет НАСТОЯЩИЕ chatNotifications.svelte.ts + unreadStore.svelte.ts + toast.svelte.ts +
// userScoped.svelte.ts + requestTarget.ts через стенд _rune-harness.mjs. Заглушены только
// внешние границы: WS (onWsEvent), authStore, API чата, systemNotify (фокус и показ ОС),
// svelte-spa-router (push). Подписка — через настоящий startChatRealtime().
//
// Кейсы:
//   [N1] чужое сообщение, окно в фокусе → ровно один info-тост «Открыть чат» со ссылкой
//        #/requests?id=N&tab=chat, системного уведомления нет;
//   [N2] окно без фокуса → тост И системное уведомление; activate() открывает заявку на вкладке «Чат»;
//   [N3] своё сообщение → тишина;  [N4] открыт чат этой заявки → тишина;
//   [N5] заявка заглушена (mutedRequestIds с сервера) → тишина, а соседняя — нет;
//   [N6] повторное сообщение при живом тосте → тост не дублируется; после снятия — новый;
//   [N7] dismissChatToast(id) снимает тост этой заявки и не трогает чужой; неизвестный id — нет-оп;
//   [N8] автор неизвестен → «Участник»; в тексте нет undefined/null;
//   [N9] не-chat события и смена пользователя (resetUserScoped) уведомлений не порождают;
//        после сброса тост прежнего пользователя не «держит» дедупликацию.
//
// Usage: node scripts/check-chat-notify-behavior.mjs [--selftest]

import { gateMain, installHarness, pkgUrl, srcUrl } from './_rune-harness.mjs';

const TAG = '[check-chat-notify-behavior]';
const CN = 'features/requests/chat/chatNotifications.svelte.ts';

const STUBS = {
  '@trackly/transport/ws': `export function onWsEvent(h) { const s = globalThis.__h.ws; s.add(h); return () => s.delete(h); }
  export function onWsResync(h) { const s = globalThis.__h.resync; s.add(h); return () => s.delete(h); }`,
  '$lib/stores/auth.svelte': `export const authStore = globalThis.__h.auth;`,
  '$lib/platform/systemNotify': `
    export function appHasFocus() { return globalThis.__h.focus; }
    export function armPermissionPrompt() { return () => {}; }
    export async function showSystemNotification(title, body, target) { globalThis.__h.sys.push({ title, body, target }); }`,
  './api': `export const chat = {
    unreadSummary: (...a) => globalThis.__h.api.unreadSummary(...a),
    setMuted: (...a) => globalThis.__h.api.setMuted(...a),
  };`,
  'svelte-spa-router': `export function push(h) { globalThis.__h.pushed.push(h); }`,
};

async function runCases(overrides) {
  const H = {
    ws: new Set(),
    resync: new Set(),
    auth: { user: { id: 1, login: 'ivanov', fullName: 'Иванов И.И.', role: 'manager' } },
    focus: true,
    sys: [],
    pushed: [],
    summary: { items: [], mutedRequestIds: [] },
    api: {
      unreadSummary: async () => H.summary,
      setMuted: async () => {},
    },
  };
  globalThis.__h = H;
  installHarness({ stubs: STUBS, overrides });
  const toast = await import(pkgUrl('kit', 'stores/toast.svelte.ts'));
  const scoped = await import(pkgUrl('kit', 'stores/userScoped.svelte.ts'));
  const unread = await import(srcUrl('features/requests/chat/unreadStore.svelte.ts'));
  const cn = await import(srcUrl(CN));

  const bad = [];
  const expect = (code, cond, msg) => {
    if (!cond) bad.push(`[${code}] ${msg}`);
  };
  const emit = (ev) => {
    for (const h of [...H.ws]) h(ev);
  };
  const msg = (requestId, actorUserId = 2, actorName = 'Петров П.П.') => ({
    type: 'chat_message',
    requestId,
    messageId: 100 + requestId,
    actorUserId,
    actorName,
  });
  const chatToasts = (id) =>
    toast.toastStore.items.filter((t) => t.link && t.link.href.includes(`id=${id}&`));
  const clear = () => {
    toast.clearToasts();
    H.sys.length = 0;
    H.pushed.length = 0;
  };

  const stop = cn.startChatRealtime();
  // Дать стартовому refreshUnread завершиться.
  await new Promise((r) => setTimeout(r, 0));
  expect(
    'N0',
    H.ws.size >= 2,
    `startChatRealtime подписал ${H.ws.size} обработчиков WS (ожидалось ≥ 2)`,
  );

  // N1
  clear();
  emit(msg(5));
  let t = chatToasts(5);
  expect('N1', t.length === 1, `тостов по заявке 5: ${t.length}`);
  expect('N1', t[0]?.kind === 'info', `вид тоста: ${t[0]?.kind}`);
  expect('N1', t[0]?.link?.label === 'Открыть чат', `подпись ссылки: ${t[0]?.link?.label}`);
  expect(
    'N1',
    /^#\/requests\?id=5&tab=chat&/.test(t[0]?.link?.href ?? ''),
    `ссылка: ${t[0]?.link?.href}`,
  );
  expect('N1', H.sys.length === 0, `в фокусе ушло системное уведомление: ${H.sys.length}`);

  // N2
  clear();
  H.focus = false;
  emit(msg(6));
  expect('N2', chatToasts(6).length === 1, 'без фокуса нет тоста (D-G1: оба канала)');
  expect('N2', H.sys.length === 1, `системных уведомлений: ${H.sys.length}`);
  const target = H.sys[0]?.target;
  expect('N2', /tab=chat/.test(target?.href ?? ''), `href уведомления: ${target?.href}`);
  if (target && typeof target.activate === 'function') {
    target.activate();
    expect(
      'N2',
      /^#\/requests\?id=6&tab=chat&/.test(H.pushed[0] ?? ''),
      `activate открыл: ${H.pushed[0]}`,
    );
  } else bad.push('[N2] у уведомления нет activate()');
  H.focus = true;

  // N3
  clear();
  H.focus = false;
  emit(msg(7, 1, 'Иванов И.И.'));
  expect('N3', chatToasts(7).length === 0 && H.sys.length === 0, 'своё сообщение дало уведомление');
  H.focus = true;

  // N4
  clear();
  unread.setActiveChat(8);
  H.focus = false;
  emit(msg(8));
  expect('N4', chatToasts(8).length === 0 && H.sys.length === 0, 'открытый чат дал уведомление');
  unread.setActiveChat(null);
  emit(msg(8));
  expect('N4', chatToasts(8).length === 1, 'после закрытия чата уведомления по заявке нет');
  H.focus = true;

  // N5
  clear();
  H.summary = { items: [], mutedRequestIds: [9] };
  await unread.refreshUnread();
  H.focus = false;
  emit(msg(9));
  expect(
    'N5',
    chatToasts(9).length === 0 && H.sys.length === 0,
    'заглушённая заявка дала уведомление',
  );
  emit(msg(10));
  expect(
    'N5',
    chatToasts(10).length === 1 && H.sys.length === 1,
    'соседняя незаглушённая заявка молчит',
  );
  H.summary = { items: [], mutedRequestIds: [] };
  await unread.refreshUnread();
  H.focus = true;

  // N6
  clear();
  emit(msg(11));
  emit(msg(11));
  expect('N6', chatToasts(11).length === 1, `дубль тоста: ${chatToasts(11).length}`);
  toast.removeToast(chatToasts(11)[0]?.id);
  emit(msg(11));
  expect('N6', chatToasts(11).length === 1, 'после снятия тоста новое сообщение не дало тост');

  // N7
  clear();
  emit(msg(12));
  emit(msg(13));
  cn.dismissChatToast(12);
  expect('N7', chatToasts(12).length === 0, 'dismissChatToast не снял тост своей заявки');
  expect('N7', chatToasts(13).length === 1, 'dismissChatToast снял чужой тост');
  try {
    cn.dismissChatToast(999);
  } catch (e) {
    bad.push(`[N7] dismissChatToast(неизвестный) бросил: ${e}`);
  }
  emit(msg(12));
  expect('N7', chatToasts(12).length === 1, 'после dismiss новое сообщение не дало тост');

  // N8
  clear();
  emit(msg(14, 3, null));
  const m8 = chatToasts(14)[0]?.message ?? '';
  expect('N8', m8.includes('№14') && m8.includes('Участник'), `текст тоста без автора: ${m8}`);
  expect('N8', !/undefined|null/.test(m8), `в тексте пустое значение: ${m8}`);

  // N9
  clear();
  emit({ type: 'request_status_changed', requestId: 15, status: 'in_progress' });
  emit({ type: 'chat_read_changed', requestId: 15, userId: 1 });
  expect(
    'N9',
    toast.toastStore.items.length === 0 && H.sys.length === 0,
    'не-chat событие дало уведомление',
  );
  emit(msg(16));
  scoped.resetUserScoped();
  H.auth.user = { id: 2, login: 'petrov', fullName: 'Петров П.П.', role: 'manager' };
  // Тост прежнего пользователя ещё жив: дедупликация не должна глотать тост нового.
  emit(msg(16, 3, 'Сидорова А.А.'));
  expect(
    'N9',
    chatToasts(16).some((x) => x.message.includes('Сидорова А.А.')),
    'после смены пользователя тост новому не показан (карта тостов не сброшена)',
  );
  H.auth.user = null;
  toast.clearToasts();
  emit(msg(17));
  expect('N9', toast.toastStore.items.length === 0, 'без пользователя показан тост');

  // N10: ресинк тихий — сводка с «новыми» данными не порождает тостов и ОС-уведомлений
  H.auth.user = { id: 1, login: 'ivanov', fullName: 'Иванов И.И.', role: 'manager' };
  clear();
  H.sys.length = 0;
  H.summary = { items: [{ requestId: 21, count: 4 }], mutedRequestIds: [] };
  expect('N10', H.resync.size >= 1, 'ресинк-обработчик не зарегистрирован');
  for (const h of [...H.resync]) h();
  await new Promise((r) => setTimeout(r, 50));
  expect(
    'N10',
    toast.toastStore.items.length === 0 && H.sys.length === 0,
    `ресинк породил уведомления: тостов ${toast.toastStore.items.length}, ОС ${H.sys.length}`,
  );

  stop();
  expect('N10', H.resync.size === 0, `после stop остались обработчики ресинка: ${H.resync.size}`);
  expect('N0', H.ws.size === 0, `после stop остались подписки WS: ${H.ws.size}`);
  return bad;
}

const MUTANTS = [
  {
    name: 'перестановка: проверка mute перенесена ниже pushToast',
    file: CN,
    edits: [
      ['  if (isMuted(ev.requestId)) return;\n', ''],
      ['  // D-G1: оба канала.', '  if (isMuted(ev.requestId)) return;\n  // D-G1: оба канала.'],
    ],
    expect: 'N5',
  },
  {
    name: 'инверсия: системное уведомление при фокусе',
    file: CN,
    from: 'if (!appHasFocus()) {',
    to: 'if (appHasFocus()) {',
    expect: 'N1',
  },
  {
    name: 'удалена проверка своего сообщения',
    file: CN,
    from: '  if (ev.actorUserId !== null && ev.actorUserId === me.id) return;\n',
    to: '',
    expect: 'N3',
  },
  {
    name: 'инверсия: тишина в ЧУЖОМ открытом чате',
    file: CN,
    from: 'if (unreadStore.activeChatRequestId === ev.requestId) return;',
    to: 'if (unreadStore.activeChatRequestId !== ev.requestId) return;',
    expect: 'N1',
  },
  {
    name: 'перестановка: dismissChatToast удаляет из карты раньше чтения',
    file: CN,
    from: '  const id = toastByRequest.get(requestId);\n  if (id === undefined) return;',
    to: '  toastByRequest.delete(requestId);\n  const id = toastByRequest.get(requestId);\n  if (id === undefined) return;',
    expect: 'N7',
  },
  {
    name: 'дедупликация снята: тост на каждое сообщение',
    file: CN,
    from: 'if (!prev || !toastStore.items.some((t) => t.id === prev)) {',
    to: 'if (true) {',
    expect: 'N6',
  },
  {
    name: 'activate ведёт на вкладку «Информация»',
    file: CN,
    from: "activate: () => openRequestTarget(ev.requestId, 'chat'),",
    to: "activate: () => openRequestTarget(ev.requestId, 'info'),",
    expect: 'N2',
  },
  {
    name: 'ресинк показывает тост',
    file: 'features/requests/chat/unreadStore.svelte.ts',
    from: 'const offResync = onWsResync(() => {\n    void refreshUnread();',
    to: "const offResync = onWsResync(() => {\n    pushToast('info', 'x');\n    void refreshUnread();",
    expect: 'N10',
  },
  {
    name: 'смена пользователя не чистит карту тостов',
    file: CN,
    from: '  toastByRequest.clear();\n',
    to: '',
    expect: 'N9',
  },
];

await gateMain({ tag: TAG, scriptUrl: import.meta.url, mutants: MUTANTS, runCases });
