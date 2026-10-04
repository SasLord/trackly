<script lang="ts">
  // Phase 41 Plan 21 (GRP-04..GRP-06, D-01..D-08, 41-UI-SPEC.md §11): состав
  // группы — главная рабочая поверхность раздела. Таблица повторяет «Содержимое
  // места» (PlaceContents): Тип · Название · Инв. № / Серийный № · Место ·
  // Статус + колонка действий. Вложенные группы — раскрываемые строки
  // (лениво, groups.composition(child.id)), как DeviceGroupRow и drill-in
  // Dropdown формы акта.
  //
  // Строка добавления внизу — ОДИН Dropdown-combobox для двух видов элементов:
  // свёртки устройств (devices.listGrouped, БЕЗ фильтра статуса — АРМ в
  // работе) и группы (groups.search); секции «Устройства»/«Группы» рисует сам
  // Dropdown (getGroupSection). Перетаскивания нет: HTML5 DnD не работает в
  // WKWebView.
  //
  // Источник истины — сервер: «устройство уже в группе», цикл, teardown,
  // потолок пакета — отказ приходит с сервера и показывается Toast. Пометка
  // «уже в группе «…»» — подсказка (одним запросом groups.forDevices на
  // результат поиска), выбор занятого устройства НЕ блокируется.
  import { untrack } from 'svelte';
  import Table from '$lib/components/Table.svelte';
  import TableRow from '$lib/components/TableRow.svelte';
  import Badge from '$lib/components/Badge.svelte';
  import ActionMenu from '$lib/components/ActionMenu.svelte';
  import Dropdown from '$lib/components/Dropdown.svelte';
  import Spinner from '$lib/components/Spinner.svelte';
  import { groups } from '$lib/api/groups';
  import { devices } from '$lib/api/devices';
  import { pluralizeRu } from '$lib/utils/pluralize';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { notifyPlaceContentChanged } from '$lib/stores/placeContentEvents.svelte';
  import type { AppError } from '$lib/api/errors';
  import type {
    DeviceDto,
    DeviceGroup,
    GroupCompositionDto,
    GroupDto,
    GroupMemberDeviceDto,
    GroupSearchHitDto,
  } from '../../bindings';

  interface Props {
    group: GroupDto;
    /** admin | manager: строка добавления, «Вывести из состава», «Переименовать». */
    canEdit: boolean;
    /** Страница увеличивает счётчик, чтобы заставить состав перечитаться. */
    refreshToken?: number;
    /** Состав изменился (добавили/вывели): страница обновляет дерево и карточку. */
    onChanged?: () => void;
    /** «Открыть группу»: страница фокусирует её узел в дереве. */
    onOpenGroup: (_groupId: number) => void;
    onRenameGroup: (_groupId: number) => void;
    /** Необязательно: клик/Enter по первой ячейке строки устройства. */
    onOpenDevice?: (_deviceId: number) => void;
  }

  const {
    group,
    canEdit,
    refreshToken = 0,
    onChanged,
    onOpenGroup,
    onRenameGroup,
    onOpenDevice,
  }: Props = $props();

  const COLUMNS = 6;

  function errMessage(e: unknown, fallback: string): string {
    const err = e as Partial<AppError> | undefined;
    return err?.message ?? fallback;
  }

  // ---- Состав ---------------------------------------------------------------
  let comp = $state<GroupCompositionDto | null>(null);
  let loading = $state(true);
  let loadSeq = 0;
  let expanded = $state<Record<number, boolean>>({});
  let childComp = $state<Record<number, { loading: boolean; data: GroupCompositionDto | null }>>(
    {},
  );

  async function loadComposition(): Promise<void> {
    const seq = ++loadSeq;
    try {
      const data = await groups.composition(group.id);
      if (seq !== loadSeq) return;
      const openIds = Object.keys(expanded)
        .map(Number)
        .filter((id) => expanded[id]);
      const fresh: Record<number, { loading: boolean; data: GroupCompositionDto | null }> = {};
      await Promise.all(
        openIds.map(async (id) => {
          try {
            fresh[id] = { loading: false, data: await groups.composition(id) };
          } catch {
            // Вложенная группа исчезла или недоступна — просто сворачиваем.
            expanded[id] = false;
          }
        }),
      );
      if (seq !== loadSeq) return;
      comp = data;
      childComp = fresh;
    } catch {
      if (seq === loadSeq) {
        pushToast('error', 'Не удалось загрузить группы. Проверьте подключение и повторите.');
      }
    } finally {
      if (seq === loadSeq) loading = false;
    }
  }

  // Смена группы или refreshToken -> перечитать. Записи состояния — в untrack,
  // чтобы эффект не зависел от собственных записей.
  let lastGroupId: number | null = null;
  $effect(() => {
    const id = group.id;
    void refreshToken;
    untrack(() => {
      if (lastGroupId !== id) {
        lastGroupId = id;
        comp = null;
        loading = true;
        expanded = {};
        childComp = {};
        query = '';
        suggestions = [];
      }
      void loadComposition();
    });
  });

  async function toggleChild(id: number): Promise<void> {
    const next = !expanded[id];
    expanded[id] = next;
    if (!next || childComp[id]?.data) return;
    childComp[id] = { loading: true, data: null };
    try {
      const data = await groups.composition(id);
      childComp[id] = { loading: false, data };
    } catch (e) {
      delete childComp[id];
      expanded[id] = false;
      pushToast('error', errMessage(e, 'Не удалось загрузить состав вложенной группы.'));
    }
  }

  const isEmpty = $derived(
    comp !== null && comp.devices.length === 0 && comp.child_groups.length === 0,
  );

  function deviceCountLabel(g: GroupDto): string {
    const word = pluralizeRu(g.device_count, ['устройство', 'устройства', 'устройств']);
    return `${g.name} (${g.device_count} ${word})`;
  }

  // ---- Отображение строки устройства ---------------------------------------
  type BadgeVariant = 'default' | 'accent' | 'success' | 'warning' | 'destructive';
  const STATUS_VARIANT_BY_NAME: Record<string, BadgeVariant> = {
    'На складе': 'default',
    'В работе': 'accent',
    'На ремонте': 'warning',
    'На заправке': 'warning',
    Списано: 'destructive',
  };
  function statusVariant(name: string | null): BadgeVariant {
    if (!name) return 'default';
    return STATUS_VARIANT_BY_NAME[name] ?? 'default';
  }

  function activateDevice(e: KeyboardEvent, d: GroupMemberDeviceDto): void {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      onOpenDevice?.(d.device_id);
    }
  }

  // ---- Вывод из состава (D-02: без подтверждения) --------------------------
  async function removeDevice(deviceId: number, ownerGroupId: number): Promise<void> {
    try {
      await groups.removeDevices({ group_id: ownerGroupId, device_ids: [deviceId] });
      await loadComposition();
      onChanged?.();
    } catch (e) {
      pushToast('error', errMessage(e, 'Не удалось вывести устройство из состава.'));
    }
  }

  // Вложение/вывод группы меняют место её устройств (если у родителя есть
  // место) — обязательна инвалидация счётчиков дерева «Места» (D-08, INV-7).
  async function detachGroup(child: GroupDto): Promise<void> {
    try {
      const result = await groups.setParent({
        id: child.id,
        version: child.version,
        parent_group_id: null,
      });
      notifyPlaceContentChanged(result.changed_place_ids);
      await loadComposition();
      onChanged?.();
    } catch (e) {
      pushToast('error', errMessage(e, 'Не удалось вывести группу из состава.'));
    }
  }

  // ---- Строка добавления ----------------------------------------------------
  type SearchItem =
    | { kind: 'device'; key: string; group: DeviceGroup }
    | { kind: 'group'; key: string; hit: GroupSearchHitDto };

  type MemberRow =
    | { kind: 'instance'; key: string; device: DeviceDto }
    | { kind: 'subgroup'; key: string; state: string | null; devices: DeviceDto[] }
    | { kind: 'group'; key: string; hit: GroupSearchHitDto };

  const SECTION_DEVICES = 'Устройства';
  const SECTION_GROUPS = 'Группы';
  // Серверный потолок пакета — 500 id; запрос занятости не должен его превышать.
  const FOR_DEVICES_CAP = 500;

  let query = $state('');
  let suggestions = $state<SearchItem[]>([]);
  let searching = $state(false);
  let searchSeq = 0;
  /** device_id -> имя группы, в которой оно уже состоит (подсказка). */
  let occupied = $state<Record<number, string>>({});

  async function loadOccupancy(ids: number[]): Promise<void> {
    const unique = Array.from(new Set(ids)).slice(0, FOR_DEVICES_CAP);
    if (unique.length === 0) return;
    try {
      const memberships = await groups.forDevices(unique);
      const next = { ...occupied };
      for (const id of unique) delete next[id];
      for (const m of memberships) next[m.device_id] = m.group_name;
      occupied = next;
    } catch {
      // Подсказка необязательна: без неё выбор остаётся рабочим, отказ даст сервер.
    }
  }

  async function fetchSuggestions(q: string): Promise<void> {
    const seq = ++searchSeq;
    searching = true;
    try {
      const [devGroups, hits] = await Promise.all([
        devices
          .listGrouped(
            {
              type_id: null,
              place_id: null,
              status_id: null,
              state: null,
              name_prefix: q,
              include_deleted: false,
              group_by_condition: true,
            },
            { offset: 0, limit: 20 },
          )
          .catch((): DeviceGroup[] => []),
        groups.search(q, group.id).catch((): GroupSearchHitDto[] => []),
      ]);
      if (seq !== searchSeq) return;
      suggestions = [
        ...devGroups.map((g): SearchItem => ({ kind: 'device', key: `d-${g.repr.id}`, group: g })),
        ...hits.map((h): SearchItem => ({ kind: 'group', key: `g-${h.id}`, hit: h })),
      ];
      void loadOccupancy(devGroups.flatMap((g) => g.ids));
    } finally {
      if (seq === searchSeq) searching = false;
    }
  }

  function joinSnInv(
    sn: string | null | undefined,
    inv: string | null | undefined,
  ): string | undefined {
    const parts = [sn ? `SN ${sn}` : null, inv ? `инв. ${inv}` : null].filter(
      (p): p is string => p !== null,
    );
    return parts.length > 0 ? parts.join(' · ') : undefined;
  }

  function joinMeta(...parts: (string | null | undefined)[]): string | undefined {
    const filled = parts.filter((p): p is string => !!p);
    return filled.length > 0 ? filled.join(' · ') : undefined;
  }

  function occupiedLabel(deviceId: number): string | undefined {
    const name = occupied[deviceId];
    return name === undefined ? undefined : `уже в группе «${name}»`;
  }

  /** Раскрываемость свёртки — то же правило, что в форме акта. */
  function isDeviceGroupExpandable(g: DeviceGroup): boolean {
    if (g.ids.length <= 1) return false;
    return g.condition_distinct_count > 1 || !!g.repr.serial_no || !!g.repr.inventory_no;
  }

  function partitionMembers(members: DeviceDto[]): MemberRow[] {
    const rows: MemberRow[] = [];
    const subgroups = new Map<string | null, DeviceDto[]>();
    for (const d of members) {
      if (d.serial_no || d.inventory_no) {
        rows.push({ kind: 'instance', key: `d-${d.id}`, device: d });
      } else {
        const key = d.state ?? null;
        const list = subgroups.get(key) ?? [];
        list.push(d);
        subgroups.set(key, list);
      }
    }
    for (const [state, devs] of subgroups) {
      rows.push({ kind: 'subgroup', key: `sg-${state ?? '_'}`, state, devices: devs });
    }
    return rows;
  }

  // Dropdown зовёт это и при ручном drill-in, и при авто-раскрытии единственного
  // результата (AUTO-05). Единственный результат-группа раскрывается в одну
  // синтетическую строку «эта группа» — выбор вложит именно её.
  async function expandItem(item: SearchItem): Promise<MemberRow[]> {
    if (item.kind === 'group') return [{ kind: 'group', key: `g-${item.hit.id}`, hit: item.hit }];
    try {
      const list = await devices.listByIds(item.group.ids);
      void loadOccupancy(list.map((d) => d.id));
      return partitionMembers(list);
    } catch {
      return [];
    }
  }

  function itemName(item: SearchItem): string {
    return item.kind === 'device' ? item.group.repr.name : item.hit.name;
  }
  function itemMeta(item: SearchItem): string | undefined {
    if (item.kind === 'group') return item.hit.type_name;
    const g = item.group;
    if (g.ids.length === 1) return joinMeta(g.repr.model, occupiedLabel(g.ids[0]));
    const allBusy = g.ids.every((id) => occupied[id] !== undefined);
    return joinMeta(g.repr.model, allBusy ? 'все уже в группах' : undefined);
  }
  function itemSub(item: SearchItem): string | undefined {
    if (item.kind !== 'device' || item.group.ids.length !== 1) return undefined;
    return joinSnInv(item.group.repr.serial_no, item.group.repr.inventory_no);
  }
  function memberName(m: MemberRow): string {
    if (m.kind === 'instance') return m.device.name;
    if (m.kind === 'group') return m.hit.name;
    return `Без номера · ${m.state ?? '—'}`;
  }
  function memberMeta(m: MemberRow): string | undefined {
    if (m.kind === 'group') return m.hit.type_name;
    if (m.kind === 'subgroup') return `×${m.devices.length}`;
    return joinMeta(
      joinSnInv(m.device.serial_no, m.device.inventory_no),
      occupiedLabel(m.device.id),
    );
  }
  function memberSub(m: MemberRow): string | undefined {
    return m.kind === 'instance' ? (m.device.state ?? '—') : undefined;
  }

  /** Из набора одинаковых устройств берём первое, не помеченное занятым. */
  function firstFree(ids: number[]): number {
    return ids.find((id) => occupied[id] === undefined) ?? ids[0];
  }

  async function afterComposed(): Promise<void> {
    query = '';
    suggestions = [];
    await loadComposition();
    onChanged?.();
  }

  // Оба пути добавления устройств (поиск и модалка) идут в ОДИН серверный
  // обработчик groups.addDevices (D-01).
  async function addDevice(deviceId: number): Promise<void> {
    try {
      const result = await groups.addDevices({ group_id: group.id, device_ids: [deviceId] });
      notifyPlaceContentChanged(result.changed_place_ids);
      await afterComposed();
    } catch (e) {
      pushToast('error', errMessage(e, 'Не удалось добавить устройство.'));
    }
  }

  async function attachGroup(hit: GroupSearchHitDto): Promise<void> {
    try {
      // groups.search возвращает кандидата без version — берём свежую.
      const fresh = await groups.get(hit.id);
      const result = await groups.setParent({
        id: hit.id,
        version: fresh.version,
        parent_group_id: group.id,
      });
      notifyPlaceContentChanged(result.changed_place_ids);
      await afterComposed();
    } catch (e) {
      pushToast('error', errMessage(e, 'Не удалось вложить группу.'));
    }
  }

  function pickItem(item: SearchItem): void {
    if (item.kind === 'group') void attachGroup(item.hit);
    else void addDevice(firstFree(item.group.ids));
  }

  function pickMember(m: MemberRow): void {
    if (m.kind === 'group') void attachGroup(m.hit);
    else if (m.kind === 'instance') void addDevice(m.device.id);
    else void addDevice(firstFree(m.devices.map((d) => d.id)));
  }
