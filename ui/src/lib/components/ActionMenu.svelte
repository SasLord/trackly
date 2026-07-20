<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    label?: string;
    children: Snippet;
  }

  const { label = 'Действия', children }: Props = $props();

  let open = $state(false);
  let rootEl = $state<HTMLElement | null>(null);

  $effect(() => {
    function onDown(e: MouseEvent) {
      if (open && rootEl && !rootEl.contains(e.target as Node)) open = false;
    }
    function onKey(e: KeyboardEvent) {
      if (e.key === 'Escape') open = false;
    }
    function onClick(e: MouseEvent) {
      const t = e.target as HTMLElement;
      if (open && t.closest('.action-menu-panel')) open = false;
    }
    document.addEventListener('mousedown', onDown);
    document.addEventListener('keydown', onKey);
    document.addEventListener('click', onClick);
    return () => {
      document.removeEventListener('mousedown', onDown);
      document.removeEventListener('keydown', onKey);
      document.removeEventListener('click', onClick);
    };
  });
</script>

<div class="action-menu" bind:this={rootEl}>
  <button
    type="button"
    class="action-menu-trigger"
    aria-haspopup="menu"
    aria-expanded={open}
    aria-label={label}
    onclick={() => (open = !open)}
  >
    <svg width="18" height="18" viewBox="0 0 18 18" aria-hidden="true">
      <circle cx="9" cy="3.5" r="1.5" fill="currentColor" />
      <circle cx="9" cy="9" r="1.5" fill="currentColor" />
      <circle cx="9" cy="14.5" r="1.5" fill="currentColor" />
    </svg>
  </button>
  {#if open}
    <div class="action-menu-panel" role="menu" tabindex="-1">
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
