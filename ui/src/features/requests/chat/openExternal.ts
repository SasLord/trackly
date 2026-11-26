// Открытие внешней ссылки из текста чата (UI-SPEC §7.2, T-41.5-51).
// В десктопе webview не должен уходить со страницы: клик перехватывается и адрес
// открывается во внешнем браузере. В LAN-браузере работает обычная ссылка target=_blank.

import { pushToast } from '@trackly/kit/stores/toast.svelte';

const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

export function onExternalLinkClick(event: MouseEvent, href: string): void {
  if (!isTauri) return;
  // Только http(s): linkify уже отдаёт только такие адреса, проверка — второй рубеж.
  if (!/^https?:\/\//i.test(href)) {
    event.preventDefault();
    return;
  }
  event.preventDefault();
  void (async () => {
    try {
      const { open } = await import('@tauri-apps/plugin-shell');
      await open(href);
    } catch {
      // Адрес в тосте и в консоли не показываем: в ссылке могут быть токены (T-41.5-74).
      pushToast('error', 'Не удалось открыть ссылку');
    }
  })();
}
