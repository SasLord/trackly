<script lang="ts">
  import { apiCall } from '$lib/api/client';
  import { authStore } from '$lib/stores/auth.svelte';
  import type { UserDto } from '../../bindings';
  import type { UserRole } from '$lib/stores/auth.svelte';

  let login = $state('');
  let password = $state('');
  let loading = $state(false);
  let loginError = $state<string | null>(null);
  let passwordError = $state<string | null>(null);
  let serverError = $state<string | null>(null);

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
      const user = await apiCall<UserDto>('auth_login', { req: { login: login.trim(), password } });
      authStore.user = {
        id: user.id,
        login: user.login,
        fullName: user.full_name,
        role: user.role as UserRole,
      };
      window.location.hash = '#/';
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Неверный логин или пароль';
      serverError = msg;
    } finally {
      loading = false;
    }
  }
</script>

<div class="login-container">
  <div class="login-card">
    <h1 class="login-title">Вход в систему</h1>
    <form class="login-form" onsubmit={(e) => { e.preventDefault(); handleSubmit(); }}>
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

      {#if serverError}
        <div class="server-error">{serverError}</div>
      {/if}

      <button class="btn-submit" type="submit" disabled={loading}>
        {#if loading}Вход...{:else}Войти{/if}
      </button>
    </form>
  </div>
</div>

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
