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
    min-height: calc(100vh - 240px);
  }

  .master {
    background: var(--tr-surface-raised);
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-md);
    box-shadow: var(--tr-elev-1);
    overflow: hidden;
    min-width: 320px;
  }

  .detail {
    background: var(--tr-surface-raised);
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-md);
    box-shadow: var(--tr-elev-1);
    overflow: auto;
    min-width: 480px;
  }

  @media (max-width: 1099px) {
    .master-detail {
      grid-template-columns: 380px 1fr;
      min-width: 900px;
    }
  }
</style>
