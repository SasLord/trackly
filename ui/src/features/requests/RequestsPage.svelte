<script lang="ts">
  // Plan 06-05: корневой компонент раздела «Заявки».
  // Роль-зависимая логика: сотрудник видит только свои заявки; специалист/admin — все.
  // WS push: specialist/admin получают toast о новой заявке через onWsEvent(new_request).
  // По паттерну PrintersPage.svelte.
  import { onMount } from 'svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { authStore } from '$lib/stores/auth.svelte';
  import { connectWs, onWsEvent } from '$lib/api/ws';
  import { apiCall } from '$lib/api/client';
  import RequestsMasterDetail from './RequestsMasterDetail.svelte';
  import RequestsSearchAndTabs from './RequestsSearchAndTabs.svelte';
  import RequestsList from './RequestsList.svelte';
  import RequestDetail from './RequestDetail.svelte';
  import RequestFormModal from './RequestFormModal.svelte';
  import StatWidget from '../dashboard/StatWidget.svelte';
  import { requests } from './api';
  import type { RequestDto, RequestFilter, WsEvent } from '../../bindings-phase6';
  import type { DashboardWidgetDto } from '../../bindings';

  let items = $state<RequestDto[]>([]);
  let listLoading = $state(false);
  let selectedId = $state<number | null>(null);
  let selectedRequest = $state<RequestDto | null>(null);
  let detailLoading = $state(false);
  let formModalOpen = $state(false);

  // D-GATE-03: «Мои заявки» summary card data (employee-scoped dashboard_get_all_widgets
  // branch built in 10-03) — fetched only for the employee role, see isEmployee effect below.
  let dashboardWidget: DashboardWidgetDto | null = $state(null);
  let dashboardLoading = $state(true);
  let dashboardError: string | null = $state(null);

  const identity = $derived(authStore.user);

  // Role-based filter: employees see only their own requests (D-RBAC-02).
  // Backend enforces this — UI passes requestedByUserId for employee role.
  const baseFilter = $derived<RequestFilter>({
    status: null,
    requestType: null,
    assignedToUserId: null,
    requestedByUserId: identity?.role === 'employee' ? (identity?.id ?? null) : null,
  });

  let filter = $state<RequestFilter>({
    status: null,
    requestType: null,
    assignedToUserId: null,
    requestedByUserId: null,
  });

  // Sync filter with role-based baseFilter when identity changes.
  $effect(() => {
    filter = { ...baseFilter };
  });

  // Reload list when filter changes.
  $effect(() => {
    void filter;
    void refresh();
  });

  // Load detail when selectedId changes.
  $effect(() => {
    const id = selectedId;
    if (id === null) {
      selectedRequest = null;
      return;
    }
    detailLoading = true;
    requests
      .get(id)
      .then((dto) => {
        selectedRequest = dto;
      })
      .catch((e: unknown) => {
        const msg =
          e && typeof e === 'object' && 'message' in e
            ? String((e as { message: unknown }).message)
            : 'Не удалось загрузить данные заявки';
        pushToast('error', msg);
        selectedRequest = null;
      })
      .finally(() => {
        detailLoading = false;
      });
  });

  async function refresh() {
    listLoading = true;
    try {
      const resp = await requests.list(filter, { offset: 0, limit: 100 });
      items = resp.items;
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось загрузить заявки';
      pushToast('error', msg);
    } finally {
      listLoading = false;
    }
  }

  function handleWsEvent(event: WsEvent) {
    const role = identity?.role;
    if (event.type === 'new_request' && (role === 'admin' || role === 'manager')) {
      pushToast(
        'info',
        `Новая заявка: ${event.requestType === 'cartridge_replace' ? 'Замена картриджа' : 'Свободная форма'} от ${event.requesterName}`,
      );
      void refresh();
    } else if (event.type === 'request_status_changed') {
      // Refresh list for anyone.
      void refresh();
      // Also refresh selected request detail if it's the one changed.
      if (selectedId === event.requestId) {
        detailLoading = true;
        requests
          .get(event.requestId)
          .then((dto) => {
            selectedRequest = dto;
          })
          .catch(() => {
            // Non-fatal.
          })
          .finally(() => {
            detailLoading = false;
          });
      }
    }
  }

  onMount(() => {
    void refresh();
    // Connect WS for real-time notifications.
    let unlisten: (() => void) | undefined;
    connectWs()
      .then((fn) => {
        unlisten = fn;
      })
      .catch(() => {
        // WS connection is non-fatal.
      });
    const unsubscribe = onWsEvent(handleWsEvent);
    return () => {
      unsubscribe();
      unlisten?.();
    };
  });

  function handleStatusFilterChange(f: RequestFilter) {
    filter = { ...f, requestedByUserId: baseFilter.requestedByUserId };
    selectedId = null;
  }

  function handleTransition() {
    void refresh();
    // Re-fetch the detail if a request is selected.
    if (selectedId !== null) {
      const id = selectedId;
      detailLoading = true;
      requests
        .get(id)
        .then((dto) => {
          selectedRequest = dto;
        })
        .catch(() => {
          selectedRequest = null;
        })
        .finally(() => {
          detailLoading = false;
        });
    }
  }

  const isEmployee = $derived(identity?.role === 'employee');

  // D-GATE-03: fetch the employee-scoped dashboard summary once, only for employees.
  // period: null per UI-SPEC — the card shows an all-time total, not a month-scoped period.
  $effect(() => {
    if (!isEmployee) return;
    dashboardLoading = true;
    apiCall<DashboardWidgetDto>('dashboard_get_all_widgets', { period: null })
      .then((dto) => {
        dashboardWidget = dto;
        dashboardError = null;
      })
      .catch(() => {
        dashboardError = 'Не удалось загрузить сводку';
      })
      .finally(() => {
        dashboardLoading = false;
      });
  });

  const emptyConfig = $derived(
    filter.status !== null
      ? {
          heading: 'Ничего не найдено',
          body: 'Попробуйте изменить фильтр статуса.',
          actionLabel: null as string | null,
          onAction: undefined as (() => void) | undefined,
        }
      : isEmployee
        ? {
            heading: 'У вас пока нет заявок',
            body: 'Создайте заявку — выберите тип и опишите проблему.',
            actionLabel: 'Создать заявку',
            onAction: () => (formModalOpen = true),
          }
        : {
            heading: 'Заявок пока нет',
            body: 'Новые заявки от сотрудников появятся здесь.',
            actionLabel: null as string | null,
            onAction: undefined as (() => void) | undefined,
          },
  );
