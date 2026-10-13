<script module lang="ts">
  import { groupTypes } from '$lib/api/groups';
  import type { GroupTypeDto } from '../../bindings';

  // Типы групп общие для всех строк: меню порождает быстрые действия из них
  // (D-22), а грузить список на каждую строку незачем. Кешируются ТИПЫ, а не
  // готовый список пунктов: пункты зависят от устройства. Кеш обновляется в
  // фоне при каждом открытии меню, чтобы переименование быстрого действия
  // администратором доходило без перезапуска.
  const typesCache = $state<{ items: GroupTypeDto[] | null }>({ items: null });
  let typesInflight: Promise<void> | null = null;

  function refreshTypes(): Promise<void> {
    if (typesInflight) return typesInflight;
    typesInflight = groupTypes
      .list(false)
      .then((rows) => {
        typesCache.items = rows;
      })
      .catch(() => {
        // Ошибка загрузки: быстрых действий в меню не будет, остальные пункты
        // работают (меню никогда не открывается с пустой панелью).
      })
      .finally(() => {
        typesInflight = null;
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
  import ActionMenu from '$lib/components/ActionMenu.svelte';
  import Button from '$lib/components/Button.svelte';
  import Modal from '$lib/components/Modal.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { notifyPlaceContentChanged } from '$lib/stores/placeContentEvents.svelte';
  import { ensureLoaded, statusLabel } from '$lib/stores/deviceStatuses.svelte';
  import { devices } from './api';
  import PlaceEntityViewModal from '../places/PlaceEntityViewModal.svelte';
  import AddToGroupModal from '../groups/AddToGroupModal.svelte';
  import GroupFormModal from '../groups/GroupFormModal.svelte';
  import {
    buildMenuSections,
    isGroupActionAllowed,
    quickActionsForDevice,
    type MenuItemSpec,
  } from './deviceRowMenu';
  import type { DeviceDto, PlaceContentDto } from '../../bindings';

  interface Props {
    /** Устройство строки; null — взять по `deviceId` перед каждым открытием (принтеры). */
    device: DeviceDto | null;
    deviceId?: number;
    trigger?: 'button' | 'none';
    /** false для сетевого принтера: групповые пункты не показываются. */
    groupActionsEnabled?: boolean;
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
    groupActionsEnabled = true,
    onPoll,
    onEdit,
    onChanged,
    onPrintAcceptance,
  }: Props = $props();

  // Вынесено в константу: prettier не переносит строковые литералы, поэтому
  // маркер словарного гейта (check-group-vocabulary) остаётся одной строкой.
  const REFUSE_HINT =
    'Сначала удалите группу в разделе «Группы»: после этого устройство снова станет обычным.';

  // type_id принтера: тот же, что различает kind строки в PlaceEntityViewModal.
  const PRINTER_DEVICE_TYPE_ID = 2;

  let menu = $state<ReturnType<typeof ActionMenu> | null>(null);
  // Устройство, подтянутое по deviceId (когда device === null).
  let fetched = $state<DeviceDto | null>(null);
  const dev = $derived(device ?? fetched);

  let viewRow = $state<PlaceContentDto | null>(null);
  let addToGroupOpen = $state(false);
  let quickTypeId = $state<number | null>(null);
  let refuseOpen = $state(false);
  let confirmOpen = $state(false);
  let deleting = $state(false);

  const sections = $derived(
    buildMenuSections({
      hasPoll: onPoll !== undefined,
      hasPrint: onPrintAcceptance !== undefined,
      groupActionsVisible:
        dev !== null &&
        groupActionsEnabled &&
        isGroupActionAllowed({
          statusCode: dev.status_code,
          isMember: dev.group_id != null,
          isAnchor: dev.anchor_group_id != null,
          isNetworkPrinter: false,
        }),
      quickActions: dev === null ? [] : quickActionsForDevice(typesCache.items ?? [], dev.type_id),
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
    }
  });

  function errorMessage(e: unknown, fallback: string): string {
    return e && typeof e === 'object' && 'message' in e
      ? String((e as { message: unknown }).message)
      : fallback;
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

  function handleQuickSaved() {
    quickTypeId = null;
    onChanged();
  }

  function openAnchorGroup() {
    const groupId = dev?.anchor_group_id;
    refuseOpen = false;
    if (groupId != null) window.location.hash = '#/groups?id=' + groupId;
  }

  async function handleDelete() {
    if (!dev) return;
    deleting = true;
    try {
      await devices.delete(dev.id, dev.version);
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
    if (isOpen) void refreshTypes();
  }}
>
  {#each sections as section, si (si)}
    {#if si > 0}
      <!-- svelte-ignore a11y_no_redundant_roles -->
      <hr role="separator" />
    {/if}
    {#each section as item (item.kind + ':' + (item.typeId ?? '') + ':' + item.label)}
      <button
        type="button"
        role="menuitem"
        class:menu-danger={item.danger}
        onclick={() => activate(item)}
      >
        {item.label}
      </button>
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
