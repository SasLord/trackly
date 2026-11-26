/**
 * Svelte use-action: перемещает DOM-узел в `<body>` (или другой целевой элемент),
 * чтобы он не обрезался контейнером с `overflow: hidden/auto`.
 *
 * Использование:
 *   <div use:portal class="floating-menu">...</div>
 *
 * Принцип: при монтировании узел перемещается из текущего родителя в `target`
 * (по умолчанию `document.body`). При уничтожении компонента узел удаляется.
 */
export function portal(
  node: HTMLElement,
  target: HTMLElement | string = 'body',
): { destroy(): void } {
  let targetEl: HTMLElement | null;

  if (typeof target === 'string') {
    targetEl = document.querySelector(target);
  } else {
    targetEl = target;
  }

  if (targetEl) {
    node.setAttribute('data-tr-portal', '');
    targetEl.appendChild(node);
  }

  return {
    destroy() {
      node.parentNode?.removeChild(node);
    },
  };
}
