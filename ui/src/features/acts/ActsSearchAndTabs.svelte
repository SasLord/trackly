<script lang="ts">
  // Plan 03-02: search input + switch-bar (Акты / Возвраты / Архив) с counter-badges.
  // Plan 27-02 (D-05): switch-bar migrated to shared Tabs primitive (variant="underline"),
  // per DeviceFilters.svelte precedent — count now built into Tabs, no bespoke <button class="tab">.
  import Input from '$lib/components/Input.svelte';
  import Tabs from '$lib/components/Tabs.svelte';
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

  // Tabs primitive requires a string-keyed contract — TabKey is already a string
  // literal union, so the adapter is a direct map (no String()/Number() coercion
  // needed, unlike DeviceFilters' number|null status ids).
  const tabItems = $derived([
    { key: 'handover' as TabKey, label: 'Акты', count: counts.handover_active },
    { key: 'returns' as TabKey, label: 'Возвраты', count: counts.returns },
    { key: 'archive' as TabKey, label: 'Архив', count: counts.archived },
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
  <Tabs
    variant="underline"
    tabs={tabItems}
    active={activeTab}
    ariaLabel="Категории актов"
    onchange={(key) => onTabChange(key as TabKey)}
  />
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
