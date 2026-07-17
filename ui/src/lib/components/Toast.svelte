<script lang="ts">
  import type { ToastKind } from '$lib/stores/toast.svelte';

  interface Props {
    kind: ToastKind;
    message: string;
    onClose: () => void;
  }

  const { kind, message, onClose }: Props = $props();

  const roleMap: Record<ToastKind, string> = {
    info: 'status',
    success: 'status',
    error: 'alert',
    warning: 'alert',
  };
</script>

<div class="toast toast-{kind}" role={roleMap[kind]} aria-live="polite">
  <span class="toast-message">{message}</span>
  <button class="toast-close" onclick={onClose} aria-label="Закрыть уведомление">×</button>
</div>

<style lang="scss">
  .toast {
    display: flex;
    align-items: flex-start;
    gap: var(--space-sm);
    padding: var(--space-md);
    background: var(--tr-surface-raised);
    border-radius: var(--radius-md);
    box-shadow: var(--tr-elev-2);
    border-left: 4px solid var(--tr-border);
    animation: toast-in 150ms ease-out;
    min-width: 280px;
    max-width: 400px;
  }

  .toast-success {
    border-left-color: var(--tr-success);
  }
  .toast-error {
    border-left-color: var(--tr-danger);
  }
  .toast-info {
    border-left-color: var(--tr-accent);
  }
  .toast-warning {
    border-left-color: var(--tr-warning);
  }

  .toast-message {
    flex: 1;
    font-size: var(--font-size-body);
    color: var(--tr-text-primary);
    line-height: var(--line-height-body);
  }

  .toast-close {
    background: transparent;
    border: none;
    cursor: pointer;
    color: var(--tr-text-tertiary);
    font-size: 18px;
    line-height: 1;
    padding: 0;
    flex-shrink: 0;
    width: 20px;
    height: 20px;
    display: flex;
    align-items: center;
    justify-content: center;

    &:hover {
      color: var(--tr-text-primary);
    }
    &:focus-visible {
      box-shadow: 0 0 0 2px var(--tr-focus-ring);
      outline: none;
    }
  }

  @keyframes toast-in {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
</style>
