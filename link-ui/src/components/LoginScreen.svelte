<script lang="ts">
  // UI-SPEC §6.5-§6.6: вход, «после выхода» (D-11), «отозван» (D-14).
  // Пароль отправляется только по submit формы и нигде не хранится (D-09, T-44-34, T-44-35).
  import AuthShell from '@trackly/kit/components/AuthShell.svelte';
  import FormField from '@trackly/kit/components/FormField.svelte';
  import Input from '@trackly/kit/components/Input.svelte';
  import Button from '@trackly/kit/components/Button.svelte';
  import LinkLogo from './LinkLogo.svelte';
  import { login as loginCmd, retrySso, quit } from '../lib/linkApi';
  import type { LinkState, LoginReason } from '../lib/linkState';

  interface Props {
    linkState: Extract<LinkState, { kind: 'needs_login' | 'revoked' }>;
    secretsPersistent?: boolean;
    platform?: string;
    showQuit?: boolean;
  }

  const { linkState, secretsPersistent = true, platform = '', showQuit = false }: Props = $props();

  const REASON_TEXT: Record<LoginReason, string> = {
    ip_address:
      'Сервер подключён по IP-адресу, а автовход работает только по имени сервера. Войдите по логину и паролю.',
    sso_disabled: 'Автовход выключен на сервере. Войдите по логину и паролю.',
    kerberos:
      'Компьютер не смог подтвердить вашу доменную учётную запись. Войдите по логину и паролю или повторите позже.',
    not_mapped:
      'Ваша доменная учётная запись не найдена в Trackly. Обратитесь к администратору или войдите по логину и паролю.',
    no_answer:
      'Сервер не ответил на автоматический вход. Повторите позже или войдите по логину и паролю.',
  };

  let loginValue = $state('');
  let password = $state('');
  let loading = $state(false);
  let ssoLoading = $state(false);
  let loginError = $state<string | null>(null);
  let passwordError = $state<string | null>(null);
  let formError = $state<string | null>(null);

  const isRevoked = $derived(linkState.kind === 'revoked');
  const reason = $derived(linkState.kind === 'needs_login' ? linkState.reason : null);
  const canRetrySso = $derived(linkState.kind === 'needs_login' && linkState.can_retry_sso);
  // Callout причины автовхода: был автовход и он не удался; после выхода причины нет.
  const showReason = $derived(linkState.kind === 'needs_login' && reason !== null);
  const showAfterLogout = $derived(
    linkState.kind === 'needs_login' && linkState.after_logout && platform === 'windows',
  );

  async function runSso() {
    formError = null;
    ssoLoading = true;
    try {
      await retrySso();
    } catch (e) {
      formError = (e as { message: string }).message;
    } finally {
      ssoLoading = false;
    }
  }

  async function submit() {
    loginError = null;
    passwordError = null;
    formError = null;
    let bad = false;
    if (!loginValue.trim()) {
      loginError = 'Введите логин';
      bad = true;
    }
    if (!password) {
      passwordError = 'Введите пароль';
      bad = true;
    }
    if (bad) return;

    loading = true;
    try {
      await loginCmd(loginValue.trim(), password);
    } catch (e) {
      formError = (e as { message: string }).message;
      password = '';
      // фокус в «Пароль» после снятия disabled
      queueMicrotask(() => document.getElementById('link-password')?.focus());
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    document.getElementById('link-login')?.focus();
  });
</script>

