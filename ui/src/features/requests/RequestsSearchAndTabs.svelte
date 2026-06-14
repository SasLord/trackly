<script lang="ts">
  // Plan 06-05: поиск + switch-bar статусов заявок + кнопка «Создать заявку».
  // По паттерну PrintersSearchAndTabs.svelte.
  import Button from '$lib/components/Button.svelte';
  import type { RequestFilter } from '../../bindings-phase6';
  import type { CurrentUser } from '$lib/stores/auth.svelte';

  interface Props {
    filter: RequestFilter;
    onFilterChange: (_f: RequestFilter) => void;
    onCreateClick: () => void;
    /** Passed for role-based enhancements; currently used to gate «Создать заявку» visibility (all roles can create). */
    identity: CurrentUser | null;
  }

  const { filter, onFilterChange, onCreateClick, identity: _identity }: Props = $props();

  // «Создать заявку» visible to all roles (REQ-01). identity accepted for future role-based use.
  const canCreate = $derived(_identity !== undefined);

  type StatusTab = null | 'open' | 'in_progress' | 'completed' | 'rejected';

  interface Tab {
    key: StatusTab;
    label: string;
  }

  const TABS: Tab[] = [
    { key: null, label: 'Все' },
    { key: 'open', label: 'Созданные' },
    { key: 'in_progress', label: 'В работе' },
    { key: 'completed', label: 'Выполненные' },
    { key: 'rejected', label: 'Отклонённые' },
  ];

  function handleTabClick(key: StatusTab) {
    onFilterChange({ ...filter, status: key });
  }

  // identity is accepted for future role-based tab enhancements (RBAC enforced in backend).
</script>

<div class="search-and-tabs">
  <nav class="tabs" aria-label="Статус заявок" role="tablist">
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
  </nav>
  {#if canCreate}
    <Button variant="primary" onclick={onCreateClick}>Создать заявку</Button>
  {/if}
</div>

<style lang="scss">
  .search-and-tabs {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: var(--space-md);
    margin-bottom: var(--space-md);
    flex-wrap: wrap;
  }

  .tabs {
    display: flex;
    gap: var(--space-xs);
    flex-wrap: wrap;
    flex: 1;
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
</style>
