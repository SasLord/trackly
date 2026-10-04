<script lang="ts">
  // Phase 41 Plan 20 (D-09, D-13..D-16, UI-SPEC 9.2): таблица свойств типа
  // группы прямо в панели типа — правка названия/типа/флагов, добавление,
  // скрытие/возврат, перестановка «Выше»/«Ниже» и перетаскивание за ручку «⠿» на pointer-events (D-10).
  //
  // Источник истины — сервер: UI-блокировки (disabled типа данных у заполненного
  // свойства, скрытие вместо удаления) — удобство; отказ показывается его
  // сообщением. Не зеркалим серверные правила (нормализация, 50 свойств).
  import Table from '$lib/components/Table.svelte';
  import TableRow from '$lib/components/TableRow.svelte';
  import Dropdown from '$lib/components/Dropdown.svelte';
  import Checkbox from '$lib/components/Checkbox.svelte';
  import Badge from '$lib/components/Badge.svelte';
  import ActionMenu from '$lib/components/ActionMenu.svelte';
  import Input from '$lib/components/Input.svelte';
  import Button from '$lib/components/Button.svelte';
  import Modal from '$lib/components/Modal.svelte';
  import PropertyRequiredViolatorsPopup from './PropertyRequiredViolatorsPopup.svelte';
  import { groupTypes } from '$lib/api/groups';
  import { insertionIndex, reorder } from '$lib/utils/reorder';
  import { pluralizeRu } from '$lib/utils/pluralize';
  import { pushToast } from '$lib/stores/toast.svelte';
  import type { AppError } from '$lib/api/errors';
  import type { GroupTypePropertyDto, PropertyUpdateDto } from '../../bindings';

  interface Props {
    typeId: number;
    /** Свойства в порядке сервера; скрытые уже отфильтрованы им при showHidden=false. */
    properties: GroupTypePropertyDto[];
    /** admin: правка, добавление, скрытие, перестановка. Иначе — только чтение. */
    canEdit: boolean;
    loading?: boolean;
    /** Перезагрузить данные типа (после успешной мутации). */
    onReload: () => void | Promise<void>;
    /** «Открыть» в попапе нарушителей: страница фокусирует группу в дереве. */
    onOpenGroup: (_groupId: number) => void;
  }

  const { typeId, properties, canEdit, loading = false, onReload, onOpenGroup }: Props = $props();

  interface TypeOption {
    value: string;
    label: string;
  }

  const DATA_TYPES: TypeOption[] = [
    { value: 'text', label: 'Текст' },
    { value: 'number', label: 'Число' },
    { value: 'ip', label: 'IP-адрес' },
    { value: 'mac', label: 'MAC-адрес' },
    { value: 'users', label: 'Пользователи' },
    { value: 'device_refs', label: 'Ссылки на устройства' },
  ];

  function typeLabel(value: string): string {
    return DATA_TYPES.find((o) => o.value === value)?.label ?? value;
  }

  // Плоский список без drill-in: Dropdown всё равно требует типизированный колбэк.
  function noExpand(): TypeOption[] {
    return [];
  }

  const newNameId = $derived(`gtp-new-name-${typeId}`);

  // ---- Очередь мутаций ------------------------------------------------------
  // Каждая правка читает ТЕКУЩУЮ version свойства из props уже после того, как
  // предыдущая правка перезагрузила данные: две быстрые правки подряд не
  // ловят конфликт версий (CAS) сами с собой. Задания не бросают исключений.
  let chain: Promise<void> = Promise.resolve();
  function enqueue(job: () => Promise<void>): Promise<void> {
    const next = chain.then(job);
    chain = next;
    return next;
  }

  function currentProperty(id: number): GroupTypePropertyDto | undefined {
    return properties.find((p) => p.id === id);
  }

  function errMessage(e: unknown, fallback: string): string {
    const err = e as Partial<AppError> | undefined;
    return err?.message ?? fallback;
  }

  // ---- Название (правка по blur) -------------------------------------------
  let nameDraft = $state<Record<number, string>>({});
  let nameErr = $state<Record<number, string>>({});

  function onNameInput(id: number, v: string): void {
    nameDraft[id] = v;
    if (nameErr[id]) delete nameErr[id];
  }

  function saveName(p: GroupTypePropertyDto): void {
    const raw = nameDraft[p.id];
    if (raw === undefined) return;
    const name = raw.trim();
    if (name === p.name) {
      delete nameDraft[p.id];
      return;
    }
    if (name === '') {
      nameErr[p.id] = 'Укажите название.';
      return;
    }
    void enqueue(async () => {
      const cur = currentProperty(p.id);
      if (!cur) return;
      try {
        await groupTypes.updateProperty(cur.id, cur.version, {
          name,
          data_type: null,
          is_required: null,
          show_on_map: null,
        });
        delete nameDraft[p.id];
        delete nameErr[p.id];
        await onReload();
      } catch (e) {
        nameErr[p.id] = errMessage(e, 'Не удалось сохранить название.');
      }
    });
  }

  // ---- Тип данных, «Обязательное», «На карте» --------------------------------
  // Checkbox держит собственное состояние: при отказе сервера пересоздаём его
  // через {#key} (rev), иначе галочка осталась бы стоять при неизменных props.
  let rev = $state<Record<number, number>>({});
  function bumpRev(id: number): void {
    rev[id] = (rev[id] ?? 0) + 1;
  }

  let violators = $state<{ groups: { id: number; name: string }[]; total: number } | null>(null);

  function isRequiredRefusal(e: unknown): boolean {
    const err = e as Partial<AppError> | undefined;
    const details = err?.details;
    return (
      err?.code === 'VALIDATION' &&
      !!details &&
      typeof details === 'object' &&
      !Array.isArray(details) &&
      (details as { field?: unknown }).field === 'is_required'
    );
  }

  function patchProperty(p: GroupTypePropertyDto, patch: Partial<PropertyUpdateDto>): void {
    void enqueue(async () => {
      const cur = currentProperty(p.id);
      if (!cur) return;
      const dto: PropertyUpdateDto = {
        name: null,
        data_type: null,
        is_required: null,
        show_on_map: null,
        ...patch,
      };
      try {
        await groupTypes.updateProperty(cur.id, cur.version, dto);
        await onReload();
      } catch (e) {
        bumpRev(p.id);
        if (isRequiredRefusal(e)) {
          // D-14: список нарушителей даёт сервер; UI его не считает.
          try {
            const refs = await groupTypes.emptyGroups(p.id);
            if (refs.length > 0) {
              violators = { groups: refs.slice(0, 20), total: refs.length };
              return;
            }
          } catch {
            // ниже покажем исходное сообщение отказа
          }
        }
        pushToast('error', errMessage(e, 'Не удалось сохранить свойство.'));
      }
    });
  }

  function pickType(p: GroupTypePropertyDto, o: TypeOption): void {
    if (o.value === p.data_type) return;
    patchProperty(p, { data_type: o.value });
  }

  function openViolator(id: number): void {
    violators = null;
    onOpenGroup(id);
  }

  // ---- Порядок: «Выше»/«Ниже» ------------------------------------------------
  // Порядок отправляется только по ЖИВЫМ свойствам (сервер переставляет живые).
  // Оптимистично: локальный порядок поверх props, при ошибке откат + Toast.
  let optimisticIds = $state<number[] | null>(null);
  let orderSeq = 0;
  let liveMessage = $state('');

  const rows = $derived.by((): GroupTypePropertyDto[] => {
    if (optimisticIds === null) return properties;
    const byId = new Map(properties.filter((p) => !p.archived).map((p) => [p.id, p]));
    const ordered = optimisticIds
      .map((id) => byId.get(id))
      .filter((p): p is GroupTypePropertyDto => p !== undefined);
    let k = 0;
    // Скрытые остаются на своих местах, живые занимают свои слоты в новом порядке.
    return properties.map((p) => (p.archived ? p : (ordered[k++] ?? p)));
  });
  const liveRows = $derived(rows.filter((p) => !p.archived));

  /** `to` — индекс вставки в исходном списке живых свойств (контракт reorder). */
  function moveLive(from: number, to: number): void {
    const live = liveRows;
    const moved = live[from];
    if (!moved) return;
    const next = reorder(live, from, to);
    if (next.every((p, i) => p.id === live[i].id)) return;
    const ids = next.map((p) => p.id);
    const seq = ++orderSeq;
    optimisticIds = ids;
    liveMessage = `Свойство «${moved.name}» перемещено на позицию ${ids.indexOf(moved.id) + 1} из ${ids.length}.`;
    void enqueue(async () => {
      try {
        await groupTypes.reorderProperties(typeId, ids);
        await onReload();
      } catch {
        pushToast(
          'error',
          'Не удалось сохранить порядок свойств. Порядок возвращён, попробуйте снова.',
        );
      } finally {
        // Позднее перемещение уже держит свой оптимистичный порядок.
        if (seq === orderSeq) optimisticIds = null;
      }
    });
  }

  function moveUp(p: GroupTypePropertyDto): void {
    const idx = liveRows.findIndex((r) => r.id === p.id);
    if (idx > 0) moveLive(idx, idx - 1);
  }

  function moveDown(p: GroupTypePropertyDto): void {
    const idx = liveRows.findIndex((r) => r.id === p.id);
    if (idx >= 0 && idx < liveRows.length - 1) moveLive(idx, idx + 2);
  }

  // ---- Перестановка перетаскиванием (D-10) -------------------------------------
  // Структура обработчиков — как в PlaceTree (прошла живой UAT на Tauri и в LAN-
  // браузере): pointerdown/move/up/cancel + setPointerCapture на контейнере. HTML5
  // DnD не используется (в WKWebView drop не срабатывает). Hit-test — только по
  // прямоугольникам строк через insertionIndex, не поиск элемента под курсором. Сохранение —
  // тот же moveLive, что и у пунктов «Выше»/«Ниже» (оптимистично, с откатом).
  const DRAG_START_THRESHOLD_PX = 6;

  let draggingId = $state<number | null>(null);
  let dragGhost = $state<{ label: string; x: number; y: number } | null>(null);
  let indicatorTop = $state<number | null>(null);

  // Служебное состояние жеста: не читается шаблоном, реактивность не нужна.
  let dragOriginId: number | null = null;
  let dragPointerId: number | null = null;
  let dragStartX = 0;
  let dragStartY = 0;
  let dragStarted = false;
  let dragInsertIndex: number | null = null;
  let dragContainer: HTMLElement | null = null;

  function liveRowEls(container: HTMLElement): HTMLElement[] {
    // Строки живых свойств — те, у кого есть ручка; скрытые в драге не участвуют.
    return Array.from(container.querySelectorAll<HTMLElement>('button.drag-handle'))
      .map((h) => h.closest<HTMLElement>('tr'))
      .filter((tr): tr is HTMLElement => tr !== null);
  }

  function resetDrag(): void {
    if (dragContainer !== null && dragPointerId !== null) {
      try {
        dragContainer.releasePointerCapture(dragPointerId);
      } catch {
        // Захват уже снят (например, pointercancel опередил).
      }
    }
    dragOriginId = null;
    dragPointerId = null;
    dragStarted = false;
    dragInsertIndex = null;
    dragContainer = null;
    draggingId = null;
    dragGhost = null;
    indicatorTop = null;
  }

  function handlePointerDown(e: PointerEvent): void {
    if (!canEdit) return;
    if (e.pointerType === 'mouse' && e.button !== 0) return;
    const handle = (e.target as HTMLElement).closest<HTMLElement>('.drag-handle');
    if (!handle) return;
    const id = Number(handle.dataset.propId);
    if (!Number.isFinite(id)) return;
    dragOriginId = id;
    dragPointerId = e.pointerId;
    dragStartX = e.clientX;
    dragStartY = e.clientY;
    dragStarted = false;
    dragInsertIndex = null;
  }

  function handlePointerMove(e: PointerEvent): void {
    if (dragOriginId === null || e.pointerId !== dragPointerId) return;
    const container = e.currentTarget as HTMLElement;
    if (!dragStarted) {
      if (Math.hypot(e.clientX - dragStartX, e.clientY - dragStartY) < DRAG_START_THRESHOLD_PX) {
        return;
      }
      dragStarted = true;
      dragContainer = container;
      draggingId = dragOriginId;
      const dragged = currentProperty(dragOriginId);
      dragGhost = { label: dragged?.name ?? '', x: e.clientX, y: e.clientY };
      container.setPointerCapture(e.pointerId);
    }
    e.preventDefault();
    if (dragGhost) dragGhost = { ...dragGhost, x: e.clientX, y: e.clientY };

    const rects = liveRowEls(container).map((el) => el.getBoundingClientRect());
    const idx = insertionIndex(rects, e.clientY);
    dragInsertIndex = idx;
    if (rects.length > 0) {
      const edge = idx < rects.length ? rects[idx].top : rects[rects.length - 1].bottom;
      indicatorTop = edge - container.getBoundingClientRect().top;
    }
  }

  function handlePointerUp(e: PointerEvent): void {
    if (dragOriginId === null || e.pointerId !== dragPointerId) return;
    const originId = dragOriginId;
    const started = dragStarted;
    const to = dragInsertIndex;
    resetDrag();
    if (!started || to === null) return; // обычный клик по ручке — без перестановки
    const from = liveRows.findIndex((r) => r.id === originId);
    if (from >= 0) moveLive(from, to);
  }

  function handlePointerCancel(e: PointerEvent): void {
    if (dragOriginId === null || e.pointerId !== dragPointerId) return;
    resetDrag();
  }

  function handleWindowKeydown(e: KeyboardEvent): void {
    if (draggingId !== null && e.key === 'Escape') {
      e.preventDefault();
      resetDrag();
    }
  }

  // ---- Скрыть / вернуть ------------------------------------------------------
  let hideTarget = $state<GroupTypePropertyDto | null>(null);
  let hiding = $state(false);

  async function confirmHide(): Promise<void> {
    const target = hideTarget;
    if (!target) return;
    hiding = true;
    try {
      const outcome = await groupTypes.deleteProperty(target.id);
      hideTarget = null;
      // Сервер сам решает: заполненное скрывает, пустое удаляет физически.
      pushToast('success', outcome.archived ? 'Свойство скрыто' : 'Свойство удалено');
      await onReload();
    } catch (e) {
      pushToast('error', errMessage(e, 'Не удалось скрыть свойство.'));
    } finally {
      hiding = false;
    }
  }

  function unhide(p: GroupTypePropertyDto): void {
    void enqueue(async () => {
      try {
        await groupTypes.unarchiveProperty(p.id);
        pushToast('success', 'Свойство возвращено');
        await onReload();
      } catch (e) {
        pushToast('error', errMessage(e, 'Не удалось вернуть свойство.'));
      }
    });
  }

  // ---- Строка добавления -----------------------------------------------------
  let newName = $state('');
  let newDataType = $state('text');
  let newNameErr = $state<string | null>(null);
  let adding = $state(false);

  async function addProperty(): Promise<void> {
    const name = newName.trim();
    if (name === '') {
      newNameErr = 'Укажите название.';
      return;
    }
    adding = true;
    newNameErr = null;
    try {
      await groupTypes.createProperty({
        type_id: typeId,
        name,
        data_type: newDataType,
        is_required: false,
        show_on_map: false,
      });
      newName = '';
      newDataType = 'text';
      pushToast('success', 'Свойство добавлено');
      await onReload();
      document.getElementById(newNameId)?.focus();
    } catch (e) {
      const err = e as Partial<AppError> | undefined;
      if (err?.code === 'VALIDATION') newNameErr = err.message ?? 'Ошибка валидации';
      else pushToast('error', errMessage(e, 'Не удалось добавить свойство.'));
    } finally {
      adding = false;
    }
  }

  function onAddSubmit(e: Event): void {
    e.preventDefault();
    void addProperty();
  }

  function filledText(n: number): string {
    return `заполнено в ${n} ${pluralizeRu(n, ['группе', 'группах', 'группах'])}`;
  }
