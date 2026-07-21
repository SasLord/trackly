<script lang="ts">
  // Plan 04-06: строка списка моделей. По образцу ActListRow.svelte.
  // Plan 27-04 (D-03): rebuilt on shared TableRow primitive — bespoke `.row` div
  // (name/badges/kebab, count/notes on a second line) replaced with a 4-column
  // table row (модель / экземпляры / примечания / действия); kebab menu markup
  // unchanged (inline, no portal — same as before, no overflow clipping issue
  // inside a <td>: menu positioned `position: absolute` relative to its own
  // wrapper, td has `overflow: visible`).
  import Badge from '$lib/components/Badge.svelte';
  import TableRow from '$lib/components/TableRow.svelte';
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

<TableRow class="model-row">
  <td class="cell cell-name" title="{model.brand} {model.model}">
    <span class="name">{model.brand} {model.model}</span>
    <span class="badges">
      <Badge variant={model.kind_id === 1 ? 'accent' : 'default'} size="sm">
        {model.kind_id === 1 ? 'Картридж' : 'Фотобарабан'}
      </Badge>
      {#if model.kind_id === 1 && model.color}
        <Badge variant="default" size="sm">{model.color}</Badge>
      {/if}
    </span>
  </td>
  <td class="cell cell-count">{instanceCount} шт.</td>
  <td class="cell cell-notes" title={model.notes ?? ''}>{model.notes ?? '—'}</td>
  <td class="cell cell-actions">
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
  </td>
</TableRow>

<style lang="scss">
  .cell {
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-primary);
  }

  // FIX F4 (Phase 27 batch F): name is the PRIMARY, prominent element — a
  // flex-row layout let the (flex-shrink:0) badges squeeze the name span down
  // to near-nothing on narrow columns, making rows indistinguishable. Stacking
  // name (top, larger) above badges (bottom, small/secondary) guarantees full
  // width for the name regardless of badge count/width.
  .cell-name {
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 2px;
    max-width: 0; // makes text-overflow work in table cells
  }

  .name {
    font-size: var(--tr-font-size-body);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }

  .badges {
    display: flex;
    align-items: center;
    gap: var(--tr-space-2xs);
    flex-shrink: 0;
  }

  .cell-count {
    width: 130px;
    text-align: right;
    font-variant-numeric: tabular-nums;
    color: var(--tr-text-secondary);
  }

  .cell-notes {
    color: var(--tr-text-tertiary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 0;
  }

  .cell-actions {
    width: 40px;
    text-align: center;
    overflow: visible;
  }

  .kebab-wrap {
    position: relative;
    display: inline-flex;
  }

  .kebab-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    background: transparent;
    border: none;
    border-radius: var(--tr-radius-xs);
    cursor: pointer;
    color: var(--tr-text-secondary);
    font-size: 16px;
    line-height: 1;

    &:hover {
      background: var(--tr-surface-sunken);
      color: var(--tr-text-primary);
    }

    &:focus-visible {
      outline: none;
      box-shadow: 0 0 0 3px var(--tr-focus-ring);
    }
  }

  .ctx-menu {
    position: absolute;
    top: calc(100% + 4px);
    right: 0;
    z-index: 100;
    background: var(--tr-surface-raised);
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-xs);
    box-shadow: var(--tr-elev-2);
    min-width: 160px;
    overflow: hidden;
  }

  .ctx-menu-item {
    display: block;
    width: 100%;
    padding: var(--tr-space-xs) var(--tr-space-md);
    background: transparent;
    border: none;
    text-align: left;
    color: var(--tr-text-primary);
    font-family: inherit;
    font-size: var(--tr-font-size-body);
    cursor: pointer;

    &:hover {
      background: var(--tr-surface-sunken);
    }

    &--destructive {
      color: var(--tr-danger);
    }
  }

  .ctx-menu-sep {
    margin: var(--tr-space-2xs) 0;
    border: none;
    border-top: 1px solid var(--tr-border);
  }
</style>
