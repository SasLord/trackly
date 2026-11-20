<script lang="ts">
  // Раздел «Устройства» сотрудника (41.6, EMP-01..03, UI-SPEC §7, §14.2, §15, §16, §19).
  // Страница читает список одной командой employee_devices_list, порядок строк задаёт
  // сервер (D-06, D-20): клиент не сортирует и не фильтрует. Единственные экземпляры
  // карточки и формы заявки стоят здесь, вне строки и вне <td>.
  import { onMount } from 'svelte';
  import { connectWs, onWsEvent } from '$lib/api/ws';
  import { pushToast } from '$lib/stores/toast.svelte';
  import PageHeader from '$lib/components/PageHeader.svelte';
  import Table from '$lib/components/Table.svelte';
  import Button from '$lib/components/Button.svelte';
  import RequestFormModal from '../requests/RequestFormModal.svelte';
  import { requestTargetHref } from '../requests/requestTarget';
  import EmployeeDeviceRow from './EmployeeDeviceRow.svelte';
  import EmployeeDeviceTile from './EmployeeDeviceTile.svelte';
  import EmployeeDeviceViewModal from './EmployeeDeviceViewModal.svelte';
  import { employeeDevices } from './api';
  import type { EmployeeDeviceDto } from '../../bindings';
  import type { WsEvent } from '../../bindings-phase6';

  // Дебаунс живого перечитывания (D-19).
  const REFRESH_DEBOUNCE_MS = 300;

  let items = $state<EmployeeDeviceDto[]>([]);
  let hasGroups = $state(false);
  let loading = $state(true);
  let loadFailed = $state(false);
  let narrow = $state(false);

  let viewId = $state<number | null>(null);
  let formOpen = $state(false);
  let formPrefill = $state<{ type: 'repair' | 'cartridge_replace'; deviceId: number } | undefined>(
    undefined,
  );

  const viewRow = $derived(viewId === null ? null : (items.find((r) => r.id === viewId) ?? null));
  const emptyTitle = $derived(
    hasGroups ? 'В ваших рабочих местах нет устройств' : 'Нет привязанных рабочих мест',
  );
  const emptyBody = $derived(
    hasGroups
      ? 'Если здесь должно быть устройство, обратитесь к администратору.'
      : 'Вы пока не привязаны ни к одному АРМ. Обратитесь к администратору, чтобы он добавил вас в рабочее место.',
  );
  const isEmpty = $derived(!loading && !loadFailed && items.length === 0);

  // Счётчик поколений: устаревший ответ не затирает более свежий.
  let generation = 0;
  let timer: ReturnType<typeof setTimeout> | undefined;

  async function load(background: boolean) {
    const gen = ++generation;
    if (!background) {
      loading = true;
      loadFailed = false;
    }
    try {
      const res = await employeeDevices.list();
      if (gen !== generation) return;
      items = res.items;
      hasGroups = res.hasGroups;
      loadFailed = false;
      // Карточка открыта, а устройства в списке больше нет (D-15, UI-SPEC §13.4).
      if (viewId !== null && !res.items.some((r) => r.id === viewId)) {
        viewId = null;
        pushToast('info', 'Устройство больше не относится к вашим рабочим местам');
      }
    } catch {
      if (gen !== generation) return;
      // Фоновая ошибка — без тоста, прежние данные остаются; первичная — блок с «Повторить».
      if (!background) loadFailed = true;
    } finally {
      if (gen === generation && !background) loading = false;
    }
  }

  function scheduleRefresh() {
    if (timer !== undefined) clearTimeout(timer);
    timer = setTimeout(() => {
      timer = undefined;
      void load(true);
    }, REFRESH_DEBOUNCE_MS);
  }

  function handleWsEvent(event: WsEvent) {
    // Сигнал employee_scope_changed приходит без данных: читаем своей командой.
    if (
      event.type === 'employee_scope_changed' ||
      event.type === 'request_status_changed' ||
      event.type === 'chat_message'
    ) {
      scheduleRefresh();
    }
  }

  onMount(() => {
    void load(false);

    let unlisten: (() => void) | undefined;
    let disposed = false;
    connectWs()
      .then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      })
      .catch(() => {
        // Сокет не критичен: список остаётся читаемым.
      });
    const unsubscribe = onWsEvent(handleWsEvent);

    const mql = window.matchMedia('(max-width: 767px)');
    narrow = mql.matches;
    const onMql = (e: MediaQueryListEvent) => {
      narrow = e.matches;
    };
    mql.addEventListener('change', onMql);

    return () => {
      disposed = true;
      unsubscribe();
      unlisten?.();
      mql.removeEventListener('change', onMql);
      if (timer !== undefined) clearTimeout(timer);
      generation++;
    };
  });

  function openRow(row: EmployeeDeviceDto) {
    viewId = row.id;
  }

  function openForm(type: 'repair' | 'cartridge_replace', row: EmployeeDeviceDto) {
    // Модалки не стопкой: карточка закрывается, форма открывается с предзаполнением (D-12).
    viewId = null;
    formPrefill = { type, deviceId: row.id };
    formOpen = true;
  }

  function gotoRequest(id: number) {
    viewId = null;
    location.hash = requestTargetHref(id, 'info');
  }

  async function copyInventory(row: EmployeeDeviceDto) {
    const value = row.inventoryNumber ?? '';
    try {
      await navigator.clipboard.writeText(value);
      pushToast('success', 'Инвентарный номер скопирован');
    } catch {
      pushToast(
        'error',
        'Не удалось скопировать. Откройте карточку устройства и скопируйте номер вручную.',
      );
    }
  }

  function created(req: { id: number; status: string; joined?: boolean }) {
    formOpen = false;
    pushToast(
      'success',
      req.joined ? `Вы присоединились к заявке №${req.id}` : `Заявка №${req.id} создана`,
      { link: { label: 'Открыть заявку', href: requestTargetHref(req.id, 'info') } },
    );
    // Метка заявки появляется после перечитывания, без скелета (D-13).
    void load(true);
  }
