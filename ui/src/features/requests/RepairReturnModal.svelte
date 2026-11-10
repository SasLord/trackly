<script lang="ts">
  // Фаза 41.4, план 16 (D-06, D-07, D-08, D-29): попап «Забрать из ремонта».
  // Стоимость уходит строкой: разбор и текст ошибки приходят с сервера (JS-зеркала
  // формулы нет). Файлы грузятся вложениями заявки после забора и связываются с
  // заходом через repairs.linkScans.
  import { untrack } from 'svelte';
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';
  import Checkbox from '$lib/components/Checkbox.svelte';
  import DatePicker from '$lib/components/DatePicker.svelte';
  import Input from '$lib/components/Input.svelte';
  import Textarea from '$lib/components/Textarea.svelte';
  import PersonAutocomplete from '$lib/components/PersonAutocomplete.svelte';
  import PlacePicker from '$lib/components/PlacePicker.svelte';
  import { apiCall } from '$lib/api/client';
  import { parseAppError } from '$lib/api/errors';
  import { authStore } from '$lib/stores/auth.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { formatFileSize } from '$lib/utils/formatFileSize';
  import { attachments, uploadAttachment } from '../attachments/api';
  import { repairs } from './api';
  import type {
    PlaceDto,
    RepairStateDto,
    RepairTransitionResultDto,
    RequestDto,
  } from '../../bindings';

  interface Props {
    open: boolean;
    request: RequestDto;
    repairState: RepairStateDto;
    onClose: () => void;
    onSuccess: (_result: RepairTransitionResultDto) => void;
  }

  const { open, request, repairState, onClose, onSuccess }: Props = $props();

  const ACCEPT =
    '.jpg,.jpeg,.png,.webp,.gif,.heic,.heif,.pdf,.doc,.docx,.xls,.xlsx,.odt,.ods,.txt,.csv,.log,.zip,.7z';
  const MAX_QUICK_PICK = 4;
  const COST_ERROR_FALLBACK = 'Введите сумму числом, например 1500 или 1500,50.';

  let dateIso = $state('');
  let receivedBy = $state('');
  let placeId = $state<number | null>(null);
  let returnToWorkplace = $state(false);
  let worksDone = $state('');
  let cost = $state('');
  let files = $state<globalThis.File[]>([]);
  let submitting = $state(false);
  let done = $state(false);
  let fileErrors = $state<string[]>([]);
  let fileInput = $state<HTMLInputElement | null>(null);
  // Предзаполнение «Куда вернуть» без отметки (D-25) — запоминаем, чтобы вернуть
  // его при снятии отметки.
  let lastMovePlaceId = $state<number | null>(null);

  let storageIds = $state<number[]>([]);
  let storageLabels = $state<Record<number, string>>({});

  let receivedByError = $state('');
  let placeError = $state('');
  let costError = $state('');

  function todayIso(): string {
    const now = new Date();
    const m = String(now.getMonth() + 1).padStart(2, '0');
    const d = String(now.getDate()).padStart(2, '0');
    return `${now.getFullYear()}-${m}-${d}`;
  }

  function isoToUnix(iso: string): number {
    if (!iso) return Math.floor(Date.now() / 1000);
    return Math.floor(new Date(iso + 'T00:00:00Z').getTime() / 1000);
  }

  $effect(() => {
    if (!open) return;
    let cancelled = false;
    untrack(() => {
      dateIso = todayIso();
      receivedBy = authStore.user?.fullName ?? '';
      placeId = null;
      returnToWorkplace = false;
      worksDone = '';
      cost = '';
      files = [];
      submitting = false;
      done = false;
      fileErrors = [];
      lastMovePlaceId = null;
      storageIds = [];
      storageLabels = {};
      receivedByError = '';
      placeError = '';
      costError = '';
    });
    (async () => {
      try {
        const last = await repairs.lastPlaces();
        if (cancelled) return;
        lastMovePlaceId = last.movePlaceId;
        if (placeId === null && !returnToWorkplace) placeId = last.movePlaceId;
      } catch {
        // Предзаполнение — удобство, не условие.
      }
      try {
        const ids = (await apiCall<number[]>('cartridge_storage_place_ids', {})).slice(
          0,
          MAX_QUICK_PICK,
        );
        if (cancelled) return;
        storageIds = ids;
        const labels: Record<number, string> = {};
        await Promise.all(
          ids.map(async (id) => {
            try {
              const place = await apiCall<PlaceDto>('places_get', { id });
              labels[id] = place.full_path ?? place.name;
            } catch {
              labels[id] = `#${id}`;
            }
          }),
        );
        if (!cancelled) storageLabels = labels;
      } catch {
        if (!cancelled) storageIds = [];
      }
    })();
    return () => {
      cancelled = true;
    };
  });

  function onToggleWorkplace(checked: boolean) {
    returnToWorkplace = checked;
    placeError = '';
    if (checked) {
      placeId = repairState.suggestedReturnPlaceId ?? placeId;
    } else {
      placeId = lastMovePlaceId ?? placeId;
    }
  }

  function onFilesPicked(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    files = [...files, ...Array.from(input.files ?? [])];
    input.value = '';
  }

  function removeFile(index: number) {
    files = files.filter((_, i) => i !== index);
  }

  function rejectLine(name: string, code: string, size: number, maxBytes: number): string {
    switch (code) {
      case 'too_large':
        return `«${name}» — больше ${Math.round(maxBytes / (1024 * 1024))} МБ (размер файла ${formatFileSize(size)}).`;
      case 'empty':
        return `«${name}» — файл пустой.`;
      case 'type_not_allowed':
        return `«${name}» — такой файл приложить нельзя. Подходят фото (JPEG, PNG, WebP, GIF, HEIC), PDF, документы Word, Excel и OpenDocument, TXT, CSV, LOG, архивы ZIP и 7z.`;
      case 'heic_failed':
        return `«${name}» — не удалось преобразовать фото HEIC. Сохраните его в JPEG и приложите снова.`;
      case 'forbidden':
        return `«${name}» — нет доступа к карточке или она удалена.`;
      default:
        return `«${name}» — не удалось загрузить. Проверьте подключение и попробуйте ещё раз.`;
    }
  }

  function validate(): boolean {
    receivedByError = receivedBy.trim() ? '' : 'Укажите, кто принял устройство.';
    placeError = placeId === null ? 'Выберите место.' : '';
    return !receivedByError && !placeError;
  }

  /** Загрузка файлов и привязка к заходу; возвращает строки ошибок. */
  async function attachFiles(result: RepairTransitionResultDto): Promise<string[]> {
    const lines: string[] = [];
    const ids: number[] = [];
    let maxBytes = 0;
    for (const file of files) {
      const res = await uploadAttachment('request', request.id, file);
      if (res.rejectCode !== null || res.attachment === null) {
        const code = res.rejectCode ?? 'network';
        if (code === 'too_large' && maxBytes === 0) {
          try {
            maxBytes = (await attachments.getLimit()).maxBytes;
          } catch {
            maxBytes = 0;
          }
        }
        lines.push(rejectLine(file.name, code, file.size, maxBytes));
      } else {
        ids.push(res.attachment.id);
      }
    }
    if (ids.length > 0) {
      // Заход, с которого только что забрали устройство, — с наибольшим seq.
      const visits = result.state.visits;
      const visit = visits.reduce((a, b) => (b.seq > a.seq ? b : a), visits[0]);
      try {
        if (visit) await repairs.linkScans(visit.id, ids);
      } catch {
        lines.push(
          'Файлы загружены, но не привязаны к заходу. Они доступны в блоке «Вложения» заявки.',
        );
      }
    }
    return lines;
  }

  async function handleSubmit() {
    if (submitting || done) return;
    costError = '';
    if (!validate()) return;
    submitting = true;
    let result: RepairTransitionResultDto;
    try {
      result = await repairs.receive({
        requestId: request.id,
        version: request.version,
        dateUtc: isoToUnix(dateIso),
        receivedByName: receivedBy.trim(),
        placeId: placeId as number,
        returnToWorkplace,
        worksDone: worksDone.trim() || null,
        cost: cost.trim() || null,
      });
    } catch (e) {
      const err = parseAppError(e);
      if (err.code === 'VALIDATION' && err.details?.field === 'cost') {
        costError =
          typeof err.details.message === 'string' && err.details.message
            ? err.details.message
            : COST_ERROR_FALLBACK;
      } else {
        pushToast('error', 'Не удалось забрать устройство из ремонта. Попробуйте ещё раз.');
      }
      submitting = false;
      return;
    }
    pushToast(
      'success',
      returnToWorkplace
        ? 'Устройство забрано из ремонта, заявка завершена'
        : 'Устройство забрано из ремонта',
    );
    onSuccess(result);
    done = true;
    const lines = files.length > 0 ? await attachFiles(result) : [];
    files = [];
    submitting = false;
    if (lines.length === 0) {
      onClose();
    } else {
      fileErrors = lines;
    }
  }

  const confirmLabel = $derived(
    returnToWorkplace ? 'Забрать и завершить заявку' : 'Забрать устройство',
  );
