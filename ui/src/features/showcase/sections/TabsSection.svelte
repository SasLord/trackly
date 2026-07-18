<script lang="ts">
  import Tabs from '$lib/components/Tabs.svelte';

  const underlineTabs = [
    { key: 'all', label: 'Все', count: 12 },
    { key: 'open', label: 'Открытые', count: 4 },
    { key: 'closed', label: 'Закрытые' },
    { key: 'archived', label: 'Архив', disabled: true },
  ];
  let underlineActive = $state('all');

  const segmentedTabs = [
    { key: 'list', label: 'Список' },
    { key: 'map', label: 'Карта' },
    { key: 'table', label: 'Таблица' },
  ];
  let segmentedActive = $state('list');
</script>

<section class="tabs-section">
  <h2>Вкладки</h2>

  <div class="variant-block">
    <h3 class="variant-label">Switch-bar (underline)</h3>
    <Tabs variant="underline" tabs={underlineTabs} bind:active={underlineActive} />
  </div>

  <div class="variant-block">
    <h3 class="variant-label">Сегментированный</h3>
    <Tabs variant="segmented" tabs={segmentedTabs} bind:active={segmentedActive} />
  </div>
</section>

<style lang="scss">
  .tabs-section {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-lg);
  }

  h2 {
    margin: 0;
    font-size: var(--tr-font-size-h2);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-primary);
  }

  .variant-block {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--tr-space-sm);
  }

  .variant-label {
    margin: 0;
    font-size: var(--tr-font-size-label);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-secondary);
    text-transform: uppercase;
  }
</style>
