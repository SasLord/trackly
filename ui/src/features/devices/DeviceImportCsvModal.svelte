<script lang="ts">
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';
  import Select from '$lib/components/Select.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { isTauri } from '$lib/stores/transport.svelte';
  import { apiCall } from '$lib/api/client';
  import { devices } from './api';
  import type { CsvImportPreviewResponse, CsvImportReport } from '../../bindings';

  // ---------------------------------------------------------------------------
  // Props
  // ---------------------------------------------------------------------------
  interface Props {
    open: boolean;
    onClose: () => void;
    onImported: () => void;
  }

  const { open, onClose, onImported }: Props = $props();

  // ---------------------------------------------------------------------------
  // State (Svelte 5 runes)
  // ---------------------------------------------------------------------------
  let step = $state<1 | 2 | 3 | 4>(1);
  let preview = $state<CsvImportPreviewResponse | null>(null);
  let mapping = $state<Record<string, string>>({}); // csv header → device field name
  let report = $state<CsvImportReport | null>(null);
  let loading = $state(false);
  let errorsExpanded = $state(false);

  // ---------------------------------------------------------------------------
  // Device field options (for mapping select)
  // ---------------------------------------------------------------------------
  const FIELD_OPTIONS: { value: string; label: string }[] = [
    { value: '', label: '— не импортировать —' },
    { value: 'type', label: 'Тип' },
    { value: 'name', label: 'Наименование' },
    { value: 'inventory_no', label: 'Инвентарный №' },
    { value: 'serial_no', label: 'Серийный №' },
    { value: 'model', label: 'Модель' },
    { value: 'specs', label: 'Технические характеристики' },
    { value: 'kit', label: 'Комплектация' },
    { value: 'state', label: 'Состояние' },
    { value: 'location', label: 'Расположение' },
    { value: 'status', label: 'Статус' },
  ];

  // ---------------------------------------------------------------------------
  // Auto-mapping: match Russian headers to device field names
  // ---------------------------------------------------------------------------
  function autoMapHeaders(headers: string[]): Record<string, string> {
    const result: Record<string, string> = {};
    for (const header of headers) {
      const h = header.trim();
      const field = (() => {
        switch (h) {
          case 'Тип':
          case 'тип':
            return 'type';
          case 'Наименование':
          case 'наименование':
          case 'Имя':
          case 'имя':
            return 'name';
          case 'Инвентарный №':
          case 'Инв.№':
          case 'Инвентарный':
            return 'inventory_no';
          case 'Серийный №':
          case 'Серийный':
            return 'serial_no';
          case 'Модель':
          case 'модель':
            return 'model';
          case 'Технические характеристики':
          case 'Тех.характеристики':
            return 'specs';
          case 'Комплектация':
            return 'kit';
          case 'Состояние':
            return 'state';
          case 'Расположение':
          case 'Местоположение':
            return 'location';
          case 'Статус':
            return 'status';
          default:
            return '';
        }
      })();
      result[header] = field;
    }
    return result;
  }

  // ---------------------------------------------------------------------------
  // Step 1: open file picker and call preview
  // ---------------------------------------------------------------------------
  async function openFilePicker() {
    if (!isTauri) {
      pushToast('error', 'Выбор файла доступен только в десктопном приложении');
      return;
    }

    let filePath: string;
    try {
      const { open: openDialog } = await import('@tauri-apps/plugin-dialog');
      const result = await openDialog({
        filters: [{ name: 'CSV', extensions: ['csv'] }],
        multiple: false,
      });
      if (!result || Array.isArray(result)) return;
      filePath = result;
    } catch {
      pushToast('error', 'Не удалось открыть диалог выбора файла');
      return;
    }

    loading = true;
    try {
      // Read file bytes via backend FS helper (T-02-05-02 path validation).
      const bytes = await apiCall<number[]>('read_file_bytes', { path: filePath });

      // Preview: detect encoding + delimiter, return first 5 rows.
      preview = await devices.importCsvPreview(bytes);
      mapping = autoMapHeaders(preview.headers);
      step = 2;
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось прочитать файл';
      pushToast('error', msg);
    } finally {
      loading = false;
    }
  }

  // ---------------------------------------------------------------------------
  // Step 3 → 4: commit import
  // ---------------------------------------------------------------------------
  async function doImport() {
    if (!preview) return;
    loading = true;
    try {
      // Filter out unmapped columns.
      const activeMapping = Object.fromEntries(Object.entries(mapping).filter(([, v]) => v !== ''));
      report = await devices.importCsvCommit(preview.token, activeMapping);
      step = 4;
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Ошибка при импорте';
      pushToast('error', msg);
    } finally {
      loading = false;
    }
  }

  // ---------------------------------------------------------------------------
  // Finish
  // ---------------------------------------------------------------------------
  function handleDone() {
    onImported();
    resetState();
  }

  function handleClose() {
    onClose();
    resetState();
  }

  function resetState() {
    step = 1;
    preview = null;
    mapping = {};
    report = null;
    loading = false;
    errorsExpanded = false;
  }

  // ---------------------------------------------------------------------------
  // Helpers
  // ---------------------------------------------------------------------------
  function stepTitle(s: 1 | 2 | 3 | 4): string {
    switch (s) {
      case 1:
        return 'Импорт устройств из CSV';
      case 2:
        return 'Проверьте данные';
      case 3:
        return 'Сопоставление колонок';
      case 4:
        return 'Импорт завершён';
    }
  }
