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
 * живого предпросмотра «Настройки → Организация» (D-11). Зеркалит Rust
 * `shorten_place_path` (RESEARCH.md Pattern 3, D-13..D-16) — используется
 * ТОЛЬКО для локального предпросмотра, реальное сокращение приходит с
 * бэкенда (D-01).
 *
 * Разделитель варианта ставится, только когда что-то реально выброшено (D-13).
 * На пути из 2 сегментов `ends`/`last_two` выбрасывать нечего — путь
 * возвращается целиком через штатный ` / ` (D-14), — но `last` СУЖАЕТ его до
 * последнего сегмента, ровно как ветка `2 => match variant` в Rust.
 *
 * Контракт, который обязан держаться (нет JS-тестов — проверять глазами при
 * правке): для каждого `variant` результат этой функции побайтово равен
 * `trackly_core::domain::places::shorten_place_path` на том же входе.
 *   'Здание А / 1 этаж' + last     -> '1 этаж'
 *   'Здание А / 1 этаж' + ends     -> 'Здание А / 1 этаж'
 *   'Здание А / 1 этаж' + last_two -> 'Здание А / 1 этаж'
 * Оба образца предпросмотра трёхсегментные (так предписывает UI-SPEC), поэтому
 * двухсегментная ветка на экране не видна — расхождение здесь тихое.
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
  if (segments.length === 2) return variant === 'last' ? segments[1] : fullPath;
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
