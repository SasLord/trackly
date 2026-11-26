// Plan 06-04: Dual-transport WebSocket client.
// Plan 12-17 (GAP-12-10): refcounted singleton — see connectWs() below.
//
// Browser path: native WebSocket → /api/v1/ws, exponential backoff reconnect.
// Tauri path: @tauri-apps/api/event listen('trackly-event') — no WS server needed.
//
// WsEvent union is synchronised with Rust dto/printer.rs WsEvent enum:
//   { type: 'new_request' | 'request_status_changed' | 'printer_alert'
//          | 'number_space_changed' | 'entities_changed' | 'attachments_changed'
//          | 'chat_message' | 'chat_read_changed' }
// Видимость событий фильтруется на сервере (is_visible_to, D-23); клиент решает
// только, что перечитать.
// Фаза 41.7 (D-18): сокет теперь открывает и оболочка Admin/Manager (Layout.svelte),
// поэтому тост «Соединение с сервером потеряно» (showReconnectingToast) может
// появиться на любом экране админа — принято, придушивание по экранам отвергнуто.
// Инварианты индикации обрыва (оба обязаны держаться одновременно):
//   * НЕ БОЛЬШЕ одного тоста за эпизод — флаг `reconnecting` (введён против
//     спама Bug A, сессия `ui-ws-toast-reports-flicker`);
//   * НЕ МЕНЬШЕ одного — тост залипающий, без TTL, снимается на `onopen`
//     (сессия `ws-disconnect-toast-no-show`: транзиентный тост истекал раньше,
//     чем пользователь успевал переключиться из десктопного окна в браузер).
// Фаза 43 (DLV-01/DLV-04) — протокол служебных кадров и ресинк:
//   * сервер -> клиент: `hello` (первый кадр: protocol, pingIntervalSecs, idleTimeoutSecs),
//     `pong` (ответ на пробу), `resync_required` (клиент отстал и потерял события — Lagged);
//     клиент -> сервер: текстовый `{"type":"ping"}` (проба живости). Служебные кадры
//     разбираются ДО dispatch и обработчикам WsEvent не отдаются (D-03).
//   * ресинк (D-08): `onWsResync(handler)` — реестр «перечитай всё»; `resyncAll()`
//     зовётся на resync_required, на успешном открытии после обрыва и (Tauri) на событии
//     `trackly-resync`; вызовы в пределах RESYNC_DEBOUNCE_MS схлопываются. Ресинк ТИХИЙ —
//     тостов не показывает (D-09).
//   * тост обрыва (D-14) показывается не раньше RECONNECT_TOAST_DELAY_MS после обрыва;
//     быстрое переподключение тоста не даёт. Таймер отложенного тоста отменяется на
//     onopen и в activeCleanup (иначе призрачный тост восстановленного соединения).
//     Итого инварианты: не больше одного, не меньше одного, не раньше 5 с, таймер отменяем.
//   * D-15: `online` и `visibilitychange` (visible) сбрасывают backoff и открывают сокет
//     сразу; при открытом сокете шлют ping и при молчании PROBE_TIMEOUT_MS рвут его.
// NOTE: 'request_status_changed' — NOT 'request_updated' (06-CONTEXT sync).

import { isTauri } from './transport.svelte';

/**
 * Структурная копия `WsEvent` из bindings-phase6 (пакет не импортирует bindings).
 * Дрейф ловит `ui/src/packages-compat.ts` (взаимная присваиваемость, svelte-check).
 */
export type WsEvent =
  | { type: 'new_request'; requestId: number; requestType: string; requesterName: string }
  | {
      type: 'request_status_changed';
      requestId: number;
      newStatus: string;
      requestedByUserId: number;
      participantUserIds: number[];
    }
  | { type: 'printer_alert'; printerId: number; printerName: string; alertType: string }
  | { type: 'number_space_changed'; contexts: string[] }
  | { type: 'entities_changed'; placeIds: number[]; deviceIds: number[]; groupIds: number[] }
  | { type: 'employee_scope_changed' }
  | {
      type: 'attachments_changed';
      parentKind: string;
      parentId: number;
      requestedByUserId: number | null;
      participantUserIds: number[];
    }
  | {
      type: 'chat_message';
      requestId: number;
      messageId: number;
      actorUserId: number | null;
      actorName: string | null;
      requestedByUserId: number | null;
      participantUserIds: number[];
    }
  | { type: 'chat_read_changed'; userId: number };

/** Служебные кадры сервера (D-03) — не входят в WsEvent. */
type WsControlFrame =
  | { type: 'hello'; protocol: number; pingIntervalSecs: number; idleTimeoutSecs: number }
  | { type: 'resync_required' }
  | { type: 'pong' };

