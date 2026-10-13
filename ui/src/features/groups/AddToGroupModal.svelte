<script lang="ts">
  // Phase 41.1 Plan 14 (GRD-01, D-27/D-28, 41.1-UI-SPEC.md §11): попап быстрого
  // действия «Добавить в группу» — выбор СУЩЕСТВУЮЩЕЙ группы для устройства.
  //
  // Размещён в features/groups/ намеренно — отклонение от UI-SPEC §6.2 ради
  // allowlist check-group-vocabulary.mjs (решение планирования фазы 41.1: гейт
  // пропускает каталог целиком); вызывается со страницы «Устройства».
  //
  // Попап только ВЫБИРАЕТ: создания группы здесь нет и модалки над модалкой нет
  // (D-28). Пустое состояние «Групп пока нет» отсылает к быстрому действию меню.
  // Поиск по названию делает сам Dropdown (flat + select + searchable фильтрует
  // по getGroupName), поэтому состояния пустого поиска («Ничего не найдено») —
  // встроенные в Dropdown; локально список только сортируется (тип, имя), чтобы
  // секции getGroupSection были непрерывными.
  //
  // Серверные отказы (устройство уже в группе, «Списано», «На списание», сетевой
  // принтер — D-32) показываются инлайн текстом сервера; сетевая ошибка — Toast.
  //
  // Монтируется родителем через {#if} (каждый показ — свежий экземпляр).
  import { onMount } from 'svelte';
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';
  import Dropdown from '$lib/components/Dropdown.svelte';
  import { groups } from '$lib/api/groups';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { notifyPlaceContentChanged } from '$lib/stores/placeContentEvents.svelte';
  import type { AppError } from '$lib/api/errors';
  import type { GroupDto } from '../../bindings';

  interface Props {
    device: { id: number; name: string };
    onClose: () => void;
    onAdded: (_group: GroupDto) => void;
  }

  const { device, onClose, onAdded }: Props = $props();

  let allGroups = $state<GroupDto[]>([]);
  let loading = $state(true);
  let loadErr = $state<string | null>(null);
  let selected = $state<GroupDto | null>(null);
  let fieldErr = $state<string | null>(null);
  let saving = $state(false);

  // Секции (тип) идут подряд: сортировка по типу, затем по названию.
  const sortedGroups = $derived(
    [...allGroups].sort(
      (a, b) => a.type_name.localeCompare(b.type_name, 'ru') || a.name.localeCompare(b.name, 'ru'),
    ),
  );

  const placeCallout = $derived(
    selected === null
      ? null
      : selected.place_path
        ? `Устройство переедет в «${selected.place_path}».`
        : 'У группы нет места — место устройства не изменится.',
  );

  function noExpand(): never[] {
    return [];
  }

  onMount(() => {
    groups
      .list()
      .then((list) => {
        allGroups = list;
      })
      .catch(() => {
        loadErr = 'Не удалось загрузить';
      })
      .finally(() => {
        loading = false;
      });
  });

  function pickGroup(g: GroupDto) {
    selected = g;
    fieldErr = null;
  }

  // Одна функция: мутация состава и инвалидация счётчиков «Мест» (INV-7) вместе.
  async function submit() {
    if (selected === null || saving) return;
    const target = selected;
    saving = true;
    fieldErr = null;
    try {
      const result = await groups.addDevices({ group_id: target.id, device_ids: [device.id] });
      notifyPlaceContentChanged(result.changed_place_ids);
      pushToast('success', 'Устройство добавлено в группу «' + target.name + '»');
      onAdded(target);
    } catch (e) {
      const err = e as Partial<AppError> | undefined;
      // Отказ по правилам (уже в группе, «Списано», «На списание», сетевой принтер) —
      // текст сервера в форме; всё остальное — сбой связи/сервера, тостом.
      if (err?.code === 'VALIDATION' || err?.code === 'CONFLICT') {
        fieldErr = err.message ?? 'Устройство нельзя добавить в эту группу.';
      } else {
        pushToast(
          'error',
          'Не удалось добавить устройство в группу. Проверьте подключение и повторите.',
        );
      }
    } finally {
      saving = false;
    }
  }
</script>

<Modal open={true} title="Добавить в группу" size="md" {onClose}>
  <div class="add-to-group">
    <p class="device-line">
      <span class="form-label">Устройство:</span>
      <span class="device-name">{device.name}</span>
    </p>

    {#if !loading && loadErr === null && allGroups.length === 0}
      <div class="empty">
        <p class="empty-title">Групп пока нет</p>
        <p class="empty-body">
          Создайте группу быстрым действием в том же меню — например «Сформировать группу».
        </p>
      </div>
    {:else}
      <div class="form-field" class:has-error={fieldErr !== null}>
        <label class="form-label" for="atg-group">Группа</label>
        <Dropdown
          id="atg-group"
          variant="select"
          flat={true}
          value={selected?.name ?? ''}
          placeholder="Выберите группу"
          searchPlaceholder="Поиск по названию группы"
          invalid={fieldErr !== null}
          disabled={saving || loadErr !== null}
          {loading}
          groups={sortedGroups}
          getGroupId={(g) => g.id}
          getGroupName={(g) => g.name}
          getGroupMeta={(g) => g.place_path ?? 'Без места'}
          getGroupSection={(g) => g.type_name}
          getGroupCount={() => 0}
          isGroupExpandable={() => false}
          isGroupSelected={(g) => g.id === selected?.id}
          onExpandGroup={noExpand}
          getMemberId={(m: GroupDto) => m.id}
          getMemberName={(m: GroupDto) => m.name}
          onSearch={() => {}}
          onPickGroup={pickGroup}
          onPickMember={() => {}}
        />
        {#if loadErr}
          <span class="field-error">{loadErr}</span>
        {/if}
        {#if fieldErr}
          <span class="field-error">{fieldErr}</span>
        {/if}
      </div>

      {#if placeCallout}
        <div class="callout" role="note">{placeCallout}</div>
      {/if}
    {/if}
  </div>

  {#snippet footer()}
    <Button variant="secondary" onclick={onClose} disabled={saving}>Отмена</Button>
    <Button variant="primary" loading={saving} disabled={selected === null} onclick={submit}
      >Добавить</Button
    >
  {/snippet}
</Modal>

<style lang="scss">
  .add-to-group {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-md);
    padding: var(--tr-space-md) 0;
  }

  .device-line {
    display: flex;
    gap: var(--tr-space-xs);
    margin: 0;
  }

  .device-name {
    font: var(--tr-text-body);
    color: var(--tr-text-primary);
  }

  .form-field {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-2xs);
  }

  .form-label {
    font-size: var(--tr-font-size-label);
    font-weight: var(--tr-font-weight-medium);
    color: var(--tr-text-secondary);
  }

  .field-error {
    font-size: var(--tr-font-size-label);
    color: var(--tr-danger-text);
  }

  .callout {
    padding: var(--tr-space-xs) var(--tr-space-md);
    background: var(--tr-warning-soft);
    border-radius: var(--tr-radius-xs);
    font-size: 13px;
    font-weight: 500;
    color: var(--tr-warning-text);
  }

  .empty {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-2xs);
    padding: var(--tr-space-md) 0;
  }

  .empty-title {
    margin: 0;
    font-weight: var(--tr-font-weight-medium);
    color: var(--tr-text-primary);
  }

  .empty-body {
    margin: 0;
    color: var(--tr-text-secondary);
  }
</style>
