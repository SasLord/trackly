<script lang="ts">
  // Plan 03-02: inline-editable таблица позиций в ActFormModal.
  //
  // DeviceAutocompleteField возвращает только строку (имя устройства), а нам нужен
  // device_id. Для этого в каждой строке мы используем встроенный поиск через
  // `devices.search(query, pagination)` (FTS5 search), фильтруем локально по
  // status_id=1 («на складе») и показываем dropdown с устройствами.
  //
  // Каждая позиция: { device_id, quantity, device_label } где device_label —
  // human-readable (name + inv_no), нужный только для UI.
  import { onDestroy } from 'svelte';
  import Button from '$lib/components/Button.svelte';
  import Spinner from '$lib/components/Spinner.svelte';
  import { devices } from '$lib/api/devices';
  import { portal } from '$lib/utils/portal';
  import { dropdownAnchor } from '$lib/utils/dropdownAnchor';
  import type { DeviceDto, DeviceGroup } from '../../bindings';

  export interface FormItemRow {
    device_id: number | null;
    quantity: number;
    /** Human-readable label for the picked device (name + inv_no). */
    device_label: string;
    /** Search query — what the user is typing. */
    query: string;
    /** True if user picked a device — disables further suggestions until cleared. */
    picked: boolean;
    /** W-5 (Phase 3.1 Plan 04): true if picked device has non-null serial_number.
     *  Backend всё равно clones (с serial=NULL у клонов), но UX-уровень
     *  блокирует qty>1 для серийных. */
    has_serial?: boolean;
    /** UAT Fix #3/#4 (Phase 3.1): кол-во девайсов в одной "группе" (одинаковые
     *  name + model + inventory_number IS NULL + status='на_складе') которые
     *  могут быть использованы для этой позиции. qty input bounded к этому
     *  значению. Для серийных устройств = 1. */
    stock_available?: number;
    /** UAT Fix #3/#4: все device_ids в группе (для backend submit без cloning). */
    group_ids?: number[];
    /** Plan 19-05 (ACT-02): комплектация на момент акта (act_items.complectation_at_time).
     *  Only ever populated by ActFormBody's edit-mode prefill (itemsFromInitialAct) — its
     *  presence (not its value) is what distinguishes a RETAINED position from a row
     *  freshly added during this edit session, which never has this field set. */
    complectation_at_time?: string | null;
  }

  // G-3 / T-03.1-02 mirror: backend MAX_CLONE_QTY = 1000.
  const MAX_CLONE_QTY = 1000;

  /** Resolve max qty bound для row: 1 для serial, stock_available если
   *  известен, иначе MAX_CLONE_QTY (как fallback до выбора device). */
  function qtyMax(row: FormItemRow): number {
    if (row.has_serial) return 1;
    if (typeof row.stock_available === 'number' && row.stock_available > 0) {
      return Math.min(row.stock_available, MAX_CLONE_QTY);
    }
    return MAX_CLONE_QTY;
  }

  interface Props {
    items: FormItemRow[];
    fieldErrors: Record<string, string>;
    mode?: 'create' | 'edit';
    onChange: (_items: FormItemRow[]) => void;
  }

  const { items, fieldErrors, mode = 'create', onChange }: Props = $props();

  /** Plan 19-05: complectation_at_time is only ever set by ActFormBody's edit-mode
   *  prefill — a fresh row (added during THIS edit session) never has it. */
  function handleComplectationInput(idx: number, v: string) {
    const next = items.map((it, i) =>
      i === idx ? { ...it, complectation_at_time: v } : it,
    );
    onChange(next);
  }

  // Per-row search state — keyed by row index. Reset when the row is mutated.
  // UAT Fix #3: changed DeviceDto[] → DeviceGroup[] чтобы dropdown показывал
  // ОДНУ запись на группу (name+model+inv_no=NULL) с count badge, а не 20
  // одинаковых клонов.
  let suggestionsByRow = $state<Record<number, DeviceGroup[]>>({});
  let loadingByRow = $state<Record<number, boolean>>({});
  let openByRow = $state<Record<number, boolean>>({});
  // Plan 18-04 (AUTO-01): raw <input> refs per-row — Input.svelte не поддерживает
  // bind:this (нет ref-forwarding), а use:dropdownAnchor нужен реальный anchorEl.
  let rowInputEls = $state<Record<number, HTMLInputElement | null>>({});
  let rowDropdownEls = $state<Record<number, HTMLUListElement | null>>({});
  // Plan 18-04 (AUTO-02/AUTO-03): индекс активного (клавиатурного) элемента
  // дропдауна по строке; -1 = нет активного.
  let activeIndexByRow = $state<Record<number, number>>({});
  const debounceTimers: Record<number, ReturnType<typeof setTimeout>> = {};

  // WR-05: removeRow() очищает debounceTimers[idx] удаляемой строки, но при
  // размонтировании ВСЕЙ таблицы (модал закрыт) прочие ещё pending таймеры
  // не отменялись — компонент мог размонтироваться и таймеры всё равно
  // issue-или бы API-запрос, записывая результат в $state уже мёртвого
  // компонента.
  onDestroy(() => {
    for (const key of Object.keys(debounceTimers)) {
      clearTimeout(debounceTimers[Number(key)]);
    }
  });

  // Plan 18-05 (AUTO-04/D-06/D-07 + AUTO-05/D-09): drill-in view-mode per row.
  // 'groups' — список групп (Plan 18-04 поведение); 'members' — раскрытая
  // группа (клик по раскрываемой группе ИЛИ auto-flatten единственной группы).
  let viewModeByRow = $state<Record<number, 'groups' | 'members'>>({});
  let drillGroupByRow = $state<Record<number, DeviceGroup | null>>({});
  let membersByRow = $state<Record<number, DeviceDto[]>>({});
  // Plan 18-05 (checkpoint fix #1): в member-view sticky-заголовок с названием
  // группы показывается ВСЕГДА (в т.ч. при auto-flatten единственной группы),
  // но кнопка «← Назад» — только когда пользователь пришёл кликом по группе
  // (showBackByRow=true), а не auto-flatten (false).
  let showBackByRow = $state<Record<number, boolean>>({});

  type MemberRow =
    | { kind: 'instance'; key: string; device: DeviceDto }
    | { kind: 'subgroup'; key: string; state: string | null; devices: DeviceDto[] };

  function makeEmpty(): FormItemRow {
    return { device_id: null, quantity: 1, device_label: '', query: '', picked: false };
  }

  function addRow() {
    onChange([...items, makeEmpty()]);
  }

  /** WR-01: индекс-ключевые transient-мапы дропдауна (10 штук) должны следовать
   *  за сдвигом строк при удалении — иначе после removeRow(idx) все строки
   *  после idx показывают состояние (открытый дропдаун/drill-in) ПРЕДЫДУЩЕГО
   *  жильца этого индекса. shift() удаляет запись под idx и сдвигает все
   *  записи с ключом > idx на -1, сохраняя записи с ключом < idx нетронутыми. */
  function shiftRowState<T>(m: Record<number, T>, idx: number): Record<number, T> {
    const out: Record<number, T> = {};
    for (const k of Object.keys(m)) {
      const i = Number(k);
      if (i < idx) out[i] = m[i];
      else if (i > idx) out[i - 1] = m[i];
    }
    return out;
  }

  function removeRow(idx: number) {
    const next = items.filter((_, i) => i !== idx);
    // WR-05: удаляемая строка могла иметь pending debounce-таймер — если его
    // не отменить, поздний fetch запишет результат в реиндексированные мапы
    // сдвинутой строки (stale write).
    if (debounceTimers[idx]) clearTimeout(debounceTimers[idx]);
    delete debounceTimers[idx];
    suggestionsByRow = shiftRowState(suggestionsByRow, idx);
    loadingByRow = shiftRowState(loadingByRow, idx);
    openByRow = shiftRowState(openByRow, idx);
    viewModeByRow = shiftRowState(viewModeByRow, idx);
    drillGroupByRow = shiftRowState(drillGroupByRow, idx);
    membersByRow = shiftRowState(membersByRow, idx);
    activeIndexByRow = shiftRowState(activeIndexByRow, idx);
    showBackByRow = shiftRowState(showBackByRow, idx);
    onChange(next);
  }

  function handleQueryInput(idx: number, v: string) {
    // Plan 18-05 (UI-SPEC "изменение текста фильтра сбрасывает view-mode строки
    // обратно к списку групп"): любое изменение ввода прерывает drill-in/flatten.
    viewModeByRow[idx] = 'groups';
    drillGroupByRow[idx] = null;
    membersByRow[idx] = [];
    showBackByRow[idx] = false;

    const next = items.map((it, i) =>
      i === idx ? { ...it, query: v, picked: false, device_id: null, device_label: '' } : it,
    );
    onChange(next);

    // AUTO-03: пустой ввод теперь валиден — backend (Plan 18-01) возвращает
    // top-20-по-остатку при пустом name_prefix, ранний return убран.
    if (debounceTimers[idx]) clearTimeout(debounceTimers[idx]);
    debounceTimers[idx] = setTimeout(() => {
      void fetchGroups(idx, v.trim());
    }, 250);
  }

  /** AUTO-02/AUTO-03: общая fetch-логика, переиспользуемая и debounced-веткой
   *  ввода (handleQueryInput), и focus-веткой (handleFocus, delay 0). Дропдаун
   *  остаётся ОТКРЫТЫМ даже при нуле совпадений — рендерит empty-state вместо
   *  закрытия (UI-SPEC Copywriting Contract «Ничего не найдено»). */
  async function fetchGroups(idx: number, query: string) {
    loadingByRow[idx] = true;
    let filtered: DeviceGroup[] = [];
    try {
      // UAT Fix #3/#4: listGrouped возвращает группы (одинаковые
      // name+model+inv_no=NULL) с count + ids. Filter status_id=1 (на_складе).
      // group_by_condition: true — сохраняет DEF-2B разбивку по condition (ITEM-1).
      const groups = await devices.listGrouped(
        {
          type_id: null,
          location_id: null,
          status_id: 1,
          state: null,
          name_prefix: query,
          include_deleted: false,
          group_by_condition: true,
        },
        { offset: 0, limit: 20 },
      );
      // DEF-2A: exclude groups whose IDs overlap with already-picked rows.
      const selectedIds = getSelectedIds(idx);
      filtered = groups.filter((g) => !g.ids.some((id) => selectedIds.has(id)));
      suggestionsByRow[idx] = filtered;
      activeIndexByRow[idx] = -1;
      openByRow[idx] = true;
    } catch {
      suggestionsByRow[idx] = [];
      activeIndexByRow[idx] = -1;
      openByRow[idx] = true;
      filtered = [];
    } finally {
      loadingByRow[idx] = false;
    }

    // Plan 18-05 Task 2 (AUTO-05/D-09): если после фильтрации осталась ровно
    // одна группа — она НЕ отображается как строка группы, а сразу
    // разворачивается в плоский member-список с sticky-заголовком группы, но
    // БЕЗ кнопки «← Назад» (showBack=false — пользователь не «нырял» вручную).
    if (filtered.length === 1) {
      await drillInto(idx, filtered[0], false);
    } else {
      viewModeByRow[idx] = 'groups';
      drillGroupByRow[idx] = null;
      membersByRow[idx] = [];
      showBackByRow[idx] = false;
    }
  }

  /** Раскрываемость группы (checkpoint fix #4 + D-08):
   *  - если в группе ровно один экземпляр (`ids.length === 1`) — НЕ раскрывается,
   *    клик сразу выбирает это устройство (независимо от serial/inventory);
   *  - иначе раскрывается только при смешанном condition ИЛИ наличии
   *    серийного/инвентарного номера у представителя (несерийные с одним
   *    condition — D-08 прямой clone-выбор). */
  function isExpandable(g: DeviceGroup): boolean {
    if (g.ids.length <= 1) return false;
    return g.condition_distinct_count > 1 || !!g.repr.serial_no || !!g.repr.inventory_no;
  }

  /** AUTO-04/D-06: клик по раскрываемой группе — заменяет список группами на
   *  её экземпляры (devices.listByIds), не закрывая дропдаун. showBack
   *  различает ручной drill-in (true → кнопка «← Назад») и AUTO-05
   *  auto-flatten (false → sticky-заголовок группы без «← Назад»). */
  async function drillInto(idx: number, g: DeviceGroup, showBack: boolean = true) {
    const selectedIds = getSelectedIds(idx);
    const ids = g.ids.filter((id) => !selectedIds.has(id));
    loadingByRow[idx] = true;
    try {
      membersByRow[idx] = await devices.listByIds(ids);
    } catch {
      membersByRow[idx] = [];
    } finally {
      loadingByRow[idx] = false;
    }
    drillGroupByRow[idx] = g;
    viewModeByRow[idx] = 'members';
    showBackByRow[idx] = showBack;
  }

  /** Клик по строке группы: раскрываемая группа → drill-in; иначе (D-08) —
   *  прямой clone-выбор через существующий pickGroup (без изменений). */
  function handleGroupClick(idx: number, g: DeviceGroup) {
    if (isExpandable(g)) {
      void drillInto(idx, g);
    } else {
      pickGroup(idx, g);
    }
  }

  /** D-06: кнопка «← Назад» — возврат от member-списка к списку групп. */
  function backToGroups(idx: number) {
    viewModeByRow[idx] = 'groups';
    drillGroupByRow[idx] = null;
    membersByRow[idx] = [];
    showBackByRow[idx] = false;
  }

  /** D-07: партиционирует member-список раскрытой/схлопнутой группы на
   *  отдельные строки серийных/инвентарных экземпляров и client-side
   *  под-группы по state для несерийных/безынвентарных. Инстансы идут первыми
   *  (порядок из devices.listByIds), затем под-группы (порядок вставки в Map)
   *  — соответствует ASCII-макету UI-SPEC. */
  function memberRows(idx: number): MemberRow[] {
    const members = membersByRow[idx] ?? [];
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

  /** D-07: выбор устройства из drill-in/flatten member-списка — зеркалит
   *  присваивания pickGroup() в items[idx], но источник — конкретный
   *  DeviceDto (не DeviceGroup.repr) + явный набор group_ids (id одного
   *  экземпляра ИЛИ id'ы под-группы по state). Количество (для несерийных)
   *  правится позже в колонке «Количество» таблицы — здесь клик только
   *  выбирает устройство (checkpoint fix #2: спиннер убран из дропдауна). */
  function pickDevice(idx: number, d: DeviceDto, groupIds: number[]) {
    const hasSerial = !!d.serial_no || !!d.inventory_no;
    const label = d.serial_no
      ? d.inventory_no
        ? `${d.name} (SN ${d.serial_no}, инв. ${d.inventory_no})`
        : `${d.name} (SN ${d.serial_no})`
      : d.inventory_no
        ? `${d.name} (инв. ${d.inventory_no})`
        : `${d.name}${d.model ? ` · ${d.model}` : ''} ×${groupIds.length}`;
    const next = items.map((it, i) =>
      i === idx
        ? {
            ...it,
            device_id: d.id,
            device_label: label,
            query: label,
            picked: true,
            has_serial: hasSerial,
            // Серийный/инвентарный экземпляр — qty жёстко 1; несерийная
            // под-группа — clamp текущего qty к размеру под-группы (правится
            // в колонке «Количество», как pickGroup).
            // WR-02 (Plan 19-08): в edit-режиме добавляемая строка — ровно
            // одно устройство (ActUpdateItemDto не несёт quantity/device_ids),
            // поэтому qty жёстко клампится к 1 независимо от размера группы.
            quantity: hasSerial || mode === 'edit' ? 1 : Math.min(it.quantity, groupIds.length),
            stock_available: groupIds.length,
            group_ids: groupIds,
          }
        : it,
    );
    onChange(next);
    suggestionsByRow[idx] = [];
    openByRow[idx] = false;
    activeIndexByRow[idx] = -1;
    viewModeByRow[idx] = 'groups';
    drillGroupByRow[idx] = null;
    membersByRow[idx] = [];
    showBackByRow[idx] = false;
  }

  /** AUTO-02/D-03: фокус на поле открывает список немедленно (delay 0), без
   *  ввода текста — реплицирует LocationAutocomplete.handleFocus. */
  function handleFocus(idx: number) {
    if (debounceTimers[idx]) clearTimeout(debounceTimers[idx]);
    void fetchGroups(idx, (items[idx]?.query ?? '').trim());
  }

  /** AUTO-02: клавиатурная навигация по дропдауну строки — реплицирует
   *  LocationAutocomplete.handleKeydown, адаптировано под per-row state. */
  function handleRowKeydown(idx: number, e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      openByRow[idx] = false;
      return;
    }
    if (e.key === 'ArrowDown' && !openByRow[idx]) {
      e.preventDefault();
      handleFocus(idx);
      return;
    }
    if (!openByRow[idx]) return;
    // Plan 18-05: клавиатурная навигация ArrowUp/Down/Enter/Tab этой функции
    // адресована списку групп (visibleGroups); в member-режиме (drill-in /
    // AUTO-05 auto-flatten) рендерится другой список строк (инстансы +
    // под-группы по state, часть с инлайн-инпутом количества) — применение
    // group-навигации здесь выбрало бы неверный элемент (Rule 1 bug guard).
    // «← Назад» (Escape/клик) остаётся доступным через backToGroups().
    if (viewModeByRow[idx] === 'members') {
      // WR-02: в groups-режиме открытый дропдаун глотает Enter через
      // preventDefault()/stopPropagation() (ветка ниже). В member/drill-in
      // режиме навигация обрабатывается кликом (нет ArrowUp/Down-выбора),
      // но Enter должен ТАК ЖЕ подавляться, иначе он всплывает к native
      // <form> submit прямо во время выбора устройства в раскрытой группе.
      if (e.key === 'Enter') {
        e.preventDefault();
        e.stopPropagation();
      }
      return;
    }
    const list = visibleGroups(idx);
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (list.length === 0) return;
      const cur = activeIndexByRow[idx] ?? -1;
      activeIndexByRow[idx] = (cur + 1) % list.length;
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (list.length === 0) return;
      const cur = activeIndexByRow[idx] ?? -1;
      activeIndexByRow[idx] = cur <= 0 ? list.length - 1 : cur - 1;
    } else if (e.key === 'Enter') {
      const cur = activeIndexByRow[idx] ?? -1;
      if (cur >= 0 && cur < list.length) {
        e.preventDefault();
        e.stopPropagation();
        pickGroup(idx, list[cur]);
      }
    } else if (e.key === 'Tab') {
      const cur = activeIndexByRow[idx] ?? -1;
      if (cur >= 0 && cur < list.length) {
        pickGroup(idx, list[cur]);
      }
      openByRow[idx] = false;
    }
  }

  /** DEF-2A dedup применённый к текущим suggestions строки — используется и
   *  в разметке (список опций + empty-state gate), и в keyboard-навигации,
   *  чтобы оба пути видели один и тот же видимый список. */
  function visibleGroups(idx: number): DeviceGroup[] {
    const list = suggestionsByRow[idx] ?? [];
    const selectedIds = getSelectedIds(idx);
    return list.filter((g) => !g.ids.some((id) => selectedIds.has(id)));
  }

  function pickGroup(idx: number, g: DeviceGroup) {
    const d = g.repr;
    // Для серийных устройств — суффикс с inv_no (каждое уникальное).
    // Для групп — суффикс «×{count}» вместо инв.№.
    const label = d.serial_no
      ? d.inventory_no
        ? `${d.name} (инв. ${d.inventory_no})`
        : d.name
      : `${d.name}${d.model ? ` · ${d.model}` : ''} ×${g.count}`;
    const hasSerial = !!d.serial_no;
    const next = items.map((it, i) =>
      i === idx
        ? {
            ...it,
            device_id: d.id,
            device_label: label,
            query: label,
            picked: true,
            has_serial: hasSerial,
            // W-5: если выбранное устройство имеет serial — clamp qty=1.
            // WR-02 (Plan 19-08): edit-режим — та же логика, что и в pickDevice.
            quantity: hasSerial || mode === 'edit' ? 1 : Math.min(it.quantity, g.count),
            stock_available: g.count,
            group_ids: g.ids,
          }
        : it,
    );
    onChange(next);
    suggestionsByRow[idx] = [];
    openByRow[idx] = false;
    activeIndexByRow[idx] = -1;
  }

  function handleQtyInput(idx: number, v: string) {
    const parsed = parseInt(v, 10);
    let qty = Number.isFinite(parsed) && parsed > 0 ? parsed : 1;
    // G-3: hard cap UX feedback (mirror backend MAX_CLONE_QTY = 1000).
    if (qty > MAX_CLONE_QTY) qty = MAX_CLONE_QTY;
    // W-5: serialised devices must stay at qty=1.
    if (items[idx]?.has_serial && qty > 1) qty = 1;
    // UAT Fix #4: hard cap к stock_available — иначе можно создать акт на
    // несуществующее кол-во устройств (1000 мышек при stock=5 → bug-report).
    const cap = items[idx]?.stock_available;
    if (typeof cap === 'number' && cap > 0 && qty > cap) qty = cap;
    const next = items.map((it, i) => (i === idx ? { ...it, quantity: qty } : it));
    onChange(next);
  }

  /** DEF-2A (Phase 03.2): собрать Set всех device IDs, уже занятых picked-строками,
   *  исключая строку с индексом excludeIdx (чтобы текущая строка не блокировала сама себя). */
  function getSelectedIds(excludeIdx: number): Set<number> {
    const ids = new Set<number>();
    items.forEach((it, i) => {
      if (i !== excludeIdx && it.picked && it.group_ids) {
        it.group_ids.forEach((id) => ids.add(id));
      }
    });
    return ids;
  }

  function errFor(idx: number, field: string): string | null {
    return fieldErrors[`items[${idx}].${field}`] ?? null;
  }

  /** AUTO-01: закрыть дропдаун строки при клике вне И её input, И её портированного
   *  (перенесённого в <body>) dropdown — по аналогии с LocationAutocomplete,
   *  но по массиву строк, т.к. в этой таблице несколько независимых пикеров. */
  function handleClickOutside(e: MouseEvent) {
    const target = e.target as Node;
    for (const key of Object.keys(openByRow)) {
      const i = Number(key);
      if (!openByRow[i]) continue;
      const insideInput = rowInputEls[i]?.contains(target) ?? false;
      const insideDropdown = rowDropdownEls[i]?.contains(target) ?? false;
      if (!insideInput && !insideDropdown) {
        openByRow[i] = false;
      }
    }
  }

  $effect(() => {
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  });
</script>

