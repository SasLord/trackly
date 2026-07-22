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
  import Button from '$lib/components/Button.svelte';
  import Badge from '$lib/components/Badge.svelte';
  import Spinner from '$lib/components/Spinner.svelte';
  import Modal from '$lib/components/Modal.svelte';
  import Textarea from '$lib/components/Textarea.svelte';
  import Select from '$lib/components/Select.svelte';
  import DetailPanel from '$lib/components/DetailPanel.svelte';
  import DetailSection from '$lib/components/DetailSection.svelte';
  import DetailField from '$lib/components/DetailField.svelte';
  import OperationModal from '../cartridges/OperationModal.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { apiCall } from '$lib/api/client';
  import { requests } from './api';
  import type { RequestDto } from '../../bindings-phase6';
  import type { CurrentUser } from '$lib/stores/auth.svelte';
  import type { RequestHistoryEntry } from './api';
  import type { AdSettingsDto, ApproveAdRegisterDto } from '../../bindings-phase9';

  interface Props {
    request: RequestDto | null;
    loading: boolean;
    identity: CurrentUser | null;
    onTransition: () => void;
  }

  const { request, loading, identity, onTransition }: Props = $props();

  // --- Local state ---
  let operationModalOpen = $state(false);
  let rejectModalOpen = $state(false);
  let rejectNotes = $state('');
  let rejectSubmitting = $state(false);
  let completeNotes = $state('');
  let completeSubmitting = $state(false);
  let historyEntries = $state<RequestHistoryEntry[]>([]);
  let historyLoading = $state(false);

  // ad_register approval modal state (Screen 5b, D-REG-02: role defaults to employee).
  let approveModalOpen = $state(false);
  let approveRole = $state('employee');
  let approveSubmitting = $state(false);

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

  $effect(() => {
    if (!isAdRegister) return;
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
        : request.status === 'in_progress'
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
          : 'Свободная форма',
  );

  // D-01: DetailPanel's header title is a plain string — the two-badge +
  // meta-row header lives as bespoke content inside children (see template).
  const panelTitle = $derived<string | undefined>(request ? typeLabel : undefined);

  // Load history when request changes
  $effect(() => {
    const id = request?.id;
    if (id === undefined || id === null) {
      historyEntries = [];
      return;
    }
    historyLoading = true;
    requests
      .getHistory(id)
      .then((entries) => {
        historyEntries = entries;
      })
      .catch(() => {
        historyEntries = [];
      })
      .finally(() => {
        historyLoading = false;
      });
  });

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

  // --- Lifecycle actions ---

  async function handleAccept() {
    if (!request) return;
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
    if (!request || completeSubmitting) return;
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
    if (!request || rejectSubmitting) return;
    rejectSubmitting = true;
    try {
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
    if (!request || deleteSubmitting) return;
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
    if (!request || approveSubmitting) return;
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
  async function handleInstallSuccess(cartridgeId: number) {
    if (!request) return;
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
        linkedCartridgeId: cartridgeId,
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
          {:else}
            {#if request.categoryName}
              <DetailField label="Категория" value={request.categoryName} />
            {/if}
            {#if request.description}
              <div class="field-wide">
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

      <!-- История (REQ-07) -->
      <DetailSection heading="История">
        {#if historyLoading}
          <div class="history-loading">
            <Spinner size="sm" />
          </div>
        {:else if historyEntries.length === 0}
          <p class="history-empty">История пуста</p>
        {:else}
          <ul class="history-list">
            {#each historyEntries as entry (entry.id)}
              <li class="history-row">
                <span class="history-text">
                  {formatFullDate(entry.createdAtUtc)} — {actionLabel(entry.action)}{entry.actorName
                    ? `; ${entry.actorName}`
                    : ''}{entry.notes ? `; ${entry.notes}` : ''}
                </span>
              </li>
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
      <label class="label" for="approve-role">Роль</label>
      <Select value={approveRole} id="approve-role" onchange={(v) => (approveRole = v)}>
        <option value="employee">Сотрудник</option>
        <option value="manager">Специалист</option>
        <option value="admin">Администратор</option>
      </Select>
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
{#if request !== null}
  <OperationModal
    open={operationModalOpen}
    op="install"
    cartridge={null}
    preFillPrinterId={request.printerDeviceId ?? undefined}
    cartridgeModelId={request.cartridgeModelId ?? undefined}
    prefillLocation={request.printerLocation ?? undefined}
    prefillGivenToName={request.requesterName ?? undefined}
    suppressSuccessToast={true}
    onClose={() => (operationModalOpen = false)}
    onSuccess={handleInstallSuccess}
  />
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

  .field {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .label {
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-tertiary);
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

  .confirm-body {
    margin: 0;
    color: var(--tr-text-secondary);
    line-height: var(--tr-line-height-body);
  }
</style>
