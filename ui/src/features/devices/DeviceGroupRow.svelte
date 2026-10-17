<script lang="ts">
  // DeviceGroupRow — expandable row for non-unique device group.
  // Per UI-SPEC §DeviceGroupRow, DEV-11.
  // On expand: fetches full DeviceDto list via devices.listByIds().

  import Badge from '$lib/components/Badge.svelte';
  import TableRow from '$lib/components/TableRow.svelte';
  import DeviceListRow from './DeviceListRow.svelte';
  import { isRowOverlayEvent } from './deviceRowMenu';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { statusLabel, statusTone } from '$lib/stores/deviceStatuses.svelte';
  import { devices } from './api';
  import { groupIdsKey, visibleChildren } from './deviceListMode';
  import type { DeviceDto, DeviceGroup } from '../../bindings';

  // Compute a stable string key for this group from its grouping dimensions.
  // The key is based on the repr device's displayable fields that form the GROUP BY
  // clause in list_grouped SQL. Using repr.id as the key is fragile (repr changes
  // when the representative is deleted); this hash-based key is stable as long as
  // the group exists with the same attributes.
  function groupStableKey(g: DeviceGroup): string {
    return [
      g.repr.name,
      g.repr.model ?? '',
      g.repr.specs ?? '',
      g.repr.kit ?? '',
      g.repr.state ?? '',
      g.repr.full_path ?? '',
      String(g.repr.status_id),
    ].join('\x00');
  }

  interface Props {
    group: DeviceGroup;
    /** Set of stable keys that should be rendered expanded (from parent DevicesPage). */
    expandedGroups: Set<string>;
    /** Called when this group's expanded state changes. */
    onExpandToggle?: (_key: string, _expanded: boolean) => void;
    onEdit: (_d: DeviceDto) => void;
    onDelete: () => void;
    /** Plan 03-05 (DEV-14): pass-through. */
    onPrintAcceptance?: (_d: DeviceDto) => void;
    /** ITEM-3: when true, shows the «Статус» column. Hide on filtered status tabs. */
    showStatus?: boolean;
  }

  const {
    group,
    expandedGroups,
    onExpandToggle,
    onEdit,
    onDelete,
    onPrintAcceptance,
    showStatus = true,
  }: Props = $props();

  const stableKey = $derived(groupStableKey(group));
  const expanded = $derived(expandedGroups.has(stableKey));

  // Кеш детей псевдогруппы. Он валиден, пока ключ состава group.ids не изменился
  // (GAP-1): когда ребёнок стал якорем («Замещение группой»), ушёл «На разбор» или
  // вошёл в якорную группу, сервер отдаёт у той же группы новый group.ids.
  // Сам компонент остаётся прежним: ключ {#each} в DeviceList — group.repr.id, у
  // не-репрезентанта он не меняется. Поэтому кеш привязан к ключу состава, под которым
  // он загружен (loadedKey), а не к жизни компонента. Сравнивать результат загрузки с
  // group.ids нельзя: listByIds ограничен 1000 id.
  const idsKey = $derived(groupIdsKey(group.ids));
  let children = $state<DeviceDto[] | null>(null);
  let loadedKey = $state<string | null>(null);
  // Обычные переменные, не $state: отбрасывание устаревшего ответа и дедупликация
  // запроса не должны делать эффект зависимым от собственных записей.
  let loadSeq = 0;
  let loadingKey: string | null = null;

  // Рисуются только дети, которые ещё входят в group.ids: ушедший ребёнок пропадает
  // сразу, пока кеш перечитывается, без мигания «Загрузка…».
  const shownChildren = $derived(visibleChildren(children ?? [], group.ids));

  // Подпись и тон статуса представителя — из единого справочника (фаза 41.1).
  const label = $derived(statusLabel(group.repr.status_id));
  const tone = $derived(statusTone(group.repr.status_code));

  // ITEM-1: «разное» для смешанной группы по condition_distinct_count
  const conditionDisplay = $derived(
    group.condition_distinct_count > 1 ? 'разное' : (group.repr.state ?? '—'),
  );

  // Раскрытие только сообщает родителю новое состояние; детей загрузит $effect ниже.
  // Так путь «раскрыли вручную» и путь «список обновился после мутации» совпадают.
  function toggleExpand() {
    onExpandToggle?.(stableKey, !expanded);
  }

  // Единственный путь загрузки детей. $state пишется только в продолжении после await
  // (не из тела эффекта), иначе эффект, читающий собственные записи, зацикливается
  // (effect_update_depth_exceeded).
  function loadChildren(key: string) {
    const seq = ++loadSeq;
    loadingKey = key;
    devices
      .listByIds(group.ids)
      .then((rows) => {
        if (seq !== loadSeq) return;
        children = rows;
        loadedKey = key;
      })
      .catch((e: unknown) => {
        if (seq !== loadSeq) return;
        const msg =
          e && typeof e === 'object' && 'message' in e
            ? String((e as { message: unknown }).message)
            : 'Не удалось загрузить устройства';
        pushToast('error', msg);
        // Collapse again on error.
        onExpandToggle?.(stableKey, false);
      })
      .finally(() => {
        if (seq === loadSeq) loadingKey = null;
      });
  }

  // Загружает детей, когда группа раскрыта, а кеш загружен под другим составом (или
  // не загружен). Читает expanded, idsKey и loadedKey; children не читает.
  $effect(() => {
    if (expanded && loadedKey !== idsKey && loadingKey !== idsKey) loadChildren(idsKey);
  });

  // Двойной клик по свёртке: ЯВНО раскрыть состав. Два одиночных клика двойного
  // уже дважды переключили строку, поэтому toggle здесь дал бы третье
  // переключение; у свёртки нет сущности для просмотра (D-26).
  function handleGroupDblClick(e: MouseEvent) {
    if (isRowOverlayEvent(e.target)) return;
    window.getSelection()?.removeAllRanges();
    if (!expanded) void toggleExpand();
  }

  // После мутации (правка/удаление/действие из меню) родитель перечитывает список
  // (onEdit/onDelete зовут refresh() в DevicesPage). Здесь кеш помечается устаревшим,
  // и эффект перечитывает детей; если состав group.ids при этом изменился, ключ состава
  // сам отсечёт ушедших детей (visibleChildren).
  function handleEdit(d: DeviceDto) {
    loadedKey = null;
    onEdit(d);
  }

  function handleDelete() {
    loadedKey = null;
    onDelete();
  }
