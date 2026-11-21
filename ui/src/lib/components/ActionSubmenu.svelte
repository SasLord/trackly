<script lang="ts">
  import { getContext, onDestroy, type Snippet } from 'svelte';
  import { portal } from '$lib/utils/portal';
  import { ACTION_MENU_CONTEXT, type ActionMenuContext } from './actionMenuContext';

  /* Вложенный попап сбоку внутри ActionMenu (D-35). Общий компонент без знания про
   * статусы: варианты передаются снимком разметки (`children`) — кнопки с
   * role="menuitemradio" (aria-checked) или "menuitem". Попап выводится порталом в
   * <body>, поэтому overflow панели его не обрезает; он вне DOM панели, значит её
   * onclick/onPanelKeydown его не затрагивают. Esc внутри попапа закрывает только
   * подменю: решающая защита в документном onKey ActionMenu (defaultPrevented или
   * цель внутри [data-action-submenu]). */

  interface Props {
    label: string;
    disabled?: boolean;
    children: Snippet;
  }

  const { label, disabled = false, children }: Props = $props();

  const menu = getContext<ActionMenuContext | undefined>(ACTION_MENU_CONTEXT);
  const uid = `action-submenu-${Math.random().toString(36).slice(2, 9)}`;
  const CLOSE_DELAY = 150;

  let open = $state(false);
  let triggerEl = $state<HTMLButtonElement | null>(null);
  let popupEl = $state<HTMLElement | null>(null);
  let closeTimer: ReturnType<typeof setTimeout> | null = null;

  function cancelClose() {
    if (closeTimer) {
      clearTimeout(closeTimer);
      closeTimer = null;
    }
  }

  function scheduleClose() {
    cancelClose();
    closeTimer = setTimeout(() => {
      closeTimer = null;
      open = false;
    }, CLOSE_DELAY);
  }

  onDestroy(cancelClose);

  function items(): HTMLElement[] {
    return popupEl
      ? Array.from(
          popupEl.querySelectorAll<HTMLElement>(
            '[role="menuitemradio"]:not([aria-disabled="true"]), [role="menuitem"]:not([aria-disabled="true"])',
          ),
        )
      : [];
  }

  function focusFirstSoon() {
    // попап появляется в DOM после обновления состояния
    requestAnimationFrame(() => items()[0]?.focus());
  }

  function openMenu(focusFirst: boolean) {
    if (disabled) return;
    cancelClose();
    open = true;
    if (focusFirst) focusFirstSoon();
  }

  function closeToTrigger() {
    cancelClose();
    open = false;
    triggerEl?.focus();
  }

  function onTriggerClick(e: MouseEvent) {
    // Клик по панели родителя закрывает всё меню безусловно — гасим.
    e.stopPropagation();
    if (disabled) return;
    if (open) {
      cancelClose();
      open = false;
    } else {
      openMenu(e.detail === 0);
    }
  }

  function onTriggerKeydown(e: KeyboardEvent) {
    if (e.key === 'ArrowRight') {
      e.preventDefault();
      openMenu(true);
    }
  }

  function onPopupKeydown(e: KeyboardEvent) {
    const its = items();
    const idx = its.indexOf(document.activeElement as HTMLElement);
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (its.length) its[(idx + 1) % its.length]?.focus();
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (its.length) its[(idx - 1 + its.length) % its.length]?.focus();
    } else if (e.key === 'Home') {
      e.preventDefault();
      its[0]?.focus();
    } else if (e.key === 'End') {
      e.preventDefault();
      its[its.length - 1]?.focus();
    } else if (e.key === 'ArrowLeft' || e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      closeToTrigger();
    } else if (e.key === 'Tab') {
      menu?.close(true);
    }
  }

  function onPopupClick(e: MouseEvent) {
    const btn = (e.target as HTMLElement | null)?.closest<HTMLElement>('button');
    if (!btn) return;
    if (btn.getAttribute('aria-disabled') === 'true') {
      e.stopPropagation();
      return;
    }
    // обработчик варианта (глубже по дереву) уже отработал
    menu?.close(true);
  }

  /** Позиция: справа от триггера (отступ 2px), верх по верху триггера; у края окна —
   *  влево и вверх; выше окна — max-height и прокрутка. */
  function submenuPosition(node: HTMLElement, anchor: HTMLElement | null) {
    let anchorEl = anchor;
    const gap = 2;
    const edge = 8;

    function reposition() {
      if (!anchorEl) return;
      const rect = anchorEl.getBoundingClientRect();
      node.style.position = 'fixed';
      node.style.maxHeight = `${window.innerHeight - edge * 2}px`;
      const width = node.offsetWidth;
      const height = node.offsetHeight;
      if (rect.right + gap + width > window.innerWidth - edge) {
        node.style.left = 'auto';
        node.style.right = `${window.innerWidth - rect.left + gap}px`;
      } else {
        node.style.left = `${rect.right + gap}px`;
        node.style.right = 'auto';
      }
      let top = rect.top - 4;
      if (top + height > window.innerHeight - edge) top = window.innerHeight - edge - height;
      if (top < edge) top = edge;
      node.style.top = `${top}px`;
      node.style.bottom = 'auto';
    }

    reposition();
    window.addEventListener('scroll', reposition, true);
    window.addEventListener('resize', reposition);
    return {
      update(next: HTMLElement | null) {
        anchorEl = next;
        reposition();
      },
      destroy() {
        window.removeEventListener('scroll', reposition, true);
        window.removeEventListener('resize', reposition);
      },
    };
  }
