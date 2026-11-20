// Единая формула заливки уровня тонера (41.6, D-18): TonerGauge (карточка) и
// TonerMini (индикатор строки) берут цвет отсюда. Без импортов.
// ≥25% accent, 10-24% warning, <10% danger; неизвестно (null) — пустой трек.
export function tonerFillColor(pct: number | null): string {
  if (pct === null) return 'var(--tr-surface-sunken)';
  if (pct >= 25) return 'var(--tr-accent)';
  if (pct >= 10) return 'var(--tr-warning)';
  return 'var(--tr-danger)';
}
