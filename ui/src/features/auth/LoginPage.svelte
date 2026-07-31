<script lang="ts">
  import { apiCall } from '$lib/api/client';
  import { authStore } from '$lib/stores/auth.svelte';
  import type { UserDto } from '../../bindings';
  import type { UserRole } from '$lib/stores/auth.svelte';
  import type { AccessBlockedDetails, AppError } from '$lib/api/errors';
  import PendingScreen from './PendingScreen.svelte';
  import BlockedScreen from './BlockedScreen.svelte';
  import AuthShell from '$lib/components/AuthShell.svelte';
  import FormField from '$lib/components/FormField.svelte';
  import Input from '$lib/components/Input.svelte';
  import Button from '$lib/components/Button.svelte';
  import Checkbox from '$lib/components/Checkbox.svelte';

  // D-Sec-01 / T-09-20: single generic message for ALL credential/account-state
  // failures (no enumeration). Distinct copy ONLY for infra (AD unreachable).
  const GENERIC_AUTH_ERROR = 'Неверный логин или пароль';
  const AD_UNREACHABLE_ERROR =
    'Сервер аутентификации недоступен. Повторите попытку позже или обратитесь к администратору.';

  let login = $state('');
  let password = $state('');
  let remember = $state(false);
  let loading = $state(false);
  let ssoLoading = $state(false);
  let loginError = $state<string | null>(null);
  let passwordError = $state<string | null>(null);
  let serverError = $state<string | null>(null);

  // AD SSO button is server-mode/LAN-browser only (no Negotiate flow in the Tauri desktop).
  const isTauri = typeof (window as any).__TAURI_INTERNALS__ !== 'undefined';

  // Screen routing: 'login' (default) | 'pending' | 'blocked' (D-REG / D-REG-03).
  let screen = $state<'login' | 'pending' | 'blocked'>('login');
  // ACCESS_BLOCKED details (09-AD-GAPS restoration-flow UX) — passed through
  // to BlockedScreen so it can render the three states (none / pending /
  // rejected-with-reason) without re-deriving them itself.
  let blockedDetails = $state<AccessBlockedDetails>({ pending: false, rejection_reason: null });

  function backToLogin() {
    screen = 'login';
    password = '';
    serverError = null;
  }

  // Explicit "Вход через Active Directory" — triggers the browser Negotiate handshake
  // against GET /api/v1/auth_ad_sso. On success the backend issues a session cookie and we
  // reload so App.svelte picks up the authenticated user. Clears the `ad_skip` suppression
  // cookie first so an explicit click always retries even after a silent attempt was skipped.
  async function handleAdSso() {
    serverError = null;
    ssoLoading = true;
    document.cookie = 'trackly_ad_skip=; path=/; Max-Age=0; SameSite=Lax';
    try {
      const res = await fetch('/api/v1/auth_ad_sso', { credentials: 'same-origin' });
      if (res.ok) {
        const data = (await res.json().catch(() => null)) as { ok?: boolean } | null;
        if (data?.ok) {
          window.location.reload();
          return;
        }
      }
      serverError =
        res.status === 503
          ? 'Вход через Active Directory не настроен на сервере.'
          : 'Не удалось войти через Active Directory — войдите по логину и паролю.';
    } catch {
      serverError = 'Не удалось связаться с сервером для входа через Active Directory.';
    } finally {
      ssoLoading = false;
    }
  }

  async function handleSubmit() {
    loginError = null;
    passwordError = null;
    serverError = null;

    let hasError = false;
    if (!login.trim()) {
      loginError = 'Введите логин';
      hasError = true;
    }
    if (!password) {
      passwordError = 'Введите пароль';
      hasError = true;
    }
    if (hasError) return;

    loading = true;
    try {
      const user = await apiCall<UserDto>('auth_login', {
        req: { login: login.trim(), password, remember },
      });
      authStore.user = {
        id: user.id,
        login: user.login,
        fullName: user.full_name,
        role: user.role as UserRole,
      };
      window.location.hash = '#/';
    } catch (e: unknown) {
      const err = e as Partial<AppError> | undefined;
      const code = err && typeof err === 'object' ? err.code : undefined;
      if (code === 'REGISTRATION_PENDING') {
        screen = 'pending';
      } else if (code === 'ACCESS_BLOCKED') {
        const details = (err && err.details) as Partial<AccessBlockedDetails> | undefined;
        blockedDetails = {
          pending: details?.pending === true,
          rejection_reason:
            typeof details?.rejection_reason === 'string' ? details.rejection_reason : null,
        };
        screen = 'blocked';
      } else if (code === 'SERVICE_UNAVAILABLE') {
        serverError = AD_UNREACHABLE_ERROR;
      } else {
        serverError = GENERIC_AUTH_ERROR;
      }
    } finally {
      loading = false;
    }
  }
</script>

{#if screen === 'pending'}
  <PendingScreen onBackToLogin={backToLogin} />
{:else if screen === 'blocked'}
  <BlockedScreen {login} {password} {blockedDetails} onBackToLogin={backToLogin} />
{:else}
  <AuthShell>
    <h1 class="login-title">Вход в систему</h1>
    <form
      class="login-form"
      onsubmit={(e) => {
        e.preventDefault();
        handleSubmit();
      }}
    >
      <FormField
        label="Логин"
        id="login-input"
        error={loginError}
        hint="Логин: us100, user@domain или DOMAIN\User"
      >
        {#snippet children({ describedBy, invalid })}
          <Input
            id="login-input"
            type="text"
            bind:value={login}
            disabled={loading}
            {invalid}
            aria-describedby={describedBy}
            autocomplete="username"
          />
        {/snippet}
      </FormField>

      <FormField label="Пароль" id="password-input" error={passwordError}>
        {#snippet children({ describedBy, invalid })}
          <Input
            id="password-input"
            type="password"
            bind:value={password}
            disabled={loading}
            {invalid}
            aria-describedby={describedBy}
            autocomplete="current-password"
          />
        {/snippet}
      </FormField>

      <Checkbox bind:checked={remember} disabled={loading}>Запомнить меня</Checkbox>

      {#if serverError}
        <div class="server-error">{serverError}</div>
      {/if}

      <Button type="submit" variant="primary" {loading}>Войти</Button>

      <!-- AD SSO (Kerberos/SPNEGO) — server-mode/LAN-browser only. Hidden in the Tauri
           desktop (no Negotiate flow). Triggers the transparent domain login; if there is
           no ticket / SSO is off, it just shows an error and normal login stays available. -->
      {#if !isTauri}
        <Button
          type="button"
          variant="ghost"
          loading={ssoLoading}
          disabled={loading}
          onclick={handleAdSso}
        >
          Вход через Active Directory
        </Button>
      {/if}
    </form>
  </AuthShell>
{/if}

<style lang="scss">
  .login-title {
    margin: 0 0 var(--tr-space-xl);
    font-size: var(--tr-font-size-h3);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-primary);
    text-align: center;
  }

  .login-form {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-md);
  }

  .server-error {
    padding: var(--tr-space-xs) var(--tr-space-md);
    background: color-mix(in srgb, var(--tr-danger) 10%, transparent);
    border: 1px solid color-mix(in srgb, var(--tr-danger) 30%, transparent);
    border-radius: var(--tr-radius-xs);
    font-size: var(--tr-font-size-body);
    color: var(--tr-danger);
  }
</style>
