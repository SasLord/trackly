<script lang="ts">
  // Фаза 41.2, план 27 (UD-2, D-45, D-46): вопрос, когда возврат разбивает
  // контейнерную группу. Презентационный компонент: ничего не грузит и не
  // отправляет, группы приходят пропсом, выбор уходит в onChoose. «Отмена» ничего
  // не меняет — на сервере к этому моменту возврат не оформлен.
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';
  import type { GroupSplitChoiceDto, ReturnGroupSplitItemDto } from '../../bindings';

  interface Props {
    items: ReturnGroupSplitItemDto[];
    loading?: boolean;
    onChoose: (choice: GroupSplitChoiceDto) => void;
    onClose: () => void;
  }

  const { items, loading = false, onChoose, onClose }: Props = $props();

  // D-46 (UD-3): перенос группы возможен, только если у возвращаемых одно место.
  const moveDisabled = $derived(items.some((item) => !item.places_match));
</script>

<!-- Начальный фокус на «Отмена»: случайный Enter не должен разделять группу. -->
<Modal
  open={true}
  size="md"
  title="Группа будет разделена"
  initialFocus=".modal-footer .btn-secondary"
  {onClose}
>
  <div class="choice-body">
    {#each items as item (item.group_id)}
      {@const kept = item.kept_names.length}
      {@const total = item.returned_names.length + kept}
      <section class="group">
        {#if kept > 0}
          <p class="group-title">
            Возвращается {item.returned_names.length} из {total} устройств группы «{item.group_name}».
          </p>
        {:else}
          <p class="group-title">Возвращаются устройства группы «{item.group_name}».</p>
        {/if}
        <p class="explain">
          Вывести из группы — возвращаемые выйдут из группы, группа останется на месте.
        </p>
        <p class="explain">
          Изменить расположение группы — группа с возвращаемыми устройствами переедет в «{item.target_place_name ??
            'без места'}»{#if kept > 0}, остальные устройства выйдут из группы и останутся на
            прежнем месте по старому акту{/if}.
        </p>
        {#if !item.places_match}
          <p class="warn">
            У возвращаемых устройств разные места — укажите одно место, чтобы перенести группу.
          </p>
        {/if}
        <p class="list-title">Возвращаются:</p>
        <ul class="devices">
          {#each item.returned_names as name, i (i)}
            <li class="device">{name}</li>
          {/each}
        </ul>
        {#if kept > 0}
          <p class="list-title">Останутся вне группы:</p>
          <ul class="devices">
            {#each item.kept_names as name, i (i)}
              <li class="device">{name}</li>
            {/each}
          </ul>
        {/if}
      </section>
    {/each}
  </div>

  {#snippet footer()}
    <Button variant="secondary" onclick={onClose}>Отмена</Button>
    <Button variant="secondary" {loading} onclick={() => onChoose('release')}
      >Вывести из группы</Button
    >
    <Button
      variant="primary"
      {loading}
      disabled={moveDisabled}
      onclick={() => onChoose('move_group')}>Изменить расположение группы</Button
    >
  {/snippet}
</Modal>

<style lang="scss">
  // Попап может оказаться внутри ячейки таблицы: nowrap и центрирование
  // наследуются от <td class="cell">, поэтому заданы явно.
  .choice-body {
    white-space: normal;
    text-align: left;
    display: flex;
    flex-direction: column;
    row-gap: var(--tr-space-md);
    max-height: 60vh;
    overflow-y: auto;
  }

  .group-title {
    margin: 0 0 var(--tr-space-xs);
    font: var(--tr-text-body-strong);
    color: var(--tr-text-primary);
    overflow-wrap: anywhere;
  }

  .explain {
    margin: 0 0 var(--tr-space-xs);
    font: var(--tr-text-label);
    color: var(--tr-text-secondary);
    overflow-wrap: anywhere;
  }

  .warn {
    margin: 0 0 var(--tr-space-xs);
    font: var(--tr-text-label);
    color: var(--tr-danger);
    overflow-wrap: anywhere;
  }

  .list-title {
    margin: var(--tr-space-xs) 0 var(--tr-space-2xs);
    font: var(--tr-text-label);
    color: var(--tr-text-primary);
  }

  .devices {
    list-style: none;
    margin: 0 0 var(--tr-space-sm);
    padding: 0;
    display: flex;
    flex-direction: column;
    row-gap: var(--tr-space-2xs);
    max-height: 20vh;
    overflow-y: auto;
  }

  .device {
    font: var(--tr-text-label);
    color: var(--tr-text-secondary);
    overflow-wrap: anywhere;
  }
</style>