</script>

<Modal {open} title={stepTitle(step)} size="wide" onClose={handleClose}>
  <!-- Step indicator -->
  <div class="step-indicator" role="status" aria-label="Шаг {step} из 4">
    {#each [1, 2, 3, 4] as s}
      <span class="step-dot" class:active={s === step} class:done={s < step} aria-hidden="true"
      ></span>
    {/each}
    <span class="step-label">Шаг {step} из 4</span>
  </div>

  <!-- -------------------------------------------------------------------------
       Step 1: File pick
  --------------------------------------------------------------------------- -->
  {#if step === 1}
    <div class="step-body">
      <p class="step-help">
        Выберите CSV-файл. Поддерживаются UTF-8, UTF-8 с BOM, Windows-1251. Разделители — запятая
        или точка с запятой.
      </p>
      <div class="step-actions">
        <Button variant="primary" onclick={openFilePicker} disabled={loading}>
          {loading ? 'Загрузка…' : 'Выбрать файл…'}
        </Button>
      </div>
    </div>

    <!-- -------------------------------------------------------------------------
       Step 2: Preview
  --------------------------------------------------------------------------- -->
  {:else if step === 2 && preview}
    <div class="step-body">
      <p class="encoding-info">
        Определена кодировка: <strong>{preview.encoding}</strong>, разделитель:
        <strong>«{preview.delimiter}»</strong>. Показаны первые {preview.preview_rows.length} строк из
        {preview.total_rows}.
      </p>
      {#if preview.had_replacements}
        <div class="warning-banner" role="alert">
          Внимание: при декодировании обнаружены некорректные символы. Возможны ошибки.
        </div>
      {/if}
      <div class="preview-table-wrap">
        <table class="preview-table">
          <thead>
            <tr>
              {#each preview.headers as header}
                <th>{header}</th>
              {/each}
            </tr>
          </thead>
          <tbody>
            {#each preview.preview_rows as row}
              <tr>
                {#each preview.headers as _h, i}
                  <td>{row[i] ?? ''}</td>
                {/each}
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </div>

    <!-- -------------------------------------------------------------------------
       Step 3: Mapping
  --------------------------------------------------------------------------- -->
  {:else if step === 3 && preview}
    <div class="step-body">
      <p class="step-help">
        Колонки CSV сопоставлены с полями устройств автоматически по заголовкам. При необходимости
        измените.
      </p>
      <table class="mapping-table">
        <thead>
          <tr>
            <th>Колонка CSV</th>
            <th>Поле устройства</th>
          </tr>
        </thead>
        <tbody>
          {#each preview.headers as header}
            <tr>
              <td class="csv-col">{header}</td>
              <td>
                <Select
                  value={mapping[header] ?? ''}
                  onchange={(v) => {
                    mapping = { ...mapping, [header]: v };
                  }}
                >
                  {#each FIELD_OPTIONS as opt}
                    <option value={opt.value}>{opt.label}</option>
                  {/each}
                </Select>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>

    <!-- -------------------------------------------------------------------------
       Step 4: Result
  --------------------------------------------------------------------------- -->
  {:else if step === 4 && report}
    <div class="step-body">
      <p class="result-summary">
        Импортировано: <strong>{report.inserted}</strong>. Пропущено с ошибками:
        <strong>{report.failed.length}</strong>.
      </p>
      {#if report.failed.length > 0}
        <button
          class="errors-toggle"
          type="button"
          onclick={() => (errorsExpanded = !errorsExpanded)}
        >
          {errorsExpanded ? 'Скрыть ошибки' : `Показать ошибки (${report.failed.length})`}
        </button>
        {#if errorsExpanded}
          <ul class="error-list">
            {#each report.failed as err}
              <li>
                <span class="error-row-num">Строка {err.row_index}:</span>
                {err.error_message}
              </li>
            {/each}
          </ul>
        {/if}
      {/if}
    </div>
  {/if}

  {#snippet footer()}
    <div class="modal-footer-actions">
      {#if step === 1}
        <Button variant="secondary" onclick={handleClose}>Отмена</Button>
      {:else if step === 2}
        <Button variant="secondary" onclick={handleClose}>Отмена</Button>
        <Button
          variant="primary"
          onclick={() => {
            step = 3;
          }}
        >
          Далее
        </Button>
      {:else if step === 3}
        <Button
          variant="secondary"
          onclick={() => {
            step = 2;
          }}
        >
          Назад
        </Button>
        <Button variant="primary" onclick={doImport} disabled={loading}>
          {loading ? 'Импортирование…' : 'Импортировать'}
        </Button>
      {:else if step === 4}
        <Button variant="primary" onclick={handleDone}>Готово</Button>
      {/if}
    </div>
  {/snippet}
</Modal>

<style lang="scss">
  .step-indicator {
    display: flex;
    align-items: center;
    gap: var(--tr-space-2xs, 4px);
    margin-bottom: var(--tr-space-md, 12px);
  }

  .step-dot {
    display: inline-block;
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--tr-border);
    transition: background 0.15s;

    &.active {
      background: var(--tr-accent);
    }

    &.done {
      background: var(--tr-success);
    }
  }

  .step-label {
    margin-left: var(--tr-space-xs, 8px);
    font-size: var(--font-size-caption, 12px);
    color: var(--tr-text-secondary);
  }

  .step-body {
    min-height: 180px;
  }

  .step-help {
    color: var(--tr-text-secondary);
    margin-bottom: var(--tr-space-md, 12px);
    font-size: var(--font-size-body);
    line-height: var(--line-height-body);
  }

  .step-actions {
    margin-top: var(--tr-space-md, 12px);
  }

  .encoding-info {
    margin-bottom: var(--tr-space-md, 12px);
    font-size: var(--font-size-body);
    color: var(--tr-text-primary);
  }

  .warning-banner {
    background: var(--tr-warning-soft);
    color: var(--tr-warning-text);
    border: 1px solid var(--tr-warning);
    border-radius: var(--radius-sm, 4px);
    padding: var(--tr-space-xs, 8px) var(--tr-space-md, 12px);
    margin-bottom: var(--tr-space-md, 12px);
    font-size: var(--font-size-body);
  }

  .preview-table-wrap {
    overflow-x: auto;
    overflow-y: auto;
    max-height: 280px;
    border: 1px solid var(--tr-border);
    border-radius: var(--radius-sm, 4px);
  }

  .preview-table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--font-size-body);

    th {
      position: sticky;
      top: 0;
      background: var(--tr-surface);
      padding: var(--tr-space-xs, 8px) var(--tr-space-md, 12px);
      text-align: left;
      font-weight: var(--font-weight-semibold, 600);
      border-bottom: 1px solid var(--tr-border);
      white-space: nowrap;
    }

    td {
      padding: var(--tr-space-2xs, 4px) var(--tr-space-md, 12px);
      border-bottom: 1px solid var(--tr-border);
      max-width: 200px;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    tr:last-child td {
      border-bottom: none;
    }
  }

  .mapping-table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--font-size-body);

    th {
      padding: var(--tr-space-xs, 8px) var(--tr-space-md, 12px);
      text-align: left;
      font-weight: var(--font-weight-semibold, 600);
      border-bottom: 2px solid var(--tr-border);
      background: var(--tr-surface);
    }

    td {
      padding: var(--tr-space-2xs, 4px) var(--tr-space-md, 12px);
      border-bottom: 1px solid var(--tr-border);
      vertical-align: middle;
    }

    tr:last-child td {
      border-bottom: none;
    }

    .csv-col {
      font-weight: var(--font-weight-medium, 500);
      white-space: nowrap;
    }
  }

  .result-summary {
    font-size: var(--font-size-body);
    color: var(--tr-text-primary);
    margin-bottom: var(--tr-space-md, 12px);
  }

  .errors-toggle {
    background: none;
    border: none;
    color: var(--tr-accent);
    cursor: pointer;
    font-size: var(--font-size-body);
    padding: 0;
    text-decoration: underline;
    margin-bottom: var(--tr-space-xs, 8px);

    &:hover {
      color: var(--tr-accent-hover);
    }
  }

  .error-list {
    list-style: none;
    padding: 0;
    margin: 0;
    max-height: 200px;
    overflow-y: auto;
    border: 1px solid var(--tr-border);
    border-radius: var(--radius-sm, 4px);
    font-size: var(--font-size-body);

    li {
      padding: var(--tr-space-2xs, 4px) var(--tr-space-md, 12px);
      border-bottom: 1px solid var(--tr-border);
      color: var(--tr-danger);

      &:last-child {
        border-bottom: none;
      }
    }

    .error-row-num {
      font-weight: var(--font-weight-medium, 500);
      margin-right: var(--tr-space-2xs, 4px);
    }
  }

  .modal-footer-actions {
    display: flex;
    gap: var(--tr-space-xs, 8px);
    justify-content: flex-end;
  }
</style>