</script>

{#snippet typePicker(
  id: string,
  value: string,
  disabled: boolean,
  invalid: boolean,
  onPick: (_o: TypeOption) => void,
)}
  <Dropdown
    {id}
    variant="select"
    flat={true}
    value={typeLabel(value)}
    searchable={false}
    {invalid}
    {disabled}
    loading={false}
    groups={DATA_TYPES}
    getGroupId={(o) => o.value}
    getGroupName={(o) => o.label}
    getGroupCount={() => 0}
    isGroupExpandable={() => false}
    isGroupSelected={(o) => o.value === value}
    onExpandGroup={noExpand}
    getMemberId={(o) => o.value}
    getMemberName={(o) => o.label}
    onSearch={() => {}}
    onPickGroup={onPick}
    onPickMember={() => {}}
  />
{/snippet}

<svelte:window onkeydown={handleWindowKeydown} />

<div
  class="props-table"
  class:dragging-active={draggingId !== null}
  role="presentation"
  onpointerdown={handlePointerDown}
  onpointermove={handlePointerMove}
  onpointerup={handlePointerUp}
  onpointercancel={handlePointerCancel}
>
  <Table
    columns={6}
    framed={false}
    {loading}
    empty={rows.length === 0}
    emptyTitle="Свойств нет"
    emptyBody="Добавьте первое свойство в строке ниже — оно появится в форме каждой группы этого типа."
  >
    {#snippet head()}
      <th class="col-handle" aria-hidden="true"></th>
      <th>Название</th>
      <th>Тип данных</th>
      <th class="col-flag">Обязательное</th>
      <th class="col-flag" title="Будет показано на карте">На карте</th>
      <th class="col-menu" aria-hidden="true"></th>
    {/snippet}
    {#each rows as p, i (p.id)}
      {@const liveIdx = liveRows.findIndex((r) => r.id === p.id)}
      <TableRow
        class={['prop-row', p.archived && 'prop-row-hidden', draggingId === p.id && 'dragging']
          .filter(Boolean)
          .join(' ')}
        last={i === rows.length - 1}
      >
        <td class="col-handle">
          {#if canEdit && !p.archived}
            <button
              type="button"
              class="drag-handle"
              data-prop-id={p.id}
              aria-label={`Переместить свойство «${p.name}»`}
              title="Перетащите, чтобы изменить порядок"
            >
              ⠿
            </button>
          {/if}
        </td>
        <td class="cell-name">
          {#if canEdit && !p.archived}
            <div class="name-edit" onfocusout={() => saveName(p)}>
              <Input
                value={nameDraft[p.id] ?? p.name}
                invalid={!!nameErr[p.id]}
                oninput={(v) => onNameInput(p.id, v)}
              />
            </div>
            {#if nameErr[p.id]}
              <p class="field-error" role="alert">{nameErr[p.id]}</p>
            {/if}
          {:else}
            <span class="name-text" class:muted={p.archived} title={p.name}>{p.name}</span>
            {#if p.archived}<Badge variant="default" size="sm">Скрыто</Badge>{/if}
          {/if}
        </td>
        <td class="cell-type">
          {#if canEdit && !p.archived}
            {@render typePicker(
              `gtp-type-${p.id}`,
              p.data_type,
              p.filled_group_count > 0,
              false,
              (o) => pickType(p, o),
            )}
            {#if p.filled_group_count > 0}
              <p class="filled-hint">{filledText(p.filled_group_count)}</p>
            {/if}
          {:else}
            <span class="name-text" class:muted={p.archived}>{typeLabel(p.data_type)}</span>
          {/if}
        </td>
        <td class="col-flag">
          {#key rev[p.id] ?? 0}
            <Checkbox
              checked={p.is_required}
              disabled={!canEdit || p.archived}
              onchange={(v) => patchProperty(p, { is_required: v })}
            >
              <span class="sr-only">Обязательное: {p.name}</span>
            </Checkbox>
          {/key}
        </td>
        <td class="col-flag">
          {#key rev[p.id] ?? 0}
            <Checkbox
              checked={p.show_on_map}
              disabled={!canEdit || p.archived}
              onchange={(v) => patchProperty(p, { show_on_map: v })}
            >
              <span class="sr-only">На карте: {p.name}</span>
            </Checkbox>
          {/key}
        </td>
        <td class="col-menu">
          {#if canEdit}
            <ActionMenu variant="ghost-sm" label={`Действия: ${p.name}`}>
              {#if p.archived}
                <button type="button" role="menuitem" onclick={() => unhide(p)}>Вернуть</button>
              {:else}
                <button
                  type="button"
                  role="menuitem"
                  disabled={liveIdx <= 0}
                  onclick={() => moveUp(p)}
                >
                  Выше
                </button>
                <button
                  type="button"
                  role="menuitem"
                  disabled={liveIdx < 0 || liveIdx >= liveRows.length - 1}
                  onclick={() => moveDown(p)}
                >
                  Ниже
                </button>
                <button type="button" role="menuitem" onclick={() => (hideTarget = p)}>
                  Скрыть
                </button>
              {/if}
            </ActionMenu>
          {/if}
        </td>
      </TableRow>
    {/each}
    {#snippet footer()}
      {#if canEdit}
        <form class="add-row" onsubmit={onAddSubmit}>
          <div class="add-name">
            <Input
              id={newNameId}
              value={newName}
              placeholder="Название свойства"
              invalid={newNameErr !== null}
              disabled={adding}
              oninput={(v) => {
                newName = v;
                newNameErr = null;
              }}
            />
            {#if newNameErr}
              <p class="field-error" role="alert">{newNameErr}</p>
            {/if}
          </div>
          <div class="add-type">
            {@render typePicker(`gtp-new-type-${typeId}`, newDataType, adding, false, (o) => {
              newDataType = o.value;
            })}
          </div>
          <Button variant="secondary" type="submit" loading={adding}>Добавить свойство</Button>
        </form>
      {/if}
    {/snippet}
  </Table>
  <div class="sr-only" aria-live="polite" role="status">{liveMessage}</div>
  {#if draggingId !== null && indicatorTop !== null}
    <div class="insert-indicator" style:top="{indicatorTop}px" aria-hidden="true"></div>
  {/if}
</div>

{#if dragGhost}
  <div
    class="drag-ghost"
    style:left="{dragGhost.x + 12}px"
    style:top="{dragGhost.y + 12}px"
    aria-hidden="true"
  >
    {dragGhost.label}
  </div>
{/if}

{#if hideTarget}
  <Modal open={true} size="md" title="Скрыть свойство" onClose={() => (hideTarget = null)}>
    <p class="confirm-text">
      Свойство «{hideTarget.name}» исчезнет из форм групп. Заполненные значения останутся в базе,
      свойство можно будет вернуть.
    </p>
    {#snippet footer()}
      <Button variant="secondary" onclick={() => (hideTarget = null)} disabled={hiding}>
        Отмена
      </Button>
      <Button variant="primary" loading={hiding} onclick={confirmHide}>Скрыть</Button>
    {/snippet}
  </Modal>
{/if}

{#if violators}
  <PropertyRequiredViolatorsPopup
    groups={violators.groups}
    total={violators.total}
    onOpen={openViolator}
    onClose={() => (violators = null)}
  />
{/if}

<style lang="scss">
  .props-table {
    position: relative;

    // Во время драга не выделяем текст (снимается в resetDrag).
    &.dragging-active {
      user-select: none;
      -webkit-user-select: none;
    }
  }

  // Исходная строка на время драга; TableRow рендерит <tr> в своём компоненте.
  :global(tr.prop-row.dragging) {
    opacity: 0.4;
  }

  // Индикатор вставки: 2px на всю ширину таблицы, на границе вставки.
  .insert-indicator {
    position: absolute;
    left: 0;
    right: 0;
    height: 2px;
    margin-top: -1px;
    background: var(--tr-accent);
    pointer-events: none;
    z-index: 2;
  }

  // Призрак: pointer-events: none обязателен — иначе перехватывает попадания
  // (GAP-11 фазы 39). Анимаций нет (prefers-reduced-motion не затрагивается).
  .drag-ghost {
    position: fixed;
    z-index: 1000;
    max-width: 260px;
    padding: var(--tr-space-2xs) var(--tr-space-sm);
    border: 1px solid var(--tr-border-strong);
    border-radius: var(--tr-radius-sm);
    background: var(--tr-surface-raised);
    box-shadow: var(--tr-elev-3);
    color: var(--tr-text-primary);
    font-size: var(--tr-font-size-body);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    pointer-events: none;
  }

  .col-handle {
    width: 36px;
    padding-right: 0;
  }
  .col-flag {
    width: 120px;
    text-align: center;
  }
  .col-menu {
    width: 44px;
    text-align: right;
  }

  .drag-handle {
    width: 28px;
    height: 28px;
    padding: 0;
    border: none;
    border-radius: var(--tr-radius-xs);
    background: transparent;
    color: var(--tr-text-tertiary);
    font-size: 16px;
    line-height: 1;
    cursor: grab;
    // Жест стартует только за ручкой; отключение жестов браузера нужно именно здесь,
    // чтобы тач не превращал перетаскивание в прокрутку панели.
    touch-action: none;

    &:hover {
      background: var(--tr-row-hover);
      color: var(--tr-text-secondary);
    }
  }

  .cell-name,
  .cell-type {
    vertical-align: top;
  }

  .name-text {
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-primary);

    &.muted {
      color: var(--tr-text-tertiary);
    }
  }

  .filled-hint {
    margin: var(--tr-space-3xs) 0 0;
    font: var(--tr-text-label);
    color: var(--tr-warning-text);
  }

  .field-error {
    margin: var(--tr-space-3xs) 0 0;
    font-size: var(--tr-font-size-label);
    color: var(--tr-danger);
  }

  .add-row {
    display: flex;
    align-items: flex-start;
    gap: var(--tr-space-sm);
  }
  .add-name {
    flex: 1 1 auto;
    min-width: 0;
  }
  .add-type {
    flex: 0 0 220px;
  }

  .confirm-text {
    margin: 0;
    color: var(--tr-text-primary);
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    margin: -1px;
    padding: 0;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }
</style>