/** Задержка показа тоста обрыва (D-14). */
export const RECONNECT_TOAST_DELAY_MS = 5000;
/** Окно схлопывания ресинков (D-08). */
export const RESYNC_DEBOUNCE_MS = 500;
/** Сколько ждать любой кадр после ping, прежде чем считать сокет мёртвым (D-15). */
export const PROBE_TIMEOUT_MS = 5000;

type WsEventHandler = (event: WsEvent) => void;

let handlers: WsEventHandler[] = [];
let ws: WebSocket | null = null;
let reconnectDelay = 1000;
let reconnecting = false;

// Plan 12-17: refcounted singleton state. Multiple onMount call sites
// (EmployeeLayout, RequestsPage, PrintersPage) each call connectWs() — without
// a refcount each call opened its OWN WebSocket/listen subscription, so a
// single backend event fanned out into N toasts (GAP-12-10). `refCount` is the
// source of truth for idempotency (NOT `ws !== null` — the browser branch nulls
// `ws` on every reconnect cycle, so that check is unreliable across retries).
// `activeCleanup` replaces the old single-shot `disconnectFn` and is the one
// real teardown (Tauri unlisten / browser ws.close) shared by every consumer.
let refCount = 0;
let activeCleanup: (() => void) | null = null;
// Поколение соединения: растёт при каждом сносе (WR-02), см. teardown().
let epoch = 0;

// WR-04 ревью 41.7: висящий таймер переподключения. Без хранения его нельзя было
// отменить: release()/disconnectWs() в окне backoff закрывали только `ws`, а
// таймер позже открывал сокет, который уже никто не держит; следующий первый
// потребитель открывал второй — события дублировались (класс GAP-12-10).
let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
// D-14: таймер отложенного тоста обрыва; отменяется на onopen и в activeCleanup.
let reconnectToastTimer: ReturnType<typeof setTimeout> | null = null;
// D-15: таймер пробы живости сокета.
let probeTimer: ReturnType<typeof setTimeout> | null = null;
// D-08: дебаунс ресинка.
let resyncTimer: ReturnType<typeof setTimeout> | null = null;
let resyncPending = false;
let resyncHandlers: Array<() => void> = [];

export function onWsEvent(handler: WsEventHandler): () => void {
  handlers.push(handler);
  return () => {
    handlers = handlers.filter((h) => h !== handler);
  };
}

function dispatch(event: WsEvent): void {
  handlers.forEach((h) => h(event));
}

/** Подписка на «перечитай всё» (D-08): потеря событий или повторное открытие. */
export function onWsResync(handler: () => void): () => void {
  resyncHandlers.push(handler);
  return () => {
    resyncHandlers = resyncHandlers.filter((h) => h !== handler);
  };
}

function runResync(): void {
  for (const h of resyncHandlers) {
    try {
      h();
    } catch (e) {
      console.error('[ws] ошибка обработчика ресинка', e);
    }
  }
}

// Первый вызов выполняется сразу, повторные в окне схлопываются в один хвостовой.
// Ресинк тихий: никаких тостов (D-09).
function resyncAll(): void {
  if (resyncTimer !== null) {
    resyncPending = true;
    return;
  }
  runResync();
  resyncTimer = setTimeout(() => {
    resyncTimer = null;
    if (resyncPending) {
      resyncPending = false;
      resyncAll();
    }
  }, RESYNC_DEBOUNCE_MS);
}

function clearProbe(): void {
  if (probeTimer !== null) {
    clearTimeout(probeTimer);
    probeTimer = null;
  }
}

// id залипающего тоста обрыва — нужен, чтобы снять его на успешном `onopen`.
let reconnectToastId: string | null = null;

// Импорт ленивый ИСТОРИЧЕСКИ («avoid circular deps») — обоснование неверно:
// `stores/toast.svelte.ts` не импортирует вообще ничего, цикл невозможен, и тот
// же специфаер статически импортируют ~50 модулей, так что по сети тут ничего
// не тянется. Оставлен ленивым только чтобы не менять форму модуля, но
// `.catch` больше НЕ глотает молча: невидимая ошибка стоила этой сессии
// лишних витков (отладочная сессия `ws-disconnect-toast-no-show`).
function showReconnectingToast(): void {
  import('@trackly/kit/stores/toast.svelte')
    .then(({ pushToast }) => {
      // sticky: индикация ДЛЯЩЕГОСЯ состояния. Обычный тост живёт 5000 мс, а
      // оба сценария обрыва (кнопка «Остановить сервер», Cmd+Q) пользователь
      // запускает в ДРУГОМ окне — к моменту переключения в LAN-браузер
      // транзиентный тост уже истекал, и D-18 выглядел как «не показывается
      // вообще». Снимается в `ws.onopen` ниже.
      reconnectToastId = pushToast('warning', 'Соединение с сервером потеряно. Переподключение…', {
        sticky: true,
      });
    })
    .catch((e) => {
      // Non-fatal, но молчать нельзя — иначе отладка обрыва слепая.
      console.error('[ws] не удалось показать тост обрыва соединения', e);
    });
}

