<script lang="ts">
  // Phase 3.1 Plan 03: per-DEVICE-ID table inside ReturnModal (G-10).
  //
  // G-10: каждая row соответствует ОДНОМУ device_id из outstanding_device_ids.
  //       Если act_item имеет outstanding_device_ids=[10,11,12] — render 3 rows.
  // G-6 (symmetric apply_to_all disable): condition И location inputs обе имеют
  //       ОДИНАКОВУЮ disabled formula = `!row.checked || applyToAll`. Раньше
  //       location имел только `!row.checked` (asymmetric — UAT bug).
  // Когда apply_to_all=true: per-row inputs disabled, placeholder «(по умолчанию)».
  // Когда apply_to_all=false: per-row inputs enabled per checked row.
  // Когда row.checked=false: row opacity 0.5 + оба disabled regardless of applyToAll.
  import Input from '$lib/components/Input.svelte';
  import Checkbox from '$lib/components/Checkbox.svelte';
  import PlacePicker from '$lib/components/PlacePicker.svelte';
  import DeviceAutocompleteField from '../devices/DeviceAutocompleteField.svelte';

  export interface ReturnRowState {
    actItemId: number;
    deviceId: number;
    /** Device name — always shown as the row's primary label. */
    deviceName: string;
    /** Inventory number, if present — rendered in monospace (D-13). Null → row
     *  falls back to showing the device id instead. */
    inventoryNo: string | null;
    /** Per-row checked flag — default true. */
    checked: boolean;
    /** Per-row condition override; null → fallback на bulk (если applyToAll). */
    conditionOverride: string | null;
    /** Per-row place override, выбранный через PlacePicker; null → fallback
     *  на bulk (если applyToAll). */
    placeIdOverride: number | null;
  }

  interface Props {
    items: ReturnRowState[];
    applyToAll: boolean;
    bulkCondition: string | null;
    bulkPlaceId: number | null;
    onChange: (_items: ReturnRowState[]) => void;
  }

  const { items, applyToAll, bulkCondition, bulkPlaceId, onChange }: Props = $props();

  function toggleChecked(idx: number) {
    const next = items.map((r, i) => (i === idx ? { ...r, checked: !r.checked } : r));
    onChange(next);
  }

  function setCondOverride(idx: number, v: string) {
    const next = items.map((r, i) =>
      i === idx ? { ...r, conditionOverride: v.length > 0 ? v : null } : r,
    );
    onChange(next);
  }

  function setPlaceOverride(idx: number, placeId: number | null) {
    const next = items.map((r, i) => (i === idx ? { ...r, placeIdOverride: placeId } : r));
    onChange(next);
  }
</script>

<div class="rows">
  <div class="thead" role="row">
    <div class="th col-check" aria-label="Выбрано"></div>
    <div class="th col-device">Устройство</div>
    <div class="th col-condition">Состояние</div>
    <div class="th col-place">Место</div>
  </div>

  {#each items as row, idx (`${row.actItemId}-${row.deviceId}`)}
    {@const effectiveCondPlaceholder = applyToAll && bulkCondition ? bulkCondition : ''}
    {@const condOverridden = row.conditionOverride !== null}
    {@const placeOverridden = row.placeIdOverride !== null}
    <div class="tr" class:tr-unchecked={!row.checked} role="row">
      <div class="td col-check">
        <Checkbox checked={row.checked} onchange={() => toggleChecked(idx)}>
          <span class="visually-hidden">Включить позицию {idx + 1} в возврат</span>
        </Checkbox>
      </div>
      <div class="td col-device">
        <span class="device-label">
          {row.deviceName}
          {#if row.inventoryNo}
            (инв. <span class="tr-mono">{row.inventoryNo}</span>)
          {:else}
            #{row.deviceId}
          {/if}
        </span>
      </div>
      <div class="td col-condition">
        {#if row.checked && !applyToAll}
          <DeviceAutocompleteField
            field="state"
            value={row.conditionOverride ?? ''}
            placeholder={effectiveCondPlaceholder || 'Хорошее / Б/У / Среднее'}
            onChange={(v) => setCondOverride(idx, v)}
          />
        {:else}
          <Input
            type="text"
            value={row.conditionOverride ?? ''}
            placeholder={applyToAll && row.checked ? '(по умолчанию)' : ''}
            disabled
          />
        {/if}
        {#if row.checked && applyToAll && !condOverridden}
          <span class="hint hint-default">(по умолчанию)</span>
        {:else if row.checked && condOverridden && !applyToAll}
          <span class="hint hint-warning">(переопределено)</span>
        {/if}
      </div>
      <div class="td col-place">
        {#if row.checked && !applyToAll}
          <PlacePicker
            value={row.placeIdOverride}
            onChange={(id) => setPlaceOverride(idx, id)}
            id={`ret-row-place-${idx}`}
          />
        {:else}
          <PlacePicker
            value={applyToAll ? bulkPlaceId : row.placeIdOverride}
            onChange={() => {}}
            id={`ret-row-place-${idx}`}
            disabled
          />
        {/if}
        {#if row.checked && applyToAll && !placeOverridden}
          <span class="hint hint-default">(по умолчанию)</span>
        {:else if row.checked && placeOverridden && !applyToAll}
          <span class="hint hint-warning">(переопределено)</span>
        {/if}
      </div>
    </div>
  {/each}
</div>

<style lang="scss">
  .rows {
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-xs);
    overflow: visible;
  }
  .thead,
  .tr {
    display: grid;
    grid-template-columns: 40px 1.4fr 1fr 1.4fr;
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
    min-height: 44px;
    &:last-of-type {
      border-bottom: none;
    }
  }
  .tr-unchecked {
    opacity: 0.5;
  }
  .col-check {
    display: flex;
    align-items: center;
    justify-content: center;
    padding-top: 6px;
  }
  .col-condition,
  .col-place {
    font-variant-numeric: tabular-nums;
  }
  .device-label {
    display: inline-block;
    padding-top: var(--tr-space-xs);
    color: var(--tr-text-primary);
    font-size: var(--tr-font-size-body);
  }
  .hint {
    display: block;
    margin-top: var(--tr-space-3xs);
    font-size: var(--tr-font-size-label);
    line-height: 1.2;
  }
  .hint-default {
    color: var(--tr-text-tertiary);
  }
  .hint-warning {
    color: var(--tr-warning);
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
