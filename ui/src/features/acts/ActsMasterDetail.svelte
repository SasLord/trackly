<script lang="ts">
  // Plan 03-02: master-detail CSS-grid layout (35% / 65%). На viewport <1100px —
  // horizontal scroll на родительском main (Phase 2 D-UI-Responsive-01).
  import type { Snippet } from 'svelte';

  interface Props {
    master: Snippet;
    detail: Snippet;
  }
  const { master, detail }: Props = $props();
</script>

<div class="master-detail">
  <aside class="master">
    {@render master()}
  </aside>
  <section class="detail">
    {@render detail()}
  </section>
</div>

<style lang="scss">
  .master-detail {
    display: grid;
    grid-template-columns: 35% 65%;
    gap: var(--tr-space-md);
    align-items: stretch;
    // FIX B1: fill the remaining height of page-content instead of sizing to
    // a viewport-relative min-height — closes the gap at the bottom of the
    // window and lets the panels below scroll internally.
    flex: 1 1 auto;
    min-height: 0;
  }

  .master {
    background: var(--tr-surface-raised);
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-md);
    box-shadow: var(--tr-elev-1);
    overflow: hidden;
    min-width: 320px;
    // FIX B1: panel is a flex column so its single child (the master List's
    // Table) can flex-fill and scroll internally instead of the panel itself
    // growing to content height.
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  // FIX B1: stretch whatever the master snippet renders (the List's Table
  // root) to fill the panel — Table's own fillHeight mode then owns the
  // sticky header / internal scroll / pinned footer.
  .master > :global(*) {
    flex: 1 1 auto;
    min-height: 0;
  }

  .detail {
    background: var(--tr-surface-raised);
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-md);
    box-shadow: var(--tr-elev-1);
    // FIX B1: was `overflow: auto` (the panel itself scrolled, sizing to
    // content and leaving a gap below). Now hidden — DetailPanel/detail-loading
    // scroll internally instead.
    overflow: hidden;
    min-width: 480px;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  // FIX B1: stretch whatever the detail snippet renders (DetailPanel, or the
  // detail-loading spinner wrapper) to fill the panel height.
  .detail > :global(*) {
    flex: 1 1 auto;
    min-height: 0;
  }

  @media (max-width: 1099px) {
    .master-detail {
      grid-template-columns: 380px 1fr;
      min-width: 900px;
    }
  }
</style>
