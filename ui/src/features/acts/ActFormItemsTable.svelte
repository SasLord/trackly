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
  import Input from '$lib/components/Input.svelte';
  import Button from '$lib/components/Button.svelte';
  import Spinner from '$lib/components/Spinner.svelte';
  import { devices } from '$lib/api/devices';
  import type { DeviceGroup } from '../../bindings';

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
    onChange: (_items: FormItemRow[]) => void;
  }

  const { items, fieldErrors, onChange }: Props = $props();

  // Per-row search state — keyed by row index. Reset when the row is mutated.
  // UAT Fix #3: changed DeviceDto[] → DeviceGroup[] чтобы dropdown показывал
  // ОДНУ запись на группу (name+model+inv_no=NULL) с count badge, а не 20
  // одинаковых клонов.
  let suggestionsByRow = $state<Record<number, DeviceGroup[]>>({});
  let loadingByRow = $state<Record<number, boolean>>({});
  let openByRow = $state<Record<number, boolean>>({});
  const debounceTimers: Record<number, ReturnType<typeof setTimeout>> = {};

  function makeEmpty(): FormItemRow {
    return { device_id: null, quantity: 1, device_label: '', query: '', picked: false };
  }

  function addRow() {
    onChange([...items, makeEmpty()]);
  }

  function removeRow(idx: number) {
    const next = items.filter((_, i) => i !== idx);
    delete suggestionsByRow[idx];
    delete loadingByRow[idx];
    delete openByRow[idx];
    onChange(next);
  }

  function handleQueryInput(idx: number, v: string) {
    const next = items.map((it, i) =>
      i === idx ? { ...it, query: v, picked: false, device_id: null, device_label: '' } : it,
    );
    onChange(next);

    if (debounceTimers[idx]) clearTimeout(debounceTimers[idx]);
    if (v.trim().length < 1) {
      suggestionsByRow[idx] = [];
      openByRow[idx] = false;
      return;
    }
    debounceTimers[idx] = setTimeout(async () => {
      loadingByRow[idx] = true;
      try {
        // UAT Fix #3/#4: listGrouped возвращает группы (одинаковые
        // name+model+inv_no=NULL) с count + ids. Filter status_id=1 (на_складе).
        const groups = await devices.listGrouped(
          {
            type_id: null,
            location_id: null,
            status_id: 1,
            state: null,
            name_prefix: v.trim(),
            include_deleted: false,
          },
          { offset: 0, limit: 20 },
        );
        // DEF-2A: exclude groups whose IDs overlap with already-picked rows.
        const selectedIds = getSelectedIds(idx);
        const filtered = groups.filter((g) => !g.ids.some((id) => selectedIds.has(id)));
        suggestionsByRow[idx] = filtered;
        openByRow[idx] = filtered.length > 0;
      } catch {
        suggestionsByRow[idx] = [];
        openByRow[idx] = false;
      } finally {
        loadingByRow[idx] = false;
      }
    }, 250);
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
            quantity: hasSerial ? 1 : Math.min(it.quantity, g.count),
            stock_available: g.count,
            group_ids: g.ids,
          }
        : it,
    );
    onChange(next);
    suggestionsByRow[idx] = [];
    openByRow[idx] = false;
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
          <Input
            type="text"
            value={row.query}
            placeholder="Устройство со склада"
            invalid={!!errFor(idx, 'device_id')}
            oninput={(v) => handleQueryInput(idx, v)}
          />
          {#if loadingByRow[idx]}
            <div class="loading-row"><Spinner size="sm" /></div>
          {/if}
          {#if openByRow[idx] && suggestionsByRow[idx]?.length > 0}
            <ul class="dropdown" role="listbox">
              {#each suggestionsByRow[idx].filter((g) => !g.ids.some((id) => getSelectedIds(idx).has(id))) as g (g.repr.id)}
                <li>
                  <button type="button" class="opt" onclick={() => pickGroup(idx, g)}>
                    <span class="opt-name">{g.repr.name}</span>
                    {#if g.repr.model}<span class="opt-model">{g.repr.model}</span>{/if}
                    {#if g.repr.serial_no}
                      <span class="opt-sn">SN {g.repr.serial_no}</span>
                    {:else if g.repr.inventory_no}
                      <span class="opt-inv">инв. {g.repr.inventory_no}</span>
                    {:else}
                      <span class="opt-count">×{g.count}</span>
                    {/if}
                    {#if g.repr.state}
                      <span class="opt-state">{g.repr.state}</span>
                    {/if}
                  </button>
                </li>
              {/each}
            </ul>
          {/if}
          {#if errFor(idx, 'device_id')}
            <p class="row-error">{errFor(idx, 'device_id')}</p>
          {/if}
        </div>
        <div class="td col-qty" class:has-error={!!errFor(idx, 'quantity')}>
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

  .dropdown {
    position: absolute;
    top: 40px;
    left: 0;
    right: 0;
    max-height: 240px;
    overflow: auto;
    background: var(--color-surface-raised, var(--color-surface));
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    margin: 0;
    padding: 0;
    list-style: none;
    z-index: 10;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.08);
  }
  .opt {
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

    &:hover {
      background: var(--color-surface-sunken);
    }
  }
  .opt-name {
    font-weight: 500;
  }
  .opt-inv,
  .opt-sn,
  .opt-model {
    font-size: var(--font-size-label);
    color: var(--color-text-secondary);
  }
  .opt-count {
    font-size: var(--font-size-label);
    color: var(--color-accent, var(--color-text-secondary));
    font-weight: 600;
  }
  .opt-state {
    font-size: var(--font-size-label);
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

  .hint-warn {
    margin: var(--space-xs) 0 0;
    font-size: 12px;
    color: var(--color-warning, #b45309);
  }
</style>
