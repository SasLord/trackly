<script lang="ts">
  import { apiCall } from '$lib/api/client';
  import { authStore } from '$lib/stores/auth.svelte';
  import type { UserDto } from '../../bindings';
  import type { UserRole } from '$lib/stores/auth.svelte';
  import AuthShell from '$lib/components/AuthShell.svelte';
  import FormField from '$lib/components/FormField.svelte';
  import Input from '$lib/components/Input.svelte';
  import Button from '$lib/components/Button.svelte';

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

<AuthShell maxWidth={400}>
  <h1 class="wizard-title">Добро пожаловать в Trackly</h1>
  <p class="wizard-subtitle">Создайте учётную запись администратора</p>

  <form
    class="wizard-form"
    onsubmit={(e) => {
      e.preventDefault();
      handleSubmit();
    }}
  >
    <FormField label="Логин" id="wiz-login" error={loginErr}>
      {#snippet children({ describedBy, invalid })}
        <Input
          id="wiz-login"
          type="text"
          placeholder="Логин (не менее 3 символов)"
          bind:value={login}
          disabled={loading}
          {invalid}
          aria-describedby={describedBy}
          autocomplete="username"
        />
      {/snippet}
    </FormField>

    <FormField label="Полное имя" id="wiz-fullname" error={fullNameErr}>
      {#snippet children({ describedBy, invalid })}
        <Input
          id="wiz-fullname"
          type="text"
          placeholder="Иванов Иван Иванович"
          bind:value={fullName}
          disabled={loading}
          {invalid}
          aria-describedby={describedBy}
          autocomplete="name"
        />
      {/snippet}
    </FormField>

    <FormField label="Пароль" id="wiz-password" error={passwordErr}>
      {#snippet children({ describedBy, invalid })}
        <Input
          id="wiz-password"
          type="password"
          placeholder="Не менее 8 символов"
          bind:value={password}
          disabled={loading}
          {invalid}
          aria-describedby={describedBy}
          autocomplete="new-password"
        />
      {/snippet}
    </FormField>

    <FormField label="Подтвердите пароль" id="wiz-confirm" error={confirmErr}>
      {#snippet children({ describedBy, invalid })}
        <Input
          id="wiz-confirm"
          type="password"
          placeholder="Повторите пароль"
          bind:value={confirmPassword}
          disabled={loading}
          {invalid}
          aria-describedby={describedBy}
          autocomplete="new-password"
        />
      {/snippet}
    </FormField>

    {#if error}
      <div class="server-error">{error}</div>
    {/if}

    <Button type="submit" variant="primary" {loading}>Создать и войти</Button>
  </form>
</AuthShell>

<style lang="scss">
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

  .server-error {
    padding: var(--tr-space-xs) var(--tr-space-md);
    background: color-mix(in srgb, var(--tr-danger) 10%, transparent);
    border: 1px solid color-mix(in srgb, var(--tr-danger) 30%, transparent);
    border-radius: var(--tr-radius-xs);
    font-size: var(--tr-font-size-body);
    color: var(--tr-danger);
  }
</style>
