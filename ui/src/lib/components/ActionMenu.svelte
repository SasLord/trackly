<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    label?: string;
    /** 'default' — текущий бордер-триггер (36×36, используется в DevicesPage
     *  «Импорт и экспорт» — НЕ трогать). 'ghost-sm' — без бордера, 28px,
     *  ghost-стиль как Button variant="ghost" size="sm" (quick 260820-rdj). */
    variant?: 'default' | 'ghost-sm';
    children: Snippet;
  }

  const { label = 'Действия', variant = 'default', children }: Props = $props();

  let open = $state(false);
  let rootEl = $state<HTMLElement | null>(null);
  let triggerEl = $state<HTMLButtonElement | null>(null);
  let panelEl = $state<HTMLElement | null>(null);

  function menuItems(): HTMLElement[] {
    return panelEl ? Array.from(panelEl.querySelectorAll<HTMLElement>('[role="menuitem"]')) : [];
  }

  function close(returnFocus = false) {
    open = false;
    if (returnFocus) triggerEl?.focus();
  }

  // Move focus to the first menu item whenever the panel opens.
  $effect(() => {
    if (open && panelEl) menuItems()[0]?.focus();
  });

  $effect(() => {
    function onDown(e: MouseEvent) {
      // Outside pointer click — close without forcing focus back to the
      // trigger; focus should follow wherever the user clicked.
      if (open && rootEl && !rootEl.contains(e.target as Node)) open = false;
    }
    function onKey(e: KeyboardEvent) {
      if (e.key === 'Escape') close(true);
    }
    document.addEventListener('mousedown', onDown);
    document.addEventListener('keydown', onKey);
    return () => {
      document.removeEventListener('mousedown', onDown);
      document.removeEventListener('keydown', onKey);
    };
  });

  function onTriggerKeydown(e: KeyboardEvent) {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      open = true;
    }
  }

  function onPanelKeydown(e: KeyboardEvent) {
    const its = menuItems();
    if (its.length === 0) return;
    const idx = its.indexOf(document.activeElement as HTMLElement);
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      its[(idx + 1) % its.length]?.focus();
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      its[(idx - 1 + its.length) % its.length]?.focus();
    } else if (e.key === 'Home') {
      e.preventDefault();
      its[0]?.focus();
    } else if (e.key === 'End') {
      e.preventDefault();
      its[its.length - 1]?.focus();
    } else if (e.key === 'Escape') {
      e.preventDefault();
      close(true);
    }
  }
</script>

<div class="action-menu" bind:this={rootEl}>
  <button
    type="button"
    class="action-menu-trigger"
    class:action-menu-trigger--ghost-sm={variant === 'ghost-sm'}
    aria-haspopup="menu"
    aria-expanded={open}
    aria-label={label}
    bind:this={triggerEl}
    onclick={() => (open = !open)}
    onkeydown={onTriggerKeydown}
  >
    <svg width="18" height="18" viewBox="0 0 18 18" aria-hidden="true">
      <circle cx="9" cy="3.5" r="1.5" fill="currentColor" />
      <circle cx="9" cy="9" r="1.5" fill="currentColor" />
      <circle cx="9" cy="14.5" r="1.5" fill="currentColor" />
    </svg>
  </button>
  {#if open}
    <div
      class="action-menu-panel"
      role="menu"
      tabindex="-1"
      bind:this={panelEl}
      onkeydown={onPanelKeydown}
      onclick={() => close(true)}
    >
      {@render children()}
    </div>
  {/if}
</div>

<style lang="scss">
  .action-menu {
    position: relative;
    display: inline-flex;
  }

  .action-menu-trigger {
    width: 36px;
    height: 36px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--tr-radius-sm);
    background: transparent;
    border: 1px solid var(--tr-border-strong);
    color: var(--tr-text-secondary);
    cursor: pointer;

    &:hover {
      background: var(--tr-row-hover);
    }
    &:focus-visible {
      outline: none;
      box-shadow: 0 0 0 3px var(--tr-focus-ring);
    }
  }

  .action-menu-trigger--ghost-sm {
    width: 28px;
    height: 28px;
    border: none;
    background: transparent;
    color: var(--tr-text-primary);

    &:hover {
      background: var(--tr-surface-sunken);
    }
  }

  .action-menu-panel {
    position: absolute;
    top: calc(100% + 4px);
    right: 0;
    z-index: 1000;
    min-width: 180px;
    display: flex;
    flex-direction: column;
    padding: 4px;
    background: var(--tr-surface-raised);
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-md);
    box-shadow: var(--tr-elev-2);
  }

  .action-menu-panel :global(button) {
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
</style>
