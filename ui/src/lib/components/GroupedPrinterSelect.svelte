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
    padding: 0 var(--space-xl) 0 var(--space-md);
    background: var(--color-bg);
    color: var(--color-text-primary);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    font-family: var(--font-family-base);
    font-size: var(--font-size-body);
    line-height: var(--line-height-body);
    appearance: none;
    cursor: pointer;

    &:focus-visible {
      outline: none;
      border-color: var(--color-accent);
      box-shadow: 0 0 0 3px var(--color-accent-focus);
    }

    &.invalid {
      border-color: var(--color-destructive);
    }

    &:disabled {
      background: var(--color-surface-sunken);
      color: var(--color-text-muted);
      cursor: not-allowed;
    }

    // Gray section headers for grouped printer options.
    optgroup {
      background: var(--color-surface-sunken);
      color: var(--color-text-secondary);
      font-weight: var(--font-weight-semibold);
      font-style: normal;
    }

    option {
      background: var(--color-bg);
      color: var(--color-text-primary);
      font-weight: var(--font-weight-regular);
    }
  }

  .caret {
    position: absolute;
    right: var(--space-md);
    top: 50%;
    transform: translateY(-50%);
    color: var(--color-text-secondary);
    pointer-events: none;
  }
</style>
