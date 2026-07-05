---
status: resolved
slug: desktop-webview-print-dialog
trigger: "GAP-16-01 — В desktop-приложении печать акта (Акт приёма-передачи, Печать документа приёма) не открывает нативный диалог печати, и HTML показан растянутым во всю ширину без рамки A4. В LAN-браузере с другого ноутбука диалог печати открывается корректно."
created: "2026-07-05T11:00:00Z"
updated: "2026-07-05T14:50:00Z"
phase: 16-documents-html-print
related_gap: GAP-16-01
---

# Debug: desktop-webview-print-dialog

## Symptoms

- **Expected behavior:** В desktop-приложении (Tauri webview) по кнопке «Печать» в модалке предпросмотра акта открывается нативный диалог выбора принтера (как в браузере). Preview показан в рамке страницы формата A4, а не растянут во всю ширину.
- **Actual behavior:** В desktop-webview диалог печати НЕ открывается — по «Печать» ничего не происходит. Preview-HTML растянут во всю ширину окна без визуальной рамки страницы A4. В LAN-браузере (Chrome/Edge на другом ноутбуке) диалог печати открывается корректно — там всё работает.
- **Error messages:** Нет явной ошибки/тоста; кнопка просто не открывает диалог (тихий no-op). Тост «Не удалось вызвать диалог печати» НЕ появляется (значит `iframeEl.contentWindow.print()` не бросает исключение — просто не открывает диалог).
- **Timeline:** Появилось в Phase 16 — новый HTML-print пайплайн заменил прежний PDF-blob-в-iframe предпросмотр. Прежний путь (krilla PDF blob в iframe, WebView2/WKWebView рендерит PDF нативно) в этом сценарии не тестировался на печать через `contentWindow.print()`.
- **Reproduction:** Desktop-приложение → Акты → открыть акт → «Печать документа» / «Печать документа приёма» → модалка PdfPreviewModal с `<iframe srcdoc={html}>` → нажать «Печать» (footer). Диалог не открывается. Тот же поток в LAN-браузере с другой машины — диалог печати открывается.

## Suspected Area (orchestrator pre-analysis, not confirmed)

- `ui/src/features/acts/PdfPreviewModal.svelte:103-113` — `handlePrint()` вызывает `iframeEl.contentWindow.focus(); iframeEl.contentWindow.print();` на `srcdoc`-iframe. Гипотеза: в Tauri webview (macOS WKWebView / Windows WebView2) вызов `print()` на дочернем окне `srcdoc`-iframe не открывает системный диалог печати (в отличие от полноценного браузера).
- Растянутый preview — отдельная (стилевая) часть: `.pdf-iframe { width: 100% }` без обёртки шириной A4; `@page` в `srcdoc` влияет только на печать, не на экранный вид.
- **Важно (dev-констрейнт):** Windows-webview (WebView2) недоступен на dev-macOS; поведение печати в WebView2 нужно верифицировать на Windows-машине.

## Current Focus

