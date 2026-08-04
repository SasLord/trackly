/**
 * Phase 33 (D-05/C-01): postMessage-мост между приложением и превью-iframe.
 *
 * Iframe рендерится с `sandbox="allow-scripts"` без `allow-same-origin`
 * (D-05) — у документа opaque origin, поэтому `event.origin` входящих
 * сообщений всегда равен строке `"null"` и не несёт различающей информации
 * (RESEARCH.md Pitfall 2). Единственный надёжный способ убедиться, что
 * сообщение пришло именно от НАШЕГО превью-iframe — сравнение identity
 * `event.source` с `iframeEl.contentWindow`.
 */

export function attachBridge(
  iframeEl: HTMLIFrameElement,
  onMsg: (data: unknown) => void,
): () => void {
  function handler(e: MessageEvent) {
    if (e.source !== iframeEl.contentWindow) return;
    onMsg(e.data);
  }

  window.addEventListener('message', handler);

  return () => {
    window.removeEventListener('message', handler);
  };
}
