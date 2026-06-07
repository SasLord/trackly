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
    onSelect: () => void;
    onMenuAction: (_op: string, _cartridge: CartridgeDto) => void;
  }

  const { cartridge, selected, onSelect, onMenuAction }: Props = $props();

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
    <span class="code" style="font-variant-numeric: tabular-nums">{cartridge.code}</span>
    {#if modelLabel}
      <span class="model">{modelLabel}</span>
    {/if}
    <span class="badge-wrap">
      <Badge variant={statusVariant}>{cartridge.status_name ?? ''}</Badge>
    </span>
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
    border-bottom: 1px solid var(--color-border);
    cursor: pointer;
    border-left: 3px solid transparent;

    &:hover {
      background: var(--color-surface-sunken);
    }

    &:focus-visible {
      outline: none;
      box-shadow: inset 0 0 0 2px var(--color-accent);
    }

    &.selected {
      border-left-color: var(--color-accent);
      background: color-mix(in srgb, var(--color-accent) 8%, transparent);
    }
  }

  .top {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    font-size: var(--font-size-body);
    line-height: 1.2;
  }

  .code {
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
    flex-shrink: 0;
  }

  .model {
    color: var(--color-text-secondary);
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
    color: var(--color-text-secondary);
  }

  .location {
    color: var(--color-text-secondary);
  }
</style>
