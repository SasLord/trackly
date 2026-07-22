<script lang="ts">
  // Phase 9 Plan 05 — Screen 4 (UI-SPEC). Mirrors NetworkSettings.svelte
  // structure. `enabled`/`auto_accept` are the only writable fields
  // (settings_set_ad) — host/port/domain/base_dn/name_attr/no_tls_verify
  // are read-only bootstrap config from trackly.config.toml, shown here
  // for visibility only (UI-SPEC Screen 4 / AdSettingsDto doc).
  import { onMount } from 'svelte';
  import Button from '$lib/components/Button.svelte';
  import Checkbox from '$lib/components/Checkbox.svelte';
  import Radio from '$lib/components/Radio.svelte';
  import Input from '$lib/components/Input.svelte';
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

  // Radio-group адаптер: Radio требует bind:group на одной переменной,
  // settings.auto_accept — независимый boolean. regMode синхронизирован
  // двунаправленно через $effect (settings.auto_accept -> regMode при
  // внешней загрузке; regMode -> settings.auto_accept при клике radio).
  let regMode = $state<'auto' | 'confirm'>('auto');
  $effect(() => {
    regMode = settings.auto_accept ? 'auto' : 'confirm';
  });
  $effect(() => {
    settings.auto_accept = regMode === 'auto';
  });

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
      <Checkbox
        id="ad-enabled"
        checked={settings.enabled}
        disabled={saving}
        onchange={(checked) => (settings.enabled = checked)}
      >
        Использовать Active Directory
      </Checkbox>
      <p class="helper-text">
        Сотрудники смогут входить через браузер по доменному логину и паролю.
      </p>
    </div>

    <div class="form-field" class:is-dimmed={!settings.enabled}>
      <span class="form-label">Регистрация новых пользователей</span>

      <div class="radio-label">
        <Radio bind:group={regMode} value="auto" disabled={saving || !settings.enabled}>
          <span class="radio-text">
            <span class="radio-title">Автоматически принимать</span>
            <span class="helper-text">
              Новый доменный пользователь сразу получает доступ с ролью «Сотрудник».
            </span>
          </span>
        </Radio>
      </div>

      <div class="radio-label">
        <Radio bind:group={regMode} value="confirm" disabled={saving || !settings.enabled}>
          <span class="radio-text">
            <span class="radio-title">Требовать подтверждения</span>
            <span class="helper-text">
              Новый пользователь ждёт, пока администратор подтвердит заявку.
            </span>
          </span>
        </Radio>
      </div>
    </div>

    <details class="advanced-details">
      <summary class="advanced-summary">Расширенные настройки</summary>
      <p class="helper-text">
        Заполняется автоматически на доменном компьютере. Меняйте только если автоопределение не
        сработало.
      </p>

      <div class="form-grid">
        <div class="form-field">
          <label class="form-label" for="ad-host">Адрес сервера (host:port)</label>
          <Input
            id="ad-host"
            type="text"
            value={settings.port ? `${settings.host}:${settings.port}` : settings.host}
            disabled
          />
        </div>

        <div class="form-field">
          <label class="form-label" for="ad-domain">Домен (например, corp.local)</label>
          <Input id="ad-domain" type="text" value={settings.domain} disabled />
        </div>

        <div class="form-field form-field--full">
          <label class="form-label" for="ad-base-dn">Base DN</label>
          <Input id="ad-base-dn" type="text" value={settings.base_dn} disabled />
        </div>

        <div class="form-field">
          <label class="form-label" for="ad-name-attr">Атрибут ФИО</label>
          <Input id="ad-name-attr" type="text" value={settings.name_attr} disabled />
        </div>

        <div class="form-field form-field--full">
          <Checkbox id="ad-no-tls-verify" checked={settings.no_tls_verify} disabled>
            Не проверять TLS-сертификат (небезопасно)
          </Checkbox>
        </div>
      </div>
    </details>

    <div class="save-row">
      <Button variant="primary" loading={saving} onclick={saveSettings}>Сохранить настройки</Button>
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
    gap: var(--tr-space-2xl);
    max-width: 640px;
  }

  .settings-section {
    background: var(--tr-surface);
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-md);
    padding: var(--tr-space-xl);
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-xl);
  }

  .section-title {
    margin: 0;
    font-size: var(--tr-font-size-h3);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-primary);
  }

  .form-field {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-2xs);

    &--full {
      grid-column: 1 / -1;
    }

    &.is-dimmed {
      opacity: 0.6;
    }
  }

  .form-label {
    font-size: var(--tr-font-size-label);
    font-weight: var(--tr-font-weight-medium);
    color: var(--tr-text-secondary);
  }

  .radio-label {
    margin-top: var(--tr-space-xs);
  }

  .radio-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .radio-title {
    font-size: var(--tr-font-size-body);
    font-weight: var(--tr-font-weight-medium);
    color: var(--tr-text-primary);
  }

  .helper-text {
    margin: 0;
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-tertiary);
    line-height: 1.5;
  }

  .advanced-details {
    border-top: 1px solid var(--tr-border);
    padding-top: var(--tr-space-md);
  }

  .advanced-summary {
    font-size: var(--tr-font-size-body);
    font-weight: var(--tr-font-weight-medium);
    color: var(--tr-text-primary);
    cursor: pointer;
  }

  .form-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--tr-space-md);
    margin-top: var(--tr-space-md);
  }

  .save-row {
    display: flex;
    align-items: center;
    gap: var(--tr-space-xs);
    flex-wrap: wrap;
  }

  .save-hint {
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-tertiary);

    &.is-success {
      color: var(--tr-success);
    }

    &.is-error {
      color: var(--tr-danger);
    }
  }
</style>
