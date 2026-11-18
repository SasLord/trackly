// Фаза 41.5, план 13: тексты отказов файла вынесены из AttachmentsBlock, чтобы
// блок «Вложения» и поле ввода чата говорили одними словами. Чистые функции.
import { formatFileSize } from '$lib/utils/formatFileSize';

export function limitMb(maxBytes: number): number {
  return Math.floor(maxBytes / (1024 * 1024));
}

export function rejectLine(name: string, code: string, size: number, maxBytes: number): string {
  switch (code) {
    case 'too_large':
      return `«${name}» — больше ${limitMb(maxBytes)} МБ (размер файла ${formatFileSize(size)}).`;
    case 'empty':
      return `«${name}» — файл пустой.`;
    case 'type_not_allowed':
      return `«${name}» — такой файл приложить нельзя. Подходят фото (JPEG, PNG, WebP, GIF, HEIC), PDF, документы Word, Excel и OpenDocument, TXT, CSV, LOG, архивы ZIP и 7z.`;
    case 'heic_failed':
      return `«${name}» — не удалось преобразовать фото HEIC. Сохраните его в JPEG и приложите снова.`;
    case 'forbidden':
      return `«${name}» — нет доступа к карточке или она удалена.`;
    default:
      return `«${name}» — не удалось загрузить. Проверьте подключение и попробуйте ещё раз.`;
  }
}
