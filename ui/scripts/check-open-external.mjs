#!/usr/bin/env node
// [check-open-external] Поведенческий гейт открытия ссылок из чата
// (фаза 41.5, план 20 т.2; Nyquist 41.5 G7; MSG-01, UI-SPEC §7.2, T-41.5-51, T-41.5-74).
//
// Исполняет НАСТОЯЩИЙ openExternal.ts + toast.svelte.ts через стенд _rune-harness.mjs;
// заглушен только @tauri-apps/plugin-shell (open). Признак десктопа — window.__TAURI_INTERNALS__,
// он читается при загрузке модуля, поэтому «браузерный» кейс грузит модуль второй раз (?web).
//
// Кейсы:
//   [O1] десктоп + https: переход webview отменён СИНХРОННО (до возврата из обработчика),
//        адрес открыт через shell.open ровно один раз, тоста нет;
//   [O2] shell.open отказал → тост error «Не удалось открыть ссылку», адреса в тосте нет;
//   [O3] десктоп + javascript:/file: → переход отменён, shell.open НЕ вызван;
//   [O4] LAN-браузер → обработчик не вмешивается (обычная ссылка target=_blank).
//
// Usage: node scripts/check-open-external.mjs [--selftest]

import { gateMain, installHarness, srcUrl, until } from './_rune-harness.mjs';

const TAG = '[check-open-external]';
const OE = 'features/requests/chat/openExternal.ts';

const STUBS = {
  '@tauri-apps/plugin-shell': `export async function open(href) { return globalThis.__h.open(href); }`,
};

async function runCases(overrides) {
  const H = { opened: [], fail: false };
  H.open = async (href) => {
    H.opened.push(href);
    if (H.fail) throw new Error('shell scope');
  };
  globalThis.__h = H;
  installHarness({ stubs: STUBS, overrides });
  const toast = await import(srcUrl('lib/stores/toast.svelte.ts'));

  const bad = [];
  const expect = (code, cond, msg) => {
    if (!cond) bad.push(`[${code}] ${msg}`);
  };
  const ev = () => {
    const e = { prevented: 0 };
    e.preventDefault = () => {
      e.prevented += 1;
    };
    return e;
  };
  const settle = () => new Promise((r) => setTimeout(r, 20));

  globalThis.window = { __TAURI_INTERNALS__: {} };
  const desk = await import(srcUrl(OE, '?desktop'));

  // O1
  const url = 'https://example.com/a?token=SECRET';
  let e = ev();
  desk.onExternalLinkClick(e, url);
  expect(
    'O1',
    e.prevented === 1,
    `переход webview не отменён синхронно (preventDefault ×${e.prevented})`,
  );
  await until(() => H.opened.length > 0, 1000);
  expect(
    'O1',
    H.opened.length === 1 && H.opened[0] === url,
    `shell.open: ${JSON.stringify(H.opened)}`,
  );
  await settle();
  expect('O1', toast.toastStore.items.length === 0, 'успешное открытие дало тост');

  // O2
  H.opened.length = 0;
  H.fail = true;
  toast.clearToasts();
  e = ev();
  desk.onExternalLinkClick(e, url);
  const shown = await until(() => toast.toastStore.items.length > 0, 1000);
  const t = toast.toastStore.items[0];
  expect(
    'O2',
    shown && t?.kind === 'error' && t?.message === 'Не удалось открыть ссылку',
    `тост при сбое: ${JSON.stringify(t)}`,
  );
  expect(
    'O2',
    !toast.toastStore.items.some(
      (x) => x.message.includes('SECRET') || x.message.includes('example.com'),
    ),
    'адрес попал в тост',
  );
  H.fail = false;

  // O3
  for (const href of [
    'javascript:alert(1)',
    'file:///C:/Windows/system32/calc.exe',
    'JAVASCRIPT:alert(1)',
  ]) {
    H.opened.length = 0;
    e = ev();
    desk.onExternalLinkClick(e, href);
    await settle();
    expect('O3', e.prevented === 1, `${href}: переход не отменён`);
    expect('O3', H.opened.length === 0, `${href}: открыт через shell.open`);
  }

  // O4
  globalThis.window = {};
  const web = await import(srcUrl(OE, '?web'));
  H.opened.length = 0;
  e = ev();
  web.onExternalLinkClick(e, 'https://example.com/');
  await settle();
  expect(
    'O4',
    e.prevented === 0 && H.opened.length === 0,
    `браузер: preventDefault ×${e.prevented}, open ×${H.opened.length}`,
  );
  return bad;
}

const MUTANTS = [
  {
    name: 'инверсия признака десктопа',
    file: OE,
    from: '  if (!isTauri) return;',
    to: '  if (isTauri) return;',
    expect: 'O1',
  },
  {
    name: 'сбой открытия проглатывается молча',
    file: OE,
    from: "      pushToast('error', 'Не удалось открыть ссылку');",
    to: '      void 0;',
    expect: 'O2',
  },
  {
    name: 'перестановка: preventDefault после await shell.open',
    file: OE,
    edits: [
      ['  event.preventDefault();\n  void (async () => {', '  void (async () => {'],
      ['      await open(href);\n', '      await open(href);\n      event.preventDefault();\n'],
    ],
    expect: 'O1',
  },
  {
    name: 'инверсия фильтра схемы',
    file: OE,
    from: '  if (!/^https?:\\/\\//i.test(href)) {',
    to: '  if (/^https?:\\/\\//i.test(href)) {',
    expect: 'O3',
  },
  {
    name: 'адрес выводится в тосте',
    file: OE,
    from: "      pushToast('error', 'Не удалось открыть ссылку');",
    to: "      pushToast('error', `Не удалось открыть ссылку ${href}`);",
    expect: 'O2',
  },
];

await gateMain({ tag: TAG, scriptUrl: import.meta.url, mutants: MUTANTS, runCases });
