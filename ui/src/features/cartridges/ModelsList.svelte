<script lang="ts">
  // Plan 04-06: полноширинный CRUD-список моделей картриджей.
  // По образцу ActsList.svelte + CartridgesList.svelte.
  // Plan 27-04 (D-03): rebuilt on shared Table/TableRow primitives per
  // CartridgesList.svelte precedent — bespoke .rows/.loading/.empty removed,
  // Table now owns the frame/skeleton/empty-state. Toolbar (heading + «Добавить
  // модель») stays outside Table — Table has no header-toolbar slot.
  // Callbacks-first: ModelFormModal и confirm-delete управляются из CartridgesPage.
  import Button from '$lib/components/Button.svelte';
  import Table from '$lib/components/Table.svelte';
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

  const skeletonLoading = $derived(loading && models.length === 0);
  const isEmpty = $derived(!loading && models.length === 0);
</script>

{#snippet tableHead()}
  <th>Модель</th>
  <th class="th-count">Экземпляров</th>
  <th>Примечания</th>
  <th class="th-actions">Действия</th>
{/snippet}

<div class="models-list">
  <header class="models-toolbar">
    <h2 class="models-heading">Модели картриджей</h2>
    <Button variant="primary" onclick={onCreateModel}>+ Добавить модель</Button>
  </header>

  <Table
    columns={4}
    loading={skeletonLoading}
    empty={isEmpty}
    emptyTitle="Моделей пока нет"
    emptyBody="Добавьте модель картриджа — укажите бренд, тип и совместимые принтеры."
    head={tableHead}
    framed={false}
    fillHeight
  >
    {#each models as m (m.id)}
      <ModelListRow
        model={m}
        instanceCount={m.instance_count ?? 0}
        onEdit={() => onEditModel(m)}
        onDelete={() => onDeleteModel(m)}
      />
    {/each}
  </Table>
</div>

<style lang="scss">
  .models-list {
    display: flex;
    flex-direction: column;
    min-height: 240px;
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-md);
    background: var(--tr-surface);
    box-shadow: var(--tr-elev-1);
    overflow: hidden;
  }

  // FIX B1/B2: stretch the Table (fillHeight mode) to consume the remaining
  // height of .models-list instead of sizing to content — same flex-fill
  // pattern as *MasterDetail's `.master > :global(*)` rule.
  .models-list :global(.tr-table-framed) {
    flex: 1 1 auto;
    min-height: 0;
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
    font-size: var(--tr-font-size-h3);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-primary);
    line-height: var(--tr-line-height-h3);
  }

  .th-count {
    width: 130px;
    text-align: right;
  }
  .th-actions {
    width: 40px;
  }
</style>
