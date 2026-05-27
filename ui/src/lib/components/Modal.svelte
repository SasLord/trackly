<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    open: boolean;
    title: string;
    size?: 'md' | 'wide';
    onClose: () => void;
    children?: Snippet;
    footer?: Snippet;
  }

  const { open, title, size = 'md', onClose, children, footer }: Props = $props();

  const titleId = `modal-title-${Math.random().toString(36).slice(2)}`;

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onClose();
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) onClose();
  }
</script>

<svelte:window onkeydown={open ? handleKeydown : undefined} />

{#if open}
  <div
    class="modal-backdrop"
    onclick={handleBackdropClick}
    onkeydown={handleKeydown}
    aria-modal="true"
    role="dialog"
    aria-labelledby={titleId}
    tabindex="-1"
  >
    <div class="modal-container modal-{size}">
      <header class="modal-header">
        <h2 id={titleId} class="modal-title">{title}</h2>
        <button type="button" class="modal-close" onclick={onClose} aria-label="Закрыть">×</button>
      </header>
      <div class="modal-body">
        {@render children?.()}
      </div>
      {#if footer}
        <footer class="modal-footer">
          {@render footer()}
        </footer>
      {/if}
    </div>
  </div>
{/if}

<svelte:head>
  {#if open}
    <style>
      body {
        overflow: hidden;
      }
    </style>
  {/if}
</svelte:head>

<style lang="scss">
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.4);
    backdrop-filter: blur(2px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 500;

    :global([data-theme='dark']) & {
      background: rgba(0, 0, 0, 0.6);
    }
  }

  .modal-container {
    background: var(--color-surface-raised);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-elev-2);
    display: flex;
    flex-direction: column;
    max-height: calc(100vh - 64px);
    animation: modal-in 150ms ease-out;
  }

  .modal-md {
    width: var(--modal-max-width);
    max-width: var(--modal-max-width);
  }
  .modal-wide {
    width: var(--modal-max-width-wide);
    max-width: var(--modal-max-width-wide);
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-md) var(--space-lg);
    border-bottom: 1px solid var(--color-border);
    flex-shrink: 0;
  }

  .modal-title {
    margin: 0;
    font-size: var(--font-size-heading);
    font-weight: var(--font-weight-semibold);
    line-height: var(--line-height-heading);
    color: var(--color-text-primary);
  }

  .modal-close {
    background: transparent;
    border: none;
    cursor: pointer;
    color: var(--color-text-secondary);
    font-size: 20px;
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-sm);
    padding: 0;
    line-height: 1;

    &:hover {
      background: var(--color-surface);
      color: var(--color-text-primary);
    }
    &:focus-visible {
      box-shadow: 0 0 0 3px var(--color-accent-focus);
      outline: none;
    }
  }

  .modal-body {
    padding: var(--space-lg);
    overflow-y: auto;
    overflow-x: hidden; // prevent horizontal scroll from long unbreakable strings
    flex: 1;
    // Ensure text inside modal body always wraps.
    overflow-wrap: anywhere;
    word-break: break-word;
  }

  .modal-footer {
    padding: var(--space-md) var(--space-lg);
    border-top: 1px solid var(--color-border);
    display: flex;
    justify-content: flex-end;
    gap: var(--space-sm);
    flex-shrink: 0;
  }

  @keyframes modal-in {
    from {
      opacity: 0;
      transform: scale(0.98);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }
</style>
