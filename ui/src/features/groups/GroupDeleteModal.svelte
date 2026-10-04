<script lang="ts">
  // Phase 41 Plan 19 (GRP-10, 41-UI-SPEC.md §17.5): подтверждение удаления
  // группы или типа группы. Монтируется родителем через {#if}.
  //
  // Группа: «N устройств освободятся из состава, их место не изменится» — число
  // это direct_device_count (только прямые члены, серверное; клиент ничего не
  // считает). Тип: удаляется только невстроенный тип без групп; всё остальное
  // отклоняет сервер, его сообщение показывается инлайн (§17.3).
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';
  import { groupTypes, groups } from '$lib/api/groups';
  import { pluralizeRu } from '$lib/utils/pluralize';
  import type { AppError } from '$lib/api/errors';

  interface Props {
    kind: 'group' | 'type';
    id: number;
    name: string;
    /** Только для группы: сколько прямых устройств освободится. */
    directDeviceCount?: number;
    onClose: () => void;
    onDeleted: () => void;
  }

  const { kind, id, name, directDeviceCount = 0, onClose, onDeleted }: Props = $props();

  let saving = $state(false);
  let serverErr = $state<string | null>(null);

  const title = $derived(kind === 'group' ? 'Удалить группу?' : 'Удалить тип группы?');
  const devicesWord = $derived(
    pluralizeRu(directDeviceCount, ['устройство', 'устройства', 'устройств']),
  );

  async function confirmDelete() {
    saving = true;
    serverErr = null;
    try {
      if (kind === 'group') await groups.delete(id);
      else await groupTypes.delete(id);
      onDeleted();
    } catch (e) {
      const err = e as Partial<AppError> | undefined;
      serverErr = err?.message ?? 'Не удалось удалить. Повторите попытку.';
    } finally {
      saving = false;
    }
  }
</script>

<Modal open={true} {title} size="md" {onClose}>
  <p class="modal-body-text">
    {#if kind === 'group'}
      Группа «{name}» будет удалена.
      {#if directDeviceCount > 0}
        {directDeviceCount}
        {devicesWord}
        {directDeviceCount % 10 === 1 && directDeviceCount % 100 !== 11
          ? 'освободится из состава'
          : 'освободятся из состава'}, их место не изменится.
      {:else}
        Устройств в составе нет.
      {/if}
    {:else}
      Тип «{name}» и все его свойства будут удалены.
    {/if}
  </p>
  {#if serverErr}
    <div class="server-error">{serverErr}</div>
  {/if}

  {#snippet footer()}
    <Button variant="secondary" onclick={onClose} disabled={saving}>Отмена</Button>
    <Button variant="destructive" loading={saving} onclick={confirmDelete}>Удалить</Button>
  {/snippet}
</Modal>

<style lang="scss">
  .modal-body-text {
    margin: 0 0 var(--tr-space-md) 0;
    color: var(--tr-text-primary);
  }

  .server-error {
    padding: var(--tr-space-xs) var(--tr-space-md);
    background: color-mix(in srgb, var(--tr-danger) 10%, transparent);
    border: 1px solid color-mix(in srgb, var(--tr-danger) 30%, transparent);
    border-radius: var(--tr-radius-xs);
    font-size: var(--tr-font-size-body);
    color: var(--tr-danger);
  }
</style>
