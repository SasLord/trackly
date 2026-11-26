/**
 * Согласование числительных с существительным по стандартному правилу
 * русского языка: 1 → forms[0], 2-4 → forms[1], 5+ (и 11-14 — исключение)
 * → forms[2]. Используется, например, для счётчика страниц предпросмотра
 * печати (Phase 33, D-10): `pluralizeRu(3, ['страница', 'страницы', 'страниц'])`.
 */
export function pluralizeRu(n: number, forms: [string, string, string]): string {
  const mod10 = n % 10;
  const mod100 = n % 100;

  if (mod10 === 1 && mod100 !== 11) return forms[0];
  if (mod10 >= 2 && mod10 <= 4 && (mod100 < 10 || mod100 >= 20)) return forms[1];
  return forms[2];
}
