<script lang="ts">
  // Phase 27, plan 01 (D-01): shared detail-panel primitive.
  // Field label/value widget — covers the field-grid item duplicated across
  // ActDetail/CartridgeDetail/PrinterDetail, and is contract-compatible with
  // the acts window's former field-widget component (label/value, null -> '—').
  import type { Snippet } from 'svelte';

  interface Props {
    label: string;
    value?: string | null;
    /** Фаза 41.1: интерактивное значение (например, ссылка) вместо текста.
     *  Если передано — `value` не показывается. */
    children?: Snippet;
  }

  const { label, value, children }: Props = $props();
</script>

<div class="field">
  <span class="field-label">{label}</span>
  {#if children}
    <span class="field-value">{@render children()}</span>
  {:else}
    <span class="field-value">{value ?? '—'}</span>
  {/if}
</div>

<style lang="scss">
  .field {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-3xs);
  }
  .field-label {
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-tertiary);
  }
  .field-value {
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-primary);
  }
</style>
