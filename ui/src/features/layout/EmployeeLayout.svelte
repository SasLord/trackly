<script lang="ts">
  // Plan 10-04: отдельная минимальная header-оболочка для роли «Сотрудник» (D-UI-01).
  // НЕ ветка Layout.svelte/Sidebar.svelte — самостоятельный компонент: у Сотрудника
  // нет доступа к разделам, которые отображает Sidebar, поэтому нет смысла переиспользовать
  // sidebar-grid. Реальная граница доступа — backend 403 (10-01/10-02/10-03), этот компонент
  // только формирует честный UX.
  import { onMount } from 'svelte';
  import type { Snippet } from 'svelte';
  import Button from '$lib/components/Button.svelte';
  import ThemeSwitcher from '$lib/components/ThemeSwitcher.svelte';
  import { apiCall } from '$lib/api/client';
  import { authStore } from '$lib/stores/auth.svelte';
  import { connectWs, onWsEvent } from '$lib/api/ws';
  import { pushToast } from '$lib/stores/toast.svelte';
  import type { WsEvent } from '../../bindings-phase6';

  interface Props {
    children?: Snippet;
  }

  const { children }: Props = $props();

  let loggingOut = $state(false);

  // D-WS-01: realtime delivery of the admin's response to the employee's OWN
  // request — toast while the tab is active, system Notification when the
  // tab is hidden (Page Visibility) and permission has been granted.
  // Server-side `is_visible_to` (dto/printer.rs) is the SOLE security
  // boundary — it only forwards this event to the request's author (or
  // admin/manager). The client never needs to re-check ownership; this
  // handler is UX-only (T-11-03-E: never rely on the client as a filter).
  function statusToastText(newStatus: string): string {
    switch (newStatus) {
      case 'in_progress':
        return 'Ваша заявка принята в работу';
      case 'completed':
        return 'Ваша заявка выполнена';
      case 'rejected':
        return 'Ваша заявка отклонена';
      case 'cancelled':
        return 'Ваша заявка отменена';
      default:
        return 'Статус вашей заявки изменён';
    }
  }

  function handleEmployeeWsEvent(event: WsEvent) {
    if (event.type !== 'request_status_changed') return;

    const text = statusToastText(event.newStatus);
    const canNotify =
      'Notification' in window && window.isSecureContext && Notification.permission === 'granted';

    if (document.hidden && canNotify) {
      // Plain-text body only — never HTML (T-11-03-T).
      new Notification('Trackly', { body: text });
    } else {
      const negative = event.newStatus === 'rejected' || event.newStatus === 'cancelled';
      pushToast(negative ? 'info' : 'success', text);
    }
  }

  onMount(() => {
    if (authStore.user?.role !== 'employee') return;

    let unlisten: (() => void) | undefined;
    connectWs()
      .then((fn) => {
        unlisten = fn;
      })
      .catch(() => {
        // WS connection is non-fatal — graceful-degrade, no notifications.
      });
    const unsubscribe = onWsEvent(handleEmployeeWsEvent);

    return () => {
      unsubscribe();
      unlisten?.();
    };
  });

  async function logout() {
    if (loggingOut) return;
    loggingOut = true;
    try {
      await apiCall<null>('auth_logout', {});
    } catch {
      // Even if the server call fails, drop the local session so the user can
      // re-authenticate. apiCall already clears authStore on 401.
    } finally {
      authStore.user = null;
      loggingOut = false;
      window.location.hash = '#/login';
    }
  }
</script>

<a href="#main" class="skip-link">Перейти к основному содержимому</a>

<div class="employee-shell">
  <header class="employee-header">
    <span class="employee-brand">Trackly</span>
    <div class="employee-header-actions">
      {#if authStore.user}
        <span class="user-name">{authStore.user.fullName}</span>
        <span class="user-role">Сотрудник</span>
      {/if}
      <ThemeSwitcher />
      <Button variant="ghost" size="sm" onclick={logout} disabled={loggingOut}>
        {loggingOut ? 'Выход…' : 'Выйти'}
      </Button>
    </div>
  </header>
  <main id="main" class="employee-content">
    {@render children?.()}
  </main>
</div>

<style lang="scss">
  .employee-shell {
    display: flex;
    flex-direction: column;
    min-height: 100vh;
    background: var(--color-bg);
  }

  .employee-header {
    height: var(--header-height, 56px);
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 var(--space-lg);
    background: var(--color-surface);
    border-bottom: 1px solid var(--color-border);
  }

  .employee-brand {
    font-size: var(--font-size-heading);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
  }

  .employee-header-actions {
    display: flex;
    align-items: center;
    gap: var(--space-md);
  }

  .user-name {
    font-size: var(--font-size-body);
    font-weight: var(--font-weight-medium);
    color: var(--color-text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 200px;
  }

  .user-role {
    font-size: var(--font-size-label);
    color: var(--color-text-muted);
  }

  .employee-content {
    flex: 1;
    padding: var(--space-lg);
    min-height: calc(100vh - 56px);
    overflow: auto;
  }

  .skip-link {
    position: absolute;
    left: -9999px;
    top: -9999px;
    z-index: 9999;
    padding: var(--space-md);
    background: var(--color-accent);
    color: var(--color-text-inverse);
    font-size: var(--font-size-body);
    text-decoration: none;
    border-radius: var(--radius-sm);

    &:focus {
      left: 0;
      top: 0;
    }
  }
</style>
