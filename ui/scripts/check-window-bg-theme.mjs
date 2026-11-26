#!/usr/bin/env node
// [check-window-bg-theme] Поведенческий гейт фона нативного окна (quick 261126-rbg).
//
// Исполняет НАСТОЯЩИЙ packages/kit/src/stores/theme.svelte.ts через стенд _rune-harness.mjs;
// заглушены только `@tauri-apps/api/window` (getCurrentWindow) и браузерные глобалы (window,
// document, localStorage, getComputedStyle, requestAnimationFrame).
//
// Кейсы:
//   [W1] Tauri + initTheme (светлая): setBackgroundColor вызван с --tr-bg светлой темы (#eef1f6),
//        значение из getComputedStyle приходит с пробелом и обязано быть обрезано;
//   [W2] после setTheme('dark') последний вызов несёт --tr-bg тёмной темы (#0e1218), а не константу;
//   [W3] браузер (нет __TAURI_INTERNALS__): ни одного вызова и ни одного запроса импорта
//        @tauri-apps/api/window;
//   [W4] setBackgroundColor отклоняется: setTheme не бросает и нет unhandled rejection.
//
// Usage: node scripts/check-window-bg-theme.mjs [--selftest]

import { gateMain, installHarness, pkgUrl, until } from './_rune-harness.mjs';

const TAG = '[check-window-bg-theme]';
const THEME = '../../packages/kit/src/stores/theme.svelte.ts';
const COLORS = { light: ' #eef1f6', dark: ' #0e1218' };

const STUBS = {
  '@tauri-apps/api/window': `globalThis.__h.imports += 1;
export function getCurrentWindow() {
  return { setBackgroundColor: (c) => globalThis.__h.setBg(c) };
}`,
};

function def(name, value) {
  Object.defineProperty(globalThis, name, { value, configurable: true, writable: true });
}

async function runCases(overrides) {
  const H = {
    imports: 0,
    calls: [],
    fail: false,
    setBg: async (c) => {
      H.calls.push(c);
      if (H.fail) throw new Error('нет прав');
    },
  };
  globalThis.__h = H;
  const unhandled = [];
  process.on('unhandledRejection', (e) => unhandled.push(e));
  installHarness({ stubs: STUBS, overrides });

  const store = new Map();
  const root = { classList: { add() {}, remove() {} }, dataset: {} };
  const win = { matchMedia: () => ({ matches: false, addEventListener() {} }) };
  def('window', win);
  def('document', { documentElement: root });
  def('localStorage', {
    getItem: (k) => (store.has(k) ? store.get(k) : null),
    setItem: (k, v) => store.set(k, String(v)),
  });
  def('requestAnimationFrame', (cb) => cb());
  win.getComputedStyle = (el) => ({
    getPropertyValue: (n) => (n === '--tr-bg' ? (COLORS[el.dataset.theme] ?? '') : ''),
  });

  const bad = [];
  const expect = (code, cond, msg) => {
    if (!cond) bad.push(`[${code}] ${msg}`);
  };
  const settle = () => new Promise((r) => setTimeout(r, 80));
  let n = 0;
  const fresh = () => import(pkgUrl('kit', 'stores/theme.svelte.ts', `?t=${++n}`));

  // W3: браузер
  let t = await fresh();
  store.set('trackly:theme', 'light');
  t.initTheme();
  t.setTheme('dark');
  await settle();
  expect(
    'W3',
    H.calls.length === 0 && H.imports === 0,
    `в браузере вызовов ${H.calls.length}, импортов API окна ${H.imports}`,
  );

  // W1, W2: Tauri
  win.__TAURI_INTERNALS__ = {};
  t = await fresh();
  store.set('trackly:theme', 'light');
  t.initTheme();
  const got1 = await until(() => H.calls.length > 0);
  expect('W1', got1 && H.calls[0] === '#eef1f6', `initTheme: вызовы ${JSON.stringify(H.calls)}`);
  H.calls.length = 0;
  t.setTheme('dark');
  const got2 = await until(() => H.calls.length > 0);
  expect(
    'W2',
    got2 && H.calls[H.calls.length - 1] === '#0e1218',
    `setTheme(dark): вызовы ${JSON.stringify(H.calls)}`,
  );

  // W4: отказ
  H.fail = true;
  H.calls.length = 0;
  let threw = null;
  try {
    t.setTheme('light');
  } catch (e) {
    threw = e;
  }
  await settle();
  expect('W4', threw === null, `setTheme бросил: ${threw}`);
  expect('W4', H.calls.length > 0, 'при отказе вызов даже не состоялся (проба не несущая)');
  expect('W4', unhandled.length === 0, `unhandled rejection: ${unhandled.length}`);
  return bad;
}

const MUTANTS = [
  {
    name: 'убран вызов syncWindowBackground из applyResolved',
    file: THEME,
    from: '  void syncWindowBackground();',
    to: '',
    expect: 'W1',
  },
  {
    name: 'снят guard __TAURI_INTERNALS__ (вызов и в браузере)',
    file: THEME,
    from: "!('__TAURI_INTERNALS__' in window)",
    to: 'false',
    expect: 'W3',
  },
  {
    name: 'проглатывание ошибки заменено пробросом',
    file: THEME,
    from: '  } catch {',
    to: '  } catch (e) {\n    throw e;',
    expect: 'W4',
  },
  {
    name: 'цвет захардкожен вместо --tr-bg темы',
    file: THEME,
    from: "style.getPropertyValue('--tr-bg').trim();",
    to: "style.getPropertyValue('--tr-bg').trim() && '#eef1f6';",
    expect: 'W2',
  },
  {
    name: 'значение не обрезается',
    file: THEME,
    from: "style.getPropertyValue('--tr-bg').trim();",
    to: "style.getPropertyValue('--tr-bg');",
    expect: 'W1',
  },
];

await gateMain({ tag: TAG, scriptUrl: import.meta.url, mutants: MUTANTS, runCases });
