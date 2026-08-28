/**
 * Единственная реализация сокращения пути места (quick 260827-ui3) — заменяет
 * два независимых дубля, которые раньше жили в `ReportTable.svelte` и
 * `PlaceContents.svelte`. Вариант приходит с бэкенда через `AuthStatusDto.
 * place_path_display` (boot-time, `App.svelte::loadAuthStatus`) и хранится в
 * `authStore.placePathDisplay`.
 */
export type PlacePathDisplay = 'ends' | 'last_two' | 'full';

/**
 * Защитный whitelist-фолбэк на нераспознанное/отсутствующее значение с
 * бэкенда — никогда не бросает и не оставляет пустую ячейку, всегда
 * возвращает валидный вариант (дефолт `'ends'`, зеркалит бэкенд-дефолт
 * `PlacePathDisplay::Ends`).
 */
export function normalizePlacePathDisplay(raw: string | null | undefined): PlacePathDisplay {
  return raw === 'ends' || raw === 'last_two' || raw === 'full' ? raw : 'ends';
}

/**
 * Сократить путь места согласно выбранному варианту. Путь из 1-2 сегментов
 * НИКОГДА не сокращается, независимо от варианта — «Здание А / 2 этаж» не
 * превращается в «Здание А // 2 этаж» (нечего убирать, это была бы ложь про
 * элизию).
 */
export function shortenPlacePath(fullPath: string, variant: PlacePathDisplay): string {
  if (!fullPath) return fullPath;
  const segments = fullPath.split(' / ');
  if (variant === 'full' || segments.length <= 2) return fullPath;
  if (variant === 'last_two') return segments.slice(-2).join(' / ');
  return `${segments[0]} // ${segments[segments.length - 1]}`;
}

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
