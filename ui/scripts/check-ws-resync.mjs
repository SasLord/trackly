#!/usr/bin/env node
// [check-ws-resync] Исполняющий гейт клиента транспорта WS (фаза 43, план 03; DLV-01, DLV-04).
//
// Зачем: ресинк на resync_required/повторном открытии, отложенный тост обрыва, мгновенное
// переподключение по online/visibilitychange и проба живости сокета — рантаймовая логика с
// таймерами. svelte-check/build её не видят (проектный урок «compile gates miss runtime»),
// структурный гейт check-ws-disconnect-toast читает текст, а не исполняет.
//
// Исполняет НАСТОЯЩИЙ packages/transport/src/ws.ts через стенд _rune-harness.mjs. Подменены:
// `./transport.svelte` (isTauri=false), тост-стор (пишет вызовы в globalThis.__h), WebSocket,
// window/document и часы (setTimeout/clearTimeout с ручным advance — реального ожидания нет).
//
// Кейсы:
//   [R1]  resync_required -> ровно один ресинк, WsEvent-обработчик не вызван, тостов нет;
//   [R2]  hello и pong -> ни ресинка, ни dispatch;
//   [R3]  первое открытие без обрыва -> ресинка нет; обрыв + переоткрытие -> ровно один;
//   [R4]  два resync_required в окне 500 мс -> один вызов сразу + хвостовой; дебаунс не залипает;
//   [R5]  тост обрыва: до 5000 мс нет, на 5000 мс ровно один sticky, повторы эпизода не плодят;
//   [R6]  переподключение быстрее 5 с -> тоста нет вовсе (таймер отменён на onopen);
//   [R7]  показанный тост снимается на onopen;
//   [R8]  release в окне до 5 с -> тоста нет (таймер отменён в cleanup);
//   [R9]  online при ws===null -> сокет сразу; visibilitychange в hidden -> ничего;
//   [R10] visibilitychange при OPEN: ping, при молчании 5 с сокет рвётся и открывается новый;
//         любой кадр до истечения отменяет разрыв;
//   [R11] после release слушатели online/visibilitychange сняты;
//   [R12] onclose сокета ВО ВРЕМЯ пробы (CR-03): после всех таймеров открыт ровно один сокет, кадр
//         через onmessage каждого созданного сокета даёт ровно один вызов WsEvent-обработчика;
//   [R13] onclose во время пробы, новый сокет уже открывается — проба его НЕ убивает;
//   [R12b] запоздалый onclose замещённого сокета не обнуляет ws живого: следующая проба шлёт
//         ping в живой сокет, а не открывает второй;
//   [R2]  (IN-06) hello с protocol 2 — один console.warn, ни dispatch, ни ресинка.
//
// Usage: node scripts/check-ws-resync.mjs [--selftest]

import { setImmediate } from 'node:timers';
import { gateMain, installHarness, pkgUrl } from './_rune-harness.mjs';

const TAG = '[check-ws-resync]';
const WS = '../../packages/transport/src/ws.ts';

const TOAST_STUB = `
export const pushToast = (kind, message, opts) => {
  const id = 't' + (globalThis.__h.toasts.length + 1);
  globalThis.__h.toasts.push({ id, kind, message, opts });
  return id;
};
export const removeToast = (id) => { globalThis.__h.removed.push(id); };
`;

