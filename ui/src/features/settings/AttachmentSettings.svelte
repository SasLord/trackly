<script lang="ts">
  // Фаза 41.3, план 12 (D-07, D-20, D-21): подраздел «Вложения» вкладки «Хранилище».
  // Каталог — только для чтения (без «Открыть папку»), лимит размера файла и отчёт
  // проверки целостности. Работает и в десктопе, и в LAN-браузере.
  import { onMount } from 'svelte';
  import Button from '$lib/components/Button.svelte';
  import Input from '$lib/components/Input.svelte';
  import Spinner from '$lib/components/Spinner.svelte';
  import DetailField from '$lib/components/DetailField.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { formatFileSize } from '$lib/utils/formatFileSize';
  import { formatDateTimeShort } from '$lib/utils/date';
  import { attachments } from '../attachments/api';
  import type { AttachmentLimitDto, IntegrityReportDto } from '../../bindings';

  const MB = 1024 * 1024;
  const MIN_MB = 1;
  const MAX_MB = 100;

  let limit = $state<AttachmentLimitDto | null>(null);
  let savedMb = $state<number | null>(null);
  let inputValue = $state('');
  let saving = $state(false);
  let touched = $state(false);

  let reportLoading = $state(true);
  let reportError = $state(false);
  let report = $state<IntegrityReportDto | null>(null);

  const parsed = $derived.by(() => {
    const raw = inputValue.trim();
    const n = Number(raw);
    return raw !== '' && Number.isInteger(n) && String(n) === raw ? n : null;
  });
  const valid = $derived(parsed !== null && parsed >= MIN_MB && parsed <= MAX_MB);
  const changed = $derived(valid && parsed !== savedMb);
  const showError = $derived(touched && !valid);

  async function loadLimit() {
    try {
      const dto = await attachments.getLimit();
      limit = dto;
      savedMb = Math.round(dto.maxBytes / MB);
      inputValue = String(savedMb);
    } catch {
      pushToast('error', 'Не удалось получить настройки вложений.');
    }
  }

  async function loadReport() {
    reportLoading = true;
    reportError = false;
    try {
      report = await attachments.report();
    } catch {
      reportError = true;
    } finally {
      reportLoading = false;
    }
  }

  onMount(() => {
    void loadLimit();
    void loadReport();
  });

  async function saveLimit() {
    touched = true;
    if (!valid || parsed === null) return;
    saving = true;
    try {
      await attachments.setLimit(parsed * MB);
      savedMb = parsed;
      pushToast('success', `Лимит размера файла: ${parsed} МБ`);
    } catch {
      // Значение остаётся в поле.
      pushToast('error', 'Не удалось сохранить лимит. Попробуйте ещё раз.');
    } finally {
      saving = false;
    }
  }

  const reportUnavailable = $derived(report !== null && report.status !== 'ok');
</script>

<section class="settings-section" aria-labelledby="attachment-settings-title">
  <h2 id="attachment-settings-title" class="section-title">Вложения</h2>

  <div class="form-field">
    <span class="form-label">Каталог вложений</span>
    <p class="path-display">
      <code class="path-code">{limit?.path || 'Загрузка…'}</code>
    </p>
    <span class="caption">
      По умолчанию — папка attachments рядом с базой данных. Другой путь задаётся параметром
      attachments_path в trackly.config.toml.
    </span>
  </div>

  <div class="form-field">
    <label class="form-label" for="attachment-limit-input">Максимальный размер файла</label>
    <div class="limit-row">
      <div class="limit-input">
        <Input
          id="attachment-limit-input"
          type="text"
          value={inputValue}
          invalid={showError}
          aria-describedby="attachment-limit-hint"
          oninput={(v) => {
            inputValue = v;
            touched = true;
          }}
        />
      </div>
      <span class="unit">МБ</span>
      <Button variant="primary" size="sm" disabled={!changed} loading={saving} onclick={saveLimit}>
        Сохранить лимит
      </Button>
    </div>
    {#if showError}
      <span class="error-text" role="alert">Укажите целое число от 1 до 100.</span>
    {/if}
    <span id="attachment-limit-hint" class="caption">От 1 до 100 МБ. По умолчанию 20 МБ.</span>
  </div>

  <div class="report">
    <h3 class="report-title">Проверка целостности</h3>
    {#if reportLoading}
      <div class="report-state">
        <Spinner size="sm" />
        <span class="state-text">Проверка выполняется…</span>
      </div>
    {:else if reportError || reportUnavailable}
      <p class="state-text">Не удалось получить отчёт. Обновите страницу.</p>
    {:else if report === null}
      <p class="state-text">Проверка ещё не выполнялась.</p>
    {:else}
      <div class="report-grid">
        <DetailField
          label="Занято на диске"
          value={`${formatFileSize(report.bytes)} (${report.files} файлов)`}
        />
        <div class="report-item">
          <DetailField label="Потерянные файлы">
            <span class:warn={report.lost > 0}>{report.lost}</span>
          </DetailField>
          {#if report.lost > 0}
            <span class="caption">
              Записи есть, а файлов нет. Такие вложения показываются плиткой «Файл не найден».
              Верните файлы в каталог вложений из резервной копии.
            </span>
          {/if}
        </div>
        <div class="report-item">
          <DetailField label="Файлы-сироты" value={String(report.orphansYoung)} />
          {#if report.orphansYoung > 0}
            <span class="caption">
              Файлы без записей в базе. Удаляются автоматически при следующем запуске, если старше
              суток.
            </span>
          {/if}
        </div>
        <DetailField
          label="Удалённые вложения"
          value={`${report.deletedRows} (${formatFileSize(report.deletedBytes)})`}
        />
        <DetailField label="Последняя проверка" value={formatDateTimeShort(report.checkedAtUtc)} />
      </div>
    {/if}
  </div>
</section>

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

  .path-display {
    margin: 0;
  }

  .path-code {
    font-family: var(--tr-font-mono);
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-primary);
    word-break: break-all;
    background: var(--tr-surface-sunken);
    padding: var(--tr-space-2xs) var(--tr-space-xs);
    border-radius: var(--tr-radius-xs);
    display: inline-block;
  }

  .caption {
    font-size: var(--tr-font-size-caption);
    color: var(--tr-text-secondary);
  }

  .limit-row {
    display: flex;
    align-items: center;
    gap: var(--tr-space-xs);
    flex-wrap: wrap;
  }

  .limit-input {
    width: 120px;
  }

  .unit {
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-secondary);
  }

  .error-text {
    font-size: var(--tr-font-size-caption);
    color: var(--tr-danger-text);
  }

  .report {
    border-top: 1px solid var(--tr-border);
    padding-top: var(--tr-space-md);
  }

  .report-title {
    margin: 0 0 var(--tr-space-sm);
    font-size: var(--tr-font-size-body-strong);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-primary);
  }

  .report-grid {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-sm);
  }

  .report-item {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-2xs);
  }

  .report-state {
    display: flex;
    align-items: center;
    gap: var(--tr-space-xs);
  }

  .state-text {
    margin: 0;
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-secondary);
  }

  .warn {
    color: var(--tr-warning-muted-text);
    font-weight: var(--tr-font-weight-semibold);
  }
</style>
