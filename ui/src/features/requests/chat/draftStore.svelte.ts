// .svelte.ts extension REQUIRED — Svelte 5 runes.
//
// Фаза 41.5, план 13 (UI-SPEC §10.6): черновик сообщения живёт в сторе, а не в
// ChatComposer: неактивная вкладка не монтируется, и состояние внутри компонента
// пропало бы при переключении «Чат» / «Информация». Ключ — id заявки; смена
// заявки сбрасывает черновик.
import { registerUserScopedReset } from '@trackly/kit/stores/userScoped.svelte';
import { rejectLine } from '../../attachments/rejectText';

export interface Chip {
  id: string;
  file: globalThis.File;
  previewUrl: string | null;
  rejected: boolean;
}

export const draft = $state({
  requestId: null as number | null,
  text: '',
  chips: [] as Chip[],
  errors: [] as string[],
});

let nextChip = 1;

function revokeAll(): void {
  for (const c of draft.chips) {
    if (c.previewUrl) URL.revokeObjectURL(c.previewUrl);
  }
}

/** Полный сброс: текст, чипы (с очисткой object URL) и ошибки. */
export function clearDraft(): void {
  // Идемпотентно: на пустом черновике ничего не пишем в $state (запись нового
  // пустого массива будила подписчиков и зацикливала эффекты).
  if (draft.chips.length > 0) {
    revokeAll();
    draft.chips = [];
  }
  if (draft.text !== '') draft.text = '';
  if (draft.errors.length > 0) draft.errors = [];
}

/** Привязать черновик к заявке; смена заявки отбрасывает прежний. */
export function useDraftFor(requestId: number): void {
  if (draft.requestId === requestId) return;
  clearDraft();
  draft.requestId = requestId;
}

registerUserScopedReset(() => {
  clearDraft();
  draft.requestId = null;
});

/** Добавить файлы в чипы; пустые и больше лимита отсекаются строкой ошибки. */
export function addFiles(files: globalThis.File[], maxBytes: number, folderNames: string[] = []) {
  const lines: string[] = folderNames.map((n) => `«${n}» — это папка. Приложите файлы из неё.`);
  const added: Chip[] = [];
  for (const file of files) {
    if (file.size === 0) {
      lines.push(rejectLine(file.name, 'empty', 0, maxBytes));
      continue;
    }
    if (maxBytes > 0 && file.size > maxBytes) {
      lines.push(rejectLine(file.name, 'too_large', file.size, maxBytes));
      continue;
    }
    const isPhoto = file.type.toLowerCase().startsWith('image/');
    added.push({
      id: `c${nextChip++}`,
      file,
      previewUrl: isPhoto ? URL.createObjectURL(file) : null,
      rejected: false,
    });
  }
  if (added.length > 0) draft.chips = [...draft.chips, ...added];
  if (lines.length > 0) draft.errors = [...draft.errors, ...lines];
}

export function removeChip(id: string): void {
  const chip = draft.chips.find((c) => c.id === id);
  if (chip?.previewUrl) URL.revokeObjectURL(chip.previewUrl);
  draft.chips = draft.chips.filter((c) => c.id !== id);
}

/** Сброс пометок «отклонён» (перед новой попыткой отправки). */
export function resetRejected(): void {
  draft.chips = draft.chips.map((c) => (c.rejected ? { ...c, rejected: false } : c));
}
