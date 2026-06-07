<script lang="ts">
  // Plan 04-04: строка списка картриджей.
  // По образцу ActListRow.svelte, паттерн из PATTERNS.md §CartridgeListRow.svelte.
  import Badge from '$lib/components/Badge.svelte';
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

  function handleKebabClick(e: MouseEvent) {
    e.stopPropagation();
    // CartridgeContextMenu wire-up в плане 04-05; сейчас — заглушка.
    onMenuAction('menu', cartridge);
  }

  function handleKebabKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      e.stopPropagation();
      onMenuAction('menu', cartridge);
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
    <button
      type="button"
      class="kebab-btn"
      aria-label="Действия с картриджем {cartridge.code}"
      onclick={handleKebabClick}
      onkeydown={handleKebabKeydown}
      tabindex="-1">⋮</button
    >
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

  .kebab-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    flex-shrink: 0;
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--color-text-muted);
    cursor: pointer;
    font-size: 16px;
    padding: 0;

    &:hover {
      background: var(--color-surface-sunken);
      color: var(--color-text-primary);
    }

    &:focus-visible {
      outline: none;
      box-shadow: 0 0 0 2px var(--color-accent-focus);
    }
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
