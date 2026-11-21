// Фаза 41.8, план 22 (D-26, D-19): нужен ли выбор «оставить / вывести» при
// сохранении формы устройства. Чистая функция, зеркалит серверное правило
// DeviceService::update (план 21); сервер остаётся границей безопасности.
//
// Модуль БЕЗ импортов: golden-гейт check-write-off-choice.mjs транспилирует его
// изолированно. Код статуса объявлен единственным локальным литералом; гейт
// сверяет его с STATUS_WRITTEN_OFF из deviceRowMenu.ts.
const WRITTEN_OFF_CODE = 'списано';

export interface WriteOffChoiceInput {
  statusCodeBefore: string | null;
  statusCodeAfter: string | null;
  groupId: number | null;
  anchorGroupId: number | null;
}

export type WriteOffChoiceVerdict = 'none' | 'choose' | 'anchor_refuse';

export function writeOffChoiceNeeded(input: WriteOffChoiceInput): WriteOffChoiceVerdict {
  if (input.statusCodeAfter !== WRITTEN_OFF_CODE) return 'none';
  if (input.statusCodeBefore === WRITTEN_OFF_CODE) return 'none';
  if (input.groupId === null) return 'none';
  if (input.anchorGroupId !== null) return 'anchor_refuse';
  return 'choose';
}
