<script lang="ts">
  // Plan 06-05: поиск + switch-bar статусов заявок + кнопка «Создать заявку».
  // По паттерну PrintersSearchAndTabs.svelte.
  // Plan 28-01 (D-05): switch-bar migrated to shared Tabs primitive (variant="underline"),
  // per ActsSearchAndTabs.svelte precedent — bespoke <button class="tab"> removed. This
  // component has no search input (no debounce/Input, unlike Acts) — only status tabs +
  // «Создать заявку». StatusTab includes `null` ("Все") — Tabs requires a string key, so a
  // String()-adapter + `key === 'null'` round-trip is required (same pattern as
  // DeviceFilters.svelte).
  import Button from '$lib/components/Button.svelte';
  import Tabs from '$lib/components/Tabs.svelte';
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

  type StatusTab = null | 'open' | 'in_progress' | 'completed' | 'rejected' | 'cancelled';

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
    { key: 'cancelled', label: 'Отменённые' },
  ];

  function handleTabClick(key: StatusTab) {
    onFilterChange({ ...filter, status: key });
  }

  // identity is accepted for future role-based tab enhancements (RBAC enforced in backend).

  // String-key adapter — required because Tabs' contract is `Tab.key: string`, but
  // StatusTab includes `null` (for «Все»). No `count` — this component currently has
  // no status counters to pass, and inventing them is out of scope (SC #4).
  const tabItems = $derived(TABS.map((t) => ({ key: String(t.key), label: t.label })));
</script>

<div class="search-and-tabs">
  <Tabs
    variant="underline"
    tabs={tabItems}
    active={String(filter.status)}
    ariaLabel="Статус заявок"
    onchange={(key) => handleTabClick(key === 'null' ? null : (key as StatusTab))}
  />
  {#if canCreate}
    <Button variant="primary" onclick={onCreateClick}>Создать заявку</Button>
  {/if}
</div>

<style lang="scss">
  .search-and-tabs {
    display: flex;
    flex-direction: row;
    align-items: center;
    // Tabs replaces the old .tabs{flex:1} wrapper — space-between keeps the same
    // visual result (tabs left-aligned, «Создать заявку» pushed to the far right)
    // without reintroducing a bespoke flex-growing wrapper div around Tabs.
    justify-content: space-between;
    gap: var(--tr-space-md);
    margin-bottom: var(--tr-space-md);
    flex-wrap: wrap;
  }
</style>
