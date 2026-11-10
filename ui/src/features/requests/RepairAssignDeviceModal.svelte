<script lang="ts">
  // Фаза 41.4, план 17 (D-24): попап «Указать устройство» для ремонтной заявки «Другое».
  // Конфликт (устройство уже в открытой заявке) показывается внутри попапа, тост не нужен.
  import { untrack } from 'svelte';
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';
  import Dropdown from '$lib/components/Dropdown.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { repairs } from './api';
  import type { RepairDeviceOptionDto, RequestDto } from '../../bindings';

  interface Props {
    open: boolean;
    request: RequestDto;
    onClose: () => void;
    onSuccess: (_request: RequestDto) => void;
    /** Открыть заявку, занявшую устройство. */
    onOpenRequest: (_id: number) => void;
  }

  const { open, request, onClose, onSuccess, onOpenRequest }: Props = $props();

  interface PickOption {
    id: string;
    label: string;
    sub?: string;
    meta?: string;
    section?: string;
  }

  let devices = $state<RepairDeviceOptionDto[]>([]);
  let devicesLoading = $state(false);
  let choice = $state<string | null>(null);
  let query = $state('');
  let submitting = $state(false);
  let conflictRequestId = $state<number | null>(null);
  let loadFailed = $state(false);

  $effect(() => {
    if (!open) return;
    let cancelled = false;
    untrack(() => {
      choice = null;
      query = '';
      submitting = false;
      conflictRequestId = null;
      loadFailed = false;
      devicesLoading = true;
    });
    (async () => {
      try {
        const list = await repairs.deviceOptions();
        if (!cancelled) devices = list;
      } catch {
        if (!cancelled) loadFailed = true;
      } finally {
        if (!cancelled) devicesLoading = false;
      }
    })();
    return () => {
      cancelled = true;
    };
  });

  const options = $derived.by<PickOption[]>(() => {
    const q = query.trim().toLowerCase();
    const matched = devices.filter(
      (d) =>
        !q ||
        d.name.toLowerCase().includes(q) ||
        (d.inventoryNo ?? '').toLowerCase().includes(q) ||
        d.typeName.toLowerCase().includes(q),
    );
    const sorted = [...matched].sort(
      (a, b) => a.typeName.localeCompare(b.typeName, 'ru') || a.name.localeCompare(b.name, 'ru'),
    );
    return sorted.map((d) => ({
      id: String(d.id),
      label: d.name,
      sub: d.inventoryNo ?? undefined,
      meta: d.inRepair ? 'уже в ремонте' : undefined,
      section: d.typeName,
    }));
  });
  const selectedLabel = $derived(
    choice === null ? '' : (devices.find((d) => String(d.id) === choice)?.name ?? ''),
  );

  function noExpand(): PickOption[] {
    return [];
  }

  async function submit() {
    if (submitting || choice === null) return;
    submitting = true;
    conflictRequestId = null;
    try {
      const result = await repairs.assignDevice({
        requestId: request.id,
        version: request.version,
        deviceId: parseInt(choice, 10),
      });
      if (result.conflictRequestId !== null) {
        conflictRequestId = result.conflictRequestId;
        return;
      }
      pushToast('success', 'Устройство указано');
      if (result.request) onSuccess(result.request);
      onClose();
    } catch {
      pushToast('error', 'Не удалось указать устройство. Попробуйте ещё раз.');
    } finally {
      submitting = false;
    }
  }
</script>

<Modal {open} title="Указать устройство" size="md" {onClose} initialFocus="#rep-assign-device">
  <div class="form">
    <div class="field">
      <label class="label" for="rep-assign-device">Устройство</label>
      <Dropdown
        variant="select"
        flat={true}
        id="rep-assign-device"
        value={selectedLabel}
        placeholder="Выберите устройство"
        searchPlaceholder="Название или инв. номер"
        loading={devicesLoading}
        groups={options}
        getGroupId={(o) => o.id}
        getGroupName={(o) => o.label}
        getGroupSub={(o) => o.sub}
        getGroupMeta={(o) => o.meta}
        getGroupSection={(o) => o.section}
        getGroupCount={() => 0}
        isGroupExpandable={() => false}
        isGroupSelected={(o) => o.id === choice}
        onExpandGroup={noExpand}
        getMemberId={(o) => o.id}
        getMemberName={(o) => o.label}
        onSearch={(q) => (query = q)}
        onPickGroup={(o) => {
          choice = o.id;
          conflictRequestId = null;
          query = '';
        }}
        onPickMember={() => {}}
      />
      {#if loadFailed}
        <span class="error" role="alert">Не удалось загрузить список устройств.</span>
      {/if}
    </div>

    {#if conflictRequestId !== null}
      {@const conflictId = conflictRequestId}
      <div class="conflict" role="alert">
        <span>По этому устройству уже есть заявка на ремонт.</span>
        <Button variant="link" size="sm" onclick={() => onOpenRequest(conflictId)}>
          Открыть заявку №{conflictId}
        </Button>
      </div>
    {/if}
  </div>

  {#snippet footer()}
    <Button variant="secondary" onclick={onClose}>Отмена</Button>
    <Button
      variant="primary"
      loading={submitting}
      disabled={submitting || choice === null}
      onclick={submit}
    >
      Указать устройство
    </Button>
  {/snippet}
</Modal>

<style lang="scss">
  .form {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-md);
    white-space: normal;
    text-align: left;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-2xs);
  }
  .label {
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-secondary);
  }
  .error {
    font-size: var(--tr-font-size-label);
    color: var(--tr-danger-text);
  }
  .conflict {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--tr-space-xs);
    font-size: var(--tr-font-size-label);
    color: var(--tr-danger-text);
  }
</style>
