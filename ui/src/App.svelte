<script lang="ts">
  import { onMount } from 'svelte';
  import Router from 'svelte-spa-router';
  import { routes, employeeRoutes } from './routes';
  import Layout from './features/layout/Layout.svelte';
  import EmployeeLayout from './features/layout/EmployeeLayout.svelte';
  import ToastHost from '$lib/components/ToastHost.svelte';
  import LoginPage from './features/auth/LoginPage.svelte';
  import FirstRunWizard from './features/auth/FirstRunWizard.svelte';
  import { apiCall } from '$lib/api/client';
  import { trySilentAdSso } from '$lib/api/adSso';
  import { authStore } from '$lib/stores/auth.svelte';
  import type { UserRole } from '$lib/stores/auth.svelte';
  import type { AuthStatusDto } from './bindings';
  import { normalizePlacePathDisplay } from '$lib/utils/placePath';

  // D-Desktop-01: detect Tauri context.
  const isTauri = typeof (window as any).__TAURI_INTERNALS__ !== 'undefined';

  let appLoading = $state(true);
  let bootstrapNeeded = $state(false);

  // Fetch auth status and populate authStore. Returns true if a user was resolved.
  async function loadAuthStatus(): Promise<boolean> {
    const status = await apiCall<AuthStatusDto>('auth_status', {});
    bootstrapNeeded = status.needs_bootstrap;
    authStore.placePathDisplay = normalizePlacePathDisplay(status.place_path_display);

    if (status.user) {
      // Backend returned an authenticated user (HTTP session case).
      authStore.user = {
        id: status.user.id,
        login: status.user.login,
        fullName: status.user.full_name,
        role: status.user.role as UserRole,
      };
      return true;
    }
    if (isTauri && !status.desktop_lock_enabled) {
      // D-Desktop-01: unlocked desktop (desktop_lock_enabled=false) — auto-set trusted-admin UI state.
      // D-Desktop-02: when desktop_lock_enabled=true this branch is skipped → LoginPage is shown.
      // All API calls still go through backend authorization regardless of this UI sentinel.
      authStore.user = {
        id: 0,
        login: 'desktop',
        fullName: 'Рабочий стол',
        role: 'admin',
      };
      return true;
    }
    // If desktop_lock_enabled=true or web browser: authStore.user remains null.
    return false;
  }

  // An authenticated user must never sit on the `#/login` route. This happens after an
  // AD-SSO login (the SSO button reloads the page while the hash is still `#/login`, left
  // over from client.ts's 401 redirect) — the router would then render LoginPage INSIDE the
  // Layout: sidebar + a stuck "Вход в систему". Send them to the role's default page (`#/`
  // → Dashboard for admin/manager, Заявки for employee).
  function redirectAwayFromLogin() {
    const h = window.location.hash;
    if (h === '#/login' || h === '#login' || h === '' || h === '#') {
      window.location.hash = '#/';
    }
  }

  onMount(async () => {
    try {
      let authed = await loadAuthStatus();

      // Passwordless AD SSO (spike-002/003): only in a server-mode/LAN browser, only when
      // not already authenticated and not on the first-run bootstrap screen. On success the
      // backend has issued a session cookie — re-load status to populate the real user.
      // On failure `trySilentAdSso` self-suppresses (ad_skip) and we fall through to LoginPage.
      if (!authed && !isTauri && !bootstrapNeeded) {
        const ssoOk = await trySilentAdSso();
        if (ssoOk) authed = await loadAuthStatus();
      }

      if (authed) redirectAwayFromLogin();
    } catch {
      // Ignore — apiCall 401 redirect is handled in client.ts.
    } finally {
      appLoading = false;
    }
  });
</script>

{#if appLoading}
  <div class="app-loading">Загрузка...</div>
{:else if bootstrapNeeded && !authStore.user}
  <FirstRunWizard />
{:else if !authStore.user}
  <LoginPage />
{:else if authStore.user.role === 'employee'}
  <EmployeeLayout>
    <Router routes={employeeRoutes} />
  </EmployeeLayout>
{:else}
  <Layout>
    <Router {routes} />
  </Layout>
{/if}
<ToastHost />

<style lang="scss">
  .app-loading {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 100vh;
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-secondary);
    background: var(--tr-bg);
  }
</style>
