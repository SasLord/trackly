<script lang="ts">
  // Phase 41 Plan 23 (GRP-04, GRP-06, GRP-09, UI-SPEC 7.1-7.4): корневой компонент
  // раздела «Группы» — PageHeader + «Создать группу» + PlacesMasterDetail
  // (GroupTree | GroupTypePanel | GroupPanel) и модалки раздела.
  //
  // Выбор: страница хранит выбранный узел (`selected`), активную вкладку панели
  // группы и сохраняет их в localStorage `trackly:groups:*` (дерево хранит только
  // раскрытые ключи — одно место записи на одно состояние). URL-хеш
  // `#/groups?id=<группа>` / `#/groups?type=<тип>` ПОБЕЖДАЕТ сохранённое: так
  // работает переход-фокус из формы устройства (D-19), из таймлайна (D-28) и из
  // меню вложенной группы (D-07). Хеш при выборе пишется history.replaceState
  // (без hashchange и без лишней записи в истории) — как в PlacesPage.
  //
  // D-18: «Перенести…» — ОДИН обработчик `openMove` для двух точек входа (пункт
  // меню узла и кнопка в панели группы) и один GroupMoveModal.
  //
  // Все права здесь — UX-слой (граница безопасности — сервер): admin правит типы,
  // admin|manager — группы; employee раздел не видит (маршрут не в employeeRoutes).
  import { untrack } from 'svelte';
  import { push } from 'svelte-spa-router';
  import { authStore } from '$lib/stores/auth.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import {
    notifyPlaceContentChanged,
    placeContentEventsStore,
  } from '$lib/stores/placeContentEvents.svelte';
  import { groups as groupsApi } from '$lib/api/groups';
  import { devices } from '$lib/api/devices';
  import PageHeader from '$lib/components/PageHeader.svelte';
  import Button from '$lib/components/Button.svelte';
  import DetailPanel from '$lib/components/DetailPanel.svelte';
  import PlacesMasterDetail from '../places/PlacesMasterDetail.svelte';
  import GroupTree, { type GroupTreeNodeRef } from './GroupTree.svelte';
  import GroupTypePanel from './GroupTypePanel.svelte';
  import GroupPanel from './GroupPanel.svelte';
  import GroupFormModal from './GroupFormModal.svelte';
  import GroupTypeFormModal from './GroupTypeFormModal.svelte';
  import GroupDeleteModal from './GroupDeleteModal.svelte';
  import GroupMoveModal from './GroupMoveModal.svelte';
  import type { AppError } from '$lib/api/errors';
  import type { GroupDto, GroupTypeDto } from '../../bindings';

  // PRINTER_TYPE_ID: у принтера собственный раздел (`#/printers?id=…`).
  const PRINTER_TYPE_ID = 2;

  const SELECTED_STORAGE_KEY = 'trackly:groups:selected';
  const ACTIVE_TAB_STORAGE_KEY = 'trackly:groups:activeTab';
  const VALID_TABS = ['contents', 'properties', 'history'];

  function parseTargetFromHash(): GroupTreeNodeRef | null {
    if (typeof window === 'undefined') return null;
    const hash = window.location.hash;
    const qIdx = hash.indexOf('?');
    if (qIdx === -1) return null;
    const qs = new URLSearchParams(hash.slice(qIdx + 1));
    for (const [param, kind] of [
      ['id', 'group'],
      ['type', 'type'],
    ] as const) {
      const raw = qs.get(param);
      if (!raw) continue;
      const n = Number(raw);
      if (Number.isInteger(n)) return { kind, id: n };
    }
    return null;
  }

  function readPersistedSelected(): GroupTreeNodeRef | null {
    if (typeof window === 'undefined') return null;
    try {
      const raw = localStorage.getItem(SELECTED_STORAGE_KEY);
      if (!raw) return null;
      const parsed: unknown = JSON.parse(raw);
      if (parsed === null || typeof parsed !== 'object') return null;
      const { kind, id } = parsed as { kind?: unknown; id?: unknown };
      if ((kind === 'group' || kind === 'type') && typeof id === 'number' && Number.isInteger(id)) {
        return { kind, id };
      }
      return null;
    } catch {
      return null;
    }
  }

  function persistSelected(node: GroupTreeNodeRef | null): void {
    if (typeof window === 'undefined') return;
    try {
      if (node === null) localStorage.removeItem(SELECTED_STORAGE_KEY);
      else localStorage.setItem(SELECTED_STORAGE_KEY, JSON.stringify(node));
    } catch {
      // Хранилище недоступно — выбор просто не переживёт навигацию.
    }
  }

  function readPersistedTab(): string {
    if (typeof window === 'undefined') return 'contents';
    try {
      const raw = localStorage.getItem(ACTIVE_TAB_STORAGE_KEY);
      return raw !== null && VALID_TABS.includes(raw) ? raw : 'contents';
    } catch {
      return 'contents';
    }
  }

  function persistTab(key: string): void {
    if (typeof window === 'undefined') return;
    try {
      localStorage.setItem(ACTIVE_TAB_STORAGE_KEY, key);
    } catch {
      // Хранилище недоступно — вкладка просто не переживёт навигацию.
    }
  }

  // Читается один раз при монтировании (страница создаётся заново при каждом
  // переходе на /groups). Хеш побеждает сохранённое.
  const hashTarget = parseTargetFromHash();
  const initialSelected: GroupTreeNodeRef | null = hashTarget ?? readPersistedSelected();

  const isAdmin = $derived(authStore.user?.role === 'admin');
  const canMutate = $derived(
    authStore.user?.role === 'admin' || authStore.user?.role === 'manager',
  );

  let selected = $state<GroupTreeNodeRef | null>(initialSelected);
  let activeTab = $state<string>(readPersistedTab());

  // Токены перезагрузки: дерево отдельно от правых панелей.
  let refreshToken = $state(0);
  let panelRefreshToken = $state(0);

  // Не $state: монотонный счётчик для token'ов focusRequest.
  let focusCounter = 0;
  let focusRequest = $state<(GroupTreeNodeRef & { token: number }) | null>(
    hashTarget ? { ...hashTarget, token: ++focusCounter } : null,
  );

  // Данные дерева, нужные обработчикам (имя/версия типа для переименования и
  // удаления). Только читаются в обработчиках, шаблон их не рисует — не $state.
  let loadedTypes: GroupTypeDto[] = [];

  type TypeModal = { mode: 'create' } | { mode: 'rename'; type: GroupTypeDto };
  type GroupModal =
    | { mode: 'create'; presetTypeId: number | null }
    | { mode: 'rename'; group: GroupDto };
  type DeleteTarget = {
    kind: 'group' | 'type';
    id: number;
    name: string;
    directDeviceCount: number;
  };

  let typeModal = $state<TypeModal | null>(null);
  let groupModal = $state<GroupModal | null>(null);
  let moveGroup = $state<GroupDto | null>(null);
  let deleteTarget = $state<DeleteTarget | null>(null);

  function errMessage(e: unknown, fallback: string): string {
    return (e as Partial<AppError> | undefined)?.message ?? fallback;
  }

  function refreshTree(): void {
    refreshToken += 1;
  }

  function refreshAll(): void {
    refreshToken += 1;
    panelRefreshToken += 1;
  }

  // Фаза 41.7 (D-17): панели справа перечитываются по событию другого клиента.
  // Читаем ТОЛЬКО reloadSeq; запись токена и lastReloadSeq — в untrack
  // (иначе effect_update_depth_exceeded, который не ловят svelte-check/eslint/build).
  // Не $state: нужен лишь для сравнения, первый запуск эффекта токен не поднимает.
  let lastReloadSeq = placeContentEventsStore.reloadSeq;
  $effect(() => {
    const seq = placeContentEventsStore.reloadSeq;
    untrack(() => {
      if (seq !== lastReloadSeq) {
        lastReloadSeq = seq;
        panelRefreshToken += 1;
      }
    });
  });

  function focusNode(node: GroupTreeNodeRef): void {
    focusRequest = { ...node, token: ++focusCounter };
  }

  // Переход-фокус по хешу, когда страница УЖЕ открыта (ссылка вставлена в адрес,
  // кнопка «назад»): обычный вход читает хеш при монтировании выше.
  $effect(() => {
    if (typeof window === 'undefined') return;
    const onHashChange = () => {
      const target = parseTargetFromHash();
      if (target) untrack(() => focusNode(target));
    };
    window.addEventListener('hashchange', onHashChange);
    return () => window.removeEventListener('hashchange', onHashChange);
  });

  // --- Выбор ---------------------------------------------------------------------
  function handleSelect(node: GroupTreeNodeRef | null): void {
    selected = node;
    const newHash = node
      ? node.kind === 'group'
        ? `#/groups?id=${node.id}`
        : `#/groups?type=${node.id}`
      : '#/groups';
    if (window.location.hash !== newHash) {
      window.history.replaceState(null, '', newHash);
    }
    persistSelected(node);
  }

  function handleTabChange(key: string): void {
    activeTab = key;
    persistTab(key);
  }

  function handleTreeLoaded(types: GroupTypeDto[]): void {
    loadedTypes = types;
  }

  // --- Создание / переименование -------------------------------------------------
  async function renameNode(node: GroupTreeNodeRef): Promise<void> {
    if (node.kind === 'type') {
      const type = loadedTypes.find((t) => t.id === node.id);
      if (type) typeModal = { mode: 'rename', type };
      return;
    }
    await renameGroup(node.id);
  }

  async function renameGroup(groupId: number): Promise<void> {
    try {
      groupModal = { mode: 'rename', group: await groupsApi.get(groupId) };
    } catch (e) {
      pushToast('error', errMessage(e, 'Не удалось загрузить группу.'));
    }
  }

  // --- Перенос: ОДИН обработчик для двух точек входа (D-18) -----------------------
  async function openMove(groupId: number): Promise<void> {
    try {
      moveGroup = await groupsApi.get(groupId);
    } catch (e) {
      pushToast('error', errMessage(e, 'Не удалось загрузить группу.'));
    }
  }

  // Вывод вложенной группы из состава меняет её место (оно снова своё) — счётчики
  // «Мест» должны узнать о смене в ТОЙ ЖЕ функции (реестровый гейт INV-7).
  async function removeFromParent(groupId: number): Promise<void> {
    try {
      const g = await groupsApi.get(groupId);
      const result = await groupsApi.setParent({
        id: g.id,
        version: g.version,
        parent_group_id: null,
      });
      notifyPlaceContentChanged(result.changed_place_ids);
      refreshAll();
    } catch (e) {
      pushToast('error', errMessage(e, 'Не удалось вывести группу из состава.'));
    }
  }

  // --- Удаление ------------------------------------------------------------------
  async function requestDelete(node: GroupTreeNodeRef): Promise<void> {
    if (node.kind === 'type') {
      const type = loadedTypes.find((t) => t.id === node.id);
      if (type) {
        deleteTarget = { kind: 'type', id: type.id, name: type.name, directDeviceCount: 0 };
      }
      return;
    }
    try {
      const g = await groupsApi.get(node.id);
      deleteTarget = {
        kind: 'group',
        id: g.id,
        name: g.name,
        directDeviceCount: g.direct_device_count,
      };
    } catch (e) {
      pushToast('error', errMessage(e, 'Не удалось загрузить группу.'));
    }
  }

  function handleDeleted(): void {
    const gone = deleteTarget;
    deleteTarget = null;
    if (gone && selected && selected.kind === gone.kind && selected.id === gone.id) {
      handleSelect(null);
    }
    refreshAll();
  }

  // --- Переходы ------------------------------------------------------------------
  // Из карточки состава клик по устройству ведёт в его раздел: принтер — в
  // «Принтеры», остальное — в «Устройства».
  async function openDevice(deviceId: number): Promise<void> {
    try {
      const d = await devices.get(deviceId);
      const section = d.type_id === PRINTER_TYPE_ID ? '#/printers' : '#/devices';
      await push(`${section}?id=${deviceId}`);
    } catch (e) {
      pushToast('error', errMessage(e, 'Не удалось открыть устройство.'));
    }
  }
