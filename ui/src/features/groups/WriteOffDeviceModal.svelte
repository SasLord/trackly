<script lang="ts">
  // Фаза 41.8, план 09 (D-19, D-20, UI-SPEC §11.1-§11.2): подтверждение списания
  // устройства. Показывается всегда. Для участника группы (не якоря) — выбор:
  // оставить в группе списанным (по умолчанию) или вывести из группы.
  //
  // Modal может стоять внутри <td>: тело явно задаёт white-space: normal и
  // text-align: left. Монтируется родителем через {#if}.
  import Modal from '$lib/components/Modal.svelte';
  import Button from '@trackly/kit/components/Button.svelte';
  import Radio from '$lib/components/Radio.svelte';
  import { groups } from '$lib/api/groups';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { notifyPlaceContentChanged } from '$lib/stores/placeContentEvents.svelte';
  import type { DeviceDto } from '../../bindings';

  interface Props {
    device: DeviceDto;
    /** Деталь разбора (тип группы teardown): другая подсказка у «Вывести». */
    isTeardownMember?: boolean;
    onDone: () => void;
    onClose: () => void;
  }

  const { device, isTeardownMember = false, onDone, onClose }: Props = $props();

  let saving = $state(false);
  let serverErr = $state<string | null>(null);
  let choice = $state<string>('keep');

  const isMember = $derived(device.group_id != null && device.anchor_group_id == null);
  const invText = $derived(device.inventory_no ? ` (инв. № ${device.inventory_no})` : '');

  async function confirm() {
    if (saving) return;
    saving = true;
    serverErr = null;
    const release = isMember && choice === 'release';
    const groupName = device.group_name ?? '';
    try {
      const res = await groups.writeOffDevice({
        device_id: device.id,
        version: device.version,
        release_from_group: release,
      });
      notifyPlaceContentChanged(res.changed_place_ids);
      pushToast(
        'success',
        release ? `Устройство списано и выведено из группы «${groupName}»` : 'Устройство списано',
      );
      onDone();
    } catch (e) {
      const err = e as { message?: string } | undefined;
      serverErr = err?.message ?? 'Не удалось списать. Проверьте подключение и повторите.';
    } finally {
      saving = false;
    }
  }
</script>

<Modal
  open={true}
  title="Списать устройство?"
  size="md"
  {onClose}
  initialFocus=".modal-footer .btn-secondary"
>
  <div class="writeoff">
    <p class="body">
      Устройство «{device.name}»{invText} получит статус «Списано».{isMember
        ? ''
        : ' Вернуть прежний статус можно вручную в форме устройства.'}
    </p>
    {#if isMember}
      <fieldset class="choice" disabled={saving}>
        <legend class="legend">Устройство входит в группу «{device.group_name}»:</legend>
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
    {/if}
    {#if serverErr}
      <div class="server-error">{serverErr}</div>
    {/if}
  </div>

  {#snippet footer()}
    <Button variant="secondary" onclick={onClose} disabled={saving}>Отменить</Button>
    <Button variant="destructive" loading={saving} onclick={confirm}>Списать</Button>
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

  .server-error {
    padding: var(--tr-space-xs) var(--tr-space-md);
    background: color-mix(in srgb, var(--tr-danger) 10%, transparent);
    border: 1px solid color-mix(in srgb, var(--tr-danger) 30%, transparent);
    border-radius: var(--tr-radius-xs);
    color: var(--tr-danger);
  }
</style>
