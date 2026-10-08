// .svelte.ts extension REQUIRED — Svelte 5 runes.

export type ToastKind = 'success' | 'error' | 'info' | 'warning';

export interface ToastItem {
  id: string;
  kind: ToastKind;
  message: string;
  /** Залипающий тост: без TTL, снимается только `removeToast`/кликом. */
  sticky?: boolean;
}

export interface PushToastOptions {
  /**
   * Не снимать по TTL. Для индикации ДЛЯЩЕГОСЯ состояния — например «соединение
   * с сервером потеряно»: такой тост обязан дожить до возврата пользователя в
   * окно браузера, а не истечь за 5 с (отладочная сессия
   * `ws-disconnect-toast-no-show`). Вызывающий сам снимает его `removeToast(id)`,
   * когда состояние закончилось.
   */
  sticky?: boolean;
}

const TTL: Record<ToastKind, number> = {
  error: 6000,
  warning: 5000,
  success: 4000,
  info: 4000,
};

const MAX_TOASTS = 10;

export const toastStore = $state({ items: [] as ToastItem[] });

/** Возвращает id созданного (или уже существующего залипающего) тоста. */
export function pushToast(kind: ToastKind, message: string, opts?: PushToastOptions): string {
  const sticky = opts?.sticky === true;

  // Залипающие тосты дедуплицируются по тексту. Без этого флапающее соединение
  // (connect/disconnect по кругу) копило бы стопку одинаковых несгораемых
  // тостов — ровно симптом Bug A из сессии `ui-ws-toast-reports-flicker`, но уже
  // без спасительного TTL. С дедупом их не больше одного одновременно.
  if (sticky) {
    const existing = toastStore.items.find((t) => t.sticky && t.message === message);
    if (existing) {
      return existing.id;
    }
  }

  const id = crypto.randomUUID();
  // Лимит очереди: освобождаем место под новый тост, вытесняя САМЫЕ СТАРЫЕ, но
  // ТОЛЬКО НЕ залипающие.
  //
  // Раньше здесь был безусловный `slice(length - MAX_TOASTS + 1)`, который
  // срезал старейший тост независимо от `sticky`. Это воспроизводило GAP-1
  // (сессия `ws-disconnect-toast-no-show`): при обрыве связи залипающий тост
  // «Соединение с сервером потеряно…» ставится ПЕРВЫМ, а каждый следующий
  // падающий API-вызов добавляет свой error-тост (в `ui/src` больше сотни
  // мест `pushToast('error', …)`). Через MAX_TOASTS-1 таких тостов индикатор
  // вытеснялся, а флаг эпизода `reconnecting` в `ws.ts` остаётся `true` до
  // `onopen` — значит `showReconnectingToast()` больше не позовут и индикация
  // обрыва исчезает навсегда, ровно на том экране, который активно сыплет
  // ошибками. Дедуп залипающих (выше) от этого не спасал: он защищает от
  // СТОПКИ одинаковых, а не от вытеснения единственного.
  //
  // Порядок остальных тостов сохраняется (фильтром, не пересборкой), иначе
  // залипающий всплывал бы в начало списка при каждом вытеснении.
  //
  // Если вытеснять нечего (все записи залипающие), очередь растёт — это
  // осознанный компромисс: залипающие дедуплицируются по тексту, поэтому их
  // не больше, чем различных длящихся состояний.
  let toDrop = toastStore.items.length - MAX_TOASTS + 1;
  if (toDrop > 0) {
    const dropped = new Set<string>();
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

export function removeToast(id: string): void {
  toastStore.items = toastStore.items.filter((t) => t.id !== id);
}

// Convenience helpers
export const toast = {
  success: (msg: string) => pushToast('success', msg),
  error: (msg: string) => pushToast('error', msg),
  warning: (msg: string) => pushToast('warning', msg),
  info: (msg: string) => pushToast('info', msg),
};
