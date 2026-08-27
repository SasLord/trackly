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
