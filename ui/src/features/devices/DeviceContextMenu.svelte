<script module lang="ts">
  // Shared across ALL DeviceContextMenu instances: only one menu may be open at a
  // time. Opening a menu closes any previously-open one — fixes the bug where you
  // could Tab from one kebab to the next and stack multiple open menus.
  let closeCurrentlyOpenMenu: (() => void) | null = null;
</script>

<script lang="ts">
  import { tick } from 'svelte';
  import Button from '$lib/components/Button.svelte';
  import Modal from '$lib/components/Modal.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { devices } from './api';
  import { portal } from '$lib/utils/portal';
  import type { DeviceDto } from '../../bindings';

  interface Props {
    device: DeviceDto;
    onEdit: (_d: DeviceDto) => void;
    onDelete: () => void;
    /** Plan 03-05 (DEV-14): открыть intermediate-модал документа приёма
     *  для этого устройства. Optional — если не передан, пункт меню скрыт. */
    onPrintAcceptance?: (_d: DeviceDto) => void;
  }

  const { device, onEdit, onDelete, onPrintAcceptance }: Props = $props();

  let menuOpen = $state(false);
  let confirmOpen = $state(false);
  let deleting = $state(false);

  // Координаты плавающего меню в viewport (px).
  let menuX = $state(0);
  let menuY = $state(0);

  // Ссылка на кнопку-триггер (⋮) и на само меню (для управления фокусом).
  let triggerEl = $state<HTMLButtonElement | null>(null);
  let menuEl = $state<HTMLDivElement | null>(null);

  async function openMenu() {
    // Закрыть любое другое открытое меню (fix «висящих» меню при Tab).
    closeCurrentlyOpenMenu?.();
    if (triggerEl) {
      const rect = triggerEl.getBoundingClientRect();
      // Позиционируем меню так, чтобы оно открывалось вниз и вправо от кнопки,
      // выровнено по правому краю кнопки.
      menuX = rect.right - 160; // 160px — min-width меню
      menuY = rect.bottom + 4;
    }
    menuOpen = true;
    closeCurrentlyOpenMenu = closeMenu;
    // Перевести фокус на первый пункт ПОСЛЕ того, как DOM обновился и use:portal
    // перенёс меню в <body>. Через `tick()` — иначе фокус ставится до переноса
    // узла (detach→attach сбрасывает фокус), и клавиатурой по меню не пройти.
    await tick();
    menuEl?.querySelector<HTMLElement>('.ctx-menu-item')?.focus();
  }

  /** Закрыть меню. `returnFocus` — вернуть фокус на кнопку-триггер (для клавиатуры). */
  function closeMenu(returnFocus = false) {
    if (!menuOpen) return;
    menuOpen = false;
    if (closeCurrentlyOpenMenu === closeMenu) closeCurrentlyOpenMenu = null;
    if (returnFocus) triggerEl?.focus();
  }

  function toggleMenu() {
    if (menuOpen) closeMenu(true);
    else openMenu();
  }

  // Клавиатурная навигация внутри меню: стрелки/Home/End — между пунктами,
  // Escape — закрыть и вернуть фокус на триггер, Tab — закрыть (не оставлять висеть).
  function handleMenuKeydown(e: KeyboardEvent) {
    if (!menuEl) return;
    const items = Array.from(menuEl.querySelectorAll<HTMLElement>('.ctx-menu-item'));
    if (items.length === 0) return;
    const idx = items.indexOf(document.activeElement as HTMLElement);
    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault();
        items[(idx + 1) % items.length]?.focus();
        break;
      case 'ArrowUp':
        e.preventDefault();
        items[(idx - 1 + items.length) % items.length]?.focus();
        break;
      case 'Home':
        e.preventDefault();
        items[0]?.focus();
        break;
      case 'End':
        e.preventDefault();
        items[items.length - 1]?.focus();
        break;
      case 'Escape':
        e.preventDefault();
        closeMenu(true);
        break;
      case 'Tab':
        // Не оставляем меню висеть при уходе фокусом — закрываем и возвращаем
        // фокус на триггер, дальше пользователь табает штатно.
        e.preventDefault();
        closeMenu(true);
        break;
    }
  }

  function handleEdit() {
    closeMenu();
    onEdit(device);
  }

  function handlePrintAcceptance() {
    closeMenu();
    onPrintAcceptance?.(device);
  }

  function openConfirm() {
    closeMenu();
    confirmOpen = true;
  }

  async function handleDelete() {
    deleting = true;
    try {
      await devices.delete(device.id, device.version);
      pushToast('success', 'Устройство удалено');
      confirmOpen = false;
      onDelete();
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось удалить устройство';
      pushToast('error', msg);
    } finally {
      deleting = false;
    }
  }

  // Закрыть меню при клике вне его (mousedown на <body>).
  function handleBodyMousedown(e: MouseEvent) {
    if (!menuOpen) return;
    const target = e.target as HTMLElement;
    // Если клик на триггере — toggleMenu уже обработает это.
    if (triggerEl && triggerEl.contains(target)) return;
    // Если клик внутри самого меню — игнорируем (клик по пункту закроет сам).
    if (target.closest('.ctx-menu-portal')) return;
    closeMenu();
  }

  // Закрыть меню при прокрутке или ресайзе — простейший способ избежать
  // «висящего» меню с устаревшими координатами.
  function handleScrollOrResize() {
    if (menuOpen) closeMenu();
  }
