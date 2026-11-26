<script module lang="ts">
  // type_id принтера: тот же, что различает kind строки в PlaceEntityViewModal
  // (единый модуль, INT-31); имя PRINTER_DEVICE_TYPE_ID держат якоря check-device-row-menu.
  import { PRINTER_TYPE_ID as PRINTER_DEVICE_TYPE_ID } from '$lib/deviceTypeIds';
  import { groupTypes, groups } from '$lib/api/groups';
  import { registerUserScopedReset } from '@trackly/kit/stores/userScoped.svelte';
  import type { GroupTypeDto } from '../../bindings';

  // Типы групп общие для всех строк: меню порождает быстрые действия из них
  // (D-22), а грузить список на каждую строку незачем. Кешируются ТИПЫ, а не
  // готовый список пунктов: пункты зависят от устройства. Кеш обновляется в
  // фоне при каждом открытии меню, чтобы переименование быстрого действия
  // администратором доходило без перезапуска.
  const typesCache = $state<{ items: GroupTypeDto[] | null }>({ items: null });
  let typesInflight: Promise<void> | null = null;
  // D-39: поколение сброса; ответ прежнего пользователя в кэш не пишется.
  let typesGeneration = 0;

  registerUserScopedReset(() => {
    typesGeneration += 1;
    typesCache.items = null;
    typesInflight = null;
  });

  function refreshTypes(): Promise<void> {
    if (typesInflight) return typesInflight;
    const gen = typesGeneration;
    typesInflight = groupTypes
      .list(false)
      .then((rows) => {
        if (gen === typesGeneration) typesCache.items = rows;
      })
      .catch(() => {
        // Ошибка загрузки: быстрых действий в меню не будет, остальные пункты
        // работают (меню никогда не открывается с пустой панелью).
      })
      .finally(() => {
        if (gen === typesGeneration) typesInflight = null;
      });
    return typesInflight;
  }
</script>

