<script lang="ts">
  // Фаза 41.10, UAT-3: выбор при удалении акта, когда устройства акта перемещали после
  // акта. Презентационный компонент: ничего не грузит и не удаляет, устройства приходят
  // пропсом, выбор (один на все устройства) уходит в onConfirm. Отмена ничего не меняет —
  // на сервере к этому моменту ничего не удалено.
  import Modal from '@trackly/kit/components/Modal.svelte';
  import Button from '@trackly/kit/components/Button.svelte';
  import Radio from '@trackly/kit/components/Radio.svelte';
  import Checkbox from '@trackly/kit/components/Checkbox.svelte';
  import type { MovedAfterActChoiceDto, MovedAfterActDeviceDto } from '../../bindings';

  interface Props {
    actNumber: string;
    devices: MovedAfterActDeviceDto[];
    loading?: boolean;
    onConfirm: (choice: MovedAfterActChoiceDto) => void;
    onClose: () => void;
  }

  const { actNumber, devices, loading = false, onConfirm, onClose }: Props = $props();

  // По умолчанию — «Вернуть на место до акта» со стиранием перемещений после акта.
  let mode = $state<string>('return');
  let erase = $state(true);

  function placeName(path: string | null): string {
    const s = path?.trim() ?? '';
    return s.length > 0 ? s : 'без места';
  }

  function submit() {
    if (mode === 'keep') onConfirm('keep');
    else onConfirm(erase ? 'return_erase' : 'return_mark');
  }
</script>

<!-- Начальный фокус на «Отмена»: случайный Enter не должен удалять акт. -->
<Modal
  open={true}
  size="md"
  title="Устройства перемещали после акта"
  initialFocus=".modal-footer .btn-secondary"
  {onClose}
>
  <div class="choice-body">
    <p class="lead">После акта №{actNumber} эти устройства ещё раз перемещали:</p>
    <ul class="devices">
      {#each devices as d (d.device_id)}
        <li class="device">
          {d.name} — сейчас «{placeName(d.current_place_path)}», до акта «{placeName(
            d.before_place_path,
          )}»
        </li>
      {/each}
    </ul>
    <p class="prompt">Где оставить устройства?</p>
    <div class="options">
      <div class="option">
        <Radio bind:group={mode} value="return">Вернуть на место до акта</Radio>
        <div class="nested">
          <Checkbox bind:checked={erase} disabled={mode !== 'return'}
            >Стереть из истории перемещения после акта</Checkbox
          >
          <p class="hint">Без отметки в истории появится запись о возврате на место до акта.</p>
        </div>
      </div>
      <div class="option">
        <Radio bind:group={mode} value="keep">Оставить на текущем месте</Radio>
        <p class="hint nested">
          В истории перемещение будет показано с места до акта сразу на текущее.
        </p>
      </div>
    </div>
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
    row-gap: var(--tr-space-sm);
    max-height: 60vh;
    overflow-y: auto;
  }

  .lead,
  .prompt {
    margin: 0;
    font: var(--tr-text-body-strong);
    color: var(--tr-text-primary);
    overflow-wrap: anywhere;
  }

  .devices {
    list-style: none;
    margin: 0;
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
    row-gap: var(--tr-space-sm);
  }

  .option {
    display: flex;
    flex-direction: column;
    row-gap: var(--tr-space-2xs);
  }

  .nested {
    margin-left: var(--tr-space-lg);
    display: flex;
    flex-direction: column;
    row-gap: var(--tr-space-2xs);
  }

  .hint {
    margin: 0;
    font: var(--tr-text-label);
    color: var(--tr-text-secondary);
    overflow-wrap: anywhere;
  }
</style>