/** Снять залипающий тост обрыва, если он висит. */
function clearReconnectingToast(): void {
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

function connectBrowser(): void {
  const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
  ws = new WebSocket(`${protocol}//${window.location.host}/api/v1/ws`);

  ws.onmessage = (e) => {
    clearProbe(); // любой кадр доказывает, что сокет жив
    try {
      const msg = JSON.parse(e.data) as WsEvent | WsControlFrame;
      if (msg.type === 'resync_required') {
        resyncAll();
        return;
      }
      if (msg.type === 'hello' || msg.type === 'pong') {
        return;
      }
      dispatch(msg);
    } catch {
      // Malformed message — ignore.
    }
  };

  ws.onopen = () => {
    if (reconnecting) {
      // Successfully reconnected after a failure — эпизод закрыт, залипающую
      // индикацию обрыва снимаем.
      reconnecting = false;
      clearReconnectingToast();
      if (reconnectToastTimer !== null) {
        clearTimeout(reconnectToastTimer);
        reconnectToastTimer = null;
      }
      resyncAll();
    }
    reconnectDelay = 1000; // Reset backoff on success.
  };

  ws.onclose = handleLost;

  ws.onerror = () => {
    // onclose fires after onerror; backoff handled there.
  };
}

// Потеря соединения (onclose или проба живости): тост обрыва по таймеру + backoff.
function handleLost(): void {
  ws = null;
  // Plan 12-17: once the last consumer has released the singleton,
  // `activeCleanup` (set below) nulls the `onclose` handler before
  // calling `ws.close()` — so in practice this branch only runs while
  // refCount > 0. The explicit guard is defence-in-depth in case a stray
  // close event fires in between (e.g. a server-initiated close raced with
  // release()) — it must NOT resurrect a connection nobody asked for.
  if (refCount <= 0) {
    return;
  }
  // Show the "reconnecting" toast at most once per disconnection episode.
  // `reconnecting` stays true across the whole backoff sequence and is only
  // reset on a successful onopen, so a failing connection (e.g. an untrusted
  // self-signed wss:// cert that closes every ~1s) no longer spams a toast on
  // every retry. See debug session ui-ws-toast-reports-flicker (Bug A).
  // D-14: показ отложен — быстрое переподключение тоста не даёт.
  if (!reconnecting) {
    reconnecting = true;
    reconnectToastTimer = setTimeout(() => {
      reconnectToastTimer = null;
      showReconnectingToast();
    }, RECONNECT_TOAST_DELAY_MS);
  }
  const delay = reconnectDelay;
  reconnectDelay = Math.min(reconnectDelay * 2, 30000);
  reconnectTimer = setTimeout(() => {
    reconnectTimer = null;
    if (refCount <= 0) {
      return;
    }
    connectBrowser();
  }, delay);
}

// D-15: открыть соединение немедленно, сбросив backoff.
function connectNow(): void {
  if (reconnectTimer !== null) {
    clearTimeout(reconnectTimer);
    reconnectTimer = null;
  }
  reconnectDelay = 1000;
  if (refCount > 0 && ws === null) {
    connectBrowser();
  }
}

// D-15: online/visibilitychange. Нет сокета — открыть сразу; сокет OPEN — проверить
// живость: ping и разрыв, если за PROBE_TIMEOUT_MS не пришло ни одного кадра.
function probeOrReconnect(): void {
  if (refCount <= 0) {
    return;
  }
  if (ws === null) {
    connectNow();
    return;
  }
  if (ws.readyState !== 1 || probeTimer !== null) {
    return; // CONNECTING/CLOSING или проба уже идёт
  }
  try {
    ws.send('{"type":"ping"}');
  } catch {
    // send на полумёртвом сокете может бросить — проба всё равно истечёт.
  }
  probeTimer = setTimeout(() => {
    probeTimer = null;
    const dead = ws;
    if (dead) {
      // Закрытие полуоткрытого сокета браузером может идти долго — onclose не ждём.
      dead.onclose = null;
      dead.onmessage = null;
      dead.onopen = null;
      try {
        dead.close();
      } catch {
        // уже закрыт
      }
    }
    handleLost();
    connectNow();
  }, PROBE_TIMEOUT_MS);
}

function onOnline(): void {
  probeOrReconnect();
}

function onVisibility(): void {
  if (document.visibilityState === 'visible') {
    probeOrReconnect();
  }
}

// Plan 12-17: refcounted singleton. Public contract unchanged — every caller
// still gets back a release function and calls it on teardown — but only the
// FIRST concurrent caller (refCount 0→1) actually opens a connection
// (Tauri listen() or browser WebSocket). Each subsequent caller shares that
// same connection and gets a release() that just decrements the count.
// The real close()/unlisten() only runs when the LAST consumer releases
// (refCount 1→0), which is what fixes GAP-12-10 (duplicate toasts from N
// independent sockets all dispatching the same backend event).
export async function connectWs(): Promise<() => void> {
  refCount += 1;

  if (refCount === 1) {
    const myEpoch = epoch;
    // First consumer: establish the real connection.
    if (isTauri) {
      // Tauri path: native events, no WebSocket needed.
      try {
        const { listen } = await import('@tauri-apps/api/event');
        const unlistenEvent = await listen<WsEvent>('trackly-event', (e) => {
          dispatch(e.payload);
        });
        // D-08: мост десктопа сообщает о потере событий (Lagged) именем trackly-resync
        // (desktop_event_bridge::RESYNC_EVENT).
        const unlistenResync = await listen('trackly-resync', () => {
          resyncAll();
        });
        const unlisten = () => {
          unlistenEvent();
          unlistenResync();
        };
        if (epoch !== myEpoch) {
          // WR-02: пока listen() ждал, соединение уже снесли (последний потребитель
          // сделал release()/disconnectWs()). Сохранять `unlisten` некуда — снимаем
          // подписки сразу, иначе они живут вечно, а следующий connectWs() даёт дубль.
          unlisten();
        } else {
          activeCleanup = unlisten;
        }
      } catch (e) {
        // FE-WR-09: a rejected connect hands the caller no release function,
        // so the count taken above must be given back here — otherwise it
        // stays > 0 forever and a later first consumer never reconnects.
        // Если эпоха сменилась, счётчик уже обнулён сносом (и мог быть взят заново
        // новым первым потребителем) — чужой счёт не трогаем.
        if (epoch === myEpoch) refCount = Math.max(0, refCount - 1);
        throw e;
      }
    } else {
      // Browser path.
      connectBrowser();
      window.addEventListener('online', onOnline);
      document.addEventListener('visibilitychange', onVisibility);
      activeCleanup = () => {
        // Иначе утечка слушателей на каждый цикл connect/release (класс GAP-12-10).
        window.removeEventListener('online', onOnline);
        document.removeEventListener('visibilitychange', onVisibility);
        clearProbe();
        if (reconnectTimer !== null) {
          clearTimeout(reconnectTimer);
          reconnectTimer = null;
        }
        // D-14: отложенный тост уже снесённого соединения показывать нельзя.
        if (reconnectToastTimer !== null) {
          clearTimeout(reconnectToastTimer);
          reconnectToastTimer = null;
        }
        if (ws) {
          ws.onclose = null; // Prevent reconnect loop on intentional close.
          ws.close();
          ws = null;
        }
        // Reset reconnect state so a future first-consumer connectWs() call
        // starts a fresh backoff/toast cycle instead of inheriting stale state.
        // Залипающий тост тоже снимаем: намеренное отключение последнего
        // потребителя — не «потеря связи», висеть индикации не за чем.
        reconnecting = false;
        reconnectDelay = 1000;
        clearReconnectingToast();
      };
    }
  }

  let released = false;
  return () => {
    if (released) {
      // Idempotent: a consumer calling its release twice must not double-decrement.
      return;
    }
    released = true;
    refCount = Math.max(0, refCount - 1);
    if (refCount === 0) teardown();
  };
}

export function disconnectWs(): void {
  refCount = 0;
  teardown();
}

/**
 * Снос соединения. Всегда сдвигает `epoch`: connectWs(), ещё ждущий `listen()`, по
 * возвращении видит, что его эпоха устарела, и снимает свою подписку сам (WR-02).
 */
function teardown(): void {
  epoch += 1;
  if (resyncTimer !== null) {
    clearTimeout(resyncTimer);
    resyncTimer = null;
  }
  resyncPending = false;
  if (activeCleanup) {
    activeCleanup();
    activeCleanup = null;
  }
}
