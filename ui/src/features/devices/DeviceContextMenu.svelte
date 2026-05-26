<script lang="ts">
  import Button from '$lib/components/Button.svelte';
  import Modal from '$lib/components/Modal.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { devices } from './api';
  import type { DeviceDto } from '../../bindings';

  interface Props {
    device: DeviceDto;
    onEdit: (_d: DeviceDto) => void;
    onDelete: () => void;
  }

  const { device, onEdit, onDelete }: Props = $props();

  let menuOpen = $state(false);
  let confirmOpen = $state(false);
  let deleting = $state(false);

  function toggleMenu() {
    menuOpen = !menuOpen;
  }

  function handleEdit() {
    menuOpen = false;
    onEdit(device);
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

  function handleOutsideClick(e: MouseEvent) {
    if (menuOpen) {
      const target = e.target as HTMLElement;
      if (!target.closest('.context-menu-wrapper')) {
        menuOpen = false;
      }
    }
  }
</script>

<svelte:window onclick={handleOutsideClick} />

<div class="context-menu-wrapper">
  <button
    class="kebab-btn"
    onclick={toggleMenu}
    aria-label="Действия с устройством"
    aria-expanded={menuOpen}
    aria-haspopup="menu"
  >
    <span class="dots">⋮</span>
  </button>

  {#if menuOpen}
    <div class="dropdown" role="menu">
      <button class="dropdown-item" role="menuitem" onclick={handleEdit}> Редактировать </button>
      <hr class="dropdown-sep" />
      <button
        class="dropdown-item dropdown-item--destructive"
        role="menuitem"
        onclick={openConfirm}
      >
        Удалить
      </button>
    </div>
  {/if}
</div>

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
    position: relative;
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
    color: var(--color-text-secondary);
    font-size: 18px;
    line-height: 1;
    padding: 0;

    &:hover {
      background: var(--color-surface);
      color: var(--color-text-primary);
    }

    &:focus-visible {
      outline: none;
      box-shadow: 0 0 0 3px var(--color-accent-focus);
    }
  }

  .dots {
    user-select: none;
  }

  .dropdown {
    position: absolute;
    right: 0;
    top: calc(100% + 4px);
    background: var(--color-surface-raised);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    box-shadow: var(--shadow-elev-1);
    min-width: 160px;
    z-index: 100;
    padding: var(--space-xs) 0;
  }

  .dropdown-item {
    display: block;
    width: 100%;
    padding: var(--space-xs) var(--space-md);
    background: transparent;
    border: none;
    text-align: left;
    font-size: var(--font-size-body);
    color: var(--color-text-primary);
    cursor: pointer;
    white-space: nowrap;
    font-family: var(--font-family-base);

    &:hover {
      background: var(--color-surface);
    }

    &--destructive {
      color: var(--color-destructive);
    }
  }

  .dropdown-sep {
    border: none;
    border-top: 1px solid var(--color-border);
    margin: var(--space-xs) 0;
  }

  .confirm-body {
    margin: 0;
    color: var(--color-text-secondary);
    line-height: var(--line-height-body);
  }
</style>
