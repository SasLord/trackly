---
status: investigating
trigger: "Печать отчёта «Перемещения» из LAN-браузера добавляет третий лист — дубликат первого; из десктопа та же печать работает верно."
created: 2026-09-03T00:00:00Z
updated: 2026-09-03T00:00:00Z
---

## Current Focus

hypothesis: (не сформирована — фаза 0/1, сбор улик)
test: чтение пути печати отчёта (PdfPreviewModal + @media print каскад)
expecting: найти элемент, который печатается дополнительно в браузере
next_action: прочитать ui/src/features/acts/PdfPreviewModal.svelte целиком, найти LAN-ветку handlePrint

## Symptoms

expected: Печать/экспорт PDF отчёта «Перемещения» из LAN-браузера даёт тот же результат, что и из десктопа — столько же страниц, сколько показал предпросмотр (2).
actual: В предпросмотре 2 страницы; при отправке на принтер / сохранении в PDF появляется третий лист — дубликат первого. Из десктопа печать корректна.
errors: нет
reproduction: Тест 18 фазы 40. `pnpm --dir ui build`, открыть приложение в LAN-браузере под менеджером, Отчёты → Перемещения → Печать/Экспорт PDF.
started: обнаружено при UAT фазы 40 (2026-09-03)

## Eliminated

## Evidence

- timestamp: phase-0
  checked: .planning/debug/knowledge-base.md
  found: Совпадение по ключевым словам (LAN print, PdfPreviewModal, @media print, act-print-root) с записью desktop-webview-print-dialog. Там зафиксировано: LAN-ветка печати внедряет style+body отрендеренного документа в скрытый #act-print-root в ТОП-УРОВНЕВОМ документе и печатает его, а не iframe.
  implication: Кандидат-гипотеза: лишняя страница приходит из top-level DOM приложения (или из самого предпросмотра), а не из содержимого отчёта.

## Resolution

root_cause:
fix:
verification:
files_changed: []
