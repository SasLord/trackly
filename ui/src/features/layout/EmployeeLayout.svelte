<script lang="ts">
  // Plan 10-04: отдельная минимальная header-оболочка для роли «Сотрудник» (D-UI-01).
  // НЕ ветка Layout.svelte/Sidebar.svelte — самостоятельный компонент: у Сотрудника
  // нет доступа к разделам, которые отображает Sidebar, поэтому нет смысла переиспользовать
  // sidebar-grid. Реальная граница доступа — backend 403 (10-01/10-02/10-03), этот компонент
  // только формирует честный UX.
  import { onMount } from 'svelte';
  import type { Snippet } from 'svelte';
  import active from 'svelte-spa-router/active';
  import Button from '@trackly/kit/components/Button.svelte';
  import UnreadBadge from '@trackly/kit/components/UnreadBadge.svelte';
  import { unreadStore } from '../requests/chat/unreadStore.svelte';
  import { sidebarActivePattern } from './sidebar-config';
  import ThemeSwitcher from '$lib/components/ThemeSwitcher.svelte';
  import ActionMenu from '@trackly/kit/components/ActionMenu.svelte';
  import { themeStore, setTheme } from '@trackly/kit/stores/theme.svelte';
  import { apiCall } from '$lib/api/client';
  import { authStore } from '$lib/stores/auth.svelte';
  import { connectWs, onWsEvent } from '$lib/api/ws';
  import { startChatRealtime } from '../requests/chat/chatNotifications.svelte';
  import { pushToast } from '@trackly/kit/stores/toast.svelte';
  import { appHasFocus, showSystemNotification } from '$lib/platform/systemNotify';
  import { openRequestTarget, requestTargetHref } from '../requests/requestTarget';
  import type { WsEvent } from '../../bindings-phase6';

  interface Props {
    children?: Snippet;
  }

  const { children }: Props = $props();

  let loggingOut = $state(false);

  // UAT 41.6 п.14: на узком экране шапка не влезала в строку — ФИО, роль, тема и
  // «Выйти» уходят в меню за квадратной кнопкой-кебабом (≤959px: ниже роль, тема и
  // «Выйти» не помещаются в строку с вкладками).
  const themeOptions = [
    { key: 'light' as const, label: 'Светлая тема' },
    { key: 'system' as const, label: 'Системная тема' },
    { key: 'dark' as const, label: 'Тёмная тема' },
  ];

  // D-01: две вкладки-ссылки. Активность подсвечивает use:active (класс), а aria-current
  // считается от того же шаблона по текущему маршруту (хвост querystring допускается).
  const requestsPattern = sidebarActivePattern('/requests');
  const devicesPattern = sidebarActivePattern('/devices');
  const currentPath = () => window.location.hash.replace(/^#/, '') || '/';
  let routePath = $state(currentPath());
  const requestsCurrent = $derived(requestsPattern.test(routePath));
  const devicesCurrent = $derived(devicesPattern.test(routePath));

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
      case 'in_repair':
        return 'Ваша заявка: устройство в ремонте';
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

    // D-G1: тост со ссылкой — всегда; уведомление ОС — дополнительно при потере фокуса
    // окна (не document.hidden). Только простой текст, не HTML (T-11-03-T).
    const text = statusToastText(event.newStatus);
    const negative = event.newStatus === 'rejected' || event.newStatus === 'cancelled';
    pushToast(negative ? 'info' : 'success', text, {
      link: { label: 'Открыть заявку', href: requestTargetHref(event.requestId, 'info') },
    });
    if (!appHasFocus()) {
      void showSystemNotification('Trackly', text, {
        activate: () => openRequestTarget(event.requestId, 'info'),
        href: requestTargetHref(event.requestId, 'info'),
        label: `Открыть заявку №${event.requestId}`,
      });
    }
  }

  onMount(() => {
    if (authStore.user?.role !== 'employee') return;

    // WR-01: connectWs() bumps the shared refCount synchronously but resolves
    // its teardown asynchronously. If this component unmounts before the promise
    // resolves, the cleanup below runs while `unlisten` is still undefined — the
    // later-arriving release would never fire and refCount would leak across
    // fast mount/unmount cycles. The `disposed` flag tears the connection down
    // immediately when the promise resolves after unmount. The release fn is
    // idempotent (see ws.ts refCount/released guards), so this is safe.
    let disposed = false;
    let unlisten: (() => void) | undefined;
    connectWs()
      .then((fn) => {
        if (disposed) {
          fn();
        } else {
          unlisten = fn;
        }
      })
      .catch(() => {
        // WS connection is non-fatal — graceful-degrade, no notifications.
      });
    const unsubscribe = onWsEvent(handleEmployeeWsEvent);
    const stopChat = startChatRealtime();

    return () => {
      disposed = true;
      unsubscribe();
      stopChat();
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

<svelte:window onhashchange={() => (routePath = currentPath())} />

<a href="#main" class="skip-link">Перейти к основному содержимому</a>

<div class="employee-shell">
  <header class="employee-header">
    <div class="employee-header-main">
      <span class="employee-brand">Trackly</span>
      <nav aria-label="Разделы" class="employee-tabs">
        <a
          href="#/requests"
          class="tab"
          use:active={{ path: sidebarActivePattern('/requests'), className: 'is-active' }}
          aria-current={requestsCurrent ? 'page' : undefined}
        >
          <span>Заявки</span>
          <UnreadBadge count={unreadStore.requestsWithUnread} label="Заявок с непрочитанным" />
        </a>
        <a
          href="#/devices"
          class="tab"
          use:active={{ path: sidebarActivePattern('/devices'), className: 'is-active' }}
          aria-current={devicesCurrent ? 'page' : undefined}
        >
          <span>Устройства</span>
        </a>
      </nav>
    </div>
    <div class="employee-header-menu">
      <ActionMenu label="Меню пользователя" panelMinWidth="220px">
        {#if authStore.user}
          <div class="menu-user">
            <span class="menu-user-name">{authStore.user.fullName}</span>
            <span class="menu-user-role">Сотрудник</span>
          </div>
          <hr />
        {/if}
        {#each themeOptions as opt (opt.key)}
          <button
            type="button"
            role="menuitem"
            aria-label={themeStore.preference === opt.key ? `${opt.label} (выбрана)` : opt.label}
            onclick={() => setTheme(opt.key)}
          >
            <span class="menu-check" aria-hidden="true"
              >{themeStore.preference === opt.key ? '✓' : ''}</span
            >
            {opt.label}
          </button>
        {/each}
        <hr />
        <button type="button" role="menuitem" onclick={logout} disabled={loggingOut}>
          <span class="menu-check" aria-hidden="true"></span>
          {loggingOut ? 'Выход…' : 'Выйти'}
        </button>
      </ActionMenu>
    </div>
    <div class="employee-header-actions">
      {#if authStore.user}
        <span class="user-name">{authStore.user.fullName}</span>
        <span class="user-role">Сотрудник</span>
      {/if}
      <div class="theme-switcher-slot">
        <ThemeSwitcher />
      </div>
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
    height: 100vh;
    overflow: hidden;
    background: var(--tr-bg);
  }

  .employee-header {
    height: var(--header-height);
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 var(--tr-space-xl);
    background: var(--tr-surface);
    border-bottom: 1px solid var(--tr-border);
  }

  .employee-header-main {
    display: flex;
    align-items: center;
    gap: var(--tr-space-lg);
    // UAT 41.6: вкладки не сжимаются — иначе ФИО наезжало на «Устройства».
    flex-shrink: 0;
  }

  .employee-tabs {
    display: flex;
    align-items: center;
    gap: var(--tr-space-2xs);
  }

  .tab {
    display: inline-flex;
    align-items: center;
    gap: var(--tr-space-xs);
    height: 36px;
    padding: 0 var(--tr-space-sm);
    border-radius: var(--tr-radius-sm);
    font-size: var(--tr-font-size-body);
    font-weight: var(--tr-font-weight-regular);
    color: var(--tr-text-secondary);
    text-decoration: none;
    white-space: nowrap;

    &:hover {
      background: color-mix(in srgb, var(--tr-text-primary) 5%, transparent);
      color: var(--tr-text-primary);
    }

    &:focus-visible {
      outline: none;
      box-shadow: inset 0 0 0 2px var(--tr-focus-ring);
    }
  }

  :global(.tab.is-active) {
    background: var(--tr-accent-soft);
    color: var(--tr-accent-text);
    font-weight: var(--tr-font-weight-medium);
    box-shadow: inset 0 -3px 0 var(--tr-accent);
  }

  .employee-brand {
    font-size: var(--tr-font-size-h3);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-primary);
  }

  .employee-header-actions {
    display: flex;
    flex: 1 1 auto;
    justify-content: flex-end;
    align-items: center;
    gap: var(--tr-space-md);
    min-width: 0;
    margin-left: var(--tr-space-lg);
  }

  .user-name {
    font-size: var(--tr-font-size-body);
    font-weight: var(--tr-font-weight-medium);
    color: var(--tr-text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
    flex-shrink: 1;
  }

  .user-role {
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-tertiary);
    flex-shrink: 0;
    white-space: nowrap;
  }

  .theme-switcher-slot {
    flex-shrink: 0;
    width: max-content;
  }

  .employee-header-menu {
    display: none;
    flex-shrink: 0;
  }

  .menu-user {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: var(--tr-space-xs) var(--tr-space-sm);
    max-width: 280px;
  }

  .menu-user-name {
    font-size: var(--tr-font-size-body);
    font-weight: var(--tr-font-weight-medium);
    color: var(--tr-text-primary);
    overflow-wrap: anywhere;
  }

  .menu-user-role {
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-tertiary);
  }

  .menu-check {
    display: inline-block;
    width: 1.2em;
  }

  @media (max-width: 959px) {
    .employee-header {
      padding: 0 var(--tr-space-md);
      gap: var(--tr-space-sm);
    }

    .employee-header-actions {
      display: none;
    }

    .employee-header-menu {
      display: block;
    }
  }

  @media (max-width: 399px) {
    .employee-brand {
      display: none;
    }
  }

  .employee-content {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }

  .skip-link {
    position: absolute;
    left: -9999px;
    top: -9999px;
    z-index: 9999;
    padding: var(--tr-space-md);
    background: var(--tr-accent);
    color: var(--tr-text-inverse);
    font-size: var(--tr-font-size-body);
    text-decoration: none;
    border-radius: var(--tr-radius-xs);

    &:focus {
      left: 0;
      top: 0;
    }
  }
</style>
