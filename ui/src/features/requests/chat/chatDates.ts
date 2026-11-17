// Время и подписи дней ленты чата (UI-SPEC §4, §8.3). Всё в ЛОКАЛЬНОМ времени:
// getHours/getDate, а не getUTC*. Чистый TS без импортов — Node импортирует напрямую
// (check-chat-dates.mjs).

const MONTHS = [
  'января',
  'февраля',
  'марта',
  'апреля',
  'мая',
  'июня',
  'июля',
  'августа',
  'сентября',
  'октября',
  'ноября',
  'декабря',
];

const pad = (n: number): string => String(n).padStart(2, '0');

/** `ЧЧ:мм` в локальном времени. */
export function timeHHmm(seconds: number): string {
  const d = new Date(seconds * 1000);
  return `${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

/** `гггг-мм-дд` по локальной дате — ключ группировки по дням. */
export function dayKey(seconds: number): string {
  const d = new Date(seconds * 1000);
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

/** `Сегодня` / `Вчера` / `16 ноября` / `16 ноября 2025`. */
export function dayLabel(seconds: number, nowSeconds: number): string {
  const d = new Date(seconds * 1000);
  const now = new Date(nowSeconds * 1000);
  const today = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  const that = new Date(d.getFullYear(), d.getMonth(), d.getDate());
  const diffDays = Math.round((today.getTime() - that.getTime()) / 86_400_000);
  if (diffDays === 0) return 'Сегодня';
  if (diffDays === 1) return 'Вчера';
  const base = `${d.getDate()} ${MONTHS[d.getMonth()]}`;
  return d.getFullYear() === now.getFullYear() ? base : `${base} ${d.getFullYear()}`;
}

/** Подряд идущие сообщения одного автора с разницей ≤ 5 минут — одна группа. */
export function sameGroup(prevSeconds: number, nextSeconds: number): boolean {
  return nextSeconds - prevSeconds <= 300;
}
