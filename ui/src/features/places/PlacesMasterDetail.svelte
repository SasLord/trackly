<script lang="ts">
  // Phase 39 Plan 14 (39-UI-SPEC.md §6.2/§7): literal copy of
  // RequestsMasterDetail.svelte's structure — grid 35%/65%, gap
  // var(--tr-space-md), panels --tr-surface-raised + --tr-border +
  // --tr-radius-md + --tr-elev-1, flex-fill, <1099px fallback. UI-SPEC
  // explicitly forbids recalculating these values (§3's "320px / 480px —
  // скопировано дословно из RequestsMasterDetail.svelte, не пересчитывается").
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
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .master > :global(*) {
    flex: 1 1 auto;
    min-height: 0;
  }

  .detail {
    background: var(--tr-surface-raised);
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-md);
    box-shadow: var(--tr-elev-1);
    overflow: hidden;
    min-width: 480px;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

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
