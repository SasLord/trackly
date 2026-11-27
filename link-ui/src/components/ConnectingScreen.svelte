<script lang="ts">
  // UI-SPEC §6.2: экран «Подключение…» (старт и автовход). Кнопок нет.
  import AuthShell from '@trackly/kit/components/AuthShell.svelte';
  import Spinner from '@trackly/kit/components/Spinner.svelte';
  import LinkLogo from './LinkLogo.svelte';

  interface Props {
    /** Что делаем сейчас: проверяем сервер или выполняем автовход. */
    phase?: 'server' | 'sso';
    /** Адрес сервера, если уже известен. */
    server?: string;
    /** Сервер молчит дольше 15 с. */
    slow?: boolean;
  }

  const { phase = 'server', server, slow = false }: Props = $props();
</script>

<AuthShell maxWidth={400}>
  <div class="logo"><LinkLogo /></div>
  <div class="body" role="status" aria-live="polite">
    <Spinner size="lg" />
    <p class="text">
      {phase === 'sso' ? 'Выполняется автоматический вход…' : 'Подключение к серверу…'}
    </p>
    {#if slow}
      <p class="caption">Сервер не отвечает. Link продолжает попытки.</p>
    {/if}
  </div>
  {#if server}
    <p class="server">Сервер: {server}</p>
  {/if}
</AuthShell>

<style lang="scss">
  .logo {
    display: flex;
    justify-content: center;
    margin-bottom: var(--tr-space-md);
  }

  .body {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--tr-space-sm);
  }

  .text {
    margin: 0;
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-secondary);
    text-align: center;
  }

  .caption,
  .server {
    margin: 0;
    font-size: var(--tr-font-size-caption);
    color: var(--tr-text-tertiary);
    text-align: center;
  }

  .server {
    margin-top: var(--tr-space-md);
  }
</style>
