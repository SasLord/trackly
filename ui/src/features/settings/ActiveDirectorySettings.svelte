<script lang="ts">
  // Phase 9 Plan 05 — Screen 4 (UI-SPEC). Mirrors NetworkSettings.svelte
  // structure. `enabled`/`auto_accept` are the only writable fields
  // (settings_set_ad) — host/port/domain/base_dn/name_attr/no_tls_verify
  // are read-only bootstrap config from trackly.config.toml, shown here
  // for visibility only (UI-SPEC Screen 4 / AdSettingsDto doc).
  import { onMount } from 'svelte';
  import Button from '$lib/components/Button.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { apiCall } from '$lib/api/client';
  import type { AdSettingsDto, SetAdPayload } from '../../bindings-phase9';

  let settings = $state<AdSettingsDto>({
    enabled: false,
    auto_accept: false,
    host: '',
    port: 389,
    domain: '',
    base_dn: '',
    name_attr: 'displayName',
    no_tls_verify: false,
  });

  let saving = $state(false);
  let testing = $state(false);
  let testResult = $state<{ ok: boolean; message: string } | null>(null);

  async function loadSettings() {
    try {
      const s = await apiCall<AdSettingsDto>('settings_get_ad', {});
      settings = s;
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
      const payload: SetAdPayload = {
        enabled: settings.enabled,
        autoAccept: settings.auto_accept,
      };
      await apiCall<void>('settings_set_ad', { payload });
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

  async function testConnection() {
    testing = true;
    testResult = null;
    try {
      await apiCall<void>('ad_test_connection', {});
      testResult = { ok: true, message: 'Подключение успешно' };
      pushToast('success', 'Подключение к Active Directory успешно');
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'AD недоступен';
      testResult = { ok: false, message: msg };
      pushToast('error', msg);
    } finally {
      testing = false;
    }
  }
</script>

<div class="ad-settings">
  <section class="settings-section">
    <h2 class="section-title">Active Directory</h2>

    <div class="form-field">
      <label class="checkbox-label">
        <input
          type="checkbox"
          checked={settings.enabled}
          disabled={saving}
          onchange={(e) => (settings.enabled = (e.target as HTMLInputElement).checked)}
        />
        <span class="checkbox-text">Использовать Active Directory</span>
      </label>
      <p class="helper-text">
        Сотрудники смогут входить через браузер по доменному логину и паролю.
      </p>
    </div>

    <div class="form-field" class:is-dimmed={!settings.enabled}>
      <span class="form-label">Регистрация новых пользователей</span>

      <label class="radio-label">
        <input
          type="radio"
          name="ad-reg-mode"
          checked={settings.auto_accept}
          disabled={saving || !settings.enabled}
          onchange={() => (settings.auto_accept = true)}
        />
        <span class="radio-text">
          <span class="radio-title">Автоматически принимать</span>
          <span class="helper-text">
            Новый доменный пользователь сразу получает доступ с ролью «Сотрудник».
          </span>
        </span>
      </label>

      <label class="radio-label">
        <input
          type="radio"
          name="ad-reg-mode"
          checked={!settings.auto_accept}
          disabled={saving || !settings.enabled}
          onchange={() => (settings.auto_accept = false)}
        />
        <span class="radio-text">
          <span class="radio-title">Требовать подтверждения</span>
          <span class="helper-text">
            Новый пользователь ждёт, пока администратор подтвердит заявку.
          </span>
        </span>
      </label>
    </div>

    <details class="advanced-details">
      <summary class="advanced-summary">Расширенные настройки</summary>
      <p class="helper-text">
        Заполняется автоматически на доменном компьютере. Меняйте только если
        автоопределение не сработало.
      </p>

      <div class="form-grid">
        <div class="form-field">
          <label class="form-label" for="ad-host">Адрес сервера (host:port)</label>
          <input
            id="ad-host"
            class="form-input"
            type="text"
            value={settings.port ? `${settings.host}:${settings.port}` : settings.host}
            disabled
          />
        </div>

        <div class="form-field">
          <label class="form-label" for="ad-domain">Домен (например, corp.local)</label>
          <input id="ad-domain" class="form-input" type="text" value={settings.domain} disabled />
        </div>

        <div class="form-field form-field--full">
          <label class="form-label" for="ad-base-dn">Base DN</label>
          <input id="ad-base-dn" class="form-input" type="text" value={settings.base_dn} disabled />
        </div>

        <div class="form-field">
          <label class="form-label" for="ad-name-attr">Атрибут ФИО</label>
          <input
            id="ad-name-attr"
            class="form-input"
            type="text"
            value={settings.name_attr}
            disabled
          />
        </div>

        <div class="form-field form-field--full">
          <label class="checkbox-label">
            <input type="checkbox" checked={settings.no_tls_verify} disabled />
            <span class="checkbox-text">Не проверять TLS-сертификат (небезопасно)</span>
          </label>
        </div>
      </div>
    </details>

    <div class="save-row">
      <Button variant="primary" loading={saving} onclick={saveSettings}>
        Сохранить настройки
      </Button>
      <Button
        variant="secondary"
        loading={testing}
        disabled={!settings.enabled || saving}
        onclick={testConnection}
      >
        Проверить подключение
      </Button>
      {#if !settings.enabled}
        <span class="save-hint">Включите Active Directory, чтобы проверить подключение</span>
      {:else if testResult}
        <span class="save-hint" class:is-success={testResult.ok} class:is-error={!testResult.ok}>
          {testResult.ok ? 'Подключение успешно' : testResult.message}
        </span>
      {/if}
    </div>
  </section>
</div>

<style lang="scss">
  .ad-settings {
    display: flex;
    flex-direction: column;
    gap: var(--space-xl);
    max-width: 640px;
  }

  .settings-section {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    padding: var(--space-lg);
    display: flex;
    flex-direction: column;
    gap: var(--space-lg);
  }

  .section-title {
    margin: 0;
    font-size: var(--font-size-heading);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
  }

  .form-field {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);

    &--full {
      grid-column: 1 / -1;
    }

    &.is-dimmed {
      opacity: 0.6;
    }
  }

  .form-label {
    font-size: var(--font-size-label);
    font-weight: var(--font-weight-medium);
    color: var(--color-text-secondary);
  }

  .checkbox-label {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    font-size: var(--font-size-body);
    color: var(--color-text-primary);
    cursor: pointer;

    input[type='checkbox'] {
      width: 16px;
      height: 16px;
      accent-color: var(--color-accent);
    }
  }

  .checkbox-text {
    font-weight: var(--font-weight-medium);
  }

  .radio-label {
    display: flex;
    align-items: flex-start;
    gap: var(--space-sm);
    cursor: pointer;
    margin-top: var(--space-sm);

    input[type='radio'] {
      width: 16px;
      height: 16px;
      margin-top: 2px;
      accent-color: var(--color-accent);
      cursor: pointer;
    }
  }

  .radio-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .radio-title {
    font-size: var(--font-size-body);
    font-weight: var(--font-weight-medium);
    color: var(--color-text-primary);
  }

  .helper-text {
    margin: 0;
    font-size: var(--font-size-label);
    color: var(--color-text-muted);
    line-height: 1.5;
  }

  .advanced-details {
    border-top: 1px solid var(--color-border);
    padding-top: var(--space-md);
  }

  .advanced-summary {
    font-size: var(--font-size-body);
    font-weight: var(--font-weight-medium);
    color: var(--color-text-primary);
    cursor: pointer;
  }

  .form-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-md);
    margin-top: var(--space-md);
  }

  .form-input {
    padding: var(--space-sm) var(--space-md);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    font-size: var(--font-size-body);
    background: var(--color-bg);
    color: var(--color-text-primary);

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
    color: var(--color-text-muted);

    &.is-success {
      color: var(--color-success);
    }

    &.is-error {
      color: var(--color-destructive);
    }
  }
</style>
