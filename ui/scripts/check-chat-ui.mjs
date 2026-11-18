#!/usr/bin/env node
// [check-chat-ui] Структурный гейт UI чата заявки (фаза 41.5, план 15).
//
// Компиляционные гейты не видят рантайм-дефектов, а счётный гейт слеп к порядку
// (охрана после preventDefault проходит по счёту). Поэтому порядок проверяется
// ПОЗИЦИОННО, а самотест включает мутации-перестановки, не только удаление.
//
// Правила (сообщение начинается с кода):
//   [C0] обязательные файлы присутствуют (отсутствие = красный, не «пропуск»);
//   [C1] в chat/*.svelte нет `{@html}` и `<select`; ссылки с rel/target; текст через linkify;
//   [C2] ChatComposer: isComposing раньше вызова отправки в обработчике клавиш;
//   [C3] ChatComposer.send: uploadDraft раньше chat.send, есть discardDrafts;
//   [C4] RequestChat: присваивание firstUnreadId раньше markRead; setActiveChat ставится/снимается;
//        ровно один <AttachmentViewer, не внутри <ol;
//   [C5] chatNotifications: сравнение actorUserId раньше pushToast/new Notification;
//        текст сообщения в уведомление не передаётся;
//   [C6] unreadStore: registerUserScopedReset, Array.isArray, пять WS-событий; счётчик requestsUnread в сайдбаре;
//   [C7] RequestDetail: <RequestChat ровно один раз внутри ветки вкладки «Чат»; showTabs исключает ad_register;
//   [C8] счётный минимум: сработало не меньше 8 различных правил.
//
// Usage: node scripts/check-chat-ui.mjs [--selftest]

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { stripComments } from './_strip.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const TAG = '[check-chat-ui]';
const SRC = path.resolve(__dirname, '../src');
const CHAT = 'features/requests/chat/';
const REQUIRED = [
  `${CHAT}RequestChat.svelte`,
  `${CHAT}ChatComposer.svelte`,
  `${CHAT}ChatMessage.svelte`,
  `${CHAT}ChatEvent.svelte`,
  `${CHAT}ChatAttachmentEvent.svelte`,
  `${CHAT}chatNotifications.svelte.ts`,
  `${CHAT}unreadStore.svelte.ts`,
  `${CHAT}draftStore.svelte.ts`,
  `${CHAT}tabStore.svelte.ts`,
  `${CHAT}linkify.ts`,
  'features/requests/RequestDetail.svelte',
  'features/layout/sidebar-config.ts',
];
const MIN_RULES = 8;
const WS_EVENTS = [
  'chat_message',
  'request_status_changed',
  'attachments_changed',
  'new_request',
  'chat_read_changed',
];

function readFiles() {
  const files = {};
  for (const rel of REQUIRED) {
    const p = path.join(SRC, rel);
    if (fs.existsSync(p)) files[rel] = fs.readFileSync(p, 'utf8');
  }
  // все svelte в chat/ — для C1
  const dir = path.join(SRC, CHAT);
  if (fs.existsSync(dir)) {
    for (const f of fs.readdirSync(dir)) {
      if (f.endsWith('.svelte')) files[CHAT + f] = fs.readFileSync(path.join(dir, f), 'utf8');
    }
  }
  return files;
}

/** Тело функции: от заголовка (RegExp) до парной закрывающей скобки. */
function funcBody(src, headerRe) {
  const m = headerRe.exec(src);
  if (!m) return null;
  const open = src.indexOf('{', m.index + m[0].length - 1);
  if (open < 0) return null;
  let depth = 0;
  for (let i = open; i < src.length; i++) {
    if (src[i] === '{') depth++;
    else if (src[i] === '}' && --depth === 0) return src.slice(open, i + 1);
  }
  return null;
}

const count = (s, sub) => s.split(sub).length - 1;

