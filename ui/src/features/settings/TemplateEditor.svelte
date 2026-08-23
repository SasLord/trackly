<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import Button from '$lib/components/Button.svelte';
  import Modal from '$lib/components/Modal.svelte';
  // Plan 28-12 (GAP-1): Select (нативный <select>) заменён на кастомный Dropdown
  // (flat + variant="select") — implicit-label pattern, как в CartridgeFormBody.svelte
  // (Phase 27-G1 precedent). D-08: strictly the kind-selector control only — the
  // rest of this component's editing/preview surface is untouched.
  import Dropdown from '$lib/components/Dropdown.svelte';
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

  // WR-05 (D-17): per-file on-disk status. The backend endpoint shipped in
  // Phase 34 with zero consumers on either transport — this is the consumer it
  // was built for. Mirrors `TemplateStatusDto` / `TemplateFileStatus` in
  // bindings.ts (snake_case: the DTO has no camelCase rename).
  type TemplateFileStatus = 'current' | 'customized' | 'unreadable';
  interface TemplateStatusItem {
    filename: string;
    status: TemplateFileStatus;
    templates_dir: string;
  }

  const STATUS_BADGE: Record<
    Exclude<TemplateFileStatus, 'current'>,
    { label: string; title: string; kind: 'warn' | 'error' }
  > = {
    customized: {
      label: 'изменён вручную',
      title:
        'Файл шаблона на диске отличается от встроенного по умолчанию — обновления шаблона из новых версий приложения к нему больше не применяются автоматически.',
      kind: 'warn',
    },
    unreadable: {
      label: 'файл не читается',
      title:
        'Файл шаблона существует, но не читается как UTF-8 (например, сохранён в ANSI/Windows-1251). Печать использует встроенный шаблон по умолчанию, ваши правки не применяются. Сохраните файл в кодировке UTF-8.',
      kind: 'error',
    },
  };

  // WR-08: all three forms now render the SAME shared `_header.html` partial,
  // so every kind gets the identical org.* block. Listed once here instead of
  // being re-typed (and drifting) per kind.
  //
  // `org.full_name` is documented as `org.full_name | safe` DELIBERATELY: the
  // backend pre-escapes it and converts newlines to `<br />`
  // (`pdf::minijinja_env::org_full_name_html`), so a user who follows this
  // panel and writes plain `{{ org.full_name }}` gets autoescaped output —
  // the literal text `<br />` and `&lt;` sequences printed on the act. The
  // `| safe` requirement previously existed only in the `_header.html`
  // doc-comment, which is exactly the file this editor hides.
  const ORG_VARIABLES: VariableEntry[] = [
    { code: 'org.name', desc: 'краткое название организации' },
    {
      code: 'org.full_name | safe',
      desc: 'полное юридическое наименование (многострочное, уже экранировано — используйте с | safe)',
    },
    { code: 'org.inn', desc: 'ИНН' },
    { code: 'org.kpp', desc: 'КПП' },
    { code: 'org.address', desc: 'адрес организации' },
    { code: 'org.address_line2', desc: 'адрес, вторая строка' },
    { code: 'org.phone', desc: 'телефон' },
    { code: 'org.fax', desc: 'факс' },
    { code: 'org.email', desc: 'e-mail' },
    { code: 'org.okpo', desc: 'ОКПО' },
    { code: 'org.ogrn', desc: 'ОГРН' },
    { code: 'org.logo_data_uri', desc: 'логотип (data: URI)' },
  ];

  // Plan 17-03 (D-12): per-kind variables panel — each entry mirrors the
  // context documented in the corresponding templates/*.html doc-comment.
  const VARIABLES_BY_KIND: Record<string, VariableEntry[]> = {
    act_handover: [
      ...ORG_VARIABLES,
      { code: 'act.number', desc: 'номер акта' },
      { code: 'act.suffix', desc: 'суффикс номера' },
      { code: 'act.date_human', desc: 'дата акта (человекочитаемая)' },
      { code: 'act.receiver_name', desc: 'кто принял' },
      { code: 'act.deadline_human', desc: 'срок до (человекочитаемый)' },
      { code: 'act.place_path', desc: 'расположение' },
      {
        code: 'act.items[]',
        desc: 'позиции: name, inventory_no, serial_no, model, specs, kit, condition, quantity',
      },
    ],
    act_acceptance: [
      ...ORG_VARIABLES,
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
      ...ORG_VARIABLES,
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
  let templateStatuses = $state<TemplateStatusItem[]>([]);

  // Svelte 5: $derived for unsaved changes indicator
  const isDirty = $derived(body !== originalBody);

  // The currently selected template object
  const selectedTemplate = $derived(templates.find((t) => t.kind === selectedKind) ?? null);

  // GAP-1: опции для Dropdown (flat + variant="select") — «Шаблон».
  const templateOptions = $derived(
    templates.map((tmpl) => ({
      id: tmpl.kind,
      label: KIND_LABELS[tmpl.kind] ?? tmpl.label ?? tmpl.kind,
    })),
  );
  const selectedKindLabel = $derived(
    templateOptions.find((o) => o.id === selectedKind)?.label ?? '',
  );
  // Плоские опции без drill-in — onExpandGroup никогда реально не вызывается
  // (isGroupExpandable всегда false), но Dropdown требует типизированную
  // функцию, чтобы вывести TMember (иначе `() => []` выводит `never[]`).
  function noExpandKind(): { id: string; label: string }[] {
    return [];
  }

  // Plan 17-03 (D-12): per-kind variables panel content
  const currentVariables = $derived(VARIABLES_BY_KIND[selectedKind] ?? []);

  // WR-05: on-disk status of the currently selected kind, if it is not
  // `current`. `null` renders no badge at all — the common case.
  const currentStatusBadge = $derived.by(() => {
    const entry = templateStatuses.find((s) => s.filename === `${selectedKind}.html`);
    if (!entry || entry.status === 'current') return null;
    return STATUS_BADGE[entry.status] ?? null;
  });

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

  // WR-05: read-only, ManageSettings-gated. Failure is non-fatal — the badge
  // is informational, so a lost status must never block editing or raise a
  // toast the user cannot act on.
  async function loadTemplateStatuses() {
    try {
      templateStatuses = await apiCall<TemplateStatusItem[]>('templates_status', {});
    } catch {
      templateStatuses = [];
    }
  }

  onMount(() => {
    loadTemplates();
    loadTemplateStatuses();
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
      // Saving makes the file differ from the bundled default — refresh so the
      // badge appears immediately rather than after a reload.
      await loadTemplateStatuses();
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
      // Reset makes the file byte-identical to the default again — the badge
      // must disappear.
      await loadTemplateStatuses();
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
    <label class="form-label dropdown-label">
      <span>Шаблон</span>
      <div class="select-shrink">
        <Dropdown
          variant="select"
          flat={true}
          value={selectedKindLabel}
          placeholder="Выберите шаблон"
          searchPlaceholder="Поиск"
          loading={false}
          groups={templateOptions}
          getGroupId={(o) => o.id}
          getGroupName={(o) => o.label}
          getGroupCount={() => 0}
          isGroupExpandable={() => false}
          isGroupSelected={(o) => o.id === selectedKind}
          onExpandGroup={noExpandKind}
          getMemberId={(o) => o.id}
          getMemberName={(o) => o.label}
          onSearch={() => {}}
          onPickGroup={(o) => (selectedKind = o.id)}
          onPickMember={() => {}}
        />
      </div>
    </label>

    <!-- WR-05 (D-17): on-disk status of the selected template file. Rendered
         only when the file is NOT byte-identical to the bundled default, so the
         normal case shows nothing. -->
    {#if currentStatusBadge}
      <span
        class="status-badge"
        class:status-badge--error={currentStatusBadge.kind === 'error'}
        title={currentStatusBadge.title}
      >
        {currentStatusBadge.label}
      </span>
    {/if}
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
      <iframe sandbox="" srcdoc={previewHtml} title="Превью" class="pdf-iframe"></iframe>
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
    background: var(--tr-surface);
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-md);
    padding: var(--tr-space-xl);
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-md);
  }

  /* Do NOT add max-width: 640px here — template editor is full-width */
  .settings-section--full-width {
    max-width: none;
  }

  .section-title {
    margin: 0;
    font-size: var(--tr-font-size-h3);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-primary);
  }

  .template-selector-row {
    display: flex;
    align-items: center;
    gap: var(--tr-space-xs);
    flex-wrap: wrap;
  }

  .status-badge {
    align-self: flex-end;
    padding: 2px var(--tr-space-xs);
    border-radius: var(--tr-radius-xs);
    font-size: var(--tr-font-size-label);
    font-weight: var(--tr-font-weight-medium);
    color: var(--tr-warning);
    border: 1px solid var(--tr-warning);
    white-space: nowrap;
    cursor: help;

    &.status-badge--error {
      color: var(--tr-danger);
      border-color: var(--tr-danger);
    }
  }

  .form-label {
    font-size: var(--tr-font-size-label);
    font-weight: var(--tr-font-weight-medium);
    color: var(--tr-text-secondary);
    white-space: nowrap;
  }

  // Plan 28-12 (GAP-1): Dropdown не принимает `id`, поэтому подпись оборачивает
  // поле (implicit label) вместо `for`/`id` association.
  .dropdown-label {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-2xs);
    white-space: normal;
  }

  .select-shrink {
    width: fit-content;
    min-width: 220px;
  }

  .variables-panel {
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-xs);
    padding: 0;

    &[open] {
      padding-bottom: var(--tr-space-xs);
    }
  }

  .variables-summary {
    padding: var(--tr-space-xs) var(--tr-space-md);
    font-size: var(--tr-font-size-label);
    font-weight: var(--tr-font-weight-medium);
    color: var(--tr-text-secondary);
    cursor: pointer;
    user-select: none;

    &:hover {
      color: var(--tr-text-primary);
    }

    &:focus-visible {
      outline: none;
      box-shadow: 0 0 0 3px var(--tr-focus-ring);
      border-radius: var(--tr-radius-xs);
    }
  }

  .variables-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0 var(--tr-space-xl);
    padding: var(--tr-space-xs) var(--tr-space-md);
  }

  .var-col {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .var-item {
    margin: 0;
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-secondary);
    line-height: 1.6;

    code {
      font-family: monospace;
      background: var(--tr-surface-sunken);
      padding: 1px var(--tr-space-2xs);
      border-radius: var(--tr-radius-xs);
      color: var(--tr-text-primary);
    }
  }

  .editor-wrapper {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-2xs);
  }

  .unsaved-indicator {
    font-size: var(--tr-font-size-label);
    color: var(--tr-warning);
    font-weight: var(--tr-font-weight-medium);
    align-self: flex-end;
  }

  .template-textarea {
    font-family: monospace;
    font-size: var(--tr-font-size-body);
    min-height: 320px;
    width: 100%;
    resize: vertical;
    padding: var(--tr-space-md);
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-xs);
    background: var(--tr-bg);
    color: var(--tr-text-primary);
    line-height: 1.6;
    box-sizing: border-box;

    &:focus {
      outline: none;
      border-color: var(--tr-accent);
      box-shadow: 0 0 0 2px color-mix(in srgb, var(--tr-accent) 20%, transparent);
    }
  }

  .footer-row {
    display: flex;
    gap: var(--tr-space-xs);
    flex-wrap: wrap;
    align-items: center;
  }

  .preview-wrapper {
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-xs);
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
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-primary);
    line-height: 1.5;
  }
</style>