</script>

<div class="employee-devices-page">
  <PageHeader title="Устройства" />
  <div class="page-content">
    {#if loadFailed}
      <div class="load-error" role="alert">
        <p class="load-error-title">Не удалось загрузить устройства</p>
        <p class="load-error-body">Проверьте подключение к серверу и повторите попытку.</p>
        <Button variant="secondary" onclick={() => void load(false)}>Повторить</Button>
      </div>
    {:else if narrow}
      {#if loading}
        <ul class="tiles" aria-hidden="true">
          {#each [1, 2, 3, 4] as n (n)}<li class="tile-skeleton"></li>{/each}
        </ul>
      {:else if isEmpty}
        <div class="tiles-empty">
          <p class="empty-title">{emptyTitle}</p>
          <p class="empty-body">{emptyBody}</p>
        </div>
      {:else}
        <ul class="tiles" aria-label="Мои устройства">
          {#each items as row (row.id)}
            <li>
              <EmployeeDeviceTile
                {row}
                onOpen={openRow}
                onRepair={(r) => openForm('repair', r)}
                onCartridge={(r) => openForm('cartridge_replace', r)}
                onGoto={gotoRequest}
                onCopy={copyInventory}
              />
            </li>
          {/each}
        </ul>
        <p class="tiles-footer">Устройств: {items.length}</p>
      {/if}
    {:else}
      <Table
        columns={9}
        {loading}
        empty={isEmpty}
        {emptyTitle}
        {emptyBody}
        skeletonRows={6}
        head={tableHead}
        {footer}
        framed
        fillHeight
        minWidth="1280px"
      >
        {#each items as row (row.id)}
          <EmployeeDeviceRow
            {row}
            onOpen={openRow}
            onRepair={(r) => openForm('repair', r)}
            onCartridge={(r) => openForm('cartridge_replace', r)}
            onGoto={gotoRequest}
            onCopy={copyInventory}
          />
        {/each}
      </Table>
    {/if}
  </div>
</div>

{#snippet tableHead()}
  <th class="th-name">Наименование</th>
  <th class="th-type">Инв. №</th>
  <th class="th-serial">Серийный №</th>
  <th class="th-model">Модель</th>
  <th class="th-place">Место</th>
  <th class="th-place">АРМ</th>
  <th class="th-status">Статус</th>
  <th class="th-toner">Тонер</th>
  <th class="th-actions" aria-label="Действия"></th>
{/snippet}

{#snippet footer()}
  <span class="list-footer">Устройств: {items.length}</span>
{/snippet}

<!-- Один экземпляр карточки и формы на странице, вне строки и вне <td> -->
<EmployeeDeviceViewModal
  open={viewRow !== null}
  row={viewRow}
  onClose={() => (viewId = null)}
  onRepair={(r) => openForm('repair', r)}
  onCartridge={(r) => openForm('cartridge_replace', r)}
  onGotoRequest={gotoRequest}
/>

<RequestFormModal
  open={formOpen}
  prefill={formPrefill}
  onClose={() => (formOpen = false)}
  onSuccess={created}
/>

<style lang="scss">
  .employee-devices-page {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }

  .page-content {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    padding: var(--tr-space-xl) var(--tr-space-2xl);

    @media (max-width: 767px) {
      padding: var(--tr-space-md);
    }
  }

  .th-name {
    min-width: 240px;
  }
  .th-type {
    width: 112px;
  }
  .th-serial {
    width: 128px;
  }
  .th-model {
    width: 144px;
  }
  .th-place {
    width: 176px;
  }
  .th-status {
    width: 128px;
  }
  .th-toner {
    width: 80px;
  }
  .th-actions {
    width: 40px;
  }

  .list-footer,
  .tiles-footer {
    font-size: var(--tr-font-size-caption);
    color: var(--tr-text-secondary);
  }

  .tiles-footer {
    margin: 0;
    padding-top: var(--tr-space-xs);
  }

  .tiles {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-xs);
    overflow-y: auto;
    flex: 1;
    min-height: 0;
  }

  .tile-skeleton {
    height: 112px;
    flex-shrink: 0;
    background: var(--tr-surface-sunken);
    border-radius: var(--tr-radius-md);
  }

  .tiles-empty,
  .load-error {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: var(--tr-space-xs);
    padding: var(--tr-space-2xl) var(--tr-space-md);
    font-size: var(--tr-font-size-body);
  }

  .empty-title,
  .load-error-title {
    margin: 0;
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-primary);
  }

  .empty-body,
  .load-error-body {
    margin: 0;
    color: var(--tr-text-secondary);
  }

  .load-error {
    gap: var(--tr-space-sm);
  }
</style>
