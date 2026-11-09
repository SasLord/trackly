<script lang="ts">
  import { push } from 'svelte-spa-router';
  import Table from '$lib/components/Table.svelte';
  import Button from '$lib/components/Button.svelte';
  import Spinner from '$lib/components/Spinner.svelte';
  import RowReload from '$lib/components/icons/RowReload.svelte';
  import DeviceListRow from './DeviceListRow.svelte';
  import DeviceContainerRow from './DeviceContainerRow.svelte';
  import DeviceGroupRow from './DeviceGroupRow.svelte';
  import { footerText, type GroupMembersState } from './deviceListMode';
  import type { DeviceDto, DeviceEntryDto, DeviceGroup } from '../../bindings';

  interface Props {
    /** Номер поколения списка (DevicesPage): входит в ключ кеша детей свёртки. */
    listGen: number;
    /** Строки верхнего уровня: устройства и строки АРМ (плоский список, поиск). */
    rows: DeviceEntryDto[];
    groups: DeviceGroup[];
    total: number;
    loading: boolean;
    grouped: boolean;
    searchActive: boolean;
    /** Set of stable group keys that should be rendered expanded. */
    expandedGroups?: Set<string>;
    /** Called when a group's expansion state toggles. */
    onExpandToggle?: (_key: string, _expanded: boolean) => void;
    onEdit: (_d: DeviceDto) => void;
    onDelete: () => void;
    /** Plan 03-05 (DEV-14): pass-through. */
    onPrintAcceptance?: (_d: DeviceDto) => void;
    /** ITEM-3: when true, shows the «Статус» column. Hide on filtered status tabs. */
    showStatus?: boolean;
    /** GAP-8: cross-section focus id — see DeviceListRow.svelte's doc-comment. */
    highlightId?: number | null;
    /** D-05: в режиме «Свернуть одинаковые» блок якорей и АРМ над свёртками. */
    headRows?: DeviceEntryDto[];
    /** group_id раскрытых якорей и АРМ: живёт в DevicesPage и переживает перезагрузку списка. */
    expandedAnchors?: Set<number>;
    /** Состав раскрытых групп по group_id. */
    anchorMembers?: Record<number, GroupMembersState<DeviceEntryDto>>;
    onToggleGroup?: (_groupId: number, _kind: 'anchor' | 'container') => void;
    onLoadRest?: (_groupId: number) => void;
    /** K для футера «(+K в составе групп)» (D-34): hidden_member_count сервера. */
    hiddenMemberCount?: number;
    /** Скрытие членов якорных групп сейчас включено (shouldHideAnchorMembers). */
    hideActive?: boolean;
    /** D-07: подгрузка следующей порции. */
    loadingMore?: boolean;
    moreError?: boolean;
    onNearEnd?: () => void;
    onRetryMore?: () => void;
    /** Прокручиваемый контейнер Table (для сброса scrollTop страницей). */
    scrollEl?: HTMLDivElement | null;
  }

  let {
    listGen,
    rows,
    groups,
    total,
    loading,
    grouped,
    searchActive,
    expandedGroups,
    onExpandToggle,
    onEdit,
    onDelete,
    onPrintAcceptance,
    showStatus = true,
    highlightId = null,
    headRows = [],
    expandedAnchors = new Set<number>(),
    anchorMembers = {},
    onToggleGroup,
    onLoadRest,
    hiddenMemberCount = 0,
    hideActive = false,
    loadingMore = false,
    moreError = false,
    onNearEnd,
    onRetryMore,
    scrollEl = $bindable(null),
  }: Props = $props();

  // GAP-8: scroll the highlighted row into view once it's actually rendered
  // (DevicesPage forces `grouped=false` while a focus id is pending, so the
  // row is guaranteed to be a plain DeviceListRow in `rows`, never hidden
  // inside a collapsed group). One-shot per highlightId — re-running on
  // every list refresh would fight the user's own subsequent scrolling.
  let scrolledToId = $state<number | null>(null);
  $effect(() => {
    const id = highlightId;
    if (id === null || id === scrolledToId || loading) return;
    const el = document.getElementById(`device-row-${id}`);
    if (el) {
      el.scrollIntoView({ block: 'center' });
      scrolledToId = id;
    }
  });

  // Якоря и АРМ в режиме свёртки выводятся отдельными строками (D-05), поэтому список,
  // состоящий из одного блока, тоже «свёрнутый», а не пустой.
  const showGroups = $derived(
    grouped && !searchActive && (groups.length > 0 || headRows.length > 0),
  );
  const isEmpty = $derived(
    !loading && (showGroups ? groups.length === 0 && headRows.length === 0 : rows.length === 0),
  );
  // Mirrors the exact skeleton-branch condition of the pre-migration 3-way
  // if/else — used both as Table's `loading` prop and to gate the footer
  // (footer only rendered in the "real table" branch, matching prior behavior).
  const skeletonLoading = $derived(
    loading && rows.length === 0 && groups.length === 0 && headRows.length === 0,
  );

  const memberColspan = $derived(showStatus ? 8 : 7);

  function openGroup(groupId: number) {
    void push(`#/groups?id=${groupId}`);
  }

  const EMPTY_STATE: GroupMembersState<DeviceEntryDto> = {
    loading: true,
    entries: [],
    total: 0,
    loadedAll: false,
    loadingRest: false,
  };

  /** Отступ первой ячейки строки-заглушки по глубине (токены, не px). */
  function noteIndent(depth: number): string {
    const token = depth <= 1 ? '--tr-space-2xl' : depth === 2 ? '--tr-space-4xl' : '--tr-space-5xl';
    return `padding-left: var(${token})`;
  }

  const emptyMessage = $derived(
    searchActive ? 'По вашему запросу ничего не найдено' : 'Устройств пока нет',
  );
  const emptySubtext = $derived(
    searchActive
      ? 'Попробуйте изменить поисковый запрос или сбросить фильтр статуса.'
      : 'Создайте первое устройство или импортируйте список из CSV.',
  );
