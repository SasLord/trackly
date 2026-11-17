// .svelte.ts extension REQUIRED — Svelte 5 runes.
//
// Фаза 41.5, план 11 (D-21): тост и системное уведомление о чужом сообщении.
// Текст сообщения в WS-событии отсутствует и в уведомление не попадает.
import { onWsEvent } from '$lib/api/ws';
import { registerUserScopedReset } from '$lib/stores/userScoped.svelte';
import { authStore } from '$lib/stores/auth.svelte';
import { pushToast, toastStore } from '$lib/stores/toast.svelte';
import type { WsEvent } from '../../../bindings-phase6';
import { startUnreadRealtime, unreadStore } from './unreadStore.svelte';

const toastByRequest = new Map<number, string>();

registerUserScopedReset(() => {
  toastByRequest.clear();
});

export function handleChatMessageEvent(ev: WsEvent): void {
  if (ev.type !== 'chat_message') return;
  const me = authStore.user;
  if (!me) return;
  if (ev.actorUserId !== null && ev.actorUserId === me.id) return;
  if (unreadStore.activeChatRequestId === ev.requestId) return;

  const href = `#/requests?id=${Number(ev.requestId)}&tab=chat`;
  const text = `Новое сообщение в заявке №${ev.requestId} · ${ev.actorName ?? 'Участник'}`;

  const canNotify =
    typeof window !== 'undefined' &&
    'Notification' in window &&
    window.isSecureContext &&
    Notification.permission === 'granted';

  if (me.role === 'employee' && document.hidden && canNotify) {
    const n = new Notification('Trackly', { body: text });
    n.onclick = () => {
      window.focus();
      location.hash = href;
    };
    return;
  }

  const prev = toastByRequest.get(ev.requestId);
  if (prev && toastStore.items.some((t) => t.id === prev)) return;
  const id = pushToast('info', text, { link: { label: 'Открыть чат', href } });
  toastByRequest.set(ev.requestId, id);
}

export function startChatRealtime(): () => void {
  const stopUnread = startUnreadRealtime();
  const off = onWsEvent(handleChatMessageEvent);
  return () => {
    off();
    stopUnread();
  };
}