</script>

<button
  type="button"
  role="menuitem"
  class="action-submenu-trigger"
  aria-haspopup="menu"
  aria-expanded={open}
  aria-controls={open ? uid : undefined}
  {disabled}
  bind:this={triggerEl}
  onclick={onTriggerClick}
  onkeydown={onTriggerKeydown}
  onmouseenter={() => openMenu(false)}
  onmouseleave={scheduleClose}
>
  <span class="action-submenu-label">{label}</span>
  <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true" class="action-submenu-chevron">
    <path
      d="M6 3.5 10.5 8 6 12.5"
      fill="none"
      stroke="currentColor"
      stroke-width="1.6"
      stroke-linecap="round"
      stroke-linejoin="round"
    />
  </svg>
</button>

{#if open}
  <div
    id={uid}
    class="action-submenu-panel"
    role="menu"
    tabindex="-1"
    aria-label={label}
    data-action-submenu
    bind:this={popupEl}
    use:portal
    use:submenuPosition={triggerEl}
    onkeydown={onPopupKeydown}
    onclick={onPopupClick}
    onmouseenter={cancelClose}
    onmouseleave={scheduleClose}
  >
    {@render children()}
  </div>
{/if}

<style lang="scss">
  .action-submenu-trigger {
    justify-content: space-between;
  }

  .action-submenu-label {
    flex: 1;
    text-align: left;
  }

  .action-submenu-chevron {
    flex: none;
    color: var(--tr-text-secondary);
  }

  // Портал в <body>; top/left/right задаёт submenuPosition. z-index на единицу выше
  // панели ActionMenu (1000) и подложки (999).
  .action-submenu-panel {
    position: fixed;
    z-index: 1001;
    min-width: 180px;
    max-width: 420px;
    overflow: auto;
    display: flex;
    flex-direction: column;
    padding: 4px;
    background: var(--tr-surface-raised);
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-md);
    box-shadow: var(--tr-elev-2);
  }

  .action-submenu-panel :global(button) {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 8px 12px;
    text-align: left;
    background: transparent;
    border: none;
    border-radius: var(--tr-radius-sm);
    color: var(--tr-text-primary);
    font-family: inherit;
    font-size: 14px;
    cursor: pointer;

    &:hover {
      background: var(--tr-row-hover);
    }
  }

  // колонка под галочку: пустая у неотмеченных для выравнивания
  .action-submenu-panel :global(button[role='menuitemradio']::before) {
    content: '';
    flex: none;
    width: 16px;
    height: 16px;
  }

  .action-submenu-panel :global(button[role='menuitemradio'][aria-checked='true']::before) {
    content: '\2713';
    line-height: 16px;
    text-align: center;
    font-size: 14px;
  }

  .action-submenu-panel :global(button[aria-disabled='true']) {
    color: var(--tr-text-secondary);
    opacity: 0.6;
    cursor: not-allowed;
  }

  .action-submenu-panel :global(button[aria-disabled='true']:hover) {
    background: transparent;
  }
</style>
