<script lang="ts">
  // Plan 06-04: поиск + switch-bar статусов + кнопка «Найти принтеры».
  // По паттерну CartridgesSearchAndTabs.svelte.
  // Кнопка «Найти принтеры» — видна только admin (D-RBAC-03, UI-SPEC §Interaction Contracts 2).
  import Input from '$lib/components/Input.svelte';
  import Button from '$lib/components/Button.svelte';
  import type { PrinterFilter } from '../../bindings-phase6';
  import type { CurrentUser } from '$lib/stores/auth.svelte';

  interface Props {
    filter: PrinterFilter;
    onFilterChange: (_f: PrinterFilter) => void;
    onDiscoveryClick: () => void;
    identity: CurrentUser | null;
  }

  const { filter, onFilterChange, onDiscoveryClick, identity }: Props = $props();

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

  const isAdmin = $derived(identity?.role === 'admin');
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
  <div class="tabs" role="tablist" aria-label="Статус принтеров">
    {#each TABS as tab (String(tab.key))}
      <button
        class="tab"
        class:active={filter.status === tab.key}
        onclick={() => handleTabClick(tab.key)}
        role="tab"
        aria-selected={filter.status === tab.key}
        type="button"
      >
        <span class="tab-label">{tab.label}</span>
      </button>
    {/each}
  </div>
  {#if isAdmin}
    <Button variant="primary" onclick={onDiscoveryClick}>Найти принтеры</Button>
  {/if}
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

  .tabs {
    display: flex;
    gap: var(--tr-space-2xs);
    flex-wrap: wrap;
    flex: 1;
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
