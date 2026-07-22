<script lang="ts">
  import { onMount } from 'svelte';
  import Button from '$lib/components/Button.svelte';
  // Plan 28-12 (GAP-1): Select (нативный <select>) заменён на кастомный Dropdown
  // (flat + variant="select") — Dropdown не принимает `id`/`for`, поэтому
  // подпись оборачивает поле (implicit label), как в CartridgeFormBody.svelte
  // (Phase 27-G1 precedent).
  import Dropdown from '$lib/components/Dropdown.svelte';
  import Input from '$lib/components/Input.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { apiCall } from '$lib/api/client';

  interface BackupConfigDto {
    backup_folder: string | null;
    schedule: string;
    retention: number;
  }

  interface BackupResult {
    timestamp_utc: number;
    file_path: string;
  }

  let backupFolder = $state<string | null>(null);
  let schedule = $state('');
  let retention = $state(7);

  // GAP-1: опции для Dropdown (flat + variant="select") — «Расписание».
  const SCHEDULE_OPTIONS = [
    { id: '', label: 'Отключено' },
    { id: 'daily', label: 'Ежедневно' },
    { id: 'weekly', label: 'Еженедельно' },
  ];
  const scheduleLabel = $derived(
    SCHEDULE_OPTIONS.find((o) => o.id === schedule)?.label ?? '',
  );
  // Плоские опции без drill-in — onExpandGroup никогда реально не вызывается
  // (isGroupExpandable всегда false), но Dropdown требует типизированную
  // функцию, чтобы вывести TMember (иначе `() => []` выводит `never[]`).
  function noExpandSchedule(): { id: string; label: string }[] {
    return [];
  }
  let lastBackupTime = $state<string | null>(null);
  let backingUp = $state(false);
  let savingConfig = $state(false);
  let pickingFolder = $state(false);

  onMount(async () => {
    try {
      const cfg = await apiCall<BackupConfigDto>('settings_get_backup_config', {});
      backupFolder = cfg.backup_folder;
      schedule = cfg.schedule === 'disabled' ? '' : (cfg.schedule ?? '');
      retention = cfg.retention ?? 7;
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
      lastBackupTime = new Date(result.timestamp_utc * 1000).toLocaleString('ru-RU');
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
      await apiCall<void>('settings_save_backup_config', { patch: { backup_folder: selected } });
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
      await apiCall<void>('settings_save_backup_config', {
        patch: { schedule: schedule === '' ? 'disabled' : schedule, retention },
      });
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
          <code class="folder-code tr-mono">{backupFolder ?? 'Не выбрана'}</code>
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
        <label class="config-label dropdown-label">
          <span>Расписание</span>
          <div class="select-shrink">
            <Dropdown
              variant="select"
              flat={true}
              value={scheduleLabel}
              placeholder="Отключено"
              searchPlaceholder="Поиск"
              disabled={!backupFolder}
              loading={false}
              groups={SCHEDULE_OPTIONS}
              getGroupId={(o) => o.id}
              getGroupName={(o) => o.label}
              getGroupCount={() => 0}
              isGroupExpandable={() => false}
              isGroupSelected={(o) => o.id === schedule}
              onExpandGroup={noExpandSchedule}
              getMemberId={(o) => o.id}
              getMemberName={(o) => o.label}
              onSearch={() => {}}
              onPickGroup={(o) => (schedule = o.id)}
              onPickMember={() => {}}
            />
          </div>
        </label>
        {#if !backupFolder}
          <p class="helper-text">Выберите папку для активации автобэкапа</p>
        {/if}
      </div>

      <!-- Retention -->
      <div class="config-row">
        <label class="config-label" for="backup-retention">Ретенция</label>
        <div class="input-group">
          <div class="input-shrink">
            <Input
              id="backup-retention"
              type="number"
              value={String(retention)}
              oninput={(v) => (retention = Number(v) || 1)}
            />
          </div>
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
    background: var(--tr-surface);
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-md);
    padding: var(--tr-space-xl);
    max-width: 640px;
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-xl);
  }

  .section-title {
    margin: 0 0 0;
    font-size: var(--tr-font-size-h3);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-primary);
  }

  .subsection {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-xs);
    border-top: 1px solid var(--tr-border);
    padding-top: var(--tr-space-md);

    &:first-of-type {
      border-top: none;
      padding-top: 0;
    }
  }

  .subsection-title {
    margin: 0;
    font-size: var(--tr-font-size-body);
    font-weight: var(--tr-font-weight-medium);
    color: var(--tr-text-secondary);
  }

  .manual-row {
    display: flex;
    align-items: center;
    gap: var(--tr-space-md);
    flex-wrap: wrap;
  }

  .last-backup-label {
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-tertiary);
  }

  .config-grid {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-md);
  }

  .config-row {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-2xs);
  }

  .config-label {
    font-size: var(--tr-font-size-label);
    font-weight: var(--tr-font-weight-medium);
    color: var(--tr-text-secondary);
  }

  // Plan 28-12 (GAP-1): Dropdown не принимает `id`, поэтому подпись оборачивает
  // поле (implicit label) вместо `for`/`id` association.
  .dropdown-label {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-2xs);
  }

  .folder-display {
    display: flex;
    align-items: center;
    gap: var(--tr-space-xs);
    flex-wrap: wrap;
  }

  .folder-code {
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 300px;
    background: var(--tr-surface-sunken);
    padding: var(--tr-space-2xs) var(--tr-space-xs);
    border-radius: var(--tr-radius-xs);
    display: inline-block;
  }

  .select-shrink {
    width: fit-content;
    min-width: 180px;
  }

  .input-group {
    display: flex;
    align-items: center;
    gap: var(--tr-space-xs);
  }

  .input-shrink {
    width: 80px;
  }

  .input-suffix {
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-secondary);
  }

  .save-row {
    display: flex;
    margin-top: var(--tr-space-xs);
  }

  .helper-text {
    margin: 0;
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-tertiary);
  }
</style>
