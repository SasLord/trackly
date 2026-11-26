#!/usr/bin/env node
// [check-ws-disconnect-toast] Гейт индикации обрыва WS (D-18).
//
// Почему он существует: этот код ломался ДВА РАЗА В ПРОТИВОПОЛОЖНЫЕ СТОРОНЫ.
//   * сессия `ui-ws-toast-reports-flicker` (Bug A): тост обрыва показывался
//     КАЖДУЮ СЕКУНДУ — лечилось флагом эпизода `reconnecting`;
//   * сессия `ws-disconnect-toast-no-show` (GAP-1 фазы 41.7): тост не
//     показывался НИ РАЗУ — транзиентный тост с TTL 5000 мс истекал раньше, чем
//     пользователь успевал переключиться из десктопного окна в LAN-браузер
//     (оба сценария обрыва запускаются действием в ДРУГОМ окне).
// Отсюда инварианты, которые обязаны держаться ОДНОВРЕМЕННО:
//   не больше одного тоста за эпизод, не меньше одного, наблюдаемого, и (фаза 43,
//   D-14) не раньше RECONNECT_TOAST_DELAY_MS (5 с) после обрыва — кратковременный
//   обрыв не мигает тостом, — при этом таймер отложенного показа отменяем (иначе
//   призрачный тост уже восстановленного соединения, класс WR-04).
// Починка одного инварианта ломала другой, поэтому они закреплены вместе.
//
// `svelte-check` / `eslint` / `pnpm build` этот класс дефектов не видят: обе
// поломки были рантаймовыми и типам безразличны (`sticky` — необязательное
// поле, снятый вызов — просто мёртвый код).
//
// Гейт ИСПОЛНЯЮЩИЙ и состоит из двух частей.
//  1. Исполняемые проверки E1-E5 стора `packages/kit/src/stores/toast.svelte.ts`:
//     модуль транспилируется и реально вызывается, `setTimeout` подменяется
//     рекордером, поэтому «залипающий тост всё-таки получил TTL» ловится
//     фактом планирования таймера, а не чтением исходника.
//     Руна `$state(...)` перед транспиляцией срезается до `(...)` — гейт
//     проверяет ЛОГИКУ стора (push/TTL/дедуп/remove/вытеснение), не
//     реактивность.
//     E5 отдельно закрывает ТРЕТИЙ способ потерять индикацию обрыва, найденный
//     ревью фазы 41.7 (WR-01 отчёта): лимит очереди `MAX_TOASTS` вытеснял
//     старейший тост не глядя на `sticky`, а индикатор обрыва ставится ПЕРВЫМ и
//     дальше экран сыплет error-тостами — через десяток их индикатор исчезал, и
//     флаг эпизода `reconnecting` уже не давал показать его снова. Дедуп (E2)
//     от этого не защищал: он про стопку одинаковых, а не про вытеснение
//     единственного.
//  2. Структурные правила W1-W7 для `packages/transport/src/ws.ts` (текст без
//     комментариев, пробелы схлопнуты):
//       W1 — тост обрыва ставится залипающим (`sticky: true`);
//       W2 — вызов придушен флагом эпизода (`if (!reconnecting)` +
//            `reconnecting = true`) — инвариант «не больше одного»;
//       W3 — на успешном `onopen` залипающий тост снимается
//            (`clearReconnectingToast`) — иначе он висит навсегда;
//       W4 — `showReconnectingToast()` вообще вызывается — инвариант
//            «не меньше одного»;
//       W5 — каждая цепочка `import('@trackly/kit/stores/toast.svelte')` логирует
//            сбой в `.catch`. Пустой `.catch(() => {})` однажды уже скрыл
//            причину и стоил сессии лишних витков;
//       W6 — таймер переподключения отменяем (WR-04 ревью 41.7): результат
//            `setTimeout` хранится в модульной `reconnectTimer`, колбэк
//            обнуляет её и НЕ зовёт `connectBrowser()` при `refCount <= 0`,
//            browser-cleanup (`activeCleanup`, через него же `disconnectWs`)
//            зовёт `clearTimeout(reconnectTimer)` и обнуляет переменную. Иначе
//            release() в окне backoff оставлял таймер, который открывал сокет
//            без потребителя, а следующий первый потребитель — второй
//            (дубли событий, класс GAP-12-10);
//       W7 — тост обрыва отложен и отменяем (D-14, фаза 43): модульная
//            `reconnectToastTimer`; внутри гарда эпизода её значение присваивается
//            из `setTimeout`, колбэк обнуляет её и зовёт `showReconnectingToast()`,
//            задержка не меньше 5000 мс; тело `onopen` и `activeCleanup` зовут
//            `clearTimeout(reconnectToastTimer)` с обнулением. Поведение (5 с,
//            отмена на onopen/release) исполняет check-ws-resync.mjs.
//
// `--selftest` прогоняет те же проверки на встроенных эталонах: эталон обязан
// пройти, мутанты M1-M19 — провалиться. `--file-ws=<path>` подменяет путь
// ws.ts (мутационная проверка гейта на дореформенном исходнике из git).
//
// Реальных данных организации и людей гейт не содержит.
//
// Usage:
//   node scripts/check-ws-disconnect-toast.mjs             # проверить репозиторий
//   node scripts/check-ws-disconnect-toast.mjs --selftest  # встроенный self-test
//   node scripts/check-ws-disconnect-toast.mjs --file-ws=/tmp/ws.ts  # другой ws.ts

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';
import { Buffer } from 'node:buffer';
import { stripComments } from './_strip.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const STORE = path.join(UI_ROOT, '../packages/kit/src/stores/toast.svelte.ts');
const WS = path.join(UI_ROOT, '../packages/transport/src/ws.ts');
const TAG = '[check-ws-disconnect-toast]';
const RECONNECT_MESSAGE = 'Соединение с сервером потеряно. Переподключение…';
/// Зеркало `MAX_TOASTS` из стора (стор его не экспортирует). Используется
/// только как ВЕРХНЯЯ граница в E5 («лимит не отключён целиком»), поэтому
/// расхождение на единицу гейт не ломает — важен порядок величины, а не
/// точное значение.
const MAX_TOASTS_EXPECTED = 10;

