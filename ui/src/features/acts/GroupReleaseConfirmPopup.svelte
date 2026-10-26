<script lang="ts">
  // Фаза 41.2, план 12 (GAM-02, D-06/D-07, UI-SPEC §9): подтверждение выноса
  // позиций из состава их групп. Презентационный компонент: ничего не грузит и
  // не сохраняет, все данные и обе реакции приходят пропсами от ActFormBody
  // (через ActFormModal, который рисует попап на верхнем уровне).
  //
  // Две кнопки, третьей («добавить всю группу вместо этого») нет: попап не
  // правит состав акта за спиной формы. Primary, а не destructive (R-06):
  // действие ничего не удаляет и откатывается удалением акта.
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';

  export interface GroupReleaseItem {
    name: string;
    groupName: string;
  }

  interface Props {
    items: GroupReleaseItem[];
    loading?: boolean;
    onConfirm: () => void;
    onClose: () => void;
  }

  const { items, loading = false, onConfirm, onClose }: Props = $props();
</script>

<!-- Начальный фокус на «Отмена» (вторичная кнопка подвала): случайный Enter по
     инерции сохранения не должен выводить устройства из состава (T-41.2-50). -->
<Modal
  open={true}
  size="md"
  title="Вывести из состава и передать?"
  initialFocus=".modal-footer .btn-secondary"
  {onClose}
>
  <div class="release-body">
    <p class="intro">
      В акт попала только часть состава. Эти позиции передаются отдельно от своей группы и
      перестанут входить в её состав:
    </p>
    <ul class="items">
      {#each items as item, i (i)}
        <li class="item">
          <span class="item-name">{item.name}</span>
          <span class="item-origin">из состава «{item.groupName}»</span>
        </li>
      {/each}
    </ul>
    <p class="outro">
      Остальной состав группы останется на месте. Удаление акта вернёт эти позиции в группу.
    </p>
  </div>

  {#snippet footer()}
    <Button variant="secondary" onclick={onClose}>Отмена</Button>
    <Button variant="primary" {loading} onclick={onConfirm}>Вывести и передать</Button>
  {/snippet}
</Modal>

<style lang="scss">
  // Попап может оказаться внутри ячейки таблицы: nowrap и центрирование
  // наследуются от <td class="cell">, поэтому заданы явно.
  .release-body {
    white-space: normal;
    text-align: left;
  }

  .intro {
    margin: 0 0 var(--tr-space-md);
    font: var(--tr-text-body);
    color: var(--tr-text-primary);
  }

  .items {
    display: flex;
    flex-direction: column;
    row-gap: var(--tr-space-xs);
    list-style: none;
    margin: 0;
    padding: 0;
    max-height: 40vh;
    overflow-y: auto;
  }

  .item {
    display: flex;
    flex-direction: column;
    row-gap: var(--tr-space-2xs);
  }

  .item-name {
    font: var(--tr-text-body-strong);
    color: var(--tr-text-primary);
    overflow-wrap: anywhere;
  }

  .item-origin {
    font: var(--tr-text-label);
    color: var(--tr-text-secondary);
    overflow-wrap: anywhere;
  }

  .outro {
    margin: var(--tr-space-md) 0 0;
    font: var(--tr-text-body);
    color: var(--tr-text-primary);
  }
</style>