</script>

<Modal {open} title="Забрать из ремонта" size="md" {onClose}>
  <form
    class="form"
    onsubmit={(e) => {
      e.preventDefault();
      void handleSubmit();
    }}
  >
    {#if done}
      <p class="done">Устройство забрано из ремонта. Часть файлов приложить не удалось.</p>
      <ul class="file-errors" role="alert">
        {#each fileErrors as line (line)}
          <li>{line}</li>
        {/each}
      </ul>
      <span class="hint">Файлы можно приложить позже в блоке «Вложения» заявки.</span>
    {:else}
      <div class="field">
        <label class="label" for="rep-ret-date">Дата возврата</label>
        <DatePicker bind:value={dateIso} id="rep-ret-date" required />
      </div>

      <div class="field">
        <label class="label" for="rep-ret-by">Кто принял</label>
        <PersonAutocomplete
          field="receiver"
          bind:value={receivedBy}
          placeholder="ФИО принявшего"
          id="rep-ret-by"
          invalid={!!receivedByError}
        />
        {#if receivedByError}<span class="error" role="alert">{receivedByError}</span>{/if}
      </div>

      <div class="field">
        <label class="label" for="rep-ret-place"
          >{returnToWorkplace ? 'Рабочее место' : 'Куда вернуть'}</label
        >
        {#if !returnToWorkplace && storageIds.length > 0}
          <div class="chips" role="group" aria-label="Быстрый выбор склада">
            {#each storageIds as id (id)}
              <Button
                variant={placeId === id ? 'primary' : 'secondary'}
                size="sm"
                ariaPressed={placeId === id}
                onclick={() => {
                  placeId = id;
                  placeError = '';
                }}
              >
                {storageLabels[id] ?? `#${id}`}
              </Button>
            {/each}
          </div>
        {/if}
        <PlacePicker
          value={placeId}
          id="rep-ret-place"
          invalid={!!placeError}
          onChange={(id) => {
            placeId = id;
            placeError = '';
          }}
        />
        {#if placeError}<span class="error" role="alert">{placeError}</span>{/if}
      </div>

      <div class="field">
        <Checkbox checked={returnToWorkplace} onchange={onToggleWorkplace}>
          Сразу вернуть на рабочее место
        </Checkbox>
        {#if returnToWorkplace}
          <span class="hint">Устройство вернётся в работу, заявка будет завершена.</span>
        {/if}
      </div>

      <div class="field">
        <div class="label-row">
          <label class="label" for="rep-ret-works">Выполненные работы</label>
          <span class="optional">необязательно</span>
        </div>
        <Textarea
          bind:value={worksDone}
          id="rep-ret-works"
          rows={3}
          placeholder="Что сделал сервис"
        />
      </div>

      <div class="field">
        <div class="label-row">
          <label class="label" for="rep-ret-cost">Стоимость, ₽</label>
          <span class="optional">необязательно</span>
        </div>
        <div class="cost">
          <Input
            bind:value={cost}
            id="rep-ret-cost"
            inputmode="decimal"
            invalid={!!costError}
            oninput={() => (costError = '')}
          />
        </div>
        {#if costError}<span class="error" role="alert">{costError}</span>{/if}
      </div>

      <div class="field">
        <div class="label-row">
          <span class="label">Скан или фото результатов</span>
          <span class="optional">необязательно</span>
        </div>
        <div>
          <Button variant="secondary" size="sm" onclick={() => fileInput?.click()}>
            Приложить файлы
          </Button>
        </div>
        <input
          bind:this={fileInput}
          class="file-input"
          type="file"
          accept={ACCEPT}
          multiple
          tabindex="-1"
          aria-hidden="true"
          onchange={onFilesPicked}
        />
        {#if files.length > 0}
          <ul class="files">
            {#each files as file, i (i)}
              <li class="file">
                <span class="file-name">{file.name}</span>
                <button
                  type="button"
                  class="file-remove"
                  aria-label={`Убрать файл «${file.name}»`}
                  onclick={() => removeFile(i)}>×</button
                >
              </li>
            {/each}
          </ul>
        {/if}
      </div>

      <span class="hint"
        >Устройство могут вернуть без документов или по гарантии — поля можно оставить пустыми.</span
      >
    {/if}
  </form>

  {#snippet footer()}
    {#if done}
      <Button variant="primary" disabled={submitting} onclick={onClose}>Закрыть</Button>
    {:else}
      <Button variant="secondary" disabled={submitting} onclick={onClose}>Отмена</Button>
      <Button variant="primary" loading={submitting} onclick={handleSubmit}>
        {confirmLabel}
      </Button>
    {/if}
  {/snippet}
</Modal>

<style lang="scss">
  .form {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-md);
    white-space: normal;
    text-align: left;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-2xs);
  }
  .label {
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-secondary);
  }
  .label-row {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--tr-space-xs);
  }
  .optional {
    font: var(--tr-text-caption);
    color: var(--tr-text-tertiary);
  }
  .hint {
    font: var(--tr-text-caption);
    color: var(--tr-text-tertiary);
  }
  .error {
    font-size: var(--tr-font-size-label);
    color: var(--tr-danger);
  }
  .done {
    margin: 0;
    color: var(--tr-text-primary);
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--tr-space-2xs);
  }
  .cost {
    width: 160px;
  }
  .file-input {
    display: none;
  }
  .files,
  .file-errors {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-2xs);
    font-size: var(--tr-font-size-label);
  }
  .file-errors {
    color: var(--tr-danger);
  }
  .file {
    display: flex;
    align-items: center;
    gap: var(--tr-space-xs);
  }
  .file-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .file-remove {
    border: 0;
    background: transparent;
    color: var(--tr-text-secondary);
    cursor: pointer;
    font-size: 16px;
    line-height: 1;
    padding: 0 var(--tr-space-2xs);
  }
</style>
