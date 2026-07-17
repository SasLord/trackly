<script lang="ts">
  // Plan 04-04: search input + tab switcher («Картриджи» / «Модели») для CartridgesPage.
  // Паттерн по образцу ActsSearchAndTabs.svelte.
  import Input from '$lib/components/Input.svelte';
  import Badge from '$lib/components/Badge.svelte';
  import type { CartridgeCountsDto } from '../../bindings';

  type TabKey = 'cartridges' | 'models';

  interface Props {
    searchQuery: string;
    activeTab: TabKey;
    counts: CartridgeCountsDto;
    onSearchChange: (_q: string) => void;
    onTabChange: (_tab: TabKey) => void;
  }

  const { searchQuery, activeTab, counts, onSearchChange, onTabChange }: Props = $props();

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

  const TABS: { key: TabKey; label: string }[] = [
    { key: 'cartridges', label: 'Картриджи' },
    { key: 'models', label: 'Модели' },
  ];
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
    <div class="search-spacer"></div>
  {/if}
  <nav class="tabs" aria-label="Разделы картриджей">
    {#each TABS as tab (tab.key)}
      <button
        class="tab"
        class:active={tab.key === activeTab}
        onclick={() => onTabChange(tab.key)}
        role="tab"
        aria-selected={tab.key === activeTab}
        type="button"
      >
        <span class="tab-label">{tab.label}</span>
        {#if tab.key === 'cartridges'}
          <span class="tab-badge">
            <Badge variant={tab.key === activeTab ? 'accent' : 'default'} size="sm">
              {counts.all}
            </Badge>
          </span>
        {/if}
      </button>
    {/each}
  </nav>
</div>

<style lang="scss">
  .search-and-tabs {
    // Поиск (слева) + свитч-бар (справа) всегда в одну строку. Свитч-бар
    // визуально остаётся на месте при переключении на «Модели» благодаря
    // .search-spacer, занимающему ту же долю строки, что и поиск.
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

  .search-spacer {
    flex: 1;
    max-width: 480px;
    height: 36px; // Reserve height to avoid layout shift when switching tabs
  }

  .tabs {
    display: flex;
    gap: var(--tr-space-2xs);
    flex-wrap: wrap;
  }

  .tab {
    display: inline-flex;
    align-items: center;
    gap: var(--tr-space-2xs);
    padding: var(--tr-space-2xs) var(--tr-space-md);
    background: transparent;
    color: var(--tr-text-primary);
    border: 1px solid var(--tr-border);
    border-radius: var(--radius-sm);
    font-family: var(--font-family-base);
    font-size: var(--font-size-body);
    font-weight: var(--font-weight-regular);
    cursor: pointer;
    height: 32px;

    &:hover {
      background: var(--tr-surface-sunken);
    }
    &:focus-visible {
      outline: none;
      box-shadow: 0 0 0 3px var(--tr-focus-ring);
    }
    &.active {
      background: color-mix(in srgb, var(--tr-accent) 10%, transparent);
      border-color: var(--tr-accent);
      color: var(--tr-text-primary);
      font-weight: var(--font-weight-semibold);
    }
  }
</style>
