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
  import LocationAutocomplete from '$lib/components/LocationAutocomplete.svelte';

  export interface ReturnRowState {
    actItemId: number;
    deviceId: number;
    deviceLabel: string;
    /** Per-row checked flag — default true. */
    checked: boolean;
    /** Per-row condition override; null → fallback на bulk (если applyToAll). */
    conditionOverride: string | null;
    /** Локально набираемое имя расположения (autocomplete возвращает строку). */
    locationOverrideName: string;
  }

  interface Props {
    items: ReturnRowState[];
    applyToAll: boolean;
    bulkCondition: string | null;
    bulkLocationName: string;
    onChange: (_items: ReturnRowState[]) => void;
  }

  const { items, applyToAll, bulkCondition, bulkLocationName, onChange }: Props = $props();

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

  function setLocOverrideName(idx: number, v: string) {
    const next = items.map((r, i) => (i === idx ? { ...r, locationOverrideName: v } : r));
    onChange(next);
  }
</script>

<div class="rows">
  <div class="thead" role="row">
    <div class="th col-check" aria-label="Выбрано"></div>
    <div class="th col-device">Устройство</div>
    <div class="th col-condition">Состояние</div>
    <div class="th col-location">Расположение</div>
  </div>

  {#each items as row, idx (`${row.actItemId}-${row.deviceId}`)}
    {@const effectiveCondPlaceholder = applyToAll && bulkCondition ? bulkCondition : ''}
    {@const effectiveLocPlaceholder = applyToAll && bulkLocationName ? bulkLocationName : ''}
    {@const condOverridden = row.conditionOverride !== null}
    {@const locOverridden = row.locationOverrideName.trim().length > 0}
    {@const perRowDisabled = !row.checked || applyToAll}
    <div class="tr" class:tr-unchecked={!row.checked} role="row">
      <div class="td col-check">
        <input
          type="checkbox"
          checked={row.checked}
          onchange={() => toggleChecked(idx)}
          aria-label="Включить позицию {idx + 1} в возврат"
        />
      </div>
      <div class="td col-device">
        <span class="device-label">{row.deviceLabel}</span>
      </div>
      <div class="td col-condition">
        <Input
          type="text"
          value={row.conditionOverride ?? ''}
          placeholder={applyToAll && row.checked
            ? '(по умолчанию)'
            : effectiveCondPlaceholder || 'Хорошее / Б/У / Среднее'}
          disabled={perRowDisabled}
          oninput={(v) => setCondOverride(idx, v)}
        />
        {#if row.checked && applyToAll && !condOverridden}
          <span class="hint hint-default">(по умолчанию)</span>
        {:else if row.checked && condOverridden && !applyToAll}
          <span class="hint hint-warning">(переопределено)</span>
        {/if}
      </div>
      <div class="td col-location">
        {#if row.checked && !applyToAll}
          <LocationAutocomplete
            value={row.locationOverrideName}
            placeholder={effectiveLocPlaceholder || 'Куда вернуть на склад'}
            onChange={(v) => setLocOverrideName(idx, v)}
          />
        {:else}
          <Input
            type="text"
            value={row.locationOverrideName}
            placeholder={applyToAll && row.checked ? '(по умолчанию)' : ''}
            disabled
          />
        {/if}
        {#if row.checked && applyToAll && !locOverridden}
          <span class="hint hint-default">(по умолчанию)</span>
        {:else if row.checked && locOverridden && !applyToAll}
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
  .col-location {
    font-variant-numeric: tabular-nums;
  }
  .device-label {
    display: inline-block;
    padding-top: 8px;
    color: var(--tr-text-primary);
    font-size: var(--tr-font-size-body);
  }
  .hint {
    display: block;
    margin-top: 2px;
    font-size: 13px;
    line-height: 1.2;
  }
  .hint-default {
    color: var(--tr-text-tertiary);
  }
  .hint-warning {
    color: var(--tr-warning);
  }
</style>
