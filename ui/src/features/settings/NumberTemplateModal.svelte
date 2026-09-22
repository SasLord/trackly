<script lang="ts">
  // Phase 40.2 Plan 10 (NUM-01/NUM-03) — модалка создания/правки шаблона
  // инвентарного номера. Использует ТОЛЬКО команды группы A плана 05
  // (Action::ManageSettings): number_templates_create/_update_mask/_preview_mask.
  //
  // D-10: тип шаблона неизменяем в правке — Dropdown `disabled` + подсказка.
  // Предпросмотр и валидация маски — ВСЕГДА с сервера (js_rust_mirror_needs_
  // fixture_gate): 300 мс debounce после изменения типа/маски, устаревшие
  // ответы отбрасываются (см. previewRequestId).
  import { onMount } from 'svelte';
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';
  import Input from '$lib/components/Input.svelte';
  import Dropdown from '$lib/components/Dropdown.svelte';
  import NumberMaskHelp from './NumberMaskHelp.svelte';
  import { apiCall } from '$lib/api/client';
  import { pushToast } from '$lib/stores/toast.svelte';
  import type { AppError } from '$lib/api/errors';
  import type { NextNumberDto, NumberTemplateDto, TemplateTypeDto } from '../../bindings';

  interface EditableTemplate {
    id: number;
    templateType: TemplateTypeDto;
    mask: string;
    version: number;
  }

  interface Props {
    mode: 'create' | 'edit';
    /** Обязателен при mode === 'edit'. */
    template?: EditableTemplate | null;
    onClose: () => void;
    /** Сигнал родителю: шаблон сохранён. Toast и перезагрузка списка — на
     *  стороне родителя (OrgSettings.svelte, Task 2) — модалка сама не
     *  показывает success-тост. */
    onSaved: () => void;
    /** FE-WR-15: шаблон изменён в другой сессии (OPTIMISTIC_LOCK_MISMATCH) —
     *  родитель перезагружает список (без success-тоста, в отличие от
     *  `onSaved`). */
    onStale?: () => void;
  }

  const { mode, template = null, onClose, onSaved, onStale }: Props = $props();

  // Ровно 4 значения — закрытый список (dto::number_template::TemplateTypeDto),
  // копия display_name_ru() из trackly-core, сверенная 1:1 (см. SUMMARY).
  const TEMPLATE_TYPE_OPTIONS: { id: TemplateTypeDto; label: string }[] = [
    { id: 'device_inventory', label: 'Устройства и принтеры' },
    { id: 'act_number', label: 'Акты' },
    { id: 'cartridge_code', label: 'Картриджи' },
    { id: 'drum_code', label: 'Фотобарабаны' },
  ];

  let templateType = $state<TemplateTypeDto | null>(
    mode === 'edit' ? (template?.templateType ?? null) : null,
  );
  let mask = $state(mode === 'edit' ? (template?.mask ?? '') : '');
  let maskTouched = $state(false);

  const selectedTypeLabel = $derived(
    TEMPLATE_TYPE_OPTIONS.find((o) => o.id === templateType)?.label ?? '',
  );

  // Dropdown требует типизированную функцию для вывода TMember — плоский
  // список без drill-in, onExpandGroup никогда реально не вызывается
  // (isGroupExpandable всегда false). Паттерн — CartridgeFormBody.svelte.
  function noExpandType(): { id: TemplateTypeDto; label: string }[] {
    return [];
  }

  // ---------------------------------------------------------------------
  // Серверный предпросмотр/валидация маски (300 мс debounce)
  // ---------------------------------------------------------------------
  let preview = $state<NextNumberDto | null>(null);
  let previewLoading = $state(false);
  let previewError = $state<string | null>(null);
  let previewRequestId = 0;

  function extractDetail(e: unknown, key: string): string | null {
    const err = e as { details?: unknown } | undefined;
    const details = err?.details;
    if (details && typeof details === 'object' && key in (details as Record<string, unknown>)) {
      const v = (details as Record<string, unknown>)[key];
      if (typeof v === 'string') return v;
    }
    return null;
  }

  $effect(() => {
    const type = templateType;
    const currentMask = mask;

    if (type === null || currentMask.trim() === '') {
      preview = null;
      previewError = null;
      previewLoading = false;
      return;
    }

    previewLoading = true;
    const requestId = ++previewRequestId;

    const timer = setTimeout(() => {
      apiCall<NextNumberDto>('number_templates_preview_mask', {
        templateType: type,
        mask: currentMask,
      })
        .then((dto) => {
          if (requestId !== previewRequestId) return; // устаревший ответ
          preview = dto;
          previewError = null;
          previewLoading = false;
        })
        .catch((e: unknown) => {
          if (requestId !== previewRequestId) return; // устаревший ответ
          preview = null;
          previewLoading = false;
          const err = e as Partial<AppError> | undefined;
          if (err?.code === 'VALIDATION') {
            previewError =
              extractDetail(e, 'message') ?? err.message ?? 'Маска не прошла проверку.';
          } else {
            previewError = 'Не удалось проверить маску.';
          }
        });
    }, 300);

    return () => clearTimeout(timer);
  });

  // ---------------------------------------------------------------------
  // Сохранение (объявлено здесь, чтобы fieldError ниже мог читать saveError)
  // ---------------------------------------------------------------------
  let saving = $state(false);
  let saveError = $state<string | null>(null);

  // «Введите маску.» — клиентская проверка (сервер её никогда не видит:
  // debounce выше не отправляет запрос для пустой маски). Показывается
  // только после того, как поле маски тронуто хотя бы раз — чтобы не
  // ругаться на пустое поле сразу при открытии модалки.
  const emptyMaskError = $derived(maskTouched && mask.trim() === '' ? 'Введите маску.' : null);
  const fieldError = $derived(emptyMaskError ?? previewError ?? saveError);

  const canSave = $derived(
    templateType !== null && mask.trim() !== '' && fieldError === null && !saving,
  );

  async function handleSave() {
    if (!canSave) return;
    saving = true;
    saveError = null;
    try {
      if (mode === 'create') {
        await apiCall<NumberTemplateDto>('number_templates_create', {
          templateType,
          mask: mask.trim(),
        });
      } else if (template) {
        await apiCall<NumberTemplateDto>('number_templates_update_mask', {
          id: template.id,
          mask: mask.trim(),
          version: template.version,
        });
      }
      onSaved();
      onClose();
    } catch (e: unknown) {
      const err = e as Partial<AppError> | undefined;
      if (err?.code === 'CONFLICT') {
        saveError = extractDetail(e, 'reason') ?? err.message ?? 'Такой шаблон уже есть.';
      } else if (err?.code === 'OPTIMISTIC_LOCK_MISMATCH') {
        // FE-WR-15: версия устарела — правка поверх чужой невозможна.
        pushToast(
          'error',
          'Шаблон изменён другим пользователем. Список обновлён — откройте шаблон и повторите правку.',
        );
        onStale?.();
        onClose();
      } else {
        pushToast('error', 'Не удалось сохранить шаблон. Попробуйте ещё раз.');
      }
    } finally {
      saving = false;
    }
  }

  // Автофокус на «Маска» в ОБОИХ режимах (UI-SPEC §5) — даже в create, где
  // «Тип» рендерится первым и не disabled, поэтому Modal.svelte's own
  // initial-focus effect (first focusable element) увело бы фокус на него.
  // setTimeout(0) гарантированно исполняется ПОСЛЕ синхронных $effect'ов
  // Modal.svelte в этом же тике/микротаске.
  const MASK_INPUT_ID = 'number-template-mask-input';
  onMount(() => {
    setTimeout(() => {
      document.getElementById(MASK_INPUT_ID)?.focus();
    }, 0);
  });
