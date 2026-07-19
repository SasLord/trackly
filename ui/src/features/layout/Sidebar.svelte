<script lang="ts">
  import { link } from 'svelte-spa-router';
  import active from 'svelte-spa-router/active';
  import { getVisibleItems } from './sidebar-config';
  import { authStore } from '$lib/stores/auth.svelte';
  import type { UserRole } from '$lib/stores/auth.svelte';
  import ThemeSwitcher from '$lib/components/ThemeSwitcher.svelte';
  import { apiCall } from '$lib/api/client';

  const visibleItems = $derived(getVisibleItems((authStore.user?.role as UserRole | null) ?? null));

  const ROLE_LABELS: Record<UserRole, string> = {
    admin: 'Администратор',
    manager: 'Специалист',
    employee: 'Сотрудник',
  };

  // D-Desktop-01: unlocked desktop uses a trusted-admin sentinel (id === 0).
  // Logout is meaningless there (the sentinel auto-restores on next auth_status),
  // so only show it for a real authenticated session (browser or locked desktop).
  const canLogout = $derived(authStore.user != null && authStore.user.id !== 0);

  let loggingOut = $state(false);

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

<nav class="sidebar" aria-label="Основная навигация">
  <div class="sidebar-logo" aria-hidden="false">
    <span class="logo-mark" aria-hidden="true"></span>
    <span class="logo-text">Trackly</span>
  </div>
  <ul class="nav-list" role="list">
    {#each visibleItems as entry}
      {#if entry.kind === 'divider'}
        <li class="divider" aria-hidden="true" role="separator"></li>
      {:else}
        <li class="nav-item">
          <a
            href={entry.route}
            use:link
            use:active={{ path: entry.route, className: 'is-active' }}
            class="nav-link"
          >
            {entry.label}
          </a>
        </li>
      {/if}
    {/each}
  </ul>

  <div class="sidebar-footer">
    {#if canLogout && authStore.user}
      <div class="user-block">
        <div class="user-info">
          <span class="user-name">{authStore.user.fullName}</span>
          <span class="user-role">{ROLE_LABELS[authStore.user.role]}</span>
        </div>
        <button type="button" class="logout-btn" onclick={logout} disabled={loggingOut}>
          {loggingOut ? 'Выход…' : 'Выйти'}
        </button>
      </div>
    {/if}
    <div class="theme-row">
      <span class="theme-label">Оформление</span>
      <ThemeSwitcher />
    </div>
  </div>
</nav>

<style lang="scss">
  .sidebar {
    width: var(--sidebar-width);
    height: 100%;
    background: var(--tr-bg);
    border-right: 1px solid var(--tr-border);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .sidebar-logo {
    display: flex;
    align-items: center;
    gap: 9px;
    height: 56px;
    padding: 0 16px;
    border-bottom: 1px solid var(--tr-border);
    flex-shrink: 0;
  }

  .logo-mark {
    width: 11px;
    height: 11px;
    border-radius: 3px;
    background: var(--tr-accent);
    flex-shrink: 0;
  }

  .logo-text {
    font-size: 16px;
    font-weight: 600;
    color: var(--tr-text-primary);
  }

  .nav-list {
    flex: 1;
    list-style: none;
    margin: 0;
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 1px;
    overflow-y: auto;
  }

  .nav-item {
    margin: 0;
    padding: 0;
  }

  .nav-link {
    display: block;
    padding: 0 12px;
    height: 38px;
    line-height: 38px;
    border-radius: var(--tr-radius-sm);
    font-size: var(--tr-font-size-body);
    font-weight: var(--tr-font-weight-regular);
    color: var(--tr-text-secondary);
    text-decoration: none;
    transition: none;

    &:hover {
      background: color-mix(in srgb, var(--tr-text-primary) 5%, transparent);
      color: var(--tr-text-primary);
    }

    &:focus-visible {
      outline: none;
      box-shadow: inset 0 0 0 2px var(--tr-focus-ring);
    }
  }

  :global(.nav-link.is-active) {
    box-shadow: inset 3px 0 0 var(--tr-accent);
    background: var(--tr-accent-soft);
    color: var(--tr-accent-text);
    font-weight: var(--tr-font-weight-medium);
  }

  .divider {
    height: 1px;
    background: var(--tr-border);
    margin: 6px 8px;
  }

  .sidebar-footer {
    padding: 14px 16px;
    border-top: 1px solid var(--tr-border);
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-2xs);
  }

  .theme-row {
    display: flex;
    flex-direction: column;
    gap: 7px;
  }

  .theme-label {
    font-size: 12px;
    color: var(--tr-text-secondary);
    font-weight: var(--tr-font-weight-regular);
  }

  .user-block {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-2xs);
    padding-bottom: 10px;
    margin-bottom: 10px;
    border-bottom: 1px solid var(--tr-border);
  }

  .user-info {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .user-name {
    font-size: var(--tr-font-size-body);
    font-weight: 600;
    color: var(--tr-text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .user-role {
    font-size: 12px;
    color: var(--tr-text-tertiary);
  }

  .logout-btn {
    appearance: none;
    border: 1px solid var(--tr-border);
    background: var(--tr-bg);
    color: var(--tr-text-secondary);
    font-size: var(--tr-font-size-label);
    font-weight: var(--tr-font-weight-medium);
    padding: var(--tr-space-2xs) var(--tr-space-xs);
    border-radius: var(--tr-radius-xs);
    cursor: pointer;
    text-align: center;

    &:hover:not(:disabled) {
      background: color-mix(in srgb, var(--tr-text-primary) 5%, transparent);
      color: var(--tr-text-primary);
    }

    &:focus-visible {
      outline: none;
      box-shadow: 0 0 0 2px var(--tr-focus-ring);
    }

    &:disabled {
      opacity: 0.6;
      cursor: not-allowed;
    }
  }
</style>
