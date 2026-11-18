<script lang="ts">
  // Поле ввода чата (D-07, D-08, D-14; UI-SPEC §10). Черновик — в draftStore
  // (переживает переключение вкладок). Отправка — одна операция: сначала файлы-
  // черновики, затем chat_send; отказ любого файла — сообщение не создаётся.
  // Ошибки выводятся по машинному коду, серверные тексты не показываются.
  import { untrack } from 'svelte';
  import Button from '$lib/components/Button.svelte';
  import { parseAppError } from '$lib/api/errors';
  import { formatFileSize } from '$lib/utils/formatFileSize';
  import AttachmentTypeIcon, { iconKindForMime } from '../../attachments/AttachmentTypeIcon.svelte';
  import { rejectLine } from '../../attachments/rejectText';
  import { uploadDraft, chat } from './api';
  import {
    addFiles as storeAddFiles,
    clearDraft,
    draft,
    removeChip,
    resetRejected,
  } from './draftStore.svelte';
  import type { ChatMessageDto } from '../../../bindings';

  interface Props {
    requestId: number;
    canPost: boolean;
    maxBytes: number;
    loaded: boolean;
    onSent: (_message: ChatMessageDto) => void;
    onClosed: () => void;
  }

  const { requestId, canPost, maxBytes, loaded, onSent, onClosed }: Props = $props();

  const ACCEPT =
    '.jpg,.jpeg,.png,.webp,.gif,.heic,.heif,.pdf,.doc,.docx,.xls,.xlsx,.odt,.ods,.txt,.csv,.log,.zip,.7z';
  const MAX_LEN = 4000;

  let textareaEl = $state<HTMLTextAreaElement | null>(null);
  let fileInput = $state<HTMLInputElement | null>(null);
  let sending = $state(false);
  // Превью, которое не загрузилось (например, CSP): вместо битой картинки — значок типа.
  let failedPreviews = $state<Record<string, boolean>>({});

  const hasContent = $derived(draft.text.trim().length > 0 || draft.chips.length > 0);
  const showCounter = $derived(draft.text.length >= 3800);
  const counterText = $derived(
    `${draft.text.length.toLocaleString('ru-RU').replace(/\s/g, ' ')} / ${MAX_LEN}`,
  );

  // Закрытая заявка: несохранённый черновик пропадает без отправки (D-07).
  $effect(() => {
    if (!canPost && (draft.text !== '' || draft.chips.length > 0)) {
      untrack(() => clearDraft());
    }
  });

  export function addFiles(files: globalThis.File[], folderNames: string[] = []) {
    if (!canPost) return;
    storeAddFiles(files, maxBytes, folderNames);
  }

  export function focus() {
    textareaEl?.focus();
  }

  function autosize() {
    const el = textareaEl;
    if (!el) return;
    el.style.height = 'auto';
    el.style.height = `${Math.min(el.scrollHeight, 192)}px`;
  }

  // Сброс высоты при очистке текста (после отправки / смены заявки).
  $effect(() => {
    void draft.text;
    autosize();
  });

  function pad(n: number): string {
    return String(n).padStart(2, '0');
  }

  function screenshotName(ext: string): string {
    const d = new Date();
    return `Скриншот ${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}-${pad(d.getMinutes())}-${pad(d.getSeconds())}.${ext}`;
  }

  function handlePaste(e: globalThis.ClipboardEvent) {
    const raw: globalThis.File[] = [];
    for (const it of Array.from(e.clipboardData?.items ?? [])) {
      if (it.kind !== 'file') continue;
      const f = it.getAsFile();
      if (f) raw.push(f);
    }
    if (raw.length === 0) return;
    e.preventDefault();
    const files = raw.map((f) => {
      if (f.type.startsWith('image/') && (f.name === '' || /^image\.[a-z0-9]+$/i.test(f.name))) {
        const ext = f.type.split('/')[1]?.replace('jpeg', 'jpg') || 'png';
        return new globalThis.File([f], screenshotName(ext), { type: f.type });
      }
      return f;
    });
    addFiles(files);
  }

  function handlePicked() {
    const input = fileInput;
    if (!input) return;
    const files = Array.from(input.files ?? []);
    input.value = '';
    if (files.length > 0) addFiles(files);
  }

  function handleKeydown(e: globalThis.KeyboardEvent) {
    if (e.key !== 'Enter' || e.isComposing) return;
    if (e.shiftKey) return;
    if (e.altKey) return;
    // Enter без модификаторов и Ctrl/Cmd+Enter отправляют.
    e.preventDefault();
    void send();
  }

  function isForbidden(e: unknown): boolean {
    const code = parseAppError(e).code;
    return code === 'FORBIDDEN' || code === 'Forbidden' || code === 'UNAUTHORIZED';
  }

  async function checkClosed() {
    try {
      const feed = await chat.feed(requestId);
      if (feed && feed.canPost === false) onClosed();
    } catch {
      // Недоступность ленты не меняет исхода отправки.
    }
  }

  function fail(line: string) {
    draft.errors = [...draft.errors, line];
  }

  async function send() {
    if (sending || !canPost || !loaded || !hasContent) return;
    sending = true;
    const sentFor = requestId;
    draft.errors = [];
    resetRejected();
    try {
      const draftIds: number[] = [];
      for (const chip of [...draft.chips]) {
        const res = await uploadDraft(sentFor, chip.file);
        if (draft.requestId !== sentFor) return;
        if (res.rejectCode !== null || res.attachment === null) {
          const code = res.rejectCode ?? 'network';
          draft.chips = draft.chips.map((c) => (c.id === chip.id ? { ...c, rejected: true } : c));
          fail(rejectLine(chip.file.name, code, chip.file.size, maxBytes));
          try {
            await chat.discardDrafts(sentFor);
          } catch {
            // Осиротевшие черновики подчищает сервер.
          }
          if (code === 'forbidden') await checkClosed();
          return;
        }
        draftIds.push(res.attachment.id);
      }
      let outcome;
      try {
        outcome = await chat.send(sentFor, draft.text, draftIds);
      } catch (e) {
        try {
          await chat.discardDrafts(sentFor);
        } catch {
          // см. выше
        }
        fail(
          isForbidden(e)
            ? 'Нет доступа к переписке этой заявки.'
            : 'Не удалось отправить сообщение. Проверьте подключение и попробуйте ещё раз.',
        );
        return;
      }
      if (draft.requestId !== sentFor) return;
      if (outcome.rejectCode !== null || outcome.message === null) {
        switch (outcome.rejectCode) {
          case 'too_long':
            fail('Сообщение длиннее 4000 символов. Сократите его и отправьте снова.');
            break;
          case 'closed':
            fail('Заявка закрыта — сообщение не отправлено.');
            onClosed();
            break;
          case 'no_chat':
            fail('Нет доступа к переписке этой заявки.');
            break;
          default:
            fail('Не удалось отправить сообщение. Проверьте подключение и попробуйте ещё раз.');
        }
        return;
      }
      const message = outcome.message;
      const keepErrors = draft.errors;
      clearDraft();
      draft.errors = keepErrors;
      onSent(message);
    } finally {
      sending = false;
    }
  }
