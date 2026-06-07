<script lang="ts">
  // Plan 04-06: строка списка моделей. По образцу ActListRow.svelte.
  // Kebab: только Редактировать / Удалить (inline меню, без portal — нет перекрытия overflow).
  import Badge from '$lib/components/Badge.svelte';
  import type { CartridgeModelDto } from '../../bindings';

  interface Props {
    model: CartridgeModelDto;
    instanceCount: number;
    onEdit: () => void;
    onDelete: () => void;
  }

  const { model, instanceCount, onEdit, onDelete }: Props = $props();

  let menuOpen = $state(false);

  function toggleMenu(e: MouseEvent) {
    e.stopPropagation();
    menuOpen = !menuOpen;
  }

  function handleClickOutside(e: MouseEvent) {
    if (!menuOpen) return;
    const target = e.target as HTMLElement;
    if (!wrapperEl?.contains(target)) menuOpen = false;
  }

  let wrapperEl = $state<HTMLDivElement | null>(null);

  $effect(() => {
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  });

  function handleEdit() {
    menuOpen = false;
    onEdit();
  }

  function handleDelete() {
    menuOpen = false;
    onDelete();
  }
</script>

<div class="row">
  <div class="top">
    <span class="name">{model.brand} {model.model}</span>
    <span class="badges">
      <Badge variant={model.kind_id === 1 ? 'accent' : 'default'}>
        {model.kind_id === 1 ? 'Картридж' : 'Фотобарабан'}
      </Badge>
      {#if model.kind_id === 1 && model.color}
        <Badge variant="default">{model.color}</Badge>
      {/if}
    </span>
    <div class="kebab-wrap" bind:this={wrapperEl} role="none">
      <button
        type="button"
        class="kebab-btn"
        aria-label="Действия с моделью {model.brand} {model.model}"
        aria-expanded={menuOpen}
        onclick={toggleMenu}
      >
        ⋮
      </button>
      {#if menuOpen}
        <div class="ctx-menu" role="menu">
          <button type="button" class="ctx-menu-item" role="menuitem" onclick={handleEdit}>
            Редактировать
          </button>
          <hr class="ctx-menu-sep" />
          <button
            type="button"
            class="ctx-menu-item ctx-menu-item--destructive"
            role="menuitem"
            onclick={handleDelete}
          >
            Удалить
          </button>
        </div>
      {/if}
    </div>
  </div>
  <div class="bottom">
    <span class="count">{instanceCount} шт.</span>
    {#if model.notes}
      <span class="separator">·</span>
      <span class="notes">{model.notes}</span>
    {/if}
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
  }

  .top {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    font-size: var(--font-size-body);
    line-height: 1.2;
  }

  .name {
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .badges {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    flex-shrink: 0;
  }

  .kebab-wrap {
    position: relative;
    flex-shrink: 0;
  }

  .kebab-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    cursor: pointer;
    color: var(--color-text-secondary);
    font-size: 16px;
    line-height: 1;

    &:hover {
      background: var(--color-surface-sunken);
      color: var(--color-text-primary);
    }

    &:focus-visible {
      outline: none;
      box-shadow: 0 0 0 3px var(--color-accent-focus);
    }
  }

  .ctx-menu {
    position: absolute;
    top: calc(100% + 4px);
    right: 0;
    z-index: 100;
    background: var(--color-surface-raised);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    box-shadow: var(--shadow-md);
    min-width: 160px;
    overflow: hidden;
  }

  .ctx-menu-item {
    display: block;
    width: 100%;
    padding: var(--space-sm) var(--space-md);
    background: transparent;
    border: none;
    text-align: left;
    color: var(--color-text-primary);
    font-family: inherit;
    font-size: var(--font-size-body);
    cursor: pointer;

    &:hover {
      background: var(--color-surface-sunken);
    }

    &--destructive {
      color: var(--color-destructive);
    }
  }

  .ctx-menu-sep {
    margin: var(--space-xs) 0;
    border: none;
    border-top: 1px solid var(--color-border);
  }

  .bottom {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    font-size: var(--font-size-label);
    color: var(--color-text-secondary);
  }

  .count {
    color: var(--color-text-secondary);
    font-variant-numeric: tabular-nums;
  }

  .separator {
    color: var(--color-text-muted);
  }

  .notes {
    color: var(--color-text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
