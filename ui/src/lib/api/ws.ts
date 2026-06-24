// Plan 06-04: Dual-transport WebSocket client.
// Plan 12-17 (GAP-12-10): refcounted singleton — see connectWs() below.
//
// Browser path: native WebSocket → /api/v1/ws, exponential backoff reconnect.
// Tauri path: @tauri-apps/api/event listen('trackly-event') — no WS server needed.
//
// WsEvent union is synchronised with Rust dto/printer.rs WsEvent enum:
//   { type: 'new_request' | 'request_status_changed' | 'printer_alert' }
// NOTE: 'request_status_changed' — NOT 'request_updated' (06-CONTEXT sync).

import type { WsEvent } from '../../bindings-phase6';

type WsEventHandler = (event: WsEvent) => void;

const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

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

export function onWsEvent(handler: WsEventHandler): () => void {
  handlers.push(handler);
  return () => {
    handlers = handlers.filter((h) => h !== handler);
  };
}

function dispatch(event: WsEvent): void {
  handlers.forEach((h) => h(event));
}

function showReconnectingToast(): void {
  // Import lazily to avoid circular deps; toast is not critical
  import('$lib/stores/toast.svelte')
    .then(({ pushToast }) => {
      pushToast('warning', 'Соединение с сервером потеряно. Переподключение…');
    })
    .catch(() => {
      // Non-fatal if toast fails.
    });
}

function connectBrowser(): void {
  const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
  ws = new WebSocket(`${protocol}//${window.location.host}/api/v1/ws`);

  ws.onmessage = (e) => {
    try {
      dispatch(JSON.parse(e.data) as WsEvent);
    } catch {
      // Malformed message — ignore.
    }
  };

  ws.onopen = () => {
    if (reconnecting) {
      // Successfully reconnected after a failure.
      reconnecting = false;
    }
    reconnectDelay = 1000; // Reset backoff on success.
  };

  ws.onclose = () => {
    ws = null;
    // Plan 12-17: once the last consumer has released the singleton,
    // `activeCleanup` (set below) nulls this very `onclose` handler before
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
    if (!reconnecting) {
      reconnecting = true;
      showReconnectingToast();
    }
    const delay = reconnectDelay;
    reconnectDelay = Math.min(reconnectDelay * 2, 30000);
    setTimeout(() => {
      connectBrowser();
    }, delay);
  };

  ws.onerror = () => {
    // onclose fires after onerror; backoff handled there.
  };
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
    // First consumer: establish the real connection.
    if (isTauri) {
      // Tauri path: native events, no WebSocket needed.
      const { listen } = await import('@tauri-apps/api/event');
      const unlisten = await listen<WsEvent>('trackly-event', (e) => {
        dispatch(e.payload);
      });
      activeCleanup = unlisten;
    } else {
      // Browser path.
      connectBrowser();
      activeCleanup = () => {
        if (ws) {
          ws.onclose = null; // Prevent reconnect loop on intentional close.
          ws.close();
          ws = null;
        }
        // Reset reconnect state so a future first-consumer connectWs() call
        // starts a fresh backoff/toast cycle instead of inheriting stale state.
        reconnecting = false;
        reconnectDelay = 1000;
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
    if (refCount === 0 && activeCleanup) {
      activeCleanup();
      activeCleanup = null;
    }
  };
}

export function disconnectWs(): void {
  refCount = 0;
  if (activeCleanup) {
    activeCleanup();
    activeCleanup = null;
  }
}
