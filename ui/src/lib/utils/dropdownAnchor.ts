/**
 * Svelte use-action: вычисляет fixed-позицию портального дропдауна относительно
 * якоря (`anchorEl`, обычно input автокомплита) и держит её актуальной при
 * скролле/ресайзе.
 *
 * В отличие от `DeviceContextMenu.handleScrollOrResize` (который ЗАКРЫВАЕТ меню
 * при скролле/ресайзе), этот action РЕПОЗИЦИОНИРУЕТ дропдаун — он «следует» за
 * якорем (AUTO-01/D-02). Скролл слушается в capture-фазе, чтобы скролл ЛЮБОГО
 * overflow-контейнера-предка (не только window) триггерил пересчёт координат —
 * актуально для дропдаунов внутри модалок.
 *
 * Использование (вместе с `portal`):
 *   <div use:portal use:dropdownAnchor={{ anchorEl: inputEl }}>...</div>
 */
export interface DropdownAnchorParams {
  /** Элемент-якорь (обычно input), относительно которого позиционируется дропдаун. */
  anchorEl: HTMLElement | null;
  /** Зазор между якорем и дропдауном, px. По умолчанию 4 (соответствует --space-xs). */
  gap?: number;
  /** Максимальная высота дропдауна, px, используется для расчёта флипа вверх. По умолчанию 240. */
  maxHeight?: number;
}

export function dropdownAnchor(
  node: HTMLElement,
  params: DropdownAnchorParams,
): { update(newParams: DropdownAnchorParams): void; destroy(): void } {
  let current = params;

  function reposition() {
    const anchorEl = current.anchorEl;
    if (!anchorEl) return;

    const rect = anchorEl.getBoundingClientRect();
    const gap = current.gap ?? 4;
    const maxHeight = current.maxHeight ?? 240;

    node.style.position = 'fixed';
    node.style.left = `${rect.left}px`;
    node.style.width = `${rect.width}px`;

    const spaceBelow = window.innerHeight - rect.bottom;
    const neededHeight = Math.min(maxHeight, node.scrollHeight || maxHeight);

    if (spaceBelow >= neededHeight) {
      // Достаточно места снизу — раскрываем вниз.
      node.style.top = `${rect.bottom + gap}px`;
      node.style.bottom = 'auto';
    } else {
      // Недостаточно места снизу — флип вверх (D-02).
      node.style.bottom = `${window.innerHeight - rect.top + gap}px`;
      node.style.top = 'auto';
    }
  }

  reposition();
  window.addEventListener('scroll', reposition, true);
  window.addEventListener('resize', reposition);

  return {
    update(newParams: DropdownAnchorParams) {
      current = newParams;
      reposition();
    },
    destroy() {
      window.removeEventListener('scroll', reposition, true);
      window.removeEventListener('resize', reposition);
    },
  };
}
