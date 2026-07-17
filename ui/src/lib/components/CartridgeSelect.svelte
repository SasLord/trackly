<script lang="ts">
  // D-01/D-02/D-03 (Phase 12 Plan 03): flat (no optgroup) cartridge selector
  // for OperationModal's request-centric install flow. Modeled directly on
  // GroupedPrinterSelect.svelte (DISC-03 — cartridges have no natural
  // location-group, so this is the same select shell without grouping).
  //
  // AUTO-01: этот компонент оборачивает нативный <select> — браузер рендерит
  // option-popup вне DOM-дерева страницы, поэтому overflow: hidden модалки его
  // не обрезает; portal/anchor-слой (см. dropdownAnchor.ts) здесь не требуется.
  // Единственный position: absolute элемент в файле — декоративная
  // caret-иконка (pointer-events: none), не кликабельный список.
  import type { CartridgeDto } from '../../bindings';

  interface Props {
    options: CartridgeDto[];
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
    <option value="">Выберите картридж</option>
    {#if options.length === 0}
      <option value="" disabled>Нет подходящих картриджей на складе</option>
    {:else}
      {#each options as o (o.id)}
        <option value={String(o.id)}>
          {o.code} — {[o.model_brand, o.model_name].filter(Boolean).join(' ')} ({o.state_name ??
            '—'})
        </option>
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
