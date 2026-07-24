<script lang="ts">
  // Phase 9 Plan 05 — Screen 3 (UI-SPEC), reworked by 09-AD-GAPS
  // restoration-flow UX gap-closure. Shown after `auth_login` returns
  // AppError code ACCESS_BLOCKED (blocked/soft-deleted AD user, D-REG-03).
  //
  // Plain login is now READ-ONLY: it no longer creates a restoration
  // request. `AuthService::on_ad_bind_success`'s blocked branch only
  // READS the state of the user's most recent restore request and returns
  // it via `AppError::AccessBlocked { pending, rejection_reason }`
  // (LoginPage passes those through as `blockedDetails`). This screen
  // renders one of three states based on that payload:
  //
  // - `pending === true` → an open restore request already exists.
  //   "Запрос на рассмотрении" — no new request, no create button.
  // - `pending === false` AND `rejection_reason` set → the most recent
  //   request was rejected. Shows the reason + a «Запросить снова» CTA.
  // - neither → no restore request exists yet. Shows «Запросить
  //   восстановление доступа» CTA (first-time request).
  //
  // The CTA calls the EXPLICIT `request_ad_restore` endpoint (NOT
  // `auth_login` — that path is read-only now). `request_ad_restore`
  // re-binds to AD with the same credentials (proves identity — the user
  // has no session) and idempotently creates/reuses an open restore
  // request (09-AD-GAPS Defect 1 fix's idempotent INSERT, reused here).
  import { apiCall } from '$lib/api/client';
  import { pushToast } from '$lib/stores/toast.svelte';
  import type { AccessBlockedDetails, AppError } from '$lib/api/errors';
  import AuthShell from '$lib/components/AuthShell.svelte';
  import Button from '$lib/components/Button.svelte';

  interface Props {
    login: string;
    password: string;
    blockedDetails: AccessBlockedDetails;
    onBackToLogin: () => void;
  }

  const { login, password, blockedDetails, onBackToLogin }: Props = $props();

  let submitted = $state(false);
  let submitting = $state(false);
  let serverError = $state<string | null>(null);

  async function handleRestoreRequest() {
    submitting = true;
    serverError = null;
    try {
      await apiCall<null>('request_ad_restore', { req: { login, password } });
      submitted = true;
      pushToast('success', 'Запрос на восстановление отправлен');
    } catch (e: unknown) {
      const err = e as Partial<AppError> | undefined;
      serverError =
        err && typeof err.message === 'string'
          ? err.message
          : 'Не удалось отправить запрос. Попробуйте позже.';
    } finally {
      submitting = false;
    }
  }
</script>

<AuthShell stack>
  {#if submitted}
    <h1 class="login-title">Запрос отправлен</h1>
    <p class="screen-body">
      Запрос на восстановление доступа отправлен администратору. Доступ появится после
      подтверждения.
    </p>
    <Button variant="link" onclick={onBackToLogin}>Войти под другим пользователем</Button>
  {:else if blockedDetails.pending}
    <h1 class="login-title">Запрос на рассмотрении</h1>
    <p class="screen-body">
      Ваш запрос на восстановление доступа уже отправлен администратору и ожидает решения. Повторно
      отправлять его не нужно.
    </p>
    <Button variant="link" onclick={onBackToLogin}>Войти под другим пользователем</Button>
  {:else if blockedDetails.rejection_reason !== null}
    <h1 class="login-title">Запрос отклонён</h1>
    <p class="screen-body">
      Запрос на восстановление доступа отклонён. Причина: {blockedDetails.rejection_reason}
    </p>
    {#if serverError}
      <div class="server-error">{serverError}</div>
    {/if}
    <Button variant="primary" loading={submitting} onclick={handleRestoreRequest}>
      Запросить снова
    </Button>
    <Button variant="link" onclick={onBackToLogin}>Войти под другим пользователем</Button>
  {:else}
    <h1 class="login-title">Доступ закрыт</h1>
    <p class="screen-body">
      Ваша учётная запись отключена. Вы можете запросить восстановление доступа у администратора.
    </p>
    {#if serverError}
      <div class="server-error">{serverError}</div>
    {/if}
    <Button variant="primary" loading={submitting} onclick={handleRestoreRequest}>
      Запросить восстановление доступа
    </Button>
    <Button variant="link" onclick={onBackToLogin}>Войти под другим пользователем</Button>
  {/if}
</AuthShell>

<style lang="scss">
  .login-title {
    margin: 0;
    font-size: var(--tr-font-size-h3);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-primary);
    text-align: center;
  }

  .screen-body {
    margin: 0;
    font-size: var(--tr-font-size-body);
    line-height: var(--tr-line-height-body);
    color: var(--tr-text-secondary);
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