</script>

<div class="requests-page">
  <header class="page-header">
    <h1 class="page-title">Заявки</h1>
  </header>

  <div class="page-content">
    {#if isEmployee}
      <div class="employee-summary">
        <StatWidget
          id="my-requests"
          title="Мои заявки"
          mainNumber={dashboardWidget
            ? dashboardWidget.request_counts_open + dashboardWidget.request_counts_in_progress
            : null}
          mainLabel="активных заявок"
          breakdown={dashboardWidget
            ? [
                { label: 'Новые', count: dashboardWidget.request_counts_open },
                { label: 'В работе', count: dashboardWidget.request_counts_in_progress },
                { label: 'Выполнено', count: dashboardWidget.request_counts_completed },
              ]
            : []}
          loading={dashboardLoading}
          error={dashboardError}
        />
      </div>
    {/if}

    <RequestsSearchAndTabs
      {filter}
      onFilterChange={handleStatusFilterChange}
      onCreateClick={() => (formModalOpen = true)}
      {identity}
    />

    <RequestsMasterDetail>
      {#snippet master()}
        <RequestsList
          {items}
          loading={listLoading}
          {selectedId}
          {emptyConfig}
          onSelect={(id) => (selectedId = id)}
        />
      {/snippet}
      {#snippet detail()}
        <RequestDetail
          request={selectedRequest}
          loading={detailLoading}
          {identity}
          onTransition={handleTransition}
        />
      {/snippet}
    </RequestsMasterDetail>
  </div>
</div>

<RequestFormModal
  open={formModalOpen}
  onClose={() => (formModalOpen = false)}
  onSuccess={() => {
    formModalOpen = false;
    void refresh();
  }}
/>

<style lang="scss">
  .requests-page {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .page-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--tr-space-xl) var(--tr-space-2xl);
    border-bottom: 1px solid var(--tr-border);
    flex-shrink: 0;
    gap: var(--tr-space-md);
    flex-wrap: wrap;
  }

  .page-title {
    margin: 0;
    font-size: var(--font-size-page-title, var(--font-size-heading));
    font-weight: var(--font-weight-semibold);
    color: var(--tr-text-primary);
    line-height: var(--line-height-heading);
  }

  .page-content {
    flex: 1;
    overflow: auto;
    padding: var(--tr-space-xl) var(--tr-space-2xl);
  }

  .employee-summary {
    margin-bottom: var(--tr-space-xl);
  }
</style>
