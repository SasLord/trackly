// Очередь неотправленных сообщений чата (фаза 43-10; D-11, D-12, DLV-03).
// Живёт только в памяти вкладки и сбрасывается при смене пользователя. Все попытки одного
// сообщения (автоповторы и ручной «Повторить») идут с ОДНИМ clientMsgId — сервер возвращает
// уже созданное сообщение вместо дубля. Черновики вложений стор не трогает.
import { registerUserScopedReset } from '@trackly/kit/stores/userScoped.svelte';
import { isNetworkFailure, newClientId, sendWithRetry } from '@trackly/transport/sendWithRetry';
import { chat } from './api';
import type { ChatMessageDto } from '../../../bindings';

export interface PendingMessage {
  clientMsgId: string;
  requestId: number;
  text: string;
  draftIds: number[];
  status: 'sending' | 'failed';
  createdAtUtc: number;
}

export type SubmitResult =
  | { kind: 'sent'; message: ChatMessageDto }
  | { kind: 'rejected'; code: string | null }
  | { kind: 'error'; error: unknown }
  | { kind: 'failed'; clientMsgId: string };

export interface SubmitOptions {
  sleep?: (_ms: number) => Promise<void>;
}

export const pending = $state({ items: [] as PendingMessage[] });

export function pendingFor(requestId: number): PendingMessage[] {
  return pending.items.filter((p) => p.requestId === requestId);
}

function removeItem(clientMsgId: string) {
  pending.items = pending.items.filter((p) => p.clientMsgId !== clientMsgId);
}

function setStatus(clientMsgId: string, status: 'sending' | 'failed') {
  pending.items = pending.items.map((p) => (p.clientMsgId === clientMsgId ? { ...p, status } : p));
}

async function dispatch(item: PendingMessage, opts?: SubmitOptions): Promise<SubmitResult> {
  const { clientMsgId, requestId, text, draftIds } = item;
  try {
    const outcome = await sendWithRetry(
      () => chat.send(requestId, text, draftIds, clientMsgId),
      opts?.sleep ? { sleep: opts.sleep } : {},
    );
    removeItem(clientMsgId);
    if (outcome.rejectCode !== null || outcome.message === null) {
      return { kind: 'rejected', code: outcome.rejectCode };
    }
    return { kind: 'sent', message: outcome.message };
  } catch (error) {
    if (isNetworkFailure(error)) {
      setStatus(clientMsgId, 'failed');
      return { kind: 'failed', clientMsgId };
    }
    removeItem(clientMsgId);
    return { kind: 'error', error };
  }
}

export async function submitMessage(
  requestId: number,
  text: string,
  draftIds: number[],
  opts?: SubmitOptions,
): Promise<SubmitResult> {
  const item: PendingMessage = {
    clientMsgId: newClientId(),
    requestId,
    text,
    draftIds: [...draftIds],
    status: 'sending',
    createdAtUtc: Date.now(),
  };
  pending.items = [...pending.items, item];
  return dispatch(item, opts);
}

export async function retryMessage(
  clientMsgId: string,
  opts?: SubmitOptions,
): Promise<SubmitResult | null> {
  const item = pending.items.find((p) => p.clientMsgId === clientMsgId);
  if (!item || item.status !== 'failed') return null;
  setStatus(clientMsgId, 'sending');
  return dispatch({ ...item, status: 'sending' }, opts);
}

registerUserScopedReset(() => {
  pending.items = [];
});