</script>

<Modal open={true} title={mode === 'create' ? 'Новый шаблон' : 'Изменить шаблон'} {onClose}>
  {#snippet children()}
    <div class="form">
      <div class="field">
        <label class="label dropdown-label">
          <span class="label-text">Тип</span>
          <Dropdown
            variant="select"
            flat={true}
            value={selectedTypeLabel}
            placeholder="Выберите тип"
            searchable={false}
            disabled={mode === 'edit'}
            loading={false}
            groups={TEMPLATE_TYPE_OPTIONS}
            getGroupId={(o) => o.id}
            getGroupName={(o) => o.label}
            getGroupCount={() => 0}
            isGroupExpandable={() => false}
            isGroupSelected={(o) => o.id === templateType}
            onExpandGroup={noExpandType}
            getMemberId={(o) => o.id}
            getMemberName={(o) => o.label}
            onSearch={() => {}}
            onPickGroup={(o) => {
              templateType = o.id;
              // FE-WR-15: «такой шаблон уже есть» относится к паре тип+маска —
              // новый тип снимает ошибку (иначе кнопка остаётся заблокированной).
              saveError = null;
            }}
            onPickMember={() => {}}
          />
        </label>
        {#if mode === 'edit'}
          <span class="field-hint">
            Тип нельзя изменить. Чтобы сменить тип, удалите шаблон и создайте новый.
          </span>
        {/if}
      </div>

      <div class="field">
        <label class="label" for={MASK_INPUT_ID}>Маска *</label>
        <Input
          id={MASK_INPUT_ID}
          mono
          value={mask}
          placeholder="Например, ОРГ-00-[XXXXXX]"
          invalid={fieldError !== null}
          aria-describedby={fieldError ? 'template-mask-error' : 'template-mask-preview'}
          oninput={(v) => {
            mask = v;
            maskTouched = true;
            saveError = null;
          }}
        />
        {#if fieldError}
          <span id="template-mask-error" class="field-error">{fieldError}</span>
        {/if}
      </div>

      <div class="field">
        <span class="label">Следующий номер</span>
        <div id="template-mask-preview" class="preview">
          {#if previewLoading}
            <span class="preview-value tr-mono">Считаем…</span>
          {:else if fieldError}
            <span class="preview-value tr-mono">—</span>
          {:else if preview}
            {#if preview.overflowed}
              <span class="preview-overflowed">Свободных номеров нет — шаблон переполнен.</span>
            {:else}
              <span class="preview-value tr-mono">{preview.rendered}</span>
              {#if preview.hasGap && preview.altRendered}
                <span class="preview-alt">(после максимального — {preview.altRendered})</span>
              {/if}
            {/if}
          {:else}
            <span class="preview-value tr-mono">—</span>
          {/if}
        </div>
      </div>

      <NumberMaskHelp />
    </div>
  {/snippet}
  {#snippet footer()}
    <Button variant="secondary" disabled={saving} onclick={onClose}>Отмена</Button>
    <Button variant="primary" loading={saving} disabled={!canSave} onclick={handleSave}>
      {mode === 'create' ? 'Создать шаблон' : 'Сохранить шаблон'}
    </Button>
  {/snippet}
</Modal>

<style lang="scss">
  .form {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-md);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-2xs);
  }

  .label {
    font-size: var(--tr-font-size-label);
    font-weight: var(--tr-font-weight-medium);
    color: var(--tr-text-secondary);
  }

  .dropdown-label {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-2xs);
  }

  .field-hint {
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-tertiary);
  }

  .field-error {
    font-size: var(--tr-font-size-label);
    color: var(--tr-danger-text);
  }

  .preview {
    display: flex;
    align-items: baseline;
    gap: var(--tr-space-2xs);
    flex-wrap: wrap;
  }

  .preview-value {
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-primary);
  }

  .preview-alt {
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-tertiary);
  }

  .preview-overflowed {
    font-size: var(--tr-font-size-label);
    color: var(--tr-warning-text);
  }
</style>
