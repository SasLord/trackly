<script lang="ts">
  // Plan 06-05 Task 2: карточка заявки с lifecycle кнопками + история (REQ-07).
  // REQ-05: «Установить картридж» → OperationModal с preFillPrinterId.
  // По паттерну CartridgeDetail.svelte.
  // Plan 28-02 Task 1 (D-01): rebuilt on the shared DetailPanel/DetailSection/
  // DetailField primitives (extracted in 27-01), per PrinterDetail.svelte
  // precedent (not ActDetail) for the header — the title carries two badges
  // (тип+статус) plus a meta-row (автор/дата), which does not fit a plain
  // string `title` prop. panelTitle = typeLabel; title-row (badges) +
  // meta-row are rendered verbatim as the first content inside DetailPanel's
  // children, matching PrinterDetail's title-badges precedent. Lifecycle
  // buttons without an attached inline field moved into the actions snippet;
  // «Выполнить» stays with its completeNotes Textarea in the body (inline
  // mini-form, unchanged). The 4 confirm-Modal blocks are untouched (D-04
  // territory, not D-01).
  import { untrack } from 'svelte';
  import Button from '$lib/components/Button.svelte';
  import Badge from '$lib/components/Badge.svelte';
  import Spinner from '$lib/components/Spinner.svelte';
  import Modal from '$lib/components/Modal.svelte';
  import Textarea from '$lib/components/Textarea.svelte';
  // Plan 28-11 (GAP-1): Select (нативный <select>) заменён на кастомный
  // Dropdown (flat + variant="select") — см. CartridgeFormBody.svelte
  // (Phase 27-G1) для канонического паттерна.
  import Dropdown from '$lib/components/Dropdown.svelte';
  import DetailPanel from '$lib/components/DetailPanel.svelte';
  import DetailSection from '$lib/components/DetailSection.svelte';
  import DetailField from '$lib/components/DetailField.svelte';
  import OperationModal from '../cartridges/OperationModal.svelte';
  import { notifyPlaceContentChanged } from '$lib/stores/placeContentEvents.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { apiCall } from '$lib/api/client';
  import { requests, repairs } from './api';
  import RepairDeviceStatus from './RepairDeviceStatus.svelte';
  import { openRepairRequest } from './repairLocks';
  import RepairMoveModal from './RepairMoveModal.svelte';
  import RepairSendModal from './RepairSendModal.svelte';
  import RepairReturnModal from './RepairReturnModal.svelte';
  import RepairCompleteModal from './RepairCompleteModal.svelte';
  import RepairAssignDeviceModal from './RepairAssignDeviceModal.svelte';
  import RepairVisitsSection from './RepairVisitsSection.svelte';
  import { parseAppError } from '$lib/api/errors';
  import AttachmentsBlock from '../attachments/AttachmentsBlock.svelte';
  import AttachmentHistory from '../attachments/AttachmentHistory.svelte';
  import { attachments } from '../attachments/api';
  import { onWsEvent } from '$lib/api/ws';
  import type {
    AttachmentHistoryEventDto,
    RepairStateDto,
    RepairTransitionResultDto,
  } from '../../bindings';
  import type { RequestDto } from '../../bindings-phase6';
  import type { CurrentUser } from '$lib/stores/auth.svelte';
  import type { RequestHistoryEntry } from './api';
  import type { AdSettingsDto, ApproveAdRegisterDto } from '../../bindings-phase9';
  import type { CartridgeDto } from '../../bindings';

  interface Props {
    request: RequestDto | null;
    loading: boolean;
    identity: CurrentUser | null;
    onTransition: () => void;
    /** D-35: открыть другую заявку на странице заявок (ставит выбранную напрямую). */
    onOpenRequest?: (id: number) => void;
  }

  const { request, loading, identity, onTransition, onOpenRequest }: Props = $props();

  // Вне страницы заявок колбэка нет — переходим по ссылке. На самой странице
  // push того же #/requests?id=N при совпадающем URL эффект не перезапускает.
  function openRequest(id: number) {
    if (onOpenRequest) onOpenRequest(id);
    else openRepairRequest(id);
  }

  // --- Local state ---
  let operationModalOpen = $state(false);
  let rejectModalOpen = $state(false);
  let rejectNotes = $state('');
  let rejectSubmitting = $state(false);
  let completeNotes = $state('');
  let completeSubmitting = $state(false);
  let historyEntries = $state<RequestHistoryEntry[]>([]);
  let historyLoading = $state(false);
  let attachmentEvents = $state<AttachmentHistoryEventDto[]>([]);
  let attachmentsBlock = $state<ReturnType<typeof AttachmentsBlock> | null>(null);
  let attachmentsEl = $state<HTMLElement | null>(null);

  // Фаза 41.4: ремонтная заявка. Состояние ремонта (RepairStateDto) грузится при
  // открытии, после каждого действия и по WS; попапы цикла открываются флагами.
  let repairState = $state<RepairStateDto | null>(null);
  let repairLoadSeq = 0;
  let repairBusy = $state(false);
  let moveModalOpen = $state(false);
  let moveMode = $state<'accept' | 'toStorage'>('accept');
  let sendModalOpen = $state(false);
  let returnModalOpen = $state(false);
  let completeModalOpen = $state(false);
  let assignModalOpen = $state(false);
  let cancelRepairModalOpen = $state(false);
  let cancelRepairSubmitting = $state(false);

  // ad_register approval modal state (Screen 5b, D-REG-02: role defaults to employee).
  let approveModalOpen = $state(false);
  let approveRole = $state('employee');
  let approveSubmitting = $state(false);

  // Plan 28-11 (GAP-1): опции для Dropdown (flat + variant="select") — «Роль».
  const ROLE_OPTIONS = [
    { id: 'employee', label: 'Сотрудник' },
    { id: 'manager', label: 'Специалист' },
    { id: 'admin', label: 'Администратор' },
  ];
  // Плоские опции без drill-in — onExpandGroup никогда реально не вызывается
  // (isGroupExpandable всегда false), но Dropdown требует типизированную
  // функцию, чтобы вывести TMember (иначе `() => []` выводит `never[]`).
  function noExpandRole(): { id: string; label: string }[] {
    return [];
  }
  const approveRoleLabel = $derived(ROLE_OPTIONS.find((o) => o.id === approveRole)?.label ?? '');

  // GAP-12-07/A4: delete (Admin/Manager, any status) and self-cancel
  // (Employee author, open status only) lifecycle actions.
  let deleteModalOpen = $state(false);
  let deleteSubmitting = $state(false);
  let cancelModalOpen = $state(false);
  let cancelSubmitting = $state(false);

  // Current AD registration mode — only used to pick the correct reject
  // confirmation copy (auto-accept implies the user already has access and
  // reject must soft-delete; pending implies reject simply discards). The
  // backend is authoritative regardless of what this flag shows.
  let adAutoAccept = $state(false);

  // Derived visibility — specialist/admin maps to manager/admin in the actual UserRole type.
  const isSpecialist = $derived(identity?.role === 'admin' || identity?.role === 'manager');

  // ad_register requests are admin-only (REQ-06, T-09-21) — manager cannot act on them.
  const isAdmin = $derived(identity?.role === 'admin');
  const isAdRegister = $derived(request?.requestType === 'ad_register');
  const isAdRestore = $derived(isAdRegister && request?.adSubtype === 'restore');

  // GAP-12-07/A4: UI-level ownership check (cosmetic only — server-side
  // BOLA-guard in RequestService::cancel is authoritative, T-12-15-01).
  const isOwnRequest = $derived(
    identity !== null && request !== null && identity.id === request.requestedByUserId,
  );

  const isRepair = $derived(request?.requestType === 'repair');
  const hasDevice = $derived(
    request?.printerDeviceId !== null && request?.printerDeviceId !== undefined,
  );
  // Состояние чужой заявки (прежняя карточка) не показываем.
  const curState = $derived(
    repairState !== null && request !== null && repairState.requestId === request.id
      ? repairState
      : null,
  );
  const deviceStatusCode = $derived(curState?.deviceStatusCode ?? null);
  // Название устройства для карточки: имя, инвентарный номер, тип.
  const repairDeviceLine = $derived.by(() => {
    if (!hasDevice) return 'Не указано';
    const name = curState?.deviceName ?? request?.printerName ?? null;
    if (name === null) return '—';
    const extra = [
      curState?.deviceInventoryNo ? `инв. № ${curState.deviceInventoryNo}` : null,
      curState?.deviceTypeName ?? null,
    ].filter((x): x is string => x !== null);
    return extra.length > 0 ? `${name} (${extra.join(', ')})` : name;
  });
  // Ничего не перемещалось и в сервис не ездило: доступны «Забрать на склад»,
  // «Отклонить» и отмена автором (D-11).
  const repairFresh = $derived(
    curState !== null && !curState.moved && curState.visits.length === 0,
  );
  const canRejectRepair = $derived(
    request !== null &&
      (request.status === 'open' || (request.status === 'in_progress' && repairFresh)),
  );

  async function loadRepairState(id: number) {
    const seq = ++repairLoadSeq;
    try {
      const st = await repairs.getState(id);
      if (seq !== repairLoadSeq || request?.id !== id) return;
      repairState = st;
    } catch {
      // Не критично: кнопки цикла остаются недоступны, пока состояние не получено.
    }
  }

  // Читает только id/тип/версию заявки; всё остальное — через untrack, запись
  // repairState сама эффект не перезапускает.
  $effect(() => {
    const id = request?.id;
    const type = request?.requestType;
    void request?.version;
    untrack(() => {
      if (type !== 'repair' || id === undefined) {
        repairLoadSeq++;
        repairState = null;
        return;
      }
      if (repairState !== null && repairState.requestId !== id) repairState = null;
      void loadRepairState(id);
    });
  });

  function applyRepairResult(r: RepairTransitionResultDto) {
    // Более ранний запрос состояния не должен затереть свежий итог действия.
    repairLoadSeq++;
    repairState = r.state;
    onTransition();
  }

  function repairErrorMessage(e: unknown): string {
    return parseAppError(e).message;
  }

  async function startAccept() {
    if (!isSpecialist) return;
    if (!request || repairBusy) return;
    if (hasDevice) {
      if (curState === null) return;
      moveMode = 'accept';
      moveModalOpen = true;
      return;
    }
    // D-24: «Другое» без устройства — принять без попапа перемещения.
    repairBusy = true;
    try {
      const r = await repairs.accept(request.id, request.version, { kind: 'stay' });
      pushToast('success', 'Заявка принята в работу');
      applyRepairResult(r);
    } catch (e: unknown) {
      pushToast('error', repairErrorMessage(e));
    } finally {
      repairBusy = false;
    }
  }

  function openMove(mode: 'accept' | 'toStorage') {
    moveMode = mode;
    moveModalOpen = true;
  }

  async function handleCancelRepairConfirm() {
    if (!isSpecialist) return;
    if (!request || cancelRepairSubmitting) return;
    cancelRepairSubmitting = true;
    try {
      const r = await repairs.cancelRepair({ requestId: request.id, version: request.version });
      pushToast('success', 'Ремонт отменён, устройство возвращено');
      cancelRepairModalOpen = false;
      applyRepairResult(r);
    } catch {
      pushToast('error', 'Не удалось отменить ремонт. Попробуйте ещё раз.');
    } finally {
      cancelRepairSubmitting = false;
    }
  }

  const CANCEL_REPAIR_HINT = 'Сначала заберите устройство из ремонта';

  $effect(() => {
    if (!isSpecialist || !isAdRegister) return;
    apiCall<AdSettingsDto>('settings_get_ad', {})
      .then((s) => {
        adAutoAccept = s.auto_accept;
      })
      .catch(() => {
        // Non-fatal — falls back to pending-mode copy.
      });
  });

  type BadgeVariant = 'success' | 'accent' | 'warning' | 'default';

  const statusVariant = $derived<BadgeVariant>(
    !request
      ? 'default'
      : request.status === 'open'
        ? 'accent'
        : request.status === 'in_progress' || request.status === 'in_repair'
          ? 'warning'
          : request.status === 'completed'
            ? 'success'
            : 'default',
  );

  const statusLabel = $derived(
    !request
      ? ''
      : request.status === 'open'
        ? 'Создана'
        : request.status === 'in_progress'
          ? 'В работе'
          : request.status === 'in_repair'
            ? 'В ремонте'
            : request.status === 'completed'
              ? 'Выполнена'
              : request.status === 'cancelled'
                ? 'Отменена'
                : 'Отклонена',
  );

  const typeLabel = $derived(
    !request
      ? ''
      : request.requestType === 'ad_register'
        ? 'Регистрация AD'
        : request.requestType === 'cartridge_replace'
          ? 'Замена картриджа'
          : request.requestType === 'repair'
            ? 'Ремонт'
            : 'Свободная форма',
  );

  // D-01: DetailPanel's header title is a plain string — the two-badge +
  // meta-row header lives as bespoke content inside children (see template).
  const panelTitle = $derived<string | undefined>(request ? typeLabel : undefined);

  // Load history when request changes. D-14: события вложений берём из
  // attachments.history (у них есть attachment_id, имя, состояние), а сырые
  // строки custom:attachment_* стандартного списка отбрасываем на клиенте.
  async function loadHistory(id: number, silent: boolean) {
    if (!silent) historyLoading = true;
    try {
      const [entries, events] = await Promise.all([
        requests.getHistory(id),
        attachments.history('request', id).catch(() => [] as AttachmentHistoryEventDto[]),
      ]);
      if (request?.id !== id) return;
      historyEntries = entries.filter((e) => !e.action.startsWith('custom:attachment_'));
      attachmentEvents = events;
    } catch {
      if (request?.id !== id) return;
      historyEntries = [];
      attachmentEvents = [];
    } finally {
      if (request?.id === id) historyLoading = false;
    }
  }

  $effect(() => {
    const id = request?.id;
    if (id === undefined || id === null) {
      historyEntries = [];
      attachmentEvents = [];
      return;
    }
    void loadHistory(id, false);
  });

  // D-15: история перечитывается вместе с блоком.
  $effect(() => {
    const off = onWsEvent((ev) => {
      const id = request?.id;
      if (
        id !== undefined &&
        ev.type === 'attachments_changed' &&
        ev.parentKind === 'request' &&
        ev.parentId === id
      ) {
        void loadHistory(id, true);
      }
      // Перемещение/статус устройства изменились (в т.ч. другим пользователем).
      if (id !== undefined && ev.type === 'entities_changed' && request?.requestType === 'repair') {
        void loadRepairState(id);
        void loadHistory(id, true);
      }
    });
    return off;
  });

  type HistoryRow =
    | { kind: 'entry'; at: number; entry: RequestHistoryEntry }
    | { kind: 'attachment'; at: number; event: AttachmentHistoryEventDto };

  const mergedHistory = $derived<HistoryRow[]>(
    [
      ...historyEntries.map(
        (entry): HistoryRow => ({ kind: 'entry', at: entry.createdAtUtc, entry }),
      ),
      ...attachmentEvents.map(
        (event): HistoryRow => ({ kind: 'attachment', at: event.createdAtUtc, event }),
      ),
    ].sort((a, b) => a.at - b.at),
  );

  // Relative date helper
  function relativeDate(utcSeconds: number): string {
    const now = Math.floor(Date.now() / 1000);
    const diff = now - utcSeconds;
    if (diff < 60) return 'только что';
    if (diff < 3600) return `${Math.floor(diff / 60)} мин. назад`;
    if (diff < 86400) return `${Math.floor(diff / 3600)} ч. назад`;
    const d = new Date(utcSeconds * 1000);
    return `${String(d.getUTCDate()).padStart(2, '0')}.${String(d.getUTCMonth() + 1).padStart(2, '0')}.${d.getUTCFullYear()}`;
  }

  function formatFullDate(utcSeconds: number): string {
    const d = new Date(utcSeconds * 1000);
    return `${String(d.getUTCDate()).padStart(2, '0')}.${String(d.getUTCMonth() + 1).padStart(2, '0')}.${d.getUTCFullYear()} ${String(d.getUTCHours()).padStart(2, '0')}:${String(d.getUTCMinutes()).padStart(2, '0')}`;
  }

  function actionLabel(action: string): string {
    const labels: Record<string, string> = {
      create: 'Создана',
      accept: 'Принята в работу',
      complete: 'Выполнена',
      reject: 'Отклонена',
      cancel: 'Отменена',
      'custom:create': 'Создана',
      'custom:accept': 'Принята в работу',
      'custom:complete': 'Выполнена',
      'custom:reject': 'Отклонена',
      'custom:cancel': 'Отменена',
    };
    return labels[action] ?? action;
  }

  // D-35: id целевой заявки для записи «объединена с…»; иначе null.
  function mergedIntoId(e: RequestHistoryEntry): number | null {
    if (e.action.replace(/^custom:/, '') !== 'repair_merged_into') return null;
    return e.relatedRequestId ?? null;
  }

  // REP-07: строка истории без даты. События ремонта читаемо, место/сервис/описание
  // приходят в notes (пишутся сервером при записи события); сырые коды не показываем.
  function entryText(e: RequestHistoryEntry): string {
    const code = e.action.replace(/^custom:/, '');
    const note = e.notes?.trim() ? e.notes.trim() : null;
    const who = e.actorName ? `; ${e.actorName}` : '';
    const q = (v: string | null) => (v ? `«${v}»` : '');
    switch (code) {
      case 'repair_accept':
        return `Принята в работу${who}${note ? `; Устройство перемещено: ${q(note)}` : ''}`;
      case 'repair_move':
        return `Устройство перемещено${note ? `: ${q(note)}` : ''}${who}`;
      case 'repair_send':
        return `Устройство передано в сервис${note ? ` ${q(note)}` : ''}${who}`;
      case 'repair_receive':
        return `Устройство возвращено из сервиса${who}`;
      case 'repair_cancel':
        return `Ремонт отменён${who}`;
      case 'repair_join':
        return `Присоединился к заявке${who}${note ? `; ${note}` : ''}`;
      case 'repair_merge':
        return `К заявке присоединена другая заявка${who}${note ? `; ${note}` : ''}`;
      case 'repair_merged_into':
        return `Заявка объединена с другой${who}`;
      case 'repair_assign_device':
        return `Указано устройство${who}`;
      default:
        if (code.startsWith('repair_') && code !== 'repair_complete') {
          return `Изменение заявки${who}`;
        }
        return `${actionLabel(e.action)}${who}${note ? `; ${note}` : ''}`;
    }
  }

  // --- Lifecycle actions ---

  async function handleAccept() {
    if (!isSpecialist || !request) return;
    try {
      await requests.transition({
        op: 'accept',
        requestId: request.id,
        version: request.version,
        // Assignee is resolved server-side from the authenticated caller; do not
        // send a client id. In unlocked-desktop mode identity.id is the sentinel
        // 0 ("Рабочий стол"), which has no users row and broke the FK on accept.
        assignedToUserId: null,
      });
      pushToast('success', 'Заявка принята в работу');
      onTransition();
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось выполнить операцию. Повторите попытку.';
      pushToast('error', msg);
    }
  }

  async function handleComplete() {
    if (!isSpecialist || !request || completeSubmitting) return;
    completeSubmitting = true;
    try {
      await requests.transition({
        op: 'complete',
        requestId: request.id,
        version: request.version,
        notes: completeNotes.trim() || null,
        linkedCartridgeId: null,
      });
      pushToast('success', 'Заявка выполнена');
      completeNotes = '';
      onTransition();
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось выполнить операцию. Повторите попытку.';
      pushToast('error', msg);
    } finally {
      completeSubmitting = false;
    }
  }

  async function handleRejectConfirm() {
    if (!isSpecialist || !request || rejectSubmitting) return;
    rejectSubmitting = true;
    try {
      if (isRepair) {
        // Сервер отклоняет requests_transition для ремонта: свой путь со снятием статуса устройства.
        const r = await repairs.reject({
          requestId: request.id,
          version: request.version,
          notes: rejectNotes.trim() || null,
        });
        pushToast('success', 'Заявка отклонена');
        rejectModalOpen = false;
        rejectNotes = '';
        applyRepairResult(r);
        return;
      }
      await requests.transition({
        op: 'reject',
        requestId: request.id,
        version: request.version,
        notes: rejectNotes.trim() || null,
      });
      pushToast(
        'success',
        isAdRegister
          ? isAdRestore
            ? 'Доступ останется закрытым'
            : adAutoAccept
              ? 'Пользователь удалён'
              : 'Заявка отклонена'
          : 'Заявка отклонена',
      );
      rejectModalOpen = false;
      rejectNotes = '';
      onTransition();
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось выполнить операцию. Повторите попытку.';
      pushToast('error', msg);
    } finally {
      rejectSubmitting = false;
    }
  }

  // GAP-12-07/A4: delete request (Admin/Manager, any status).
  async function handleDeleteConfirm() {
    if (!isSpecialist || !request || deleteSubmitting) return;
    deleteSubmitting = true;
    try {
      await requests.delete(request.id, request.version);
      pushToast('success', 'Заявка удалена');
      deleteModalOpen = false;
      onTransition();
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось выполнить операцию. Повторите попытку.';
      pushToast('error', msg);
    } finally {
      deleteSubmitting = false;
    }
  }

  // GAP-12-07/A4: self-cancel own request (Employee author, open status only).
  async function handleCancelConfirm() {
    if (!request || cancelSubmitting) return;
    cancelSubmitting = true;
    try {
      if (isRepair) {
        const r = await repairs.cancelByAuthor({ requestId: request.id, version: request.version });
        pushToast('success', 'Заявка отменена');
        cancelModalOpen = false;
        applyRepairResult(r);
        return;
      }
      await requests.cancel(request.id, request.version);
      pushToast('success', 'Заявка отменена');
      cancelModalOpen = false;
      onTransition();
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось выполнить операцию. Повторите попытку.';
      pushToast('error', msg);
    } finally {
      cancelSubmitting = false;
    }
  }

  // --- ad_register approval (Screen 5b, D-REG-02/D-REG-03) ---

  function openApproveModal() {
    approveRole = 'employee';
    approveModalOpen = true;
  }

  async function handleApproveConfirm() {
    if (!isSpecialist || !request || approveSubmitting) return;
    approveSubmitting = true;
    try {
      const dto: ApproveAdRegisterDto = {
        requestId: request.id,
        version: request.version,
        role: approveRole,
      };
      await requests.approveAdRegister(dto);
      pushToast('success', isAdRestore ? 'Доступ восстановлен' : 'Пользователь подтверждён');
      approveModalOpen = false;
      onTransition();
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось выполнить операцию. Повторите попытку.';
      pushToast('error', msg);
    } finally {
      approveSubmitting = false;
    }
  }

  // Reject confirm-modal copy depends on ad_register subtype + mode (UI-SPEC
  // Destructive table). Backend enforces the actual behavior regardless.
  const rejectModalTitle = $derived(
    !isAdRegister
      ? 'Отклонить заявку?'
      : isAdRestore
        ? 'Отклонить восстановление?'
        : adAutoAccept
          ? 'Отклонить и удалить пользователя?'
          : 'Отклонить заявку?',
  );

  const rejectModalBody = $derived(
    !isAdRegister
      ? 'Заявка будет закрыта без выполнения. Укажите причину в комментарии.'
      : isAdRestore
        ? 'Доступ останется закрытым.'
        : adAutoAccept
          ? 'Пользователь уже создан автоматически. Отклонение удалит его учётную запись и закроет доступ.'
          : 'Заявка на регистрацию будет отклонена, пользователь не получит доступ.',
  );

  const rejectModalButtonLabel = $derived(
    isAdRegister && !isAdRestore && adAutoAccept ? 'Удалить пользователя' : 'Отклонить',
  );

  // REQ-05: «Установить картридж» handler — called when OperationModal succeeds.
  // We then complete the request, linking the installed cartridge (D-06).
  //
  // WR-03: OperationModal now `await`s this handler and only shows its own
  // "Операция выполнена успешно." toast if it resolves. We rethrow on
  // failure so that modal-level toast is suppressed — the user only sees
  // this handler's own (more specific) error toast, not a false-positive
  // success alongside it.
  //
  // WR-05 (40.1 round-2 gap closure, 2026-09-18): the server-side install
  // transition changes `cartridges.place_id` to the target printer's place —
  // same underlying CartridgeService transition CartridgesPage's
  // handleOperationSuccess already covers (WR-01) — so this handler must
  // invalidate the place-tree counters too, mirroring that pattern exactly:
  // old+new place, deduped, nulls dropped. Unlike CartridgesPage, this
  // screen never held the cartridge before the operation (`cartridge={null}`
  // is passed to OperationModal — the operator picks one from the
  // request-centric picker INSIDE the modal), so the old place cannot be
  // read from a locally-held DTO the way CartridgesPage reads
  // `operationModalCartridge?.place_id`. OperationModal now forwards it as
  // `previousPlaceId` (its own pre-transition snapshot of
  // `effectiveCartridge.place_id`) — see OperationModal.svelte's onSuccess
  // contract.
  async function handleInstallSuccess(cartridge: CartridgeDto, previousPlaceId?: number | null) {
    if (!isSpecialist || !request) return;
    const oldPlaceId = previousPlaceId ?? null;
    const newPlaceId = cartridge.place_id ?? null;
    const changedPlaceIds = Array.from(
      new Set([oldPlaceId, newPlaceId].filter((id): id is number => id !== null)),
    );
    if (changedPlaceIds.length > 0) notifyPlaceContentChanged(changedPlaceIds);

    operationModalOpen = false;
    const requestId = request.id;
    // Complete the request after cartridge install.
    try {
      // WR-04: re-read the current version immediately before completing —
      // installing the cartridge does not bump the request row, but a
      // concurrent transition between modal-open and now (another actor
      // accepts/rejects, or a WS-driven refresh has not yet propagated to
      // the `request` prop) would otherwise send a stale version and fail
      // with OptimisticLockMismatch after the cartridge is already
      // installed.
      const current = await requests.get(requestId);
      await requests.transition({
        op: 'complete',
        requestId,
        version: current.version,
        notes: null,
        linkedCartridgeId: cartridge.id,
      });
      pushToast('success', 'Заявка выполнена');
      onTransition();
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось завершить заявку. Проверьте вручную.';
      pushToast('error', msg);
      onTransition(); // Still refresh — cartridge was installed.
      throw e;
    }
  }
