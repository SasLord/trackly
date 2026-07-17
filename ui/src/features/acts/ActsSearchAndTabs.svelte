<script lang="ts">
  // Plan 03-02: search input + switch-bar (Акты / Возвраты / Архив) с counter-badges.
  import Input from '$lib/components/Input.svelte';
  import Badge from '$lib/components/Badge.svelte';
  import type { ActsCountsDto } from '../../bindings';

  type TabKey = 'handover' | 'returns' | 'archive';

  interface Props {
    searchQuery: string;
    activeTab: TabKey;
    counts: ActsCountsDto;
    onSearchChange: (_q: string) => void;
    onTabChange: (_tab: TabKey) => void;
  }

  const { searchQuery, activeTab, counts, onSearchChange, onTabChange }: Props = $props();

  let localQuery = $state(searchQuery);
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;

  $effect(() => {
    // External reset (e.g. «Сбросить поиск»).
    if (searchQuery !== localQuery && document.activeElement?.id !== 'acts-search') {
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

  const TABS: { key: TabKey; label: string; count: number }[] = $derived([
    { key: 'handover', label: 'Акты', count: counts.handover_active },
    { key: 'returns', label: 'Возвраты', count: counts.returns },
    { key: 'archive', label: 'Архив', count: counts.archived },
  ]);
</script>

<div class="search-and-tabs">
  <div class="search-wrap">
    <Input
      id="acts-search"
      type="search"
      value={localQuery}
      placeholder="Поиск по номеру, ФИО, наименованию устройства"
      oninput={handleInput}
    />
  </div>
  <nav class="tabs" aria-label="Категории актов">
    {#each TABS as tab (tab.key)}
      <button
        class="tab"
        class:active={tab.key === activeTab}
        onclick={() => onTabChange(tab.key)}
        aria-pressed={tab.key === activeTab}
        type="button"
      >
        <span class="tab-label">{tab.label}</span>
        <span class="tab-badge">
          <Badge variant={tab.key === activeTab ? 'accent' : 'default'} size="sm">
            {tab.count}
          </Badge>
        </span>
      </button>
    {/each}
  </nav>
</div>

<style lang="scss">
  .search-and-tabs {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-md);
    margin-bottom: var(--tr-space-md);
  }

  .search-wrap {
    max-width: 480px;
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
    border-radius: var(--tr-radius-xs);
    font-family: var(--font-family-base);
    font-size: var(--font-size-body);
    font-weight: var(--font-weight-medium);
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
  }
</style>