</script>

{#snippet tableHead()}
  <th class="th-name">Наименование</th>
  <th class="th-numeric">Инвентарный №</th>
  <th class="th-numeric">Серийный №</th>
  <th>Модель</th>
  <th>Место</th>
  <th class="th-condition">Состояние</th>
  {#if showStatus}<th class="th-status">Статус</th>{/if}
  <th class="th-actions">Действия</th>
{/snippet}

{#snippet footer()}
  {#if !skeletonLoading && !isEmpty}
    <!-- Content only — Table.svelte already wraps this in <footer class="tr-table-footer">
         with its own border-top + padding. -->
    <div class="list-footer">
      <span class="pagination-info">
        {#if showGroups}
          {footerText({
            shown: groups.length,
            total,
            hidden: hiddenMemberCount,
            hideActive,
            groupedMode: true,
          })}
        {:else}
          {footerText({ shown: rows.length, total, hidden: hiddenMemberCount, hideActive })}
        {/if}
      </span>
      <div role="status" class="more-status">
        {#if loadingMore}
          <Spinner size="sm" />
          <span>Загрузка</span>
        {:else if moreError}
          <span>Не удалось загрузить продолжение списка.</span>
          <Button variant="link" size="sm" onclick={() => onRetryMore?.()}>Повторить</Button>
        {/if}
      </div>
    </div>
  {/if}
{/snippet}

<!-- Строка верхнего уровня или вложенная: устройство, якорь или АРМ. Под раскрытой
     строкой рекурсивно рисуются её дети (D-11); `closing` — строка лежит на
     «закрывающем» пути корневого раскрытия (UI-SPEC 9.3). -->
{#snippet entryRows(entry: DeviceEntryDto, depth: number, closing: boolean)}
  {@const groupId =
    entry.kind === 'container' ? entry.container.group_id : (entry.device.anchor_group_id ?? null)}
  {@const isOpen = groupId !== null && expandedAnchors.has(groupId)}
  {@const st = groupId !== null && isOpen ? (anchorMembers[groupId] ?? EMPTY_STATE) : null}
  {@const hasKids = st !== null && !st.loading && st.entries.length > 0}
  {@const loadRest =
    groupId !== null && st !== null && !st.loading && st.entries.length < st.total
      ? {
          shown: st.entries.length,
          total: st.total,
          loading: st.loadingRest,
          onclick: () => onLoadRest?.(groupId),
        }
      : null}
  {@const lastBorder = closing && !hasKids}
  {#if entry.kind === 'container'}
    <DeviceContainerRow
      container={entry.container}
      expanded={isOpen}
      {depth}
      {showStatus}
      isLastInGroup={lastBorder}
      onToggleExpand={(id) => onToggleGroup?.(id, 'container')}
      {loadRest}
    />
  {:else}
    <DeviceListRow
      device={entry.device}
      {onEdit}
      {onDelete}
      {onPrintAcceptance}
      {showStatus}
      {highlightId}
      expanded={isOpen}
      onToggleExpand={(id) => onToggleGroup?.(id, 'anchor')}
      indent={depth >= 1}
      {depth}
      isLastInGroup={lastBorder}
      {loadRest}
    />
  {/if}
  {#if groupId !== null && st !== null}
    {#if st.loading}
      <tr class="members-row">
        <td colspan={memberColspan} class="members-note" style={noteIndent(depth + 1)}>Загрузка…</td
        >
      </tr>
    {:else if st.entries.length === 0}
      <tr class="members-row">
        <td colspan={memberColspan} class="members-note" style={noteIndent(depth + 1)}>
          <span>Состав пуст. Добавьте устройства в разделе «Группы».</span>
          <span class="members-link">
            <Button variant="link" onclick={() => openGroup(groupId)}>Открыть группу</Button>
          </span>
        </td>
      </tr>
    {:else}
      {#each st.entries as child, i (child.kind === 'container' ? 'c' + child.container.group_id : 'd' + child.device.id)}
        {@render entryRows(
          child,
          depth + 1,
          loadRest !== null
            ? false
            : depth === 0
              ? i === st.entries.length - 1
              : closing && i === st.entries.length - 1,
        )}
      {/each}
      {#if loadRest !== null}
        {@const groupName =
          entry.kind === 'container'
            ? entry.container.name
            : (entry.device.anchor_group_name ?? '')}
        <tr class="members-row" class:members-row-last={depth === 0 || closing}>
          <td colspan={memberColspan} class="members-note" style={noteIndent(depth + 1)}>
            <span
              >{footerText({
                shown: loadRest.shown,
                total: loadRest.total,
                hidden: 0,
                hideActive: false,
              })}</span
            >
            <span class="members-link">
              <Button
                variant="link"
                loading={loadRest.loading}
                onclick={loadRest.onclick}
                ariaLabel={'Загрузить весь состав группы «' + groupName + '»'}
              >
                <RowReload /> Загрузить ещё
              </Button>
            </span>
          </td>
        </tr>
      {/if}
    {/if}
  {/if}
{/snippet}

<Table
  columns={showStatus ? 8 : 7}
  loading={skeletonLoading}
  empty={isEmpty}
  emptyTitle={emptyMessage}
  emptyBody={emptySubtext}
  head={tableHead}
  {footer}
  minWidth="860px"
  fillHeight
  {onNearEnd}
  bind:wrapperEl={scrollEl}
>
  {#if showGroups}
    {#each headRows as entry (entry.kind === 'container' ? 'c' + entry.container.group_id : 'd' + entry.device.id)}
      {@render entryRows(entry, 0, false)}
    {/each}
    {#each groups as group (group.repr.id)}
      {#if group.count > 1}
        <!-- Multi-device group: expandable row with chevron and count badge -->
        <DeviceGroupRow
          {group}
          {listGen}
          {onEdit}
          {onDelete}
          expandedGroups={expandedGroups ?? new Set()}
          {onExpandToggle}
          {onPrintAcceptance}
          {showStatus}
        />
      {:else}
        <!-- Singleton group (count == 1): render as plain row, no chevron -->
        {@render entryRows({ kind: 'device', device: group.repr }, 0, false)}
      {/if}
    {/each}
  {:else}
    {#each rows as entry (entry.kind === 'container' ? 'c' + entry.container.group_id : 'd' + entry.device.id)}
      {@render entryRows(entry, 0, false)}
    {/each}
  {/if}
</Table>

<style lang="scss">
  .th-name {
    width: 25%;
  }
  .th-numeric {
    width: 140px;
  }
  .th-condition {
    width: 120px;
  }
  .th-status {
    width: 120px;
  }
  .th-actions {
    width: 40px;
  }

  // Строки-заглушки раскрытого состава якоря: по образцу DeviceGroupRow.children-loading.
  .members-row td {
    background: var(--tr-bg);
  }

  .members-note {
    padding: var(--tr-space-2xs) var(--tr-space-md);
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-secondary);
    border-bottom: 1px solid var(--tr-border);
  }

  .members-row-last .members-note {
    border-bottom: 2px solid var(--tr-border-strong);
  }

  .members-link {
    margin-left: var(--tr-space-xs);
  }

  .list-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--tr-space-md);
    min-height: 28px;
    width: 100%;
  }

  .pagination-info {
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-secondary);
  }

  .more-status {
    display: inline-flex;
    align-items: center;
    gap: var(--tr-space-xs);
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-secondary);
  }
</style>