// ---------------------------------------------------------------------------
// Часть 1 — исполняемые проверки стора
// ---------------------------------------------------------------------------

function transpileToEsm(tsSource, label) {
  const require = createRequire(import.meta.url);
  let ts;
  try {
    ts = require('typescript');
  } catch {
    console.error(
      `${TAG} FAIL — не разрешается пакет \`typescript\` (прямой devDependency ui/package.json); запусти \`pnpm install\` в ui/.`,
    );
    process.exit(1);
  }
  // Руна `$state(x)` → `(x)`: вне компилятора Svelte это необъявленный вызов и
  // модуль не загрузился бы вовсе. Семантика значения при этом та же.
  const deruned = tsSource.split('$state(').join('(');
  return ts.transpileModule(deruned, {
    compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext },
    fileName: label,
    reportDiagnostics: false,
  }).outputText;
}

async function loadModuleFromTs(tsSource, label) {
  const js = transpileToEsm(tsSource, label);
  // data:-URL: стор тостов самодостаточен (ни одного import).
  return import(`data:text/javascript;base64,${Buffer.from(js, 'utf8').toString('base64')}`);
}

/**
 * Прогнать E1-E5 на загруженном модуле стора.
 * `setTimeout` подменяется рекордером на время вызовов, чтобы увидеть САМ ФАКТ
 * планирования TTL, а не ждать его истечения.
 */
