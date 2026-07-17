<script lang="ts">
  import { onMount } from 'svelte';
  import Button from '$lib/components/Button.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { apiCall } from '$lib/api/client';
  import type { ServerStatusDto } from '../../bindings';

  // NetworkSettingsDto — local type matching the backend DTO.
  // Not yet in bindings.ts (specta export pending regeneration).
  interface NetworkSettingsDto {
    enabled: boolean;
    host: string;
    port: number;
    cert_path: string;
    server_url: string | null;
    fingerprint: string | null;
    desktop_lock_enabled: boolean;
  }

  // Settings state (loaded from backend).
  let settings = $state<NetworkSettingsDto>({
    enabled: false,
    host: '0.0.0.0',
    port: 8443,
    cert_path: '',
    server_url: null,
    fingerprint: null,
    desktop_lock_enabled: false,
  });

  let saving = $state(false);
  let toggling = $state(false);
  let lockToggling = $state(false);
  let serverRunning = $state(false);
  let serverUrl = $state<string | null>(null);
  let serverFingerprint = $state<string | null>(null);

  async function loadSettings() {
    try {
      const s = await apiCall<NetworkSettingsDto>('settings_get_network', {});
      settings = s;
      // If server is currently running, populate the running state.
      if (s.server_url) {
        serverRunning = true;
        serverUrl = s.server_url;
        serverFingerprint = s.fingerprint ?? null;
      }
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось загрузить настройки';
      pushToast('error', msg);
    }
  }

  onMount(() => {
    loadSettings();
  });

  async function saveSettings() {
    saving = true;
    try {
      await apiCall<void>('settings_set_network', {
        patch: {
          host: settings.host,
          port: settings.port,
          cert_path: settings.cert_path,
        },
      });
      pushToast('success', 'Настройки сохранены');
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось сохранить настройки';
      pushToast('error', msg);
    } finally {
      saving = false;
    }
  }

  async function toggleServer(enable: boolean) {
    toggling = true;
    try {
      const status = await apiCall<ServerStatusDto>('server_toggle', { enable });
      serverRunning = status.running;
      serverUrl = status.url ?? null;
      serverFingerprint = status.fingerprint ?? null;
      if (enable && status.running) {
        pushToast('success', 'Сервер запущен');
      } else if (!enable) {
        pushToast('success', 'Сервер остановлен');
      }
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : enable
            ? 'Не удалось запустить сервер'
            : 'Не удалось остановить сервер';
      pushToast('error', msg);
    } finally {
      toggling = false;
    }
  }

  // D-Desktop-02: toggle desktop lock setting.
  async function toggleDesktopLock(enabled: boolean) {
    lockToggling = true;
    try {
      await apiCall<void>('desktop_set_lock', { enabled });
      settings.desktop_lock_enabled = enabled;
      pushToast('success', enabled ? 'Вход в десктопе включён' : 'Вход в десктопе отключён');
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось изменить настройку';
      pushToast('error', msg);
    } finally {
      lockToggling = false;
    }
  }

  // Format fingerprint with colons for readability.
  const formattedFingerprint = $derived(
    serverFingerprint ? serverFingerprint.replace(/(.{2})(?=.)/g, '$1:').toUpperCase() : null,
  );
</script>

