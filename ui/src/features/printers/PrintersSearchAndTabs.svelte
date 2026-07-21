<script lang="ts">
  // Plan 06-04: поиск + switch-bar статусов.
  // По паттерну CartridgesSearchAndTabs.svelte.
  // FIX F1 (Phase 27 batch F): кнопка «Найти принтеры» перенесена в PageHeader
  // раздела (PrintersPage.svelte) — здесь больше не отображается.
  import Input from '$lib/components/Input.svelte';
  import Tabs from '$lib/components/Tabs.svelte';
  import type { PrinterFilter } from '../../bindings-phase6';

  interface Props {
    filter: PrinterFilter;
    onFilterChange: (_f: PrinterFilter) => void;
  }

  const { filter, onFilterChange }: Props = $props();

  type StatusTab = null | 'ok' | 'warning' | 'error' | 'offline';

  interface Tab {
    key: StatusTab;
    label: string;
  }

  const TABS: Tab[] = [
    { key: null, label: 'Все' },
    { key: 'ok', label: 'В сети' },
    { key: 'warning', label: 'Предупреждение' },
    { key: 'error', label: 'Ошибка' },
    { key: 'offline', label: 'Не в сети' },
  ];

  let localQuery = $state(filter.search ?? '');
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;

  $effect(() => {
    const ext = filter.search ?? '';
    if (ext !== localQuery) localQuery = ext;
  });

  function handleInput(v: string) {
    localQuery = v;
    if (debounceTimer !== null) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
      onFilterChange({ ...filter, search: v.trim() || null });
    }, 250);
  }

  function handleTabClick(key: StatusTab) {
    // Map 'ok' → null for search-compatible status (server uses 'ok'/'warning'/'error'/'offline').
    onFilterChange({ ...filter, status: key });
  }

  // D-05: Tabs требует string key — адаптер туда-обратно (String(null) === 'null').
  const tabItems = $derived(TABS.map((t) => ({ key: String(t.key), label: t.label })));

  function handleTabsChange(key: string) {
    handleTabClick(key === 'null' ? null : (key as StatusTab));
  }
</script>

<div class="search-and-tabs">
  <div class="search-wrap">
    <Input
      id="printers-search"
      type="search"
      value={localQuery}
      placeholder="Поиск по имени, IP, модели"
      oninput={handleInput}
    />
  </div>
  <div class="tabs-wrap">
    <Tabs
      variant="underline"
      tabs={tabItems}
      active={String(filter.status)}
      ariaLabel="Статус принтеров"
      onchange={handleTabsChange}
    />
  </div>
</div>

<style lang="scss">
  .search-and-tabs {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: var(--tr-space-md);
    margin-bottom: var(--tr-space-md);
    flex-wrap: wrap;
  }

  .search-wrap {
    flex: 1;
    max-width: 380px;
    min-width: 160px;
  }

  .tabs-wrap {
    flex: 1;
    min-width: 0;
  }
</style>