<script lang="ts">
  // Фаза 41.1, план 16 (GRD-01, GRD-05; D-21, D-22, D-27, D-29): единая точка
  // владения меню строки устройства. Один набор пунктов на три точки входа:
  // кебаб (trigger="button"), правый клик (openAt) и клавиша контекстного меню
  // (openFromElement). Заменяет вторую реализацию портального меню
  // DeviceContextMenu, расходившуюся с ActionMenu.
  //
  // Состав и видимость пунктов решает чистая функция buildMenuSections
  // (deviceRowMenu.ts, golden-гейт check-device-row-menu.mjs); здесь только
  // отрисовка и модалки пунктов.
  //
  // События из модалок этого компонента: он стоит внутри <td>, а Modal не
  // портальный, поэтому правый клик в поле названия, двойной клик по
  // инвентарному номеру в просмотре и пробел в поле названия группы
  // всплывают до <tr> и первой ячейки. Решение единственное: ОХРАНА В
  // ОБРАБОТЧИКАХ СТРОКИ (isRowOverlayEvent / isOwnKeyEvent первым оператором,
  // правило [F] гейта). stopPropagation здесь не ставится: два механизма
  // разойдутся. Подавление родного контекстного меню — тоже забота строки.
  import { onMount } from 'svelte';
  import ActionMenu from '@trackly/kit/components/ActionMenu.svelte';
  import Button from '@trackly/kit/components/Button.svelte';
  import Modal from '@trackly/kit/components/Modal.svelte';
  import { pushToast } from '@trackly/kit/stores/toast.svelte';
  import { authStore } from '$lib/stores/auth.svelte';
  import { notifyPlaceContentChanged } from '$lib/stores/placeContentEvents.svelte';
  import ActionSubmenu from '@trackly/kit/components/ActionSubmenu.svelte';
  import { ensureLoaded, orderedStatuses, statusLabel } from '$lib/stores/deviceStatuses.svelte';
  import { devices } from './api';
  import PlaceEntityViewModal from '../places/PlaceEntityViewModal.svelte';
  import AddToGroupModal from '../groups/AddToGroupModal.svelte';
  import GroupFormModal from '../groups/GroupFormModal.svelte';
  import WriteOffChoiceModal from '../groups/WriteOffChoiceModal.svelte';
  import WriteOffDeviceModal from '../groups/WriteOffDeviceModal.svelte';
  import WriteOffGroupModal from '../groups/WriteOffGroupModal.svelte';
  import {
    buildMenuSections,
    canMoveToGroup,
    canRemoveFromGroup,
    canWriteOff,
    isGroupActionAllowed,
    quickActionsForDevice,
    type MenuItemSpec,
  } from './deviceRowMenu';
  import { canChangeStatus, statusMenuOptions, type StatusOptionSpec } from './deviceStatusMenu';
  import { writeOffChoiceNeeded } from './writeOffChoice';
  import { WRITE_OFF_GROUP_FALLBACK_NAME, anchorWriteOffRefusal } from '../groups/writeOffTexts';
  import { loadOpenRepairs } from '../requests/repairLocks';
  import type {
    DeviceDto,
    DevicePatch,
    DeviceSaveOutcome,
    GroupDto,
    PlaceContentDto,
  } from '../../bindings';

  interface Props {
    /** Устройство строки; null — взять по `deviceId` перед каждым открытием (принтеры). */
    device: DeviceDto | null;
    deviceId?: number;
    trigger?: 'button' | 'none';
    /** Пункт «Опросить сейчас» (принтеры). Не передан — пункта нет. */
    onPoll?: () => void;
    onEdit: (_d: DeviceDto) => void;
    /** Общий сигнал «перезагрузить список и счётчики». */
    onChanged: () => void;
    /** Пункт «Печать документа приёма». Не передан — пункта нет. */
    onPrintAcceptance?: (_d: DeviceDto) => void;
  }

  const {
    device,
    deviceId,
    trigger = 'button',
    onPoll,
    onEdit,
    onChanged,
    onPrintAcceptance,
  }: Props = $props();

  // Вынесено в константу: prettier не переносит строковые литералы, поэтому
  // маркер словарного гейта (check-group-vocabulary) остаётся одной строкой.
  const REFUSE_HINT =
    'Сначала удалите группу в разделе «Группы»: после этого устройство снова станет обычным.';

  let menu = $state<ReturnType<typeof ActionMenu> | null>(null);
  // Устройство, подтянутое по deviceId (когда device === null).
  let fetched = $state<DeviceDto | null>(null);
  const dev = $derived(device ?? fetched);

  let viewRow = $state<PlaceContentDto | null>(null);
  let addToGroupOpen = $state(false);
  let moveOpen = $state(false);
  let writeOffDevice = $state<DeviceDto | null>(null);
  let writeOffGroup = $state<GroupDto | null>(null);
  let quickTypeId = $state<number | null>(null);
  let refuseOpen = $state(false);
  let confirmOpen = $state(false);
  let deleting = $state(false);
  let removing = false;
  let changing = false;
  // Номер открытой ремонтной заявки по устройству строки (D-21 фазы 41.4); null —
  // заявки нет либо права на ремонты нет (запрет тогда держит сервер).
  let openRepairRequestId = $state<number | null>(null);
  // Выбор «оставить / вывести» при списании участника группы (D-26); resolve(null) — отмена.
  let writeOffPrompt = $state<{
    deviceName: string;
    groupName: string;
    isTeardownMember: boolean;
    resolve: (release: boolean | null) => void;
  } | null>(null);

  const statusOptions = $derived<StatusOptionSpec[]>(
    dev === null
      ? []
      : statusMenuOptions({
          statuses: orderedStatuses(),
          currentStatusId: dev.status_id,
          isAnchor: dev.anchor_group_id != null,
          anchorGroupName: dev.anchor_group_name ?? null,
          openRepairRequestId,
        }),
  );

  const sections = $derived(
    buildMenuSections({
      hasPoll: onPoll !== undefined,
      hasPrint: onPrintAcceptance !== undefined,
      groupActionsVisible:
        dev !== null &&
        isGroupActionAllowed({
          statusCode: dev.status_code,
          isMember: dev.group_id != null,
          isAnchor: dev.anchor_group_id != null,
        }),
      quickActions: dev === null ? [] : quickActionsForDevice(typesCache.items ?? [], dev.type_id),
      removeFromGroup:
        dev !== null &&
        canRemoveFromGroup({
          role: authStore.user?.role ?? null,
          groupId: dev.group_id,
          anchorGroupId: dev.anchor_group_id,
        }),
      moveToGroup:
        dev !== null &&
        canMoveToGroup({
          role: authStore.user?.role ?? null,
          groupId: dev.group_id,
          anchorGroupId: dev.anchor_group_id,
          statusCode: dev.status_code,
        }),
      writeOff:
        dev !== null &&
        canWriteOff({ role: authStore.user?.role ?? null, statusCode: dev.status_code }),
      status:
        dev !== null &&
        canChangeStatus({ role: authStore.user?.role ?? null }) &&
        statusOptions.length > 0,
    }),
  );

  const menuLabel = $derived(
    dev ? `Действия с устройством: ${dev.name}` : 'Действия с устройством',
  );

  onMount(() => {
    // Кебаб открывается самим ActionMenu, мимо prepare(): подготовить данные заранее.
    if (trigger === 'button') {
      void ensureLoaded();
      if (typesCache.items === null) void refreshTypes();
      void refreshOpenRepair();
    }
  });

  // Запись только из async-продолжения; устаревший ответ (устройство сменилось) отбрасывается.
  async function refreshOpenRepair(): Promise<void> {
    const id = device?.id ?? deviceId;
    if (id === undefined) return;
    const map = await loadOpenRepairs([id]);
    if ((device?.id ?? deviceId) === id) openRepairRequestId = map.get(id) ?? null;
  }

  function errorMessage(e: unknown, fallback: string): string {
    const msg =
      e && typeof e === 'object' && 'message' in e
        ? String((e as { message: unknown }).message)
        : '';
    return msg !== '' ? msg : fallback;
  }

  /**
   * Всё нужное меню до его показа: справочник статусов, типы групп и (для
   * принтеров) свежее устройство — набор пунктов считается по загруженным
   * данным, без мигания. false — открывать нечего (ошибка уже показана).
   */
  async function prepare(): Promise<boolean> {
    const jobs: Promise<unknown>[] = [ensureLoaded()];
    if (typesCache.items === null) jobs.push(refreshTypes());
    else void refreshTypes();
    if (device === null) {
      if (deviceId === undefined) return false;
      try {
        // Каждый раз заново: статус и членство в группе могли измениться.
        fetched = await devices.get(deviceId);
      } catch (e) {
        pushToast('error', errorMessage(e, 'Не удалось загрузить устройство'));
        return false;
      }
    }
    jobs.push(refreshOpenRepair());
    await Promise.all(jobs);
    return true;
  }

  /** Открыть меню у точки (правый клик). */
  export function openAt(x: number, y: number): Promise<void> {
    return prepare().then((ok) => {
      if (ok) menu?.openAt(x, y);
    });
  }

  /** Открыть меню у левого-нижнего угла элемента (клавиша контекстного меню). */
  export function openFromElement(el: HTMLElement): Promise<void> {
    const rect = el.getBoundingClientRect();
    return openAt(rect.left, rect.bottom);
  }

  /** Открыть просмотр без меню (двойной клик по строке). */
  export function openView(): Promise<void> {
    return prepare().then((ok) => {
      if (ok) showView();
    });
  }

  function showView() {
    if (!dev) return;
    viewRow = {
      kind: dev.type_id === PRINTER_DEVICE_TYPE_ID ? 'printer' : 'device',
      id: dev.id,
      name: dev.name,
      inventory_or_code: dev.inventory_no ?? null,
      full_path: dev.full_path ?? '',
      place_path_short: dev.place_path_short ?? '',
      status_name: statusLabel(dev.status_id),
      status_code: dev.status_code,
      group_type_name: null,
      member_count: null,
      printer_member_count: null,
    };
  }

  // Правка из формы внутри просмотра могла сменить место устройства:
  // аргумент (id затронутых мест) НЕ терять — иначе счётчики дерева «Мест» не
  // обновятся (WR-08).
  function handleViewChanged(changedPlaceIds: number[]) {
    viewRow = null;
    if (changedPlaceIds.length > 0) notifyPlaceContentChanged(changedPlaceIds);
    onChanged();
  }

  function activate(item: MenuItemSpec) {
    switch (item.kind) {
      case 'poll':
        onPoll?.();
        break;
      case 'view':
        showView();
        break;
      case 'edit':
        if (dev) onEdit(dev);
        break;
      case 'print':
        if (dev) onPrintAcceptance?.(dev);
        break;
      case 'addToGroup':
        addToGroupOpen = true;
        break;
      case 'quick':
        quickTypeId = item.typeId ?? null;
        break;
      case 'removeFromGroup':
        void removeFromGroup();
        break;
      case 'moveToGroup':
        moveOpen = true;
        break;
      case 'writeOff':
        void startWriteOff();
        break;
      case 'status':
        // Пункт рисуется ActionSubmenu; выбор варианта идёт через changeStatus.
        break;
      case 'delete':
        // Якорь не скрывается и не блокируется: ведёт в модалку отказа (D-14).
        if (dev?.anchor_group_id != null) refuseOpen = true;
        else confirmOpen = true;
        break;
    }
  }

  function handleAdded() {
    addToGroupOpen = false;
    onChanged();
  }

  function handleMoved() {
    moveOpen = false;
    onChanged();
  }

  function handleWrittenOff() {
    writeOffDevice = null;
    writeOffGroup = null;
    onChanged();
  }

  // R-03: у якоря одна подпись «Списать» и один смысл — списание группы.
  async function startWriteOff() {
    const target = dev;
    if (!target) return;
    if (target.anchor_group_id == null) {
      writeOffDevice = target;
      return;
    }
    try {
      writeOffGroup = await groups.get(target.anchor_group_id);
    } catch {
      pushToast('error', 'Не удалось загрузить устройство. Проверьте подключение и повторите.');
    }
  }

  function handleQuickSaved() {
    quickTypeId = null;
    onChanged();
  }

  function openAnchorGroup() {
    const groupId = dev?.anchor_group_id;
    refuseOpen = false;
    if (groupId != null) window.location.hash = '#/groups?id=' + groupId;
  }

  // GRD-12: вывод без подтверждения, как в разделе «Группы» (D-02 фазы 41, D-02 фазы 41.8).
  // Группа и устройство захватываются ДО await: `dev` может смениться при
  // перечитывании списка. Место не меняется, поэтому notifyPlaceContentChanged
  // не нужен; остальные клиенты получают WS-событие entities_changed.
  async function removeFromGroup() {
    const target = dev;
    const ownerGroupId = target?.group_id;
    const ownerGroupName = target?.group_name ?? '';
    if (!target || ownerGroupId == null || removing) return;
    removing = true;
    try {
      await groups.removeDevices({ group_id: ownerGroupId, device_ids: [target.id] });
      pushToast('success', `Устройство выведено из группы «${ownerGroupName}»`);
      onChanged();
    } catch (e: unknown) {
      pushToast(
        'error',
        errorMessage(e, 'Не удалось вывести из группы. Проверьте подключение и повторите.'),
      );
    } finally {
      removing = false;
    }
  }

  function askWriteOffChoice(target: DeviceDto): Promise<boolean | null> {
    return new Promise((resolve) => {
      writeOffPrompt = {
        deviceName: target.name,
        groupName: target.group_name ?? WRITE_OFF_GROUP_FALLBACK_NAME,
        isTeardownMember:
          target.origin_group_id != null && target.origin_group_id === target.group_id,
        resolve: (v) => {
          writeOffPrompt = null;
          resolve(v);
        },
      };
    });
  }

  // D-35: смена статуса из меню строки по правилам формы (D-26). Устройство
  // захватывается ДО await. Патч несёт только status_id и (из выбора пользователя)
  // group_write_off; обязательные ключи DevicePatch — null (не менять).
  async function changeStatus(option: StatusOptionSpec) {
    const target = dev;
    if (!target || changing || option.disabled || option.current) return;
    const verdict = writeOffChoiceNeeded({
      statusCodeBefore: target.status_code,
      statusCodeAfter: option.code,
      groupId: target.group_id,
      anchorGroupId: target.anchor_group_id,
    });
    if (verdict === 'anchor_refuse') {
      pushToast('error', anchorWriteOffRefusal(target.anchor_group_name));
      return;
    }
    const patch: DevicePatch = {
      type_id: null,
      name: null,
      number_input: null,
      status_id: option.id,
    };
    changing = true;
    try {
      if (verdict === 'choose') {
        const release = await askWriteOffChoice(target);
        if (release === null) return;
        patch.group_write_off = release ? 'release' : 'keep';
      }
      let outcome: DeviceSaveOutcome;
      let retried = false;
      for (;;) {
        try {
          outcome = await devices.update(target.id, target.version, patch);
          break;
        } catch (e) {
          const verr = e as { code?: string; details?: { field?: string } } | null;
          if (
            !retried &&
            verr?.code === 'VALIDATION' &&
            verr.details?.field === 'group_write_off'
          ) {
            // Членство устарело: сервер просит выбор. Один повтор.
            retried = true;
            const release = await askWriteOffChoice(target);
            if (release === null) return;
            patch.group_write_off = release ? 'release' : 'keep';
            continue;
          }
          pushToast('error', errorMessage(e, 'Не удалось сменить статус'));
          return;
        }
      }
      if (outcome.outcome !== 'created') {
        pushToast('error', 'Не удалось сменить статус');
        return;
      }
      // Место могло смениться (возврат из исключённого статуса переносит на место группы, D-17).
      const placeIds = [target.place_id, outcome.place_id].filter(
        (id, i, a): id is number => id != null && a.indexOf(id) === i,
      );
      if (placeIds.length > 0) notifyPlaceContentChanged(placeIds);
      pushToast('success', `Статус изменён: ${statusLabel(option.id)}`);
      onChanged();
    } finally {
      changing = false;
    }
  }

  async function handleDelete() {
    if (!dev) return;
    // Место берём ДО await: после удаления счётчики именно этого места устарели.
    // Один обработчик обслуживает страницы «Устройства» и «Принтеры» (INT-15).
    const deletedPlaceId = dev.place_id;
    deleting = true;
    try {
      await devices.delete(dev.id, dev.version);
      if (deletedPlaceId !== null) notifyPlaceContentChanged([deletedPlaceId]);
      pushToast('success', 'Устройство удалено');
      confirmOpen = false;
      onChanged();
    } catch (e: unknown) {
      pushToast('error', errorMessage(e, 'Не удалось удалить устройство'));
    } finally {
      deleting = false;
    }
  }
