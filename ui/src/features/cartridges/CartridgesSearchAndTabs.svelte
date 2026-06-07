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
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
    margin-bottom: var(--space-md);
  }

  .search-wrap {
    max-width: 480px;
  }

  .search-spacer {
    height: 36px; // Reserve height to avoid layout shift when switching tabs
  }

  .tabs {
    display: flex;
    gap: var(--space-xs);
    flex-wrap: wrap;
  }

  .tab {
    display: inline-flex;
    align-items: center;
    gap: var(--space-xs);
    padding: var(--space-xs) var(--space-md);
    background: transparent;
    color: var(--color-text-primary);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    font-family: var(--font-family-base);
    font-size: var(--font-size-body);
    font-weight: var(--font-weight-regular);
    cursor: pointer;
    height: 32px;

    &:hover {
      background: var(--color-surface-sunken);
    }
    &:focus-visible {
      outline: none;
      box-shadow: 0 0 0 3px var(--color-accent-focus);
    }
    &.active {
      background: color-mix(in srgb, var(--color-accent) 10%, transparent);
      border-color: var(--color-accent);
      color: var(--color-text-primary);
      font-weight: var(--font-weight-semibold);
    }
  }

  @media (min-width: 1280px) {
    .search-and-tabs {
      flex-direction: row;
      align-items: center;
      justify-content: space-between;
    }
    .search-wrap {
      flex: 1;
      max-width: 50%;
    }
    .search-spacer {
      flex: 1;
      max-width: 50%;
    }
  }
</style>
