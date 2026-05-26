// .svelte.ts extension REQUIRED — Svelte 5 runes.

export type ToastKind = 'success' | 'error' | 'info' | 'warning';

export interface ToastItem {
  id: string;
  kind: ToastKind;
  message: string;
}

const TTL: Record<ToastKind, number> = {
  error: 6000,
  warning: 5000,
  success: 4000,
  info: 4000,
};

const MAX_TOASTS = 10;

export const toastStore = $state({ items: [] as ToastItem[] });

export function pushToast(kind: ToastKind, message: string): void {
  const id = crypto.randomUUID();
  // Enforce max toast limit — drop oldest if over limit.
  if (toastStore.items.length >= MAX_TOASTS) {
    toastStore.items = toastStore.items.slice(toastStore.items.length - MAX_TOASTS + 1);
  }
  toastStore.items = [...toastStore.items, { id, kind, message }];
  setTimeout(() => {
    toastStore.items = toastStore.items.filter((t) => t.id !== id);
  }, TTL[kind]);
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
