// .svelte.ts extension REQUIRED — Svelte 5 runes.
//
// Фаза 41.5, план 11 (D-19, D-24): счётчики непрочитанного. Числа берутся ТОЛЬКО
// с сервера (chat_unread_summary); клиент ничего не считает локально, а лишь
// показывает и перечитывает по WS. WS-соединение стор не открывает: его держат
// layout'ы (refcount-singleton в ws.ts).
import { onWsEvent } from '$lib/api/ws';
import { registerUserScopedReset } from '$lib/stores/userScoped.svelte';
import { chat } from './api';

export const unreadStore = $state({
  byRequest: {} as Record<number, number>,
  requestsWithUnread: 0,
  loaded: false,
  activeChatRequestId: null as number | null,
});

let token = 0;
let timer: ReturnType<typeof setTimeout> | null = null;

registerUserScopedReset(() => {
  token += 1;
  if (timer !== null) {
    clearTimeout(timer);
    timer = null;
  }
  unreadStore.byRequest = {};
  unreadStore.requestsWithUnread = 0;
  unreadStore.loaded = false;
  unreadStore.activeChatRequestId = null;
});

export function unreadFor(requestId: number): number {
  return unreadStore.byRequest[requestId] ?? 0;
}

function recount(): void {
  unreadStore.requestsWithUnread = Object.values(unreadStore.byRequest).filter((n) => n > 0).length;
}

/** Заявка, чат которой открыт: счётчик всегда 0 (D-18). */
export function setActiveChat(requestId: number | null): void {
  unreadStore.activeChatRequestId = requestId;
  if (requestId !== null && unreadStore.byRequest[requestId]) {
    const next = { ...unreadStore.byRequest };
    delete next[requestId];
    unreadStore.byRequest = next;
    recount();
  }
}

export async function refreshUnread(): Promise<void> {
  const mine = ++token;
  try {
    const res = await chat.unreadSummary();
    if (mine !== token) return;
    // Форма ответа: при пропавшем маршруте приходит HTML-строка (spa_fallback).
    if (
      !res ||
      !Array.isArray(res.items) ||
      !res.items.every((i) => typeof i?.requestId === 'number' && typeof i?.count === 'number')
    ) {
      return;
    }
    const map: Record<number, number> = {};
    for (const i of res.items) {
      if (i.count > 0) map[i.requestId] = i.count;
    }
    if (unreadStore.activeChatRequestId !== null) {
      delete map[unreadStore.activeChatRequestId];
    }
    unreadStore.byRequest = map;
    recount();
    unreadStore.loaded = true;
  } catch {
    // Сбой сети не фатален: прежние числа остаются.
  }
}

function scheduleRefresh(): void {
  if (timer !== null) clearTimeout(timer);
  timer = setTimeout(() => {
    timer = null;
    void refreshUnread();
  }, 150);
}

export function startUnreadRealtime(): () => void {
  void refreshUnread();
  const off = onWsEvent((ev) => {
    switch (ev.type) {
      case 'chat_message':
        if (unreadStore.activeChatRequestId === ev.requestId) return;
        scheduleRefresh();
        break;
      case 'request_status_changed':
      case 'attachments_changed':
      case 'new_request':
      case 'chat_read_changed':
        scheduleRefresh();
        break;
    }
  });
  return () => {
    off();
    if (timer !== null) {
      clearTimeout(timer);
      timer = null;
    }
  };
}