export function check(files) {
  const v = [];
  const fired = new Set();
  const fail = (code, msg) => v.push({ code, msg });
  const get = (rel) => (files[rel] === undefined ? null : stripComments(files[rel]));

  // C0
  for (const rel of REQUIRED) {
    if (files[rel] === undefined) fail('C0', `обязательный файл отсутствует: ${rel}`);
  }
  fired.add('C0');

  // C1
  for (const [rel, raw] of Object.entries(files)) {
    if (!rel.startsWith(CHAT) || !rel.endsWith('.svelte')) continue;
    const s = stripComments(raw);
    if (s.includes('{@html')) fail('C1', `${rel}: {@html} в каталоге чата`);
    if (/<select[\s>]/.test(s)) fail('C1', `${rel}: нативный <select>`);
  }
  for (const rel of [`${CHAT}ChatMessage.svelte`, `${CHAT}ChatEvent.svelte`]) {
    const s = get(rel);
    if (s === null) continue;
    if (!s.includes('rel="noopener noreferrer"'))
      fail('C1', `${rel}: нет rel="noopener noreferrer"`);
    if (!s.includes('target="_blank"')) fail('C1', `${rel}: нет target="_blank"`);
    if (!/linkify\(/.test(s)) fail('C1', `${rel}: текст не проходит через linkify`);
  }
  fired.add('C1');

  // C2, C3
  const comp = get(`${CHAT}ChatComposer.svelte`);
  if (comp !== null) {
    const kd = funcBody(comp, /function handleKeydown\([^)]*\)[^{]*\{/);
    if (kd === null) fail('C2', 'ChatComposer: handleKeydown не найден');
    else {
      const ic = kd.indexOf('isComposing');
      const sd = kd.search(/\bsend\(/);
      if (ic < 0) fail('C2', 'ChatComposer: нет охраны isComposing');
      else if (sd < 0) fail('C2', 'ChatComposer: handleKeydown не вызывает send()');
      else if (ic > sd) fail('C2', 'ChatComposer: isComposing стоит ПОСЛЕ вызова отправки');
    }
    fired.add('C2');
    const sb = funcBody(comp, /async function send\(\)\s*\{/);
    if (sb === null) fail('C3', 'ChatComposer: send() не найден');
    else {
      const up = sb.indexOf('uploadDraft(');
      const cs = sb.indexOf('chat.send(');
      if (up < 0 || cs < 0) fail('C3', 'ChatComposer.send: нет uploadDraft( или chat.send(');
      else if (up > cs) fail('C3', 'ChatComposer.send: uploadDraft стоит ПОСЛЕ chat.send');
      if (!sb.includes('discardDrafts(')) fail('C3', 'ChatComposer.send: нет discardDrafts(');
    }
    fired.add('C3');
  }

  // C4
  const rc = get(`${CHAT}RequestChat.svelte`);
  if (rc !== null) {
    const asg = rc.search(/\bfirstUnreadId\s*=\s*res\./);
    const mr = rc.search(/\bvoid markRead\(|\bawait markRead\(|[^.\w]markRead\(\);/);
    if (asg < 0) fail('C4', 'RequestChat: нет присваивания firstUnreadId из ответа');
    else if (mr < 0) fail('C4', 'RequestChat: нет вызова markRead()');
    else if (asg > mr) fail('C4', 'RequestChat: firstUnreadId присваивается ПОСЛЕ markRead');
    if (!/setActiveChat\((?!null)[^)]+\)/.test(rc))
      fail('C4', 'RequestChat: setActiveChat(id) не ставится');
    if (!rc.includes('setActiveChat(null)'))
      fail('C4', 'RequestChat: setActiveChat(null) не снимается');
    const nv = count(rc, '<AttachmentViewer');
    if (nv !== 1) fail('C4', `RequestChat: <AttachmentViewer встречается ${nv} раз, нужен 1`);
    else {
      const at = rc.indexOf('<AttachmentViewer');
      const opens = count(rc.slice(0, at), '<ol');
      const closes = count(rc.slice(0, at), '</ol>');
      if (opens !== closes) fail('C4', 'RequestChat: <AttachmentViewer внутри <ol');
    }
    fired.add('C4');
  }

  // C5
  const cn = get(`${CHAT}chatNotifications.svelte.ts`);
  if (cn !== null) {
    const a = cn.indexOf('actorUserId');
    const pt = cn.indexOf('pushToast(', cn.indexOf('export function handleChatMessageEvent'));
    const nn = cn.indexOf('new Notification(');
    if (a < 0) fail('C5', 'chatNotifications: нет сравнения actorUserId');
    else {
      if (pt >= 0 && a > pt) fail('C5', 'chatNotifications: actorUserId ПОСЛЕ pushToast(');
      if (nn >= 0 && a > nn) fail('C5', 'chatNotifications: actorUserId ПОСЛЕ new Notification(');
      if (pt < 0 && nn < 0) fail('C5', 'chatNotifications: нет pushToast( / new Notification(');
    }
    if (/\bev\.(text|body|message|content)\b/.test(cn))
      fail('C5', 'chatNotifications: текст сообщения попадает в уведомление');
    fired.add('C5');
  }

  // C6
  const us = get(`${CHAT}unreadStore.svelte.ts`);
  if (us !== null) {
    if (!us.includes('registerUserScopedReset('))
      fail('C6', 'unreadStore: нет registerUserScopedReset(');
    if (!us.includes('Array.isArray'))
      fail('C6', 'unreadStore: нет проверки формы ответа (Array.isArray)');
    for (const e of WS_EVENTS)
      if (!us.includes(`'${e}'`)) fail('C6', `unreadStore: нет подписки на WS-событие ${e}`);
  }
  const sc = get('features/layout/sidebar-config.ts');
  if (sc !== null && !/counter:\s*'requestsUnread'/.test(sc))
    fail('C6', "sidebar-config: нет counter: 'requestsUnread'");
  fired.add('C6');

  // C7
  const rd = get('features/requests/RequestDetail.svelte');
  const tb = get(`${CHAT}tabStore.svelte.ts`);
  if (rd !== null) {
    const n = count(rd, '<RequestChat');
    if (n !== 1) fail('C7', `RequestDetail: <RequestChat встречается ${n} раз, нужен 1`);
    else {
      const at = rd.indexOf('<RequestChat');
      const ifAt = rd.lastIndexOf('{#if', at);
      const cond = rd.slice(ifAt, rd.indexOf('}', ifAt));
      if (!/activeTab === 'chat'/.test(cond))
        fail('C7', 'RequestDetail: <RequestChat вне ветки вкладки «Чат»');
    }
    if (
      !/showTabs\s*=\s*\$derived\([^;]*ad_register|isAdRegister/.test(rd) ||
      !/showTabs\s*=\s*\$derived\([^;]*!isAdRegister/.test(rd)
    )
      fail('C7', 'RequestDetail: showTabs не исключает ad_register');
  }
  if (tb !== null && !tb.includes('trackly:request-detail-tab'))
    fail('C7', 'tabStore: нет ключа trackly:request-detail-tab');
  fired.add('C7');

  // C8
  if (fired.size < MIN_RULES) fail('C8', `сработало ${fired.size} правил из ${MIN_RULES}`);
  return v;
}

function selftest(files) {
  const real = check(files);
  if (real.length > 0) {
    for (const x of real) console.error(`${TAG} FAIL — [${x.code}] ${x.msg}`);
    console.error(`${TAG} SELFTEST FAIL — реальные файлы не зелёные`);
    process.exit(1);
  }
  const uniq = (src, anchor) => {
    const n = count(src, anchor);
    if (n !== 1) {
      console.error(`${TAG} SELFTEST FAIL — якорь мутации не уникален (${n}): ${anchor}`);
      process.exit(1);
    }
  };
  // Перестановка: вырезать якорь и вставить сразу после `after` (после первого вхождения).
  const reorder = (rel, anchor, after) => () => {
    const src = files[rel];
    uniq(src, anchor);
    uniq(src, after);
    const cut = src.replace(anchor, '');
    const i = cut.indexOf(after) + after.length;
    return { ...files, [rel]: cut.slice(0, i) + '\n' + anchor + cut.slice(i) };
  };
  const swapText = (rel, from, to) => () => {
    uniq(files[rel], from);
    return { ...files, [rel]: files[rel].replace(from, to) };
  };
  const mutants = [
    {
      name: 'перестановка: isComposing после send()',
      expect: 'C2',
      make: reorder(
        `${CHAT}ChatComposer.svelte`,
        "if (e.key !== 'Enter' || e.isComposing) return;",
        'void send();',
      ),
    },
    {
      name: 'перестановка: uploadDraft после chat.send',
      expect: 'C3',
      make: reorder(
        `${CHAT}ChatComposer.svelte`,
        'const res = await uploadDraft(sentFor, chip.file);',
        'outcome = await chat.send(sentFor, draft.text, draftIds);',
      ),
    },
    {
      name: 'перестановка: firstUnreadId после markRead',
      expect: 'C4',
      make: reorder(
        `${CHAT}RequestChat.svelte`,
        'if (initial) firstUnreadId = res.firstUnreadId ?? null;',
        'void markRead();',
      ),
    },
    {
      name: 'перестановка: actorUserId после pushToast',
      expect: 'C5',
      make: reorder(
        `${CHAT}chatNotifications.svelte.ts`,
        'if (ev.actorUserId !== null && ev.actorUserId === me.id) return;',
        'toastByRequest.set(ev.requestId, id);',
      ),
    },
    {
      name: 'внесён {@html}',
      expect: 'C1',
      make: () => ({
        ...files,
        [`${CHAT}ChatMessage.svelte`]: files[`${CHAT}ChatMessage.svelte`] + '\n{@html item.text}\n',
      }),
    },
    {
      name: 'снят rel="noopener noreferrer" в ChatMessage',
      expect: 'C1',
      make: swapText(`${CHAT}ChatMessage.svelte`, 'rel="noopener noreferrer"', ''),
    },
    {
      name: 'нет сброса при смене пользователя',
      expect: 'C6',
      make: swapText(
        `${CHAT}unreadStore.svelte.ts`,
        'registerUserScopedReset(() => {',
        'noop(() => {',
      ),
    },
    {
      name: 'текст сообщения в уведомление',
      expect: 'C5',
      make: swapText(`${CHAT}chatNotifications.svelte.ts`, '{ body: text }', '{ body: ev.text }'),
    },
    {
      name: 'пропал файл (не «пропуск», а красный)',
      expect: 'C0',
      make: () => {
        const copy = { ...files };
        delete copy[`${CHAT}draftStore.svelte.ts`];
        return copy;
      },
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

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) main();