</script>

{#snippet deviceRows(rows: GroupMemberDeviceDto[], ownerGroupId: number, nested: boolean)}
  {#each rows as d (d.device_id)}
    <TableRow indent={nested}>
      <td
        class="cell cell-kind"
        role="button"
        tabindex="0"
        onclick={() => onOpenDevice?.(d.device_id)}
        onkeydown={(e) => activateDevice(e, d)}
      >
        {d.type_name}
      </td>
      <td class="cell" title={d.name}>{d.name}</td>
      <td class="cell">
        {#if d.inventory_number || d.serial_number}
          {#if d.inventory_number}<span class="tr-mono line">{d.inventory_number}</span>{/if}
          {#if d.serial_number}<span class="tr-mono line">{d.serial_number}</span>{/if}
        {:else}
          —
        {/if}
      </td>
      <td class="cell" title={d.place_path ?? undefined}>{d.place_path_short ?? '—'}</td>
      <td class="cell">
        {#if d.status_name}
          <Badge variant={statusVariant(d.status_name)}>{d.status_name}</Badge>
        {:else}
          —
        {/if}
      </td>
      <td class="col-menu">
        {#if canEdit}
          <ActionMenu variant="ghost-sm" portal label={`Действия: ${d.name}`}>
            <button
              type="button"
              role="menuitem"
              onclick={() => void removeDevice(d.device_id, ownerGroupId)}
            >
              Вывести из состава
            </button>
          </ActionMenu>
        {/if}
      </td>
    </TableRow>
  {/each}
{/snippet}

{#snippet groupRows(children: GroupDto[], depth: number)}
  {#each children as child (child.id)}
    <TableRow
      group
      groupColspan={COLUMNS - 1}
      groupName={deviceCountLabel(child)}
      groupExpanded={!!expanded[child.id]}
      onToggleGroup={() => void toggleChild(child.id)}
      class={depth > 0 ? 'gc-nested' : ''}
    >
      <!-- Клик по меню не должен сворачивать/разворачивать строку. -->
      <td class="col-menu">
        <div class="menu-stop" role="presentation" onclick={(e) => e.stopPropagation()}>
          <ActionMenu variant="ghost-sm" portal label={`Действия: ${child.name}`}>
            <button type="button" role="menuitem" onclick={() => onOpenGroup(child.id)}>
              Открыть группу
            </button>
            {#if canEdit}
              <button type="button" role="menuitem" onclick={() => void detachGroup(child)}>
                Вывести из состава
              </button>
              <button type="button" role="menuitem" onclick={() => onRenameGroup(child.id)}>
                Переименовать
              </button>
            {/if}
          </ActionMenu>
        </div>
      </td>
    </TableRow>
    {#if expanded[child.id]}
      {@const state = childComp[child.id]}
      {#if !state || state.loading || !state.data}
        <tr class="gc-loading-row">
          <td colspan={COLUMNS}><Spinner size="sm" /> Загрузка…</td>
        </tr>
      {:else}
        {@render deviceRows(state.data.devices, child.id, true)}
        {@render groupRows(state.data.child_groups, depth + 1)}
      {/if}
    {/if}
  {/each}
{/snippet}

{#snippet addRow()}
  <div class="add-row">
    <div class="add-field">
      <Dropdown
        variant="combobox"
        value={query}
        placeholder="Добавить устройство или группу — название, инвентарный или серийный номер"
        loading={searching}
        groups={suggestions}
        getGroupId={(g: SearchItem) => g.key}
        getGroupName={itemName}
        getGroupMeta={itemMeta}
        getGroupSection={(g: SearchItem) =>
          g.kind === 'device' ? SECTION_DEVICES : SECTION_GROUPS}
        getGroupSub={itemSub}
        getGroupCount={(g: SearchItem) =>
          g.kind === 'device' ? g.group.count : g.hit.device_count}
        isGroupExpandable={(g: SearchItem) =>
          g.kind === 'device' && isDeviceGroupExpandable(g.group)}
        onExpandGroup={expandItem}
        getMemberId={(m: MemberRow) => m.key}
        getMemberName={memberName}
        getMemberMeta={memberMeta}
        getMemberSub={memberSub}
        onSearch={(q) => void fetchSuggestions(q)}
        onQueryInput={(v) => (query = v)}
        onPickGroup={pickItem}
        onPickMember={pickMember}
      />
    </div>
  </div>
{/snippet}

<div class="group-contents">
  <div class="table-region">
    <Table
      columns={COLUMNS}
      fillHeight
      framed={false}
      loading={loading && comp === null}
      empty={isEmpty}
      emptyTitle="Состав пуст"
      emptyBody="Добавьте устройства поиском ниже."
      footer={canEdit ? addRow : undefined}
    >
      {#snippet head()}
        <th>Тип</th>
        <th>Название</th>
        <th>Инв. № / Серийный №</th>
        <th>Место</th>
        <th>Статус</th>
        <th class="col-menu" aria-label="Действия"></th>
      {/snippet}
      {#if comp}
        {@render groupRows(comp.child_groups, 0)}
        {@render deviceRows(comp.devices, group.id, false)}
      {/if}
    </Table>
  </div>
</div>

<style lang="scss">
  .group-contents {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }

  .table-region {
    flex: 1 1 auto;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .cell-kind {
    color: var(--tr-text-secondary);
    font-size: var(--tr-font-size-caption);
  }

  .line {
    display: block;
  }

  .col-menu {
    width: 44px;
    text-align: right;
    white-space: nowrap;
  }

  .menu-stop {
    display: inline-flex;
  }

  .gc-loading-row td {
    padding: var(--tr-space-xs) var(--tr-space-md);
    color: var(--tr-text-tertiary);
    font-size: var(--tr-font-size-caption);
  }

  .add-row {
    display: flex;
    align-items: center;
    gap: var(--tr-space-sm);
  }

  .add-field {
    flex: 1 1 auto;
    min-width: 0;
  }

  // Вложенные группы глубже первого уровня: сдвиг имени, как у indent-строк
  // устройств (TableRow в group-режиме не принимает indent).
  :global(tr.gc-nested td.tr-row-group-name) {
    padding-left: 32px;
  }
</style>
