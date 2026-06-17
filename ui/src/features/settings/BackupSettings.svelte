<script lang="ts">
  import { onMount } from 'svelte';
  import Button from '$lib/components/Button.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { apiCall } from '$lib/api/client';

  interface BackupConfigDto {
    backup_folder: string | null;
    schedule: string;
    retention: number;
    last_backup_time: string | null;
  }

  interface BackupResult {
    timestamp: number;
  }

  let backupFolder = $state<string | null>(null);
  let schedule = $state('');
  let retention = $state(7);
  let lastBackupTime = $state<string | null>(null);
  let backingUp = $state(false);
  let savingConfig = $state(false);
  let pickingFolder = $state(false);

  onMount(async () => {
    try {
      const cfg = await apiCall<BackupConfigDto>('settings_get_backup_config', {});
      backupFolder = cfg.backup_folder;
      schedule = cfg.schedule ?? '';
      retention = cfg.retention ?? 7;
      lastBackupTime = cfg.last_backup_time;
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось загрузить настройки бэкапа';
      pushToast('error', msg);
    }
  });

  async function runManualBackup() {
    if (!backupFolder) {
      pushToast('error', 'Выберите папку для резервных копий');
      return;
    }
    backingUp = true;
    try {
      const result = await apiCall<BackupResult>('backup_run_manual', {});
      lastBackupTime = new Date(result.timestamp * 1000).toLocaleString('ru-RU');
      pushToast('success', 'Резервная копия создана');
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Ошибка создания резервной копии';
      pushToast('error', `Резервная копия не создана: ${msg}. Проверьте путь к папке.`);
    } finally {
      backingUp = false;
    }
  }

  async function pickFolder() {
    const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

    if (!isTauri) {
      pushToast('error', 'Выбор папки доступен только в десктоп-приложении.');
      return;
    }

    pickingFolder = true;
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const path = await open({ directory: true, multiple: false });
      if (!path) return;
      const selected = typeof path === 'string' ? path : (path as string[])[0];
      // Persist folder selection immediately
      await apiCall<void>('settings_save_backup_config', { backup_folder: selected });
      backupFolder = selected;
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось выбрать папку';
      pushToast('error', msg);
    } finally {
      pickingFolder = false;
    }
  }

  async function saveConfig() {
    savingConfig = true;
    try {
      await apiCall<void>('settings_save_backup_config', { schedule, retention });
      pushToast('success', 'Настройки бэкапа сохранены');
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось сохранить настройки бэкапа';
      pushToast('error', msg);
    } finally {
      savingConfig = false;
    }
  }
</script>

<section class="settings-section">
  <h2 class="section-title">Бэкапы</h2>

  <!-- Manual backup -->
  <div class="subsection">
    <h3 class="subsection-title">Резервная копия вручную</h3>
    <div class="manual-row">
      <Button variant="secondary" loading={backingUp} onclick={runManualBackup}>
        Создать резервную копию
      </Button>
      {#if lastBackupTime}
        <span class="last-backup-label">Последний бэкап: {lastBackupTime}</span>
      {/if}
    </div>
  </div>

  <!-- Auto-backup config -->
  <div class="subsection">
    <h3 class="subsection-title">Автоматическое резервное копирование</h3>

    <div class="config-grid">
      <!-- Backup folder -->
      <div class="config-row">
        <span class="config-label">Папка</span>
        <div class="folder-display">
          <code class="folder-code">{backupFolder ?? 'Не выбрана'}</code>
          <Button variant="secondary" size="sm" loading={pickingFolder} onclick={pickFolder}>
            Выбрать папку
          </Button>
        </div>
        {#if !backupFolder}
          <p class="helper-text">Выберите папку для активации автобэкапа</p>
        {/if}
      </div>

      <!-- Schedule -->
      <div class="config-row">
        <label class="config-label" for="backup-schedule">Расписание</label>
        <select
          id="backup-schedule"
          class="form-select"
          bind:value={schedule}
          disabled={!backupFolder}
        >
          <option value="">Отключено</option>
          <option value="daily">Ежедневно</option>
          <option value="weekly">Еженедельно</option>
        </select>
        {#if !backupFolder}
          <p class="helper-text">Выберите папку для активации автобэкапа</p>
        {/if}
      </div>

      <!-- Retention -->
      <div class="config-row">
        <label class="config-label" for="backup-retention">Ретенция</label>
        <div class="input-group">
          <input
            id="backup-retention"
            class="form-input"
            type="number"
            min="1"
            max="99"
            bind:value={retention}
          />
          <span class="input-suffix">копий</span>
        </div>
      </div>
    </div>

    <div class="save-row">
      <Button variant="primary" loading={savingConfig} onclick={saveConfig}>
        Сохранить настройки бэкапа
      </Button>
    </div>
  </div>
</section>

<style lang="scss">
  .settings-section {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    padding: var(--space-lg);
    max-width: 640px;
    display: flex;
    flex-direction: column;
    gap: var(--space-lg);
  }

  .section-title {
    margin: 0 0 0;
    font-size: var(--font-size-heading);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
  }

  .subsection {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
    border-top: 1px solid var(--color-border);
    padding-top: var(--space-md);

    &:first-of-type {
      border-top: none;
      padding-top: 0;
    }
  }

  .subsection-title {
    margin: 0;
    font-size: var(--font-size-body);
    font-weight: var(--font-weight-medium);
    color: var(--color-text-secondary);
  }

  .manual-row {
    display: flex;
    align-items: center;
    gap: var(--space-md);
    flex-wrap: wrap;
  }

  .last-backup-label {
    font-size: var(--font-size-label);
    color: var(--color-text-muted);
  }

  .config-grid {
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
  }

  .config-row {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }

  .config-label {
    font-size: var(--font-size-label);
    font-weight: var(--font-weight-medium);
    color: var(--color-text-secondary);
  }

  .folder-display {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    flex-wrap: wrap;
  }

  .folder-code {
    font-family: monospace;
    font-size: var(--font-size-label);
    color: var(--color-text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 300px;
    background: var(--color-surface-sunken);
    padding: var(--space-xs) var(--space-sm);
    border-radius: var(--radius-sm);
    display: inline-block;
  }

  .form-select {
    padding: var(--space-sm) var(--space-md);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    font-size: var(--font-size-body);
    background: var(--color-bg);
    color: var(--color-text-primary);
    width: fit-content;
    min-width: 180px;

    &:focus {
      outline: none;
      border-color: var(--color-accent);
      box-shadow: 0 0 0 2px color-mix(in srgb, var(--color-accent) 20%, transparent);
    }

    &:disabled {
      opacity: 0.6;
      cursor: not-allowed;
    }
  }

  .input-group {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
  }

  .form-input {
    width: 80px;
    padding: var(--space-sm) var(--space-md);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    font-size: var(--font-size-body);
    background: var(--color-bg);
    color: var(--color-text-primary);
    text-align: right;

    &:focus {
      outline: none;
      border-color: var(--color-accent);
      box-shadow: 0 0 0 2px color-mix(in srgb, var(--color-accent) 20%, transparent);
    }
  }

  .input-suffix {
    font-size: var(--font-size-body);
    color: var(--color-text-secondary);
  }

  .save-row {
    display: flex;
    margin-top: var(--space-sm);
  }

  .helper-text {
    margin: 0;
    font-size: var(--font-size-label);
    color: var(--color-text-muted);
  }
</style>
