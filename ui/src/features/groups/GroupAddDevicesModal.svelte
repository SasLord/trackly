<script lang="ts">
  // Phase 41 Plan 21 (GRP-04, D-01, 41-UI-SPEC.md §11.3/§11.4): модальный
  // мультивыбор устройств для добавления в состав группы. Второй путь
  // добавления — первый (строка-поиск) живёт в GroupContentsTable; ОБА идут в
  // один серверный обработчик groups.addDevices.
  //
  // Монтируется родителем через {#if} (каждый показ — свежий экземпляр).
  //
  // Источник данных. Сервер умеет: devices.list — фильтры «Тип» и «Статус», без
  // текста; devices.search — текст (FTS), без фильтров. Фильтра по месту нет
  // нигде. Поэтому: с текстом — devices.search + «Тип»/«Статус» на клиенте;
  // без текста — devices.list с фильтрами на сервере; «Место» — всегда на
  // клиенте (поддерево по full_path выбранного места). Когда клиентский фильтр
  // активен, страницы подгружаются подряд, пока не наберётся порция (потолок
  // FETCH_CAP) — иначе страница с узким фильтром выглядела бы пустой.
  //
  // Правила занятости (устройство в одной группе, пакет до 500) — серверная
  // истина: пометка «Уже в группе «…»» — подсказка одним groups.forDevices на
  // порцию, отказ сервера показывается текстом в теле модалки.
  import { onDestroy, onMount } from 'svelte';
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';
  import Input from '$lib/components/Input.svelte';
  import Dropdown from '$lib/components/Dropdown.svelte';
  import Checkbox from '$lib/components/Checkbox.svelte';
  import Badge from '$lib/components/Badge.svelte';
  import Table from '$lib/components/Table.svelte';
  import TableRow from '$lib/components/TableRow.svelte';
  import PlacePicker from '$lib/components/PlacePicker.svelte';
  import { apiCall } from '$lib/api/client';
  import { groups } from '$lib/api/groups';
  import { devices } from '$lib/api/devices';
  import { notifyPlaceContentChanged } from '$lib/stores/placeContentEvents.svelte';
  import type { AppError } from '$lib/api/errors';
  import type { DeviceDto, GroupAddDevicesResultDto, GroupDto, PlaceDto } from '../../bindings';

  interface Props {
    group: GroupDto;
    onClose: () => void;
    onAdded: (_result: GroupAddDevicesResultDto) => void;
  }

  const { group, onClose, onAdded }: Props = $props();

  interface Option {
    id: number | null;
    label: string;
  }
  // Справочники те же, что у страницы «Устройства» (DeviceFilters / DeviceFormModal).
  const TYPE_OPTIONS: Option[] = [
    { id: null, label: 'Все типы' },
    { id: 1, label: 'Устройство' },
    { id: 2, label: 'Принтер' },
  ];
  const STATUS_OPTIONS: Option[] = [
    { id: null, label: 'Все статусы' },
    { id: 1, label: 'На складе' },
    { id: 2, label: 'В работе' },
    { id: 3, label: 'На ремонте' },
    { id: 4, label: 'Списано' },
  ];
  type BadgeVariant = 'default' | 'accent' | 'success' | 'warning' | 'destructive';
  const STATUS_VARIANTS: Record<number, BadgeVariant> = {
    1: 'default',
    2: 'accent',
    3: 'warning',
    4: 'destructive',
  };

  const PAGE = 100;
  const FETCH_CAP = 1000;
  const MIN_VISIBLE_PER_CALL = 30;
  const PLACE_PATH_SEPARATOR = ' / ';

  function errMessage(e: unknown, fallback: string): string {
    const err = e as Partial<AppError> | undefined;
    return err?.message ?? fallback;
  }

  // ---- Фильтры --------------------------------------------------------------
  let search = $state('');
  let typeId = $state<number | null>(null);
  let statusId = $state<number | null>(null);
  let placeId = $state<number | null>(null);
  let placePath = $state<string | null>(null);

  const typeLabel = $derived(TYPE_OPTIONS.find((o) => o.id === typeId)?.label ?? '');
  const statusLabel = $derived(STATUS_OPTIONS.find((o) => o.id === statusId)?.label ?? '');

  // ---- Данные ---------------------------------------------------------------
  let rows = $state<DeviceDto[]>([]);
  let offset = $state(0);
  let serverTotal = $state(0);
  let loading = $state(true);
  let loadingMore = $state(false);
  let loadErr = $state<string | null>(null);
  let fetchSeq = 0;
  let searchTimer: ReturnType<typeof setTimeout> | undefined;

  const hasMore = $derived(offset < serverTotal);

  /** device_id -> имя группы, где устройство уже состоит. */
  let occupied = $state<Record<number, string>>({});

  function clientFilterActive(): boolean {
    return placeId !== null || (search.trim() !== '' && (typeId !== null || statusId !== null));
  }

  function passes(d: DeviceDto): boolean {
    if (search.trim() !== '') {
      if (typeId !== null && d.type_id !== typeId) return false;
      if (statusId !== null && d.status_id !== statusId) return false;
    }
    if (placeId !== null) {
      const full = d.full_path;
      if (!placePath || !full) return false;
      if (full !== placePath && !full.startsWith(placePath + PLACE_PATH_SEPARATOR)) return false;
    }
    return true;
  }

  async function loadOccupancy(ids: number[]): Promise<void> {
    if (ids.length === 0) return;
    try {
      const memberships = await groups.forDevices(ids);
      const next = { ...occupied };
      for (const id of ids) delete next[id];
      for (const m of memberships) next[m.device_id] = m.group_name;
      occupied = next;
      // Устройство, которое оказалось занятым, выбранным оставаться не может.
      if ([...selected].some((id) => next[id] !== undefined)) {
        selected = new Set([...selected].filter((id) => next[id] === undefined));
      }
    } catch {
      // Подсказка необязательна: отказ придёт с сервера при добавлении.
    }
  }

  async function loadPage(reset: boolean): Promise<void> {
    const seq = ++fetchSeq;
    if (reset) {
      rows = [];
      offset = 0;
      serverTotal = 0;
      loading = true;
    } else {
      loadingMore = true;
    }
    loadErr = null;
    let addedThisCall = 0;
    try {
      for (;;) {
        const q = search.trim();
        const pagination = { offset, limit: PAGE };
        const res = q
          ? await devices.search(q, pagination)
          : await devices.list(
              {
                type_id: typeId,
                place_id: null,
                status_id: statusId,
                state: null,
                name_prefix: null,
                include_deleted: false,
                group_by_condition: false,
              },
              pagination,
            );
        if (seq !== fetchSeq) return;
        serverTotal = res.total;
        offset += res.items.length;
        const add = res.items.filter(passes);
        rows = [...rows, ...add];
        addedThisCall += add.length;
        void loadOccupancy(add.map((d) => d.id));
        if (
          !clientFilterActive() ||
          res.items.length === 0 ||
          offset >= serverTotal ||
          offset >= FETCH_CAP ||
          addedThisCall >= MIN_VISIBLE_PER_CALL
        ) {
          break;
        }
      }
    } catch (e) {
      if (seq === fetchSeq) loadErr = errMessage(e, 'Не удалось загрузить устройства.');
    } finally {
      if (seq === fetchSeq) {
        loading = false;
        loadingMore = false;
      }
    }
  }

  onMount(() => {
    void loadPage(true);
  });
  onDestroy(() => {
    if (searchTimer) clearTimeout(searchTimer);
  });

  function onSearchInput(v: string): void {
    search = v;
    if (searchTimer) clearTimeout(searchTimer);
    searchTimer = setTimeout(() => void loadPage(true), 250);
  }

  function pickType(o: Option): void {
    typeId = o.id;
    void loadPage(true);
  }
  function pickStatus(o: Option): void {
    statusId = o.id;
    void loadPage(true);
  }

  async function onPlaceChange(id: number | null): Promise<void> {
    placeId = id;
    placePath = null;
    if (id !== null) {
      try {
        const place = await apiCall<PlaceDto>('places_get', { id });
        // Пользователь мог уже выбрать другое место, пока шёл запрос.
        if (placeId !== id) return;
        placePath = place.full_path;
      } catch (e) {
        loadErr = errMessage(e, 'Не удалось определить место.');
        return;
      }
    }
    void loadPage(true);
  }

  const noExpand = (): never[] => [];

  // ---- Выбор ----------------------------------------------------------------
  let selected = $state<Set<number>>(new Set());
  let busy = $state(false);
  let serverErr = $state<string | null>(null);

  const selectedCount = $derived(selected.size);

  function toggle(id: number, checked: boolean): void {
    const next = new Set(selected);
    if (checked) next.add(id);
    else next.delete(id);
    selected = next;
    serverErr = null;
  }

  // Тот же серверный обработчик, что и у строки-поиска состава (D-01).
  async function submit(): Promise<void> {
    if (selected.size === 0 || busy) return;
    busy = true;
    serverErr = null;
    try {
      const result = await groups.addDevices({
        group_id: group.id,
        device_ids: Array.from(selected),
      });
      notifyPlaceContentChanged(result.changed_place_ids);
      onAdded(result);
      onClose();
    } catch (e) {
      serverErr = errMessage(e, 'Не удалось добавить устройства.');
    } finally {
      busy = false;
    }
  }