</script>

{#if !canPost}
  <div class="readonly" role="status">
    <svg
      width="16"
      height="16"
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      stroke-width="1.5"
      stroke-linecap="round"
      stroke-linejoin="round"
      aria-hidden="true"
    >
      <rect x="3" y="7" width="10" height="7" rx="1.5" />
      <path d="M5.5 7V5a2.5 2.5 0 0 1 5 0v2" />
    </svg>
    <span>Заявка закрыта — переписка доступна только для чтения</span>
  </div>
  {#if draft.errors.length > 0}
    <div class="errors errors-closed" role="alert">
      <ul>
        {#each draft.errors as line, i (i)}<li>{line}</li>{/each}
      </ul>
    </div>
  {/if}
{:else}
  <div class="composer">
    {#if draft.chips.length > 0}
      <ul class="chips">
        {#each draft.chips as chip (chip.id)}
          <li class="chip" class:rejected={chip.rejected}>
            {#if chip.previewUrl && !failedPreviews[chip.id]}
              <img
                class="chip-thumb"
                src={chip.previewUrl}
                alt=""
                onerror={() => (failedPreviews[chip.id] = true)}
              />
            {:else}
              <span class="chip-icon">
                <AttachmentTypeIcon kind={iconKindForMime(chip.file.type, chip.file.name)} />
              </span>
            {/if}
            <span class="chip-name" title={chip.file.name}>{chip.file.name}</span>
            <span class="chip-size">{formatFileSize(chip.file.size)}</span>
            <Button
              variant="ghost"
              size="sm"
              iconOnly
              ariaLabel={`Убрать «${chip.file.name}»`}
              onclick={() => removeChip(chip.id)}
            >
              <svg
                width="14"
                height="14"
                viewBox="0 0 16 16"
                fill="none"
                stroke="currentColor"
                stroke-width="1.5"
                stroke-linecap="round"
                aria-hidden="true"
              >
                <path d="M4 4l8 8M12 4l-8 8" />
              </svg>
            </Button>
          </li>
        {/each}
      </ul>
    {/if}
    <div class="row">
      <Button
        variant="ghost"
        iconOnly
        ariaLabel="Прикрепить файлы"
        title="Прикрепить файлы"
        onclick={() => fileInput?.click()}
      >
        <svg
          width="18"
          height="18"
          viewBox="0 0 16 16"
          fill="none"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linecap="round"
          stroke-linejoin="round"
          aria-hidden="true"
        >
          <path
            d="M13 7.5l-4.6 4.6a3 3 0 0 1-4.2-4.2l5-5a2 2 0 0 1 2.8 2.8l-5 5a1 1 0 0 1-1.4-1.4L10 4.3"
          />
        </svg>
      </Button>
      <input
        bind:this={fileInput}
        class="file-input"
        type="file"
        multiple
        accept={ACCEPT}
        tabindex="-1"
        aria-hidden="true"
        onchange={handlePicked}
      />
      <textarea
        bind:this={textareaEl}
        class="field"
        rows="1"
        maxlength={MAX_LEN}
        placeholder="Напишите сообщение…"
        aria-label="Сообщение"
        bind:value={draft.text}
        oninput={autosize}
        onkeydown={handleKeydown}
        onpaste={handlePaste}
      ></textarea>
      <Button
        variant="primary"
        loading={sending}
        disabled={!hasContent || !loaded}
        onclick={() => void send()}
      >
        Отправить
      </Button>
    </div>
    <div class="foot">
      <span class="hint">Enter — отправить, Shift+Enter — новая строка</span>
      {#if showCounter}
        <span class="counter" class:over={draft.text.length >= MAX_LEN}>{counterText}</span>
      {/if}
    </div>
    {#if draft.errors.length > 0}
      <div class="errors" role="alert">
        <ul>
          {#each draft.errors as line, i (i)}<li>{line}</li>{/each}
        </ul>
        <Button variant="link" size="sm" onclick={() => (draft.errors = [])}>Скрыть</Button>
      </div>
    {/if}
  </div>
{/if}

<style lang="scss">
  .composer {
    flex-shrink: 0;
    padding: var(--tr-space-md);
    background: var(--tr-surface-raised);
    border-top: 1px solid var(--tr-border);
  }

  .readonly {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--tr-space-xs);
    padding: var(--tr-space-md);
    background: var(--tr-surface-sunken);
    border-top: 1px solid var(--tr-border);
    color: var(--tr-text-secondary);
    font-size: var(--tr-font-size-label);
    font-weight: var(--tr-font-weight-medium);
    white-space: normal;
    text-align: center;
  }

  .row {
    display: flex;
    align-items: flex-end;
    gap: var(--tr-space-xs);
  }

  .file-input {
    display: none;
  }

  .field {
    flex: 1;
    min-width: 0;
    min-height: 36px;
    max-height: 192px;
    box-sizing: border-box;
    padding: var(--tr-space-xs) var(--tr-space-sm);
    background: var(--tr-surface);
    color: var(--tr-text-primary);
    border: 1px solid var(--tr-border-strong);
    border-radius: var(--tr-radius-sm);
    font-family: var(--tr-font-family);
    font-size: var(--tr-font-size-body);
    line-height: var(--tr-line-height-body);
    resize: none;
    overflow-y: auto;

    &::placeholder {
      color: var(--tr-text-tertiary);
    }

    &:focus-visible {
      outline: none;
      border-color: var(--tr-accent);
      box-shadow: 0 0 0 3px var(--tr-focus-ring);
    }
  }

  .foot {
    display: flex;
    justify-content: space-between;
    gap: var(--tr-space-sm);
    margin-top: var(--tr-space-2xs);
    font-size: var(--tr-font-size-caption);
  }

  .hint {
    color: var(--tr-text-tertiary);
  }

  .counter {
    color: var(--tr-text-secondary);

    &.over {
      color: var(--tr-danger-text);
    }
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--tr-space-sm);
    margin: 0 0 var(--tr-space-sm);
    padding: 0;
    list-style: none;
  }

  .chip {
    display: flex;
    align-items: center;
    gap: var(--tr-space-xs);
    height: 48px;
    box-sizing: border-box;
    padding: 0 var(--tr-space-xs);
    background: var(--tr-surface);
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-md);
    white-space: nowrap;
    text-align: left;

    &.rejected {
      border-color: var(--tr-danger-text);
    }
  }

  .chip-thumb {
    width: 32px;
    height: 32px;
    object-fit: cover;
    border-radius: var(--tr-radius-xs);
  }

  .chip-icon {
    display: inline-flex;
    width: 32px;
    height: 32px;
    align-items: center;
    justify-content: center;
    color: var(--tr-text-secondary);
  }

  .chip-name {
    max-width: 200px;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--tr-text-primary);
    font-size: var(--tr-font-size-label);
    font-weight: var(--tr-font-weight-medium);
  }

  .chip-size {
    color: var(--tr-text-secondary);
    font-size: var(--tr-font-size-caption);
  }

  .errors {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--tr-space-sm);
    margin-top: var(--tr-space-xs);
    color: var(--tr-danger-text);
    font-size: var(--tr-font-size-label);
    font-weight: var(--tr-font-weight-medium);

    ul {
      margin: 0;
      padding: 0;
      list-style: none;
    }
  }

  .errors-closed {
    padding: var(--tr-space-xs) var(--tr-space-md);
    background: var(--tr-surface-sunken);
  }
</style>
