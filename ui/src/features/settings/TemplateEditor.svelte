<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import Button from '$lib/components/Button.svelte';
  import Modal from '$lib/components/Modal.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { apiCall } from '$lib/api/client';

  interface TemplateEditorItem {
    kind: string;
    label: string;
    body: string;
  }

  // Human-readable labels for template kinds (D-20)
  const KIND_LABELS: Record<string, string> = {
    act_handover: 'Акт приёма-передачи',
    act_acceptance: 'Документ приёмки товара',
    report: 'Отчёт',
  };

  interface VariableEntry {
    code: string;
    desc: string;
  }

  // Plan 17-03 (D-12): per-kind variables panel — each entry mirrors the
  // context documented in the corresponding templates/*.html doc-comment.
  const VARIABLES_BY_KIND: Record<string, VariableEntry[]> = {
    act_handover: [
      { code: 'org.name', desc: 'название организации' },
      { code: 'org.inn', desc: 'ИНН' },
      { code: 'org.kpp', desc: 'КПП' },
      { code: 'org.address', desc: 'адрес организации' },
      { code: 'org.phone', desc: 'телефон' },
      { code: 'org.fax', desc: 'факс' },
      { code: 'org.email', desc: 'e-mail' },
      { code: 'org.okpo', desc: 'ОКПО' },
      { code: 'org.ogrn', desc: 'ОГРН' },
      { code: 'org.logo_data_uri', desc: 'логотип (data: URI)' },
      { code: 'act.number', desc: 'номер акта' },
      { code: 'act.suffix', desc: 'суффикс номера' },
      { code: 'act.date_human', desc: 'дата акта (человекочитаемая)' },
      { code: 'act.receiver_name', desc: 'кто принял' },
      { code: 'act.deadline_human', desc: 'срок до (человекочитаемый)' },
      { code: 'act.location_name', desc: 'расположение' },
      {
        code: 'act.items[]',
        desc: 'позиции: name, inventory_no, serial_no, model, specs, kit, condition, quantity',
      },
    ],
    act_acceptance: [
      { code: 'org.name', desc: 'название организации' },
      { code: 'org.inn', desc: 'ИНН' },
      { code: 'org.kpp', desc: 'КПП' },
      { code: 'org.address', desc: 'адрес организации' },
      { code: 'org.logo_data_uri', desc: 'логотип (data: URI)' },
      { code: 'document.date_human', desc: 'дата приёма (человекочитаемая)' },
      { code: 'document.giver_name', desc: 'кто передал' },
      { code: 'document.receiver_name', desc: 'кто принял' },
      { code: 'device.name', desc: 'наименование устройства' },
      { code: 'device.inventory_no', desc: 'инвентарный номер' },
      { code: 'device.serial_no', desc: 'серийный номер' },
      { code: 'device.model', desc: 'модель' },
      { code: 'device.condition', desc: 'состояние' },
    ],
    report: [
      { code: 'org.name', desc: 'название организации' },
      { code: 'org.inn', desc: 'ИНН' },
      { code: 'org.kpp', desc: 'КПП' },
      { code: 'org.address', desc: 'адрес организации' },
      { code: 'org.phone', desc: 'телефон' },
      { code: 'org.fax', desc: 'факс' },
      { code: 'org.email', desc: 'e-mail' },
      { code: 'org.okpo', desc: 'ОКПО' },
      { code: 'org.ogrn', desc: 'ОГРН' },
      { code: 'org.logo_data_uri', desc: 'логотип (data: URI)' },
      { code: 'report_name', desc: 'название отчёта' },
      { code: 'period_label', desc: 'описание периода' },
      { code: 'columns', desc: 'список заголовков колонок (строки)' },
      { code: 'groups[].month_label', desc: 'подпись месяца-раздела' },
      { code: 'groups[].rows[]', desc: 'строки таблицы (список значений ячеек)' },
    ],
  };

  let templates = $state<TemplateEditorItem[]>([]);
  let selectedKind = $state('act_handover');
  let body = $state('');
  let originalBody = $state('');
  let previewHtml = $state<string | null>(null);
  let validating = $state(false);
  let saving = $state(false);
  let confirmReset = $state(false);
  let resetting = $state(false);

  // Svelte 5: $derived for unsaved changes indicator
  const isDirty = $derived(body !== originalBody);

  // The currently selected template object
  const selectedTemplate = $derived(templates.find((t) => t.kind === selectedKind) ?? null);

  // Plan 17-03 (D-12): per-kind variables panel content
  const currentVariables = $derived(VARIABLES_BY_KIND[selectedKind] ?? []);

  async function loadTemplates() {
    try {
      const list = await apiCall<TemplateEditorItem[]>('templates_list_for_editor', {});
      templates = list;
      // Select first item if current selectedKind not in list
      if (list.length > 0) {
        const found = list.find((t) => t.kind === selectedKind);
        if (!found) {
          selectedKind = list[0].kind;
        }
        const current = list.find((t) => t.kind === selectedKind);
        if (current) {
          body = current.body;
          originalBody = current.body;
        }
      }
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось загрузить шаблоны';
      pushToast('error', msg);
    }
  }

  onMount(() => {
    loadTemplates();
  });

  // When selectedKind changes: update body from loaded templates
  $effect(() => {
    const found = templates.find((t) => t.kind === selectedKind);
    if (found) {
      body = found.body;
      originalBody = found.body;
      // Clear preview when switching templates. untrack so previewHtml is NOT
      // a dependency of this effect — otherwise validateAndPreview() setting
      // previewHtml re-triggers this effect, which immediately nulls it again
      // and the preview never renders (G2-4).
      untrack(() => {
        previewHtml = null;
      });
    }
  });

  async function validateAndPreview() {
    validating = true;
    try {
      // T-07-04-02: body is sent to backend for validation — never eval'd in browser
      previewHtml = await apiCall<string>('templates_validate_preview', {
        kind: selectedKind,
        body,
      });
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Ошибка валидации шаблона';
      pushToast('error', `Шаблон содержит ошибки: ${msg}. Исправьте и попробуйте снова.`);
    } finally {
      validating = false;
    }
  }

  async function saveTemplate() {
    saving = true;
    try {
      await apiCall<void>('templates_update_body', { kind: selectedKind, body });
      // Update local cache
      const idx = templates.findIndex((t) => t.kind === selectedKind);
      if (idx >= 0) {
        templates[idx] = { ...templates[idx], body };
      }
      originalBody = body;
      pushToast('success', 'Шаблон сохранён');
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Ошибка синтаксиса MiniJinja';
      pushToast('error', `Не удалось сохранить шаблон. Ошибка синтаксиса MiniJinja: ${msg}.`);
    } finally {
      saving = false;
    }
  }

  async function resetTemplate() {
    resetting = true;
    try {
      await apiCall<void>('templates_reset_to_default', { kind: selectedKind });
      confirmReset = false;
      // Reload templates to get default body
      await loadTemplates();
      pushToast('success', 'Шаблон сброшен до умолчания');
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось сбросить шаблон';
      pushToast('error', msg);
    } finally {
      resetting = false;
      confirmReset = false;
    }
  }
