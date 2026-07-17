<script lang="ts">
  import type { Snippet } from 'svelte';
  import Spinner from './Spinner.svelte';

  interface Props {
    variant?: 'primary' | 'secondary' | 'destructive' | 'ghost' | 'link';
    size?: 'sm' | 'md';
    loading?: boolean;
    disabled?: boolean;
    type?: 'button' | 'submit';
    onclick?: () => void;
    children?: Snippet;
  }

  const {
    variant = 'primary',
    size = 'md',
    loading = false,
    disabled = false,
    type = 'button',
    onclick,
    children,
  }: Props = $props();

  const isDisabled = $derived(disabled || loading);
</script>

<button {type} class="btn btn-{variant} btn-{size}" class:loading disabled={isDisabled} {onclick}>
  {#if loading}
    <Spinner size="sm" />
  {/if}
  {@render children?.()}
</button>

<style lang="scss">
  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(--tr-space-2xs);
    border: none;
    border-radius: var(--tr-radius-sm);
    font-family: var(--font-family-base);
    font-weight: var(--font-weight-semibold);
    cursor: pointer;
    transition: none; // Theme switch: no transitions per UI-SPEC §Motion
    white-space: nowrap;
    text-decoration: none;

    &:disabled {
      opacity: 0.5;
      cursor: not-allowed;
      pointer-events: none;
    }

    &.loading {
      cursor: wait;
      pointer-events: none;
    }
  }

  // Sizes
  .btn-md {
    height: 36px;
    padding: 0 var(--tr-space-md);
    font-size: var(--font-size-body);
  }

  .btn-sm {
    height: 28px;
    padding: 0 12px;
    font-size: var(--font-size-label);
  }

  // Variants
  .btn-primary {
    background: var(--tr-accent);
    color: var(--tr-on-accent);

    &:hover:not(:disabled) {
      background: var(--tr-accent-hover);
    }
    &:focus-visible {
      box-shadow: 0 0 0 3px var(--tr-focus-ring);
    }
  }

  .btn-secondary {
    background: transparent;
    color: var(--tr-text-primary);
    border: 1px solid var(--tr-border-strong);

    &:hover:not(:disabled) {
      background: var(--tr-surface-sunken);
    }
    &:focus-visible {
      box-shadow: 0 0 0 3px var(--tr-focus-ring);
    }
  }

  .btn-destructive {
    background: var(--tr-danger);
    color: var(--tr-on-accent);

    &:hover:not(:disabled) {
      filter: brightness(0.92);
    }
    &:focus-visible {
      box-shadow: 0 0 0 3px rgba(220, 38, 38, 0.3);
    }
  }

  .btn-ghost {
    background: transparent;
    color: var(--tr-text-primary);

    &:hover:not(:disabled) {
      background: var(--tr-surface);
    }
    &:focus-visible {
      box-shadow: 0 0 0 3px var(--tr-focus-ring);
    }
  }

  .btn-link {
    background: transparent;
    color: var(--tr-accent);
    padding: 0;
    height: auto;

    &:hover:not(:disabled) {
      text-decoration: underline;
    }
    &:focus-visible {
      box-shadow: 0 0 0 3px var(--tr-focus-ring);
    }
  }
</style>