</script>

<svelte:window
  onmousedown={handleBodyMousedown}
  onscroll={handleScrollOrResize}
  onresize={handleScrollOrResize}
/>

<div class="context-menu-wrapper">
  <button
    bind:this={triggerEl}
    class="kebab-btn"
    onclick={toggleMenu}
    aria-label="Действия с устройством"
    aria-expanded={menuOpen}
    aria-haspopup="menu"
  >
    <span class="dots">⋮</span>
  </button>
</div>

<!--
  Меню рендерится в портале (<body>), поэтому оно не обрезается контейнером
  с overflow:hidden/auto. z-index: 2000 гарантирует видимость поверх всех слоёв.
-->
{#if menuOpen}
  <div
    bind:this={menuEl}
    use:portal
    class="ctx-menu-portal"
    role="menu"
    tabindex="-1"
    style="left:{menuX}px; top:{menuY}px;"
    onkeydown={handleMenuKeydown}
  >
    <button class="ctx-menu-item" role="menuitem" onclick={handleEdit}> Редактировать </button>
    {#if onPrintAcceptance}
      <button class="ctx-menu-item" role="menuitem" onclick={handlePrintAcceptance}>
        Печать документа приёма
      </button>
    {/if}
    <hr class="ctx-menu-sep" />
    <button class="ctx-menu-item ctx-menu-item--destructive" role="menuitem" onclick={openConfirm}>
      Удалить
    </button>
  </div>
{/if}

<Modal open={confirmOpen} title="Удалить устройство?" onClose={() => (confirmOpen = false)}>
  <p class="confirm-body">
    «{device.name}» (инв. № {device.inventory_no ?? '—'}) будет помечено как удалённое. Действие
    можно отменить только восстановлением из резервной копии БД.
  </p>

  {#snippet footer()}
    <Button variant="secondary" onclick={() => (confirmOpen = false)}>Отмена</Button>
    <Button variant="destructive" loading={deleting} onclick={handleDelete}>Удалить</Button>
  {/snippet}
</Modal>

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
    border-radius: var(--tr-radius-xs);
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

  .confirm-body {
    margin: 0;
    color: var(--tr-text-secondary);
    line-height: var(--tr-line-height-body);
    text-align: center;
    overflow-wrap: anywhere;
    word-break: break-word;
    white-space: normal;
    max-width: 100%;
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
    border-radius: var(--tr-radius-xs);
    box-shadow: var(--tr-elev-1);
    min-width: 160px;
    padding: var(--tr-space-2xs) 0;
  }

  :global(.ctx-menu-item) {
    display: block;
    width: 100%;
    padding: var(--tr-space-2xs) var(--tr-space-md);
    background: transparent;
    border: none;
    text-align: left;
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-primary);
    cursor: pointer;
    white-space: nowrap;
    font-family: var(--tr-font-family);

    &:hover {
      background: var(--tr-row-hover);
    }
  }

  // Focus highlight for menu items. Use `:focus`, NOT `:focus-visible`: we move
  // focus into the menu programmatically (openMenu → tick → item.focus()), and
  // browsers do not reliably apply `:focus-visible` to a scripted focus — so the
  // highlight would not show. `:focus` always matches when the item is focused,
  // which is exactly the ARIA-menu "active item" indicator. The app-wide outward
  // ring is suppressed here (it would spill past the narrow portal), replaced by a
  // background tint. Must be --tr-row-hover, NOT --tr-surface: the menu background
  // is --tr-surface-raised and in LIGHT theme --tr-surface == --tr-surface-raised
  // (both #ffffff), so a --tr-surface highlight was invisible.
  :global(.ctx-menu-item:focus) {
    outline: none;
    box-shadow: none;
    background: var(--tr-row-hover);
  }

  :global(.ctx-menu-item--destructive) {
    color: var(--tr-danger);
  }

  :global(.ctx-menu-sep) {
    border: none;
    border-top: 1px solid var(--tr-border);
    margin: var(--tr-space-2xs) 0;
  }
</style>
