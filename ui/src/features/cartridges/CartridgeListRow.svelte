<script lang="ts">
  // Plan 04-04: строка списка картриджей.
  // Plan 04-05: kebab заглушка заменена на CartridgeContextMenu с portal (wire-up).
  // Plan 27-04 (D-03): rebuilt on shared TableRow primitive per DeviceListRow.svelte/
  // ActListRow.svelte precedent — bespoke two-line `.row` div replaced with a
  // table row (код+заряд / модель / расположение / статус / действия); select state
  // now via TableRow's `selected` prop, not bespoke `.row.selected`.
  // NOTE: TableRow.svelte does not forward onclick/role/tabindex to its own <tr> —
  // row click/keyboard-select is wired on the <td> cells we own here (onclick on
  // every non-kebab cell for full-row mouse click; role="button"+tabindex+onkeydown
  // on the first cell as the single keyboard entry point).
  import Badge from '$lib/components/Badge.svelte';
  import TableRow from '$lib/components/TableRow.svelte';
  import CartridgeContextMenu from './CartridgeContextMenu.svelte';
  import type { CartridgeDto } from '../../bindings';

  interface Props {
    cartridge: CartridgeDto;
    selected: boolean;
    /** Скрывать колонку статуса (список уже отфильтрован по статусу). */
    statusFiltered?: boolean;
    onSelect: () => void;
    onMenuAction: (_op: string, _cartridge: CartridgeDto) => void;
  }

  const { cartridge, selected, statusFiltered = false, onSelect, onMenuAction }: Props = $props();

  // Badge variant по status_id (UI-SPEC §Badge-цвета статусов):
  // 1→success, 2→accent, 3→warning, 4→default
  type BadgeVariant = 'success' | 'accent' | 'warning' | 'default';

  const statusVariant = $derived<BadgeVariant>(
    cartridge.status_id === 1
      ? 'success'
      : cartridge.status_id === 2
        ? 'accent'
        : cartridge.status_id === 3
          ? 'warning'
          : 'default',
  );

  // Индикатор состояния по state_id. Картриджи: 1 Полный, 2 Частичный, 3 Пустой.
  // Фотобарабаны (V017): 4 Новый, 5 Изношенный, 6 Отработанный. Цвет по уровню
  // «годности»: хорошее → зелёный, среднее → янтарный, плохое → красный.
  // Списанные (status_id === 4) — всегда серый (UAT R3 №2). Барабаны тоже
  // должны окрашиваться по состоянию, а не быть серыми (UAT R4 №2).
  const chargeColor = $derived(
    cartridge.status_id === 4
      ? 'var(--tr-text-tertiary)'
      : cartridge.state_id === 1 || cartridge.state_id === 4
        ? 'var(--tr-success)'
        : cartridge.state_id === 2 || cartridge.state_id === 5
          ? 'var(--tr-warning)'
          : cartridge.state_id === 3 || cartridge.state_id === 6
            ? 'var(--tr-danger)'
            : 'var(--tr-border)',
  );
  const chargeTitle = $derived(
    cartridge.state_name ? `Заряд: ${cartridge.state_name}` : 'Заряд неизвестен',
  );

  function handleClick() {
    onSelect();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      onSelect();
    }
  }

  const modelLabel = $derived(
    cartridge.model_brand || cartridge.model_name
      ? `${cartridge.model_brand ?? ''} ${cartridge.model_name ?? ''}`.trim()
      : null,
  );
</script>

<TableRow {selected} class="cartridge-row">
  <td
    id="cartridge-row-{cartridge.id}"
    class="cell cell-code"
    role="button"
    tabindex="0"
    aria-pressed={selected}
    onclick={handleClick}
    onkeydown={handleKeydown}
  >
    <span class="cell-code-inner">
      <span
        class="charge-dot"
        style="background: {chargeColor}"
        title={chargeTitle}
        aria-label={chargeTitle}
      ></span>
      <span class="tr-mono">{cartridge.code}</span>
    </span>
  </td>
  <td class="cell" title={modelLabel ?? ''} onclick={handleClick}>{modelLabel ?? '—'}</td>
  <td class="cell" title={cartridge.full_path ?? ''} onclick={handleClick}
    >{cartridge.full_path ?? '—'}</td
  >
  {#if !statusFiltered}
    <td class="cell cell-status" onclick={handleClick}>
      <Badge variant={statusVariant}>{cartridge.status_name ?? ''}</Badge>
    </td>
  {/if}
  <td class="cell cell-actions">
    <CartridgeContextMenu
      {cartridge}
      onInstall={() => onMenuAction('install', cartridge)}
      onReturnToStock={() => onMenuAction('return_to_stock', cartridge)}
      onToRefill={() => onMenuAction('to_refill', cartridge)}
      onFromRefill={() => onMenuAction('from_refill', cartridge)}
      onWriteOff={() => onMenuAction('write_off', cartridge)}
      onEdit={() => onMenuAction('edit', cartridge)}
      onDelete={() => onMenuAction('delete', cartridge)}
    />
  </td>
</TableRow>

<style lang="scss">
  // TableRow renders its own <tr> (a DIFFERENT Svelte scope-hash than this
  // file) — caller-supplied class needs `:global()`, and the ancestor part of
  // the selector must stay in THIS file's scope per the TableRow contract:
  // `.cartridge-row :global(> td)`, never `:global(.cartridge-row > td)`.
  :global(tr.cartridge-row) {
    cursor: pointer;
  }

  .cell {
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 0; // makes text-overflow work in table cells
  }

  // FIX B3: `display: flex` on the <td> ITSELF overrides `display: table-cell`,
  // pulling the cell out of the table's column model — every column collapses/
  // overlaps. The <td> stays a normal table cell (width + cursor + focus ring
  // only); the flex layout lives on the inner span below.
  .cell-code {
    width: 140px;
    cursor: pointer;

    &:focus-visible {
      // ring теперь на уровне строки, см. TableRow.svelte .tr-row:has(:focus-visible) (Gap 4, план 30-05)
      // check-focus-outline: ignore
      outline: none;
    }
  }

  .cell-code-inner {
    display: flex;
    align-items: center;
    gap: var(--tr-space-2xs);
  }

  .charge-dot {
    flex-shrink: 0;
    width: 10px;
    height: 10px;
    border-radius: 50%;
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--tr-text-primary) 12%, transparent);
  }

  .cell-status {
    width: 120px;
  }

  .cell-actions {
    width: 40px;
    text-align: center;
    overflow: visible;
  }
</style>