</script>

<!-- Full-width card: no max-width: 640px constraint per UI-SPEC §Template editor (SET-09, D-20) -->
<section class="settings-section settings-section--full-width">
  <h2 class="section-title">Шаблоны документов</h2>

  <!-- Template selector -->
  <div class="template-selector-row">
    <label class="form-label" for="template-kind">Шаблон</label>
    <select id="template-kind" class="form-select" bind:value={selectedKind}>
      {#each templates as tmpl (tmpl.kind)}
        <option value={tmpl.kind}>
          {KIND_LABELS[tmpl.kind] ?? tmpl.label ?? tmpl.kind}
        </option>
      {/each}
    </select>
  </div>

  <!-- Available variables panel (T-07-04-02: reference only — not executed in browser).
       Plan 17-03 (D-12): content is per-kind, driven by VARIABLES_BY_KIND. -->
  <details class="variables-panel">
    <summary class="variables-summary">Доступные переменные</summary>
    <div class="variables-grid">
      <div class="var-col">
        {#each currentVariables as v (v.code)}
          <p class="var-item"><code>{v.code}</code> — {v.desc}</p>
        {/each}
      </div>
    </div>
  </details>

  <!-- Template textarea -->
  <div class="editor-wrapper">
    {#if isDirty}
      <span class="unsaved-indicator">• Не сохранено</span>
    {/if}
    <textarea
      class="template-textarea"
      bind:value={body}
      spellcheck="false"
      aria-label="Тело шаблона MiniJinja"
    ></textarea>
  </div>

  <!-- Footer action row -->
  <div class="footer-row">
    <Button variant="secondary" loading={validating} onclick={validateAndPreview}>
      Проверить (превью)
    </Button>
    <Button variant="primary" loading={saving} onclick={saveTemplate}>Сохранить шаблон</Button>
    <Button variant="destructive" onclick={() => (confirmReset = true)}>
      Сбросить до умолчания
    </Button>
  </div>

  <!-- HTML preview iframe (Plan 17-03, D-11: srcdoc, no blob/PDF object URL) -->
  {#if previewHtml}
    <div class="preview-wrapper">
      <iframe srcdoc={previewHtml} title="Превью" class="pdf-iframe"></iframe>
    </div>
  {/if}
</section>

<!-- Reset confirmation modal -->
<Modal
  open={confirmReset}
  title="Сбросить шаблон?"
  size="md"
  onClose={() => (confirmReset = false)}
>
  <p class="modal-body-text">
    Шаблон «{selectedTemplate
      ? (KIND_LABELS[selectedTemplate.kind] ?? selectedTemplate.label ?? selectedTemplate.kind)
      : ''}» будет заменён версией по умолчанию. Ваши изменения будут потеряны.
  </p>
  {#snippet footer()}
    <Button variant="secondary" onclick={() => (confirmReset = false)}>Отмена</Button>
    <Button variant="destructive" loading={resetting} onclick={resetTemplate}>Сбросить</Button>
  {/snippet}
</Modal>

<style lang="scss">
  /* Full-width exception per UI-SPEC §Settings Layout (SET-09, D-20) */
  .settings-section {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    padding: var(--space-lg);
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
  }

  /* Do NOT add max-width: 640px here — template editor is full-width */
  .settings-section--full-width {
    max-width: none;
  }

  .section-title {
    margin: 0;
    font-size: var(--font-size-heading);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
  }

  .template-selector-row {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    flex-wrap: wrap;
  }

  .form-label {
    font-size: var(--font-size-label);
    font-weight: var(--font-weight-medium);
    color: var(--color-text-secondary);
    white-space: nowrap;
  }

  .form-select {
    padding: var(--space-sm) var(--space-md);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    font-size: var(--font-size-body);
    background: var(--color-bg);
    color: var(--color-text-primary);
    min-width: 220px;

    &:focus {
      outline: none;
      border-color: var(--color-accent);
      box-shadow: 0 0 0 2px color-mix(in srgb, var(--color-accent) 20%, transparent);
    }
  }

  .variables-panel {
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    padding: 0;

    &[open] {
      padding-bottom: var(--space-sm);
    }
  }

  .variables-summary {
    padding: var(--space-sm) var(--space-md);
    font-size: var(--font-size-label);
    font-weight: var(--font-weight-medium);
    color: var(--color-text-secondary);
    cursor: pointer;
    user-select: none;

    &:hover {
      color: var(--color-text-primary);
    }

    &:focus-visible {
      outline: none;
      box-shadow: 0 0 0 3px var(--color-accent-focus);
      border-radius: var(--radius-sm);
    }
  }

  .variables-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0 var(--space-lg);
    padding: var(--space-sm) var(--space-md);
  }

  .var-col {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .var-item {
    margin: 0;
    font-size: var(--font-size-label);
    color: var(--color-text-secondary);
    line-height: 1.6;

    code {
      font-family: monospace;
      background: var(--color-surface-sunken);
      padding: 1px var(--space-xs);
      border-radius: var(--radius-sm);
      color: var(--color-text-primary);
    }
  }

  .editor-wrapper {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }

  .unsaved-indicator {
    font-size: var(--font-size-label);
    color: var(--color-warning);
    font-weight: var(--font-weight-medium);
    align-self: flex-end;
  }

  .template-textarea {
    font-family: monospace;
    font-size: var(--font-size-body);
    min-height: 320px;
    width: 100%;
    resize: vertical;
    padding: var(--space-md);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-bg);
    color: var(--color-text-primary);
    line-height: 1.6;
    box-sizing: border-box;

    &:focus {
      outline: none;
      border-color: var(--color-accent);
      box-shadow: 0 0 0 2px color-mix(in srgb, var(--color-accent) 20%, transparent);
    }
  }

  .footer-row {
    display: flex;
    gap: var(--space-sm);
    flex-wrap: wrap;
    align-items: center;
  }

  .preview-wrapper {
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    overflow: hidden;
  }

  .pdf-iframe {
    width: 100%;
    height: 400px;
    border: none;
    display: block;
  }

  .modal-body-text {
    margin: 0;
    font-size: var(--font-size-body);
    color: var(--color-text-primary);
    line-height: 1.5;
  }
</style>
