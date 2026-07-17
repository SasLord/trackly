<script lang="ts">
  // D-PRN-01 (Phase 11 Plan 02): printer dropdown grouped by location, with
  // gray section headers. Replaces the flat <Select> previously used in
  // RequestFormModal.svelte once the data source switched from the closed
  // devices.list({type_id:2}) call to requests.printerOptions() (minimal DTO:
  // id/name/location only).
  //
  // Server already sorts options by location then name, no-location last
  // (RequestService::printer_options ORDER BY clause) — this component only
  // groups for rendering, it does not re-sort.
  //
  // AUTO-01: этот компонент оборачивает нативный <select> — браузер рендерит
  // option-popup вне DOM-дерева страницы, поэтому overflow: hidden модалки его
  // не обрезает; portal/anchor-слой (см. dropdownAnchor.ts) здесь не требуется.
  // Единственный position: absolute элемент в файле — декоративная
  // caret-иконка (pointer-events: none), не кликабельный список.
  import type { RequestPrinterOptionDto } from '../../bindings-phase6';

  interface Props {
    options: RequestPrinterOptionDto[];
    value: string;
    disabled?: boolean;
    invalid?: boolean;
    id?: string;
    onchange?: (_value: string) => void;
  }

  const {
    options,
    value = $bindable(''),
    disabled = false,
    invalid = false,
    id,
    onchange,
  }: Props = $props();

  const NO_LOCATION_LABEL = 'Без расположения';

  // Group by location label, preserving server-provided order (server sorts
  // location-having groups alphabetically already; the no-location group is
  // already last in `options` since the server appends it last).
  const groups = $derived.by(() => {
    const map = new Map<string, RequestPrinterOptionDto[]>();
    for (const opt of options) {
      const label = opt.location ?? NO_LOCATION_LABEL;
      const bucket = map.get(label);
      if (bucket) {
        bucket.push(opt);
      } else {
        map.set(label, [opt]);
      }
    }
    return Array.from(map.entries());
  });
</script>

<div class="select-wrapper">
  <select
    {id}
    {disabled}
    class="select"
    class:invalid
    {value}
    onchange={(e) => {
      const v = (e.currentTarget as HTMLSelectElement).value;
      onchange?.(v);
    }}
  >
    <option value="">Выберите принтер</option>
    {#if options.length === 0}
      <option value="" disabled>Принтеры не найдены</option>
    {:else}
      {#each groups as [label, printers] (label)}
        <optgroup {label}>
          {#each printers as p (p.id)}
            <option value={String(p.id)}>{p.name || `Принтер #${p.id}`}</option>
          {/each}
        </optgroup>
      {/each}
    {/if}
  </select>
  <!-- Caret icon -->
  <svg class="caret" width="12" height="12" viewBox="0 0 12 12" fill="none" aria-hidden="true">
    <path
      d="M2 4l4 4 4-4"
      stroke="currentColor"
      stroke-width="1.5"
      stroke-linecap="round"
      stroke-linejoin="round"
    />
  </svg>
</div>

<style lang="scss">
  .select-wrapper {
    position: relative;
    display: block;
    width: 100%;
  }

  .select {
    display: block;
    width: 100%;
    height: 36px;
    padding: 0 var(--tr-space-2xl) 0 var(--tr-space-md);
    background: var(--tr-bg);
    color: var(--tr-text-primary);
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-xs);
    font-family: var(--font-family-base);
    font-size: var(--font-size-body);
    line-height: var(--line-height-body);
    appearance: none;
    cursor: pointer;

    &:focus-visible {
      outline: none;
      border-color: var(--tr-accent);
      box-shadow: 0 0 0 3px var(--tr-focus-ring);
    }

    &.invalid {
      border-color: var(--tr-danger);
    }

    &:disabled {
      background: var(--tr-surface-sunken);
      color: var(--tr-text-tertiary);
      cursor: not-allowed;
    }

    // Gray section headers for grouped printer options.
    optgroup {
      background: var(--tr-surface-sunken);
      color: var(--tr-text-secondary);
      font-weight: var(--font-weight-semibold);
      font-style: normal;
    }

    option {
      background: var(--tr-bg);
      color: var(--tr-text-primary);
      font-weight: var(--font-weight-regular);
    }
  }

  .caret {
    position: absolute;
    right: var(--tr-space-md);
    top: 50%;
    transform: translateY(-50%);
    color: var(--tr-text-secondary);
    pointer-events: none;
  }
</style>
