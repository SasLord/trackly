---
status: diagnosed
phase: 16-documents-html-print
source: [16-VERIFICATION.md]
started: "2026-07-05T10:35:00Z"
updated: "2026-07-05T10:50:00Z"
---

## Current Test

[gap closure — desktop acts print]

## Tests

### 1. Печать акта в desktop-окне Tauri (Акт приёма-передачи + Печать документа приёма)
expected: В desktop-приложении по кнопке «Печать» открывается нативный диалог выбора принтера; preview показан в рамке страницы A4, не растянут во всю ширину.
result: issue — В desktop-webview диалог печати НЕ открывается (`iframe.contentWindow.print()` — no-op в WKWebView/WebView2). Preview растянут во всю ширину без рамки A4. В LAN-браузере с другого ноутбука диалог печати открывается корректно.

### 2. Печать акта в LAN-браузере
expected: По кнопке «Печать» открывается нативный диалог браузера.
result: passed — В браузере с другого ноутбука диалог печати открывается.

### 3. Рендер офлайн (логотип + кириллица) / правка шаблона на лету
expected: Логотип из `data:` URI, кириллица без «тофу»; правка `templates/act_handover.html` без перезапуска отражается сразу (D-08).
result: [pending] — не проверено в этом прогоне (заблокировано issue #1 на десктопе).

## Summary

total: 3
passed: 1
issues: 1
pending: 1
skipped: 0
blocked: 0

## Gaps

### GAP-16-01 — Desktop-печать акта не открывает диалог + растянутый preview (IN SCOPE)
- **Симптом:** В desktop-приложении (Акты приёма-передачи, Печать документа приёма) HTML показан растянутым во всю ширину; по «Печать» диалог выбора принтера не открывается. В LAN-браузере — работает.
- **Причина:** `PdfPreviewModal.svelte` вызывает `iframeEl.contentWindow.print()`, который в Tauri webview (WKWebView macOS / WebView2) на `srcdoc`-iframe не открывает системный диалог печати. Также preview-стили не задают рамку страницы A4 (растянут).
- **Нарушает:** D-09 (печать/сохранение в PDF через диалог браузера в ОБОИХ режимах — desktop + LAN).
- **Fix-направление:** десктоп-совместимый путь печати (напр. Tauri print API / печать main-окна вместо iframe / открытие в системном браузере) + preview-обёртка с шириной A4. Возможно `/gsd-debug` для подбора рабочего механизма печати в webview.
- **status:** failed

### Deferred (OUT of scope — вести отдельно, решение пользователя 2026-07-05)
- **Отчёты — миграция с krilla на HTML-печать (2a)** + **баг `reports_export_pdf` «Ошибка при создании PDF» (2b)**: Phase 16 не трогал `report_service.rs`/`tauri_cmds/reports.rs` (подтверждено git diff). Reports по-прежнему на krilla `render_docspec`; ошибка печати отчёта — предсуществующий баг, не регрессия Phase 16. Вынесено в отдельную работу (новая фаза/quick-task), НЕ входит в gap-closure Phase 16.
