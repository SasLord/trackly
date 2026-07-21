<script lang="ts">
  // Phase 27, plan 01 (D-01): shared detail-panel primitive.
  // Extracted per PageHeader.svelte precedent — Snippet-slots, single responsibility.
  // Covers the scroll-container + empty-state + header (title/actions) that
  // ActDetail/CartridgeDetail/PrinterDetail duplicate verbatim.
  // NOTE: does NOT paint a background — the master-detail wrapper (D-02, plans
  // 27-02/04/07) owns the panel surface, to avoid a double fill.
  import type { Snippet } from 'svelte';

  interface Props {
    title?: string;
    empty?: boolean;
    emptyTitle?: string;
    emptyBody?: string;
    actions?: Snippet;
    emptyActions?: Snippet;
    children?: Snippet;
  }

  const {
    title,
    empty = false,
    emptyTitle,
    emptyBody,
    actions,
    emptyActions,
    children,
  }: Props = $props();
</script>

<div class="detail-panel">
  {#if empty}
    <div class="empty">
      {#if emptyTitle}
        <h2 class="empty-heading">{emptyTitle}</h2>
      {/if}
      {#if emptyBody}
        <p class="empty-body">{emptyBody}</p>
      {/if}
      {@render emptyActions?.()}
    </div>
  {:else}
    <header class="detail-header">
      <h2 class="detail-title">{title}</h2>
      <div class="actions">
        {@render actions?.()}
      </div>
    </header>
    {@render children?.()}
  {/if}
</div>

<style lang="scss">
  .detail-panel {
    height: 100%;
    overflow: auto;
    padding: var(--tr-space-xl);
  }

  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--tr-space-md);
    min-height: 320px;
    text-align: center;
    color: var(--tr-text-secondary);
  }
  .empty-heading {
    margin: 0;
    font-size: var(--tr-font-size-h3);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-primary);
  }
  .empty-body {
    margin: 0;
    max-width: 360px;
    color: var(--tr-text-secondary);
  }

  // Sticky at the top of .detail-panel's own scroll container (FIX A4) so the
  // title + actions stay visible while detail content scrolls. .detail-panel
  // intentionally paints no background (the master-detail wrapper owns the
  // panel surface, see NOTE above), so the sticky bar needs its own opaque
  // background or scrolled content would show through underneath it. Pulled
  // out to the panel's edges via negative margins matching --tr-space-xl (the
  // panel's own padding) and re-padded so it spans full width without clipping,
  // plus a border-bottom for separation from the scrolling content below it.
  .detail-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--tr-space-md);
    flex-wrap: wrap;
    margin: calc(var(--tr-space-xl) * -1) calc(var(--tr-space-xl) * -1) var(--tr-space-2xl);
    padding: var(--tr-space-xl) var(--tr-space-xl) var(--tr-space-md);
    position: sticky;
    top: 0;
    z-index: 2;
    background: var(--tr-surface-raised);
    border-bottom: 1px solid var(--tr-border);
  }
  .detail-title {
    margin: 0;
    font-size: var(--tr-font-size-h3);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-primary);
    font-variant-numeric: tabular-nums;
  }
  .actions {
    display: flex;
    gap: var(--tr-space-xs);
    flex-wrap: wrap;
  }
</style>
