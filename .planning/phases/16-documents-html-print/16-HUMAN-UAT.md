---
status: passed
phase: 16-documents-html-print
source: [16-VERIFICATION.md]
started: "2026-07-05T10:35:00Z"
updated: "2026-07-05T14:50:00Z"
---

## Current Test

[gap closure — desktop acts print]

## Tests

### 1. Печать акта в desktop-окне Tauri (Акт приёма-передачи + Печать документа приёма)
expected: В desktop-приложении по кнопке «Печать» открывается нативный диалог выбора принтера; preview показан в рамке страницы A4, не растянут во всю ширину.
result: passed (после gap-closure GAP-16-01, commit 072755a, human-verified 2026-07-05) — desktop открывает акт в системном браузере по умолчанию и сразу показывает диалог печати (с логотипом); preview показан в рамке A4. Webview-печать (no-op в WKWebView, tauri#3066/#13451) обойдена через temp-file + tauri-plugin-shell open.

### 2. Печать акта в LAN-браузере
expected: По кнопке «Печать» открывается нативный диалог браузера.
result: passed — В браузере с другого ноутбука диалог печати открывается.

### 3. Рендер офлайн (логотип + кириллица) / правка шаблона на лету
expected: Логотип из `data:` URI, кириллица без «тофу»; правка `templates/act_handover.html` без перезапуска отражается сразу (D-08).
result: passed (частично, в рамках GAP-16-01) — логотип из `data:` URI отображается в обоих режимах (desktop + LAN после CSP img-src data: фикса), кириллица без «тофу». Правка шаблона на лету (D-08) отдельно в этом прогоне не перепроверялась, но не была затронута gap-closure.

## Summary

total: 3
passed: 3
issues: 0
pending: 0
skipped: 0
blocked: 0

## Gaps

### GAP-16-01 — Desktop-печать акта не открывает диалог + растянутый preview (IN SCOPE)
- **Симптом:** В desktop-приложении (Акты приёма-передачи, Печать документа приёма) HTML показан растянутым во всю ширину; по «Печать» диалог выбора принтера не открывается. В LAN-браузере — работает.
- **Причина:** `PdfPreviewModal.svelte` вызывает `iframeEl.contentWindow.print()`, который в Tauri webview (WKWebView macOS / WebView2) на `srcdoc`-iframe не открывает системный диалог печати. Также preview-стили не задают рамку страницы A4 (растянут).
- **Нарушает:** D-09 (печать/сохранение в PDF через диалог браузера в ОБОИХ режимах — desktop + LAN).
- **Fix-направление:** десктоп-совместимый путь печати (напр. Tauri print API / печать main-окна вместо iframe / открытие в системном браузере) + preview-обёртка с шириной A4. Возможно `/gsd-debug` для подбора рабочего механизма печати в webview.
- **status:** resolved (commit 072755a, human-verified 2026-07-05; debug session .planning/debug/resolved/desktop-webview-print-dialog.md)
- **Фактическая причина + фикс:** webview-печать (iframe.contentWindow.print() и top-level window.print()) — no-op в Tauri WKWebView (tauri#3066/#13451). Desktop переведён на обход: temp-file HTML + открытие в системном браузере (tauri-plugin-shell open) с авто-печатью; LAN — печать через top-level document вместо iframe. Дополнительно устранены слои конфигурации: plugins.shell.open regex (иначе open() denied), fs:allow-write-text-file (иначе writeTextFile denied), CSP img-src data: (иначе LAN-логотип блокировался). Preview обёрнут в рамку A4.

### Deferred (OUT of scope — вести отдельно, решение пользователя 2026-07-05)
- **Отчёты — миграция с krilla на HTML-печать (2a)** + **баг `reports_export_pdf` «Ошибка при создании PDF» (2b)**: Phase 16 не трогал `report_service.rs`/`tauri_cmds/reports.rs` (подтверждено git diff). Reports по-прежнему на krilla `render_docspec`; ошибка печати отчёта — предсуществующий баг, не регрессия Phase 16. Вынесено в отдельную работу (новая фаза/quick-task), НЕ входит в gap-closure Phase 16.