</script>

<ActionMenu
  bind:this={menu}
  {trigger}
  variant="ghost-sm"
  label={menuLabel}
  onOpenChange={(isOpen) => {
    if (isOpen) {
      void refreshTypes();
      void refreshOpenRepair();
    }
  }}
>
  {#each sections as section, si (si)}
    {#if si > 0}
      <!-- svelte-ignore a11y_no_redundant_roles -->
      <hr role="separator" />
    {/if}
    {#each section as item (item.kind + ':' + (item.typeId ?? '') + ':' + item.label)}
      {#if item.kind === 'status'}
        <ActionSubmenu label={item.label}>
          {#each statusOptions as opt (opt.id)}
            <button
              type="button"
              role="menuitemradio"
              aria-checked={opt.current}
              aria-disabled={opt.disabled}
              title={opt.disabledReason ?? undefined}
              onclick={() => void changeStatus(opt)}
            >
              {opt.label}
            </button>
          {/each}
        </ActionSubmenu>
      {:else}
        <button
          type="button"
          role="menuitem"
          class:menu-danger={item.danger}
          onclick={() => activate(item)}
        >
          {item.label}
        </button>
      {/if}
    {/each}
  {/each}
</ActionMenu>

{#if viewRow}
  <PlaceEntityViewModal
    row={viewRow}
    onClose={() => (viewRow = null)}
    onChanged={handleViewChanged}
  />
{/if}

{#if addToGroupOpen && dev}
  <AddToGroupModal
    device={{ id: dev.id, name: dev.name }}
    onClose={() => (addToGroupOpen = false)}
    onAdded={handleAdded}
  />
{/if}

{#if moveOpen && dev}
  <AddToGroupModal
    mode="move"
    device={{ id: dev.id, name: dev.name }}
    currentGroup={dev.group_id != null ? { id: dev.group_id, name: dev.group_name ?? '' } : null}
    onClose={() => (moveOpen = false)}
    onAdded={handleMoved}
  />
{/if}

{#if writeOffDevice}
  <WriteOffDeviceModal
    device={writeOffDevice}
    isTeardownMember={writeOffDevice.origin_group_id != null &&
      writeOffDevice.origin_group_id === writeOffDevice.group_id}
    onClose={() => (writeOffDevice = null)}
    onDone={handleWrittenOff}
  />
{/if}

{#if writeOffPrompt}
  {@const prompt = writeOffPrompt}
  <WriteOffChoiceModal
    deviceName={prompt.deviceName}
    groupName={prompt.groupName}
    isTeardownMember={prompt.isTeardownMember}
    onChoose={(release) => prompt.resolve(release)}
    onClose={() => prompt.resolve(null)}
  />
{/if}

{#if writeOffGroup}
  <WriteOffGroupModal
    group={writeOffGroup}
    onClose={() => (writeOffGroup = null)}
    onDone={handleWrittenOff}
  />
{/if}

{#if quickTypeId !== null && dev}
  <GroupFormModal
    mode="create"
    presetTypeId={quickTypeId}
    presetAnchorDevice={{ id: dev.id, name: dev.name }}
    onClose={() => (quickTypeId = null)}
    onSaved={handleQuickSaved}
  />
{/if}

<Modal open={refuseOpen} title="Удалить нельзя" onClose={() => (refuseOpen = false)}>
  {#if dev}
    <p class="confirm-body">
      Устройство «{dev.name}» — якорь группы «{dev.anchor_group_name ?? '—'}». {REFUSE_HINT}
    </p>
  {/if}

  {#snippet footer()}
    <Button variant="secondary" onclick={() => (refuseOpen = false)}>Закрыть</Button>
    <Button variant="link" onclick={openAnchorGroup}>Открыть группу</Button>
  {/snippet}
</Modal>

<Modal open={confirmOpen} title="Удалить устройство?" onClose={() => (confirmOpen = false)}>
  {#if dev}
    <p class="confirm-body">
      «{dev.name}» (инв. № {dev.inventory_no ?? '—'}) будет помечено как удалённое. Действие можно
      отменить только восстановлением из резервной копии БД.
    </p>
  {/if}

  {#snippet footer()}
    <Button variant="secondary" onclick={() => (confirmOpen = false)}>Отмена</Button>
    <Button variant="destructive" loading={deleting} onclick={handleDelete}>Удалить</Button>
  {/snippet}
</Modal>

<style lang="scss">
  .confirm-body {
    margin: 0;
    color: var(--tr-text-secondary);
    line-height: var(--tr-line-height-body);
    text-align: center;
    overflow-wrap: anywhere;
    word-break: break-word;
    white-space: normal;
    max-width: 100%;
  }
</style>
