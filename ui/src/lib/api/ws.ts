// Plan 06-04: Dual-transport WebSocket client.
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
let disconnectFn: (() => void) | null = null;

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
    reconnecting = true;
    showReconnectingToast();
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

export async function connectWs(): Promise<() => void> {
  if (isTauri) {
    // Tauri path: native events, no WebSocket needed.
    const { listen } = await import('@tauri-apps/api/event');
    const unlisten = await listen<WsEvent>('trackly-event', (e) => {
      dispatch(e.payload);
    });
    disconnectFn = unlisten;
    return unlisten;
  }

  // Browser path.
  connectBrowser();
  const cleanup = () => {
    if (ws) {
      ws.onclose = null; // Prevent reconnect loop on intentional close.
      ws.close();
      ws = null;
    }
  };
  disconnectFn = cleanup;
  return cleanup;
}

export function disconnectWs(): void {
  if (disconnectFn) {
    disconnectFn();
    disconnectFn = null;
  }
}