async function runCases(overrides) {
  installHarness({
    stubs: {
      './transport.svelte': 'export const isTauri = false;',
      '@trackly/kit/stores/toast.svelte': TOAST_STUB,
    },
    overrides,
  });

  const h = { toasts: [], removed: [], resyncs: 0, events: [] };
  globalThis.__h = h;

  // Управляемые часы.
  let now = 0;
  let seq = 0;
  const timers = new Map();
  globalThis.setTimeout = (fn, ms) => {
    seq += 1;
    timers.set(seq, { id: seq, at: now + (ms ?? 0), fn });
    return seq;
  };
  globalThis.clearTimeout = (id) => {
    timers.delete(id);
  };
  const advance = (ms) => {
    const target = now + ms;
    for (;;) {
      const next = [...timers.values()]
        .filter((t) => t.at <= target)
        .sort((a, b) => a.at - b.at || a.id - b.id)[0];
      if (!next) break;
      now = next.at;
      timers.delete(next.id);
      next.fn();
    }
    now = target;
  };
  const flush = () => new Promise((r) => setImmediate(r));

  // Фейковый WebSocket.
  const sockets = [];
  globalThis.WebSocket = class FakeWebSocket {
    constructor(url) {
      this.url = url;
      this.readyState = 0;
      this.sent = [];
      sockets.push(this);
    }
    open() {
      this.readyState = 1;
      this.onopen?.();
    }
    recv(obj) {
      this.onmessage?.({ data: JSON.stringify(obj) });
    }
    drop() {
      this.readyState = 3;
      this.onclose?.();
    }
    send(d) {
      this.sent.push(d);
    }
    close() {
      this.readyState = 3;
    }
  };

  // Фейковые window/document с реестром слушателей.
  const mkTarget = () => {
    const l = new Map();
    return {
      l,
      addEventListener: (t, f) => {
        if (!l.has(t)) l.set(t, new Set());
        l.get(t).add(f);
      },
      removeEventListener: (t, f) => {
        l.get(t)?.delete(f);
      },
      fire: (t) => [...(l.get(t) ?? [])].forEach((f) => f()),
      count: (t) => l.get(t)?.size ?? 0,
    };
  };
  const win = mkTarget();
  win.location = { protocol: 'http:', host: 'host.test' };
  const doc = mkTarget();
  doc.visibilityState = 'visible';
  globalThis.window = win;
  globalThis.document = doc;

  const mod = await import(pkgUrl('transport', 'ws.ts'));

  const bad = [];
  const expect = (code, cond, msg) => {
    if (!cond) bad.push(`[${code}] ${msg}`);
  };

  /** Свежий потребитель: подписки, connectWs, первый сокет открыт. */
  const fresh = async () => {
    timers.clear();
    sockets.length = 0;
    h.toasts.length = 0;
    h.removed.length = 0;
    h.resyncs = 0;
    h.events.length = 0;
    doc.visibilityState = 'visible';
    const unE = mod.onWsEvent((e) => h.events.push(e));
    const unR = mod.onWsResync(() => {
      h.resyncs += 1;
    });
    const release = await mod.connectWs();
    sockets[0].open();
    return {
      s: sockets[0],
      done: () => {
        release();
        unE();
        unR();
      },
    };
  };

  let c;

  // R1
  c = await fresh();
  c.s.recv({ type: 'resync_required' });
  await flush();
  expect('R1', h.resyncs === 1, `resync_required: ресинков ${h.resyncs}, нужен 1`);
  expect('R1', h.events.length === 0, 'resync_required попал в обработчики WsEvent');
  expect('R1', h.toasts.length === 0, 'ресинк показал тост (D-09)');
  c.done();

  // R2
  c = await fresh();
  c.s.recv({ type: 'hello', protocol: 1, pingIntervalSecs: 25, idleTimeoutSecs: 60 });
  c.s.recv({ type: 'pong' });
  expect('R2', h.resyncs === 0, `hello/pong вызвали ресинк (${h.resyncs})`);
  expect('R2', h.events.length === 0, 'hello/pong попали в обработчики WsEvent');
  c.s.recv({ type: 'entities_changed', placeIds: [], deviceIds: [], groupIds: [] });
  expect('R2', h.events.length === 1, 'обычное событие не дошло до обработчика');
  c.done();

  // R2 (IN-06): версия протокола hello
  {
    const origWarn = console.warn;
    let warns = 0;
    console.warn = () => {
      warns += 1;
    };
    try {
      c = await fresh();
      c.s.recv({ type: 'hello', protocol: 1, pingIntervalSecs: 25, idleTimeoutSecs: 60 });
      expect('R2', warns === 0, `hello protocol 1 вызвал предупреждение (${warns})`);
      c.s.recv({ type: 'hello', protocol: 2, pingIntervalSecs: 25, idleTimeoutSecs: 60 });
      expect('R2', warns === 1, `hello protocol 2: предупреждений ${warns}, нужно ровно 1`);
      expect('R2', h.events.length === 0, 'hello protocol 2 попал в обработчики WsEvent');
      expect('R2', h.resyncs === 0, 'hello protocol 2 вызвал ресинк');
      c.done();
    } finally {
      console.warn = origWarn;
    }
  }

  // R3
  c = await fresh();
  expect('R3', h.resyncs === 0, `первое открытие вызвало ресинк (${h.resyncs})`);
  c.s.drop();
  advance(1000);
  expect('R3', sockets.length === 2, `после backoff сокетов ${sockets.length}, нужно 2`);
  sockets[1].open();
  expect('R3', h.resyncs === 1, `переоткрытие: ресинков ${h.resyncs}, нужен 1`);
  c.done();

  // R4
  c = await fresh();
  c.s.recv({ type: 'resync_required' });
  c.s.recv({ type: 'resync_required' });
  expect('R4', h.resyncs === 1, `два кадра в окне: сразу ${h.resyncs} вызовов, нужен 1`);
  advance(600);
  expect('R4', h.resyncs === 2, `хвостовой вызов: ${h.resyncs}, нужно 2`);
  advance(600);
  c.s.recv({ type: 'resync_required' });
  expect(
    'R4',
    h.resyncs === 3,
    `после окна новый ресинк не выполнен (${h.resyncs}) — дебаунс залип`,
  );
  c.done();

  // R5
  c = await fresh();
  c.s.drop();
  advance(4999);
  await flush();
  expect('R5', h.toasts.length === 0, `тост раньше 5000 мс (${h.toasts.length})`);
  advance(1);
  await flush();
  expect('R5', h.toasts.length === 1, `на 5000 мс тостов ${h.toasts.length}, нужен 1`);
  expect('R5', h.toasts[0]?.opts?.sticky === true, 'тост обрыва не sticky');
  sockets[sockets.length - 1].drop();
  advance(20000);
  await flush();
  expect('R5', h.toasts.length === 1, `повторы эпизода создали тосты (${h.toasts.length})`);
  c.done();

  // R6
  c = await fresh();
  c.s.drop();
  advance(1000);
  sockets[1].open();
  advance(10000);
  await flush();
  expect('R6', h.toasts.length === 0, `быстрое переподключение дало тост (${h.toasts.length})`);
  c.done();

  // R7
  c = await fresh();
  c.s.drop();
  advance(5000);
  await flush();
  sockets[sockets.length - 1].open();
  await flush();
  expect(
    'R7',
    h.toasts.length === 1 && h.removed.includes(h.toasts[0].id),
    `onopen не снял тост (removed=${JSON.stringify(h.removed)})`,
  );
  c.done();

  // R8
  c = await fresh();
  c.s.drop();
  advance(2000);
  c.done();
  advance(10000);
  await flush();
  expect('R8', h.toasts.length === 0, `тост показан после release (${h.toasts.length})`);

  // R9
  c = await fresh();
  c.s.drop();
  const before = sockets.length;
  win.fire('online');
  expect('R9', sockets.length === before + 1, 'online при ws===null не открыл сокет сразу');
  c.done();
  c = await fresh();
  c.s.drop();
  doc.visibilityState = 'hidden';
  doc.fire('visibilitychange');
  expect('R9', sockets.length === 1, 'visibilitychange в hidden открыл сокет');
  c.done();

  // R10
  c = await fresh();
  doc.fire('visibilitychange');
  expect(
    'R10',
    c.s.sent.some((m) => JSON.parse(m).type === 'ping'),
    'при OPEN-сокете ping не отправлен',
  );
  advance(5000);
  expect('R10', c.s.readyState === 3, 'молчащий сокет не закрыт');
  expect('R10', sockets.length === 2, `новый сокет не открыт (${sockets.length})`);
  c.done();
  c = await fresh();
  doc.fire('visibilitychange');
  advance(2000);
  c.s.recv({ type: 'pong' });
  advance(10000);
  expect('R10', sockets.length === 1, 'живой сокет (pong пришёл) всё равно разорван');
  c.done();

  // R11
  c = await fresh();
  expect('R11', win.count('online') === 1, 'слушатель online не зарегистрирован');
  c.done();
  expect(
    'R11',
    win.count('online') === 0 && doc.count('visibilitychange') === 0,
    'слушатели online/visibilitychange не сняты после release',
  );

  // R12: onclose во время пробы, когда reconnectTimer ещё висит в момент срабатывания пробы
  // (onclose на 4500 мс пробы -> reconnectTimer на 5500, проба истекает на 5000).
  c = await fresh();
  doc.fire('visibilitychange');
  advance(4500);
  c.s.drop();
  advance(2 * 5000 + 30000 + 1000);
  const live12 = sockets.filter((s) => s.readyState !== 3);
  expect('R12', live12.length === 1, `открыто сокетов ${live12.length}, нужен ровно 1 (двойной сокет)`);
  h.events.length = 0;
  for (const s of sockets) {
    s.onmessage?.({
      data: JSON.stringify({ type: 'entities_changed', placeIds: [], deviceIds: [], groupIds: [] }),
    });
  }
  expect(
    'R12',
    h.events.length === 1,
    `кадр через onmessage каждого сокета: обработчик вызван ${h.events.length} раз, нужен 1 (дубли)`,
  );
  c.done();

  // R13: onclose во время пробы на 3000 мс; backoff 1000 -> новый сокет на 4000 мс (CONNECTING);
  // проба истекает на 5000 мс и не должна его убить.
  c = await fresh();
  doc.fire('visibilitychange');
  advance(3000);
  c.s.drop();
  advance(1000);
  expect('R13', sockets.length === 2, `после backoff сокетов ${sockets.length}, нужно 2`);
  advance(1000);
  expect('R13', sockets[1]?.readyState !== 3, 'проба убила сокет, созданный переподключением');
  expect('R13', sockets.length === 2, `число сокетов выросло до ${sockets.length}, нужно 2`);
  c.done();

  // R12b: запоздалый onclose замещённого сокета не обнуляет ws живого.
  c = await fresh();
  const stale = c.s;
  stale.drop();
  advance(1000);
  sockets[1].open();
  stale.onclose?.();
  advance(30000);
  doc.fire('visibilitychange');
  expect(
    'R12b',
    sockets.length === 2,
    `запоздалый onclose породил второй сокет (${sockets.length})`,
  );
  expect(
    'R12b',
    sockets[1].sent.some((m) => JSON.parse(m).type === 'ping'),
    'после запоздалого onclose ping не ушёл в живой сокет',
  );
  c.done();

  return bad;
}

