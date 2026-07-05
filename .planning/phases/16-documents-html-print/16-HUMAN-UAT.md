---
status: partial
phase: 16-documents-html-print
source: [16-VERIFICATION.md]
started: "2026-07-05T10:35:00Z"
updated: "2026-07-05T10:35:00Z"
---

## Current Test

[awaiting human testing]

## Tests

### 1. Печать многоустройственного акта в браузерном print-preview
expected: Печать акта с 2+ устройствами и длинным текстом «Комплектация»/«Технические характеристики» в Chrome/Edge (Windows-target) с отключёнными колонтитулами браузера — все строки устройств печатаются полностью, без наложения/обрезки текста на разрывах страниц A4, без вставленного браузером URL/даты/номера страницы; вёрстка визуально соответствует образцу Word из Phase 15.
result: [pending]

### 2. Рендер в desktop-webview и LAN-браузере офлайн (логотип + кириллица)
expected: Открыть print-preview в desktop-окне Tauri и в LAN-браузере (другая машина/вкладка) без доступа в интернет — логотип отображается из встроенного `data:` URI, кириллица рендерится корректно системными шрифтами в обоих webview, без пропавших глифов/«тофу»-квадратов.
result: [pending]

### 3. Правка HTML-шаблона на лету без перезапуска (D-08)
expected: Отредактировать `templates/act_handover.html` вручную (например, в Notepad) при запущенном приложении, сохранить, затем сгенерировать акт заново без перезапуска — новая генерация сразу отражает правку (read-on-render, D-08).
result: [pending]

## Summary

total: 3
passed: 0
issues: 0
pending: 3
skipped: 0
blocked: 0

## Gaps
