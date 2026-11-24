---
quick_id: 260827-wsu
slug: csv-export-date-column-raw-unix-timestam
status: complete
completed: 2026-08-27
summary_written: 2026-11-24
requirements: [WSU-01, WSU-02]
commits:
  - 0130206c  # fix: handover_date_utc → читаемая локальная дата в CSV/HTML-экспорте
  - 9c9861c6  # fix: экранная ячейка даты в таблице отчёта приведена к формату экспорта
  - e52e16d4  # docs: PLAN + строка STATE
---

# Quick 260827-wsu: читаемая дата в CSV/PDF-экспорте отчётов — итог (восстановлен)

Исходный SUMMARY в 2026-08-27 не был записан: docs-коммит `e52e16d4` содержит только PLAN и
правку STATE.md. Итог восстановлен 2026-11-24 (quick 261124-umn) по git log и строке таблицы
«Quick Tasks Completed» в STATE.md, где задача отмечена `complete ✓`.

## Сделано (по коммитам)
- `0130206c` — `crates/trackly-app/src/services/report_service.rs`: колонка «Дата» в экспорте
  отчёта «Заявки» (CSV и HTML/печать) выводит читаемую локальную дату-время вместо сырого
  unix-timestamp; оба пути экспорта идут через один форматтер.
- `9c9861c6` — `ui/src/features/reports/ReportTable.svelte`: экранная таблица показывает ту же
  дату-время, что и экспорт.

## Подтверждение
- Коммиты присутствуют в `main`.
- STATE.md, таблица «Quick Tasks Completed»: строка `260827-wsu-…` со статусом `complete ✓`.
