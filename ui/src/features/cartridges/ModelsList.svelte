<script lang="ts">
  // Plan 04-06: полноширинный CRUD-список моделей картриджей.
  // По образцу ActsList.svelte + CartridgesList.svelte.
  // Callbacks-first: ModelFormModal и confirm-delete управляются из CartridgesPage.
  import Button from '$lib/components/Button.svelte';
  import Spinner from '$lib/components/Spinner.svelte';
  import ModelListRow from './ModelListRow.svelte';
  import type { CartridgeModelDto } from '../../bindings';

  interface Props {
    models: CartridgeModelDto[];
    loading: boolean;
    onCreateModel: () => void;
    onEditModel: (_model: CartridgeModelDto) => void;
    onDeleteModel: (_model: CartridgeModelDto) => void;
  }

  const { models, loading, onCreateModel, onEditModel, onDeleteModel }: Props = $props();
</script>

<div class="models-list">
  <header class="models-toolbar">
    <h2 class="models-heading">Модели картриджей</h2>
    <Button variant="primary" onclick={onCreateModel}>+ Добавить модель</Button>
  </header>

  {#if loading && models.length === 0}
    <div class="loading">
      <Spinner size="md" />
    </div>
  {:else if models.length === 0}
    <div class="empty">
      <h3 class="empty-heading">Моделей пока нет</h3>
      <p class="empty-body">
        Добавьте модель картриджа — укажите бренд, тип и совместимые принтеры.
      </p>
      <Button variant="primary" onclick={onCreateModel}>+ Добавить модель</Button>
    </div>
  {:else}
    <div class="rows">
      {#each models as m (m.id)}
        <ModelListRow
          model={m}
          instanceCount={m.instance_count ?? 0}
          onEdit={() => onEditModel(m)}
          onDelete={() => onDeleteModel(m)}
        />
      {/each}
    </div>
  {/if}
</div>

<style lang="scss">
  .models-list {
    display: flex;
    flex-direction: column;
    min-height: 240px;
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-md);
    background: var(--tr-surface);
    overflow: hidden;
  }

  .models-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--tr-space-md) var(--tr-space-xl);
    border-bottom: 1px solid var(--tr-border);
    gap: var(--tr-space-md);
    flex-shrink: 0;
  }

  .models-heading {
    margin: 0;
    font-size: var(--font-size-heading);
    font-weight: var(--font-weight-semibold);
    color: var(--tr-text-primary);
    line-height: var(--line-height-heading);
  }

  .loading,
  .empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--tr-space-xs);
    padding: var(--tr-space-4xl);
    text-align: center;
  }

  .empty-heading {
    margin: 0 0 var(--tr-space-2xs);
    font-size: var(--font-size-heading);
    font-weight: var(--font-weight-semibold);
    color: var(--tr-text-primary);
  }

  .empty-body {
    margin: 0 0 var(--tr-space-md);
    color: var(--tr-text-secondary);
    font-size: var(--font-size-body);
    max-width: 400px;
  }

  .rows {
    flex: 1;
    overflow: auto;
  }
</style>
