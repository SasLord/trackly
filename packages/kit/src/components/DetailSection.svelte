<script lang="ts">
  // Phase 27, plan 01 (D-01): shared detail-panel primitive.
  // Section wrapper (heading + body) extracted from ActDetail/CartridgeDetail/
  // PrinterDetail `.section`/`.section-heading` markup (identical by value).
  import type { Snippet } from 'svelte';

  interface Props {
    heading?: string;
    children?: Snippet;
  }

  const { heading, children }: Props = $props();
</script>

<section class="detail-section">
  {#if heading}
    <h3 class="section-heading">{heading}</h3>
  {/if}
  {@render children?.()}
</section>

<style lang="scss">
  .detail-section {
    margin-bottom: var(--tr-space-2xl);
  }
  .section-heading {
    margin: 0 0 var(--tr-space-md);
    font-size: var(--tr-font-size-body);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-primary);
  }
</style>
