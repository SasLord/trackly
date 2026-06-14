<script lang="ts">
  import { link } from 'svelte-spa-router';
  import active from 'svelte-spa-router/active';
  import { getVisibleItems } from './sidebar-config';
  import { authStore } from '$lib/stores/auth.svelte';
  import type { UserRole } from '$lib/stores/auth.svelte';
  import ThemeSwitcher from '$lib/components/ThemeSwitcher.svelte';
  import { apiCall } from '$lib/api/client';

  const visibleItems = $derived(getVisibleItems(authStore.user?.role as UserRole | null ?? null));

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
        <button
          type="button"
          class="logout-btn"
          onclick={logout}
          disabled={loggingOut}
        >
          {loggingOut ? 'Выход…' : 'Выйти'}
        </button>
      </div>
    {/if}
    <div class="theme-row">
      <span class="theme-label">Тема</span>
      <ThemeSwitcher />
    </div>
  </div>
</nav>

<style lang="scss">
  .sidebar {
    width: var(--sidebar-width);
    height: 100%;
    background: var(--color-surface);
    border-right: 1px solid var(--color-border);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .nav-list {
    flex: 1;
    list-style: none;
    margin: 0;
    padding: var(--space-sm) 0;
    overflow-y: auto;
  }

  .nav-item {
    margin: 0;
    padding: 0;
  }

  .nav-link {
    display: block;
    padding: 0 var(--space-md);
    height: var(--row-height);
    line-height: var(--row-height);
    font-size: var(--font-size-body);
    font-weight: var(--font-weight-regular);
    color: var(--color-text-secondary);
    text-decoration: none;
    border-left: 3px solid transparent;
    transition: none;

    &:hover {
      background: color-mix(in srgb, var(--color-text-primary) 5%, transparent);
      color: var(--color-text-primary);
    }

    &:focus-visible {
      outline: none;
      box-shadow: inset 0 0 0 2px var(--color-accent-focus);
    }
  }

  :global(.nav-link.is-active) {
    border-left-color: var(--color-accent);
    background: color-mix(in srgb, var(--color-accent) 10%, transparent);
    color: var(--color-text-primary);
    font-weight: var(--font-weight-medium);
  }

  .divider {
    height: 1px;
    background: var(--color-border);
    margin: var(--space-xs) var(--space-md);
  }

  .sidebar-footer {
    padding: var(--space-md);
    border-top: 1px solid var(--color-border);
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }

  .theme-row {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }

  .theme-label {
    font-size: var(--font-size-label);
    color: var(--color-text-muted);
    font-weight: var(--font-weight-regular);
  }

  .user-block {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
    padding-bottom: var(--space-sm);
    margin-bottom: var(--space-xs);
    border-bottom: 1px solid var(--color-border);
  }

  .user-info {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .user-name {
    font-size: var(--font-size-body);
    font-weight: var(--font-weight-medium);
    color: var(--color-text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .user-role {
    font-size: var(--font-size-label);
    color: var(--color-text-muted);
  }

  .logout-btn {
    appearance: none;
    border: 1px solid var(--color-border);
    background: var(--color-bg);
    color: var(--color-text-secondary);
    font-size: var(--font-size-label);
    font-weight: var(--font-weight-medium);
    padding: var(--space-xs) var(--space-sm);
    border-radius: var(--radius-sm);
    cursor: pointer;
    text-align: center;

    &:hover:not(:disabled) {
      background: color-mix(in srgb, var(--color-text-primary) 5%, transparent);
      color: var(--color-text-primary);
    }

    &:focus-visible {
      outline: none;
      box-shadow: 0 0 0 2px var(--color-accent-focus);
    }

    &:disabled {
      opacity: 0.6;
      cursor: not-allowed;
    }
  }
</style>
