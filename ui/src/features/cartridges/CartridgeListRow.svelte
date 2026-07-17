<script lang="ts">
  // Plan 04-04: строка списка картриджей.
  // Plan 04-05: kebab заглушка заменена на CartridgeContextMenu с portal (wire-up).
  // По образцу ActListRow.svelte, паттерн из PATTERNS.md §CartridgeListRow.svelte.
  import Badge from '$lib/components/Badge.svelte';
  import CartridgeContextMenu from './CartridgeContextMenu.svelte';
  import type { CartridgeDto } from '../../bindings';

  interface Props {
    cartridge: CartridgeDto;
    selected: boolean;
    /** Скрывать бейдж статуса (список уже отфильтрован по статусу). */
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

<div
  class="row"
  class:selected
  role="button"
  tabindex="0"
  aria-pressed={selected}
  onclick={handleClick}
  onkeydown={handleKeydown}
>
  <div class="top">
    <span
      class="charge-dot"
      style="background: {chargeColor}"
      title={chargeTitle}
      aria-label={chargeTitle}
    ></span>
    <span class="code" style="font-variant-numeric: tabular-nums">{cartridge.code}</span>
    {#if modelLabel}
      <span class="model">{modelLabel}</span>
    {/if}
    {#if !statusFiltered}
      <span class="badge-wrap">
        <Badge variant={statusVariant}>{cartridge.status_name ?? ''}</Badge>
      </span>
    {/if}
    <span
      class="kebab-wrap"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
      role="none"
    >
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
    </span>
  </div>
  <div class="bottom">
    <span class="location">{cartridge.location ?? '—'}</span>
  </div>
</div>

<style lang="scss">
  .row {
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: var(--space-xs);
    min-height: var(--row-height, 40px);
    padding: var(--space-sm) var(--space-md);
    border-bottom: 1px solid var(--tr-border);
    cursor: pointer;
    border-left: 3px solid transparent;

    &:hover {
      background: var(--tr-surface-sunken);
    }

    &:focus-visible {
      outline: none;
      box-shadow: inset 0 0 0 2px var(--tr-accent);
    }

    &.selected {
      border-left-color: var(--tr-accent);
      background: color-mix(in srgb, var(--tr-accent) 8%, transparent);
    }
  }

  .top {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    font-size: var(--font-size-body);
    line-height: 1.2;
  }

  .charge-dot {
    flex-shrink: 0;
    width: 10px;
    height: 10px;
    border-radius: 50%;
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--tr-text-primary) 12%, transparent);
  }

  .code {
    font-weight: var(--font-weight-semibold);
    color: var(--tr-text-primary);
    flex-shrink: 0;
  }

  .model {
    color: var(--tr-text-secondary);
    font-size: var(--font-size-label);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
    min-width: 0;
  }

  .badge-wrap {
    flex-shrink: 0;
    margin-left: auto;
  }

  .kebab-wrap {
    flex-shrink: 0;
    display: flex;
    align-items: center;
  }

  .bottom {
    display: flex;
    align-items: center;
    font-size: var(--font-size-label);
    color: var(--tr-text-secondary);
  }

  .location {
    color: var(--tr-text-secondary);
  }
</style>
