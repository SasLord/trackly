<script lang="ts">
  // Фаза 41.8, план 22 (D-26, D-19, D-20): подтверждение списания участника группы
  // при сохранении ФОРМЫ устройства. API не вызывает: выбор возвращается родителю
  // (onChoose), тот кладёт его в patch.group_write_off.
  //
  // Modal может стоять поверх формы в ячейке таблицы: тело явно задаёт
  // white-space: normal и text-align: left. Монтируется родителем через {#if}.
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';
  import Radio from '$lib/components/Radio.svelte';

  interface Props {
    deviceName: string;
    groupName: string;
    /** Деталь разбора: другая подсказка у «Вывести из группы». */
    isTeardownMember?: boolean;
    onChoose: (release: boolean) => void;
    onClose: () => void;
  }

  const { deviceName, groupName, isTeardownMember = false, onChoose, onClose }: Props = $props();

  let choice = $state<string>('keep');
</script>

<Modal
  open={true}
  title="Списать устройство?"
  size="md"
  {onClose}
  initialFocus=".modal-footer .btn-secondary"
>
  <div class="writeoff">
    <p class="body">Устройство «{deviceName}» получит статус «Списано».</p>
    <fieldset class="choice">
      <legend class="legend">Устройство входит в группу «{groupName}»:</legend>
      <div class="opt">
        <Radio bind:group={choice} value="keep">
          <span class="opt-text">
            <span class="opt-title">Оставить в группе списанным</span>
            <span class="hint">В составе группы оно будет показано неактивной строкой.</span>
          </span>
        </Radio>
      </div>
      <div class="opt">
        <Radio bind:group={choice} value="release">
          <span class="opt-text">
            <span class="opt-title">Вывести из группы</span>
            <span class="hint"
              >{isTeardownMember
                ? 'Деталь останется в составе разбора выведенной.'
                : 'Место устройства не изменится.'}</span
            >
          </span>
        </Radio>
      </div>
    </fieldset>
  </div>

  {#snippet footer()}
    <Button variant="secondary" onclick={onClose}>Отменить</Button>
    <Button variant="destructive" onclick={() => onChoose(choice === 'release')}>Списать</Button>
  {/snippet}
</Modal>

<style lang="scss">
  .writeoff {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-md);
    white-space: normal;
    text-align: left;
  }

  .body {
    margin: 0;
    color: var(--tr-text-primary);
    overflow-wrap: anywhere;
    white-space: normal;
    text-align: left;
  }

  .choice {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-xs);
    margin: 0;
    padding: 0;
    border: 0;
    min-width: 0;
  }

  .legend {
    padding: 0;
    margin-bottom: var(--tr-space-xs);
    font: var(--tr-text-label);
    color: var(--tr-text-secondary);
  }

  .opt-text {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-2xs);
    white-space: normal;
    text-align: left;
  }

  .opt-title {
    font: var(--tr-text-body);
    color: var(--tr-text-primary);
  }

  .hint {
    font: var(--tr-text-caption);
    color: var(--tr-text-secondary);
  }
</style>
