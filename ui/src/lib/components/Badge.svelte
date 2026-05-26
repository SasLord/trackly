<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    variant?: 'default' | 'accent' | 'success' | 'warning' | 'destructive';
    size?: 'sm' | 'md';
    children?: Snippet;
  }

  const { variant = 'default', size = 'md', children }: Props = $props();
</script>

<span class="badge badge-{variant} badge-{size}">
  {@render children?.()}
</span>

<style lang="scss">
  .badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0 var(--space-sm);
    border-radius: 10px;
    font-size: 12px;
    font-weight: var(--font-weight-medium);
    line-height: 1;
    white-space: nowrap;
  }

  .badge-md {
    height: 20px;
  }
  .badge-sm {
    height: 16px;
    font-size: 11px;
  }

  .badge-default {
    background: var(--color-surface-sunken);
    color: var(--color-text-primary);
  }

  .badge-accent {
    background: var(--color-accent);
    color: #ffffff;
  }

  .badge-success {
    background: color-mix(in srgb, var(--color-success) 15%, transparent);
    color: var(--color-success);
  }

  .badge-warning {
    background: color-mix(in srgb, var(--color-warning) 15%, transparent);
    color: var(--color-warning);
  }

  .badge-destructive {
    background: color-mix(in srgb, var(--color-destructive) 15%, transparent);
    color: var(--color-destructive);
  }
</style>
