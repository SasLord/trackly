/**
 * Форматирует Unix-timestamp (секунды) в читаемую строку для отображения.
 * Backend хранит UTC; отображение — в локали пользователя.
 */
export function formatUnixSeconds(seconds: number): string {
  return new Date(seconds * 1000).toLocaleString('ru-RU', {
    year: 'numeric',
    month: 'long',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}

/** `07.11.2026 14:32` — короткий формат даты и времени (локальное время). */
export function formatDateTimeShort(seconds: number): string {
  const d = new Date(seconds * 1000);
  const p = (n: number) => String(n).padStart(2, '0');
  return `${p(d.getDate())}.${p(d.getMonth() + 1)}.${d.getFullYear()} ${p(d.getHours())}:${p(d.getMinutes())}`;
}
