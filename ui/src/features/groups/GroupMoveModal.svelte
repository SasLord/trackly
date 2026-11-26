<script lang="ts">
  // Phase 41 Plan 23 (D-17, D-18, D-24, UI-SPEC 12.1): подтверждение переноса
  // группы на другое место. Перенос — ОСОЗНАННОЕ отдельное действие, а не
  // следствие сохранения формы свойств (в форме группы поля места нет вовсе).
  // Модалку открывает ОДИН обработчик страницы (`openMove`) из двух точек входа:
  // пункт меню узла дерева и кнопка «Перенести…» в панели группы.
  //
  // Числа в callout — серверные поля: `group.device_count` (включая вложенные
  // группы) и `device_count` вложенных групп из `groups.composition`; клиент
  // только склоняет существительное и суммирует то, что показывает. Текст тоста
  // успеха — `summary` из ответа сервера (склонение считает сервер).
  //
  // Инвалидация (D-08, INV-7): `notifyPlaceContentChanged` вызывается в ТОЙ ЖЕ
  // функции, что и `groups.move(`, — это требование реестрового гейта.
  import { untrack } from 'svelte';
  import Modal from '@trackly/kit/components/Modal.svelte';
  import Button from '@trackly/kit/components/Button.svelte';
  import FormField from '@trackly/kit/components/FormField.svelte';
  import PlacePicker from '$lib/components/PlacePicker.svelte';
  import Checkbox from '@trackly/kit/components/Checkbox.svelte';
  import Spinner from '@trackly/kit/components/Spinner.svelte';
  import { groups } from '$lib/api/groups';
  import { pluralizeRu } from '@trackly/kit/utils/pluralize';
  import { pushToast } from '@trackly/kit/stores/toast.svelte';
  import { notifyPlaceContentChanged } from '$lib/stores/placeContentEvents.svelte';
  import type { AppError } from '$lib/api/errors';
  import type { GroupDto, GroupMoveResultDto, GroupMoveStatusPreviewDto } from '../../bindings';

  interface Props {
    group: GroupDto;
    onClose: () => void;
    onMoved: (_result: GroupMoveResultDto) => void;
  }

  const { group, onClose, onMoved }: Props = $props();

  let targetPlaceId = $state<number | null>(null);
  let placeError = $state<string | null>(null);
  let moving = $state(false);

  // D-34: серверный предпросмотр смены статусов. Правило перехода «склад <-> не
  // склад», число устройств и подпись целевого статуса приходят с сервера; клиент
  // ничего не зеркалит и литералов статусов не хранит.
  let preview = $state<GroupMoveStatusPreviewDto | null>(null);
  let changeStatus = $state(true);
  let previewSeq = $state(0);

  // Вложенные группы (прямые дети): состав запрашивается один раз при открытии.
  let nested = $state<GroupDto[]>([]);
  let compositionLoading = $state(true);

  $effect(() => {
    const id = group.id;
    let cancelled = false;
    untrack(() => {
      compositionLoading = true;
      nested = [];
    });
    groups
      .composition(id)
      .then((c) => {
        if (!cancelled) nested = c.child_groups;
      })
      .catch(() => {
        // Нет данных о вложенных — callout покажет только общее число устройств.
        if (!cancelled) nested = [];
      })
      .finally(() => {
        if (!cancelled) compositionLoading = false;
      });
    return () => {
      cancelled = true;
    };
  });

  $effect(() => {
    const id = group.id;
    const placeId = targetPlaceId;
    let stale = false;
    untrack(() => {
      changeStatus = true;
      if (placeId === null) {
        preview = null;
        return;
      }
      previewSeq += 1;
      const mySeq = previewSeq;
      groups
        .moveStatusPreview({ id, target_place_id: placeId })
        .then((p) => {
          if (!stale && mySeq === previewSeq) preview = p;
        })
        .catch((e) => {
          if (stale || mySeq !== previewSeq) return;
          preview = null;
          const err = e as Partial<AppError> | undefined;
          if (err?.code === 'VALIDATION') placeError = err.message ?? null;
        });
    });
    return () => {
      stale = true;
    };
  });

  const showStatusBox = $derived(
    preview !== null && preview.transition !== null && !!preview.target_status_name,
  );
  const affected = $derived(preview?.devices_affected ?? 0);
  const affectedNoun = $derived(pluralizeRu(affected, ['устройства', 'устройств', 'устройств']));

  const total = $derived(group.device_count);
  const nestedDevices = $derived(nested.reduce((sum, g) => sum + g.device_count, 0));

  const verb = $derived(pluralizeRu(total, ['переедет', 'переедут', 'переедут']));
  const noun = $derived(pluralizeRu(total, ['устройство', 'устройства', 'устройств']));

  // Весь текст callout собирается одной строкой: «…переедут 6 устройств, включая
  // 2 из вложенной группы «…».»
  const calloutText = $derived.by(() => {
    const base = `Вместе с группой ${verb} ${total} ${noun}`;
    if (nested.length === 0) return `${base}.`;
    if (nestedDevices === 0) {
      const rest = nested.length === 1 ? 'Вложенная группа переедет' : 'Вложенные группы переедут';
      return `${base}. ${rest} вместе с ней.`;
    }
    if (nested.length === 1) {
      return `${base}, включая ${nestedDevices} из вложенной группы «${nested[0].name}».`;
    }
    return `${base}, включая ${nestedDevices} из вложенных групп.`;
  });

  async function confirmMove(): Promise<void> {
    if (targetPlaceId === null || moving) return;
    moving = true;
    placeError = null;
    try {
      const result = await groups.move({
        id: group.id,
        version: group.version,
        target_place_id: targetPlaceId,
        change_status: preview?.transition != null && preview.devices_affected > 0 && changeStatus,
      });
      notifyPlaceContentChanged(result.changed_place_ids);
      pushToast('success', `Перенесено: ${result.summary}`);
      onMoved(result);
    } catch (e) {
      const err = e as Partial<AppError> | undefined;
      const message = err?.message ?? 'Не удалось перенести группу.';
      if (err?.code === 'VALIDATION') {
        placeError = message;
      } else {
        pushToast('error', message);
      }
    } finally {
      moving = false;
    }
  }

  function handleClose(): void {
    if (moving) return;
    onClose();
  }
