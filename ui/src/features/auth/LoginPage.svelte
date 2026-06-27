<script lang="ts">
  import { apiCall } from '$lib/api/client';
  import { authStore } from '$lib/stores/auth.svelte';
  import type { UserDto } from '../../bindings';
  import type { UserRole } from '$lib/stores/auth.svelte';
  import type { AccessBlockedDetails, AppError } from '$lib/api/errors';
  import PendingScreen from './PendingScreen.svelte';
  import BlockedScreen from './BlockedScreen.svelte';

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
  <div class="login-container">
    <div class="login-card">
      <h1 class="login-title">Вход в систему</h1>
      <form
        class="login-form"
        onsubmit={(e) => {
          e.preventDefault();
          handleSubmit();
        }}
      >
        <div class="form-field">
          <label class="form-label" for="login-input">Логин</label>
          <input
            id="login-input"
            class="form-input"
            class:is-error={loginError !== null}
            type="text"
            placeholder="Логин"
            bind:value={login}
            disabled={loading}
            autocomplete="username"
          />
          {#if loginError}
            <span class="field-error">{loginError}</span>
          {:else}
            <span class="format-hint">Логин: us100, user@domain или DOMAIN\user</span>
          {/if}
        </div>

        <div class="form-field">
          <label class="form-label" for="password-input">Пароль</label>
          <input
            id="password-input"
            class="form-input"
            class:is-error={passwordError !== null}
            type="password"
            placeholder="Пароль"
            bind:value={password}
            disabled={loading}
            autocomplete="current-password"
          />
          {#if passwordError}
            <span class="field-error">{passwordError}</span>
          {/if}
        </div>

        <label class="checkbox-label">
          <input type="checkbox" bind:checked={remember} disabled={loading} />
          <span class="checkbox-text">Запомнить меня</span>
        </label>

        {#if serverError}
          <div class="server-error">{serverError}</div>
        {/if}

        <button class="btn-submit" type="submit" disabled={loading}>
          {#if loading}Вход...{:else}Войти{/if}
        </button>

        <!-- D-UX-03: reserved space for v2 SSO. Visually muted/disabled, NO
             click handler, NO fabricated display name (UI-SPEC Screen 1). -->
        <button class="btn-sso-reserved" type="button" disabled tabindex="-1">
          Вход по учётной записи Windows (скоро)
        </button>
      </form>
    </div>
  </div>
{/if}

<style lang="scss">
  .login-container {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 100vh;
    background: var(--color-bg);
  }

  .login-card {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    padding: var(--space-xl) var(--space-2xl, 2rem);
    width: 100%;
    max-width: 360px;
    box-shadow: 0 2px 12px rgba(0, 0, 0, 0.08);
  }

  .login-title {
    margin: 0 0 var(--space-lg);
    font-size: var(--font-size-heading);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
    text-align: center;
  }

  .login-form {
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
  }

  .form-field {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }

  .form-label {
    font-size: var(--font-size-label);
    font-weight: var(--font-weight-medium);
    color: var(--color-text-secondary);
  }

  .form-input {
    padding: var(--space-sm) var(--space-md);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    font-size: var(--font-size-body);
    background: var(--color-bg);
    color: var(--color-text-primary);

    &:focus {
      outline: none;
      border-color: var(--color-accent);
      box-shadow: 0 0 0 2px color-mix(in srgb, var(--color-accent) 20%, transparent);
    }

    &.is-error {
      border-color: var(--color-error, #c0392b);
    }

    &:disabled {
      opacity: 0.6;
      cursor: not-allowed;
    }
  }

  .field-error {
    font-size: var(--font-size-label);
    color: var(--color-error, #c0392b);
  }

  .format-hint {
    font-size: var(--font-size-label);
    color: var(--color-text-muted);
    line-height: var(--line-height-label);
  }

  .checkbox-label {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    cursor: pointer;

    input[type='checkbox'] {
      width: 16px;
      height: 16px;
      accent-color: var(--color-accent);
      cursor: pointer;
    }
  }

  .checkbox-text {
    font-size: var(--font-size-label);
    color: var(--color-text-secondary);
  }

  .btn-sso-reserved {
    margin-top: var(--space-xs);
    padding: var(--space-sm) var(--space-md);
    background: var(--color-surface-sunken);
    color: var(--color-text-muted);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    font-size: var(--font-size-body);
    font-weight: var(--font-weight-medium);
    cursor: not-allowed;
  }

  .server-error {
    padding: var(--space-sm) var(--space-md);
    background: color-mix(in srgb, var(--color-error, #c0392b) 10%, transparent);
    border: 1px solid color-mix(in srgb, var(--color-error, #c0392b) 30%, transparent);
    border-radius: var(--radius-sm);
    font-size: var(--font-size-body);
    color: var(--color-error, #c0392b);
  }

  .btn-submit {
    margin-top: var(--space-xs);
    padding: var(--space-sm) var(--space-md);
    background: var(--color-accent);
    color: var(--color-text-inverse, #fff);
    border: none;
    border-radius: var(--radius-sm);
    font-size: var(--font-size-body);
    font-weight: var(--font-weight-medium);
    cursor: pointer;
    transition: opacity 0.1s;

    &:hover:not(:disabled) {
      opacity: 0.9;
    }

    &:disabled {
      opacity: 0.6;
      cursor: not-allowed;
    }
  }
</style>
