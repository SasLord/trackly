<script lang="ts">
  import { apiCall } from '$lib/api/client';
  import { authStore } from '$lib/stores/auth.svelte';
  import type { UserDto } from '../../bindings';
  import type { UserRole } from '$lib/stores/auth.svelte';

  let login = $state('');
  let fullName = $state('');
  let password = $state('');
  let confirmPassword = $state('');
  let loading = $state(false);
  let error = $state<string | null>(null);

  // Per-field validation errors
  let loginErr = $state<string | null>(null);
  let fullNameErr = $state<string | null>(null);
  let passwordErr = $state<string | null>(null);
  let confirmErr = $state<string | null>(null);

  function validate(): boolean {
    loginErr = null;
    fullNameErr = null;
    passwordErr = null;
    confirmErr = null;
    let ok = true;

    if (login.trim().length < 3) {
      loginErr = 'Логин должен быть не менее 3 символов';
      ok = false;
    }
    if (!fullName.trim()) {
      fullNameErr = 'Введите полное имя';
      ok = false;
    }
    if (password.length < 8) {
      passwordErr = 'Пароль должен быть не менее 8 символов';
      ok = false;
    }
    if (password !== confirmPassword) {
      confirmErr = 'Пароли не совпадают';
      ok = false;
    }
    return ok;
  }

  async function handleSubmit() {
    if (!validate()) return;

    loading = true;
    error = null;
    try {
      // Create first admin user
      await apiCall<UserDto>('users_create', {
        user_new: {
          login: login.trim(),
          full_name: fullName.trim(),
          password,
          role: 'admin',
          email: null,
        },
      });

      // Auto-login after creation
      const user = await apiCall<UserDto>('auth_login', {
        req: { login: login.trim(), password },
      });

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
          : 'Не удалось создать учётную запись';
      error = msg;
    } finally {
      loading = false;
    }
  }
</script>

<div class="wizard-container">
  <div class="wizard-card">
    <h1 class="wizard-title">Добро пожаловать в Trackly</h1>
    <p class="wizard-subtitle">Создайте учётную запись администратора</p>

    <form class="wizard-form" onsubmit={(e) => { e.preventDefault(); handleSubmit(); }}>
      <div class="form-field">
        <label class="form-label" for="wiz-login">Логин</label>
        <input
          id="wiz-login"
          class="form-input"
          class:is-error={loginErr !== null}
          type="text"
          placeholder="Логин (не менее 3 символов)"
          bind:value={login}
          disabled={loading}
          autocomplete="username"
        />
        {#if loginErr}
          <span class="field-error">{loginErr}</span>
        {/if}
      </div>

      <div class="form-field">
        <label class="form-label" for="wiz-fullname">Полное имя</label>
        <input
          id="wiz-fullname"
          class="form-input"
          class:is-error={fullNameErr !== null}
          type="text"
          placeholder="Иванов Иван Иванович"
          bind:value={fullName}
          disabled={loading}
          autocomplete="name"
        />
        {#if fullNameErr}
          <span class="field-error">{fullNameErr}</span>
        {/if}
      </div>

      <div class="form-field">
        <label class="form-label" for="wiz-password">Пароль</label>
        <input
          id="wiz-password"
          class="form-input"
          class:is-error={passwordErr !== null}
          type="password"
          placeholder="Не менее 8 символов"
          bind:value={password}
          disabled={loading}
          autocomplete="new-password"
        />
        {#if passwordErr}
          <span class="field-error">{passwordErr}</span>
        {/if}
      </div>

      <div class="form-field">
        <label class="form-label" for="wiz-confirm">Подтвердите пароль</label>
        <input
          id="wiz-confirm"
          class="form-input"
          class:is-error={confirmErr !== null}
          type="password"
          placeholder="Повторите пароль"
          bind:value={confirmPassword}
          disabled={loading}
          autocomplete="new-password"
        />
        {#if confirmErr}
          <span class="field-error">{confirmErr}</span>
        {/if}
      </div>

      {#if error}
        <div class="server-error">{error}</div>
      {/if}

      <button class="btn-submit" type="submit" disabled={loading}>
        {#if loading}Создание...{:else}Создать и войти{/if}
      </button>
    </form>
  </div>
</div>

<style lang="scss">
  .wizard-container {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 100vh;
    background: var(--color-bg);
  }

  .wizard-card {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    padding: var(--space-xl) var(--space-2xl, 2rem);
    width: 100%;
    max-width: 400px;
    box-shadow: 0 2px 12px rgba(0, 0, 0, 0.08);
  }

  .wizard-title {
    margin: 0 0 var(--space-xs);
    font-size: var(--font-size-heading);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
    text-align: center;
  }

  .wizard-subtitle {
    margin: 0 0 var(--space-lg);
    font-size: var(--font-size-body);
    color: var(--color-text-secondary);
    text-align: center;
  }

  .wizard-form {
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