hypothesis: RESOLVED — пользователь подтвердил (2026-07-05): desktop открывает акт в системном браузере и сразу показывает диалог печати с логотипом; LAN печатает с логотипом. Все слои закрыты за 4 цикла: (1) обход webview-печати через системный браузер; (2) plugins.shell.open regex; (3) fs:allow-write-text-file; (4) CSP img-src data: (LAN-логотип) + авто-печать (desktop). Временная диагностика убрана, возвращён аккуратный тост.
test: `pnpm --dir ui build` OK; `svelte-check` 0 errors; eslint/prettier чисто; `cargo test -p trackly-app --test security_headers` (env AD/SNMP mock, реальный ui/dist) — 4 passed. Human-verified.
expecting: —
next_action: DONE — закоммитить GAP-16-01 атомарно, переместить сессию в resolved/, обновить 16-HUMAN-UAT.md, дописать knowledge-base.
reasoning_checkpoint:
  hypothesis: "3-я верификация подтвердила рабочий основной механизм (desktop → системный браузер, LAN печатает). Два остаточных дефекта: (1) LAN логотип блокируется потому что CSP-заголовок не разрешает data:-картинки (нет img-src → default-src 'self' без data:); (2) desktop не запускает печать автоматически. Оба независимы от основного механизма и друг от друга."
  confirming_evidence:
    - "CSP-заголовок в http/mod.rs НЕ содержал img-src → data:image логотип (act_handover.html) падал на default-src 'self', который data: не включает. Баг LAN-only: desktop открывает file:// в системном браузере ВНЕ нашего CSP → логотип там всегда был (это и подтверждает, что причина — именно CSP, а не сам data:-URI/шаблон)."
    - "cargo test security_headers (env AD/SNMP mock + реальный ui/dist) — 4 passed после добавления img-src 'self' data' и регресс-ассерта; предыдущая версия ассерта проверяла только frame-src+blob, новый проверяет img-src+data:."
    - "Авто-печать инжектится ТОЛЬКО в desktop-ветке printViaSystemBrowser() перед записью temp-файла (LAN-ветка printViaTopLevel не тронута) — на load + setTimeout 300мс, чтобы data:-логотип успел отрисоваться в Safari до диалога печати."
  falsification_test: "После перезапуска: (1) если в LAN-браузере логотип всё ещё не виден — img-src фикс неверен/недостаточен (напр. остался другой CSP-слой или srcdoc-iframe имеет собственный CSP); проверить DevTools console на CSP-violation. (2) Если в desktop Safari открывает акт, но диалог печати не появляется сам — инжект скрипта не сработал (напр. file:// блокирует inline-скрипт, или load-событие не стрельнуло); тогда fallback — увеличить таймаут или использовать onload-атрибут в body."
  fix_rationale: "(1) img-src 'self' data: — минимальное точечное расширение CSP; data:-картинки не исполняют код, XSS-поверхность не растёт (ортогонально CR-01). (2) авто-печать через load-событие адресует именно жалобу (печать не появлялась сразу), инжект изолирован в desktop-ветке, не затрагивает LAN и preview. Оба — прямые фиксы корня, не симптома."
  blind_spots: "Не верифицировано на живом окне (нужен перезапуск + GUI). Риск (1): preview-iframe использует srcdoc — он наследует CSP родителя, так что img-src должен покрыть и preview, но это не проверено вживую. Риск (2): поведение inline-скрипта в file://-документе в Safari (некоторые браузеры ограничивают авто-print из file://); 300мс может не хватить на медленной машине для больших логотипов — но load-событие уже гарантирует загрузку картинки, setTimeout лишь страховка. Windows/WebView2 не проверялся (dev-констрейнт). Diagnostics-тост в catch ещё не убран (по указанию — после финального подтверждения)."
tdd_checkpoint:

## Evidence

- timestamp: "2026-07-05T11:15:00Z"
  checked: web search "WKWebView iframe contentWindow.print() srcdoc does not open print dialog Tauri" + tauri-apps/tauri issue #13451
  found: Documented Tauri bug (macOS/WKWebView, filed 2025-05-16) — apps using print-js (copies content into an iframe then calls iframe.contentWindow.print()) find that printing never starts. Explicitly noted: "printing using window.print() works fine, but iframe.contentWindow.print() does not — it is not doing anything." No exception thrown, matches our silent no-op symptom exactly (no error toast).
  implication: Iframe/child-window print bridge не проброшен в нативную панель печати. Наша PdfPreviewModal использует ровно этот паттерн (srcdoc-iframe → contentWindow.print()).

- timestamp: "2026-07-05T11:16:00Z"
  checked: crates/trackly-app/tauri.conf.json, Cargo.toml, capabilities/main.json — доступные плагины и права
  found: `tauri-plugin-shell`, `tauri-plugin-dialog`, `tauri-plugin-fs` подключены в Rust (main.rs:222-225). НО в capabilities/main.json гранчены только core:default, dialog:default, fs:default (+read/write .pdf scope). `shell:allow-open` НЕ гранчен. Единственное окно — main (1280x800). Плагина печати нет.
  implication: Нет встроенного Tauri print API. Для shell-open пути нужно ДОБАВИТЬ `shell:allow-open` в capabilities. Побочно обнаружено: ReportsPage.svelte уже использует `openPath()` (shell open) для desktop-печати отчётов — но без `shell:allow-open` этот путь сейчас, вероятно, падает в catch (отдельный дефект вне этого гэпа, стоит отметить в плане).

