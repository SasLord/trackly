// Фаза 41.5, план 11: API-обёртка чата. Типы — из сгенерированного bindings.ts.
import { apiCall } from '$lib/api/client';
import { uploadAttachment } from '../../attachments/api';
import type { ChatFeedDto, ChatSendOutcomeDto, UnreadSummaryDto } from '../../../bindings';

export const chat = {
  feed: (requestId: number) => apiCall<ChatFeedDto>('chat_feed', { requestId }),
  send: (requestId: number, text: string, draftAttachmentIds: number[]) =>
    apiCall<ChatSendOutcomeDto>('chat_send', { requestId, text, draftAttachmentIds }),
  markRead: (requestId: number) => apiCall<null>('chat_mark_read', { requestId }),
  unreadSummary: () => apiCall<UnreadSummaryDto>('chat_unread_summary', {}),
  discardDrafts: (requestId: number) => apiCall<null>('chat_discard_drafts', { requestId }),
};

/** Загрузка файла-черновика сообщения (привязывается при chat_send). */
export function uploadDraft(requestId: number, file: globalThis.File) {
  return uploadAttachment('message_draft', requestId, file);
}
