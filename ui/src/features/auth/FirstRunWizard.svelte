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
        userNew: {
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

    <form
      class="wizard-form"
      onsubmit={(e) => {
        e.preventDefault();
        handleSubmit();
      }}
    >
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
    background: var(--tr-bg);
  }

  .wizard-card {
    background: var(--tr-surface);
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-lg);
    padding: var(--tr-space-2xl) var(--tr-space-4xl, 2rem);
    width: 100%;
    max-width: 400px;
    box-shadow: var(--tr-elev-2);
  }

  .wizard-title {
    margin: 0 0 var(--tr-space-2xs);
    font-size: var(--tr-font-size-h3);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-primary);
    text-align: center;
  }

  .wizard-subtitle {
    margin: 0 0 var(--tr-space-xl);
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-secondary);
    text-align: center;
  }

  .wizard-form {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-md);
  }

  .form-field {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-2xs);
  }

  .form-label {
    font-size: var(--tr-font-size-label);
    font-weight: var(--tr-font-weight-medium);
    color: var(--tr-text-secondary);
  }

  .form-input {
    padding: var(--tr-space-xs) var(--tr-space-md);
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-xs);
    font-size: var(--tr-font-size-body);
    background: var(--tr-bg);
    color: var(--tr-text-primary);

    &:focus {
      outline: none;
      border-color: var(--tr-accent);
      box-shadow: 0 0 0 2px color-mix(in srgb, var(--tr-accent) 20%, transparent);
    }

    &.is-error {
      border-color: var(--tr-danger);
    }

    &:disabled {
      opacity: 0.6;
      cursor: not-allowed;
    }
  }

  .field-error {
    font-size: var(--tr-font-size-label);
    color: var(--tr-danger);
  }

  .server-error {
    padding: var(--tr-space-xs) var(--tr-space-md);
    background: color-mix(in srgb, var(--tr-danger) 10%, transparent);
    border: 1px solid color-mix(in srgb, var(--tr-danger) 30%, transparent);
    border-radius: var(--tr-radius-xs);
    font-size: var(--tr-font-size-body);
    color: var(--tr-danger);
  }

  .btn-submit {
    margin-top: var(--tr-space-2xs);
    padding: var(--tr-space-xs) var(--tr-space-md);
    background: var(--tr-accent);
    color: var(--tr-text-inverse);
    border: none;
    border-radius: var(--tr-radius-xs);
    font-size: var(--tr-font-size-body);
    font-weight: var(--tr-font-weight-medium);
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
