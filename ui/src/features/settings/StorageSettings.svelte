<script lang="ts">
  import { onMount } from 'svelte';
  import Button from '$lib/components/Button.svelte';
  import Modal from '$lib/components/Modal.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { apiCall } from '$lib/api/client';

  let dbPath = $state('');
  let confirmMove = $state(false);
  let moving = $state(false);
  let restarting = $state(false);

  async function loadDbPath() {
    try {
      dbPath = await apiCall<string>('settings_get_db_path', {});
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось получить путь к базе данных';
      pushToast('error', msg);
    }
  }

  onMount(() => {
    loadDbPath();
  });

  async function openFolder() {
    try {
      await apiCall<void>('settings_open_db_folder', {});
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось открыть папку';
      pushToast('error', msg);
    }
  }

  async function proceedWithMove() {
    const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

    if (!isTauri) {
      pushToast('error', 'Смена расположения БД доступна только в десктоп-приложении.');
      confirmMove = false;
      return;
    }

    moving = true;
    confirmMove = false;

    try {
      const { save } = await import('@tauri-apps/plugin-dialog');
      const newPath = await save({
        defaultPath: 'trackly.db',
        filters: [{ name: 'SQLite', extensions: ['db'] }],
      });

      if (!newPath) {
        moving = false;
        return;
      }

      // Show restart overlay
      restarting = true;
      moving = false;

      await apiCall<void>('settings_move_db', { newPath });
      await apiCall<void>('app_restart', {});
    } catch (e: unknown) {
      restarting = false;
      moving = false;
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось переместить базу данных. Исходный файл не изменён.';
      pushToast('error', msg);
    }
  }
</script>

<section class="settings-section">
  <h2 class="section-title">Хранилище данных</h2>

  {#if restarting}
    <div class="restart-overlay">
      <p class="restart-message">Приложение будет перезапущено…</p>
    </div>
  {:else}
    <div class="form-field">
      <span class="form-label">Текущее расположение базы данных</span>
      <p class="db-path-display">
        <code class="db-path-code">{dbPath || 'Загрузка…'}</code>
      </p>
    </div>

    <div class="action-row">
      <Button variant="ghost" size="sm" onclick={openFolder} disabled={!dbPath}>
        Открыть папку с базой данных
      </Button>
      <Button
        variant="destructive"
        size="sm"
        onclick={() => (confirmMove = true)}
        disabled={!dbPath}
      >
        Сменить расположение
      </Button>
    </div>
  {/if}
</section>

<!-- Confirmation modal (T-07-04-03: requires desktop context + user confirmation) -->
<Modal
  open={confirmMove}
  title="Сменить расположение базы данных?"
  size="md"
  onClose={() => (confirmMove = false)}
>
  <p class="modal-body-text">
    База данных будет скопирована в новое расположение через безопасный API SQLite. Приложение
    потребует перезапуска. Сетевые подключения будут прерваны.
  </p>
  {#snippet footer()}
    <Button variant="secondary" onclick={() => (confirmMove = false)}>Отмена</Button>
    <Button variant="primary" loading={moving} onclick={proceedWithMove}>Выбрать новый путь</Button>
  {/snippet}
</Modal>

<style lang="scss">
  .settings-section {
    background: var(--tr-surface);
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-md);
    padding: var(--tr-space-xl);
    max-width: 640px;
    position: relative;
  }

  .section-title {
    margin: 0 0 var(--tr-space-md);
    font-size: var(--tr-font-size-h3);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-primary);
  }

  .form-field {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-2xs);
    margin-bottom: var(--tr-space-md);
  }

  .form-label {
    font-size: var(--tr-font-size-label);
    font-weight: var(--tr-font-weight-medium);
    color: var(--tr-text-secondary);
  }

  .db-path-display {
    margin: 0;
  }

  .db-path-code {
    font-family: monospace;
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-primary);
    word-break: break-all;
    background: var(--tr-surface-sunken);
    padding: var(--tr-space-2xs) var(--tr-space-xs);
    border-radius: var(--tr-radius-xs);
    display: inline-block;
  }

  .action-row {
    display: flex;
    gap: var(--tr-space-xs);
    flex-wrap: wrap;
    align-items: center;
  }

  .restart-overlay {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 80px;
  }

  .restart-message {
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-secondary);
    font-style: italic;
    margin: 0;
  }

  .modal-body-text {
    margin: 0;
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-primary);
    line-height: 1.5;
  }
</style>
