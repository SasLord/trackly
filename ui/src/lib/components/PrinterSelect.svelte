<script lang="ts">
  // Plan 12-20 (Round 4 gap-closure, D-20/D-21): printer selector for the
  // cartridge-centric install flow (OperationModal, menu → «Установить в
  // принтер»). Skeleton copied from GroupedPrinterSelect.svelte (same
  // <select>/<optgroup>/caret SVG markup + identical SCSS block), but groups
  // by COMPATIBILITY (via a caller-supplied `compatibleDeviceIds: Set<number>`
  // reverse-lookup from cartridge_models_get_compatible_devices), not by
  // location — and operates on the full `PrinterDto[]` shape (not the
  // minimal `RequestPrinterOptionDto`).
  //
  // D-21: compatible printers are prioritized (separate optgroup, shown
  // first) but the rest of the fleet is still shown below, never blocked —
  // mirrors the existing D-13/D-14 "compatibility not configured" fallback
  // logic, just reversed (printers-by-cartridge instead of
  // cartridges-by-printer).
  //
  // AUTO-01: этот компонент оборачивает нативный <select> — браузер рендерит
  // option-popup вне DOM-дерева страницы, поэтому overflow: hidden модалки его
  // не обрезает; portal/anchor-слой (см. dropdownAnchor.ts) здесь не требуется.
  // Единственный position: absolute элемент в файле — декоративная
  // caret-иконка (pointer-events: none), не кликабельный список.
  import type { PrinterDto } from '../../bindings-phase6';

  interface Props {
    options: PrinterDto[];
    compatibleDeviceIds: Set<number>;
    value: string;
    disabled?: boolean;
    invalid?: boolean;
    id?: string;
    onchange?: (_value: string) => void;
  }

  const {
    options,
    compatibleDeviceIds,
    value = $bindable(''),
    disabled = false,
    invalid = false,
    id,
    onchange,
  }: Props = $props();

  function printerLabel(p: PrinterDto): string {
    const name = p.deviceName ?? `Принтер #${p.deviceId}`;
    return p.devicePlace ? `${name} — ${p.devicePlace}` : name;
  }

  // D-21: when no compatibility links exist for this cartridge model, render
  // a single flat group (no optgroup headers) — fallback, not a block.
  // Otherwise split into «Совместимые принтеры» (first) and «Остальные
  // принтеры» (rest, still shown, never hidden/blocked).
  const groups = $derived.by((): [string, PrinterDto[]][] => {
    if (compatibleDeviceIds.size === 0) {
      return [['', options]];
    }
    const compatible: PrinterDto[] = [];
    const rest: PrinterDto[] = [];
    for (const p of options) {
      if (compatibleDeviceIds.has(p.deviceId)) {
        compatible.push(p);
      } else {
        rest.push(p);
      }
    }
    const result: [string, PrinterDto[]][] = [];
    if (compatible.length > 0) result.push(['Совместимые принтеры', compatible]);
    if (rest.length > 0) result.push(['Остальные принтеры', rest]);
    return result;
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
    <option value="">Без привязки к принтеру</option>
    {#if options.length === 0}
      <option value="" disabled>Принтеры не найдены</option>
    {:else if compatibleDeviceIds.size === 0}
      {#each groups as [, printers] (printers)}
        {#each printers as p (p.id)}
          <option value={String(p.deviceId)}>{printerLabel(p)}</option>
        {/each}
      {/each}
    {:else}
      {#each groups as [label, printers] (label)}
        <optgroup {label}>
          {#each printers as p (p.id)}
            <option value={String(p.deviceId)}>{printerLabel(p)}</option>
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
    font-family: var(--tr-font-family);
    font-size: var(--tr-font-size-body);
    line-height: var(--tr-line-height-body);
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
      font-weight: var(--tr-font-weight-semibold);
      font-style: normal;
    }

    option {
      background: var(--tr-bg);
      color: var(--tr-text-primary);
      font-weight: var(--tr-font-weight-regular);
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
