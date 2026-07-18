<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    variant?: 'default' | 'accent' | 'success' | 'warning' | 'destructive';
    size?: 'sm' | 'md';
    appearance?: 'soft' | 'solid' | 'dot' | 'count';
    children?: Snippet;
  }

  const { variant = 'default', size = 'md', appearance, children }: Props = $props();

  const TONE_MAP = {
    default: 'neutral',
    accent: 'accent',
    success: 'success',
    warning: 'warning',
    destructive: 'danger',
  } as const;

  const tone = $derived(TONE_MAP[variant]);
</script>

{#if appearance}
  <span class="badge-m badge-m-{tone} badge-m-{appearance} badge-m-{size}">
    {#if appearance === 'dot'}
      <span class="badge-m-dot-marker" aria-hidden="true"></span>
    {/if}
    {@render children?.()}
  </span>
{:else}
  <span class="badge badge-{variant} badge-{size}">
    {@render children?.()}
  </span>
{/if}

<style lang="scss">
  .badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0 var(--tr-space-xs);
    border-radius: 10px;
    font-size: 12px;
    font-weight: var(--tr-font-weight-medium);
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
    background: var(--tr-surface-sunken);
    color: var(--tr-text-primary);
  }

  .badge-accent {
    background: var(--tr-accent);
    color: var(--tr-on-accent);
  }

  .badge-success {
    background: color-mix(in srgb, var(--tr-success) 15%, transparent);
    color: var(--tr-success);
  }

  .badge-warning {
    background: color-mix(in srgb, var(--tr-warning) 15%, transparent);
    color: var(--tr-warning);
  }

  .badge-destructive {
    background: color-mix(in srgb, var(--tr-danger) 15%, transparent);
    color: var(--tr-danger);
  }

  /* Opt-in appearance matrix (5 tones x soft/solid/dot/count) — separate namespace,
     shares zero selectors with the legacy .badge* rules above. */
  .badge-m {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 22px;
    padding: 0 10px;
    border-radius: 11px;
    font-size: 12px;
    font-weight: 600;
    white-space: nowrap;
    line-height: 1;
  }

  .badge-m-md {
    height: 22px;
  }
  .badge-m-sm {
    height: 18px;
    padding: 0 8px;
    font-size: 11px;
  }

  .badge-m-neutral {
    &.badge-m-soft {
      background: var(--tr-surface-sunken);
      color: var(--tr-text-secondary);
    }
    &.badge-m-solid {
      background: var(--tr-border-strong);
      color: var(--tr-text-primary);
    }
    &.badge-m-dot .badge-m-dot-marker {
      background: var(--tr-text-tertiary);
    }
  }

  .badge-m-accent {
    &.badge-m-soft {
      background: var(--tr-accent-soft);
      color: var(--tr-accent-text);
    }
    &.badge-m-solid {
      background: var(--tr-accent);
      color: var(--tr-on-accent);
    }
    &.badge-m-dot .badge-m-dot-marker {
      background: var(--tr-accent);
    }
  }

  .badge-m-success {
    &.badge-m-soft {
      background: var(--tr-success-soft);
      color: var(--tr-success-text);
    }
    &.badge-m-solid {
      background: var(--tr-success);
      color: var(--tr-on-accent);
    }
    &.badge-m-dot .badge-m-dot-marker {
      background: var(--tr-success);
    }
  }

  .badge-m-warning {
    &.badge-m-soft {
      background: var(--tr-warning-soft);
      color: var(--tr-warning-text);
    }
    &.badge-m-solid {
      background: var(--tr-warning);
      color: var(--tr-on-accent);
    }
    &.badge-m-dot .badge-m-dot-marker {
      background: var(--tr-warning);
    }
  }

  .badge-m-danger {
    &.badge-m-soft {
      background: var(--tr-danger-soft);
      color: var(--tr-danger-text);
    }
    &.badge-m-solid {
      background: var(--tr-danger);
      color: var(--tr-on-accent);
    }
    &.badge-m-dot .badge-m-dot-marker {
      background: var(--tr-danger);
    }
  }

  .badge-m-count {
    background: var(--tr-surface-sunken);
    color: var(--tr-text-secondary);
    border-radius: 11px;
    min-width: 18px;
    height: 18px;
    padding: 0 6px;
    font-size: 11px;
    justify-content: center;
  }

  .badge-m-accent.badge-m-count {
    background: var(--tr-accent-soft);
    color: var(--tr-accent-text);
    border: 1px solid var(--tr-accent);
    border-radius: 11px;
    padding: 0 9px;
    height: 20px;
    font-size: 11px;
  }

  .badge-m-dot-marker {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    flex: none;
  }
</style>
