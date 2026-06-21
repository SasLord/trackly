<script lang="ts">
  // Plan 06-05 Task 2: карточка заявки с lifecycle кнопками + история (REQ-07).
  // REQ-05: «Установить картридж» → OperationModal с preFillPrinterId.
  // По паттерну CartridgeDetail.svelte.
  import Button from '$lib/components/Button.svelte';
  import Badge from '$lib/components/Badge.svelte';
  import Spinner from '$lib/components/Spinner.svelte';
  import Modal from '$lib/components/Modal.svelte';
  import Textarea from '$lib/components/Textarea.svelte';
  import Select from '$lib/components/Select.svelte';
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

  // Current AD registration mode — only used to pick the correct reject
  // confirmation copy (auto-accept implies the user already has access and
  // reject must soft-delete; pending implies reject simply discards). The
  // backend is authoritative regardless of what this flag shows.
  let adAutoAccept = $state(false);

  // Derived visibility — specialist/admin maps to manager/admin in the actual UserRole type.
  const isSpecialist = $derived(
    identity?.role === 'admin' || identity?.role === 'manager',
  );

  // ad_register requests are admin-only (REQ-06, T-09-21) — manager cannot act on them.
  const isAdmin = $derived(identity?.role === 'admin');
  const isAdRegister = $derived(request?.requestType === 'ad_register');
  const isAdRestore = $derived(isAdRegister && request?.adSubtype === 'restore');

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
      'custom:create': 'Создана',
      'custom:accept': 'Принята в работу',
      'custom:complete': 'Выполнена',
      'custom:reject': 'Отклонена',
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
  // We then complete the request, linking the installed cartridge.
  async function handleInstallSuccess() {
    if (!request) return;
    operationModalOpen = false;
    // Complete the request after cartridge install.
    try {
      await requests.transition({
        op: 'complete',
        requestId: request.id,
        version: request.version,
        notes: null,
        linkedCartridgeId: null,
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
    }
  }
</script>

<div class="request-detail" aria-live="polite">
  {#if loading}
    <div class="loading">
      <Spinner size="md" />
      <span>Загрузка заявки…</span>
    </div>
  {:else if request === null}
    <div class="empty">
      <h2 class="empty-heading">Выберите заявку</h2>
      <p class="empty-body">Выберите заявку слева, чтобы увидеть детали и историю.</p>
    </div>
  {:else}
    <!-- Header -->
    <header class="detail-header">
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
    </header>

    <!-- Поля по типу заявки -->
    <section class="section">
      <h3 class="section-heading">Информация</h3>
      <div class="fields-grid">
        {#if request.requestType === 'ad_register'}
          <div class="field">
            <span class="field-label">ФИО</span>
            <span class="field-value">{request.description ?? request.requesterName ?? '—'}</span>
          </div>
          <div class="field">
            <span class="field-label">Логин</span>
            <span class="field-value">{request.requesterName ?? '—'}</span>
          </div>
          <div class="field">
            <span class="field-label">Тип</span>
            <span class="field-value">
              {isAdRestore ? 'Восстановление доступа' : 'Регистрация'}
            </span>
          </div>
        {:else if request.requestType === 'cartridge_replace'}
          <div class="field">
            <span class="field-label">Принтер</span>
            <span class="field-value">{request.printerName ?? '—'}</span>
          </div>
          {#if request.description}
            <div class="field field-wide">
              <span class="field-label">Комментарий</span>
              <span class="field-value">{request.description}</span>
            </div>
          {/if}
        {:else}
          {#if request.categoryId !== null}
            <div class="field">
              <span class="field-label">Категория</span>
              <span class="field-value">{request.categoryId}</span>
            </div>
          {/if}
          {#if request.description}
            <div class="field field-wide">
              <span class="field-label">Описание</span>
              <span class="field-value">{request.description}</span>
            </div>
          {/if}
        {/if}
      </div>
    </section>

    <!-- ad_register действия (только admin, REQ-06/T-09-21) -->
    {#if isAdRegister}
      {#if isAdmin}
        <section class="section">
          {#if request.status === 'open'}
            <div class="actions">
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
            </div>
          {:else if request.resolutionNotes}
            <div class="resolution">
              <span class="field-label">Комментарий</span>
              <span class="field-value">{request.resolutionNotes}</span>
            </div>
          {/if}
        </section>
      {/if}
    {:else if isSpecialist}
      <section class="section">
        {#if request.status === 'open'}
          <!-- open: Принять в работу / Отклонить -->
          <div class="actions">
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
          </div>
        {:else if request.status === 'in_progress' && request.requestType === 'free_form'}
          <!-- in_progress + free_form: Выполнить / Отклонить -->
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
            <Button
              variant="destructive"
              onclick={() => {
                rejectNotes = '';
                rejectModalOpen = true;
              }}
            >
              Отклонить
            </Button>
          </div>
        {:else if request.status === 'in_progress' && request.requestType === 'cartridge_replace'}
          <!-- in_progress + cartridge_replace: Установить картридж / Отклонить -->
          <div class="actions">
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
          </div>
        {:else if request.status === 'completed' || request.status === 'rejected'}
          <!-- Terminal — show resolution notes -->
          {#if request.resolutionNotes}
            <div class="resolution">
              <span class="field-label">Комментарий специалиста</span>
              <span class="field-value">{request.resolutionNotes}</span>
            </div>
          {/if}
        {/if}
      </section>
    {:else if (request.status === 'completed' || request.status === 'rejected') && request.resolutionNotes}
      <!-- Employee view of terminal state resolution notes -->
      <section class="section">
        <div class="resolution">
          <span class="field-label">Комментарий специалиста</span>
          <span class="field-value">{request.resolutionNotes}</span>
        </div>
      </section>
    {/if}

    <!-- История (REQ-07) -->
    <section class="section">
      <h3 class="section-heading">История</h3>
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
    </section>
  {/if}
</div>

<!-- Confirm-modal «Отклонить» -->
<Modal open={rejectModalOpen} title={rejectModalTitle} onClose={() => (rejectModalOpen = false)}>
  <p class="confirm-body">{rejectModalBody}</p>
  <div class="field" style="margin-top: var(--space-md);">
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

<!-- Approval modal (Screen 5b, D-REG-02) -->
{#if request !== null}
  <Modal
    open={approveModalOpen}
    title={isAdRestore ? 'Восстановить доступ' : 'Подтвердить регистрацию'}
    onClose={() => (approveModalOpen = false)}
  >
    <p class="confirm-body">
      Пользователь {request.description ?? request.requesterName ?? ''} получит доступ к системе с
      выбранной ролью.
    </p>
    <div class="field" style="margin-top: var(--space-md);">
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
    onClose={() => (operationModalOpen = false)}
    onSuccess={handleInstallSuccess}
  />
{/if}

<style lang="scss">
  .request-detail {
    height: 100%;
    overflow: auto;
    padding: var(--space-lg);
    background: var(--color-bg);
  }

  .loading,
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-md);
    min-height: 320px;
    text-align: center;
    color: var(--color-text-secondary);
  }

  .empty-heading {
    margin: 0;
    font-size: var(--font-size-heading);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
  }

  .empty-body {
    margin: 0;
    max-width: 360px;
    color: var(--color-text-secondary);
  }

  .detail-header {
    margin-bottom: var(--space-xl);
  }

  .title-row {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    flex-wrap: wrap;
    margin-bottom: var(--space-sm);
  }

  .meta-row {
    display: flex;
    gap: var(--space-lg);
    flex-wrap: wrap;
  }

  .meta-item {
    display: flex;
    gap: var(--space-xs);
    font-size: var(--font-size-label);
  }

  .meta-label {
    color: var(--color-text-muted);
  }

  .meta-value {
    color: var(--color-text-primary);
  }

  .section {
    margin-bottom: var(--space-xl);
  }

  .section-heading {
    margin: 0 0 var(--space-md);
    font-size: var(--font-size-body);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
  }

  .fields-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-md);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .field-wide {
    grid-column: 1 / -1;
  }

  .field-label,
  .label {
    font-size: var(--font-size-label);
    color: var(--color-text-muted);
  }

  .field-value {
    font-size: var(--font-size-body);
    color: var(--color-text-primary);
  }

  .actions {
    display: flex;
    gap: var(--space-sm);
    flex-wrap: wrap;
    margin-top: var(--space-sm);
  }

  .complete-form {
    margin-bottom: var(--space-md);
  }

  .resolution {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: var(--space-md);
    background: var(--color-surface);
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-border);
  }

  .history-loading {
    display: flex;
    justify-content: flex-start;
    padding: var(--space-sm) 0;
  }

  .history-empty {
    margin: 0;
    font-size: var(--font-size-body);
    color: var(--color-text-muted);
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
    font-size: var(--font-size-label);
    color: var(--color-text-primary);
    border-bottom: 1px solid var(--color-border);

    &:last-child {
      border-bottom: none;
    }
  }

  .history-text {
    flex: 1;
    padding: var(--space-xs) 0;
  }

  .confirm-body {
    margin: 0;
    color: var(--color-text-secondary);
    line-height: var(--line-height-body);
  }
</style>
