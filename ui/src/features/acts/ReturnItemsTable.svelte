<script lang="ts">
  // Plan 03-03: per-row table inside ReturnModal.
  //
  // Columns: ☑ checkbox · Устройство · Кол-во к возврату · Состояние · Расположение.
  // Per-row visual: unchecked → opacity 0.5 + disabled inputs.
  // Per-row annotation: «(по умолчанию)» когда bulk-default используется,
  // «(переопределено)» когда user задал per-row значение.
  //
  // applyToAll = false → bulk не применяется; per-row override обязателен (валидация
  // на бэке через AppError::Validation).
  import Input from '$lib/components/Input.svelte';
  import DeviceAutocompleteField from '../devices/DeviceAutocompleteField.svelte';

  export interface ReturnRowState {
    actItemId: number;
    deviceId: number;
    deviceLabel: string;
    quantity: number;
    /** Per-row checked flag — default true. */
    checked: boolean;
    /** Per-row condition override; null → fallback на bulk (если applyToAll). */
    conditionOverride: string | null;
    /** Per-row location override (locations.id); null → fallback на bulk. */
    locationOverrideId: number | null;
    /** Локально набираемое имя расположения (autocomplete возвращает строку). */
    locationOverrideName: string;
  }

  interface Props {
    items: ReturnRowState[];
    applyToAll: boolean;
    bulkCondition: string | null;
    bulkLocationName: string;
    /** Map имя расположения → id (заполняется родителем по мере выбора). */
    onChange: (_items: ReturnRowState[]) => void;
  }

  const { items, applyToAll, bulkCondition, bulkLocationName, onChange }: Props = $props();

  function toggleChecked(idx: number) {
    const next = items.map((r, i) => (i === idx ? { ...r, checked: !r.checked } : r));
    onChange(next);
  }

  function setQty(idx: number, v: string) {
    const parsed = parseInt(v, 10);
    const qty = Number.isFinite(parsed) && parsed > 0 ? parsed : 1;
    const next = items.map((r, i) => (i === idx ? { ...r, quantity: qty } : r));
    onChange(next);
  }

  function setCondOverride(idx: number, v: string) {
    const next = items.map((r, i) =>
      i === idx ? { ...r, conditionOverride: v.length > 0 ? v : null } : r,
    );
    onChange(next);
  }

  function setLocOverrideName(idx: number, v: string) {
    // Сбрасываем id при изменении строки — родитель resolve'ит имя в id при submit
    // (resolve через bulk_location_id или через server-side INSERT OR IGNORE locations).
    const next = items.map((r, i) =>
      i === idx ? { ...r, locationOverrideName: v, locationOverrideId: null } : r,
    );
    onChange(next);
  }
</script>

<div class="rows">
  <div class="thead" role="row">
    <div class="th col-check" aria-label="Выбрано"></div>
    <div class="th col-device">Устройство</div>
    <div class="th col-qty">Кол-во к возврату</div>
    <div class="th col-condition">Состояние</div>
    <div class="th col-location">Расположение</div>
  </div>

  {#each items as row, idx (row.actItemId)}
    {@const effectiveCondPlaceholder = applyToAll && bulkCondition ? bulkCondition : ''}
    {@const effectiveLocPlaceholder = applyToAll && bulkLocationName ? bulkLocationName : ''}
    {@const condOverridden = row.conditionOverride !== null}
    {@const locOverridden = row.locationOverrideName.trim().length > 0}
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
      <div class="td col-qty">
        <Input
          type="number"
          value={String(row.quantity)}
          disabled={!row.checked}
          oninput={(v) => setQty(idx, v)}
        />
      </div>
      <div class="td col-condition">
        <Input
          type="text"
          value={row.conditionOverride ?? ''}
          placeholder={effectiveCondPlaceholder || 'Хорошее / Б/У / Среднее'}
          disabled={!row.checked}
          oninput={(v) => setCondOverride(idx, v)}
        />
        {#if row.checked && applyToAll && !condOverridden}
          <span class="hint hint-default">(по умолчанию)</span>
        {:else if row.checked && condOverridden}
          <span class="hint hint-warning">(переопределено)</span>
        {/if}
      </div>
      <div class="td col-location">
        {#if row.checked}
          <DeviceAutocompleteField
            field="location"
            value={row.locationOverrideName}
            placeholder={effectiveLocPlaceholder || 'Куда вернуть на склад'}
            statusIn={['на_складе']}
            onChange={(v) => setLocOverrideName(idx, v)}
          />
        {:else}
          <Input type="text" value="" disabled />
        {/if}
        {#if row.checked && applyToAll && !locOverridden}
          <span class="hint hint-default">(по умолчанию)</span>
        {:else if row.checked && locOverridden}
          <span class="hint hint-warning">(переопределено)</span>
        {/if}
      </div>
    </div>
  {/each}
</div>

<style lang="scss">
  .rows {
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    overflow: visible;
  }
  .thead,
  .tr {
    display: grid;
    grid-template-columns: 40px 1fr 140px 1fr 1.4fr;
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
  .col-qty,
  .col-condition,
  .col-location {
    font-variant-numeric: tabular-nums;
  }
  .device-label {
    display: inline-block;
    padding-top: 8px;
    color: var(--color-text-primary);
    font-size: var(--font-size-body);
  }
  .hint {
    display: block;
    margin-top: 2px;
    font-size: 13px;
    line-height: 1.2;
  }
  .hint-default {
    color: var(--color-text-muted);
  }
  .hint-warning {
    color: var(--color-warning, #b45309);
  }
</style>
