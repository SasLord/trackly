// Чистая политика возврата текста в поле ввода (фаза 43-16; D-12, WR-03).
// Без рун и сторов: проверяется поведенческим гейтом check-chat-pending-behavior.

/** Склеивает возвращаемый текст с тем, что набрано за время отправки (через перевод строки). */
export function restoreText(sent: string, current: string): string {
  if (sent === '') return current;
  if (current === '') return sent;
  return `${sent}\n${current}`;
}
