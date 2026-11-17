// Открытие внешней ссылки из текста чата (UI-SPEC §7.2, T-41.5-51).
// В десктопе webview не должен уходить со страницы: клик перехватывается и адрес
// открывается во внешнем браузере. В LAN-браузере работает обычная ссылка target=_blank.

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
    const { open } = await import('@tauri-apps/plugin-shell');
    await open(href);
  })();
}