<div class="network-settings">
  <!-- Section 1: Server mode -->
  <section class="settings-section">
    <h2 class="section-title">Серверный режим</h2>

    <div class="server-toggle-row">
      <span class="toggle-label">Сервер:</span>
      <div class="toggle-actions">
        {#if serverRunning}
          <span class="status-badge status-badge--running">Запущен</span>
          <Button variant="secondary" loading={toggling} onclick={() => toggleServer(false)}>
            Остановить сервер
          </Button>
        {:else}
          <span class="status-badge status-badge--stopped">Остановлен</span>
          <Button variant="primary" loading={toggling} onclick={() => toggleServer(true)}>
            Запустить сервер
          </Button>
        {/if}
      </div>
    </div>

    {#if serverRunning && serverUrl}
      <div class="server-info-block">
        <div class="info-row">
          <span class="info-label">Адрес сервера:</span>
          <a href={serverUrl} target="_blank" rel="noopener noreferrer" class="server-link">
            {serverUrl}
          </a>
        </div>
        {#if formattedFingerprint}
          <div class="info-row">
            <span class="info-label">Отпечаток сертификата:</span>
            <code class="fingerprint">{formattedFingerprint}</code>
          </div>
        {/if}
        <div class="info-instruction">
          Инструкция: откройте браузер → Дополнительно → Перейти на сайт (небезопасно)
        </div>
      </div>
    {/if}

    <div class="params-section">
      <h3 class="params-title">Параметры</h3>

      <div class="form-grid">
        <div class="form-field">
          <label class="form-label" for="net-port">Порт</label>
          <input
            id="net-port"
            class="form-input"
            type="number"
            min="1"
            max="65535"
            bind:value={settings.port}
            disabled={saving || serverRunning}
          />
        </div>

        <div class="form-field">
          <label class="form-label" for="net-host">Bind-адрес</label>
          <select
            id="net-host"
            class="form-select"
            bind:value={settings.host}
            disabled={saving || serverRunning}
          >
            <option value="0.0.0.0">0.0.0.0 (все интерфейсы)</option>
            <option value="127.0.0.1">127.0.0.1 (только localhost)</option>
          </select>
        </div>

        <div class="form-field form-field--full">
          <label class="form-label" for="net-cert">Путь к сертификату (пусто = авто)</label>
          <input
            id="net-cert"
            class="form-input"
            type="text"
            bind:value={settings.cert_path}
            disabled={saving || serverRunning}
            placeholder="Оставьте пустым для самоподписанного сертификата"
          />
        </div>
      </div>

      <div class="save-row">
        <Button variant="primary" loading={saving} onclick={saveSettings} disabled={serverRunning}>
          Сохранить настройки
        </Button>
        {#if serverRunning}
          <span class="save-hint">Остановите сервер перед изменением настроек</span>
        {/if}
      </div>
    </div>
  </section>

  <!-- Section 2: Desktop security (D-Desktop-02 — mandatory) -->
  <section class="settings-section">
    <h2 class="section-title">Безопасность рабочего стола</h2>

    <div class="form-field">
      <label class="checkbox-label">
        <input
          type="checkbox"
          checked={settings.desktop_lock_enabled}
          disabled={lockToggling}
          onchange={(e) => toggleDesktopLock((e.target as HTMLInputElement).checked)}
        />
        <span class="checkbox-text">Требовать вход в десктопе</span>
      </label>
      <p class="helper-text">
        Когда включено, при запуске приложения требуется ввод логина и пароля. Без этой настройки
        рабочее место всегда работает в режиме администратора.
      </p>
    </div>
  </section>
</div>

<style lang="scss">
  .network-settings {
    display: flex;
    flex-direction: column;
    gap: var(--space-xl);
    max-width: 640px;
  }

  .settings-section {
    background: var(--tr-surface);
    border: 1px solid var(--tr-border);
    border-radius: var(--radius-md);
    padding: var(--space-lg);
  }

  .section-title {
    margin: 0 0 var(--space-md);
    font-size: var(--font-size-heading);
    font-weight: var(--font-weight-semibold);
    color: var(--tr-text-primary);
  }

  .server-toggle-row {
    display: flex;
    align-items: center;
    gap: var(--space-md);
    margin-bottom: var(--space-md);
    flex-wrap: wrap;
  }

  .toggle-label {
    font-size: var(--font-size-body);
    font-weight: var(--font-weight-medium);
    color: var(--tr-text-secondary);
  }

  .toggle-actions {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
  }

  .status-badge {
    display: inline-block;
    padding: 2px var(--space-sm);
    border-radius: var(--radius-sm);
    font-size: var(--font-size-label);
    font-weight: var(--font-weight-medium);

    &--running {
      background: color-mix(in srgb, var(--tr-success) 15%, transparent);
      color: var(--tr-success-text);
    }

    &--stopped {
      background: color-mix(in srgb, var(--tr-text-tertiary) 15%, transparent);
      color: var(--tr-text-tertiary);
    }
  }

  .server-info-block {
    background: color-mix(in srgb, var(--tr-accent) 6%, transparent);
    border: 1px solid color-mix(in srgb, var(--tr-accent) 25%, transparent);
    border-radius: var(--radius-sm);
    padding: var(--space-md);
    margin-bottom: var(--space-md);
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }

  .info-row {
    display: flex;
    align-items: flex-start;
    gap: var(--space-sm);
    flex-wrap: wrap;
  }

  .info-label {
    font-size: var(--font-size-label);
    font-weight: var(--font-weight-medium);
    color: var(--tr-text-secondary);
    white-space: nowrap;
  }

  .server-link {
    font-size: var(--font-size-body);
    color: var(--tr-accent);
    text-decoration: none;

    &:hover {
      text-decoration: underline;
    }
  }

  .fingerprint {
    font-family: monospace;
    font-size: var(--font-size-label);
    color: var(--tr-text-primary);
    word-break: break-all;
  }

  .info-instruction {
    font-size: var(--font-size-label);
    color: var(--tr-text-secondary);
    font-style: italic;
  }

  .params-section {
    border-top: 1px solid var(--tr-border);
    padding-top: var(--space-md);
    margin-top: var(--space-md);
  }

  .params-title {
    margin: 0 0 var(--space-md);
    font-size: var(--font-size-body);
    font-weight: var(--font-weight-medium);
    color: var(--tr-text-secondary);
  }

  .form-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-md);
    margin-bottom: var(--space-md);
  }

  .form-field {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);

    &--full {
      grid-column: 1 / -1;
    }
  }

  .form-label {
    font-size: var(--font-size-label);
    font-weight: var(--font-weight-medium);
    color: var(--tr-text-secondary);
  }

  .form-input,
  .form-select {
    padding: var(--space-sm) var(--space-md);
    border: 1px solid var(--tr-border);
    border-radius: var(--radius-sm);
    font-size: var(--font-size-body);
    background: var(--tr-bg);
    color: var(--tr-text-primary);

    &:focus {
      outline: none;
      border-color: var(--tr-accent);
      box-shadow: 0 0 0 2px color-mix(in srgb, var(--tr-accent) 20%, transparent);
    }

    &:disabled {
      opacity: 0.6;
      cursor: not-allowed;
    }
  }

  .save-row {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    flex-wrap: wrap;
  }

  .save-hint {
    font-size: var(--font-size-label);
    color: var(--tr-text-tertiary);
  }

  .checkbox-label {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    font-size: var(--font-size-body);
    color: var(--tr-text-primary);
    cursor: pointer;

    input[type='checkbox'] {
      width: 16px;
      height: 16px;
      accent-color: var(--tr-accent);
    }
  }

  .checkbox-text {
    font-weight: var(--font-weight-medium);
  }

  .helper-text {
    margin: var(--space-xs) 0 0;
    font-size: var(--font-size-label);
    color: var(--tr-text-tertiary);
    line-height: 1.5;
  }
</style>
