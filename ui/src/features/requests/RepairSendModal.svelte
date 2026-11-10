<script lang="ts">
  // Фаза 41.4, план 16 (D-05, D-25, D-27): попап «Отправить в ремонт».
  // Фото выбираются кнопкой и грузятся вложениями заявки (41.3) ПОСЛЕ успешной
  // отправки: до создания захода родителя у них нет. Ошибки файлов — списком
  // под полем, без тостов; сама отправка к тому моменту уже состоялась.
  import { untrack } from 'svelte';
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';
  import DatePicker from '$lib/components/DatePicker.svelte';
  import Textarea from '$lib/components/Textarea.svelte';
  import PersonAutocomplete from '$lib/components/PersonAutocomplete.svelte';
  import PlacePicker from '$lib/components/PlacePicker.svelte';
  import { authStore } from '$lib/stores/auth.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { formatFileSize } from '$lib/utils/formatFileSize';
  import { attachments, uploadAttachment } from '../attachments/api';
  import { repairs } from './api';
  import type { RepairStateDto, RepairTransitionResultDto, RequestDto } from '../../bindings';

  interface Props {
    open: boolean;
    request: RequestDto;
    repairState: RepairStateDto;
    onClose: () => void;
    onSuccess: (_result: RepairTransitionResultDto) => void;
  }

  const { open, request, onClose, onSuccess }: Props = $props();

  // Тот же whitelist, что у блока вложений 41.3; решает всё равно сервер.
  const ACCEPT =
    '.jpg,.jpeg,.png,.webp,.gif,.heic,.heif,.pdf,.doc,.docx,.xls,.xlsx,.odt,.ods,.txt,.csv,.log,.zip,.7z';

  let dateIso = $state('');
  let giverName = $state('');
  let receiverName = $state('');
  let servicePlaceId = $state<number | null>(null);
  let fault = $state('');
  let files = $state<globalThis.File[]>([]);
  let submitting = $state(false);
  // После успешной отправки повторять её нельзя: показываем итог загрузки фото.
  let sent = $state(false);
  let fileErrors = $state<string[]>([]);
  let fileInput = $state<HTMLInputElement | null>(null);

  let giverError = $state('');
  let receiverError = $state('');
  let serviceError = $state('');
  let faultError = $state('');

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
      giverName = authStore.user?.fullName ?? '';
      receiverName = '';
      servicePlaceId = null;
      fault = request.description ?? '';
      files = [];
      submitting = false;
      sent = false;
      fileErrors = [];
      giverError = '';
      receiverError = '';
      serviceError = '';
      faultError = '';
    });
    (async () => {
      try {
        const last = await repairs.lastPlaces();
        if (!cancelled && servicePlaceId === null) servicePlaceId = last.servicePlaceId;
      } catch {
        // Предзаполнение — удобство, не условие.
      }
    })();
    return () => {
      cancelled = true;
    };
  });

  function onFilesPicked(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const picked = Array.from(input.files ?? []);
    files = [...files, ...picked];
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
    giverError = giverName.trim() ? '' : 'Укажите, кто выдал устройство.';
    receiverError = receiverName.trim() ? '' : 'Укажите, кому выдано устройство.';
    serviceError = servicePlaceId === null ? 'Выберите сервис.' : '';
    faultError = fault.trim() ? '' : 'Опишите неисправность.';
    return !giverError && !receiverError && !serviceError && !faultError;
  }

  async function uploadPhotos(): Promise<string[]> {
    const lines: string[] = [];
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
      }
    }
    return lines;
  }

  async function handleSubmit() {
    if (submitting || sent) return;
    if (!validate()) return;
    submitting = true;
    let result: RepairTransitionResultDto;
    try {
      result = await repairs.send({
        requestId: request.id,
        version: request.version,
        dateUtc: isoToUnix(dateIso),
        giverName: giverName.trim(),
        receiverName: receiverName.trim(),
        servicePlaceId: servicePlaceId as number,
        faultDescription: fault.trim(),
      });
    } catch {
      pushToast('error', 'Не удалось отправить устройство в ремонт. Попробуйте ещё раз.');
      submitting = false;
      return;
    }
    // Отправка состоялась: один тост на событие, дальше — только фото.
    pushToast('success', 'Устройство отправлено в ремонт');
    onSuccess(result);
    sent = true;
    const lines = files.length > 0 ? await uploadPhotos() : [];
    files = [];
    submitting = false;
    if (lines.length === 0) {
      onClose();
    } else {
      fileErrors = lines;
    }
  }
</script>

<Modal {open} title="Отправить в ремонт" size="md" {onClose}>
  <form
    class="form"
    onsubmit={(e) => {
      e.preventDefault();
      void handleSubmit();
    }}
  >
    {#if sent}
      <p class="done">Устройство отправлено в ремонт. Часть файлов приложить не удалось.</p>
      <ul class="file-errors" role="alert">
        {#each fileErrors as line (line)}
          <li>{line}</li>
        {/each}
      </ul>
      <span class="hint">Файлы можно приложить позже в блоке «Вложения» заявки.</span>
    {:else}
      <div class="field">
        <label class="label" for="rep-send-date">Дата</label>
        <DatePicker bind:value={dateIso} id="rep-send-date" required />
      </div>

      <div class="field">
        <label class="label" for="rep-send-giver">Кто выдал</label>
        <PersonAutocomplete
          field="giver"
          bind:value={giverName}
          placeholder="ФИО выдавшего"
          id="rep-send-giver"
          invalid={!!giverError}
        />
        {#if giverError}<span class="error" role="alert">{giverError}</span>{/if}
      </div>

      <div class="field">
        <label class="label" for="rep-send-receiver">Кому выдал</label>
        <PersonAutocomplete
          field="receiver"
          bind:value={receiverName}
          placeholder="Сотрудник сервиса"
          id="rep-send-receiver"
          invalid={!!receiverError}
        />
        {#if receiverError}<span class="error" role="alert">{receiverError}</span>{/if}
      </div>

      <div class="field">
        <label class="label" for="rep-send-service">Сервис</label>
        <PlacePicker
          value={servicePlaceId}
          id="rep-send-service"
          invalid={!!serviceError}
          onChange={(id) => (servicePlaceId = id)}
        />
        {#if serviceError}<span class="error" role="alert">{serviceError}</span>{/if}
      </div>

      <div class="field">
        <label class="label" for="rep-send-fault">Описание неисправности</label>
        <Textarea bind:value={fault} id="rep-send-fault" rows={4} invalid={!!faultError} />
        {#if faultError}<span class="error" role="alert">{faultError}</span>{/if}
      </div>

      <div class="field">
        <span class="label">Фото неисправности</span>
        <div>
          <Button variant="secondary" size="sm" onclick={() => fileInput?.click()}>
            Приложить фото
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
    {/if}
  </form>

  {#snippet footer()}
    {#if sent}
      <Button variant="primary" disabled={submitting} onclick={onClose}>Закрыть</Button>
    {:else}
      <Button variant="secondary" disabled={submitting} onclick={onClose}>Отмена</Button>
      <Button variant="primary" loading={submitting} onclick={handleSubmit}>
        Отправить устройство
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