async function runStoreCases(tsSource, label) {
  const failures = [];
  let mod;
  try {
    mod = await loadModuleFromTs(tsSource, label);
  } catch (e) {
    return [`стор не загружается: ${e.message}`];
  }
  for (const fn of ['pushToast', 'removeToast']) {
    if (typeof mod[fn] !== 'function') {
      failures.push(`стор не экспортирует ${fn} — проверять нечего`);
    }
  }
  if (failures.length) return failures;

  const scheduled = [];
  const realSetTimeout = globalThis.setTimeout;
  globalThis.setTimeout = (_fn, delay) => {
    scheduled.push(delay);
    return 0;
  };
  try {
    // E1 — залипающий тост попадает в список и НЕ получает TTL-таймера.
    const stickyId = mod.pushToast('warning', RECONNECT_MESSAGE, { sticky: true });
    if (!mod.toastStore.items.some((t) => t.id === stickyId)) {
      failures.push('E1: залипающий тост не попал в toastStore.items');
    }
    if (scheduled.length !== 0) {
      failures.push(
        `E1: для залипающего тоста запланирован TTL-таймер (${scheduled.join(', ')} мс) — он истечёт сам и индикация обрыва снова станет ненаблюдаемой`,
      );
    }

    // E2 — повторный залипающий тост с тем же текстом не копится.
    const beforeDedup = mod.toastStore.items.length;
    const stickyId2 = mod.pushToast('warning', RECONNECT_MESSAGE, { sticky: true });
    if (mod.toastStore.items.length !== beforeDedup || stickyId2 !== stickyId) {
      failures.push(
        'E2: второй залипающий тост с тем же текстом создал новую запись — флапающее соединение накопит стопку несгораемых тостов (симптом Bug A без спасительного TTL)',
      );
    }

    // E3 — removeToast снимает залипающий тост (иначе он висит навсегда).
    mod.removeToast(stickyId);
    if (mod.toastStore.items.some((t) => t.id === stickyId)) {
      failures.push('E3: removeToast не снял залипающий тост');
    }

    // E4 — обычный тост по-прежнему получает TTL (залипание не стало дефолтом).
    scheduled.length = 0;
    mod.pushToast('warning', 'обычный тост');
    if (scheduled.length !== 1) {
      failures.push(
        `E4: обычный тост запланировал ${scheduled.length} таймеров вместо 1 — залипание не должно становиться поведением по умолчанию`,
      );
    }

    // E5 — залипающий индикатор переживает переполнение очереди (WR-01 ревью
    // 41.7): при обрыве связи он ставится ПЕРВЫМ, а дальше экран сыплет
    // error-тостами. Вытеснение по возрасту без учёта `sticky` снимало
    // индикатор, а флаг эпизода `reconnecting` в ws.ts не даёт показать его
    // снова → GAP-1 воспроизводится. E1-E4 этого не видели: они не доводят
    // очередь до лимита.
    //
    // Второй половиной E5 проверяется, что лимит при этом НЕ отключён
    // целиком — иначе «починкой» годился бы снос вытеснения как такового.
    mod.toastStore.items = [];
    const indicatorId = mod.pushToast('warning', RECONNECT_MESSAGE, { sticky: true });
    const FLOOD = 30;
    for (let i = 0; i < FLOOD; i += 1) {
      mod.pushToast('error', `не удалось загрузить данные (${i})`);
    }
    if (!mod.toastStore.items.some((t) => t.id === indicatorId)) {
      failures.push(
        `E5: залипающий индикатор обрыва вытеснен лимитом очереди после ${FLOOD} обычных тостов — пользователь на активно ошибающемся экране теряет индикацию обрыва, а флаг эпизода reconnecting уже не даст показать её снова (GAP-1 фазы 41.7)`,
      );
    }
    const nonSticky = mod.toastStore.items.filter((t) => !t.sticky).length;
    if (nonSticky > MAX_TOASTS_EXPECTED) {
      failures.push(
        `E5: в очереди ${nonSticky} НЕ залипающих тостов после ${FLOOD} пушей — лимит очереди перестал работать; освобождать место обязано вытеснение старейшего НЕ залипающего, а не отказ от лимита`,
      );
    }
    mod.toastStore.items = [];
  } catch (e) {
    failures.push(`исключение при прогоне стора: ${e.message}`);
  } finally {
    globalThis.setTimeout = realSetTimeout;
  }
  return failures;
}

// ---------------------------------------------------------------------------
// Часть 2 — структурные правила ws.ts
// ---------------------------------------------------------------------------

/** Текст .ts без комментариев, пробелы/переводы строк схлопнуты в один пробел. */
function normalizeTs(src) {
  return stripComments(src).replace(/\s+/g, ' ');
}

