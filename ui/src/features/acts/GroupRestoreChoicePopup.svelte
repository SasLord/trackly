<script lang="ts">
  // Фаза 41.2, план 22 (UD-1, D-42): выбор при удалении акта, когда группа, из
  // которой акт выводил устройства, перенесена после акта. Презентационный
  // компонент: ничего не грузит и не удаляет, группы приходят пропсом, результат
  // уходит в onConfirm. Отмена ничего не меняет — на сервере к этому моменту
  // ничего не удалено.
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';
  import Radio from '$lib/components/Radio.svelte';
  import type { GroupRestoreChoiceDto, GroupRestoreConflictDto } from '../../bindings';

  interface Props {
    groups: GroupRestoreConflictDto[];
    loading?: boolean;
    onConfirm: (choices: GroupRestoreChoiceDto[]) => void;
    onClose: () => void;
  }

  const { groups, loading = false, onConfirm, onClose }: Props = $props();

  // По умолчанию у каждой группы выбрано «Вернуть в группу».
  // svelte-ignore state_referenced_locally
  let modes = $state<Record<number, string>>(
    Object.fromEntries(groups.map((g) => [g.group_id, 'restore'])),
  );

  function submit() {
    onConfirm(groups.map((g) => ({ group_id: g.group_id, restore: modes[g.group_id] !== 'keep' })));
  }
</script>

<!-- Начальный фокус на «Отмена»: случайный Enter не должен удалять акт. -->
<Modal
  open={true}
  size="md"
  title="Вернуть устройства в группы?"
  initialFocus=".modal-footer .btn-secondary"
  {onClose}
>
  <div class="choice-body">
    {#each groups as g (g.group_id)}
      <section class="group">
        <p class="group-title">Группа «{g.group_name}» теперь в «{g.group_place_name}».</p>
        <ul class="devices">
          {#each g.devices as d, i (i)}
            <li class="device">
              {d.name} — сейчас на «{d.previous_place_name ?? 'без места'}»
            </li>
          {/each}
        </ul>
        <div class="options">
          <Radio bind:group={modes[g.group_id]} value="restore"
            >Вернуть в группу (на её новое место)</Radio
          >
          <Radio bind:group={modes[g.group_id]} value="keep"
            >Оставить вне группы на прежнем месте</Radio
          >
        </div>
      </section>
    {/each}
  </div>

  {#snippet footer()}
    <Button variant="secondary" onclick={onClose}>Отмена</Button>
    <Button variant="primary" {loading} onclick={submit}>Удалить акт</Button>
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

  .options {
    display: flex;
    flex-direction: column;
    row-gap: var(--tr-space-xs);
  }
</style>
