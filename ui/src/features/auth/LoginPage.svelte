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
  let loginError = $state<string | null>(null);
  let passwordError = $state<string | null>(null);
  let serverError = $state<string | null>(null);

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

      <!-- D-UX-03: reserved space for v2 SSO. Visually muted/disabled, NO
           click handler, NO fabricated display name (UI-SPEC Screen 1). -->
      <Button type="button" variant="ghost" disabled>Вход по учётной записи Windows (скоро)</Button>
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
