<script lang="ts">
  import { push } from 'svelte-spa-router';
  import Table from '$lib/components/Table.svelte';
  import Button from '$lib/components/Button.svelte';
  import DeviceListRow from './DeviceListRow.svelte';
  import DeviceGroupRow from './DeviceGroupRow.svelte';
  import { footerText, type AnchorMembersState } from './deviceListMode';
  import type { DeviceDto, DeviceGroup } from '../../bindings';

  interface Props {
    /** Номер поколения списка (DevicesPage): входит в ключ кеша детей свёртки. */
    listGen: number;
    items: DeviceDto[];
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
    /** D-11: в режиме «Свернуть одинаковые» якоря выводятся отдельно, рядом со строками свёртки. */
    anchorItems?: DeviceDto[];
    /** group_id раскрытых якорей: живёт в DevicesPage и переживает перезагрузку списка. */
    expandedAnchors?: Set<number>;
    /** Состав раскрытых якорей по group_id. */
    anchorMembers?: Record<number, AnchorMembersState>;
    onToggleAnchor?: (_groupId: number) => void;
    /** K для футера «(+K в составе групп)» (D-34): hidden_member_count сервера. */
    hiddenMemberCount?: number;
    /** Скрытие членов якорных групп сейчас включено (shouldHideAnchorMembers). */
    hideActive?: boolean;
  }

  const {
    listGen,
    items,
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
    anchorItems = [],
    expandedAnchors = new Set<number>(),
    anchorMembers = {},
    onToggleAnchor,
    hiddenMemberCount = 0,
    hideActive = false,
  }: Props = $props();

  // GAP-8: scroll the highlighted row into view once it's actually rendered
  // (DevicesPage forces `grouped=false` while a focus id is pending, so the
  // row is guaranteed to be a plain DeviceListRow in `items`, never hidden
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

  // Якоря в режиме свёртки выводятся отдельными строками (D-11), поэтому список,
  // состоящий из одних якорей, тоже «свёрнутый», а не пустой.
  const showGroups = $derived(
    grouped && !searchActive && (groups.length > 0 || anchorItems.length > 0),
  );
  const isEmpty = $derived(
    !loading && (showGroups ? groups.length === 0 && anchorItems.length === 0 : items.length === 0),
  );
  // Mirrors the exact skeleton-branch condition of the pre-migration 3-way
  // if/else — used both as Table's `loading` prop and to gate the footer
  // (footer only rendered in the "real table" branch, matching prior behavior).
  const skeletonLoading = $derived(
    loading && items.length === 0 && groups.length === 0 && anchorItems.length === 0,
  );

  const memberColspan = $derived(showStatus ? 8 : 7);

  function openGroup(groupId: number) {
    void push(`#/groups?id=${groupId}`);
  }

  // In grouped mode, singletons (count == 1) render as plain DeviceListRow.
  // Groups with count > 1 render as expandable DeviceGroupRow.
  // This ensures every device is visible in grouped mode — no vanishing singletons.

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
         with its own border-top + padding. A second wrapping <footer> here produced the
         doubled, over-tall footer with two top borders. -->
    <span class="pagination-info">
      {#if showGroups}
        Свёрнуто: {groups.length}
      {:else}
        {footerText({ shown: items.length, total, hidden: hiddenMemberCount, hideActive })}
      {/if}
    </span>
  {/if}
{/snippet}

<!-- Строка устройства; для якоря (D-12) под ней раскрывается состав группы
     плоским списком: устройства группы и вложенных групп без якоря. -->
{#snippet rowWithMembers(device: DeviceDto)}
  {@const groupId = device.anchor_group_id ?? null}
  {@const isOpen = groupId !== null && expandedAnchors.has(groupId)}
  <DeviceListRow
    {device}
    {onEdit}
    {onDelete}
    {onPrintAcceptance}
    {showStatus}
    {highlightId}
    expanded={isOpen}
    onToggleExpand={onToggleAnchor}
  />
  {#if groupId !== null && isOpen}
    {@const st = anchorMembers[groupId] ?? { loading: true, items: [], total: 0 }}
    {#if st.loading}
      <tr class="members-row">
        <td colspan={memberColspan} class="members-note">Загрузка…</td>
      </tr>
    {:else if st.items.length === 0}
      <tr class="members-row">
        <td colspan={memberColspan} class="members-note">
          <span>Состав пуст. Добавьте устройства в разделе «Группы».</span>
          <span class="members-link">
            <Button variant="link" onclick={() => openGroup(groupId)}>Открыть группу</Button>
          </span>
        </td>
      </tr>
    {:else}
      {#each st.items as member, i (member.id)}
        <DeviceListRow
          device={member}
          {onEdit}
          {onDelete}
          {onPrintAcceptance}
          {showStatus}
          {highlightId}
          indent
          isLastInGroup={i === st.items.length - 1}
        />
      {/each}
      {#if st.total > st.items.length}
        <tr class="members-row">
          <td colspan={memberColspan} class="members-note">
            Показаны первые {st.items.length} из {st.total}. Остальные — в разделе «Группы».
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
>
  {#if showGroups}
    {#each anchorItems as anchor (anchor.id)}
      {@render rowWithMembers(anchor)}
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
        {@render rowWithMembers(group.repr)}
      {/if}
    {/each}
  {:else}
    {#each items as device (device.id)}
      {@render rowWithMembers(device)}
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

  .members-link {
    margin-left: var(--tr-space-xs);
  }

  .pagination-info {
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-secondary);
  }
</style>