</script>

<div class="groups-page">
  <PageHeader title="Группы">
    {#snippet actions()}
      {#if canMutate}
        <Button
          variant="primary"
          onclick={() => (groupModal = { mode: 'create', presetTypeId: null })}
        >
          Создать группу
        </Button>
      {/if}
    {/snippet}
  </PageHeader>

  <div class="page-content">
    <PlacesMasterDetail>
      {#snippet master()}
        <GroupTree
          {initialSelected}
          {refreshToken}
          {focusRequest}
          canEditTypes={isAdmin}
          onSelect={handleSelect}
          onCreateType={() => (typeModal = { mode: 'create' })}
          onCreateGroup={(typeId) => (groupModal = { mode: 'create', presetTypeId: typeId })}
          onRename={renameNode}
          onMove={openMove}
          onRemoveFromParent={removeFromParent}
          onDelete={requestDelete}
          onLoaded={handleTreeLoaded}
        />
      {/snippet}
      {#snippet detail()}
        {#if selected && selected.kind === 'type'}
          {#key `type:${selected.id}`}
            <GroupTypePanel
              typeId={selected.id}
              canEdit={isAdmin}
              refreshToken={panelRefreshToken}
              onNavigateToGroup={(id) => focusNode({ kind: 'group', id })}
              onChanged={refreshTree}
            />
          {/key}
        {:else if selected && selected.kind === 'group'}
          {#key `group:${selected.id}`}
            <GroupPanel
              groupId={selected.id}
              canEdit={canMutate}
              refreshToken={panelRefreshToken}
              {activeTab}
              onTabChange={handleTabChange}
              onNavigateToGroup={(id) => focusNode({ kind: 'group', id })}
              onOpenType={(typeId) => focusNode({ kind: 'type', id: typeId })}
              onMoveRequest={openMove}
              onRenameRequest={renameGroup}
              onOpenDevice={openDevice}
              onChanged={refreshTree}
            />
          {/key}
        {:else}
          <DetailPanel
            empty={true}
            emptyTitle="Ничего не выбрано"
            emptyBody="Выберите тип или группу в дереве слева."
          />
        {/if}
      {/snippet}
    </PlacesMasterDetail>
  </div>
</div>

{#if typeModal}
  <GroupTypeFormModal
    mode={typeModal.mode}
    type={typeModal.mode === 'rename' ? typeModal.type : null}
    onClose={() => (typeModal = null)}
    onSaved={() => {
      typeModal = null;
      refreshAll();
    }}
  />
{/if}

{#if groupModal}
  <GroupFormModal
    mode={groupModal.mode}
    group={groupModal.mode === 'rename' ? groupModal.group : null}
    presetTypeId={groupModal.mode === 'create' ? groupModal.presetTypeId : null}
    onClose={() => (groupModal = null)}
    onSaved={(saved) => {
      const created = groupModal?.mode === 'create';
      groupModal = null;
      refreshAll();
      if (created) focusNode({ kind: 'group', id: saved.id });
    }}
  />
{/if}

{#if moveGroup}
  <GroupMoveModal
    group={moveGroup}
    onClose={() => (moveGroup = null)}
    onMoved={() => {
      moveGroup = null;
      refreshAll();
    }}
  />
{/if}

{#if deleteTarget}
  <GroupDeleteModal
    kind={deleteTarget.kind}
    id={deleteTarget.id}
    name={deleteTarget.name}
    directDeviceCount={deleteTarget.directDeviceCount}
    onClose={() => (deleteTarget = null)}
    onDeleted={handleDeleted}
  />
{/if}

<style lang="scss">
  // Оболочка `.content` не скроллится (overflow: hidden): страница занимает всю
  // высоту, а скроллят только внутренние регионы панелей.
  .groups-page {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }

  .page-content {
    flex: 1 1 auto;
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: hidden;
    padding: var(--tr-space-lg) var(--tr-space-xl);
  }
</style>
