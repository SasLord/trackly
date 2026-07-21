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
  //
  // Plan 25-07 (CMP-07): дропдаун-пикер устройства переведён на общий компонент
  // Dropdown (Plans 25-02/25-03) — drill-in/фокус-открытие/клавиатура/ARIA/portal
  // теперь внутри Dropdown; бизнес-логика (fetchGroups/expandGroup/pickGroup/
  // pickDevice/DEF-2A) осталась здесь без изменений по сути, только вызывается
  // через callback-пропы вместо inline onclick/onkeydown.
  import Button from '$lib/components/Button.svelte';
  import Spinner from '$lib/components/Spinner.svelte';
  import Dropdown from '$lib/components/Dropdown.svelte';
  import { devices } from '$lib/api/devices';
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
    /** Plan 19-05 (ACT-02) / Plan 19-09 (D-09): комплектация на момент акта
     *  (act_items.complectation_at_time). UI-editable input was REMOVED in Plan 19-09
     *  for RETAINED rows — GT2 (260715-gt2) supersedes that statement for FRESH rows
     *  only (freshly-added, non-serial edit-mode positions gained a qty-editable
     *  input; комплектация itself is still not user-editable in either case). The
     *  retained-position marker semantics of this field are UNCHANGED and still
     *  load-bearing: its presence (not its value) is the RETAINED-position marker —
     *  only ever populated by ActFormBody's edit-mode prefill (itemsFromInitialAct),
     *  so a fresh row added during this edit session never has it set. This is what
     *  the read-only device cell (Plan 19-09/D-10, ~line 532) and the qty-cell gate
     *  (~line 696) both use to distinguish retained vs. new rows. */
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

  // Per-row search state — keyed by row index. Reset when the row is mutated.
  // UAT Fix #3: changed DeviceDto[] → DeviceGroup[] чтобы dropdown показывал
  // ОДНУ запись на группу (name+model+inv_no=NULL) с count badge, а не 20
  // одинаковых клонов.
  let suggestionsByRow = $state<Record<number, DeviceGroup[]>>({});
  let loadingByRow = $state<Record<number, boolean>>({});

  // Plan 18-05 (AUTO-04/D-06/D-07 + AUTO-05/D-09) drill-in partition shape —
  // Plan 25-07: no longer row-indexed $state (Dropdown owns open/viewMode/
  // drillGroup/members/showBack/activeIndex internally per-instance now);
  // this type is still the contract expandGroup()/pickMember() below produce
  // and consume, matching what memberRows() used to return.
  type MemberRow =
    | { kind: 'instance'; key: string; device: DeviceDto }
    | { kind: 'subgroup'; key: string; state: string | null; devices: DeviceDto[] };

  function makeEmpty(): FormItemRow {
    return { device_id: null, quantity: 1, device_label: '', query: '', picked: false };
  }

  function addRow() {
    onChange([...items, makeEmpty()]);
  }

  /** WR-01: индекс-ключевые transient-мапы дропдауна должны следовать за
   *  сдвигом строк при удалении — иначе после removeRow(idx) все строки
   *  после idx показывают состояние (suggestions/loading) ПРЕДЫДУЩЕГО
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
    suggestionsByRow = shiftRowState(suggestionsByRow, idx);
    loadingByRow = shiftRowState(loadingByRow, idx);
    onChange(next);
  }

  /** Plan 25-07: syncs row.query as the user types — the drill-in view-mode
   *  reset this used to do and the 250ms-debounced re-fetch it used to
   *  schedule are both now Dropdown's own internal responsibility
   *  (onQueryInput fires synchronously before Dropdown's debounced onSearch). */
  function handleQueryInput(idx: number, v: string) {
    const next = items.map((it, i) =>
      i === idx ? { ...it, query: v, picked: false, device_id: null, device_label: '' } : it,
    );
    onChange(next);
  }

  /** AUTO-02/AUTO-03: data-fetch, now invoked by Dropdown's onSearch — Dropdown
   *  itself decides WHEN to call it (immediately on focus per AUTO-02, after its
   *  own 250ms debounce on typed input) and owns opening the panel / auto-
   *  flattening a single remaining group (AUTO-05). This function's only job is
   *  the fetch + DEF-2A dedup, writing the result into suggestionsByRow[idx]. */
  async function fetchGroups(idx: number, query: string) {
    loadingByRow[idx] = true;
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
      suggestionsByRow[idx] = groups.filter((g) => !g.ids.some((id) => selectedIds.has(id)));
    } catch {
      suggestionsByRow[idx] = [];
    } finally {
      loadingByRow[idx] = false;
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

  /** D-07: партиционирует member-список раскрытой/схлопнутой группы на
   *  отдельные строки серийных/инвентарных экземпляров и client-side
   *  под-группы по state для несерийных/безынвентарных. Инстансы идут первыми
   *  (порядок из devices.listByIds), затем под-группы (порядок вставки в Map)
   *  — соответствует ASCII-макету UI-SPEC. Plan 25-07: extracted from the old
   *  row-indexed memberRows(idx) into a pure function taking the fetched list
   *  directly, since expandGroup() below no longer stores it in $state first. */
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

  /** D-06/D-07: раскрытие группы — вызывается Dropdown'ом и при ручном клике
   *  по раскрываемой группе, И внутренне при AUTO-05 auto-flatten единственной
   *  оставшейся группы (обе ветки теперь живут в Dropdown.svelte, не здесь).
   *  DEF-2A dedup зеркалит fetchGroups. */
  async function expandGroup(idx: number, g: DeviceGroup): Promise<MemberRow[]> {
    const selectedIds = getSelectedIds(idx);
    const ids = g.ids.filter((id) => !selectedIds.has(id));
    try {
      return partitionMembers(await devices.listByIds(ids));
    } catch {
      return [];
    }
  }

  /** checkpoint fix (round 2) #2 mirror: ОБА номера, если оба заполнены
   *  (SN · инв.), иначе только заполненный; undefined если нет ни одного. */
  function joinSnInv(
    sn: string | null | undefined,
    inv: string | null | undefined,
  ): string | undefined {
    const parts = [sn ? `SN ${sn}` : null, inv ? `инв. ${inv}` : null].filter(
      (p): p is string => p !== null,
    );
    return parts.length > 0 ? parts.join(' · ') : undefined;
  }

  /** checkpoint fix (round 2) #1/#2 mirror: серийный/инвентарный № показываем
   *  ТОЛЬКО у одиночного устройства (g.ids.length === 1) — у раскрываемой
   *  группы номера у каждого экземпляра свои, показ repr-номера вводит в
   *  заблуждение. */
  function groupSub(g: DeviceGroup): string | undefined {
    if (g.ids.length !== 1) return undefined;
    return joinSnInv(g.repr.serial_no, g.repr.inventory_no);
  }

  /** Plan 18-05 (AUTO-04/D-07) mirror: instance rows show the device name
   *  (Dropdown's own primary/name slot); subgroup rows show the same
   *  "Без номера · {state}" label .member-subgroup-label used today. */
  function memberName(m: MemberRow): string {
    return m.kind === 'instance' ? m.device.name : `Без номера · ${m.state ?? '—'}`;
  }

  /** Instance rows: SN/inv (same shape as groupSub). Subgroup rows: the
   *  ×{count} badge .opt-count showed today. */
  function memberMeta(m: MemberRow): string | undefined {
    if (m.kind === 'subgroup') return `×${m.devices.length}`;
    return joinSnInv(m.device.serial_no, m.device.inventory_no);
  }

  /** Instance rows only: the .opt-state text shown today next to SN/inv. */
  function memberSub(m: MemberRow): string | undefined {
    return m.kind === 'instance' ? (m.device.state ?? '—') : undefined;
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
            // WR-02 (Plan 19-08) forced qty=1 in edit mode because
            // ActUpdateItemDto carried only one device_id per entry with no
            // expansion path. Superseded by GT2 (260715-gt2): freshly-added,
            // non-serial edit-mode positions are now qty-editable like
            // create mode — ActUpdateItemDto itself is unchanged (still one
            // device_id + complectation_at_time per entry); multi-qty
            // travels over the wire via submit-side group_ids expansion in
            // ActFormBody, not a DTO change.
            quantity: hasSerial ? 1 : Math.min(it.quantity, groupIds.length),
            stock_available: groupIds.length,
            group_ids: groupIds,
          }
        : it,
    );
    onChange(next);
    suggestionsByRow[idx] = [];
  }

  /** D-07: dispatch на pickDevice() из Dropdown's onPickMember — instance-строка
   *  выбирает единственный экземпляр, subgroup-строка выбирает представителя
   *  под-группы + все id'ы под-группы (как в pickDevice-вызовах today's markup,
   *  lines 602/635-640). */
  function pickMember(idx: number, m: MemberRow) {
    if (m.kind === 'instance') {
      pickDevice(idx, m.device, [m.device.id]);
    } else {
      pickDevice(
        idx,
        m.devices[0],
        m.devices.map((d) => d.id),
      );
    }
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
            // WR-02 (Plan 19-08) forced qty=1 in edit mode; superseded by
            // GT2 (260715-gt2) — see the matching comment in pickDevice above.
            quantity: hasSerial ? 1 : Math.min(it.quantity, g.count),
            stock_available: g.count,
            group_ids: g.ids,
          }
        : it,
    );
    onChange(next);
    suggestionsByRow[idx] = [];
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
            <Dropdown
              variant="combobox"
              value={row.query}
              placeholder="Устройство со склада"
              invalid={!!errFor(idx, 'device_id')}
              groups={suggestionsByRow[idx] ?? []}
              loading={!!loadingByRow[idx]}
              getGroupId={(g: DeviceGroup) => g.repr.id}
              getGroupName={(g: DeviceGroup) => g.repr.name}
              getGroupMeta={(g: DeviceGroup) => g.repr.model ?? undefined}
              getGroupSub={groupSub}
              getGroupCount={(g: DeviceGroup) => g.count}
              isGroupExpandable={isExpandable}
              onExpandGroup={(g: DeviceGroup) => expandGroup(idx, g)}
              getMemberId={(m: MemberRow) => m.key}
              getMemberName={memberName}
              getMemberMeta={memberMeta}
              getMemberSub={memberSub}
              onSearch={(query) => void fetchGroups(idx, query)}
              onQueryInput={(v) => handleQueryInput(idx, v)}
              onPickGroup={(g: DeviceGroup) => pickGroup(idx, g)}
              onPickMember={(m: MemberRow) => pickMember(idx, m)}
            />
            {#if loadingByRow[idx]}
              <div class="loading-row"><Spinner size="sm" /></div>
            {/if}
          {/if}
          {#if errFor(idx, 'device_id')}
            <p class="row-error">{errFor(idx, 'device_id')}</p>
          {/if}
        </div>
        <div class="td col-qty" class:has-error={!!errFor(idx, 'quantity')}>
          {#if mode === 'edit' && (row.complectation_at_time !== undefined || row.has_serial)}
            <!-- WR-02 (Plan 19-08) originally forced qty=1 for EVERY edit-mode
                 row. Superseded by GT2 (260715-gt2): now only RETAINED positions
                 (complectation_at_time !== undefined — see FormItemRow doc
                 comment) and serialised positions (has_serial, W-5, unchanged
                 in every mode) show the static "1". A freshly-added, non-serial
                 edit-mode row falls through to the editable, group-bounded
                 input below, exactly like create mode — submit-side expansion
                 into N ActUpdateItemDto entries happens in ActFormBody. -->
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
      </div>
    {/each}
  {/if}

  <div class="add-row">
    <Button variant="ghost" size="sm" onclick={addRow}>+ Добавить позицию</Button>
  </div>
</div>

<style lang="scss">
  .items {
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-xs);
    overflow: visible;
  }
  .thead,
  .tr {
    display: grid;
    grid-template-columns: 40px 1fr 140px 56px;
    gap: var(--tr-space-xs);
    align-items: start;
    padding: var(--tr-space-xs) var(--tr-space-md);
  }
  .thead {
    background: var(--tr-surface-sunken);
    border-bottom: 1px solid var(--tr-border);
    align-items: center;
  }
  .th {
    font-size: var(--tr-font-size-label);
    font-weight: 500;
    color: var(--tr-text-secondary);
  }

  .tr {
    border-bottom: 1px solid var(--tr-border);
    &:last-of-type {
      border-bottom: none;
    }
  }
  .col-num {
    font-variant-numeric: tabular-nums;
    color: var(--tr-text-tertiary);
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

  .loading-row {
    position: absolute;
    top: 8px;
    right: 8px;
  }

  .row-error {
    margin: 4px 0 0;
    font-size: var(--tr-font-size-label);
    color: var(--tr-danger);
  }

  .empty {
    padding: var(--tr-space-2xl);
    text-align: center;
    color: var(--tr-text-tertiary);
    font-size: var(--tr-font-size-body);
  }

  .add-row {
    padding: var(--tr-space-xs) var(--tr-space-md);
    border-top: 1px solid var(--tr-border);
  }

  // G-3 / W-5 — qty input native styling согласован с Input.svelte tokens.
  .qty-input {
    display: block;
    width: 100%;
    height: 36px;
    padding: 0 var(--tr-space-md);
    background: var(--tr-surface-raised);
    color: var(--tr-text-primary);
    border: 1px solid var(--tr-border-strong);
    border-radius: var(--tr-radius-sm);
    font-family: var(--tr-font-family);
    font-size: var(--tr-font-size-body);
    line-height: var(--tr-line-height-body);

    &:focus-visible {
      outline: none;
      border-color: var(--tr-accent);
      box-shadow: 0 0 0 3px var(--tr-focus-ring);
    }
    &.invalid {
      border-color: var(--tr-danger);
      box-shadow: 0 0 0 3px var(--tr-danger-ring);
    }
  }

  // WR-02 (Plan 19-08): static qty display in edit mode — no spinner control,
  // same height/alignment as .qty-input so the row layout doesn't shift.
  .qty-fixed {
    display: flex;
    align-items: center;
    height: 36px;
    padding: 0 var(--tr-space-md);
    color: var(--tr-text-secondary, var(--tr-text-primary));
    font-size: var(--tr-font-size-body);
    line-height: var(--tr-line-height-body);
  }

  // Plan 19-09 (ACT-02/D-10): read-only device name for retained edit-mode
  // positions — filled non-editable cell, visually matching .device-input
  // minus the border/background/focus (a static label, mirrors .qty-fixed).
  .device-readonly {
    display: flex;
    align-items: center;
    height: 36px;
    padding: 0 var(--tr-space-md);
    color: var(--tr-text-primary);
    font-family: var(--tr-font-family);
    font-size: var(--tr-font-size-body);
    line-height: var(--tr-line-height-body);
  }

  .hint-warn {
    margin: var(--tr-space-2xs) 0 0;
    font-size: 12px;
    color: var(--tr-warning);
  }
</style>
