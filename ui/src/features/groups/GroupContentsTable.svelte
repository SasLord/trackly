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
  // Строки устройств повторяют «Устройства» (Phase 41.8, D-05): кебаб, правый
  // клик и клавиатура открывают ОДНО меню DeviceRowMenu, двойной клик/Enter —
  // просмотр. Одиночный клик по первой ячейке только фокусирует строку.
  //
  // Источник истины — сервер: «устройство уже в группе», цикл, teardown,
  // потолок пакета — отказ приходит с сервера; «уже в группе» показывается попапом
  // D-16 «Отменить / Переместить», прочие отказы — Toast. Пометка
  // «уже в группе «…»» — подсказка (одним запросом groups.forDevices на
  // результат поиска), выбор занятого устройства НЕ блокируется.
  import { onMount, untrack } from 'svelte';
  import Table from '$lib/components/Table.svelte';
  import TableRow from '$lib/components/TableRow.svelte';
  import Badge from '$lib/components/Badge.svelte';
  import ActionMenu from '$lib/components/ActionMenu.svelte';
  import Dropdown from '$lib/components/Dropdown.svelte';
  import Spinner from '@trackly/kit/components/Spinner.svelte';
  import Button from '@trackly/kit/components/Button.svelte';
  import GroupAddDevicesModal from './GroupAddDevicesModal.svelte';
  import DeviceFormModal from '../devices/DeviceFormModal.svelte';
  import DeviceRowMenu from '../devices/DeviceRowMenu.svelte';
  import AddToGroupModal from './AddToGroupModal.svelte';
  import WriteOffGroupModal from './WriteOffGroupModal.svelte';
  import AlreadyInGroupPopup from './AlreadyInGroupPopup.svelte';
  import {
    canReturnToTeardown,
    canWriteOff,
    isOwnKeyEvent,
    isRowOverlayEvent,
    STATUS_WRITTEN_OFF,
  } from '../devices/deviceRowMenu';
  import { groups } from '$lib/api/groups';
  import { devices } from '$lib/api/devices';
  import { pluralizeRu } from '@trackly/kit/utils/pluralize';
  import { pushToast } from '@trackly/kit/stores/toast.svelte';
  import { notifyPlaceContentChanged } from '$lib/stores/placeContentEvents.svelte';
  import { ensureLoaded, statusTone } from '$lib/stores/deviceStatuses.svelte';
  import { authStore } from '$lib/stores/auth.svelte';
  import { isAlreadyInGroup, parseAppError, type AppError } from '$lib/api/errors';
  import type {
    GroupAddDevicesResultDto,
    DeviceDto,
    DeviceGroup,
    GroupCompositionDto,
    GroupDto,
    GroupGhostDeviceDto,
    GroupMemberDeviceDto,
    GroupSearchHitDto,
  } from '../../bindings';

  interface Props {
    group: GroupDto;
    /** admin | manager: строка добавления, вывод/перенос из группы, «Переименовать». */
    canEdit: boolean;
    /** Страница увеличивает счётчик, чтобы заставить состав перечитаться. */
    refreshToken?: number;
    /** Состав изменился (добавили/вывели): страница обновляет дерево и карточку. */
    onChanged?: () => void;
    /** «Открыть группу»: страница фокусирует её узел в дереве. */
    onOpenGroup: (_groupId: number) => void;
    onRenameGroup: (_groupId: number) => void;
  }

  const {
    group,
    canEdit,
    refreshToken = 0,
    onChanged,
    onOpenGroup,
    onRenameGroup,
  }: Props = $props();

  const COLUMNS = 6;
  const role = $derived(authStore.user?.role ?? null);

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
      // Слияние: ребёнок, раскрытый во время перезагрузки (loading/data из toggleChild),
      // не теряется; отбрасываются только id, более не раскрытые.
      const merged: Record<number, { loading: boolean; data: GroupCompositionDto | null }> = {};
      for (const [k, v] of Object.entries({ ...childComp, ...fresh })) {
        if (expanded[Number(k)]) merged[Number(k)] = v;
      }
      childComp = merged;
    } catch (e) {
      if (seq === loadSeq) {
        if (parseAppError(e).code === 'NOT_FOUND') {
          // Группа исчезла (удалена здесь или во второй сессии): тишина, GroupPanel
          // получит NOT_FOUND в loadCard и сообщит странице через onGone.
          return;
        }
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

  const isTeardown = $derived(group.type_behavior === 'teardown');
  // Призраки (D-10): выведенные детали разбора; в счётчики не входят (D-11).
  const ghosts = $derived(isTeardown ? (comp?.ghosts ?? []) : []);

  const isEmpty = $derived(
    comp !== null &&
      comp.devices.length === 0 &&
      comp.child_groups.length === 0 &&
      ghosts.length === 0,
  );

  function deviceCountLabel(g: GroupDto): string {
    const word = pluralizeRu(g.device_count, ['устройство', 'устройства', 'устройств']);
    return `${g.name} (${g.device_count} ${word})`;
  }

  // ---- Отображение строки устройства ---------------------------------------
  // Тон бейджа решает statusTone(status_code) из стора статусов устройств
  // (Phase 41.1, D-09): ключ — машинный code, а не русское имя, поэтому
  // переименование статуса тон не теряет. Подпись приходит с сервера как есть.
  onMount(() => {
    void ensureLoaded();
  });

  // ---- Меню и жесты строк ----------------------------------------------------
  // Экземпляры меню и ячеек — по id устройства. Намеренно не $state: ссылки
  // нужны только обработчикам, перерисовку они не вызывают.
  const deviceMenus: Record<number, ReturnType<typeof DeviceRowMenu> | null> = {};
  const cellEls: Record<number, HTMLElement | null> = {};
  const groupMenus: Record<number, ReturnType<typeof ActionMenu> | null> = {};

  /** Правый клик: у курсора; событие от клавиши меню (координат нет) — у ячейки. */
  function openRowMenu(e: MouseEvent, id: number): void {
    const cell = cellEls[id];
    if (e.clientX === 0 && e.clientY === 0 && cell) {
      void deviceMenus[id]?.openFromElement(cell);
    } else {
      void deviceMenus[id]?.openAt(e.clientX, e.clientY);
    }
  }

  function openGroupMenu(e: MouseEvent, id: number): void {
    if (e.clientX === 0 && e.clientY === 0) {
      const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
      groupMenus[id]?.openAt(rect.left, rect.bottom);
    } else {
      groupMenus[id]?.openAt(e.clientX, e.clientY);
    }
  }

  /** Правка из меню строки (пункт «Редактировать»): форма устройства. */
  let editTarget = $state<DeviceDto | null>(null);

  async function handleEditSaved(result?: { placeId: number | null }): Promise<void> {
    editTarget = null;
    if (result?.placeId != null) notifyPlaceContentChanged([result.placeId]);
    await afterChange();
  }

  /** После любого действия: перечитать состав и сообщить странице (D-05). */
  async function afterChange(): Promise<void> {
    await loadComposition();
    onChanged?.();
  }

  // ---- Призраки разбора --------------------------------------------------------
  const ghostMenus: Record<number, ReturnType<typeof ActionMenu> | null> = {};
  const ghostCellEls: Record<number, HTMLElement | null> = {};

  function ghostCanReturn(g: GroupGhostDeviceDto): boolean {
    return (
      canEdit &&
      canReturnToTeardown({ role, isDeleted: g.is_deleted, statusCode: g.status_code ?? '' })
    );
  }

  /** Пометка причины в ячейке места (UI-SPEC §12.2). */
  function ghostPlaceNote(g: GroupGhostDeviceDto): string {
    if (g.is_deleted) return 'удалено';
    if (g.current_group_id !== null) return `выведено → ${g.current_group_name ?? ''}`;
    return g.place_path_short ? `выведено · ${g.place_path_short}` : 'выведено';
  }

  function openGhostMenu(e: MouseEvent, id: number): void {
    const cell = ghostCellEls[id];
    if (e.clientX === 0 && e.clientY === 0 && cell) {
      const rect = cell.getBoundingClientRect();
      ghostMenus[id]?.openAt(rect.left, rect.bottom);
    } else {
      ghostMenus[id]?.openAt(e.clientX, e.clientY);
    }
  }

  // ---- Попап «уже в группе» (D-16) ---------------------------------------------
  // Один попап на состав: «Переместить» выполняет run() (moveDevices / setParent
  // с replace_parent), ошибки run уходят в сам попап.
  interface OccupiedPopup {
    kind: 'device' | 'group' | 'detail';
    subjectName: string;
    currentGroupName: string;
    run: () => Promise<void>;
  }
  let occupiedPopup = $state<OccupiedPopup | null>(null);

  // Возврат выведенной детали (D-15): свободная — сразу, занятая — через попап.
  async function returnGhost(g: GroupGhostDeviceDto): Promise<void> {
    const moveHere = async (): Promise<void> => {
      const res = await groups.moveDevices({ group_id: group.id, device_ids: [g.device_id] });
      notifyPlaceContentChanged(res.changed_place_ids);
      pushToast('success', `Деталь возвращена в «${group.name}»`);
      occupiedPopup = null;
      query = '';
      await afterChange();
    };
    if (g.current_group_id !== null) {
      occupiedPopup = {
        kind: 'detail',
        subjectName: g.name,
        currentGroupName: g.current_group_name ?? '',
        run: moveHere,
      };
      return;
    }
    try {
      const res = await groups.addDevices({ group_id: group.id, device_ids: [g.device_id] });
      notifyPlaceContentChanged(res.changed_place_ids);
      pushToast('success', `Деталь возвращена в «${group.name}»`);
      query = '';
      await afterChange();
    } catch (e) {
      if (isAlreadyInGroup(e)) {
        occupiedPopup = {
          kind: 'detail',
          subjectName: g.name,
          currentGroupName: e.details.items[0]?.group_name ?? '',
          run: moveHere,
        };
        return;
      }
      pushToast(
        'error',
        errMessage(e, 'Не удалось вернуть деталь в разбор. Проверьте подключение и повторите.'),
      );
    }
  }

  /** Поиск возврата (D-15): только призраки этого разбора, без запросов к серверу. */
  const returnable = $derived(ghosts.filter((g) => ghostCanReturn(g)));
  const returnHits = $derived.by(() => {
    const q = query.trim().toLowerCase();
    if (q === '') return returnable;
    return returnable.filter((g) =>
      [g.name, g.inventory_number, g.serial_number].some((v) => v?.toLowerCase().includes(q)),
    );
  });

  function ghostHitMeta(g: GroupGhostDeviceDto): string | undefined {
    const where =
      g.current_group_id !== null
        ? `сейчас в группе «${g.current_group_name ?? ''}»`
        : g.place_path_short;
    return joinMeta(joinSnInv(g.serial_number, g.inventory_number), where);
  }

  // ---- Строка вложенной группы -------------------------------------------------
  // Вывод без подтверждения (D-02): обратим вводом. Вложение/вывод группы меняют
  // место её устройств — обязательна инвалидация счётчиков «Мест» (D-08, INV-7).
  async function detachGroup(child: GroupDto): Promise<void> {
    try {
      const result = await groups.setParent({
        id: child.id,
        version: child.version,
        parent_group_id: null,
      });
      notifyPlaceContentChanged(result.changed_place_ids);
      pushToast('success', `Группа «${child.name}» выведена из «${group.name}»`);
      await afterChange();
    } catch (e) {
      pushToast(
        'error',
        errMessage(e, 'Не удалось вывести из группы. Проверьте подключение и повторите.'),
      );
    }
  }

  let moveGroupTarget = $state<GroupDto | null>(null);
  let writeOffGroupTarget = $state<GroupDto | null>(null);

  async function handleGroupMoved(): Promise<void> {
    moveGroupTarget = null;
    await afterChange();
  }

  async function handleGroupWrittenOff(): Promise<void> {
    writeOffGroupTarget = null;
    await afterChange();
  }

  /** Раскрытие строки группы: один раз на двойной клик (Pitfall 8). */
  function onGroupRowToggle(e: MouseEvent | undefined, id: number): void {
    if (e) {
      if (isRowOverlayEvent(e.target)) return;
      // 1 — первый клик мышью, 0 — клавиатура; второй клик серии игнорируется.
      if (e.detail > 1) return;
    }
    void toggleChild(id);
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
        // D-17: группа типа «Разбор» целью вложения не предлагается.
        ...hits
          .filter((h) => h.type_behavior !== 'teardown')
          .map((h): SearchItem => ({ kind: 'group', key: `g-${h.id}`, hit: h })),
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
  async function addDevice(deviceId: number, subjectName: string): Promise<void> {
    try {
      const result = await groups.addDevices({ group_id: group.id, device_ids: [deviceId] });
      notifyPlaceContentChanged(result.changed_place_ids);
      await afterComposed();
    } catch (e) {
      // D-16: «уже в группе» различается по коду; вместо тоста — попап переноса.
      if (isAlreadyInGroup(e)) {
        occupiedPopup = {
          kind: 'device',
          subjectName,
          currentGroupName: e.details.items[0]?.group_name ?? '',
          run: async () => {
            const res = await groups.moveDevices({ group_id: group.id, device_ids: [deviceId] });
            notifyPlaceContentChanged(res.changed_place_ids);
            pushToast('success', `Устройство перемещено в группу «${group.name}»`);
            occupiedPopup = null;
            await afterComposed();
          },
        };
        return;
      }
      pushToast('error', errMessage(e, 'Не удалось добавить устройство.'));
    }
  }

  async function attachGroup(hit: GroupSearchHitDto): Promise<void> {
    // groups.search возвращает кандидата без version — берём свежую.
    const setParent = async (replaceParent: boolean) => {
      const fresh = await groups.get(hit.id);
      return groups.setParent({
        id: hit.id,
        version: fresh.version,
        parent_group_id: group.id,
        replace_parent: replaceParent,
      });
    };
    try {
      const result = await setParent(false);
      notifyPlaceContentChanged(result.changed_place_ids);
      await afterComposed();
    } catch (e) {
      if (isAlreadyInGroup(e)) {
        occupiedPopup = {
          kind: 'group',
          subjectName: hit.name,
          currentGroupName: e.details.items[0]?.group_name ?? '',
          run: async () => {
            const res = await setParent(true);
            notifyPlaceContentChanged(res.changed_place_ids);
            pushToast('success', `Группа «${hit.name}» перемещена в «${group.name}»`);
            occupiedPopup = null;
            await afterComposed();
          },
        };
        return;
      }
      pushToast('error', errMessage(e, 'Не удалось вложить группу.'));
    }
  }

  // Модалка мультивыбора (D-01, второй путь): groups.addDevices и инвалидация
  // счётчиков места выполняются внутри модалки; здесь — только обновить состав.
  let addModalOpen = $state(false);

  async function handleModalAdded(_result: GroupAddDevicesResultDto): Promise<void> {
    await loadComposition();
    onChanged?.();
  }

  // Фаза 41.1 (D-17/D-18/D-35), D-28: «Создать деталь» — форма устройства в
  // create-режиме; происхождение (origin_group_id) передаёт клиент, а деталь
  // вводит в состав сервер той же транзакцией, что и создание. Второго вызова
  // groups.addDevices нет (повторный ввод был бы ошибкой). Здесь — только
  // инвалидация счётчиков «Мест» по созданным устройствам и перечитывание состава.
  let detailModalOpen = $state(false);

  async function handleDetailSaved(result?: { created?: DeviceDto[] }): Promise<void> {
    detailModalOpen = false;
    const created = result?.created ?? [];
    if (created.length === 0) return;
    const createdPlaceIds = Array.from(
      new Set(created.map((d) => d.place_id).filter((id): id is number => id !== null)),
    );
    notifyPlaceContentChanged(createdPlaceIds);
    // Состав и счётчики перечитываются.
    await loadComposition();
    onChanged?.();
  }

  function pickItem(item: SearchItem): void {
    if (item.kind === 'group') void attachGroup(item.hit);
    else void addDevice(firstFree(item.group.ids), item.group.repr.name);
  }

  function pickMember(m: MemberRow): void {
    if (m.kind === 'group') void attachGroup(m.hit);
    else if (m.kind === 'instance') void addDevice(m.device.id, m.device.name);
    else void addDevice(firstFree(m.devices.map((d) => d.id)), m.devices[0].name);
  }
</script>

{#snippet deviceRows(rows: GroupMemberDeviceDto[], nested: boolean)}
  {#each rows as d (d.device_id)}
    {@const id = d.device_id}
    <TableRow
      indent={nested}
      inactive={d.status_code === STATUS_WRITTEN_OFF}
      oncontextmenu={(e) => {
        if (isRowOverlayEvent(e.target)) return;
        e.preventDefault();
        openRowMenu(e, id);
      }}
      ondblclick={(e) => {
        if (isRowOverlayEvent(e.target)) return;
        // Двойной клик по кебабу, ссылке или кнопке не открывает просмотр.
        if ((e.target as HTMLElement | null)?.closest?.('button, a')) return;
        window.getSelection()?.removeAllRanges();
        void deviceMenus[id]?.openView();
      }}
    >
      <td
        class="cell cell-kind"
        role="button"
        tabindex="0"
        aria-haspopup="menu"
        bind:this={cellEls[id]}
        onkeydown={(e) => {
          if (!isOwnKeyEvent(e.target, e.currentTarget)) return;
          if (e.key === 'Enter') {
            e.preventDefault();
            void deviceMenus[id]?.openView();
          } else if (e.key === 'ContextMenu' || (e.key === 'F10' && e.shiftKey)) {
            e.preventDefault();
            const cell = cellEls[id];
            if (cell) void deviceMenus[id]?.openFromElement(cell);
          }
        }}
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
      <td class="cell tr-cell-solid">
        {#if d.status_name}
          <Badge
            variant={statusTone(d.status_code).variant}
            appearance={statusTone(d.status_code).appearance}
            muted={statusTone(d.status_code).muted}>{d.status_name}</Badge
          >
        {:else}
          —
        {/if}
      </td>
      <td class="col-menu tr-cell-solid">
        <DeviceRowMenu
          bind:this={deviceMenus[id]}
          device={null}
          deviceId={id}
          trigger="none"
          onEdit={(dev) => (editTarget = dev)}
          onChanged={() => void afterChange()}
        />
        <button
          type="button"
          class="kebab"
          aria-haspopup="menu"
          aria-label={`Действия: ${d.name}`}
          onclick={(e) => void deviceMenus[id]?.openFromElement(e.currentTarget)}
        >
          <svg width="18" height="18" viewBox="0 0 18 18" aria-hidden="true">
            <circle cx="9" cy="3.5" r="1.5" fill="currentColor" />
            <circle cx="9" cy="9" r="1.5" fill="currentColor" />
            <circle cx="9" cy="14.5" r="1.5" fill="currentColor" />
          </svg>
        </button>
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
      onToggleGroup={(e) => onGroupRowToggle(e, child.id)}
      oncontextmenu={(e) => {
        if (isRowOverlayEvent(e.target)) return;
        e.preventDefault();
        openGroupMenu(e, child.id);
      }}
      class={depth > 0 ? 'gc-nested' : ''}
    >
      <!-- Клик по меню не должен сворачивать/разворачивать строку. -->
      <td class="col-menu">
        <div class="menu-stop" role="presentation" onclick={(e) => e.stopPropagation()}>
          <ActionMenu
            bind:this={groupMenus[child.id]}
            variant="ghost-sm"
            label={`Действия: ${child.name}`}
          >
            <button type="button" role="menuitem" onclick={() => onOpenGroup(child.id)}>
              Открыть группу
            </button>
            {#if canEdit}
              <!-- svelte-ignore a11y_no_redundant_roles -->
              <hr role="separator" />
              <button type="button" role="menuitem" onclick={() => void detachGroup(child)}>
                Вывести из группы
              </button>
              <button type="button" role="menuitem" onclick={() => (moveGroupTarget = child)}>
                Переместить в группу
              </button>
              <!-- svelte-ignore a11y_no_redundant_roles -->
              <hr role="separator" />
              <button type="button" role="menuitem" onclick={() => onRenameGroup(child.id)}>
                Переименовать
              </button>
              {#if canWriteOff({ role, statusCode: child.anchor_status_code ?? '' })}
                <!-- svelte-ignore a11y_no_redundant_roles -->
                <hr role="separator" />
                <button
                  type="button"
                  role="menuitem"
                  class="menu-danger"
                  onclick={() => (writeOffGroupTarget = child)}
                >
                  Списать
                </button>
              {/if}
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
        {@render deviceRows(state.data.devices, true)}
        {@render groupRows(state.data.child_groups, depth + 1)}
      {/if}
    {/if}
  {/each}
{/snippet}

{#snippet ghostRows(rows: GroupGhostDeviceDto[])}
  <tr class="ghost-divider">
    <td colspan={COLUMNS}>Выведенные детали · {rows.length}</td>
  </tr>
  {#each rows as g (g.device_id)}
    {@const id = g.device_id}
    <TableRow
      inactive
      oncontextmenu={(e) => {
        if (isRowOverlayEvent(e.target)) return;
        if (!ghostCanReturn(g)) return;
        e.preventDefault();
        openGhostMenu(e, id);
      }}
      ondblclick={(e) => {
        if (isRowOverlayEvent(e.target)) return;
        if (g.is_deleted) return;
        if ((e.target as HTMLElement | null)?.closest?.('button, a')) return;
        window.getSelection()?.removeAllRanges();
        void deviceMenus[id]?.openView();
      }}
    >
      <td
        class="cell cell-kind"
        role="button"
        tabindex="0"
        title={g.is_deleted ? 'Устройство удалено — вернуть нельзя' : undefined}
        bind:this={ghostCellEls[id]}
        onkeydown={(e) => {
          if (!isOwnKeyEvent(e.target, e.currentTarget)) return;
          if (e.key === 'Enter') {
            if (g.is_deleted) return;
            e.preventDefault();
            void deviceMenus[id]?.openView();
          } else if (e.key === 'ContextMenu' || (e.key === 'F10' && e.shiftKey)) {
            if (!ghostCanReturn(g)) return;
            e.preventDefault();
            const cell = ghostCellEls[id];
            if (cell) {
              const rect = cell.getBoundingClientRect();
              ghostMenus[id]?.openAt(rect.left, rect.bottom);
            }
          }
        }}
      >
        {g.type_name}
      </td>
      <td class="cell" title={g.is_deleted ? 'Устройство удалено — вернуть нельзя' : g.name}>
        {g.name}
      </td>
      <td class="cell">
        {#if g.inventory_number || g.serial_number}
          {#if g.inventory_number}<span class="tr-mono line">{g.inventory_number}</span>{/if}
          {#if g.serial_number}<span class="tr-mono line">{g.serial_number}</span>{/if}
        {:else}
          —
        {/if}
      </td>
      <td class="cell cell-note tr-cell-solid" title={g.place_path ?? undefined}>
        {ghostPlaceNote(g)}
      </td>
      <td class="cell tr-cell-solid">
        {#if !g.is_deleted && g.status_name}
          <Badge
            variant={statusTone(g.status_code).variant}
            appearance={statusTone(g.status_code).appearance}
            muted={statusTone(g.status_code).muted}>{g.status_name}</Badge
          >
        {:else}
          —
        {/if}
      </td>
      <td class="col-menu tr-cell-solid">
        {#if !g.is_deleted}
          <DeviceRowMenu
            bind:this={deviceMenus[id]}
            device={null}
            deviceId={id}
            trigger="none"
            onEdit={(dev) => (editTarget = dev)}
            onChanged={() => void afterChange()}
          />
        {/if}
        {#if ghostCanReturn(g)}
          <ActionMenu bind:this={ghostMenus[id]} variant="ghost-sm" label={`Действия: ${g.name}`}>
            <button type="button" role="menuitem" onclick={() => void returnGhost(g)}>
              Вернуть в разбор
            </button>
          </ActionMenu>
        {/if}
      </td>
    </TableRow>
  {/each}
{/snippet}

{#snippet addRow()}
  <div class="add-row">
    <div class="add-field">
      {#if isTeardown}
        <!-- D-15: в разборе ищутся только выведенные детали этого разбора. -->
        <Dropdown
          variant="combobox"
          value={query}
          placeholder="Вернуть выведенную деталь — название, инвентарный или серийный номер"
          loading={false}
          groups={returnHits}
          getGroupId={(g: GroupGhostDeviceDto) => g.device_id}
          getGroupName={(g: GroupGhostDeviceDto) => g.name}
          getGroupMeta={ghostHitMeta}
          getGroupCount={() => 1}
          isGroupExpandable={() => false}
          onExpandGroup={(): MemberRow[] => []}
          getMemberId={(m: MemberRow) => m.key}
          getMemberName={memberName}
          onSearch={() => {}}
          onQueryInput={(v) => (query = v)}
          onPickGroup={(g: GroupGhostDeviceDto) => void returnGhost(g)}
          onPickMember={() => {}}
        />
        {#if query.trim() !== '' && returnHits.length === 0}
          <p class="add-hint">
            Выведенных деталей нет. Новые детали добавляются кнопкой «Создать деталь».
          </p>
        {/if}
      {:else}
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
      {/if}
    </div>
    {#if !isTeardown}
      <Button variant="secondary" onclick={() => (addModalOpen = true)}>Добавить несколько…</Button>
    {/if}
    {#if isTeardown}
      <Button variant="secondary" onclick={() => (detailModalOpen = true)}>Создать деталь</Button>
    {/if}
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
      emptyTitle={isTeardown ? 'Деталей пока нет' : 'Состав пуст'}
      emptyBody={isTeardown
        ? 'Создайте первую деталь кнопкой «Создать деталь» ниже.'
        : 'Добавьте устройства поиском ниже.'}
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
        {@render deviceRows(comp.devices, false)}
        {#if ghosts.length > 0}
          {@render ghostRows(ghosts)}
        {/if}
      {/if}
    </Table>
  </div>
</div>

{#if addModalOpen}
  <GroupAddDevicesModal {group} onClose={() => (addModalOpen = false)} onAdded={handleModalAdded} />
{/if}

{#if detailModalOpen}
  <DeviceFormModal
    open={detailModalOpen}
    target={null}
    presetPlaceId={group.place_id}
    presetStatusCode="на_складе"
    originGroup={{ id: group.id, name: group.name }}
    onClose={() => (detailModalOpen = false)}
    onSaved={handleDetailSaved}
  />
{/if}

{#if occupiedPopup}
  <AlreadyInGroupPopup
    kind={occupiedPopup.kind}
    subjectName={occupiedPopup.subjectName}
    currentGroupName={occupiedPopup.currentGroupName}
    targetName={group.name}
    targetPlaceLabel={group.place_path}
    onConfirm={occupiedPopup.run}
    onCancel={() => (occupiedPopup = null)}
  />
{/if}

{#if editTarget}
  <DeviceFormModal
    open={true}
    target={editTarget}
    onClose={() => (editTarget = null)}
    onSaved={handleEditSaved}
  />
{/if}

{#if moveGroupTarget}
  <AddToGroupModal
    mode="move"
    subjectGroup={moveGroupTarget}
    currentGroup={{ id: group.id, name: group.name }}
    onClose={() => (moveGroupTarget = null)}
    onAdded={handleGroupMoved}
  />
{/if}

{#if writeOffGroupTarget}
  <WriteOffGroupModal
    group={writeOffGroupTarget}
    onClose={() => (writeOffGroupTarget = null)}
    onDone={handleGroupWrittenOff}
  />
{/if}

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

  .kebab {
    width: 28px;
    height: 28px;
    padding: 0;
    background: transparent;
    border: none;
    color: var(--tr-text-primary);
    border-radius: var(--tr-radius-sm);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;

    &:hover {
      background: var(--tr-surface-sunken);
    }
    &:focus-visible {
      outline: none;
      box-shadow: 0 0 0 3px var(--tr-focus-ring);
    }
  }

  .cell-note {
    font-size: var(--tr-font-size-caption);
    color: var(--tr-text-secondary);
  }

  .ghost-divider td {
    padding: var(--tr-space-xs) var(--tr-space-md);
    border-top: 1px solid var(--tr-border-strong);
    font-size: var(--tr-font-size-caption);
    color: var(--tr-text-secondary);
  }

  .add-hint {
    margin: var(--tr-space-xs) 0 0;
    font-size: var(--tr-font-size-caption);
    color: var(--tr-text-secondary);
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