</script>

{#if loading}
  <div class="detail-loading" aria-live="polite">
    <Spinner size="md" />
    <span>Загрузка заявки…</span>
  </div>
{:else}
  <DetailPanel
    title={panelTitle}
    empty={request === null}
    emptyTitle="Выберите заявку"
    emptyBody="Выберите заявку слева, чтобы увидеть детали и историю."
  >
    {#snippet actions()}
      {#if request}
        {#if isAdRegister}
          {#if isAdmin && request.status === 'open'}
            <Button variant="primary" onclick={openApproveModal}>Подтвердить</Button>
            <Button
              variant="destructive"
              onclick={() => {
                rejectNotes = '';
                rejectModalOpen = true;
              }}
            >
              Отклонить
            </Button>
          {/if}
        {:else if isRepair}
          {#if isSpecialist}
            {#if request.status === 'open'}
              <Button
                variant="primary"
                loading={repairBusy}
                disabled={hasDevice && curState === null}
                onclick={startAccept}
              >
                Принять в работу
              </Button>
            {:else if request.status === 'in_progress'}
              <Button
                variant="primary"
                disabled={curState === null}
                onclick={() => (completeModalOpen = true)}
              >
                Завершить
              </Button>
              {#if hasDevice}
                <Button
                  variant="secondary"
                  disabled={curState === null}
                  onclick={() => (sendModalOpen = true)}
                >
                  Отправить в ремонт
                </Button>
                {#if repairFresh}
                  <Button variant="secondary" onclick={() => openMove('toStorage')}>
                    Забрать на склад
                  </Button>
                {/if}
              {:else}
                <Button variant="secondary" onclick={() => (assignModalOpen = true)}>
                  Указать устройство
                </Button>
              {/if}
              {#if curState?.canCancelRepair}
                <Button variant="destructive" onclick={() => (cancelRepairModalOpen = true)}>
                  Отменить ремонт
                </Button>
              {/if}
            {:else if request.status === 'in_repair'}
              <Button
                variant="primary"
                disabled={curState === null}
                onclick={() => (returnModalOpen = true)}
              >
                Забрать из ремонта
              </Button>
              <span class="hint-wrap" title={CANCEL_REPAIR_HINT}>
                <Button
                  variant="destructive"
                  disabled
                  title={CANCEL_REPAIR_HINT}
                  ariaDescribedby="repair-cancel-hint"
                >
                  Отменить ремонт
                </Button>
                <span id="repair-cancel-hint" class="sr-only">{CANCEL_REPAIR_HINT}</span>
              </span>
            {/if}
            {#if canRejectRepair}
              <Button
                variant="destructive"
                onclick={() => {
                  rejectNotes = '';
                  rejectModalOpen = true;
                }}
              >
                Отклонить
              </Button>
            {/if}
            {#if isOwnRequest && request.status === 'in_progress' && repairFresh}
              <Button variant="destructive" onclick={() => (cancelModalOpen = true)}>
                Отменить заявку
              </Button>
            {/if}
          {:else if isOwnRequest && (request.status === 'open' || (request.status === 'in_progress' && repairFresh))}
            <Button variant="destructive" onclick={() => (cancelModalOpen = true)}>
              Отменить заявку
            </Button>
          {/if}
        {:else if isSpecialist}
          {#if request.status === 'open'}
            <Button variant="primary" onclick={handleAccept}>Принять в работу</Button>
            <Button
              variant="destructive"
              onclick={() => {
                rejectNotes = '';
                rejectModalOpen = true;
              }}
            >
              Отклонить
            </Button>
          {:else if request.status === 'in_progress' && request.requestType === 'free_form'}
            <Button
              variant="destructive"
              onclick={() => {
                rejectNotes = '';
                rejectModalOpen = true;
              }}
            >
              Отклонить
            </Button>
          {:else if request.status === 'in_progress' && request.requestType === 'cartridge_replace'}
            <Button variant="primary" onclick={() => (operationModalOpen = true)}>
              Установить картридж
            </Button>
            <Button
              variant="destructive"
              onclick={() => {
                rejectNotes = '';
                rejectModalOpen = true;
              }}
            >
              Отклонить
            </Button>
          {/if}
        {:else if isOwnRequest && request.status === 'open'}
          <Button variant="destructive" onclick={() => (cancelModalOpen = true)}>
            Отменить заявку
          </Button>
        {/if}
        {#if isSpecialist && (!isAdRegister || isAdmin)}
          <Button variant="destructive" onclick={() => (deleteModalOpen = true)}>Удалить</Button>
        {/if}
      {/if}
    {/snippet}

    {#if request}
      <!-- Header (D-01): title-row (2 Badge) + meta-row, verbatim — PrinterDetail
           title-badges precedent, does not fit DetailPanel's plain-string title. -->
      <div class="title-row">
        <Badge variant="default">{typeLabel}</Badge>
        <Badge variant={statusVariant}>{statusLabel}</Badge>
      </div>
      <div class="meta-row">
        <span class="meta-item">
          <span class="meta-label">Автор:</span>
          <span class="meta-value">{request.requesterName ?? '—'}</span>
        </span>
        <span class="meta-item">
          <span class="meta-label">Создана:</span>
          <span class="meta-value">{relativeDate(request.createdAtUtc)}</span>
        </span>
      </div>

      <!-- Поля по типу заявки -->
      <DetailSection heading="Информация">
        <div class="fields-grid">
          {#if request.requestType === 'ad_register'}
            <DetailField label="ФИО" value={request.description ?? request.requesterName ?? null} />
            <DetailField label="Логин" value={request.requesterName ?? null} />
            <DetailField
              label="Тип"
              value={isAdRestore ? 'Восстановление доступа' : 'Регистрация'}
            />
          {:else if request.requestType === 'cartridge_replace'}
            <DetailField label="Принтер" value={request.printerName ?? null} />
            {#if request.description}
              <div class="field-wide">
                <DetailField label="Комментарий" value={request.description} />
              </div>
            {/if}
          {:else if request.requestType === 'repair'}
            <DetailField label="Устройство" value={repairDeviceLine} />
            {#if hasDevice && deviceStatusCode !== null}
              <DetailField label="Статус устройства">
                <RepairDeviceStatus
                  code={deviceStatusCode}
                  name={curState?.deviceStatusName ?? null}
                />
              </DetailField>
            {/if}
            {#if hasDevice && curState?.currentPlacePath}
              <DetailField label="Место" value={curState.currentPlacePath} />
            {/if}
            {#if request.categoryName}
              <DetailField label="Категория" value={request.categoryName} />
            {/if}
            {#if request.description}
              <div class="field-wide description-text">
                <DetailField label="Описание" value={request.description} />
              </div>
            {/if}
          {:else}
            {#if request.categoryName}
              <DetailField label="Категория" value={request.categoryName} />
            {/if}
            {#if request.description}
              <div class="field-wide description-text">
                <DetailField label="Описание" value={request.description} />
              </div>
            {/if}
          {/if}
        </div>
      </DetailSection>

      <!-- ad_register: резолюция (только admin, REQ-06/T-09-21) -->
      {#if isAdRegister}
        {#if isAdmin && request.status !== 'open' && request.resolutionNotes}
          <DetailSection>
            <div class="resolution">
              <DetailField label="Комментарий" value={request.resolutionNotes} />
            </div>
          </DetailSection>
        {/if}
      {:else if isSpecialist}
        {#if request.status === 'in_progress' && request.requestType === 'free_form'}
          <!-- in_progress + free_form: инлайн-мини-форма «Выполнить» — Textarea
               остаётся вместе с кнопкой submit, НЕ переносится в header actions. -->
          <DetailSection>
            <div class="complete-form">
              <div class="field">
                <label class="label" for="complete-notes">Комментарий специалиста</label>
                <Textarea
                  value={completeNotes}
                  placeholder="Необязательно"
                  id="complete-notes"
                  oninput={(v) => (completeNotes = v)}
                />
              </div>
            </div>
            <div class="actions">
              <Button variant="primary" loading={completeSubmitting} onclick={handleComplete}>
                Выполнить
              </Button>
            </div>
          </DetailSection>
        {:else if (request.status === 'completed' || request.status === 'rejected') && request.resolutionNotes}
          <DetailSection>
            <div class="resolution">
              <DetailField label="Комментарий специалиста" value={request.resolutionNotes} />
            </div>
          </DetailSection>
        {/if}
      {:else if isOwnRequest && request.status === 'open'}
        <!-- GAP-12-07/A4: Employee — cancel own open request; action-only
             (button lives in header actions), no body content here. -->
      {:else if (request.status === 'completed' || request.status === 'rejected') && request.resolutionNotes}
        <!-- Employee view of terminal state resolution notes -->
        <DetailSection>
          <div class="resolution">
            <DetailField label="Комментарий специалиста" value={request.resolutionNotes} />
          </div>
        </DetailSection>
      {/if}

      <!-- D-13: заходы в сервис между «Информацией» и вложениями -->
      {#if isRepair && curState !== null}
        <RepairVisitsSection
          visits={curState.visits}
          requestId={request.id}
          canManage={isSpecialist}
          onScrollToAttachments={() =>
            attachmentsEl?.scrollIntoView({ behavior: 'smooth', block: 'start' })}
          onChanged={() => {
            if (request) {
              void loadRepairState(request.id);
              void loadHistory(request.id, true);
            }
          }}
        />
      {/if}

      <!-- D-02: вложения для заявки любого статуса -->
      <div bind:this={attachmentsEl}>
        <AttachmentsBlock parentKind="request" parentId={request.id} bind:this={attachmentsBlock} />
      </div>

      <!-- История (REQ-07) -->
      <DetailSection heading="История">
        {#if historyLoading}
          <div class="history-loading">
            <Spinner size="sm" />
          </div>
        {:else if mergedHistory.length === 0}
          <p class="history-empty">История пуста</p>
        {:else}
          <ul class="history-list">
            {#each mergedHistory as row, i (row.kind === 'entry' ? `e${row.entry.id}` : `a${i}`)}
              {#if row.kind === 'entry'}
                <li class="history-row">
                  <span class="history-text">
                    {formatFullDate(row.entry.createdAtUtc)} — {entryText(row.entry)}
                    {#if mergedIntoId(row.entry) !== null}
                      {@const relatedId = mergedIntoId(row.entry) as number}
                      {'; '}<Button variant="link" onclick={() => openRequest(relatedId)}>
                        {row.entry.notes?.trim() ? row.entry.notes.trim() : `Заявка №${relatedId}`}
                      </Button>
                    {/if}
                  </span>
                </li>
              {:else}
                <AttachmentHistory
                  bare
                  events={[row.event]}
                  onOpen={(id) => attachmentsBlock?.open(id)}
                />
              {/if}
            {/each}
          </ul>
        {/if}
      </DetailSection>
    {/if}
  </DetailPanel>
{/if}

<!-- Confirm-modal «Отклонить» -->
<Modal open={rejectModalOpen} title={rejectModalTitle} onClose={() => (rejectModalOpen = false)}>
  <p class="confirm-body">{rejectModalBody}</p>
  <div class="field" style="margin-top: var(--tr-space-md);">
    <label class="label" for="reject-notes">Комментарий специалиста</label>
    <Textarea
      value={rejectNotes}
      placeholder="Необязательно"
      id="reject-notes"
      oninput={(v) => (rejectNotes = v)}
    />
  </div>
  {#snippet footer()}
    <Button variant="secondary" onclick={() => (rejectModalOpen = false)}>Отмена</Button>
    <Button variant="destructive" loading={rejectSubmitting} onclick={handleRejectConfirm}>
      {rejectModalButtonLabel}
    </Button>
  {/snippet}
</Modal>

<!-- GAP-12-07/A4: Confirm-modal «Удалить» (Admin/Manager, any status) -->
<Modal open={deleteModalOpen} title="Удалить заявку?" onClose={() => (deleteModalOpen = false)}>
  <p class="confirm-body">
    Заявка будет удалена без возможности восстановления через интерфейс. Действие необратимо.
  </p>
  {#snippet footer()}
    <Button variant="secondary" onclick={() => (deleteModalOpen = false)}>Отмена</Button>
    <Button variant="destructive" loading={deleteSubmitting} onclick={handleDeleteConfirm}>
      Удалить
    </Button>
  {/snippet}
</Modal>

<!-- GAP-12-07/A4: Confirm-modal «Отменить заявку» (Employee author, open only) -->
<Modal open={cancelModalOpen} title="Отменить заявку?" onClose={() => (cancelModalOpen = false)}>
  <p class="confirm-body">
    Заявка будет отменена. Чтобы продолжить работу с этим запросом, потребуется создать новую
    заявку.
  </p>
  {#snippet footer()}
    <Button variant="secondary" onclick={() => (cancelModalOpen = false)}>Отмена</Button>
    <Button variant="destructive" loading={cancelSubmitting} onclick={handleCancelConfirm}>
      Отменить заявку
    </Button>
  {/snippet}
</Modal>

<!-- Approval modal (Screen 5b, D-REG-02) -->
{#if request !== null}
  <Modal
    open={approveModalOpen}
    title={isAdRestore ? 'Восстановить доступ' : 'Подтвердить регистрацию'}
    onClose={() => (approveModalOpen = false)}
  >
    <p class="confirm-body">
      Пользователь {request.description ?? request.requesterName ?? ''} получит доступ к системе с выбранной
      ролью.
    </p>
    <div class="field" style="margin-top: var(--tr-space-md);">
      <label class="label dropdown-label">
        <span class="label-text">Роль</span>
        <Dropdown
          variant="select"
          flat={true}
          value={approveRoleLabel}
          placeholder="Выберите роль"
          searchPlaceholder="Поиск"
          loading={false}
          groups={ROLE_OPTIONS}
          getGroupId={(o) => o.id}
          getGroupName={(o) => o.label}
          getGroupCount={() => 0}
          isGroupExpandable={() => false}
          isGroupSelected={(o) => o.id === approveRole}
          onExpandGroup={noExpandRole}
          getMemberId={(o) => o.id}
          getMemberName={(o) => o.label}
          onSearch={() => {}}
          onPickGroup={(o) => (approveRole = o.id)}
          onPickMember={() => {}}
        />
      </label>
    </div>
    {#snippet footer()}
      <Button variant="secondary" onclick={() => (approveModalOpen = false)}>Отмена</Button>
      <Button variant="primary" loading={approveSubmitting} onclick={handleApproveConfirm}>
        Подтвердить
      </Button>
    {/snippet}
  </Modal>
{/if}

<!-- REQ-05: OperationModal для «Установить картридж» -->
{#if isSpecialist && request !== null}
  <OperationModal
    open={operationModalOpen}
    op="install"
    cartridge={null}
    preFillPrinterId={request.printerDeviceId ?? undefined}
    cartridgeModelId={request.cartridgeModelId ?? undefined}
    prefillGivenToName={request.requesterName ?? undefined}
    suppressSuccessToast={true}
    onClose={() => (operationModalOpen = false)}
    onSuccess={handleInstallSuccess}
  />
{/if}

<!-- Фаза 41.4: попапы цикла ремонта. Открываются флагами; монтируются только когда
     есть заявка и её состояние ремонта. Успех: попап сам показывает тост, здесь —
     обновить состояние и карточку. D-38: только персоналу; сотрудник эти попапы не
     получает (они читают места и справочники, закрытые для его роли). -->
{#if isSpecialist && request !== null && isRepair && curState !== null}
  <RepairMoveModal
    open={moveModalOpen}
    mode={moveMode}
    {request}
    repairState={curState}
    onClose={() => (moveModalOpen = false)}
    onSuccess={applyRepairResult}
  />
  <RepairSendModal
    open={sendModalOpen}
    {request}
    repairState={curState}
    onClose={() => (sendModalOpen = false)}
    onSuccess={applyRepairResult}
  />
  <RepairReturnModal
    open={returnModalOpen}
    {request}
    repairState={curState}
    onClose={() => (returnModalOpen = false)}
    onSuccess={applyRepairResult}
  />
  <RepairCompleteModal
    open={completeModalOpen}
    {request}
    repairState={curState}
    onClose={() => (completeModalOpen = false)}
    onSuccess={applyRepairResult}
  />
{/if}
{#if isSpecialist && request !== null && isRepair}
  <RepairAssignDeviceModal
    open={assignModalOpen}
    {request}
    onClose={() => (assignModalOpen = false)}
    onSuccess={() => onTransition()}
    onMerged={(targetId) => {
      assignModalOpen = false;
      onTransition();
      openRequest(targetId);
    }}
  />

  <!-- D-12: подтверждение «Отменить ремонт» -->
  <Modal
    open={cancelRepairModalOpen}
    title="Отменить ремонт?"
    initialFocus="#repair-cancel-back button"
    onClose={() => (cancelRepairModalOpen = false)}
  >
    <p class="confirm-body">
      Устройство вернётся на прежнее место («{curState?.originPlacePath ?? 'прежнее место'}»),
      запись о ремонте не сохранится и в отчёт не попадёт. Используйте, если ремонт не потребовался.
    </p>
    {#snippet footer()}
      <span id="repair-cancel-back" class="hint-wrap">
        <Button variant="secondary" onclick={() => (cancelRepairModalOpen = false)}>Назад</Button>
      </span>
      <Button
        variant="destructive"
        loading={cancelRepairSubmitting}
        onclick={handleCancelRepairConfirm}
      >
        Отменить ремонт
      </Button>
    {/snippet}
  </Modal>
{/if}

<style lang="scss">
  .detail-loading {
    height: 100%;
    overflow: auto;
    padding: var(--tr-space-xl);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--tr-space-md);
    min-height: 320px;
    text-align: center;
    color: var(--tr-text-secondary);
  }

  .title-row {
    display: flex;
    align-items: center;
    gap: var(--tr-space-xs);
    flex-wrap: wrap;
    margin-bottom: var(--tr-space-xs);
  }

  .meta-row {
    display: flex;
    gap: var(--tr-space-xl);
    flex-wrap: wrap;
    margin-bottom: var(--tr-space-2xl);
  }

  .meta-item {
    display: flex;
    gap: var(--tr-space-2xs);
    font-size: var(--tr-font-size-label);
  }

  .meta-label {
    color: var(--tr-text-tertiary);
  }

  .meta-value {
    color: var(--tr-text-primary);
  }

  .fields-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--tr-space-md);
  }

  .field-wide {
    grid-column: 1 / -1;
  }

  // D-35: переводы строк дописанного при слиянии блока описания.
  .description-text :global(.field-value) {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .label {
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-tertiary);
  }

  // Plan 28-11 (GAP-1): Dropdown не принимает `id`, поэтому подпись оборачивает
  // поле (implicit label) вместо `for`/`id` association — сохраняет вертикальный
  // макет «подпись сверху, поле снизу», как у остальных .field (CartridgeFormBody
  // precedent, Phase 27-G1).
  .dropdown-label {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-2xs);
  }

  .actions {
    display: flex;
    gap: var(--tr-space-xs);
    flex-wrap: wrap;
    margin-top: var(--tr-space-xs);
  }

  .complete-form {
    margin-bottom: var(--tr-space-md);
  }

  .resolution {
    padding: var(--tr-space-md);
    background: var(--tr-surface);
    border-radius: var(--tr-radius-xs);
    border: 1px solid var(--tr-border);
  }

  .history-loading {
    display: flex;
    justify-content: flex-start;
    padding: var(--tr-space-xs) 0;
  }

  .history-empty {
    margin: 0;
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-tertiary);
    font-style: italic;
  }

  .history-list {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .history-row {
    display: flex;
    align-items: center;
    min-height: var(--row-height-dense, 32px);
    padding: 0;
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-primary);
    border-bottom: 1px solid var(--tr-border);

    &:last-child {
      border-bottom: none;
    }
  }

  .history-text {
    flex: 1;
    padding: var(--tr-space-2xs) 0;
  }

  .hint-wrap {
    display: inline-flex;
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }

  .confirm-body {
    margin: 0;
    color: var(--tr-text-secondary);
    line-height: var(--tr-line-height-body);
  }
</style>
