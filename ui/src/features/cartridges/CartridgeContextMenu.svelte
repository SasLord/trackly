<script lang="ts">
  // Plan 04-05: CartridgeContextMenu — status-dependent kebab меню с portal + mousedown-outside close.
  // По образцу DeviceContextMenu.svelte с добавлением status-based пунктов (D-Op-Transitions-01).
  import { portal } from '$lib/utils/portal';
  import type { CartridgeDto } from '../../bindings';

  interface Props {
    cartridge: CartridgeDto;
    onInstall: () => void;
    onReturnToStock: () => void;
    onToRefill: () => void;
    onFromRefill: () => void;
    onWriteOff: () => void;
    onEdit: () => void;
    onDelete: () => void;
  }

  const {
    cartridge,
    onInstall,
    onReturnToStock,
    onToRefill,
    onFromRefill,
    onWriteOff,
    onEdit,
    onDelete,
  }: Props = $props();

  let menuOpen = $state(false);
  let menuX = $state(0);
  let menuY = $state(0);
  let triggerEl = $state<HTMLButtonElement | null>(null);

  // Status-dependent menu items (D-Op-Transitions-01 / UI-SPEC §Контекстное меню)
  type MenuItem =
    | { kind: 'action'; label: string; action: () => void; destructive?: boolean }
    | { kind: 'sep' };

  const menuItems = $derived.by<MenuItem[]>(() => {
    const items: MenuItem[] = [];
    const s = cartridge.status_id;
    // Фотобарабан (kind 2): нет заправки; отработанный (state 6) нельзя
    // устанавливать — только списать (V017 / UAT round 3 №4).
    const isDrum = cartridge.model_kind_id === 2;
    const isWornOut = cartridge.state_id === 6;

    // Status-specific lifecycle actions first
    if (s === 1) {
      // На складе
      if (!(isDrum && isWornOut)) {
        items.push({ kind: 'action', label: 'Установить в принтер', action: onInstall });
      }
      if (!isDrum) {
        items.push({ kind: 'action', label: 'Отправить на заправку', action: onToRefill });
      }
    } else if (s === 2) {
      // В работе
      items.push({ kind: 'action', label: 'Вернуть на склад', action: onReturnToStock });
    } else if (s === 3) {
      // На заправке (только картриджи)
      items.push({ kind: 'action', label: 'Забрать с заправки', action: onFromRefill });
    }
    // status_id === 4 (Списано): no lifecycle actions

    // Common actions: Редактировать (all statuses)
    items.push({ kind: 'action', label: 'Редактировать', action: onEdit });

    // Separator before destructive actions
    items.push({ kind: 'sep' });

    // Destructive: Списать only for status 1 (На складе)
    if (s === 1) {
      items.push({ kind: 'action', label: 'Списать', action: onWriteOff, destructive: true });
    }

    // Delete always available
    items.push({ kind: 'action', label: 'Удалить', action: onDelete, destructive: true });

    return items;
  });

  function toggleMenu() {
    if (menuOpen) {
      menuOpen = false;
      return;
    }
    if (triggerEl) {
      const rect = triggerEl.getBoundingClientRect();
      menuX = rect.right - 160; // 160px — min-width меню
      menuY = rect.bottom + 4;
    }
    menuOpen = true;
  }

  function closeMenu() {
    if (menuOpen) menuOpen = false;
  }

  // Закрыть меню при клике вне его (mousedown на <body>).
  function handleBodyMousedown(e: MouseEvent) {
    if (!menuOpen) return;
    const target = e.target as HTMLElement;
    // Если клик на триггере — toggleMenu уже обработает это.
    if (triggerEl && triggerEl.contains(target)) return;
    // Если клик внутри самого меню — игнорируем.
    if (target.closest('.ctx-menu-portal')) return;
    menuOpen = false;
  }

  function handleMenuItemClick(action: () => void) {
    menuOpen = false;
    action();
  }
</script>

<svelte:window onmousedown={handleBodyMousedown} onscroll={closeMenu} onresize={closeMenu} />

<div class="context-menu-wrapper">
  <button
    bind:this={triggerEl}
    class="kebab-btn"
    type="button"
    onclick={toggleMenu}
    aria-label="Действия с картриджем {cartridge.code}"
    aria-expanded={menuOpen}
    aria-haspopup="menu"
  >
    <span class="dots" aria-hidden="true">⋮</span>
  </button>
</div>

<!--
  Меню рендерится в портале (<body>), поэтому оно не обрезается контейнером
  с overflow:hidden/auto. z-index: 2000 гарантирует видимость поверх всех слоёв.
-->
{#if menuOpen}
  <div
    use:portal
    class="ctx-menu-portal"
    role="menu"
    tabindex="-1"
    style="left:{menuX}px; top:{menuY}px;"
    onkeydown={(e) => {
      if (e.key === 'Escape') menuOpen = false;
    }}
  >
    {#each menuItems as item}
      {#if item.kind === 'sep'}
        <hr class="ctx-menu-sep" />
      {:else}
        <button
          class="ctx-menu-item"
          class:ctx-menu-item--destructive={item.destructive}
          role="menuitem"
          onclick={() => handleMenuItemClick(item.action)}
        >
          {item.label}
        </button>
      {/if}
    {/each}
  </div>
{/if}

<style lang="scss">
  .context-menu-wrapper {
    display: inline-block;
  }

  .kebab-btn {
    background: transparent;
    border: none;
    cursor: pointer;
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-sm);
    color: var(--tr-text-secondary);
    font-size: 18px;
    line-height: 1;
    padding: 0;

    &:hover {
      background: var(--tr-surface);
      color: var(--tr-text-primary);
    }

    &:focus-visible {
      outline: none;
      box-shadow: 0 0 0 3px var(--tr-focus-ring);
    }
  }

  .dots {
    user-select: none;
  }

  /*
   * Глобальные стили для портала.
   * Элемент .ctx-menu-portal перемещён use:portal в <body>, поэтому scoped CSS
   * компонента до него не доходит — нужен :global().
   */
  :global(.ctx-menu-portal) {
    position: fixed;
    z-index: 2000;
    background: var(--tr-surface-raised);
    border: 1px solid var(--tr-border);
    border-radius: var(--radius-sm);
    box-shadow: var(--tr-elev-1);
    min-width: 160px;
    padding: var(--space-xs) 0;
  }

  :global(.ctx-menu-item) {
    display: block;
    width: 100%;
    padding: var(--space-xs) var(--space-md);
    background: transparent;
    border: none;
    text-align: left;
    font-size: var(--font-size-body);
    color: var(--tr-text-primary);
    cursor: pointer;
    white-space: nowrap;
    font-family: var(--font-family-base);

    &:hover {
      background: var(--tr-surface);
    }
  }

  :global(.ctx-menu-item--destructive) {
    color: var(--tr-danger);
  }

  :global(.ctx-menu-sep) {
    border: none;
    border-top: 1px solid var(--tr-border);
    margin: var(--space-xs) 0;
  }
</style>
