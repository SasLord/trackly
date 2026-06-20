<script lang="ts">
  // Phase 9 Plan 05 — Screen 3 (UI-SPEC). Shown after `auth_login` returns
  // AppError code ACCESS_BLOCKED (blocked/soft-deleted AD user, D-REG-03).
  // No dedicated restoration endpoint exists (09-04-SUMMARY): a successful
  // AD bind for a blocked/soft-deleted user is what creates the restoration
  // request server-side, inside AuthService::login → create_restore_request,
  // which always returns AppError::AccessBlocked (never a session). The
  // primary CTA re-submits the SAME credentials that produced this screen —
  // create_restore_request is IDEMPOTENT per user (09-AD-GAPS Defect 1 fix):
  // repeated bind attempts (login form + this button, or repeated clicks)
  // all resolve to the SAME open request row server-side, so the explicit
  // click safely doubles as the user's confirmation action without spawning
  // duplicate requests.
  import { apiCall } from '$lib/api/client';
  import { pushToast } from '$lib/stores/toast.svelte';
  import type { AppError } from '$lib/api/errors';
  import type { UserDto } from '../../bindings';

  interface Props {
    login: string;
    password: string;
    onBackToLogin: () => void;
  }

  const { login, password, onBackToLogin }: Props = $props();

  let submitted = $state(false);
  let submitting = $state(false);
  let serverError = $state<string | null>(null);

  async function handleRestoreRequest() {
    submitting = true;
    serverError = null;
    try {
      // A successful bind here can only resolve to AppError::AccessBlocked —
      // it never returns a UserDto for a blocked/soft-deleted account. We
      // still type the call against UserDto for apiCall's generic; the
      // success branch is unreachable in practice for this screen.
      await apiCall<UserDto>('auth_login', { req: { login, password, remember: false } });
      submitted = true;
      pushToast('success', 'Запрос на восстановление отправлен');
    } catch (e: unknown) {
      const err = e as Partial<AppError> | undefined;
      if (err && err.code === 'ACCESS_BLOCKED') {
        // Expected outcome — the restore request was (re-)created server-side.
        submitted = true;
        pushToast('success', 'Запрос на восстановление отправлен');
      } else {
        serverError =
          err && typeof err.message === 'string'
            ? err.message
            : 'Не удалось отправить запрос. Попробуйте позже.';
      }
    } finally {
      submitting = false;
    }
  }
</script>

<div class="login-container">
  <div class="login-card">
    {#if submitted}
      <h1 class="login-title">Запрос отправлен</h1>
      <p class="screen-body">
        Запрос на восстановление доступа отправлен администратору. Доступ появится
        после подтверждения.
      </p>
      <button class="btn-link" type="button" onclick={onBackToLogin}>
        Войти под другим пользователем
      </button>
    {:else}
      <h1 class="login-title">Доступ закрыт</h1>
      <p class="screen-body">
        Ваша учётная запись отключена. Вы можете запросить восстановление доступа у
        администратора.
      </p>
      {#if serverError}
        <div class="server-error">{serverError}</div>
      {/if}
      <button
        class="btn-submit"
        type="button"
        disabled={submitting}
        onclick={handleRestoreRequest}
      >
        {#if submitting}Отправка…{:else}Запросить восстановление доступа{/if}
      </button>
      <button class="btn-link" type="button" onclick={onBackToLogin}>
        Войти под другим пользователем
      </button>
    {/if}
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
    text-align: center;
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
  }

  .login-title {
    margin: 0;
    font-size: var(--font-size-heading);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
    text-align: center;
  }

  .screen-body {
    margin: 0;
    font-size: var(--font-size-body);
    line-height: var(--line-height-body);
    color: var(--color-text-secondary);
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

  .btn-link {
    background: transparent;
    border: none;
    padding: 0;
    color: var(--color-accent);
    font-size: var(--font-size-body);
    cursor: pointer;

    &:hover {
      text-decoration: underline;
    }
    &:focus-visible {
      outline: none;
      box-shadow: 0 0 0 3px var(--color-accent-focus);
    }
  }
</style>
