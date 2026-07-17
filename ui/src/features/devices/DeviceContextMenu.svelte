<script lang="ts">
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

  // Ссылка на кнопку-триггер (⋮).
  let triggerEl = $state<HTMLButtonElement | null>(null);

  function toggleMenu() {
    if (menuOpen) {
      menuOpen = false;
      return;
    }
    if (triggerEl) {
      const rect = triggerEl.getBoundingClientRect();
      // Позиционируем меню так, чтобы оно открывалось вниз и вправо от кнопки,
      // выровнено по правому краю кнопки.
      menuX = rect.right - 160; // 160px — min-width меню
      menuY = rect.bottom + 4;
    }
    menuOpen = true;
  }

  function handleEdit() {
    menuOpen = false;
    onEdit(device);
  }

  function handlePrintAcceptance() {
    menuOpen = false;
    onPrintAcceptance?.(device);
  }

  function openConfirm() {
    menuOpen = false;
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
    menuOpen = false;
  }

  // Закрыть меню при прокрутке или ресайзе — простейший способ избежать
  // «висящего» меню с устаревшими координатами.
  function handleScrollOrResize() {
    if (menuOpen) menuOpen = false;
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
    use:portal
    class="ctx-menu-portal"
    role="menu"
    tabindex="-1"
    style="left:{menuX}px; top:{menuY}px;"
    onkeydown={(e) => {
      if (e.key === 'Escape') menuOpen = false;
    }}
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

  .confirm-body {
    margin: 0;
    color: var(--tr-text-secondary);
    line-height: var(--line-height-body);
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
