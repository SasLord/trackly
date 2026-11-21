// quick 261121-hed (INT-09, NEW-3): единый словарь подписей источника перемещения для
// MovementTimeline. Каждый вариант Rust-enum `MovementSource`
// (crates/trackly-core/src/domain/place_movements.rs) обязан иметь здесь явную подпись,
// совпадающую с отчётом «Перемещения» (report_service.rs::movement_reason). Паритет
// держит гейт scripts/check-movement-source-labels.mjs (в `pnpm lint`).
//
// «причина не определена» — только для нераспознанного токена (значение, записанное
// более новой версией программы, Pitfall 6 / IN-01): никогда не исключение.
//
// Тип из bindings — только type-only импорт: bindings.ts в рантайме не импортируется.
import type { MovementEntryDto } from '../bindings';

export type MovementSourceToken = 'manual' | 'act' | 'map' | 'workstation' | 'group' | 'repair';

export const UNKNOWN_MOVEMENT_REASON = 'причина не определена';

function hasNote(entry: MovementEntryDto): boolean {
  return !!entry.note && entry.note.trim().length > 0;
}

/** «по ремонтной заявке №N» без примечания — общий префикс ссылки и текста. */
export function repairReasonBase(entry: MovementEntryDto): string {
  return entry.request_id !== null && entry.request_id !== undefined
    ? `по ремонтной заявке №${entry.request_id}`
    : 'по ремонтной заявке';
}

export const MOVEMENT_SOURCE_LABELS: Record<
  MovementSourceToken,
  (entry: MovementEntryDto) => string
> = {
  manual: (entry) => (hasNote(entry) ? `вручную · ${entry.note}` : 'вручную'),
  // Ссылка «актом №N» рисуется в шаблоне; здесь — текст для случая без номера/ссылки.
  act: (entry) => (entry.act_number ? `актом №${entry.act_number}` : 'актом'),
  map: () => 'по карте',
  workstation: () => 'назначение АРМ',
  // Plan 41-24 (D-28): строка самой группы — «перенос группы», строка состава —
  // «в составе группы «имя»» (ссылка на группу рисуется в шаблоне).
  group: (entry) => {
    if (entry.entity_type === 'group') return 'перенос группы';
    const label = entry.group_label?.trim() ?? '';
    return label.length > 0 ? `в составе группы «${label}»` : 'в составе группы';
  },
  // NEW-3: перенос по ремонтной заявке; примечание (каскад картриджей —
  // «вместе с принтером») через « · », как в отчёте.
  repair: (entry) => {
    const base = repairReasonBase(entry);
    return hasNote(entry) ? `${base} · ${entry.note}` : base;
  },
};

export function reasonText(entry: MovementEntryDto): string {
  if (Object.hasOwn(MOVEMENT_SOURCE_LABELS, entry.source)) {
    return MOVEMENT_SOURCE_LABELS[entry.source as MovementSourceToken](entry);
  }
  return UNKNOWN_MOVEMENT_REASON;
}