function runStructural(wsSrc) {
  const failures = [];
  const text = normalizeTs(wsSrc);

  // W1 — тост обрыва ставится залипающим.
  if (!/pushToast\(\s*'warning'\s*,[^;]*?\{\s*sticky:\s*true\s*,?\s*\}\s*\)/.test(text)) {
    failures.push(
      'W1: тост обрыва не помечен { sticky: true } — он истечёт по TTL раньше, чем пользователь переключится в LAN-браузер, и D-18 снова «не показывается вообще»',
    );
  }

  // W2 — не больше одного тоста за эпизод.
  // Намеренно НЕ требует здесь самого вызова: иначе W4 стало бы недостижимым
  // (любой мутант ловился бы этим правилом, и «не меньше одного» никогда не
  // проверялось бы по-настоящему). W2 отвечает за гард, W4 — за вызов.
  if (!/if\s*\(\s*!reconnecting\s*\)\s*\{\s*reconnecting\s*=\s*true\s*;/.test(text)) {
    failures.push(
      'W2: вызов showReconnectingToast() не придушен флагом эпизода (if (!reconnecting) { reconnecting = true; … }) — вернётся спам тостов каждую секунду (Bug A)',
    );
  }

  // W3 — залипающий тост снимается на успешном onopen.
  if (!/onopen\s*=\s*\(\s*\)\s*=>\s*\{[^}]*clearReconnectingToast\(\s*\)/.test(text)) {
    failures.push(
      'W3: в ws.onopen нет clearReconnectingToast() — залипающий тост останется висеть после восстановления связи',
    );
  }

  // W4 — не меньше одного: тост вообще вызывается.
  if (!/showReconnectingToast\(\s*\)\s*;/.test(text)) {
    failures.push(
      'W4: showReconnectingToast() не вызывается нигде — индикация обрыва мертва (ровно симптом GAP-1 фазы 41.7)',
    );
  }

  // W5 — ни одна цепочка импорта тост-стора не глушит сбой молча.
  // Окно цепочки = от её import до следующего import (или до конца файла);
  // тело .catch = от `.catch(` до ближайшего `});`, закрывающего эту ссылку.
  const IMPORT = "import('@trackly/kit/stores/toast.svelte')";
  const starts = [];
  for (let i = text.indexOf(IMPORT); i !== -1; i = text.indexOf(IMPORT, i + 1)) {
    starts.push(i);
  }
  if (starts.length === 0) {
    failures.push(`W5: в ws.ts нет ни одной цепочки ${IMPORT}`);
  }
  starts.forEach((start, n) => {
    const end = n + 1 < starts.length ? starts[n + 1] : text.length;
    const window = text.slice(start, end);
    const catchIdx = window.indexOf('.catch(');
    if (catchIdx === -1) {
      failures.push('W5: цепочка импорта тост-стора без .catch — необработанный rejection');
      return;
    }
    const closeIdx = window.indexOf('});', catchIdx);
    const body = window.slice(catchIdx, closeIdx === -1 ? window.length : closeIdx);
    if (!/console\.(error|warn)\(/.test(body)) {
      failures.push(
        'W5: .catch в цепочке импорта тост-стора ничего не логирует — пустой .catch(() => {}) однажды уже скрыл причину и стоил сессии лишних витков',
      );
    }
  });

  // W6 — висящий таймер переподключения отменяем (WR-04 ревью 41.7).
  if (!/\blet\s+reconnectTimer\b/.test(text)) {
    failures.push(
      'W6: нет модульной переменной reconnectTimer — таймер переподключения нечем отменить',
    );
  }
  if (!/reconnectTimer\s*=\s*setTimeout\(/.test(text)) {
    failures.push(
      'W6: результат setTimeout переподключения не сохраняется в reconnectTimer — release()/disconnectWs() не отменят висящий таймер, и он откроет сокет без потребителя',
    );
  }
  if (
    !/reconnectTimer\s*=\s*setTimeout\(\s*\(\s*\)\s*=>\s*\{\s*reconnectTimer\s*=\s*null\s*;\s*if\s*\(\s*refCount\s*<=\s*0\s*\)\s*\{?\s*return\s*;?\s*\}?\s*connectBrowser\(\s*\)/.test(
      text,
    )
  ) {
    failures.push(
      'W6: колбэк таймера переподключения не обнуляет reconnectTimer и/или зовёт connectBrowser() без гарда refCount <= 0 — таймер воскресит сокет, который никто не держит',
    );
  }
  const cleanupBody = arrowBody(text, 'activeCleanup = () => {');
  if (
    cleanupBody === null ||
    !/clearTimeout\(\s*reconnectTimer\s*\)/.test(cleanupBody) ||
    !/reconnectTimer\s*=\s*null/.test(cleanupBody)
  ) {
    failures.push(
      'W6: browser-cleanup (activeCleanup) не зовёт clearTimeout(reconnectTimer) с обнулением — release() в окне backoff оставит живой таймер',
    );
  }

  // W7 — тост обрыва отложен и отменяем (D-14).
  if (!/\blet\s+reconnectToastTimer\b/.test(text)) {
    failures.push(
      'W7: нет модульной переменной reconnectToastTimer — отложенный тост обрыва нечем отменить',
    );
  }
  if (
    !/if\s*\(\s*!reconnecting\s*\)\s*\{\s*reconnecting\s*=\s*true\s*;\s*reconnectToastTimer\s*=\s*setTimeout\(\s*\(\s*\)\s*=>\s*\{\s*reconnectToastTimer\s*=\s*null\s*;\s*showReconnectingToast\(\s*\)\s*;?\s*\}\s*,/.test(
      text,
    )
  ) {
    failures.push(
      'W7: в гарде эпизода тост не откладывается через reconnectToastTimer = setTimeout(() => { reconnectToastTimer = null; showReconnectingToast(); }, …) — либо тост показывается сразу (мигает на коротком обрыве), либо таймер нечем отменить',
    );
  }
  const delayMatch = /reconnectToastTimer\s*=\s*setTimeout\(.*?\}\s*,\s*(\w+)\s*\)/.exec(text);
  let delayMs = null;
  if (delayMatch) {
    if (/^\d+$/.test(delayMatch[1])) {
      delayMs = Number(delayMatch[1]);
    } else {
      const decl = new RegExp(`\\b${delayMatch[1]}\\s*=\\s*(\\d+)`).exec(text);
      delayMs = decl ? Number(decl[1]) : null;
    }
  }
  if (delayMs === null || delayMs < 5000) {
    failures.push(
      `W7: задержка тоста обрыва ${delayMs === null ? 'не определена' : `${delayMs} мс`} — должна быть не меньше 5000 мс (D-14), иначе короткий обрыв снова мигает тостом`,
    );
  }
  const openBody = arrowBody(text, 'onopen = () => {');
  if (
    openBody === null ||
    !/clearTimeout\(\s*reconnectToastTimer\s*\)/.test(openBody) ||
    !/reconnectToastTimer\s*=\s*null/.test(openBody)
  ) {
    failures.push(
      'W7: ws.onopen не отменяет таймер тоста (clearTimeout(reconnectToastTimer) с обнулением) — после быстрого переподключения всплывёт призрачный тост',
    );
  }
  if (
    cleanupBody === null ||
    !/clearTimeout\(\s*reconnectToastTimer\s*\)/.test(cleanupBody) ||
    !/reconnectToastTimer\s*=\s*null/.test(cleanupBody)
  ) {
    failures.push(
      'W7: activeCleanup не отменяет таймер тоста — после release() всплывёт тост уже снесённого соединения',
    );
  }
  return failures;
}

/** Тело стрелочной функции после `anchor` (оканчивается на `{`) по балансу скобок. */
function arrowBody(text, anchor) {
  const at = text.indexOf(anchor);
  if (at === -1) return null;
  const open = at + anchor.length - 1;
  let depth = 0;
  for (let i = open; i < text.length; i += 1) {
    if (text[i] === '{') depth += 1;
    else if (text[i] === '}') {
      depth -= 1;
      if (depth === 0) return text.slice(open + 1, i);
    }
  }
  return null;
}

// ---------------------------------------------------------------------------
// Эталоны и мутанты для --selftest
// ---------------------------------------------------------------------------

const GOOD_STORE = `
const TTL = { error: 6000, warning: 5000, success: 4000, info: 4000 };
const MAX_TOASTS = 10;
export const toastStore = $state({ items: [] });
export function pushToast(kind, message, opts) {
  const sticky = opts?.sticky === true;
  if (sticky) {
    const existing = toastStore.items.find((t) => t.sticky && t.message === message);
    if (existing) {
      return existing.id;
    }
  }
  const id = crypto.randomUUID();
  let toDrop = toastStore.items.length - MAX_TOASTS + 1;
  if (toDrop > 0) {
    const dropped = new Set();
    for (const t of toastStore.items) {
      if (toDrop <= 0) break;
      if (t.sticky) continue;
      dropped.add(t.id);
      toDrop -= 1;
    }
    if (dropped.size > 0) {
      toastStore.items = toastStore.items.filter((t) => !dropped.has(t.id));
    }
  }
  toastStore.items = [...toastStore.items, { id, kind, message, sticky }];
  if (!sticky) {
    setTimeout(() => {
      toastStore.items = toastStore.items.filter((t) => t.id !== id);
    }, TTL[kind]);
  }
  return id;
}
export function removeToast(id) {
  toastStore.items = toastStore.items.filter((t) => t.id !== id);
}
`;

const GOOD_WS = `
let reconnecting = false;
let refCount = 0;
let reconnectDelay = 1000;
let reconnectToastId = null;
let reconnectTimer = null;
let reconnectToastTimer = null;
const RECONNECT_TOAST_DELAY_MS = 5000;
let activeCleanup = null;

function showReconnectingToast() {
  import('@trackly/kit/stores/toast.svelte')
    .then(({ pushToast }) => {
      reconnectToastId = pushToast('warning', '${RECONNECT_MESSAGE}', {
        sticky: true,
      });
    })
    .catch((e) => {
      console.error('[ws] не удалось показать тост обрыва соединения', e);
    });
}

function clearReconnectingToast() {
  const id = reconnectToastId;
  if (id === null) {
    return;
  }
  reconnectToastId = null;
  import('@trackly/kit/stores/toast.svelte')
    .then(({ removeToast }) => {
      removeToast(id);
    })
    .catch((e) => {
      console.error('[ws] не удалось снять тост обрыва соединения', e);
    });
}

function connectBrowser() {
  ws.onopen = () => {
    if (reconnecting) {
      reconnecting = false;
      clearReconnectingToast();
      if (reconnectToastTimer !== null) {
        clearTimeout(reconnectToastTimer);
        reconnectToastTimer = null;
      }
    }
    reconnectDelay = 1000;
  };
  ws.onclose = () => {
    if (refCount <= 0) {
      return;
    }
    if (!reconnecting) {
      reconnecting = true;
      reconnectToastTimer = setTimeout(() => {
        reconnectToastTimer = null;
        showReconnectingToast();
      }, RECONNECT_TOAST_DELAY_MS);
    }
    reconnectTimer = setTimeout(() => {
      reconnectTimer = null;
      if (refCount <= 0) {
        return;
      }
      connectBrowser();
    }, 1000);
  };
}

function openFirst() {
  connectBrowser();
  activeCleanup = () => {
    if (reconnectTimer !== null) {
      clearTimeout(reconnectTimer);
      reconnectTimer = null;
    }
    if (reconnectToastTimer !== null) {
      clearTimeout(reconnectToastTimer);
      reconnectToastTimer = null;
    }
    if (ws) {
      ws.onclose = null;
      ws.close();
      ws = null;
    }
    reconnecting = false;
  };
}
`;

const STORE_MUTANTS = [
  {
    id: 'M1',
    name: 'залипающий тост тоже получает TTL (возврат исходного дефекта)',
    from: '  if (!sticky) {\n    setTimeout(',
    to: '  if (true) {\n    setTimeout(',
  },
  {
    id: 'M2',
    name: 'дедуп залипающих тостов снят (стопка несгораемых тостов)',
    from: '    const existing = toastStore.items.find((t) => t.sticky && t.message === message);',
    to: '    const existing = undefined;',
  },
  {
    id: 'M3',
    name: 'залипание стало дефолтом (обычные тосты перестали истекать)',
    from: '  const sticky = opts?.sticky === true;',
    to: '  const sticky = true;',
  },
  {
    id: 'M4',
    name: 'removeToast не удаляет',
    from: '  toastStore.items = toastStore.items.filter((t) => t.id !== id);\n}',
    to: '}',
  },
  {
    id: 'M10',
    name: 'вытеснение по возрасту без учёта sticky (возврат дефекта WR-01 ревью 41.7)',
    from:
      '  let toDrop = toastStore.items.length - MAX_TOASTS + 1;\n' +
      '  if (toDrop > 0) {\n' +
      '    const dropped = new Set();\n' +
      '    for (const t of toastStore.items) {\n' +
      '      if (toDrop <= 0) break;\n' +
      '      if (t.sticky) continue;\n' +
      '      dropped.add(t.id);\n' +
      '      toDrop -= 1;\n' +
      '    }\n' +
      '    if (dropped.size > 0) {\n' +
      '      toastStore.items = toastStore.items.filter((t) => !dropped.has(t.id));\n' +
      '    }\n' +
      '  }',
    to:
      '  if (toastStore.items.length >= MAX_TOASTS) {\n' +
      '    toastStore.items = toastStore.items.slice(toastStore.items.length - MAX_TOASTS + 1);\n' +
      '  }',
  },
  {
    id: 'M11',
    name: 'лимит очереди снят целиком (ложная «починка» вытеснения)',
    from: '  let toDrop = toastStore.items.length - MAX_TOASTS + 1;\n  if (toDrop > 0) {',
    to: '  let toDrop = 0;\n  if (toDrop > 0) {',
  },
  {
    id: 'M12',
    name: 'залипающие вытесняются наравне с обычными (гард `if (t.sticky) continue` снят)',
    from: '      if (t.sticky) continue;\n',
    to: '',
  },
];

const WS_MUTANTS = [
  {
    id: 'M5',
    name: 'снят { sticky: true } — тост снова истекает за 5 с',
    from:
      "reconnectToastId = pushToast('warning', '" +
      RECONNECT_MESSAGE +
      "', {\n        sticky: true,\n      });",
    to: "reconnectToastId = pushToast('warning', '" + RECONNECT_MESSAGE + "');",
  },
  {
    id: 'M6',
    name: 'снят флаг эпизода — спам тостов каждую секунду (Bug A)',
    from:
      '    if (!reconnecting) {\n      reconnecting = true;\n      reconnectToastTimer = setTimeout(() => {\n' +
      '        reconnectToastTimer = null;\n        showReconnectingToast();\n      }, RECONNECT_TOAST_DELAY_MS);\n    }',
    to: '    showReconnectingToast();',
  },
  {
    id: 'M7',
    name: 'снят вызов тоста из onclose — индикация обрыва мертва',
    from: '        showReconnectingToast();\n',
    to: '',
  },
  {
    id: 'M8',
    name: 'снято снятие залипающего тоста на onopen — висит навсегда',
    from: '      clearReconnectingToast();',
    to: '',
  },
  {
    id: 'M9',
    name: 'пустой .catch — сбой тоста снова глохнет молча',
    from: "    .catch((e) => {\n      console.error('[ws] не удалось показать тост обрыва соединения', e);\n    });",
    to: '    .catch(() => {});',
  },
  {
    // WR-01 (ревью 41.12): `/*` внутри строчного комментария не открывает блок.
    // Блочно-первая вычистка стирала бы лишнюю цепочку с пустым .catch между
    // `templates/*.html` и ближайшим `*/` ниже — W5 молчало бы.
    id: 'M10',
    name: '`/*` в строчном комментарии не прячет цепочку import() с пустым .catch',
    from: 'function connectBrowser() {',
    to:
      'function connectBrowser() {\n' +
      '  // шаблоны лежат в templates/*.html\n' +
      "  import('@trackly/kit/stores/toast.svelte').then(() => {}).catch(() => {});\n" +
      '  /* настоящий блочный комментарий */',
  },
  {
    id: 'M13',
    name: 'таймер переподключения не сохраняется (возврат WR-04)',
    from: '    reconnectTimer = setTimeout(() => {',
    to: '    setTimeout(() => {',
  },
  {
    id: 'M16',
    name: 'таймер отложенного тоста не сохраняется (нечем отменить — призрачный тост)',
    from: '      reconnectToastTimer = setTimeout(() => {',
    to: '      setTimeout(() => {',
  },
  {
    id: 'M17',
    name: 'clearTimeout таймера тоста снят из onopen — тост всплывает после восстановления',
    from:
      '      if (reconnectToastTimer !== null) {\n        clearTimeout(reconnectToastTimer);\n' +
      '        reconnectToastTimer = null;\n      }\n',
    to: '',
  },
  {
    id: 'M18',
    name: 'clearTimeout таймера тоста снят из activeCleanup — тост всплывает после release',
    from:
      '    if (reconnectToastTimer !== null) {\n      clearTimeout(reconnectToastTimer);\n' +
      '      reconnectToastTimer = null;\n    }\n',
    to: '',
  },
  {
    id: 'M19',
    name: 'задержка тоста заменена на 0 — короткий обрыв мигает тостом',
    from: '      }, RECONNECT_TOAST_DELAY_MS);',
    to: '      }, 0);',
  },
  {
    id: 'M14',
    name: 'clearTimeout убран из browser-cleanup — release() в окне backoff не отменяет таймер',
    from: '      clearTimeout(reconnectTimer);\n',
    to: '',
  },
  {
    id: 'M15',
    name: 'снят гард refCount в колбэке таймера — сокет без потребителя',
    from: '      if (refCount <= 0) {\n        return;\n      }\n      connectBrowser();',
    to: '      connectBrowser();',
  },
];

function assertUniqueAnchor(source, m) {
  // Якорь обязан быть в эталоне ровно один раз — иначе split/join молча
  // заденет соседнее место и мутант окажется неотличим от эталона.
  const occurrences = source.split(m.from).length - 1;
  if (occurrences !== 1) {
    console.error(
      `${TAG} SELFTEST FAIL — якорь мутанта ${m.id} «${m.name}» встречается ${occurrences} раз, нужен ровно 1`,
    );
    process.exit(1);
  }
}

async function selftest() {
  const goodStore = await runStoreCases(GOOD_STORE, 'good-store.ts');
  if (goodStore.length) {
    console.error(
      `${TAG} SELFTEST FAIL — эталонный стор не прошёл E1-E5:\n  - ${goodStore.join('\n  - ')}`,
    );
    process.exit(1);
  }
  const goodWs = runStructural(GOOD_WS);
  if (goodWs.length) {
    console.error(
      `${TAG} SELFTEST FAIL — эталонный ws.ts не прошёл W1-W7:\n  - ${goodWs.join('\n  - ')}`,
    );
    process.exit(1);
  }

  for (const m of STORE_MUTANTS) {
    assertUniqueAnchor(GOOD_STORE, m);
    const failures = await runStoreCases(GOOD_STORE.split(m.from).join(m.to), 'mutant-store.ts');
    if (!failures.length) {
      console.error(
        `${TAG} SELFTEST FAIL — мутант ${m.id} «${m.name}» прошёл E1-E5: гейт его не ловит`,
      );
      process.exit(1);
    }
    console.log(`${TAG}   ${m.id} «${m.name}» пойман: ${failures[0]}`);
  }
  for (const m of WS_MUTANTS) {
    assertUniqueAnchor(GOOD_WS, m);
    const failures = runStructural(GOOD_WS.split(m.from).join(m.to));
    if (!failures.length) {
      console.error(
        `${TAG} SELFTEST FAIL — мутант ${m.id} «${m.name}» прошёл W1-W7: гейт его не ловит`,
      );
      process.exit(1);
    }
    console.log(`${TAG}   ${m.id} «${m.name}» пойман: ${failures[0]}`);
  }
  console.log(
    `${TAG} selftest OK — эталоны проходят, ${STORE_MUTANTS.length + WS_MUTANTS.length} мутантов ловятся`,
  );
}

async function main() {
  const argv = process.argv.slice(2);
  if (argv.includes('--selftest')) return selftest();
  const wsArg = argv.find((a) => a.startsWith('--file-ws='));
  const wsPath = wsArg ? path.resolve(wsArg.slice('--file-ws='.length)) : WS;

  const failures = [];
  let storeSrc;
  let wsSrc;
  try {
    storeSrc = fs.readFileSync(STORE, 'utf8');
  } catch {
    console.error(`${TAG} FAIL — не удалось прочитать ${STORE}`);
    process.exit(1);
  }
  try {
    wsSrc = fs.readFileSync(wsPath, 'utf8');
  } catch {
    console.error(`${TAG} FAIL — не удалось прочитать ${wsPath}`);
    process.exit(1);
  }
  failures.push(...(await runStoreCases(storeSrc, path.basename(STORE))));
  failures.push(...runStructural(wsSrc));

  if (failures.length) {
    console.error(`${TAG} FAIL —\n  - ${failures.join('\n  - ')}`);
    process.exit(1);
  }
  console.log(`${TAG} OK — исполняемые E1-E5 по стору тостов, структурные W1-W7 по ws.ts`);
}

main();