</script>

<TableRow
  group
  groupExpanded={expanded}
  groupName={group.repr.name}
  groupColspan={4}
  onToggleGroup={toggleExpand}
  ondblclick={handleGroupDblClick}
>
  <!-- groupColspan={4} merges Наименование + Инв.№ + Серийный № + Модель columns;
       TableRow's own group-mode chevron + merged name cell replace the hand-rolled
       ones this migration removes. -->
  <td
    class="cell cell-truncate"
    title={group.place_distinct_count > 1 ? '' : (group.repr.full_path ?? '')}
    >{group.place_distinct_count > 1 ? '—' : (group.repr.place_path_short ?? '—')}</td
  >
  <td class="cell cell-truncate" title={conditionDisplay}>{conditionDisplay}</td>
  {#if showStatus}
    <td class="cell cell-status">
      <Badge variant={tone.variant} appearance={tone.appearance} muted={tone.muted}>{label}</Badge>
    </td>
  {/if}
  <!-- Actions column: count badge for multi-device groups -->
  <td class="cell cell-actions cell-count">
    <Badge variant="accent" appearance="count">{group.count} шт.</Badge>
  </td>
</TableRow>

{#if expanded}
  {#if children === null}
    <tr class="children-loading-row">
      <td colspan={showStatus ? 8 : 7} class="children-loading">Загрузка…</td>
    </tr>
  {:else}
    {#each shownChildren as child, i (child.id)}
      <DeviceListRow
        device={child}
        onEdit={handleEdit}
        onDelete={handleDelete}
        isLastInGroup={i === shownChildren.length - 1}
        {onPrintAcceptance}
        {showStatus}
      />
    {/each}
  {/if}
{/if}

<style lang="scss">
  .cell {
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-primary);
  }

  // Location + Состояние cells: single line with ellipsis (mirror DeviceListRow .cell).
  // max-width: 0 makes text-overflow work inside a table cell; title= provides the
  // full text on hover (ITEM-2 tooltip). Prevents long condition labels from
  // wrapping and stretching the group row height.
  .cell-truncate {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 0;
  }

  .cell-status {
    width: 120px;
  }

  .cell-actions {
    width: 40px;
    text-align: center;
    overflow: visible;
  }

  .cell-count {
    white-space: nowrap;
  }

  .children-loading-row td {
    background: var(--tr-bg);
  }

  .children-loading {
    padding: var(--tr-space-2xs) var(--tr-space-md);
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-tertiary);
    border-bottom: 1px solid var(--tr-border);
  }
</style>
