<script lang="ts">
  // Plan 25-06 (CMP-07): showcase gallery for Dropdown.svelte (Plans 25-02/
  // 25-03) — static demo data only, no live API calls, no DeviceGroup/
  // DeviceDto import (this section proves the primitive works with ANY
  // caller shape, not just the device picker it was extracted from).
  import { onMount, tick } from 'svelte';
  import Dropdown from '$lib/components/Dropdown.svelte';

  interface DemoGroup {
    id: string;
    name: string;
    meta?: string;
    count: number;
    expandable: boolean;
    selected?: boolean;
  }
  interface DemoMember {
    id: string;
    name: string;
    sub?: string;
  }

  // --- Block 1: combobox + groups (drill-in) ---
  const groupsDemo: DemoGroup[] = [
    { id: 'g1', name: 'Мышь Logitech M185', meta: 'Периферия', count: 5, expandable: true },
    { id: 'g2', name: 'Монитор Samsung S24', meta: 'Периферия', count: 1, expandable: false },
  ];
  const membersDemo: DemoMember[] = [
    { id: 'm1', name: 'Инв. № 000123', sub: 'SN-AA11' },
    { id: 'm2', name: 'Инв. № 000124', sub: 'SN-AA12' },
    { id: 'm3', name: 'Инв. № 000125', sub: 'SN-AA13' },
  ];
  let comboValue = $state('');

  function expandDemoGroup(g: DemoGroup): DemoMember[] {
    return g.id === 'g1' ? membersDemo : [];
  }

  // --- Block 2: flat select with in-panel search + checkmark ---
  const flatOptions: DemoGroup[] = [
    { id: 'f1', name: 'Заправлен', count: 0, expandable: false, selected: false },
    { id: 'f2', name: 'В работе', count: 0, expandable: false, selected: true },
    { id: 'f3', name: 'Списан', count: 0, expandable: false, selected: false },
  ];
  let flatValue = $state('В работе');

  // --- Blocks 3/4: empty / loading panel states (D-13) ---
  const emptyGroups: DemoGroup[] = [];
  let emptyValue = $state('');
  let loadingValue = $state('');

  function noopExpand(): DemoMember[] {
    return [];
  }
  function neverExpandable(): boolean {
    return false;
  }

  // Anchors used only to locate each Dropdown instance's rendered field
  // element for the onMount forced-open sequence below.
  let groupsDemoEl: HTMLDivElement | undefined;
  let flatDemoEl: HTMLDivElement | undefined;
  let emptyDemoEl: HTMLDivElement | undefined;
  let loadingDemoEl: HTMLDivElement | undefined;

  // Dropdown's open/viewMode/drill-in state is fully internal (Plan 25-02
  // D-02, no bindable props) — the only way to force the panel into a
  // permanently-visible drilled-in/empty/loading state on page load (so the
  // reviewer sees it without interacting, per this plan's must_haves) is a
  // programmatic focus/click sequence identical to what a real user
  // produces. Result: the drill-in header's "← Назад" button becomes
  // visible without page interaction, closing CMP-07 SC #4 visually.
  onMount(() => {
    void (async () => {
      await tick();

      // Block 1: focus opens the groups view; a synthetic click on the
      // first (expandable) option drills into its members.
      const comboInput = groupsDemoEl?.querySelector('input');
      comboInput?.focus();
      await tick();
      const panelId = comboInput?.getAttribute('aria-controls');
      if (panelId) {
        await tick();
        const panel = document.getElementById(panelId);
        const firstOption = panel?.querySelector<HTMLButtonElement>('.tr-dropdown-option');
        firstOption?.click();
      }

      // Block 2: a single click opens the flat select panel (search box +
      // checkmark on the pre-selected option), no drill-in involved.
      const flatTrigger = flatDemoEl?.querySelector('button');
      flatTrigger?.click();

      // Blocks 3/4: focus opens the panel directly onto the empty/loading
      // state — no groups to drill into.
      const emptyInput = emptyDemoEl?.querySelector('input');
      emptyInput?.focus();
      const loadingInput = loadingDemoEl?.querySelector('input');
      loadingInput?.focus();
    })();
  });
