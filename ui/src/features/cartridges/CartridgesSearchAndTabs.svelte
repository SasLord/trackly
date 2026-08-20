<script lang="ts">
  // Plan 04-04: search input + tab switcher («Картриджи» / «Модели») для CartridgesPage.
  // Паттерн по образцу ActsSearchAndTabs.svelte.
  import Input from '$lib/components/Input.svelte';
  import Tabs from '$lib/components/Tabs.svelte';
  import type { CartridgeCountsDto } from '../../bindings';

  type TabKey = 'cartridges' | 'models';

  interface Props {
    searchQuery: string;
    activeTab: TabKey;
    counts: CartridgeCountsDto;
    onSearchChange: (_q: string) => void;
    onTabChange: (_tab: TabKey) => void;
    modelSearchQuery: string;
    onModelSearchChange: (_q: string) => void;
  }

  const {
    searchQuery,
    activeTab,
    counts,
    onSearchChange,
    onTabChange,
    modelSearchQuery,
    onModelSearchChange,
  }: Props = $props();

  let localQuery = $state(searchQuery);
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;

  $effect(() => {
    // Внешний сброс поиска.
    if (searchQuery !== localQuery && document.activeElement?.id !== 'cartridges-search') {
      localQuery = searchQuery;
    }
  });

  function handleInput(v: string) {
    localQuery = v;
    if (debounceTimer !== null) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
      onSearchChange(v);
    }, 250);
  }

  let localModelQuery = $state(modelSearchQuery);
  let modelDebounceTimer: ReturnType<typeof setTimeout> | null = null;

  $effect(() => {
    // Внешний сброс фильтра моделей.
    if (modelSearchQuery !== localModelQuery && document.activeElement?.id !== 'models-search') {
      localModelQuery = modelSearchQuery;
    }
  });

  function handleModelInput(v: string) {
    localModelQuery = v;
    if (modelDebounceTimer !== null) clearTimeout(modelDebounceTimer);
    modelDebounceTimer = setTimeout(() => {
      onModelSearchChange(v);
    }, 250);
  }

  const TABS: { key: TabKey; label: string }[] = [
    { key: 'cartridges', label: 'Картриджи' },
    { key: 'models', label: 'Модели' },
  ];

  // Tabs требует string-ключи со встроенным count — TabKey уже строковый,
  // адаптер тривиален. Счётчик (было: <Badge> справа от подписи) показываем
  // только на вкладке «Картриджи» — на «Модели» count оставляем undefined.
  const tabItems = $derived(
    TABS.map((t) => ({
      key: t.key,
      label: t.label,
      count: t.key === 'cartridges' ? counts.all : undefined,
    })),
  );
</script>

<div class="search-and-tabs">
  {#if activeTab === 'cartridges'}
    <div class="search-wrap">
      <Input
        id="cartridges-search"
        type="search"
        value={localQuery}
        placeholder="Поиск по коду, модели, расположению"
        oninput={handleInput}
      />
    </div>
  {:else}
    <div class="search-wrap">
      <Input
        id="models-search"
        type="search"
        value={localModelQuery}
        placeholder="Поиск по бренду, модели, примечанию"
        oninput={handleModelInput}
      />
    </div>
  {/if}
  <Tabs
    variant="underline"
    tabs={tabItems}
    active={activeTab}
    ariaLabel="Разделы картриджей"
    onchange={(key) => onTabChange(key as TabKey)}
  />
</div>

<style lang="scss">
  .search-and-tabs {
    // Поиск (слева) + свитч-бар (справа) всегда в одну строку. Свитч-бар
    // визуально остаётся на месте при переключении на «Модели» благодаря
    // .search-wrap, занимающему ту же долю строки в обеих ветках (поиск по
    // картриджам / поиск по моделям).
    display: flex;
    flex-direction: row;
    align-items: center;
    justify-content: space-between;
    gap: var(--tr-space-md);
    margin-bottom: var(--tr-space-md);
  }

  .search-wrap {
    flex: 1;
    max-width: 480px;
  }
</style>
