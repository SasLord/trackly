// Чистые функции порядка для перестановки строк (D-10, фаза 41). Без DOM и
// рун: жест перетаскивания компиляционные гейты не видят, а эти функции —
// проверяются golden-фикстурой `ui/scripts/fixtures/reorder/cases.json`
// гейтом `ui/scripts/check-reorder.mjs`.

/**
 * Индекс вставки по вертикальной позиции курсора: первый ряд, чья середина
 * ниже курсора (строгое «меньше»); если такого нет — `rects.length`.
 */
export function insertionIndex(rects: { top: number; height: number }[], pointerY: number): number {
  for (let i = 0; i < rects.length; i++) {
    if (pointerY < rects[i].top + rects[i].height / 2) return i;
  }
  return rects.length;
}

/**
 * Перенос элемента `from` в позицию вставки `to`. `to` — индекс вставки в
 * ИСХОДНОМ массиве: после удаления элемента индекс при `to > from` сдвигается
 * на единицу. Всегда возвращает новый массив, исходный не мутируется.
 */
export function reorder<T>(items: T[], from: number, to: number): T[] {
  const next = items.slice();
  // Индекс вне массива (в т.ч. пустой массив) — нечего переносить; без этого
  // splice вставил бы `undefined`.
  if (from < 0 || from >= next.length) return next;
  const [moved] = next.splice(from, 1);
  next.splice(to > from ? to - 1 : to, 0, moved);
  return next;
}
