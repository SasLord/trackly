<script lang="ts">
  interface Props {
    section: string;
    phase: number | string;
  }

  const { section, phase }: Props = $props();

  const subline = $derived(
    phase === 'v2' ? 'В разработке (запланирована на v2)' : `Появится в Phase ${phase}`,
  );
</script>

<div class="placeholder">
  <h2 class="placeholder-heading">{section}</h2>
  <p class="placeholder-body">Раздел в разработке</p>
  <p class="placeholder-sub">{subline}</p>
</div>

<style lang="scss">
  .placeholder {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: var(--tr-space-4xl);
    text-align: center;
    min-height: 300px;
  }

  .placeholder-heading {
    margin: 0 0 var(--tr-space-xs);
    font-size: var(--tr-font-size-h3);
    font-weight: var(--tr-font-weight-semibold);
    line-height: var(--tr-line-height-h3);
    color: var(--tr-text-primary);
  }

  .placeholder-body {
    margin: 0 0 var(--tr-space-2xs);
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-secondary);
  }

  .placeholder-sub {
    margin: 0;
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-tertiary);
  }
</style>
