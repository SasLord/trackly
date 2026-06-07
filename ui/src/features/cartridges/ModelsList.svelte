<script lang="ts">
  // Plan 04-06: полноширинный CRUD-список моделей картриджей.
  // По образцу ActsList.svelte + CartridgesList.svelte.
  import Button from '$lib/components/Button.svelte';
  import Spinner from '$lib/components/Spinner.svelte';
  import Modal from '$lib/components/Modal.svelte';
  import ModelListRow from './ModelListRow.svelte';
  import ModelFormModal from './ModelFormModal.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { cartridges } from './api';
  import type { CartridgeModelDto } from '../../bindings';

  interface Props {
    models: CartridgeModelDto[];
    loading: boolean;
    onRefresh: () => void;
  }

  const { models, loading, onRefresh }: Props = $props();

  // ModelFormModal state
  let formOpen = $state(false);
  let formTarget = $state<CartridgeModelDto | null>(null);

  function openCreate() {
    formTarget = null;
    formOpen = true;
  }

  function openEdit(model: CartridgeModelDto) {
    formTarget = model;
    formOpen = true;
  }

  function handleFormSuccess() {
    formOpen = false;
    onRefresh();
  }

  // Confirm-delete state
  let confirmDeleteOpen = $state(false);
  let confirmDeleteModel = $state<CartridgeModelDto | null>(null);
  let deleting = $state(false);

  function openDelete(model: CartridgeModelDto) {
    confirmDeleteModel = model;
    confirmDeleteOpen = true;
  }

  async function handleConfirmDelete() {
    if (!confirmDeleteModel) return;
    deleting = true;
    try {
      await cartridges.modelsDelete(confirmDeleteModel.id, confirmDeleteModel.version);
      pushToast('success', `Модель «${confirmDeleteModel.brand} ${confirmDeleteModel.model}» удалена.`);
      confirmDeleteOpen = false;
      confirmDeleteModel = null;
      onRefresh();
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось удалить модель';
      // UI-SPEC §ModelListRow: если модель используется — показываем Toast error (без confirm-модала).
      if (msg.toLowerCase().includes('используется') || msg.toLowerCase().includes('conflict')) {
        confirmDeleteOpen = false;
        pushToast('error', msg);
      } else {
        pushToast('error', msg);
      }
    } finally {
      deleting = false;
    }
  }
</script>

<div class="models-list">
  <header class="models-toolbar">
    <h2 class="models-heading">Модели картриджей</h2>
    <Button variant="primary" onclick={openCreate}>+ Добавить модель</Button>
  </header>

  {#if loading && models.length === 0}
    <div class="loading">
      <Spinner size="md" />
    </div>
  {:else if models.length === 0}
    <div class="empty">
      <h3 class="empty-heading">Моделей пока нет</h3>
      <p class="empty-body">Добавьте модель картриджа — укажите бренд, тип и совместимые принтеры.</p>
      <Button variant="primary" onclick={openCreate}>+ Добавить модель</Button>
    </div>
  {:else}
    <div class="rows">
      {#each models as m (m.id)}
        <ModelListRow
          model={m}
          instanceCount={0}
          onEdit={() => openEdit(m)}
          onDelete={() => openDelete(m)}
        />
      {/each}
    </div>
  {/if}
</div>

<!-- ModelFormModal -->
<ModelFormModal
  open={formOpen}
  target={formTarget}
  onClose={() => (formOpen = false)}
  onSuccess={handleFormSuccess}
/>

<!-- Confirm-delete Modal -->
<Modal
  open={confirmDeleteOpen}
  title="Удалить модель?"
  onClose={() => (confirmDeleteOpen = false)}
>
  <p class="confirm-body">
    Модель «{confirmDeleteModel?.brand ?? ''} {confirmDeleteModel?.model ?? ''}» будет помечена как
    удалённая.
  </p>
  {#snippet footer()}
    <Button variant="secondary" onclick={() => (confirmDeleteOpen = false)}>Отмена</Button>
    <Button variant="destructive" loading={deleting} onclick={handleConfirmDelete}>Удалить</Button>
  {/snippet}
</Modal>

<style lang="scss">
  .models-list {
    display: flex;
    flex-direction: column;
    min-height: 240px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
    overflow: hidden;
  }

  .models-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-md) var(--space-lg);
    border-bottom: 1px solid var(--color-border);
    gap: var(--space-md);
    flex-shrink: 0;
  }

  .models-heading {
    margin: 0;
    font-size: var(--font-size-heading);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
    line-height: var(--line-height-heading);
  }

  .loading,
  .empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-sm);
    padding: var(--space-2xl);
    text-align: center;
  }

  .empty-heading {
    margin: 0 0 var(--space-xs);
    font-size: var(--font-size-heading);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
  }

  .empty-body {
    margin: 0 0 var(--space-md);
    color: var(--color-text-secondary);
    font-size: var(--font-size-body);
    max-width: 400px;
  }

  .rows {
    flex: 1;
    overflow: auto;
  }

  .confirm-body {
    margin: 0;
    color: var(--color-text-secondary);
    line-height: var(--line-height-body);
    text-align: center;
    overflow-wrap: anywhere;
    word-break: break-word;
    white-space: normal;
    max-width: 100%;
  }
</style>
