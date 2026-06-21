<script lang="ts">
  // Plan 10-04: отдельная минимальная header-оболочка для роли «Сотрудник» (D-UI-01).
  // НЕ ветка Layout.svelte/Sidebar.svelte — самостоятельный компонент: у Сотрудника
  // нет доступа к разделам, которые отображает Sidebar, поэтому нет смысла переиспользовать
  // sidebar-grid. Реальная граница доступа — backend 403 (10-01/10-02/10-03), этот компонент
  // только формирует честный UX.
  import type { Snippet } from 'svelte';
  import Button from '$lib/components/Button.svelte';
  import ThemeSwitcher from '$lib/components/ThemeSwitcher.svelte';
  import { apiCall } from '$lib/api/client';
  import { authStore } from '$lib/stores/auth.svelte';

  interface Props {
    children?: Snippet;
  }

  const { children }: Props = $props();

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
