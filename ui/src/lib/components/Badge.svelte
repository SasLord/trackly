<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    variant?: 'default' | 'accent' | 'success' | 'warning' | 'destructive';
    size?: 'sm' | 'md';
    appearance?: 'soft' | 'solid' | 'dot' | 'count';
    /** UAT gap 6: forwarded to the rendered span so callers (e.g. the tree's
     * per-node content counter) can attach a native tooltip without the caller
     * having to wrap the component in an extra span. */
    title?: string;
    /** Приглушённый вариант (UI-SPEC §10.2): действует только на warning/destructive
     * в appearance="soft"; для остальных тонов и appearances игнорируется. */
    muted?: boolean;
    children?: Snippet;
  }

  const {
    variant = 'default',
    size = 'md',
    appearance,
    title,
    muted = false,
    children,
  }: Props = $props();

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
  <span
    class="badge-m badge-m-{tone} badge-m-{appearance} badge-m-{size}"
    class:badge-m-muted={muted}
    {title}
  >
    {#if appearance === 'dot'}
      <span class="badge-m-dot-marker" aria-hidden="true"></span>
    {/if}
    {@render children?.()}
  </span>
{:else}
  <span class="badge badge-{variant} badge-{size}" {title}>
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
    &.badge-m-count {
      background: var(--tr-success-soft);
      color: var(--tr-success-text);
      border-color: var(--tr-success);
    }
  }

  .badge-m-warning {
    &.badge-m-soft {
      background: var(--tr-warning-soft);
      color: var(--tr-warning-text);
    }
    /* muted действует только на warning/danger в soft; у остальных тонов не определён. */
    &.badge-m-soft.badge-m-muted {
      background: var(--tr-warning-muted-soft);
      color: var(--tr-warning-muted-text);
      /* Нецветовой признак «ожидающего» статуса (WCAG 1.4.1): inset-рамка не меняет размеры чипа;
         в forced-colors box-shadow отбрасывается, поэтому признак дублирует outline (D-34). */
      box-shadow: inset 0 0 0 1px var(--tr-warning-muted-border);
      @media (forced-colors: active) {
        outline: 1px solid CanvasText;
        outline-offset: -1px;
      }
    }
    &.badge-m-solid {
      background: var(--tr-warning);
      color: var(--tr-on-accent);
    }
    &.badge-m-dot .badge-m-dot-marker {
      background: var(--tr-warning);
    }
    &.badge-m-count {
      background: var(--tr-warning-soft);
      color: var(--tr-warning-text);
      border-color: var(--tr-warning);
    }
  }

  .badge-m-danger {
    &.badge-m-soft {
      background: var(--tr-danger-soft);
      color: var(--tr-danger-text);
    }
    &.badge-m-soft.badge-m-muted {
      background: var(--tr-danger-muted-soft);
      color: var(--tr-danger-muted-text);
      /* Нецветовой признак «ожидающего» статуса (WCAG 1.4.1): inset-рамка не меняет размеры чипа;
         в forced-colors box-shadow отбрасывается, поэтому признак дублирует outline (D-34). */
      box-shadow: inset 0 0 0 1px var(--tr-danger-muted-border);
      @media (forced-colors: active) {
        outline: 1px solid CanvasText;
        outline-offset: -1px;
      }
    }
    &.badge-m-solid {
      background: var(--tr-danger);
      color: var(--tr-on-accent);
    }
    &.badge-m-dot .badge-m-dot-marker {
      background: var(--tr-danger);
    }
    &.badge-m-count {
      background: var(--tr-danger-soft);
      color: var(--tr-danger-text);
      border-color: var(--tr-danger);
    }
  }

  .badge-m-count {
    background: var(--tr-surface-sunken);
    color: var(--tr-text-secondary);
    border: 1px solid var(--tr-border-strong);
    border-radius: 11px;
    height: 20px;
    padding: 0 9px;
    font-size: 11px;
    justify-content: center;
  }

  .badge-m-count.badge-m-sm {
    height: 18px;
    padding: 0 7px;
  }

  .badge-m-accent.badge-m-count {
    background: var(--tr-accent-soft);
    color: var(--tr-accent-text);
    border-color: var(--tr-accent);
  }

  .badge-m-dot-marker {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    flex: none;
  }
</style>
