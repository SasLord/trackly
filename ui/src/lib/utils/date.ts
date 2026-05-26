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
