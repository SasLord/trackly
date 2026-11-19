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
//   [C5] chatNotifications: порядок actorUserId → isMuted → pushToast → showSystemNotification, фокус через appHasFocus,
//        без document.hidden / ветки роли / прямого new Notification;
//        текст сообщения в уведомление не передаётся;
//   [C6] unreadStore: registerUserScopedReset, Array.isArray, пять WS-событий; счётчик requestsUnread в сайдбаре;
//   [C7] RequestDetail: <RequestChat ровно один раз внутри ветки вкладки «Чат»; showTabs исключает ad_register;
//   [C9] ChatMessage: pre-wrap только на .text (не на .bubble); шапка .head — внутри .bubble и до
//        sr-only-префикса (якорь верхней границы пузыря); нет <p class="author">, author-role, ROLES;
//        class="time" ровно два: в .head (под {#if nameShown}) и в .foot (под {#if !nameShown}, после .text);
//        nameShown = groupStart && !item.isOwn; .msg:not(.group-start) с margin-top calc(2xs - …);
//   [C10] переключатель оповещений: <ChatMuteToggle ровно один раз, внутри сниппета tabsSubheader, который
//        подключается только под showTabs; в ChatMuteToggle: aria-pressed, обе подписи, toggleMute(, без $effect;
//   [C12] DetailPanel/RequestChat/ChatComposer (UAT 41.5 R2-4, R3-3): в режиме subheader две линии: под .header-row
//        и под вкладками (border-bottom на .detail-header.with-subheader; не в .subheader), базовая линия цела;
//        на вкладке «Чат» лента вплотную (.fill .fill-body с отрицательным полем, margin-bottom: 0 у шапки);
//        боковой отступ xl у .chat-feed и .composer;
//   [C11] путь клика по уведомлению (UAT 41.5 R2-1): обёртка systemNotify (onclick → focus/close/runOrOffer,
//        модалка раньше activate, notify_desktop раньше sendNotification, запасной механизм только не-Windows;
//        R3-1: AUMID по виду запуска (uninstall.exe, PORTABLE_AUMID), проба Setting до Toast::new, Disabled → suppressed
//        без плагина, тост как у плагина, логи без текста/путей
//        и только предлагает), цель в chatNotifications/EmployeeLayout (тост ВСЕГДА + уведомление, без else),
//        tab=info|chat в RequestsPage, уникальный n в ссылке, Rust-команда без AUMID/реестра/.lnk, паритет имени события;
//   [C15] ChatComposer (R3-4): нет подсказки клавиш; строка .foot только внутри {#if showCounter};
//        вертикальный padding .composer симметричен (1–2 значения);
//   [C13] RequestChat (R3-2): onloadcapture/onerrorcapture на ленте → tick() → проверка followBottom → scrollToBottom;
//        followBottom не $state и не в $effect; onFeedScroll сравнивает с lastScrollTop РАНЬШЕ присваивания;
//   [C14] (R3-5) dismissChatToast: get раньше delete, removeToast; вызов внутри untrack эффекта смены заявки;
//   [C8] счётный минимум: сработало не меньше MIN_RULES различных правил.
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
  `${CHAT}ChatMuteToggle.svelte`,
  `${CHAT}ChatEvent.svelte`,
  `${CHAT}ChatAttachmentEvent.svelte`,
  `${CHAT}chatNotifications.svelte.ts`,
  `${CHAT}unreadStore.svelte.ts`,
  `${CHAT}draftStore.svelte.ts`,
  `${CHAT}tabStore.svelte.ts`,
  `${CHAT}linkify.ts`,
  'features/requests/RequestDetail.svelte',
  'features/layout/sidebar-config.ts',
  'lib/components/DetailPanel.svelte',
  'lib/platform/systemNotify.ts',
  'features/requests/requestTarget.ts',
  'features/layout/EmployeeLayout.svelte',
  'features/requests/RequestsPage.svelte',
];
const RUST_REL = '@rust/system_notify.rs';
const RUST_ABS = path.resolve(
  __dirname,
  '../../crates/trackly-app/src/tauri_cmds/system_notify.rs',
);
const MIN_RULES = 15;
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
  if (fs.existsSync(RUST_ABS)) files[RUST_REL] = fs.readFileSync(RUST_ABS, 'utf8');
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

