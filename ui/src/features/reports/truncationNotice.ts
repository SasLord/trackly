// Уведомление об усечении отчёта «Перемещения» (W-B03, фаза 41, план 41-33).
// Сервер отдаёт максимум 1000 строк и истинное число подходящих записей
// (`total`); если `total > shown`, пользователю нужно сказать об этом прямо.
//
// Это JS-зеркало Rust-функции `movements_truncation_notice`
// (crates/trackly-app/src/services/report_service.rs), которая пишет ту же
// фразу в печать и CSV. Текст закреплён общей golden-фикстурой
// `ui/scripts/fixtures/report-truncation/cases.json`: её читают и Rust-тест, и
// гейт `ui/scripts/check-report-truncation.mjs` (в `pnpm lint`).
//
// Модуль самодостаточен (ни одного import) — гейт транспилирует и исполняет его.

/**
 * `null`, если усечения нет (`total <= shown`); иначе текст уведомления.
 * `shown` — число реально полученных строк, `total` — поле `total` ответа
 * сервера (НЕ длина массива строк).
 */
export function movementsTruncationNotice(shown: number, total: number): string | null {
  if (total <= shown) return null;
  return `Показано записей: ${shown} из ${total} (самые ранние). Сузьте период или фильтры, чтобы увидеть остальные.`;
}
