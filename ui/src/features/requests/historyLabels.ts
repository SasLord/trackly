// Общий словарь подписей событий заявки: «История» (RequestDetail) и «Чат» используют
// одни и те же тексты (UI-SPEC §9.1). Сырые коды не показываются (REP-07).

/** Структурный минимум: подходит и записи истории, и элементу ленты чата. */
export interface HistoryLabelInput {
  action: string;
  notes?: string | null;
  actorName?: string | null;
  relatedRequestId?: number | null;
}

export function actionLabel(action: string): string {
  const labels: Record<string, string> = {
    create: 'Создана',
    accept: 'Принята в работу',
    complete: 'Выполнена',
    reject: 'Отклонена',
    cancel: 'Отменена',
    'custom:create': 'Создана',
    'custom:accept': 'Принята в работу',
    'custom:complete': 'Выполнена',
    'custom:reject': 'Отклонена',
    'custom:cancel': 'Отменена',
  };
  return labels[action] ?? action;
}

// D-35: id целевой заявки для записи «объединена с…»; иначе null.
export function mergedIntoId(e: HistoryLabelInput): number | null {
  if (e.action.replace(/^custom:/, '') !== 'repair_merged_into') return null;
  return e.relatedRequestId ?? null;
}

/**
 * Строка события = text + (ФИО с разделителем) + suffix. ФИО вынесено отдельно, потому что
 * «История» отделяет его « ; », а чат — « · ». suffix — хвост после ФИО (у части событий
 * примечание идёт после ФИО, у части — до него, поэтому порядок сохраняется дословно).
 */
export function entryParts(e: HistoryLabelInput): {
  text: string;
  who: string | null;
  suffix: string;
} {
  const code = e.action.replace(/^custom:/, '');
  const note = e.notes?.trim() ? e.notes.trim() : null;
  const who = e.actorName ? e.actorName : null;
  const q = (v: string | null) => (v ? `«${v}»` : '');
  switch (code) {
    case 'repair_accept':
      return {
        text: 'Принята в работу',
        who,
        suffix: note ? `; Устройство перемещено: ${q(note)}` : '',
      };
    case 'repair_move':
      return { text: `Устройство перемещено${note ? `: ${q(note)}` : ''}`, who, suffix: '' };
    case 'repair_send':
      return { text: `Устройство передано в сервис${note ? ` ${q(note)}` : ''}`, who, suffix: '' };
    case 'repair_receive':
      return { text: 'Устройство возвращено из сервиса', who, suffix: '' };
    case 'repair_cancel':
      return { text: 'Ремонт отменён', who, suffix: '' };
    case 'repair_join':
      return { text: 'Присоединился к заявке', who, suffix: note ? `; ${note}` : '' };
    case 'repair_merge':
      return {
        text: 'К заявке присоединена другая заявка',
        who,
        suffix: note ? `; ${note}` : '',
      };
    case 'repair_merged_into':
      return { text: 'Заявка объединена с другой', who, suffix: '' };
    case 'repair_assign_device':
      return { text: 'Указано устройство', who, suffix: '' };
    default:
      if (code.startsWith('repair_') && code !== 'repair_complete') {
        return { text: 'Изменение заявки', who, suffix: '' };
      }
      return { text: actionLabel(e.action), who, suffix: note ? `; ${note}` : '' };
  }
}

// REP-07: строка «Истории» без даты; ФИО через «; ».
export function entryText(e: HistoryLabelInput): string {
  const p = entryParts(e);
  return `${p.text}${p.who ? `; ${p.who}` : ''}${p.suffix}`;
}

// Строка события в чате: ФИО через « · ».
export function chatEventText(e: HistoryLabelInput): string {
  const p = entryParts(e);
  return `${p.text}${p.who ? ` · ${p.who}` : ''}${p.suffix}`;
}
