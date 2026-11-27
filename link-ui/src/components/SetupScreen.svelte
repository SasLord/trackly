<script module lang="ts">
  /** Тримит и удаляет пробелы и переводы строк внутри (UI-SPEC §6.3 п.3). */
  export function normalizeCode(raw: string): string {
    return raw.replace(/\s+/g, '');
  }
</script>

<script lang="ts">
  // UI-SPEC §6.3 (D-03): первая настройка по коду приглашения.
  // Кнопки принятия отпечатка и ручного ввода отпечатка нет: пин берётся только из кода.
  import AuthShell from '@trackly/kit/components/AuthShell.svelte';
  import FormField from '@trackly/kit/components/FormField.svelte';
  import Input from '@trackly/kit/components/Input.svelte';
  import Button from '@trackly/kit/components/Button.svelte';
  import LinkLogo from './LinkLogo.svelte';
  import AdminPinBlock from './AdminPinBlock.svelte';
  import { submitInvite, quit } from '../lib/linkApi';

  interface PinDetails {
    server: string;
    expected: string;
    got: string;
  }

  interface Props {
    showQuit?: boolean;
    /** Сведения для администратора, если ядро их передало при несовпадении отпечатка. */
    pinDetails?: PinDetails | null;
  }

  const { showQuit = false, pinDetails = null }: Props = $props();

  let code = $state('');
  let loading = $state(false);
  let fieldError = $state<string | null>(null);
  let pinFailed = $state(false);

  async function submit() {
    fieldError = null;
    pinFailed = false;
    const value = normalizeCode(code);
    if (!value) {
      fieldError = 'Введите код приглашения';
      return;
    }
    loading = true;
    try {
      await submitInvite(value);
      // успех без тоста: ядро переведёт состояние по автомату
    } catch (e) {
      const err = e as { code: string; message: string };
      if (err.code === 'pin_mismatch') {
        pinFailed = true;
      } else {
        fieldError = err.message;
      }
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    document.getElementById('invite-code')?.focus();
  });
</script>

<AuthShell maxWidth={400}>
  <div class="logo"><LinkLogo /></div>
  <h1 class="title" tabindex="-1">Подключение к Trackly</h1>
  <p class="lead">
    Введите код приглашения. Его выдаёт администратор Trackly. Код нужен один раз — для подключения
    этого компьютера.
  </p>
  <form
    class="form"
    onsubmit={(e) => {
      e.preventDefault();
      submit();
    }}
  >
    <FormField
      label="Код приглашения"
      id="invite-code"
      error={fieldError}
      hint="Вставьте код целиком — так, как его прислал администратор."
    >
      {#snippet children({ describedBy, invalid })}
        <!-- spellcheck наследуется от предка; сам Input его не пробрасывает -->
        <div spellcheck="false">
          <Input
            id="invite-code"
            type="text"
            mono
            bind:value={code}
            disabled={loading}
            {invalid}
            aria-describedby={describedBy}
            autocomplete="off"
          />
        </div>
      {/snippet}
    </FormField>

    {#if pinFailed}
      <div class="callout" role="alert">
        Сервер не прошёл проверку подлинности. Ничего не сохранено. Обратитесь к администратору.
      </div>
      {#if pinDetails}
        <AdminPinBlock {...pinDetails} />
      {/if}
    {/if}

    <Button type="submit" variant="primary" {loading}>Подключить</Button>
  </form>
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

  .lead {
    margin: 0 0 var(--tr-space-md);
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-secondary);
  }

  .form {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-md);
  }

  .callout {
    padding: var(--tr-space-sm) var(--tr-space-md);
    background: var(--tr-danger-soft);
    color: var(--tr-danger-text);
    border-radius: var(--tr-radius-md);
    font-size: var(--tr-font-size-body);
  }

  .quit {
    display: flex;
    justify-content: center;
    margin-top: var(--tr-space-md);
  }
</style>