<AuthShell maxWidth={400}>
  <div class="logo"><LinkLogo /></div>
  <h1 class="title" tabindex="-1">Вход в Link</h1>

  {#if isRevoked}
    <div class="callout danger" role="alert">
      <p class="callout-title">Вход отозван администратором</p>
      <p class="callout-text">
        Этот компьютер отключён от Link. Войти снова можно после того, как администратор разрешит
        компьютер.
      </p>
    </div>
  {:else if showReason && reason}
    <div class="callout warning">
      <p class="callout-title">Автоматический вход не выполнен</p>
      <p class="callout-text">{REASON_TEXT[reason]}</p>
      {#if canRetrySso && reason !== 'ip_address' && reason !== 'sso_disabled'}
        <div class="callout-actions">
          <Button variant="secondary" size="sm" loading={ssoLoading} onclick={runSso}>
            Повторить автовход
          </Button>
        </div>
      {/if}
    </div>
  {:else if showAfterLogout}
    <div class="callout info">
      <p class="callout-text">
        Вы вышли из учётной записи. Можно войти другим логином или вернуться к автоматическому
        входу.
      </p>
      <div class="callout-actions">
        <Button variant="secondary" size="sm" loading={ssoLoading} onclick={runSso}>
          Войти автоматически
        </Button>
      </div>
    </div>
  {/if}

  <form
    class="form"
    onsubmit={(e) => {
      e.preventDefault();
      submit();
    }}
  >
    <FormField
      label="Логин"
      id="link-login"
      error={loginError}
      hint="Логин Trackly или доменный, например EXAMPLE\ivanov"
    >
      {#snippet children({ describedBy, invalid })}
        <Input
          id="link-login"
          type="text"
          bind:value={loginValue}
          disabled={loading}
          {invalid}
          aria-describedby={describedBy}
          autocomplete="username"
        />
      {/snippet}
    </FormField>

    <FormField label="Пароль" id="link-password" error={passwordError}>
      {#snippet children({ describedBy, invalid })}
        <Input
          id="link-password"
          type="password"
          bind:value={password}
          disabled={loading}
          {invalid}
          aria-describedby={describedBy}
          autocomplete="current-password"
        />
      {/snippet}
    </FormField>

    {#if formError}
      <div class="form-error" role="alert">{formError}</div>
    {/if}

    <Button type="submit" variant="primary" {loading}>Войти</Button>

    {#if !secretsPersistent}
      <p class="caption">
        Вход сохранится только до закрытия Link: в системе нет хранилища паролей.
      </p>
    {/if}
  </form>

  <p class="server">Сервер: {linkState.server}</p>
  {#if showQuit}
    <div class="quit">
      <Button variant="link" onclick={() => quit().catch(() => {})}>Закрыть Link</Button>
    </div>
  {/if}
</AuthShell>

<style lang="scss">
  .logo {
    display: flex;
    justify-content: center;
    margin-bottom: var(--tr-space-md);
  }

  .title {
    margin: 0 0 var(--tr-space-xl);
    font-size: var(--tr-font-size-h3);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-primary);
    text-align: center;
  }

  .callout {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-2xs);
    margin-bottom: var(--tr-space-md);
    padding: var(--tr-space-sm) var(--tr-space-md);
    border-radius: var(--tr-radius-md);
    font-size: var(--tr-font-size-body);

    &.warning {
      background: var(--tr-warning-soft);
      color: var(--tr-warning-text);
    }

    &.danger {
      background: var(--tr-danger-soft);
      color: var(--tr-danger-text);
    }

    &.info {
      background: var(--tr-surface-sunken);
      color: var(--tr-text-secondary);
    }
  }

  .callout-title,
  .callout-text {
    margin: 0;
  }

  .callout-title {
    font-weight: var(--tr-font-weight-semibold);
  }

  .callout-actions {
    display: flex;
    justify-content: flex-end;
    margin-top: var(--tr-space-xs);
  }

  .form {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-md);
  }

  .form-error {
    font-size: var(--tr-font-size-body);
    color: var(--tr-danger-text);
  }

  .caption,
  .server {
    margin: 0;
    font-size: var(--tr-font-size-caption);
    color: var(--tr-text-tertiary);
  }

  .server {
    margin-top: var(--tr-space-md);
    text-align: center;
  }

  .quit {
    display: flex;
    justify-content: center;
    margin-top: var(--tr-space-md);
  }
</style>