/** Содержимое круглых скобок с балансом; `open` — индекс '('. */
function parenBody(src, open) {
  let depth = 0;
  for (let i = open; i < src.length; i++) {
    if (src[i] === '(') depth++;
    else if (src[i] === ')' && --depth === 0) return src.slice(open, i + 1);
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
  for (const rel of [...REQUIRED, RUST_REL]) {
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
    const body = funcBody(cn, /export function handleChatMessageEvent\([^)]*\)[^{]*\{/);
    if (body === null) fail('C5', 'chatNotifications: handleChatMessageEvent не найден');
    else {
      const a = body.indexOf('actorUserId');
      const mu = body.indexOf('isMuted(');
      const pt = body.indexOf('pushToast(');
      const sn = body.indexOf('showSystemNotification(');
      if (a < 0) fail('C5', 'chatNotifications: нет сравнения actorUserId');
      if (mu < 0) fail('C5', 'chatNotifications: нет проверки isMuted(');
      if (pt < 0) fail('C5', 'chatNotifications: нет pushToast(');
      if (sn < 0) fail('C5', 'chatNotifications: нет showSystemNotification(');
      if (a >= 0 && mu >= 0 && a > mu) fail('C5', 'chatNotifications: actorUserId ПОСЛЕ isMuted(');
      if (a >= 0 && pt >= 0 && a > pt)
        fail('C5', 'chatNotifications: actorUserId ПОСЛЕ pushToast(');
      if (a >= 0 && sn >= 0 && a > sn)
        fail('C5', 'chatNotifications: actorUserId ПОСЛЕ showSystemNotification(');
      if (mu >= 0 && pt >= 0 && mu > pt) fail('C5', 'chatNotifications: isMuted( ПОСЛЕ pushToast(');
      if (mu >= 0 && sn >= 0 && mu > sn)
        fail('C5', 'chatNotifications: isMuted( ПОСЛЕ showSystemNotification(');
      if (!body.includes('appHasFocus('))
        fail('C5', 'chatNotifications: фокус не проверяется через appHasFocus(');
      if (/\.role\b/.test(body))
        fail('C5', 'chatNotifications: ветка по роли (все роли уведомляются)');
    }
    if (/\bdocument\.hidden\b/.test(cn))
      fail('C5', 'chatNotifications: document.hidden вместо фокуса окна');
    if (cn.includes('new Notification('))
      fail('C5', 'chatNotifications: прямой new Notification( (только через systemNotify)');
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
    if (!/Array\.isArray\(res\.mutedRequestIds\)/.test(us))
      fail('C6', 'unreadStore: не читается mutedRequestIds (режим «отключено»)');
    if (!us.includes('export function isMuted(')) fail('C6', 'unreadStore: нет экспорта isMuted(');
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

  // C10
  const mt = get(`${CHAT}ChatMuteToggle.svelte`);
  if (rd !== null) {
    const n = count(rd, '<ChatMuteToggle');
    if (n !== 1) fail('C10', `RequestDetail: <ChatMuteToggle встречается ${n} раз, нужен 1`);
    else {
      const sStart = rd.indexOf('{#snippet tabsSubheader()}');
      const sEnd = rd.indexOf('{/snippet}', sStart);
      const at = rd.indexOf('<ChatMuteToggle');
      if (sStart < 0 || sEnd < 0 || !(sStart < at && at < sEnd))
        fail('C10', 'RequestDetail: <ChatMuteToggle вне сниппета tabsSubheader (полосы вкладок)');
    }
    if (!rd.includes('subheader={showTabs && request ? tabsSubheader : undefined}'))
      fail('C10', 'RequestDetail: tabsSubheader подключён не только под showTabs');
  }
  if (mt !== null) {
    const code = stripComments(mt);
    if (!code.includes('aria-pressed={')) fail('C10', 'ChatMuteToggle: нет aria-pressed');
    if (
      !code.includes('Отключить оповещения по этой заявке') ||
      !code.includes('Включить оповещения по этой заявке')
    )
      fail('C10', 'ChatMuteToggle: нет обеих aria-подписей');
    if (!/onclick=\{[^}]*toggleMute\(/.test(code))
      fail('C10', 'ChatMuteToggle: onclick не вызывает toggleMute(');
    if (code.includes('$effect')) fail('C10', 'ChatMuteToggle: $effect запрещён');
  }
  fired.add('C10');

  // C9
  const cm = get(`${CHAT}ChatMessage.svelte`);
  if (cm !== null) {
    const bubbleCss = funcBody(cm, /\.bubble\s*\{/);
    const textCss = funcBody(cm, /\.text\s*\{/);
    if (bubbleCss === null) fail('C9', 'ChatMessage: CSS-блок .bubble не найден');
    else if (/white-space:\s*pre-wrap/.test(bubbleCss))
      fail('C9', 'ChatMessage: white-space: pre-wrap на .bubble (красная строка)');
    if (textCss === null) fail('C9', 'ChatMessage: CSS-блок .text не найден');
    else if (!/white-space:\s*pre-wrap/.test(textCss))
      fail('C9', 'ChatMessage: у .text нет white-space: pre-wrap (переносы строк теряются)');
    const bAt = cm.indexOf('<div class="bubble"');
    const hAt = cm.indexOf('class="head"');
    // верхняя граница пузыря для шапки — sr-only-префикс, всегда идущий сразу после шапки
    const sAt = cm.indexOf('<span class="sr-only">');
    const tAt = cm.indexOf('class="text"');
    const fAt = cm.indexOf('class="foot"');
    const endAt = cm.indexOf('</li>');
    if (bAt < 0 || hAt < 0 || sAt < 0) fail('C9', 'ChatMessage: нет .bubble / .head / sr-only');
    else {
      if (!(bAt < hAt && hAt < sAt)) fail('C9', 'ChatMessage: шапка .head не внутри .bubble');
      const timeInHead = count(cm.slice(hAt, sAt), 'class="time"');
      if (timeInHead !== 1 || count(cm, 'class="time"') !== 2)
        fail('C9', 'ChatMessage: class="time" должен быть ровно два: в .head и в .foot');
      if (!cm.slice(cm.lastIndexOf('{#if', hAt)).startsWith('{#if nameShown}'))
        fail('C9', 'ChatMessage: шапка .head не под {#if nameShown}');
    }
    if (fAt < 0 || tAt < 0 || endAt < 0) fail('C9', 'ChatMessage: нет .foot / .text');
    else {
      if (!(tAt < fAt && fAt < endAt))
        fail('C9', 'ChatMessage: подвал .foot должен стоять ПОСЛЕ .text, внутри пузыря');
      if (count(cm.slice(fAt, endAt), 'class="time"') !== 1)
        fail('C9', 'ChatMessage: время должно быть внутри .foot');
      if (!cm.slice(cm.lastIndexOf('{#if', fAt)).startsWith('{#if !nameShown}'))
        fail('C9', 'ChatMessage: подвал .foot не под {#if !nameShown}');
    }
    const ns = /\bnameShown\s*=\s*\$derived\(([^)]*)\)/.exec(cm);
    if (ns === null || !ns[1].includes('groupStart') || !ns[1].includes('!item.isOwn'))
      fail('C9', 'ChatMessage: nameShown не равен groupStart && !item.isOwn');
    const contCss = funcBody(cm, /\.msg:not\(\.group-start\)\s*\{/);
    if (contCss === null || !/margin-top:\s*calc\(var\(--tr-space-2xs\)\s*-\s/.test(contCss))
      fail(
        'C9',
        'ChatMessage: нет .msg:not(.group-start) с margin-top: calc(2xs - …) (плотная группа)',
      );
    if (/<p class="author"/.test(cm) || cm.includes('author-role') || /\bROLES\b/.test(cm))
      fail('C9', 'ChatMessage: автор вне облака или показана роль');
  }
  fired.add('C9');

  // C12
  const dp = get('lib/components/DetailPanel.svelte');
  if (dp !== null) {
    const blk = (re) => funcBody(dp, re);
    const base = blk(/(?:^|\n)[ \t]*\.detail-header\s*\{/);
    if (base === null || !/border-bottom:\s*1px solid var\(--tr-border\)/.test(base))
      fail('C12', 'DetailPanel: у базового .detail-header нет линии (другие страницы задеты)');
    const wsh = blk(/(?:^|\n)[ \t]*\.detail-header\.with-subheader\s*\{/);
    if (wsh === null) fail('C12', 'DetailPanel: нет блока .detail-header.with-subheader');
    else {
      if (!/border-bottom:\s*1px solid var\(--tr-border\)/.test(wsh))
        fail('C12', 'DetailPanel: нет линии под вкладками (.detail-header.with-subheader, R3-3)');
      if (/border-bottom:\s*none/.test(wsh))
        fail(
          'C12',
          'DetailPanel: border-bottom: none у шапки с вкладками (линия под вкладками снята)',
        );
    }
    const hr = blk(/(?:^|\n)[ \t]*\.detail-header\.with-subheader \.header-row\s*\{/);
    if (hr === null || !/border-bottom:\s*1px solid var\(--tr-border\)/.test(hr))
      fail(
        'C12',
        'DetailPanel: у .with-subheader .header-row нет линии (она должна быть под заголовком)',
      );
    const subRe = /\.subheader\b[^{};]*\{/g;
    let sm;
    while ((sm = subRe.exec(dp)) !== null) {
      const body = funcBody(dp.slice(sm.index), /^[^{]*\{/);
      if (body !== null && /border-(bottom|top)/.test(body))
        fail('C12', 'DetailPanel: граница в блоке .subheader (линия не под заголовком)');
    }
    const fb = blk(/\.detail-panel\.fill \.fill-body\s*\{/);
    if (fb === null || !/margin:[^;]*calc\(var\(--tr-space-xl\) \* -1\)/.test(fb))
      fail(
        'C12',
        'DetailPanel: .fill .fill-body без отрицательного бокового поля (лента не вплотную)',
      );
    const fh = blk(/\.detail-panel\.fill \.detail-header\.with-subheader\s*\{/);
    if (fh === null || !/margin-bottom:\s*0\s*;/.test(fh))
      fail('C12', 'DetailPanel: у шапки вкладки «Чат» margin-bottom не 0 (зазор до ленты)');
  }
  if (rc !== null) {
    const feed = funcBody(rc, /(?:^|\n)[ \t]*\.chat-feed\s*\{/);
    if (feed === null || !/padding:[^;]*var\(--tr-space-xl\)/.test(feed))
      fail('C12', 'RequestChat: у .chat-feed нет бокового отступа --tr-space-xl');
  }
  if (comp !== null) {
    const cb = funcBody(comp, /(?:^|\n)[ \t]*\.composer\s*\{/);
    if (cb === null || !/padding:[^;]*var\(--tr-space-xl\)/.test(cb))
      fail('C12', 'ChatComposer: у .composer нет бокового отступа --tr-space-xl');
  }
  fired.add('C12');

  // C15 (R3-4): композер без подсказки клавиш; строка под полем — только вместе со счётчиком
  if (comp !== null) {
    if (comp.includes('Enter — отправить') || /class="hint"/.test(comp) || /\.hint\b/.test(comp))
      fail('C15', 'ChatComposer: вернулась подсказка клавиш (R3-4)');
    const ifOpen = comp.indexOf('{#if showCounter}');
    const foot = comp.indexOf('<div class="foot">');
    const ifClose = ifOpen < 0 ? -1 : comp.indexOf('{/if}', ifOpen);
    if (ifOpen < 0 || foot < 0 || !(foot > ifOpen && foot < ifClose))
      fail(
        'C15',
        'ChatComposer: строка .foot не внутри {#if showCounter} (пустая строка под полем)',
      );
    const cb15 = funcBody(comp, /(?:^|\n)[ \t]*\.composer\s*\{/);
    if (
      cb15 === null ||
      !/padding:\s*var\(--tr-space-[a-z0-9]+\)(\s+var\(--tr-space-[a-z0-9]+\))?\s*;/.test(cb15)
    )
      fail('C15', 'ChatComposer: у .composer вертикальный padding сверху и снизу разный');
  }
  fired.add('C15');

  // C13 (R3-2): лента докручивается вниз после догрузки картинок, пока пользователь «следует за низом»
  if (rc !== null) {
    const olAt = rc.indexOf('class="chat-feed"');
    const olStart = olAt < 0 ? -1 : rc.lastIndexOf('<ol', olAt);
    const olTag = olStart < 0 ? '' : rc.slice(olStart, rc.indexOf('>', olAt));
    const onLoad = /onloadcapture=\{(\w+)\}/.exec(olTag);
    const onErr = /onerrorcapture=\{(\w+)\}/.exec(olTag);
    if (!onLoad || !onErr)
      fail('C13', 'RequestChat: у <ol class="chat-feed"> нет onloadcapture/onerrorcapture (R3-2)');
    else if (onLoad[1] !== onErr[1])
      fail('C13', 'RequestChat: onloadcapture и onerrorcapture ведут в разные обработчики');
    if (!/let followBottom = true;/.test(rc) || /followBottom\s*=\s*\$state/.test(rc))
      fail(
        'C13',
        'RequestChat: followBottom должен быть обычной переменной (let followBottom = true;)',
      );
    if (onLoad) {
      const h = funcBody(rc, new RegExp(`function ${onLoad[1]}\\(`));
      const t = h === null ? -1 : h.indexOf('tick()');
      const f = h === null ? -1 : h.indexOf('followBottom');
      const sc = h === null ? -1 : h.indexOf('scrollToBottom(');
      if (h === null || t < 0 || f < 0 || sc < 0 || !(t < f && f < sc))
        fail(
          'C13',
          'RequestChat: обработчик догрузки — порядок tick() → проверка followBottom → scrollToBottom( нарушен',
        );
    }
    const stb = funcBody(rc, /function scrollToBottom\(/);
    if (stb === null || !stb.includes('followBottom = true'))
      fail('C13', 'RequestChat: scrollToBottom не ставит followBottom = true');
    const ofs = funcBody(rc, /function onFeedScroll\(/);
    const cmp = ofs === null ? -1 : ofs.indexOf('< lastScrollTop');
    const asg = ofs === null ? -1 : ofs.indexOf('lastScrollTop = ');
    if (ofs === null || cmp < 0 || asg < 0 || asg < cmp || !ofs.includes('followBottom = false'))
      fail(
        'C13',
        'RequestChat: onFeedScroll — сравнение с lastScrollTop должно идти РАНЬШЕ его присваивания, и снимать followBottom',
      );
    const rnd = funcBody(rc, /function revealNewDivider\(/);
    if (rnd === null || !rnd.includes('followBottom = false'))
      fail('C13', 'RequestChat: revealNewDivider не снимает followBottom');
    const effRe = /\$effect(?:\.pre)?\(\(\) => \{/g;
    let em;
    while ((em = effRe.exec(rc)) !== null) {
      const eb = funcBody(rc.slice(em.index), /^[^{]*\{/);
      if (eb !== null && eb.includes('followBottom'))
        fail('C13', 'RequestChat: followBottom в $effect (флаг не реактивен, петля рун)');
    }
  }
  fired.add('C13');

  // C14 (R3-5): открытие чата заявки снимает её тост
  if (cn !== null && rc !== null) {
    const dj = funcBody(cn, /export function dismissChatToast\(/);
    if (dj === null) fail('C14', 'chatNotifications: нет export function dismissChatToast(');
    else {
      const g = dj.indexOf('toastByRequest.get(');
      const d = dj.indexOf('toastByRequest.delete(');
      if (!dj.includes('removeToast(')) fail('C14', 'dismissChatToast: нет removeToast(');
      if (g < 0 || d < 0 || g > d)
        fail('C14', 'dismissChatToast: toastByRequest.get( должен быть раньше .delete(');
    }
    if (!/import\s*\{[^}]*dismissChatToast[^}]*\}\s*from '\.\/chatNotifications\.svelte'/.test(rc))
      fail('C14', 'RequestChat: нет импорта dismissChatToast');
    const sAt = rc.indexOf('setActiveChat(id)');
    const eAt = sAt < 0 ? -1 : rc.lastIndexOf('$effect(', sAt);
    const eff = eAt < 0 ? null : funcBody(rc.slice(eAt), /^[^{]*\{/);
    const ut = eff === null ? null : funcBody(eff, /untrack\(\(\) => \{/);
    if (ut === null || !ut.includes('dismissChatToast(id)'))
      fail(
        'C14',
        'RequestChat: dismissChatToast(id) не вызывается внутри untrack эффекта смены заявки',
      );
  }
  fired.add('C14');

  // C11
  const sn = get('lib/platform/systemNotify.ts');
  if (sn !== null) {
    if (
      !/export async function showSystemNotification\(\s*title: string,\s*body: string,\s*target\?: NotifyTarget/.test(
        sn,
      )
    )
      fail('C11', 'systemNotify: showSystemNotification без третьего параметра target');
    const sb = funcBody(sn, /export async function showSystemNotification\([^)]*\)[^{]*\{/);
    if (sb === null) fail('C11', 'systemNotify: showSystemNotification не найден');
    else {
      if (!/const n = new Notification\(/.test(sb))
        fail('C11', 'systemNotify: результат new Notification( не сохранён в переменную');
      const oc = funcBody(sb, /n\.onclick = \(\) => \{/);
      if (oc === null) fail('C11', 'systemNotify: у уведомления нет обработчика .onclick');
      else {
        if (!oc.includes('window.focus()')) fail('C11', 'systemNotify: onclick без window.focus()');
        if (!oc.includes('n.close()'))
          fail('C11', 'systemNotify: onclick не закрывает уведомление');
        if (!oc.includes('runOrOffer('))
          fail('C11', 'systemNotify: onclick не вызывает runOrOffer(');
        if (/\bactivate\(/.test(oc))
          fail('C11', 'systemNotify: onclick вызывает activate() напрямую, мимо охраны модалки');
      }
      const nd = sb.indexOf("apiCall<NotifyDesktopOutcome>('notify_desktop'");
      const sp = sb.indexOf('sendNotification({');
      if (nd < 0)
        fail(
          'C11',
          "systemNotify: нет вызова apiCall<NotifyDesktopOutcome>('notify_desktop' (bindings.ts не создаётся в Windows-CI)",
        );
      else if (sp < 0) fail('C11', 'systemNotify: нет вызова sendNotification({');
      else if (nd > sp) fail('C11', 'systemNotify: sendNotification стоит РАНЬШЕ notifyDesktop');
      const sup = sb.indexOf("outcome === 'suppressed'");
      if (sup < 0) fail('C11', "systemNotify: нет ветки outcome === 'suppressed'");
      else {
        if (sup < nd || (sp >= 0 && sup > sp))
          fail(
            'C11',
            "systemNotify: ветка 'suppressed' не между notify_desktop и sendNotification",
          );
        const sBlock = sb.slice(sup, sb.indexOf('}', sup));
        if (!/\breturn false\b/.test(sBlock))
          fail('C11', "systemNotify: ветка 'suppressed' не возвращает false");
      }
      if (!/if \(target && !isWindowsDesktop\(\)\) void armFocusFallback\(/.test(sb))
        fail('C11', 'systemNotify: запасной механизм по фокусу не ограничен не-Windows');
    }
    const lb = funcBody(sn, /await listen<number>\('trackly-notify-activated', \(e\) => \{/);
    if (lb === null) fail('C11', "systemNotify: нет слушателя 'trackly-notify-activated'");
    else {
      if (!lb.includes('runOrOffer('))
        fail('C11', 'systemNotify: активация ОС не идёт через runOrOffer(');
      if (/\bactivate\(/.test(lb))
        fail('C11', 'systemNotify: активация ОС вызывает activate() напрямую');
    }
    if (!sn.includes('[role="dialog"][aria-modal="true"]'))
      fail('C11', 'systemNotify: нет селектора модального окна');
    const rb = funcBody(sn, /function runOrOffer\([^)]*\)[^{]*\{/);
    if (rb === null) fail('C11', 'systemNotify: runOrOffer не найден');
    else {
      const mq = rb.indexOf('MODAL_SELECTOR');
      const of = rb.indexOf('offer(');
      const ac = rb.search(/\btarget\.activate\(/);
      if (mq < 0 || of < 0 || ac < 0)
        fail('C11', 'systemNotify: runOrOffer без MODAL_SELECTOR / offer( / target.activate(');
      else if (mq > ac || of > ac)
        fail('C11', 'systemNotify: переход activate() стоит РАНЬШЕ проверки модального окна');
    }
    const ob = funcBody(sn, /function offer\([^)]*\)[^{]*\{/);
    if (ob === null || !ob.includes('pushToast('))
      fail('C11', 'systemNotify: offer() не показывает тост');
    const wb = funcBody(sn, /function isWindowsDesktop\(\)[^{]*\{/);
    if (wb === null || !/\/Windows\/\.test\(navigator\.userAgent\)/.test(wb))
      fail('C11', 'systemNotify: isWindowsDesktop не проверяет Windows в userAgent');
    const fb = funcBody(sn, /onFocusChanged\(\(\{[^}]*\}\) => \{/);
    if (fb === null) fail('C11', 'systemNotify: нет обработчика onFocusChanged');
    else {
      if (!fb.includes('offer('))
        fail('C11', 'systemNotify: запасной механизм не предлагает (offer)');
      if (/\bactivate\b/.test(fb))
        fail('C11', 'systemNotify: запасной механизм по фокусу переходит сам (activate)');
    }
    if (!/const FOCUS_FALLBACK_MS = \d/.test(sn))
      fail('C11', 'systemNotify: нет константы FOCUS_FALLBACK_MS');
    if (/features\//.test(sn)) fail('C11', 'systemNotify: обёртка знает про features/');
  }
  if (cn !== null) {
    if (
      !/showSystemNotification\('Trackly', text, \{\s*activate: \(\) => openRequestTarget\(ev\.requestId, 'chat'\)/.test(
        cn,
      )
    )
      fail('C11', 'chatNotifications: уведомление без цели openRequestTarget(ev.requestId, chat)');
    if (!cn.includes("const href = requestTargetHref(ev.requestId, 'chat');"))
      fail('C11', 'chatNotifications: ссылка тоста не через requestTargetHref');
    if (cn.includes('#/requests?id='))
      fail('C11', 'chatNotifications: ссылка собрана вручную, без уникального n');
  }
  const el = get('features/layout/EmployeeLayout.svelte');
  if (el !== null) {
    if (el.includes('new Notification(')) fail('C11', 'EmployeeLayout: прямой new Notification(');
    if (/\bdocument\.hidden\b/.test(el))
      fail('C11', 'EmployeeLayout: document.hidden вместо фокуса');
    const eb = funcBody(el, /function handleEmployeeWsEvent\([^)]*\)[^{]*\{/);
    if (eb === null) fail('C11', 'EmployeeLayout: handleEmployeeWsEvent не найден');
    else {
      const pt = eb.indexOf('pushToast(');
      const fo = eb.indexOf('if (!appHasFocus())');
      const sy = eb.indexOf('showSystemNotification(');
      if (pt < 0 || fo < 0 || sy < 0)
        fail(
          'C11',
          'EmployeeLayout: нет pushToast( / if (!appHasFocus()) / showSystemNotification(',
        );
      else {
        if (pt > fo) fail('C11', 'EmployeeLayout: тост стоит ПОСЛЕ проверки фокуса');
        if (sy < fo) fail('C11', 'EmployeeLayout: уведомление вне проверки фокуса');
        if (/\belse\b/.test(eb)) fail('C11', 'EmployeeLayout: «тост ИЛИ уведомление» (else)');
        if (!eb.slice(pt, fo).includes("requestTargetHref(event.requestId, 'info')"))
          fail('C11', 'EmployeeLayout: у тоста статуса нет ссылки на вкладку «Информация»');
        if (!eb.slice(sy).includes("openRequestTarget(event.requestId, 'info')"))
          fail('C11', 'EmployeeLayout: у уведомления статуса нет цели «Информация»');
      }
    }
  }
  const rp = get('features/requests/RequestsPage.svelte');
  if (rp !== null) {
    if (!/tab === 'chat' \|\| tab === 'info'/.test(rp) || !/setTab\(tab\)/.test(rp))
      fail('C11', 'RequestsPage: ссылка не выбирает вкладку и для chat, и для info');
  }
  const rt = get('features/requests/requestTarget.ts');
  if (rt !== null && (!rt.includes('&n=${Date.now()') || !rt.includes('seq')))
    fail('C11', 'requestTarget: у ссылки нет уникального параметра n');
  const rs = files[RUST_REL] === undefined ? null : stripComments(files[RUST_REL]);
  if (rs !== null) {
    if (!rs.includes('on_activated')) fail('C11', 'system_notify.rs: нет on_activated');
    if (!rs.includes('"trackly-notify-activated"'))
      fail('C11', 'system_notify.rs: нет имени события trackly-notify-activated');
    if (sn !== null && !sn.includes("'trackly-notify-activated'"))
      fail('C11', 'systemNotify: имя события расходится с Rust');
    for (const bad of [
      'windows_registry',
      'CURRENT_USER',
      'AppUserModelId\\',
      '.lnk',
      'APPDATA',
      'SetCurrentProcessExplicitAppUserModelID',
    ])
      if (rs.includes(bad)) fail('C11', `system_notify.rs: запрещённая строка ${bad} (portable)`);
    for (const must of [
      'uninstall.exe',
      'PORTABLE_AUMID',
      '.text2(',
      '.sound(None)',
      'Duration::Short',
      'on_dismissed',
      'async_runtime::spawn(',
      '.Setting()',
    ])
      if (!rs.includes(must)) fail('C11', `system_notify.rs: нет ${must} (R3-1)`);
    if (rs.includes('spawn_blocking')) fail('C11', 'system_notify.rs: вернулся spawn_blocking');
    if (rs.includes('.text1(&body)')) fail('C11', 'system_notify.rs: text1(&body) вместо text2');
    const nb = funcBody(rs, /pub async fn notify_desktop\s*\(/);
    if (nb === null) fail('C11', 'system_notify.rs: notify_desktop не найден');
    else {
      const pd = count(nb, 'probe_decision(');
      const tn = nb.indexOf('Toast::new(');
      if (pd < 2)
        fail('C11', 'system_notify.rs: в notify_desktop меньше двух вызовов probe_decision(');
      else if (tn < 0 || nb.lastIndexOf('probe_decision(') > tn)
        fail('C11', 'system_notify.rs: probe_decision( стоит ПОСЛЕ Toast::new( в notify_desktop');
      if (
        !nb.includes('ProbeDecision::Suppress') ||
        !nb.includes('NotifyDesktopOutcome::Suppressed')
      )
        fail('C11', 'system_notify.rs: notify_desktop не возвращает Suppressed при Suppress');
      const act = funcBody(nb, /\.on_activated\(move \|_action\| \{/);
      if (act === null || !/tracing::\w+!\(/.test(act))
        fail('C11', 'system_notify.rs: on_activated без tracing-вызова');
      const dis = funcBody(nb, /\.on_dismissed\(move \|reason\| \{/);
      if (dis === null || !/tracing::\w+!\(/.test(dis))
        fail('C11', 'system_notify.rs: on_dismissed без tracing-вызова');
    }
    const pb = funcBody(rs, /pub fn probe_decision\s*\(/);
    if (pb === null) fail('C11', 'system_notify.rs: probe_decision не найден');
    else {
      const arms = [...pb.matchAll(/ProbeKind::Disabled\)?\s*=>\s*ProbeDecision::(\w+)/g)];
      if (arms.length === 0) fail('C11', 'system_notify.rs: в probe_decision нет ветки Disabled');
      for (const a of arms)
        if (a[1] !== 'Suppress')
          fail('C11', `system_notify.rs: Disabled ведёт к ${a[1]}, а не к Suppress`);
    }
    // Приватность логов: ни текста, ни путей.
    const logCalls = [...rs.matchAll(/tracing::(?:trace|debug|info|warn|error)!\(/g)];
    let notifyLogs = 0;
    for (const m of logCalls) {
      const call = parenBody(rs, m.index + m[0].length - 1);
      if (call === null) continue;
      if (call.includes('notify_desktop:')) notifyLogs++;
      if (/\b(title|body|exe_dir|app_id)\b/.test(call) || call.includes('display()'))
        fail('C11', 'system_notify.rs: в tracing-вызове текст уведомления или путь');
    }
    if (notifyLogs < 6)
      fail('C11', `system_notify.rs: tracing «notify_desktop:» ${notifyLogs} < 6`);
  }
  fired.add('C11');

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
      name: 'перестановка: isMuted после pushToast',
      expect: 'C5',
      make: reorder(
        `${CHAT}chatNotifications.svelte.ts`,
        'if (isMuted(ev.requestId)) return;',
        'toastByRequest.set(ev.requestId, id);',
      ),
    },
    {
      name: 'перестановка: actorUserId после isMuted',
      expect: 'C5',
      make: reorder(
        `${CHAT}chatNotifications.svelte.ts`,
        'if (ev.actorUserId !== null && ev.actorUserId === me.id) return;',
        'if (isMuted(ev.requestId)) return;',
      ),
    },
    {
      name: 'перестановка: isMuted после showSystemNotification',
      expect: 'C5',
      make: reorder(
        `${CHAT}chatNotifications.svelte.ts`,
        'if (isMuted(ev.requestId)) return;',
        "void showSystemNotification('Trackly', text, {",
      ),
    },
    {
      name: 'возврат document.hidden',
      expect: 'C5',
      make: swapText(
        `${CHAT}chatNotifications.svelte.ts`,
        'if (!appHasFocus())',
        'if (document.hidden)',
      ),
    },
    {
      name: 'возврат ветки по роли',
      expect: 'C5',
      make: swapText(
        `${CHAT}chatNotifications.svelte.ts`,
        'const href =',
        "if (me.role === 'employee') return;\n  const href =",
      ),
    },
    {
      name: 'прямой new Notification(',
      expect: 'C5',
      make: swapText(
        `${CHAT}chatNotifications.svelte.ts`,
        "void showSystemNotification('Trackly', text, {",
        "new Notification('Trackly', { body: text });\n    void showSystemNotification('Trackly', text, {",
      ),
    },
    {
      name: 'unreadStore не читает mutedRequestIds',
      expect: 'C6',
      make: swapText(
        `${CHAT}unreadStore.svelte.ts`,
        'Array.isArray(res.mutedRequestIds)',
        'Array.isArray(res.items)',
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
      make: swapText(`${CHAT}chatNotifications.svelte.ts`, "ev.actorName ?? 'Участник'", 'ev.text'),
    },
    {
      name: 'C9: pre-wrap возвращён на .bubble',
      expect: 'C9',
      make: swapText(
        `${CHAT}ChatMessage.svelte`,
        'white-space: normal;\n    overflow-wrap: anywhere;',
        'white-space: pre-wrap;\n    overflow-wrap: anywhere;',
      ),
    },
    {
      name: 'C9: перестановка — шапка автора снова над облаком',
      expect: 'C9',
      make: () => {
        const rel = `${CHAT}ChatMessage.svelte`;
        const src = files[rel];
        const start = src.indexOf('    <div class="head">');
        const end = src.indexOf('</div>', start) + '</div>\n'.length;
        uniq(src, '    <div class="head">');
        uniq(src, '<div class="bubble"');
        const head = src.slice(start, end);
        const cut = src.slice(0, start) + src.slice(end);
        const at = cut.indexOf('  <div class="bubble"');
        return { ...files, [rel]: cut.slice(0, at) + head + cut.slice(at) };
      },
    },
    {
      name: 'C9: роль возвращена',
      expect: 'C9',
      make: swapText(
        `${CHAT}ChatMessage.svelte`,
        '<div class="head">',
        '<div class="head"><span class="author-role">x</span>',
      ),
    },
    {
      name: 'C9: второе время в конце пузыря',
      expect: 'C9',
      make: swapText(
        `${CHAT}ChatMessage.svelte`,
        '<span class="sr-only">',
        '<span class="time">x</span><span class="sr-only">',
      ),
    },
    {
      name: 'C9: перестановка — подвал со временем выше .text',
      expect: 'C9',
      make: reorder(
        `${CHAT}ChatMessage.svelte`,
        '    {#if !nameShown}\n      <div class="foot"><span class="time" title={fullTime}>{time}</span></div>\n    {/if}\n',
        '<span class="sr-only">{srPrefix}</span>',
      ),
    },
    {
      name: 'C9: подвал без условия',
      expect: 'C9',
      make: swapText(`${CHAT}ChatMessage.svelte`, '{#if !nameShown}', '{#if true}'),
    },
    {
      name: 'C9: шапка без условия',
      expect: 'C9',
      make: swapText(`${CHAT}ChatMessage.svelte`, '{#if nameShown}', '{#if true}'),
    },
    {
      name: 'C9: nameShown без !item.isOwn',
      expect: 'C9',
      make: swapText(
        `${CHAT}ChatMessage.svelte`,
        'const nameShown = $derived(groupStart && !item.isOwn);',
        'const nameShown = $derived(groupStart);',
      ),
    },
    {
      name: 'C9: удалено правило плотной группы',
      expect: 'C9',
      make: swapText(`${CHAT}ChatMessage.svelte`, '.msg:not(.group-start) {', '.msg-x {'),
    },
    {
      name: 'C9: знак в calc зазора заменён на +',
      expect: 'C9',
      make: swapText(
        `${CHAT}ChatMessage.svelte`,
        'calc(var(--tr-space-2xs) - var(--chat-row-gap',
        'calc(var(--tr-space-2xs) + var(--chat-row-gap',
      ),
    },
    {
      name: 'C12: перестановка — линия переехала из .header-row в .subheader',
      expect: 'C12',
      make: () => {
        const rel = 'lib/components/DetailPanel.svelte';
        const src = files[rel];
        const hrOpen = '.detail-header.with-subheader .header-row {';
        const subOpen = '.detail-header.with-subheader .subheader {';
        const line = '    border-bottom: 1px solid var(--tr-border);\n';
        uniq(src, hrOpen);
        uniq(src, subOpen);
        const a = src.indexOf(line, src.indexOf(hrOpen));
        const cut = src.slice(0, a) + src.slice(a + line.length);
        const at = cut.indexOf(subOpen) + subOpen.length;
        return { ...files, [rel]: cut.slice(0, at) + '\n' + line + cut.slice(at) };
      },
    },
    {
      name: 'C12: линия под вкладками снята (border-bottom: none)',
      expect: 'C12',
      make: swapText(
        'lib/components/DetailPanel.svelte',
        '    // Две линии: под строкой «заголовок + кнопки» (.header-row, R2-4) и под строкой вкладок (нижняя граница шапки, R3-3); цвет тот же, что у базовой шапки.\n    border-bottom: 1px solid var(--tr-border);',
        '    border-bottom: none;',
      ),
    },
    {
      name: 'C12: перестановка — линия под вкладками переехала в .subheader',
      expect: 'C12',
      make: () => {
        const rel = 'lib/components/DetailPanel.svelte';
        const src = files[rel];
        const wsOpen = '\n  .detail-header.with-subheader {';
        const subOpen = '.detail-header.with-subheader .subheader {';
        const line = '    border-bottom: 1px solid var(--tr-border);\n';
        uniq(src, wsOpen);
        uniq(src, subOpen);
        const a = src.indexOf(line, src.indexOf(wsOpen));
        if (a < 0 || a > src.indexOf(subOpen)) {
          console.error(`${TAG} SELFTEST FAIL — линия with-subheader не найдена`);
          process.exit(1);
        }
        const cut = src.slice(0, a) + src.slice(a + line.length);
        const at = cut.indexOf(subOpen) + subOpen.length;
        return { ...files, [rel]: cut.slice(0, at) + '\n' + line + cut.slice(at) };
      },
    },
    {
      name: 'C15: вернулась подсказка клавиш',
      expect: 'C15',
      make: swapText(
        `${CHAT}ChatComposer.svelte`,
        '<div class="foot">',
        '<div class="foot"><span class="hint">Enter — отправить, Shift+Enter — новая строка</span>',
      ),
    },
    {
      name: 'C15: перестановка — {#if showCounter} внутри .foot',
      expect: 'C15',
      make: () => {
        const rel = `${CHAT}ChatComposer.svelte`;
        const src = files[rel];
        uniq(src, '{#if showCounter}\n      <div class="foot">');
        const m = src.replace(
          /\{#if showCounter\}(\s*)<div class="foot">(\s*)(<span class="counter"[^\n]*<\/span>)(\s*)<\/div>(\s*)\{\/if\}/,
          '<div class="foot">{#if showCounter}$3{/if}</div>',
        );
        if (m === src) {
          console.error(`${TAG} SELFTEST FAIL — перестановка C15 не применилась`);
          process.exit(1);
        }
        return { ...files, [rel]: m };
      },
    },
    {
      name: 'C15: разный padding композера',
      expect: 'C15',
      make: swapText(
        `${CHAT}ChatComposer.svelte`,
        'padding: var(--tr-space-md) var(--tr-space-xl);\n    background: var(--tr-surface-raised);',
        'padding: var(--tr-space-md) var(--tr-space-xl) var(--tr-space-lg);\n    background: var(--tr-surface-raised);',
      ),
    },
    {
      name: 'C12: у .header-row снята линия',
      expect: 'C12',
      make: swapText(
        'lib/components/DetailPanel.svelte',
        '    padding: 0 var(--tr-space-xl) var(--tr-space-md);\n    border-bottom: 1px solid var(--tr-border);',
        '    padding: 0 var(--tr-space-xl) var(--tr-space-md);',
      ),
    },
    {
      name: 'C12: fill-body без отрицательного поля',
      expect: 'C12',
      make: swapText(
        'lib/components/DetailPanel.svelte',
        '.detail-panel.fill .fill-body {\n    margin: 0 calc(var(--tr-space-xl) * -1);',
        '.detail-panel.fill .fill-body {\n    margin: 0;',
      ),
    },
    {
      name: 'C12: у шапки «Чата» вернулся нижний зазор',
      expect: 'C12',
      make: swapText(
        'lib/components/DetailPanel.svelte',
        '.detail-panel.fill .detail-header.with-subheader {\n    margin-bottom: 0;',
        '.detail-panel.fill .detail-header.with-subheader {\n    margin-bottom: var(--tr-space-xl);',
      ),
    },
    {
      name: 'C12: padding ленты без бокового xl',
      expect: 'C12',
      make: swapText(
        `${CHAT}RequestChat.svelte`,
        'padding: var(--tr-space-sm) var(--tr-space-xl);\n    list-style',
        'padding: var(--tr-space-sm);\n    list-style',
      ),
    },
    {
      name: 'C12: padding композера без бокового xl',
      expect: 'C12',
      make: swapText(
        `${CHAT}ChatComposer.svelte`,
        'padding: var(--tr-space-md) var(--tr-space-xl);\n    background: var(--tr-surface-raised);',
        'padding: var(--tr-space-md);\n    background: var(--tr-surface-raised);',
      ),
    },
    {
      name: 'C12: снята линия с базового .detail-header (регрессия других страниц)',
      expect: 'C12',
      make: swapText(
        'lib/components/DetailPanel.svelte',
        '    background: var(--tr-surface-raised);\n    border-bottom: 1px solid var(--tr-border);\n  }',
        '    background: var(--tr-surface-raised);\n  }',
      ),
    },
    {
      name: 'C10: перестановка — ChatMuteToggle вынесен перед сниппетом полосы вкладок',
      expect: 'C10',
      make: reorder(
        'features/requests/RequestDetail.svelte',
        '<ChatMuteToggle requestId={request.id} />',
        '</script>',
      ),
    },
    {
      name: 'C10: пустой обработчик вместо toggleMute',
      expect: 'C10',
      make: swapText(
        `${CHAT}ChatMuteToggle.svelte`,
        'onclick={() => void toggleMute(requestId, !muted)}',
        'onclick={() => void 0}',
      ),
    },
    {
      name: 'C10: снят aria-pressed',
      expect: 'C10',
      make: swapText(`${CHAT}ChatMuteToggle.svelte`, 'aria-pressed={muted}', 'data-x={muted}'),
    },
    {
      name: 'C10: $effect в ChatMuteToggle',
      expect: 'C10',
      make: swapText(
        `${CHAT}ChatMuteToggle.svelte`,
        'const muted = $derived(',
        '$effect(() => {});\n  const muted = $derived(',
      ),
    },
    {
      name: 'C10: сниппет полосы вкладок без showTabs',
      expect: 'C10',
      make: swapText(
        'features/requests/RequestDetail.svelte',
        'subheader={showTabs && request ? tabsSubheader : undefined}',
        'subheader={request ? tabsSubheader : undefined}',
      ),
    },
    {
      name: 'C11: снят .onclick',
      expect: 'C11',
      make: swapText('lib/platform/systemNotify.ts', 'n.onclick = () => {', 'n.onclickx = () => {'),
    },
    {
      name: 'C11: перестановка — sendNotification раньше notifyDesktop',
      expect: 'C11',
      make: reorder(
        'lib/platform/systemNotify.ts',
        '      sendNotification({ title, body });\n',
        '      if (!(await isPermissionGranted())) return false;',
      ),
    },
    {
      name: 'C11: перестановка — activate раньше проверки модалки',
      expect: 'C11',
      make: reorder(
        'lib/platform/systemNotify.ts',
        '    target.activate();\n',
        '  try {\n    if (document.querySelector(MODAL_SELECTOR)) {',
      ),
    },
    {
      name: 'C11: onclick зовёт activate мимо runOrOffer',
      expect: 'C11',
      make: swapText(
        'lib/platform/systemNotify.ts',
        '        runOrOffer(target, body);\n      };',
        '        target.activate();\n      };',
      ),
    },
    {
      name: 'C11: активация ОС зовёт activate мимо runOrOffer',
      expect: 'C11',
      make: swapText(
        'lib/platform/systemNotify.ts',
        'if (rec) runOrOffer(rec.target, rec.body);',
        'if (rec) rec.target.activate();',
      ),
    },
    {
      name: 'C11: запасной механизм на Windows тоже',
      expect: 'C11',
      make: swapText(
        'lib/platform/systemNotify.ts',
        'if (target && !isWindowsDesktop()) void armFocusFallback(',
        'if (target) void armFocusFallback(',
      ),
    },
    {
      name: 'C11: запасной механизм навигирует',
      expect: 'C11',
      make: swapText(
        'lib/platform/systemNotify.ts',
        '        offer(p.target, p.body);',
        '        p.target.activate();',
      ),
    },
    {
      name: 'C11: снята проверка модалки',
      expect: 'C11',
      make: swapText(
        'lib/platform/systemNotify.ts',
        'if (document.querySelector(MODAL_SELECTOR)) {',
        'if (false) {',
      ),
    },
    {
      name: 'C11: вызов без цели в chatNotifications',
      expect: 'C11',
      make: swapText(
        `${CHAT}chatNotifications.svelte.ts`,
        "activate: () => openRequestTarget(ev.requestId, 'chat'),",
        'activate: () => {},',
      ),
    },
    {
      name: 'C11: ссылка тоста собрана вручную (без n)',
      expect: 'C11',
      make: swapText(
        `${CHAT}chatNotifications.svelte.ts`,
        "const href = requestTargetHref(ev.requestId, 'chat');",
        'const href = `#/requests?id=${ev.requestId}&tab=chat`;',
      ),
    },
    {
      name: 'C11: EmployeeLayout возвращён к «тост ИЛИ уведомление»',
      expect: 'C11',
      make: swapText(
        'features/layout/EmployeeLayout.svelte',
        '    if (!appHasFocus()) {\n      void showSystemNotification',
        '    else if (!appHasFocus()) {\n      void showSystemNotification',
      ),
    },
    {
      name: 'C11: EmployeeLayout снова document.hidden',
      expect: 'C11',
      make: swapText(
        'features/layout/EmployeeLayout.svelte',
        'if (!appHasFocus()) {',
        'if (document.hidden) {',
      ),
    },
    {
      name: 'C11: перестановка — уведомление раньше тоста в EmployeeLayout',
      expect: 'C11',
      make: reorder(
        'features/layout/EmployeeLayout.svelte',
        "    pushToast(negative ? 'info' : 'success', text, {\n      link: { label: 'Открыть заявку', href: requestTargetHref(event.requestId, 'info') },\n    });\n",
        '        label: `Открыть заявку №${event.requestId}`,\n      });\n    }\n',
      ),
    },
    {
      name: 'C11: EmployeeLayout без цели «Информация»',
      expect: 'C11',
      make: swapText(
        'features/layout/EmployeeLayout.svelte',
        "activate: () => openRequestTarget(event.requestId, 'info'),",
        "activate: () => openRequestTarget(event.requestId, 'chat'),",
      ),
    },
    {
      name: 'C11: RequestsPage не понимает tab=info',
      expect: 'C11',
      make: swapText(
        'features/requests/RequestsPage.svelte',
        "tab === 'chat' || tab === 'info'",
        "tab === 'chat'",
      ),
    },
    {
      name: 'C11: ссылка без уникального n',
      expect: 'C11',
      make: swapText('features/requests/requestTarget.ts', '&n=${Date.now()', '&x=${Date.now()'),
    },
    {
      name: 'C11: Rust — регистрация AUMID',
      expect: 'C11',
      make: () => ({
        ...files,
        [RUST_REL]: files[RUST_REL] + '\nfn x() { SetCurrentProcessExplicitAppUserModelID(); }\n',
      }),
    },
    {
      name: 'C11: Rust — запись в реестр (CURRENT_USER)',
      expect: 'C11',
      make: () => ({ ...files, [RUST_REL]: files[RUST_REL] + '\nconst K: u32 = CURRENT_USER;\n' }),
    },
    {
      name: 'C11: Rust — имя события разошлось',
      expect: 'C11',
      make: swapText(RUST_REL, '"trackly-notify-activated"', '"trackly-notify-clicked"'),
    },
    {
      name: 'C11: Rust — вернулся spawn_blocking',
      expect: 'C11',
      make: swapText(
        RUST_REL,
        'tauri::async_runtime::spawn(async move',
        'tauri::async_runtime::spawn_blocking(async move',
      ),
    },
    {
      name: 'C11: Rust — text1(&body) вместо text2',
      expect: 'C11',
      make: swapText(RUST_REL, '.text2(&body)', '.text1(&body)'),
    },
    {
      name: 'C11: Rust — без маркера uninstall.exe',
      expect: 'C11',
      make: swapText(
        RUST_REL,
        'const NSIS_UNINSTALLER: &str = "uninstall.exe";',
        'const NSIS_UNINSTALLER: &str = "x.bin";',
      ),
    },
    {
      name: 'C11: Rust — перестановка: второй probe_decision после Toast::new',
      expect: 'C11',
      make: reorder(
        RUST_REL,
        '            decision = probe_decision(choice, probe_kind);\n',
        'Toast::new(&aumid)',
      ),
    },
    {
      name: 'C11: Rust — откат при Disabled',
      expect: 'C11',
      make: swapText(
        RUST_REL,
        '(_, ProbeKind::Disabled) => ProbeDecision::Suppress,',
        '(_, ProbeKind::Disabled) => ProbeDecision::FallbackToPowershell,',
      ),
    },
    {
      name: 'C11: Rust — в лог попал body',
      expect: 'C11',
      make: swapText(
        RUST_REL,
        'probe = %probe_desc, decision = ?decision, "notify_desktop: probe");\n        if decision',
        'probe = %probe_desc, body = %body, decision = ?decision, "notify_desktop: probe");\n        if decision',
      ),
    },
    {
      name: 'C11: Rust — в лог попал путь',
      expect: 'C11',
      make: swapText(
        RUST_REL,
        'tracing::info!(token, "notify_desktop: called");',
        'tracing::info!(token, "notify_desktop: called");\n    tracing::info!(dir = %exe_dir.display(), "notify_desktop: x");',
      ),
    },
    {
      name: 'C11: Rust — on_dismissed без лога',
      expect: 'C11',
      make: swapText(
        RUST_REL,
        'tracing::info!(token, reason = ?reason, "notify_desktop: toast dismissed");',
        'let _ = reason;',
      ),
    },
    {
      name: 'C11: TS — suppressed после sendNotification',
      expect: 'C11',
      make: () => {
        const src = files['lib/platform/systemNotify.ts'];
        const blk = src.slice(
          src.indexOf("          if (outcome === 'suppressed') {"),
          src.indexOf('        } catch {\n          /* команда недоступна'),
        );
        uniq(src, blk);
        const cut = src.replace(blk, '');
        const k = '      sendNotification({ title, body });\n';
        const i = cut.indexOf(k) + k.length;
        return {
          ...files,
          ['lib/platform/systemNotify.ts']: cut.slice(0, i) + blk + cut.slice(i),
        };
      },
    },
    {
      name: 'C11: TS — нет ветки suppressed',
      expect: 'C11',
      make: swapText(
        'lib/platform/systemNotify.ts',
        "outcome === 'suppressed'",
        "outcome === 'other'",
      ),
    },
    {
      name: 'C11: пропал Rust-файл',
      expect: 'C0',
      make: () => {
        const copy = { ...files };
        delete copy[RUST_REL];
        return copy;
      },
    },
    {
      name: 'C13: удалён onloadcapture у ленты',
      expect: 'C13',
      make: swapText(
        `${CHAT}RequestChat.svelte`,
        '        onloadcapture={onFeedMediaSettled}\n',
        '',
      ),
    },
    {
      name: 'C13: перестановка — scrollToBottom раньше проверки followBottom',
      expect: 'C13',
      make: swapText(
        `${CHAT}RequestChat.svelte`,
        '      if (followBottom) scrollToBottom(false);\n',
        '      scrollToBottom(false);\n      if (followBottom) showJump = false;\n',
      ),
    },
    {
      name: 'C13: прокрутка синхронно, без tick()',
      expect: 'C13',
      make: swapText(
        `${CHAT}RequestChat.svelte`,
        '    void tick().then(() => {\n      if (followBottom) scrollToBottom(false);\n    });\n',
        '    if (followBottom) scrollToBottom(false);\n    void tick();\n',
      ),
    },
    {
      name: 'C13: followBottom стал $state',
      expect: 'C13',
      make: swapText(
        `${CHAT}RequestChat.svelte`,
        'let followBottom = true;',
        'let followBottom = $state(true);',
      ),
    },
    {
      name: 'C13: scrollToBottom не ставит followBottom = true (позиционно внутри тела)',
      expect: 'C13',
      make: () => {
        const rel = `${CHAT}RequestChat.svelte`;
        const src = files[rel];
        const body = funcBody(src, /function scrollToBottom\(/);
        uniq(src, body);
        const inner = body.replace('followBottom = true;', '');
        if (inner === body) {
          console.error(`${TAG} SELFTEST FAIL — в scrollToBottom нет followBottom = true`);
          process.exit(1);
        }
        return { ...files, [rel]: src.replace(body, inner) };
      },
    },
    {
      name: 'C13: перестановка — lastScrollTop присваивается раньше сравнения в onFeedScroll',
      expect: 'C13',
      make: () => {
        const rel = `${CHAT}RequestChat.svelte`;
        const src = files[rel];
        const body = funcBody(src, /function onFeedScroll\(/);
        uniq(src, body);
        const line = '      lastScrollTop = el.scrollTop;\n';
        uniq(body, line);
        const head = '    if (el) {\n';
        uniq(body, head);
        const moved = body.replace(line, '').replace(head, head + line);
        return { ...files, [rel]: src.replace(body, moved) };
      },
    },
    {
      name: 'C13: followBottom читается в $effect',
      expect: 'C13',
      make: swapText(
        `${CHAT}RequestChat.svelte`,
        '      generation += 1;\n      feed = null;',
        '      generation += 1;\n      if (followBottom) showJump = false;\n      feed = null;',
      ),
    },
    {
      name: 'C14: удалён вызов dismissChatToast(id)',
      expect: 'C14',
      make: swapText(`${CHAT}RequestChat.svelte`, '      dismissChatToast(id);\n', ''),
    },
    {
      name: 'C14: перестановка — вызов dismissChatToast перенесён в cleanup эффекта',
      expect: 'C14',
      make: () => {
        const rel = `${CHAT}RequestChat.svelte`;
        const src = files[rel];
        const call = '      dismissChatToast(id);\n';
        const cleanup = '    return () => {\n      setActiveChat(null);\n';
        uniq(src, call);
        uniq(src, cleanup);
        const cut = src.replace(call, '');
        return {
          ...files,
          [rel]: cut.replace(cleanup, cleanup + '      dismissChatToast(requestId);\n'),
        };
      },
    },
    {
      name: 'C14: в dismissChatToast удалён removeToast(',
      expect: 'C14',
      make: swapText(`${CHAT}chatNotifications.svelte.ts`, '  removeToast(id);\n', ''),
    },
    {
      name: 'C14: перестановка — toastByRequest.delete( раньше .get(',
      expect: 'C14',
      make: swapText(
        `${CHAT}chatNotifications.svelte.ts`,
        '  const id = toastByRequest.get(requestId);\n  if (id === undefined) return;',
        '  toastByRequest.delete(requestId);\n  const id = toastByRequest.get(requestId);\n  if (id === undefined) return;',
      ),
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