<div class="items">
  <div class="thead" role="row">
    <div class="th col-num">#</div>
    <div class="th col-device">Устройство ⃰</div>
    <div class="th col-qty">Количество ⃰</div>
    <div class="th col-actions" aria-label="Действия"></div>
  </div>

  {#if items.length === 0}
    <div class="empty">Добавьте хотя бы одну позицию.</div>
  {:else}
    {#each items as row, idx (idx)}
      <div class="tr" role="row">
        <div class="td col-num">{idx + 1}</div>
        <div class="td col-device" class:has-error={!!errFor(idx, 'device_id')}>
          {#if mode === 'edit' && row.complectation_at_time !== undefined}
            <!-- Plan 19-09 (ACT-02/D-10): retained edit-mode position — itemsFromInitialAct
                 sets query: '' for prefilled rows, so the picker input would render blank
                 ("Устройство со склада" placeholder) even though the device is already set.
                 complectation_at_time !== undefined is the retained-position marker (see
                 FormItemRow doc comment) — fresh rows added during this edit session never
                 have it set, and create mode never sets it either. -->
            <span class="device-readonly">{row.device_label}</span>
          {:else}
            <input
              type="text"
              bind:this={rowInputEls[idx]}
              class="device-input"
              class:invalid={!!errFor(idx, 'device_id')}
              value={row.query}
              placeholder="Устройство со склада"
              autocomplete="off"
              aria-autocomplete="list"
              oninput={(e) => handleQueryInput(idx, (e.currentTarget as HTMLInputElement).value)}
              onfocus={() => handleFocus(idx)}
              onkeydown={(e) => handleRowKeydown(idx, e)}
            />
            {#if loadingByRow[idx]}
              <div class="loading-row"><Spinner size="sm" /></div>
            {/if}
            {#if openByRow[idx]}
              <ul
                class="dropdown--items"
                role="listbox"
                use:portal
                use:dropdownAnchor={{ anchorEl: rowInputEls[idx] }}
                bind:this={rowDropdownEls[idx]}
              >
                {#if viewModeByRow[idx] === 'members'}
                <!-- Plan 18-05 (AUTO-04/D-06/D-07 drill-in, AUTO-05/D-09 auto-flatten) -->
                <!-- checkpoint fix #1: sticky-заголовок группы ВСЕГДА виден в
                     member-view (в т.ч. при auto-flatten); «← Назад» — только
                     при ручном drill-in (showBackByRow). -->
                <li class="drill-header">
                  {#if showBackByRow[idx]}
                    <button
                      type="button"
                      class="drill-back"
                      onmousedown={(e) => e.preventDefault()}
                      onclick={() => backToGroups(idx)}
                    >
                      ← Назад
                    </button>
                  {/if}
                  <span class="drill-title"
                    >{drillGroupByRow[idx]?.repr.name}{drillGroupByRow[idx]?.repr.model
                      ? ` · ${drillGroupByRow[idx]?.repr.model}`
                      : ''}</span
                  >
                </li>
                {#if memberRows(idx).length === 0}
                  <li class="dropdown-empty">Ничего не найдено</li>
                {:else}
                  {#each memberRows(idx) as mrow (mrow.key)}
                    {#if mrow.kind === 'instance'}
                      <li>
                        <button
                          type="button"
                          class="opt member-instance"
                          role="option"
                          aria-selected="false"
                          onmousedown={(e) => e.preventDefault()}
                          onclick={() => pickDevice(idx, mrow.device, [mrow.device.id])}
                        >
                          <span class="opt-row">
                            <!-- checkpoint fix (round 2) #2: ОБА номера, если оба
                                 заполнены (SN · инв.), иначе только заполненный. -->
                            {#if mrow.device.serial_no}
                              <span class="opt-sn">SN {mrow.device.serial_no}</span>
                            {/if}
                            {#if mrow.device.serial_no && mrow.device.inventory_no}
                              <span class="opt-sep"> · </span>
                            {/if}
                            {#if mrow.device.inventory_no}
                              <span class="opt-inv">инв. {mrow.device.inventory_no}</span>
                            {/if}
                            <span class="opt-state">{mrow.device.state ?? '—'}</span>
                            <!-- reserved chevron-slot (пустой) — column-align ×count -->
                            <span class="opt-chevron" aria-hidden="true"></span>
                          </span>
                        </button>
                      </li>
                    {:else}
                      <li>
                        <button
                          type="button"
                          class="opt member-subgroup"
                          role="option"
                          aria-selected="false"
                          onmousedown={(e) => e.preventDefault()}
                          onclick={() =>
                            pickDevice(
                              idx,
                              mrow.devices[0],
                              mrow.devices.map((d) => d.id),
                            )}
                        >
                          <span class="opt-row">
                            <span class="member-subgroup-label">Без номера · {mrow.state ?? '—'}</span>
                            <span class="opt-count">×{mrow.devices.length}</span>
                            <!-- reserved chevron-slot (пустой) — column-align ×count -->
                            <span class="opt-chevron" aria-hidden="true"></span>
                          </span>
                        </button>
                      </li>
                    {/if}
                  {/each}
                {/if}
              {:else if visibleGroups(idx).length === 0}
                <li class="dropdown-empty">Ничего не найдено</li>
              {:else}
                {#each visibleGroups(idx) as g, i (g.repr.id)}
                  <li>
                    <button
                      type="button"
                      class="opt"
                      class:active={i === (activeIndexByRow[idx] ?? -1)}
                      role="option"
                      aria-selected={i === (activeIndexByRow[idx] ?? -1)}
                      onmousedown={(e) => e.preventDefault()}
                      onclick={() => handleGroupClick(idx, g)}
                    >
                      <div class="opt-row">
                        <span class="opt-name">{g.repr.name}</span>
                        {#if g.repr.model}<span class="opt-model">{g.repr.model}</span>{/if}
                        <span class="opt-count">×{g.count}</span>
                        <!-- checkpoint fix #3: chevron-slot зарезервирован ВСЕГДА
                             (пустой у нераскрываемых) — все ×count в один столбец -->
                        <span class="opt-chevron" aria-hidden={!isExpandable(g)}
                          >{isExpandable(g) ? '›' : ''}</span
                        >
                      </div>
                      <!-- checkpoint fix (round 2) #1/#2: серийный/инвентарный №
                           показываем ТОЛЬКО у одиночного устройства
                           (g.ids.length === 1) — у раскрываемой группы номера у
                           каждого экземпляра свои, показ repr-номера вводит в
                           заблуждение. И показываем ОБА номера, если оба есть. -->
                      {#if g.ids.length === 1 && (g.repr.serial_no || g.repr.inventory_no)}
                        <span class="opt-meta-row">
                          {#if g.repr.serial_no}<span class="opt-sn">SN {g.repr.serial_no}</span>{/if}
                          {#if g.repr.serial_no && g.repr.inventory_no}<span class="opt-sep"> · </span>{/if}
                          {#if g.repr.inventory_no}<span class="opt-inv">инв. {g.repr.inventory_no}</span>{/if}
                        </span>
                      {/if}
                      {#if g.repr.state}
                        <span class="opt-state">{g.repr.state}</span>
                      {/if}
                    </button>
                  </li>
                {/each}
              {/if}
            </ul>
            {/if}
          {/if}
          {#if errFor(idx, 'device_id')}
            <p class="row-error">{errFor(idx, 'device_id')}</p>
          {/if}
        </div>
        <div class="td col-qty" class:has-error={!!errFor(idx, 'quantity')}>
          {#if mode === 'edit'}
            <!-- WR-02 (Plan 19-08): edit-режим — добавляемая строка всегда
                 ровно одно устройство (ActUpdateItemDto несёт только
                 device_id, без quantity/device_ids). Показываем статичную
                 «1» вместо редактируемого спиннера, чтобы видимое
                 количество не вводило в заблуждение относительно того, что
                 будет сохранено. -->
            <span class="qty-fixed">1</span>
          {:else}
            <input
              type="number"
              class="input qty-input"
              class:invalid={!!errFor(idx, 'quantity')}
              value={String(row.quantity)}
              min="1"
              max={qtyMax(row)}
              disabled={row.has_serial}
              oninput={(e) => handleQtyInput(idx, (e.currentTarget as HTMLInputElement).value)}
            />
          {/if}
          {#if errFor(idx, 'quantity')}
            <p class="row-error">{errFor(idx, 'quantity')}</p>
          {/if}
        </div>
        <div class="td col-actions">
          <Button variant="ghost" size="sm" onclick={() => removeRow(idx)}>×</Button>
        </div>
        {#if mode === 'edit' && row.picked && row.device_id !== null && row.complectation_at_time !== undefined}
          <!-- Plan 19-05 (ACT-02/D-04): комплектация editable ONLY on retained
               positions (rows prefilled from initialAct — see FormItemRow doc
               comment). «Технические характеристики» (devices.notes) stays
               read-only/out of scope, no input rendered anywhere for it. -->
          <div class="td col-complectation">
            <label class="label" for={`complectation-${idx}`}>Комплектация</label>
            <input
              id={`complectation-${idx}`}
              type="text"
              class="input"
              value={row.complectation_at_time ?? ''}
              placeholder="Комплектация"
              oninput={(e) =>
                handleComplectationInput(idx, (e.currentTarget as HTMLInputElement).value)}
            />
          </div>
        {/if}
      </div>
    {/each}
  {/if}

  <div class="add-row">
    <Button variant="ghost" size="sm" onclick={addRow}>+ Добавить позицию</Button>
  </div>
</div>

<style lang="scss">
  .items {
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    overflow: visible;
  }
  .thead,
  .tr {
    display: grid;
    grid-template-columns: 40px 1fr 140px 56px;
    gap: var(--space-sm);
    align-items: start;
    padding: var(--space-sm) var(--space-md);
  }
  .thead {
    background: var(--color-surface-sunken);
    border-bottom: 1px solid var(--color-border);
    align-items: center;
  }
  .th {
    font-size: var(--font-size-label);
    font-weight: 500;
    color: var(--color-text-secondary);
  }

  .tr {
    border-bottom: 1px solid var(--color-border);
    &:last-of-type {
      border-bottom: none;
    }
  }
  .col-num {
    font-variant-numeric: tabular-nums;
    color: var(--color-text-muted);
    padding-top: 8px;
  }
  .col-device {
    position: relative;
  }
  .col-qty {
    font-variant-numeric: tabular-nums;
  }
  .col-actions {
    display: flex;
    justify-content: flex-end;
  }
  // Plan 19-05 (ACT-02/D-04): комплектация row spans the full grid width,
  // rendered directly beneath the device/qty/actions row it belongs to.
  .col-complectation {
    grid-column: 1 / -1;
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
    padding-top: var(--space-xs);

    .label {
      font-size: var(--font-size-label);
      font-weight: 500;
      color: var(--color-text-secondary);
    }
    .input {
      display: block;
      width: 100%;
      height: 36px;
      padding: 0 var(--space-md);
      background: var(--color-bg);
      color: var(--color-text-primary);
      border: 1px solid var(--color-border);
      border-radius: var(--radius-sm);
      font-family: var(--font-family-base);
      font-size: var(--font-size-body);
      line-height: var(--line-height-body);

      &:focus-visible {
        outline: none;
        border-color: var(--color-accent);
        box-shadow: 0 0 0 3px var(--color-accent-focus);
      }
    }
  }

  // Plan 18-04 (AUTO-01): дропдаун перенесён use:portal в <body>, поэтому scoped
  // CSS компонента до него (и его потомков) не доходит — нужен :global().
  // Позиция (position/top/left/width/bottom) управляется JS через
  // use:dropdownAnchor, здесь только визуал (UI-SPEC AUTO-01).
  //
  // WR-03: дропдаун портирован в <body> из НЕСКОЛЬКИХ компонентов
  // (PersonAutocomplete/LocationAutocomplete/DeviceAutocompleteField/
  // ActFormItemsTable) — без namespace-класса на корне глобальные правила
  // .dropdown/.dropdown-empty коллизируют с остальными (последний
  // подключённый stylesheet выигрывает). Все правила ниже скопированы под
  // :global(.dropdown--items ...).
  :global(.dropdown--items) {
    position: fixed;
    z-index: 1000;
    max-height: 240px;
    overflow: auto;
    background: var(--color-surface-raised, var(--color-surface));
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    margin: 0;
    padding: 0;
    list-style: none;
    box-shadow: var(--shadow-elev-2);
  }
  :global(.dropdown--items .opt) {
    display: flex;
    flex-direction: column;
    gap: 2px;
    width: 100%;
    text-align: left;
    padding: var(--space-sm) var(--space-md);
    background: transparent;
    border: none;
    cursor: pointer;
    color: var(--color-text-primary);
    font-family: var(--font-family-base);
    font-size: var(--font-size-body);
  }
  :global(.dropdown--items .opt:hover),
  :global(.dropdown--items .opt.active) {
    background: var(--color-surface-sunken);
  }
  :global(.dropdown--items .opt-row) {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    width: 100%;
  }
  :global(.dropdown--items .opt-name) {
    font-weight: 500;
  }
  :global(.dropdown--items .opt-inv),
  :global(.dropdown--items .opt-sn),
  :global(.dropdown--items .opt-model) {
    font-size: var(--font-size-label);
    color: var(--color-text-secondary);
  }
  :global(.dropdown--items .opt-count) {
    margin-left: auto;
    font-size: var(--font-size-label);
    color: var(--color-accent, var(--color-text-secondary));
    font-weight: 500;
  }
  :global(.dropdown--items .opt-state) {
    font-size: var(--font-size-label);
    color: var(--color-text-secondary);
  }
  // checkpoint fix (round 2) #2: строка «SN … · инв. …» одиночного устройства —
  // inline-ряд обоих номеров через middot-разделитель (UI-SPEC мета-разделитель).
  :global(.dropdown--items .opt-meta-row) {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    font-size: var(--font-size-label);
  }
  :global(.dropdown--items .opt-sep) {
    color: var(--color-text-muted);
    font-size: var(--font-size-label);
  }
  :global(.dropdown--items .dropdown-empty) {
    padding: var(--space-xl);
    text-align: center;
    color: var(--color-text-muted);
    font-size: var(--font-size-body);
    list-style: none;
  }

  // Plan 18-05 (AUTO-04/D-06 + checkpoint fix #3): chevron-сигнал drill-in
  // справа от ×count. Слот зарезервирован ФИКСИРОВАННОЙ ширины ВСЕГДА (даже
  // пустой у нераскрываемых/member-строк), чтобы бейджи ×count всех типов
  // строк выстроились в один вертикальный столбец.
  :global(.dropdown--items .opt-chevron) {
    flex: 0 0 auto;
    width: 12px;
    text-align: center;
    color: var(--color-text-secondary);
    font-size: var(--font-size-label);
  }

  // Plan 18-05 (AUTO-04/D-06 + checkpoint fix #1): заголовок drill-in —
  // опциональная «← Назад» + название раскрытой группы. Sticky-закреплён
  // сверху скроллируемого дропдауна с непрозрачным фоном + тенью, чтобы
  // member-строки не просвечивали под ним при прокрутке.
  :global(.dropdown--items .drill-header) {
    position: sticky;
    top: 0;
    z-index: 1;
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    padding: var(--space-sm) var(--space-md);
    background: var(--color-surface-raised, var(--color-surface));
    border-bottom: 1px solid var(--color-border);
    box-shadow: var(--shadow-elev-1, 0 1px 2px rgba(0, 0, 0, 0.08));
    list-style: none;
  }
  :global(.dropdown--items .drill-back) {
    background: transparent;
    border: none;
    cursor: pointer;
    padding: 0;
    font-family: var(--font-family-base);
    font-size: var(--font-size-label);
    color: var(--color-text-secondary);
    &:hover {
      color: var(--color-text-primary);
    }
  }
  :global(.dropdown--items .drill-title) {
    font-size: var(--font-size-label);
    font-weight: 500;
    color: var(--color-text-secondary);
  }

  // Plan 18-05 (AUTO-04/D-07): подпись «Без номера · {state}» под-группы —
  // Label-стиль (13px/400), а НЕ акцентное наименование группы уровня 1.
  :global(.dropdown--items .member-subgroup-label) {
    font-size: var(--font-size-label);
    font-weight: 400;
    color: var(--color-text-secondary);
  }

  .loading-row {
    position: absolute;
    top: 8px;
    right: 8px;
  }

  .row-error {
    margin: 4px 0 0;
    font-size: var(--font-size-label);
    color: var(--color-destructive);
  }

  .empty {
    padding: var(--space-xl);
    text-align: center;
    color: var(--color-text-muted);
    font-size: var(--font-size-body);
  }

  .add-row {
    padding: var(--space-sm) var(--space-md);
    border-top: 1px solid var(--color-border);
  }

  // Plan 18-04 (AUTO-01/D-05): raw <input> заменяет Input.svelte (нет
  // ref-forwarding) — визуальная эквивалентность сохраняется теми же CSS-
  // свойствами, что .qty-input ниже.
  .device-input {
    display: block;
    width: 100%;
    height: 36px;
    padding: 0 var(--space-md);
    background: var(--color-bg);
    color: var(--color-text-primary);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    font-family: var(--font-family-base);
    font-size: var(--font-size-body);
    line-height: var(--line-height-body);

    &:focus-visible {
      outline: none;
      border-color: var(--color-accent);
      box-shadow: 0 0 0 3px var(--color-accent-focus);
    }
    &.invalid {
      border-color: var(--color-destructive);
      box-shadow: 0 0 0 3px rgba(220, 38, 38, 0.2);
    }
  }

  // G-3 / W-5 — qty input native styling согласован с Input.svelte tokens.
  .qty-input {
    display: block;
    width: 100%;
    height: 36px;
    padding: 0 var(--space-md);
    background: var(--color-bg);
    color: var(--color-text-primary);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    font-family: var(--font-family-base);
    font-size: var(--font-size-body);
    line-height: var(--line-height-body);

    &:focus-visible {
      outline: none;
      border-color: var(--color-accent);
      box-shadow: 0 0 0 3px var(--color-accent-focus);
    }
    &.invalid {
      border-color: var(--color-destructive);
      box-shadow: 0 0 0 3px rgba(220, 38, 38, 0.2);
    }
  }

  // WR-02 (Plan 19-08): static qty display in edit mode — no spinner control,
  // same height/alignment as .qty-input so the row layout doesn't shift.
  .qty-fixed {
    display: flex;
    align-items: center;
    height: 36px;
    padding: 0 var(--space-md);
    color: var(--color-text-secondary, var(--color-text-primary));
    font-size: var(--font-size-body);
    line-height: var(--line-height-body);
  }

  // Plan 19-09 (ACT-02/D-10): read-only device name for retained edit-mode
  // positions — filled non-editable cell, visually matching .device-input
  // minus the border/background/focus (a static label, mirrors .qty-fixed).
  .device-readonly {
    display: flex;
    align-items: center;
    height: 36px;
    padding: 0 var(--space-md);
    color: var(--color-text-primary);
    font-family: var(--font-family-base);
    font-size: var(--font-size-body);
    line-height: var(--line-height-body);
  }

  .hint-warn {
    margin: var(--space-xs) 0 0;
    font-size: 12px;
    color: var(--color-warning, #b45309);
  }
</style>
