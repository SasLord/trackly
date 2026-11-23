// Чистая карта тонов статуса устройства (41.6, D-08). Без runtime-импортов:
// employee-код читает тон отсюда и не втягивает стор со справочником статусов
// (`device_statuses_list` сотруднику закрыт, 41.4 D-38).
//
// Единственная в приложении таблица тонов; ключ — code из справочника
// (UI-SPEC §10.2, пересмотрено 2026-10-17: R-06 отменён). Все шесть статусов —
// soft-чипы. Тон = насколько плохо, muted = ещё только в очереди/помечено:
// пара «действующий / ожидающий» делит variant и различается яркостью.

export type StatusVariant = 'default' | 'accent' | 'success' | 'warning' | 'destructive';
export type StatusAppearance = 'soft' | 'solid' | 'count';
export interface StatusTone {
  variant: StatusVariant;
  appearance: StatusAppearance;
  /** Приглушённый чип: статус «ещё в очереди/помечен», тон тот же, ярче только действующий. */
  muted?: boolean;
}

const TONES: Record<string, StatusTone> = {
  на_складе: { variant: 'default', appearance: 'soft' },
  в_работе: { variant: 'accent', appearance: 'soft' },
  на_ремонте: { variant: 'warning', appearance: 'soft', muted: true },
  в_ремонте: { variant: 'warning', appearance: 'soft' },
  на_списание: { variant: 'destructive', appearance: 'soft', muted: true },
  списано: { variant: 'destructive', appearance: 'soft' },
};
const FALLBACK_TONE: StatusTone = { variant: 'default', appearance: 'soft' };

/** Тон бейджа по коду статуса; неизвестный или пустой код — нейтральный. */
export function statusTone(code: string | null | undefined): StatusTone {
  return (code ? TONES[code] : undefined) ?? FALLBACK_TONE;
}
