/**
 * Формула сокращения пути места (Phase 39.1, PLC-07/PLC-08) — вычисляется на
 * бэкенде (`shorten_place_path`, D-01) и приходит уже готовой в
 * `place_path_short` на списочных/отчётных DTO. Этот модуль больше не
 * содержит рантайм-логику сокращения для экрана — только локальное JS-зеркало
 * формулы для мгновенного офлайн-предпросмотра в «Настройки → Организация»
 * (`previewShortenPath`, план 08) и вспомогательный `monoReadout` для readout
 * значения разделителя.
 */

/**
 * Плановый вариант сокращения пути (39.1-08) — параметризованная версия для
 * живого предпросмотра «Настройки → Организация» (D-11). Разделитель варианта
 * ставится, только когда что-то реально выброшено (D-13): путь из ≤2
 * сегментов всегда возвращается целиком через штатный ` / ` независимо от
 * `variant` (D-14). Зеркалит Rust `shorten_place_path` (RESEARCH.md Pattern 3,
 * D-13..D-16) — используется ТОЛЬКО для локального предпросмотра, реальное
 * сокращение приходит с бэкенда (D-01).
 */
export function previewShortenPath(
  fullPath: string,
  variant: 'ends' | 'last_two' | 'last',
  sepEnds: string,
  sepLastTwo: string,
): string {
  if (!fullPath) return fullPath;
  const segments = fullPath.split(' / ');
  if (segments.length <= 1) return fullPath;
  if (segments.length === 2) return segments.join(' / ');
  if (variant === 'last') return segments[segments.length - 1];
  if (variant === 'last_two') return segments.slice(-2).join(sepLastTwo);
  return `${segments[0]}${sepEnds}${segments[segments.length - 1]}`;
}

/**
 * Читаемое представление значения разделителя для `.field-hint` под полем
 * (UI-SPEC.md «Проблема 1») — заменяет ТОЛЬКО пробел (U+0020) на «·»
 * (U+00B7), исключительно для этого readout, не для самого значения поля.
 */
export function monoReadout(v: string): string {
  return v.replace(/ /g, '·');
}