</script>

<Modal open={true} title={`Перенести группу «${group.name}»?`} size="md" onClose={handleClose}>
  <div class="move-form">
    <FormField label="Новое место" id="group-move-target" error={placeError}>
      {#snippet children({ invalid })}
        <PlacePicker
          id="group-move-target"
          value={targetPlaceId}
          disabled={moving}
          {invalid}
          onChange={(id) => {
            targetPlaceId = id;
            placeError = null;
          }}
        />
      {/snippet}
    </FormField>

    {#if compositionLoading}
      <div class="stats-loading">
        <Spinner size="sm" />
        <span>Загрузка…</span>
      </div>
    {:else}
      <div class="warning-callout" role="note">{calloutText}</div>
    {/if}

    {#if showStatusBox && preview}
      <div class="status-box">
        <Checkbox
          checked={changeStatus && affected > 0}
          onchange={(v) => (changeStatus = v)}
          disabled={moving || affected === 0}
          id="group-move-change-status"
        >
          Поменять статус на «{preview.target_status_name}»
        </Checkbox>
        <div class="status-hint">
          {#if affected > 0}
            Статус сменится у {affected} {affectedNoun}
          {:else}
            Подходящих устройств нет: статусы не изменятся
          {/if}
        </div>
      </div>
    {/if}
  </div>

  {#snippet footer()}
    <Button variant="secondary" onclick={handleClose} disabled={moving}>Отмена</Button>
    <Button
      variant="primary"
      loading={moving}
      disabled={targetPlaceId === null || moving}
      onclick={confirmMove}
    >
      Перенести
    </Button>
  {/snippet}
</Modal>

<style lang="scss">
  .move-form {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-md);
    padding: var(--tr-space-md) 0;
  }

  .stats-loading {
    display: flex;
    align-items: center;
    gap: var(--tr-space-xs);
    color: var(--tr-text-secondary);
    font-size: var(--tr-font-size-body);
  }

  .status-box {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-xs);
    white-space: normal;
    text-align: left;
  }

  .status-hint {
    color: var(--tr-text-secondary);
    font-size: var(--tr-font-size-body);
    line-height: var(--tr-line-height-body);
  }

  .warning-callout {
    padding: var(--tr-space-sm);
    border-radius: var(--tr-radius-sm);
    background: var(--tr-warning-soft);
    color: var(--tr-warning-text);
    font-size: var(--tr-font-size-body);
    line-height: var(--tr-line-height-body);
  }
</style>
