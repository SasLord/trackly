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
  // Серверные отказы (устройство уже в группе, «Списано», «На списание») показываются
  // инлайн текстом сервера; сетевая ошибка — Toast.
  //
  // Монтируется родителем через {#if} (каждый показ — свежий экземпляр).
  import { onMount } from 'svelte';
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';
  import Dropdown from '$lib/components/Dropdown.svelte';
  import { groups } from '$lib/api/groups';
  import AlreadyInGroupPopup from './AlreadyInGroupPopup.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { notifyPlaceContentChanged } from '$lib/stores/placeContentEvents.svelte';
  import { isAlreadyInGroup, type AppError } from '$lib/api/errors';
  import type { GroupDto } from '../../bindings';

  interface Props {
    /** Устройство-объект; для переноса группы не задаётся (см. subjectGroup). */
    device?: { id: number; name: string };
    /** 'move' — перенос из текущей группы (D-04); по умолчанию 'add'. */
    mode?: 'add' | 'move';
    currentGroup?: { id: number; name: string } | null;
    /** Переносимая группа вместо устройства (setParent с replace_parent). */
    subjectGroup?: GroupDto;
    onClose: () => void;
    onAdded: (_group: GroupDto) => void;
  }

  const {
    device,
    mode = 'add',
    currentGroup = null,
    subjectGroup,
    onClose,
    onAdded,
  }: Props = $props();

  const isMove = $derived(mode === 'move');
  const subjectName = $derived(subjectGroup?.name ?? device?.name ?? '');
  // Шаг «уже в группе» внутри той же Modal (D-16): модалки над модалкой нет.
  let occupied = $state<{ kind: 'device' | 'group'; currentGroupName: string } | null>(null);

  let allGroups = $state<GroupDto[]>([]);
  let loading = $state(true);
  let loadErr = $state<string | null>(null);
  let selected = $state<GroupDto | null>(null);
  let fieldErr = $state<string | null>(null);
  let saving = $state(false);

  // Секции (тип) идут подряд: сортировка по типу, затем по названию.
  // D-17: цели без «Разбор» (teardown), без текущей группы и, для переносимой
  // группы, без неё самой и всех её потомков (цикл).
  const excludedIds = $derived.by(() => {
    const ex = new Set<number>();
    if (currentGroup) ex.add(currentGroup.id);
    if (subjectGroup) {
      ex.add(subjectGroup.id);
      let grew = true;
      while (grew) {
        grew = false;
        for (const g of allGroups) {
          if (g.parent_group_id !== null && ex.has(g.parent_group_id) && !ex.has(g.id)) {
            ex.add(g.id);
            grew = true;
          }
        }
      }
    }
    return ex;
  });

  const targets = $derived(
    allGroups.filter((g) => g.type_behavior !== 'teardown' && !excludedIds.has(g.id)),
  );

  const sortedGroups = $derived(
    [...targets].sort(
      (a, b) => a.type_name.localeCompare(b.type_name, 'ru') || a.name.localeCompare(b.name, 'ru'),
    ),
  );

  const placeCallout = $derived(
    selected === null
      ? null
      : selected.place_path
        ? subjectGroup
          ? `Группа и её состав переедут в «${selected.place_path}».`
          : `Устройство переедет в «${selected.place_path}».`
        : subjectGroup
          ? 'У группы нет места — место состава не изменится.'
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

  // Перенос из другой группы подтверждён (или явный перенос режима move):
  // одна операция и инвалидация счётчиков «Мест» (INV-7) в одной функции.
  async function doMove(target: GroupDto) {
    if (subjectGroup) {
      const result = await groups.setParent({
        id: subjectGroup.id,
        version: subjectGroup.version,
        parent_group_id: target.id,
        replace_parent: true,
      });
      notifyPlaceContentChanged(result.changed_place_ids);
      pushToast('success', 'Группа «' + subjectGroup.name + '» перемещена в «' + target.name + '»');
    } else if (device) {
      const result = await groups.moveDevices({ group_id: target.id, device_ids: [device.id] });
      notifyPlaceContentChanged(result.changed_place_ids);
      pushToast('success', 'Устройство перемещено в группу «' + target.name + '»');
    }
    onAdded(target);
  }

  // Одна функция: мутация состава и инвалидация счётчиков «Мест» (INV-7) вместе.
  async function submit() {
    if (selected === null || saving) return;
    const target = selected;
    saving = true;
    fieldErr = null;
    try {
      if (isMove) {
        await doMove(target);
      } else if (device) {
        const result = await groups.addDevices({ group_id: target.id, device_ids: [device.id] });
        notifyPlaceContentChanged(result.changed_place_ids);
        pushToast('success', 'Устройство добавлено в группу «' + target.name + '»');
        onAdded(target);
      }
    } catch (e) {
      if (isAlreadyInGroup(e)) {
        // Отказ распознаётся по code, не по тексту (D-16).
        const item = e.details.items[0];
        occupied = {
          kind: subjectGroup ? 'group' : 'device',
          currentGroupName: item?.group_name ?? currentGroup?.name ?? '',
        };
        return;
      }
      const err = e as Partial<AppError> | undefined;
      // Отказ по правилам — текст сервера в форме; остальное — сбой связи, тостом.
      if (err?.code === 'VALIDATION' || err?.code === 'CONFLICT') {
        fieldErr = err.message ?? 'Устройство нельзя добавить в эту группу.';
      } else {
        pushToast(
          'error',
          isMove
            ? 'Не удалось переместить. Проверьте подключение и повторите.'
            : 'Не удалось добавить устройство в группу. Проверьте подключение и повторите.',
        );
      }
    } finally {
      saving = false;
    }
  }

  // «Переместить» в шаге «уже в группе»: ошибка уходит в попап (он покажет её инлайн).
  async function confirmMove() {
    if (selected === null) return;
    await doMove(selected);
  }
</script>

<Modal
  open={true}
  title={occupied !== null
    ? 'Подтверждение'
    : isMove
      ? 'Переместить в группу'
      : 'Добавить в группу'}
  size="md"
  {onClose}
>
  {#if occupied !== null && selected !== null}
    <AlreadyInGroupPopup
      embedded={true}
      kind={occupied.kind}
      {subjectName}
      currentGroupName={occupied.currentGroupName}
      targetName={selected.name}
      targetPlaceLabel={selected.place_path}
      onConfirm={confirmMove}
      onCancel={() => (occupied = null)}
    />
  {:else}
    <div class="add-to-group">
      <p class="device-line">
        <span class="form-label">{subjectGroup ? 'Группа:' : 'Устройство:'}</span>
        <span class="device-name">{subjectName}</span>
      </p>
      {#if isMove && currentGroup}
        <p class="device-line">
          <span class="form-label">Сейчас в группе:</span>
          <span class="current-name">{currentGroup.name}</span>
        </p>
      {/if}

      {#if !loading && loadErr === null && allGroups.length === 0 && !isMove}
        <div class="empty">
          <p class="empty-title">Групп пока нет</p>
          <p class="empty-body">Создайте группу быстрым действием в том же меню.</p>
        </div>
      {:else if !loading && loadErr === null && targets.length === 0}
        <div class="empty">
          <p class="empty-title">Подходящих групп нет</p>
          <p class="empty-body">
            Текущая группа, её вложенные группы и группы «Разбор» не подходят для переноса.
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
  {/if}

  {#snippet footer()}
    {#if occupied === null}
      <Button variant="secondary" onclick={onClose} disabled={saving}>Отмена</Button>
      <Button variant="primary" loading={saving} disabled={selected === null} onclick={submit}
        >{isMove ? 'Переместить в группу' : 'Добавить'}</Button
      >
    {/if}
  {/snippet}
</Modal>

<style lang="scss">
  .add-to-group {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-md);
    padding: var(--tr-space-md) 0;
    white-space: normal;
    text-align: left;
  }

  .device-line {
    display: flex;
    gap: var(--tr-space-xs);
    margin: 0;
  }

  .current-name,
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