const MUTANTS = [
  {
    name: 'resyncAll на resync_required снят',
    file: WS,
    from: "if (msg.type === 'resync_required') {\n        resyncAll();\n        return;",
    to: "if (msg.type === 'resync_required') {\n        return;",
    expect: 'R1',
  },
  {
    name: 'resyncAll на onopen после обрыва снят',
    file: WS,
    from: '      resyncAll();\n    }\n    reconnectDelay = 1000; // Reset',
    to: '    }\n    reconnectDelay = 1000; // Reset',
    expect: 'R3',
  },
  {
    name: 'задержка тоста 0',
    file: WS,
    from: '}, RECONNECT_TOAST_DELAY_MS);',
    to: '}, 0);',
    expect: 'R5',
  },
  {
    name: 'clearTimeout таймера тоста на onopen снят',
    file: WS,
    from:
      '      if (reconnectToastTimer !== null) {\n        clearTimeout(reconnectToastTimer);\n' +
      '        reconnectToastTimer = null;\n      }\n      resyncAll();',
    to: '      resyncAll();',
    expect: 'R6',
  },
  {
    name: 'clearTimeout таймера тоста в cleanup снят',
    file: WS,
    from:
      '        // D-14: отложенный тост уже снесённого соединения показывать нельзя.\n' +
      '        if (reconnectToastTimer !== null) {\n          clearTimeout(reconnectToastTimer);\n' +
      '          reconnectToastTimer = null;\n        }\n',
    to: '',
    expect: 'R8',
  },
  {
    name: 'дебаунс ресинка снят',
    file: WS,
    from: '  if (resyncTimer !== null) {\n    resyncPending = true;\n    return;\n  }',
    to: '',
    expect: 'R4',
  },
  {
    name: 'слушатель online не регистрируется',
    file: WS,
    from: "window.addEventListener('online', onOnline);",
    to: '',
    expect: 'R9',
  },
  {
    name: 'removeEventListener online в cleanup снят',
    file: WS,
    from: "        window.removeEventListener('online', onOnline);\n",
    to: '',
    expect: 'R11',
  },
  {
    name: 'проба не рвёт сокет по таймауту',
    file: WS,
    from: '    handleLost();\n    connectNow();\n  }, PROBE_TIMEOUT_MS);',
    to: '  }, PROBE_TIMEOUT_MS);',
    expect: 'R10',
  },
  {
    name: 'resync_required уходит в dispatch вместо служебной обработки',
    file: WS,
    from: "if (msg.type === 'resync_required') {",
    to: "if (msg.type === '__never__') {",
    expect: 'R1',
  },
];

await gateMain({ tag: TAG, scriptUrl: import.meta.url, mutants: MUTANTS, runCases });