</script>

<Modal open={true} title={`Добавить устройства в «${group.name}»`} size="wide" {onClose}>
  <div class="add-devices">
    <div class="filters">
      <div class="filter filter-search">
        <label class="form-label" for="gad-search">Поиск</label>
        <Input
          id="gad-search"
          type="search"
          value={search}
          oninput={onSearchInput}
          placeholder="Название, инвентарный или серийный номер"
        />
      </div>
      <div class="filter">
        <label class="form-label" for="gad-type">Тип</label>
        <Dropdown
          id="gad-type"
          variant="select"
          flat={true}
          value={typeLabel}
          placeholder="Все типы"
          searchable={false}
          loading={false}
          groups={TYPE_OPTIONS}
          getGroupId={(o) => String(o.id)}
          getGroupName={(o) => o.label}
          getGroupCount={() => 0}
          isGroupExpandable={() => false}
          isGroupSelected={(o) => o.id === typeId}
          onExpandGroup={noExpand}
          getMemberId={(o) => String(o)}
          getMemberName={(o) => String(o)}
          onSearch={() => {}}
          onPickGroup={pickType}
          onPickMember={() => {}}
        />
      </div>
      <div class="filter">
        <label class="form-label" for="gad-status">Статус</label>
        <Dropdown
          id="gad-status"
          variant="select"
          flat={true}
          value={statusLabel}
          placeholder="Все статусы"
          searchable={false}
          loading={false}
          groups={STATUS_OPTIONS}
          getGroupId={(o) => String(o.id)}
          getGroupName={(o) => o.label}
          getGroupCount={() => 0}
          isGroupExpandable={() => false}
          isGroupSelected={(o) => o.id === statusId}
          onExpandGroup={noExpand}
          getMemberId={(o) => String(o)}
          getMemberName={(o) => String(o)}
          onSearch={() => {}}
          onPickGroup={pickStatus}
          onPickMember={() => {}}
        />
      </div>
      <div class="filter">
        <label class="form-label" for="gad-place">Место</label>
        <PlacePicker id="gad-place" value={placeId} onChange={(id) => void onPlaceChange(id)} />
      </div>
    </div>

    {#if loadErr}
      <p class="inline-error" role="alert">{loadErr}</p>
    {/if}

    <div class="table-region">
      <Table
        columns={5}
        {loading}
        empty={!loading && rows.length === 0 && loadErr === null}
        emptyTitle="Ничего не найдено"
        emptyBody="Измените фильтры или запрос."
      >
        {#snippet head()}
          <th class="col-check"><span class="visually-hidden">Выбрать</span></th>
          <th>Название</th>
          <th>Инв. № / Серийный №</th>
          <th>Место</th>
          <th>Статус</th>
        {/snippet}
        {#each rows as d (d.id)}
          {@const busyIn = occupied[d.id]}
          {@const note = busyIn === undefined ? undefined : `Уже в группе «${busyIn}»`}
          <TableRow class={busyIn === undefined ? '' : 'row-occupied'}>
            <td class="col-check" title={note}>
              <Checkbox
                checked={selected.has(d.id)}
                disabled={busyIn !== undefined || busy}
                onchange={(v) => toggle(d.id, v)}
              >
                <span class="visually-hidden">Выбрать: {d.name}</span>
              </Checkbox>
            </td>
            <td class="cell" class:occupied={busyIn !== undefined} title={note ?? d.name}>
              {d.name}
            </td>
            <td class="cell" class:occupied={busyIn !== undefined}>
              {#if d.inventory_no || d.serial_no}
                {#if d.inventory_no}<span class="tr-mono line">{d.inventory_no}</span>{/if}
                {#if d.serial_no}<span class="tr-mono line">{d.serial_no}</span>{/if}
              {:else}
                —
              {/if}
            </td>
            <td class="cell" class:occupied={busyIn !== undefined} title={d.full_path ?? undefined}>
              {d.place_path_short ?? '—'}
            </td>
            <td class="cell">
              <Badge variant={STATUS_VARIANTS[d.status_id] ?? 'default'}>
                {STATUS_OPTIONS.find((o) => o.id === d.status_id)?.label ?? `Статус ${d.status_id}`}
              </Badge>
            </td>
          </TableRow>
        {/each}
      </Table>
      {#if hasMore && !loading}
        <div class="more-row">
          <Button variant="secondary" loading={loadingMore} onclick={() => void loadPage(false)}>
            Показать ещё
          </Button>
        </div>
      {/if}
    </div>

    {#if serverErr}
      <p class="inline-error" role="alert">{serverErr}</p>
    {/if}
  </div>

  {#snippet footer()}
    <span class="selected-count" aria-live="polite">
      Выбрано: <span class="tr-mono">{selectedCount}</span>
    </span>
    <Button variant="secondary" onclick={onClose} disabled={busy}>Отмена</Button>
    <Button
      variant="primary"
      loading={busy}
      disabled={selectedCount === 0 || busy}
      onclick={() => void submit()}
    >
      Добавить ({selectedCount})
    </Button>
  {/snippet}
</Modal>

<style lang="scss">
  .add-devices {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-md);
  }

  .filters {
    display: grid;
    grid-template-columns: minmax(0, 2fr) repeat(3, minmax(0, 1fr));
    gap: var(--tr-space-sm);
    align-items: end;
  }

  .filter {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-2xs);
    min-width: 0;
  }

  .form-label {
    font-size: var(--tr-font-size-label);
    font-weight: var(--tr-font-weight-medium);
    color: var(--tr-text-secondary);
  }

  .table-region {
    max-height: 52vh;
    overflow-y: auto;
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-sm);
  }

  .col-check {
    width: 44px;
  }

  .line {
    display: block;
  }

  .occupied {
    color: var(--tr-text-disabled);
  }

  .more-row {
    display: flex;
    justify-content: center;
    padding: var(--tr-space-sm);
    border-top: 1px solid var(--tr-border);
  }

  .selected-count {
    margin-right: auto;
    color: var(--tr-text-secondary);
    font-size: var(--tr-font-size-body);
  }

  .inline-error {
    margin: 0;
    color: var(--tr-danger);
    font-size: var(--tr-font-size-body);
  }

  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }
</style>