- timestamp: "2026-07-05T11:18:00Z"
  checked: crates/trackly-app/templates/act_handover.html (серверный HTML-шаблон)
  found: Self-contained HTML5 с inline <style> (вкл. @page A4). Нет скрипта авто-печати. @page действует только при печати — на экране iframe.pdf-iframe { width:100% } растягивает документ во всю ширину без A4-рамки. Подтверждает вторичную часть гэпа (растянутый preview).
  implication: Screen-preview требует обёртки фиксированной A4-ширины (напр. центрированный контейнер ~210mm с тенью), независимо от механизма печати.

- timestamp: "2026-07-05T11:25:00Z"
  checked: web search (authoritative) — window.print() в WKWebView vs WebView2 vs Tauri #3066
  found: (1) WKWebView (macOS): window.print() — no-op/фатальная ошибка без host-интеграции; Tauri #3066 — `-[WryWebView printOperationWithPrintInfo:]: unrecognized selector` на macOS. (2) WebView2 (Windows): window.print() РАБОТАЕТ, открывает Chromium print-диалог по умолчанию. (3) На macOS даже top-level window.print() через Tauri не работает.
  implication: КРИТИЧНО — на macOS desktop переключение на top-level window.print() НЕ решит проблему (Tauri #3066). Значит "вариант А" (печать div в главном окне) не универсален. Для desktop нужен путь, полностью обходящий webview-печать: открыть документ в системном приложении по умолчанию (браузер) или сгенерировать PDF и открыть в системном вьювере. На Windows/WebView2 print() работает, но чтобы фикс был единым и предсказуемым в обоих desktop-ОС, лучше не полагаться на webview-печать в desktop вообще.

- timestamp: "2026-07-05T12:05:00Z"
  checked: HUMAN-VERIFY провалился — на «Печать» в desktop появился тост ошибки (значит printViaSystemBrowser() бросил исключение). Раскрыл реальную ошибку (временный console.error + `${e.message}` в тосте) И прочитал исходник tauri-plugin-shell-2.3.5/src/scope.rs (fn open, строки 207-227).
  found: РЕШАЮЩЕЕ. `shell.open()` управляется НЕ через capability-scope `shell:allow-open` (это allowlist для scoped-команд `execute`/`Command`, не для `open`), а через `tauri.conf.json > plugins > shell > open` (bool | regex). В scope.rs: если `self.open` = None (наш случай — в tauri.conf.json `"plugins": {}`), open() логирует warning и использует ЗАВЕДОМО невозможный regex `"tauri^"` → ЛЮБОЙ вызов open() из JS падает с ScopeError. Если задан regex — путь валидируется по нему; дефолтный из доков `^((mailto:\w+)|(tel:\w+)|(https?://\w+)).+` не пропускает локальные file-пути. Далее open() зовёт `::open::with_detached(path, ...)` (на macOS = `/usr/bin/open`), который САМ принимает и file-path, и URL — т.е. ограничение чисто на уровне regex-валидации плагина, не ОС.
  implication: Настоящая причина провала — `plugins.shell.open` не включён в tauri.conf.json → open() из webview всегда denied. Это же объясняет, почему ReportsPage.svelte openPath() тоже не работал (тот же корень, отдельный уже-существовавший дефект). ФИКС: включить `plugins.shell.open` в tauri.conf.json с regex, разрешающим наш temp-html file-путь (macOS: `/var/folders/.../T/…`, Windows: `C:\\Users\\…\\Temp\\…`). Capability `shell:allow-open` для open() НЕ требуется и не помогает — убрать его из main.json (оставить fs-scope для writeTextFile). ВАЖНО: tauri.conf.json встраивается при сборке → требуется перезапуск cargo tauri dev.

- timestamp: "2026-07-05T12:20:00Z"
  checked: 2-я HUMAN-VERIFY (после перезапуска, shell.open уже исправлен) — новый тост diagnostics: «Печать: fs.write_text_file not allowed. Permissions associated with this command: … fs:allow-write-text-file, fs:allow-temp-write, … fs:write-all …». Проверил permissions fs-плагина: tauri-plugin-fs-2.5.1/permissions/autogenerated/commands/write_text_file.toml.
  found: РЕШАЮЩЕЕ (следующий слой). JS `writeTextFile()` вызывает команду плагина `write_text_file`, которую разрешает permission `fs:allow-write-text-file` (commands.allow = ["write_text_file"]). А я выдал `fs:allow-write-file` — это permission для ДРУГОЙ команды (бинарный `writeFile` → `write_file`). Поэтому write_text_file был denied, хотя write_file разрешён. Progressive bug: shell.open починили → всплыл следующий блокер на writeTextFile. `fs:allow-write-text-file` поддерживает object-form с inline `allow: [{path}]` (тот же механизм scoped-permission, что уже работал для fs:allow-write-file → PDF в ReportsPage).
  implication: ФИКС — добавить отдельный `fs:allow-write-text-file` с inline-scope `[$TEMP/*.html, $TEMP/**/*.html]`. `fs:allow-write-file` НЕЛЬЗЯ переиспользовать под html (он про write_file/бинарный) И его нельзя удалять (нужен ReportsPage для writeFile PDF-байтов). Поэтому не замена, а ДОБАВЛЕНИЕ отдельного permission под write_text_file. Требуется перезапуск cargo tauri dev (capabilities встраиваются при сборке).

- timestamp: "2026-07-05T14:35:00Z"
  checked: 3-я HUMAN-VERIFY — ОСНОВНОЙ МЕХАНИЗМ РАБОТАЕТ: desktop открывает акт в системном браузере, LAN печатает. Остались два уточнения в рамках GAP-16-01. Зацепки: (1) http/mod.rs CSP-заголовок ~строка 148; (2) act_handover.html логотип = <img src="data:...">; (3) printViaSystemBrowser() открывает file:// в Safari.
  found: ДВА остаточных дефекта. (1) LAN: логотип НЕ виден при печати/preview. CSP-заголовок (http/mod.rs) не имел директивы `img-src` → data:-картинка логотипа падала на `default-src 'self'`, который НЕ включает `data:` → браузер блокировал <img src="data:image/...">. Только LAN (браузер под нашим CSP); desktop открывает file:// в системном браузере ВНЕ этого CSP → логотип там был всегда (подтверждает диагноз — баг LAN-only). (2) Desktop: документ открывался в Safari, но диалог печати не появлялся автоматически — пользователь хотел авто-вызов.
  implication: ФИКС (1): добавить `img-src 'self' data:` в CSP (http/mod.rs). data:-картинки не исполняют скрипты → XSS-риск не растёт, ортогонально CR-01. Обновить регресс-ассерт в security_headers.rs (тест использует `contains`, не точную строку, но добавлен явный ассерт на img-src+data:). ФИКС (2): в desktop-ветке printViaSystemBrowser() инжектировать перед </body> `<script>window.addEventListener('load',()=>setTimeout(window.print,300))</script>` — авто-печать ПОСЛЕ полной загрузки (чтобы data:-логотип успел отрисоваться), +300мс fallback. Инжект ТОЛЬКО в desktop-ветке — LAN не затронут.

## Eliminated

- timestamp: "2026-07-05T12:05:00Z"
  hypothesis: "shell.open() из JS включается/разрешается через capability-permission `shell:allow-open` с path-scope `{path: $TEMP/**/*.html}` в capabilities/main.json (по аналогии с fs-scope)."
  why_eliminated: Опровергнуто чтением исходника tauri-plugin-shell scope.rs: команда `open` валидируется regex-ом из `tauri.conf.json > plugins > shell > open`, а НЕ capability-scope. capability `shell:allow-open` разрешает вызов команды, но при отсутствии `plugins.shell.open` в конфиге сам open() применяет невозможный regex `tauri^` и всегда отклоняет. path-scope в capability для open() не участвует в валидации пути. Значит добавление `shell:allow-open` (что я сделал в первой итерации) было необходимым, но НЕ достаточным — без `plugins.shell.open` в tauri.conf.json печать падает.

- timestamp: "2026-07-05T11:26:00Z"
  hypothesis: "Фикс = переключить iframe.contentWindow.print() на top-level window.print() в главном окне (печать скрытого div с @media print)."
  why_eliminated: Опровергнуто Tauri #3066 — на macOS WKWebView даже top-level window.print() бросает `unrecognized selector` / no-op. Паттерн 'печать div' в главном окне не работает на macOS desktop. (На Windows/WebView2 сработал бы, но фикс должен работать в обоих desktop-ОС, D-09.) Годится ТОЛЬКО для LAN-браузерного пути, где он уже фактически и используется.

## Root Cause

**Печать в Tauri desktop-webview на macOS структурно не работает через JS-print.** WKWebView не реализует нативную панель печати без host-side интеграции (Objective-C `printOperationWithPrintInfo:` не реализован во Wry — Tauri #3066), поэтому и `window.print()`, и тем более `iframe.contentWindow.print()` на `srcdoc`-iframe (#13451) — тихий no-op (без исключения, отсюда отсутствие тоста). В Phase 16 путь предпросмотра актов был переведён на `<iframe srcdoc>` + `iframeEl.contentWindow.print()`, что работает в настоящем браузере (LAN-режим, Chrome/Edge/WebView2), но НЕ в desktop-webview на macOS.

Вторичная причина растянутого preview: `.pdf-iframe { width: 100% }` без A4-обёртки; `@page` влияет только на печать, не на экранный рендер.

**Вывод для фикса:** desktop-путь должен НЕ полагаться на webview-печать. Наиболее совместимо с ограничениями проекта (portable, dual-transport, D-09) — открывать документ во внешнем системном приложении (браузер по умолчанию), где нативный диалог печати работает всегда; LAN-браузерный путь (`window.print()`/iframe-print уже работает) оставить без изменений, ветвясь по `isTauri`.

## Resolution

root_cause: "Tauri desktop-webview на macOS (WKWebView/Wry) не реализует нативную печать надёжно — window.print() и iframe.contentWindow.print() являются no-op/сбоят без host-side интеграции (Tauri #3066 / #13451). Phase 16 перевёл предпросмотр акта на srcdoc-iframe + contentWindow.print(), что работает в настоящем браузере (LAN), но молча ничего не делает в desktop-webview. Плюс экранный preview растянут т.к. .pdf-iframe{width:100%} без A4-обёртки, а @page действует только при печати."
fix: |
  ui/src/features/acts/PdfPreviewModal.svelte:
  - handlePrint() now branches on isTauri (same pattern as ReportsPage.svelte's printReport()):
    - Desktop (isTauri=true): printViaSystemBrowser(html) — injects an auto-print script
      before </body> (`window.addEventListener('load', () => setTimeout(window.print, 300))`
      so the data:image logo paints before the dialog), writes the rendered HTML to
      $TEMP/trackly-print-<timestamp>.html via tauri-plugin-fs writeTextFile(baseDir: Temp),
      resolves the full path via @tauri-apps/api/path (tempDir + join), then opens it with
      tauri-plugin-shell's open() — launches the OS default browser, where native print
      already works (proven by the existing LAN-browser path) and now shows the dialog
      automatically. Injection is desktop-branch-only; LAN path untouched.
    - LAN browser (isTauri=false): printViaTopLevel(html) — parses the self-contained HTML,
      extracts its <style> (incl. @page) and <body> markup, injects them into a hidden
      #act-print-root host + #act-print-style in the TOP-LEVEL document (not the iframe),
      scopes visibility with @media print (hide body > :not(#act-print-root)), then calls
      window.print() on the top-level window and cleans up on 'afterprint'. Removes the
      previous iframe.contentWindow.print() call and the now-unused iframeEl binding.
  - Secondary fix: on-screen preview iframe wrapped in .pdf-page-frame (centered, scrollable
    A4-sized 794x1123px .pdf-iframe with border + shadow) instead of stretching full-width.
  - handlePrint() catch shows a clean user toast «Не удалось открыть документ для печати».
    (During debugging this temporarily surfaced the raw error to pinpoint the failing plugin
    step across the 4 cycles; reverted to the generic toast after final confirmation.)
  crates/trackly-app/tauri.conf.json:
  - THE KEY FIX for the 1st-verification failure: added `plugins.shell.open` regex
    "^(https?://|file://|/|[A-Za-z]:[\\\\/]).*trackly-print-\\d+\\.html$". Without this key,
    tauri-plugin-shell's open() applies an impossible regex ("tauri^") and denies ALL
    open() calls from JS (scope.rs:207-227) — that was why the desktop print threw.
    Regex is narrow: only allows absolute/URL paths ending in trackly-print-<digits>.html.
  crates/trackly-app/capabilities/main.json:
  - THE KEY FIX for the 2nd-verification failure: added a separate `fs:allow-write-text-file`
    permission with inline scope [{$TEMP/*.html},{$TEMP/**/*.html}]. JS writeTextFile() invokes
    the plugin command `write_text_file`, allowed ONLY by fs:allow-write-text-file — NOT by the
    fs:allow-write-file I had granted (that allows the different command write_file / binary
    writeFile). Different commands → write_text_file stayed denied. fs:allow-write-file is kept
    (ReportsPage needs it for binary writeFile of PDF bytes) and left PDF-only, NOT extended to
    html.
  - Added shell:allow-open permission (plain string — the path-scope form was a red herring;
    open() path validation is governed by the tauri.conf.json regex above, not this scope).
    This permission-gate also unblocks the pre-existing (separately broken) ReportsPage.svelte
    openPath() calls once tauri.conf.json > plugins.shell.open is set.
  ui/eslint.config.js:
  - Added DOMParser and HTMLStyleElement to the browserGlobals allowlist (needed by the new
    printViaTopLevel implementation; project uses an explicit globals list, not env:browser).
  crates/trackly-app/src/http/mod.rs (4th cycle — GAP-16-01 residual defect #1):
  - Added `img-src 'self' data:` to the LAN CSP header. The act HTML embeds the org logo as a
    data:image URI; without img-src it fell back to default-src 'self' (no data:), so the logo
    was blocked in the LAN-browser preview and print. data: images cannot execute scripts →
    no added XSS surface (orthogonal to CR-01). Desktop was unaffected (opens file:// outside
    this CSP), confirming the LAN-only diagnosis.
  crates/trackly-app/tests/security_headers.rs (4th cycle):
  - Added regression assertion that CSP contains `img-src` + `data:` (alongside the existing
    frame-src+blob assertion). `cargo test -p trackly-app --test security_headers` with
    TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 + a real built ui/dist → 4 passed.
verification: |
  Progressive-config debugging across 4 human-verify cycles, each surfacing the next concrete
  blocker (proof of forward progress, not looping): (1) webview print broken → system-browser
  detour; (2) shell.open denied → plugins.shell.open regex [that toast disappeared];
  (3) writeTextFile denied → fs:allow-write-text-file [main mechanism then WORKED: desktop
  opens act in system browser, LAN prints]; (4) two residual defects — LAN logo blocked by CSP
  (→ img-src 'self' data:) and desktop print dialog not auto-shown (→ inject load→print script).
  Self-verified (automatable): `pnpm --dir ui build` succeeds; eslint/prettier clean;
  `svelte-check` 0 errors; `cargo test -p trackly-app --test security_headers` (AD/SNMP mock env
  + real ui/dist) → 4 passed including the new img-src+data: assertion; identifier
  fs:allow-write-text-file confirmed in plugin schema; JSON valid.
  HUMAN-VERIFIED (user confirmed 2026-07-05): after restarting cargo tauri dev — desktop «Печать»
  opens the act in the system browser and auto-shows the print dialog with the logo visible; LAN
  prints with the logo. Main mechanism + both residual defects (LAN logo, desktop auto-print)
  confirmed working. Temp diagnostics toast reverted to a clean user-facing toast
  «Не удалось открыть документ для печати» (no console.error/e.message leakage).
files_changed:
  - ui/src/features/acts/PdfPreviewModal.svelte
  - crates/trackly-app/tauri.conf.json
  - crates/trackly-app/capabilities/main.json
  - crates/trackly-app/src/http/mod.rs
  - crates/trackly-app/tests/security_headers.rs
  - ui/eslint.config.js
