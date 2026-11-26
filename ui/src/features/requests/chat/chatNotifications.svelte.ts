// .svelte.ts extension REQUIRED — Svelte 5 runes.
//
// Фаза 41.5, план 11 (D-21), план 25 (D-G1/D-G2): тост и системное уведомление о чужом
// сообщении для всех ролей; заглушённая заявка молчит (счётчики при этом растут).
// Текст сообщения в WS-событии отсутствует и в уведомление не попадает.
import { onWsEvent } from '@trackly/transport/ws';
import { registerUserScopedReset } from '@trackly/kit/stores/userScoped.svelte';
import { authStore } from '$lib/stores/auth.svelte';
import { pushToast, removeToast, toastStore } from '@trackly/kit/stores/toast.svelte';
import type { WsEvent } from '../../../bindings-phase6';
import {
  appHasFocus,
  armPermissionPrompt,
  showSystemNotification,
} from '$lib/platform/systemNotify';
import { openRequestTarget, requestTargetHref } from '../requestTarget';
import { isMuted, startUnreadRealtime, unreadStore } from './unreadStore.svelte';

const toastByRequest = new Map<number, string>();

registerUserScopedReset(() => {
  toastByRequest.clear();
});

/** Открытие чата заявки любым путём снимает её тост (R3-5). Тост показывается как прежде (D-G1). */
export function dismissChatToast(requestId: number): void {
  const id = toastByRequest.get(requestId);
  if (id === undefined) return;
  removeToast(id);
  toastByRequest.delete(requestId);
}

export function handleChatMessageEvent(ev: WsEvent): void {
  if (ev.type !== 'chat_message') return;
  const me = authStore.user;
  if (!me) return;
  if (ev.actorUserId !== null && ev.actorUserId === me.id) return;
  if (unreadStore.activeChatRequestId === ev.requestId) return;
  if (isMuted(ev.requestId)) return;

  const href = requestTargetHref(ev.requestId, 'chat');
  const text = `Новое сообщение в заявке №${ev.requestId} · ${ev.actorName ?? 'Участник'}`;

  const prev = toastByRequest.get(ev.requestId);
  if (!prev || !toastStore.items.some((t) => t.id === prev)) {
    const id = pushToast('info', text, { link: { label: 'Открыть чат', href } });
    toastByRequest.set(ev.requestId, id);
  }
  // D-G1: оба канала. Уведомление ОС — когда окно не в фокусе (не document.hidden).
  if (!appHasFocus()) {
    void showSystemNotification('Trackly', text, {
      activate: () => openRequestTarget(ev.requestId, 'chat'),
      href: requestTargetHref(ev.requestId, 'chat'),
      label: `Открыть заявку №${ev.requestId}`,
    });
  }
}

export function startChatRealtime(): () => void {
  const stopUnread = startUnreadRealtime();
  const off = onWsEvent(handleChatMessageEvent);
  const disarm = armPermissionPrompt();
  return () => {
    disarm();
    off();
    stopUnread();
  };
}
