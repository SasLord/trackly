/**
 * Размер файла для интерфейса (UI-SPEC §6.7): основание 1024, запятая как
 * десятичный разделитель, неразрывный пробел перед единицей.
 * `512 Б` · `48 КБ` (целые) · `1,2 МБ` (один знак, `,0` отбрасывается) · `1,4 ГБ`.
 */
const NBSP = ' ';

function oneDecimal(value: number): string {
  const rounded = Math.round(value * 10) / 10;
  const text = Number.isInteger(rounded) ? String(rounded) : rounded.toFixed(1);
  return text.replace('.', ',');
}

export function formatFileSize(bytes: number): string {
  const safe = Number.isFinite(bytes) && bytes > 0 ? bytes : 0;
  if (safe < 1024) {
    return `${Math.round(safe)}${NBSP}Б`;
  }
  const kb = safe / 1024;
  if (kb < 1024) {
    return `${Math.round(kb)}${NBSP}КБ`;
  }
  const mb = kb / 1024;
  if (mb < 1024) {
    return `${oneDecimal(mb)}${NBSP}МБ`;
  }
  return `${oneDecimal(mb / 1024)}${NBSP}ГБ`;
}