</script>

<section class="dropdown-section">
  <h2>Dropdown</h2>

  <div class="variant-block">
    <h3 class="variant-label">Комбобокс с группами (drill-in)</h3>
    <div class="demo-anchor" bind:this={groupsDemoEl}>
      <Dropdown
        variant="combobox"
        value={comboValue}
        placeholder="Выберите устройство"
        loading={false}
        groups={groupsDemo}
        getGroupId={(g) => g.id}
        getGroupName={(g) => g.name}
        getGroupMeta={(g) => g.meta}
        getGroupCount={(g) => g.count}
        isGroupExpandable={(g) => g.expandable}
        onExpandGroup={expandDemoGroup}
        getMemberId={(m) => m.id}
        getMemberName={(m) => m.name}
        getMemberSub={(m) => m.sub}
        onSearch={() => {}}
        onQueryInput={(q) => (comboValue = q)}
        onPickGroup={(g) => (comboValue = g.name)}
        onPickMember={(m) => (comboValue = m.name)}
      />
    </div>
  </div>

  <div class="variant-block">
    <h3 class="variant-label">Плоский селект</h3>
    <div class="demo-anchor" bind:this={flatDemoEl}>
      <Dropdown
        variant="select"
        flat={true}
        value={flatValue}
        placeholder="Выберите состояние"
        searchPlaceholder="Поиск состояния"
        loading={false}
        groups={flatOptions}
        getGroupId={(g) => g.id}
        getGroupName={(g) => g.name}
        getGroupCount={() => 0}
        isGroupExpandable={neverExpandable}
        isGroupSelected={(g) => !!g.selected}
        onExpandGroup={noopExpand}
        getMemberId={(m) => m.id}
        getMemberName={(m) => m.name}
        onSearch={() => {}}
        onPickGroup={(g) => (flatValue = g.name)}
        onPickMember={() => {}}
      />
    </div>
  </div>

  <div class="variant-block">
    <h3 class="variant-label">Пустое состояние</h3>
    <div class="demo-anchor" bind:this={emptyDemoEl}>
      <Dropdown
        variant="combobox"
        value={emptyValue}
        placeholder="Ничего не найдётся"
        loading={false}
        groups={emptyGroups}
        getGroupId={(g) => g.id}
        getGroupName={(g) => g.name}
        getGroupCount={(g) => g.count}
        isGroupExpandable={neverExpandable}
        onExpandGroup={noopExpand}
        getMemberId={(m) => m.id}
        getMemberName={(m) => m.name}
        onSearch={() => {}}
        onQueryInput={(q) => (emptyValue = q)}
        onPickGroup={() => {}}
        onPickMember={() => {}}
      />
    </div>
  </div>

  <div class="variant-block">
    <h3 class="variant-label">Загрузка</h3>
    <div class="demo-anchor" bind:this={loadingDemoEl}>
      <Dropdown
        variant="combobox"
        value={loadingValue}
        placeholder="Идёт поиск"
        loading={true}
        groups={emptyGroups}
        getGroupId={(g) => g.id}
        getGroupName={(g) => g.name}
        getGroupCount={(g) => g.count}
        isGroupExpandable={neverExpandable}
        onExpandGroup={noopExpand}
        getMemberId={(m) => m.id}
        getMemberName={(m) => m.name}
        onSearch={() => {}}
        onQueryInput={(q) => (loadingValue = q)}
        onPickGroup={() => {}}
        onPickMember={() => {}}
      />
    </div>
  </div>
</section>

<style lang="scss">
  .dropdown-section {
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

  .demo-anchor {
    width: 320px;
    max-width: 100%;
  }
</style>
