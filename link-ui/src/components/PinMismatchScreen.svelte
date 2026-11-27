<script lang="ts">
  // UI-SPEC §6.4 (D-07): только ошибка и сведения для администратора.
  // Ни полей входа, ни кода приглашения, ни способа принять новый сертификат.
  import AuthShell from '@trackly/kit/components/AuthShell.svelte';
  import Button from '@trackly/kit/components/Button.svelte';
  import LinkLogo from './LinkLogo.svelte';
  import AdminPinBlock, { formatFingerprint } from './AdminPinBlock.svelte';
  import { recheckPin, quit } from '../lib/linkApi';

  interface Props {
    server: string;
    expected: string;
    got: string;
    showQuit?: boolean;
  }

  const { server, expected, got, showQuit = false }: Props = $props();

  let checking = $state(false);
  let copied = $state(false);
  let copyTimer: ReturnType<typeof setTimeout> | undefined;

  async function copyDetails() {
    const text = [
      `Сервер: ${server}`,
      `Ожидаемый отпечаток (SPKI): ${formatFingerprint(expected)}`,
      `Полученный отпечаток (SPKI): ${formatFingerprint(got)}`,
    ].join('\n');
    try {
      await navigator.clipboard.writeText(text);
      copied = true;
      clearTimeout(copyTimer);
      copyTimer = setTimeout(() => (copied = false), 2000);
    } catch {
      // отказ буфера: текст остаётся выделяемым в блоке, ошибку не показываем
    }
  }

  async function check() {
    checking = true;
    try {
      await recheckPin();
    } catch {
      // экран остаётся; обновлённое состояние придёт событием link-event
    } finally {
      checking = false;
    }
  }

  $effect(() => {
    document.getElementById('pin-recheck')?.focus();
  });
</script>

<AuthShell maxWidth={400}>
  <div class="logo"><LinkLogo /></div>
  <div class="icon" aria-hidden="true">
    <!-- 32px: иконка-предупреждение по UI-SPEC §6.4; размер задан атрибутом SVG, не CSS -->
    <svg viewBox="0 0 24 24" width="32" height="32" fill="none">
      <path
        d="M12 3 2 20h20L12 3Z"
        stroke="currentColor"
        stroke-width="2"
        stroke-linejoin="round"
      />
      <path d="M12 10v5" stroke="currentColor" stroke-width="2" stroke-linecap="round" />
      <circle cx="12" cy="17.5" r="1.2" fill="currentColor" />
    </svg>
  </div>
  <h1 class="title" tabindex="-1">Сертификат сервера изменился</h1>
  <p class="text" role="alert">
    Link не может безопасно подключиться к серверу и ничего ему не отправляет. Обратитесь к
    администратору Trackly.
  </p>

  <AdminPinBlock {server} {expected} {got} />

  <div class="actions">
    <Button variant="secondary" onclick={copyDetails}>
      {copied ? 'Скопировано' : 'Скопировать сведения'}
    </Button>
    <Button variant="primary" loading={checking} onclick={check}>Проверить снова</Button>
  </div>
  <p class="server">Сервер: {server}</p>
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

  .icon {
    display: flex;
    justify-content: center;
    margin-bottom: var(--tr-space-sm);
    color: var(--tr-danger);
  }

  .title {
    margin: 0 0 var(--tr-space-md);
    font-size: var(--tr-font-size-h3);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-primary);
    text-align: center;
  }

  .text {
    margin: 0 0 var(--tr-space-md);
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-primary);
  }

  .actions {
    display: flex;
    gap: var(--tr-space-xs);
    margin-top: var(--tr-space-md);

    > :global(*) {
      flex: 1;
    }
  }

  .server {
    margin: var(--tr-space-md) 0 0;
    font-size: var(--tr-font-size-caption);
    color: var(--tr-text-tertiary);
    text-align: center;
  }

  .quit {
    display: flex;
    justify-content: center;
    margin-top: var(--tr-space-md);
  }
</style>
