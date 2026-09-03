---
status: awaiting_human_verify
trigger: "Печать отчёта «Перемещения» из LAN-браузера добавляет третий лист — дубликат первого; из десктопа та же печать работает верно."
created: 2026-09-03T00:00:00Z
updated: 2026-09-03T13:10:00Z
---

## Current Focus

reasoning_checkpoint:
  hypothesis: |
    printViaTopLevel() (ui/src/features/acts/PdfPreviewModal.svelte) calls
    `previewer.preview(bodyHtml, [{ 'act-preview.css': cssText }], printRoot)`
    with an EXPLICIT `stylesheets` argument. Passing that argument makes
    pagedjs 0.4.3's `Previewer.preview()` skip its own `removeStyles()` call
    (`if (!stylesheets) { stylesheets = this.removeStyles(); }` — pagedjs
    source, dist/paged.esm.js:33159-33161). `removeStyles()` is what
    normally finds EVERY `<style>` element anywhere in the document (not
    just `head`) and `element.remove()`s it before chunking. Because this
    call site supplies its own `stylesheets` (built from `head > style`
    only — `_header.html`'s `<style>` tag lives in `<body>` BY DESIGN, see
    that file's own IN-02 doc comment), that stray `<style>` node is never
    removed and flows into `bodyHtml` as literal CONTENT handed to
    `previewer.preview()`. pagedjs 0.4.3's Chunker, when a `<style>` tag
    (zero rendered height) is the first flowed node, has a page/overflow
    bookkeeping defect that duplicates the following real page's content
    onto an extra leading page. This is why LAN print (printViaTopLevel,
    the only path that passes an explicit `stylesheets` arg) shows the
    defect while desktop (printViaSystemBrowser) and the on-screen preview
    (bootstrapScript.js) do not — both of those call `previewer.preview()`
    WITHOUT a `stylesheets` argument, so pagedjs's own `removeStyles()` runs
    and strips the stray `<style>` tag before chunking ever sees it.
  confirming_evidence:
    - "Live reproduction in the REAL running app (throwaway server-mode instance,
      real Chrome via puppeteer-core, real login + Reports UI + Печать button,
      NOT a synthetic harness): DOM query at the moment window.print() was called
      showed #act-print-root containing 5 .pagedjs_page nodes for a report the
      preview correctly paginated to 4 pages; node[0]'s textContent was pure CSS
      text from _header.html's leaked <style> tag."
    - "Same live session: rendered the ACTUAL top-level DOM to a real PDF via
      Chrome's own print pipeline (puppeteer page.pdf(), print media, no
      overrides) at that exact moment. PDF has 5 pages; page 0 and page 1 are
      BYTE-IDENTICAL (2791 chars each, same header + same first table row) —
      i.e. the physically printed output has the first real page duplicated,
      exactly matching the user's report."
    - "Minimal isolated repro using the REAL pagedjs 0.4.3 module (ui/node_modules,
      not reconstructed) in a real Chrome tab: previewer.preview(realContent, ...)
      alone → 3 correct pages, no duplicates. previewer.preview(leadingStyleTag +
      realContent, ...) → 4 pages, page[1] an exact duplicate of page[0]'s real
      content. Confirms the mechanism in isolation, independent of the app's own
      code."
    - "pagedjs source read directly (dist/paged.esm.js Previewer.preview() /
      removeStyles()): removeStyles() queries `style:not([data-pagedjs-inserted-styles],
      [data-pagedjs-ignore],[media~='screen'])` across doc.querySelectorAll — head
      AND body — and only runs when the caller does NOT pass its own stylesheets
      argument. printViaTopLevel always passes one; the preview/desktop bootstrap
      paths never do."
  falsification_test: |
    If cssText/style extraction in printViaTopLevel is changed to strip ALL
    <style> elements from the whole parsed document (head + body) before
    computing bodyHtml — not just head > style — the duplicate-first-page
    defect must disappear in the same live repro (real app, real browser,
    real print-to-PDF render): page count must drop back to matching the
    preview's page count, and no two adjacent pages may have identical text.
  fix_rationale: |
    The root cause is that printViaTopLevel's own style-extraction is
    narrower than pagedjs's own default (head-only vs whole-document), which
    both (a) causes the LAN-only duplicate-page defect (this bug) and (b)
    was ALREADY flagged as a separate "hidden risk" in prior evidence —
    header CSS never reaching the Polisher because head > style missed the
    body-scoped style tag. Both symptoms share one mechanism and one fix:
    collect every <style> element in the parsed document (mirroring
    pagedjs's removeStyles()), REMOVE them from the DOM before reading
    bodyHtml, and pass their combined text as the stylesheets argument.
    Removing the stray <style> node from bodyHtml eliminates it from the
    content stream Chunker walks, which is what upstream pagedjs itself does
    by default on every other print/preview path in this app.
  blind_spots: |
    Have not fully explained pagedjs 0.4.3's INTERNAL chunker logic for WHY a
    leading zero-height <style> node causes it to duplicate the next page
    (did not read Chunker.render()/layout() internals) — treated as a
    confirmed-by-isolation defect of that specific library version, not
    further root-caused inside pagedjs itself. Not verified whether this
    pagedjs bug is already fixed in a newer pagedjs release (out of scope —
    the app pins 0.4.3). Have not checked whether OTHER stray <style> tags
    could exist elsewhere in act/acceptance templates (only report/_header.html
    confirmed) — the fix is general (strips every <style> in the document) so
    it should cover any future occurrence, but this was not enumerated.
next_action: READY FOR USER VERIFICATION (see Resolution below). Fix applied,
  reproduced live 3x consecutively BEFORE and AFTER the fix on the real
  throwaway server/browser, confirmed against an actual rendered PDF
  (pymupdf: 5 pages -> 4 pages, duplicate gone, no two adjacent pages
  identical anymore). svelte-check/eslint/build/privacy-check all green.
  Waiting on user confirmation on their real Windows environment (same
  "Печать / Экспорт PDF" button on the "Перемещения" report).

## Symptoms

expected: Печать/экспорт PDF отчёта «Перемещения» из LAN-браузера даёт тот же результат, что и из десктопа — столько же страниц, сколько показал предпросмотр (2).
actual: В предпросмотре 2 страницы; при отправке на принтер / сохранении в PDF появляется третий лист — дубликат первого. Из десктопа печать корректна.
errors: нет
reproduction: Тест 18 фазы 40. `pnpm --dir ui build`, открыть приложение в LAN-браузере под менеджером, Отчёты → Перемещения → Печать/Экспорт PDF.
started: обнаружено при UAT фазы 40 (2026-09-03)

## Eliminated

- hypothesis: Протечка print-каскада — при печати дополнительно печатается сама страница приложения или модалка предпросмотра (iframe).
  evidence: Правило `@media print { body > :not(#act-print-root) { display: none !important } }`
    покрывает все узлы: Svelte монтируется в `#app` (прямой потомок `body`, ui/index.html),
    портал-узлы `[data-tr-portal]` тоже вешаются на `body`. Харнесс с открытой fixed-модалкой
    и смонтированным iframe предпросмотра напечатал ровно 2 листа — вклада модалки нет.
  timestamp: 2026-09-03T03:05:00Z

- hypothesis: Расхождение шрифтовых/геометрических метрик между iframe предпросмотра и
    top-level документом приложения даёт разное разбиение (2 против 3 страниц).
  evidence: Возможный виновник — глобальный ресет `*{box-sizing:border-box}` (global.scss) —
    не работает: сам Paged.js объявляет `.pagedjs_pagebox * { box-sizing: border-box }`,
    т.е. border-box действует ОДИНАКОВО на обоих путях. Прочие унаследованные свойства
    (line-height/letter-spacing/word-spacing) уже нейтрализованы фиксом 260805-jwf/ifj.
    Кроме того, «расхождение разбиения» дало бы третий лист с ХВОСТОМ таблицы, а не
    с дубликатом первого листа.
  timestamp: 2026-09-03T03:20:00Z

- hypothesis: Лишний лист порождает сам браузер (перелив страницы Paged.js за границу
    печатного листа из-за разорванной цепочки `height:100%`, т.к. между `body` и
    `.pagedjs_pages` вклинен `#act-print-root` с height:auto).
  evidence: В Chrome-харнессе число печатных листов всегда совпадало с числом
    `.pagedjs_page` (2 = 2) при полном каскаде приложения. Механизм остаётся реальным
    структурным риском (см. Evidence), но в наблюдаемом виде лишних листов не даёт —
    и дал бы пустую/срезанную полосу, а не дубликат первого листа.
  timestamp: 2026-09-03T03:25:00Z

## Evidence

- timestamp: phase-0
  checked: .planning/debug/knowledge-base.md
  found: Совпадение по ключевым словам с записью desktop-webview-print-dialog. Там зафиксировано: LAN-ветка печати внедряет style+body отрендеренного документа в скрытый #act-print-root в ТОП-УРОВНЕВОМ документе и печатает его, а не iframe.
  implication: Виновника надо искать в top-level DOM, а не в содержимом отчёта.

- timestamp: 2026-09-03T02:40:00Z
  checked: ui/src/features/acts/PdfPreviewModal.svelte — три пути печати
  found: |
    Три РАЗНЫХ движка вывода:
    (1) предпросмотр — srcdoc-iframe, изолированный документ, UMD-бутстрап
        (ui/src/lib/pdfPreview/bootstrapScript.js), `previewer.preview()` БЕЗ аргументов
        (wrapContent + removeStyles, renderTo = body самого iframe);
    (2) десктоп — printViaSystemBrowser(): временный HTML-файл открывается в системном
        браузере, тот же бутстрап, renderTo = body отдельного документа;
    (3) LAN — printViaTopLevel(): `import('pagedjs')` (ESM) и
        `previewer.preview(bodyHtml, [{...cssText}], printRoot)`, где
        printRoot = общий div `#act-print-root` в документе ПРИЛОЖЕНИЯ.
  implication: Только путь (3) рендерит в долгоживущий разделяемый контейнер. Асимметрия
    «десктоп верно / LAN нет» структурно объясняется именно этим.

- timestamp: 2026-09-03T02:55:00Z
  checked: ui/node_modules/pagedjs/dist/paged.esm.js — Chunker.flow / Chunker.setup (0.4.3)
  found: |
    `setup(renderTo)` создаёт НОВЫЙ `<div class="pagedjs_pages">` и делает
    `renderTo.appendChild(this.pagesArea)` — цель рендера НИКОГДА не очищается.
    Ветка `removePages()` в `flow()` срабатывает только при повторном использовании
    ТОГО ЖЕ экземпляра Chunker; `printViaTopLevel` каждый раз создаёт `new Previewer()`
    → новый Chunker → всегда setup() → всегда appendChild.
  implication: Каждый вызов printViaTopLevel ДОПИСЫВАЕТ полную копию документа в
    `#act-print-root`. Идемпотентности нет.

- timestamp: 2026-09-03T02:58:00Z
  checked: ui/src/features/acts/PdfPreviewModal.svelte — жизненный цикл printRoot и cleanup
  found: |
    - `#act-print-root` создаётся один раз и переиспользуется
      (`document.getElementById(PRINT_ROOT_ID)`), при повторном вызове НЕ очищается
      перед `previewer.preview()`.
    - `printRoot.innerHTML = ''` живёт ТОЛЬКО в обработчике `window.addEventListener('afterprint', cleanup)`.
    - `handlePrint()` не имеет защиты от повторного входа; кнопка «Печать» в футере
      модалки блокируется только по `loading`/`paginationStatus`, которые в момент
      печати уже равны 'done' — то есть кнопка остаётся активной всё время, пока
      Paged.js заново пагинирует документ (для многостраничного отчёта это заметные секунды
      без какой-либо обратной связи).
    - `registerHandlers(RepeatTableHeadHandler)` вызывается при КАЖДОМ вызове и
      накапливается в глобальном реестре pagedjs (побочный, но того же класса дефект).
  implication: Второй вызов (двойной клик по «Печать» / повторная попытка «на принтер»,
    затем «сохранить в PDF») складывает второй `.pagedjs_pages` поверх первого.

- timestamp: 2026-09-03T03:15:00Z
  checked: |
    Chrome-харнесс (реальный Blink, /Applications/Google Chrome.app, headless
    --print-to-pdf): синтетический отчёт, собранный ИЗ РЕАЛЬНЫХ шаблонов
    crates/trackly-app/templates/report.html + _header.html с обезличенными данными;
    прогнаны три конфигурации — (a) чистый документ + polyfill (аналог десктопа/предпросмотра),
    (b) документ приложения (полный собранный ui/dist CSS) + точная копия printViaTopLevel,
    (c) то же, что (b), плюс открытая fixed-модалка со смонтированным iframe предпросмотра.
  found: |
    Во всех конфигурациях число листов в PDF совпадало с числом отрисованных
    `.pagedjs_page` (2 = 2). Лишний лист не возникал ни от каскада приложения,
    ни от модалки/iframe, ни от геометрии `@page`/`height:100%`.
    (Ограничение харнесса: --dump-dom снимает DOM до окончания пагинации, поэтому
    точные пороги разбиения по нему мерить нельзя; вывод 1:1 «листы ↔ страницы»
    получен из самих PDF и от этого ограничения не зависит.)
  implication: Печатных листов ровно столько, сколько страниц Paged.js в DOM. Значит третий
    лист — это НАСТОЯЩАЯ третья страница Paged.js, содержащая контент первой страницы.
    В рамках ОДНОГО прогона Paged.js такого не порождает (контент потребляется break-token'ами),
    следовательно в DOM на момент печати было ДВЕ копии рендера.

- timestamp: 2026-09-03T03:22:00Z
  checked: crates/trackly-app/templates/report.html, _header.html, tauri_cmds/reports.rs
  found: |
    Отчёт «Перемещения» не имеет ничего структурно особенного: тот же `@page {size: A4
    portrait; margin: 20mm 15mm}`, что и у актов, тот же общий партиал шапки, 7 колонок,
    никакого landscape/особой разметки. Побочно зафиксировано: `_header.html` эмитит свой
    `<style>` ВНУТРИ `<body>`, поэтому `printViaTopLevel` (он собирает только `head > style`)
    не передаёт CSS шапки полишеру — но сам узел `<style>` попадает в поток контента и
    в отрисованной странице присутствует (проверено в дампе DOM), так что вёрстка шапки
    не ломается. Это скрытый риск, а не причина текущего дефекта.
  implication: Причина не в содержимом отчёта. Отчёт лишь ДОЛЬШЕ пагинируется, чем акт,
    и потому чаще ловит повторный клик по «Печать» — этим и объясняется, почему дефект
    всплыл на отчёте, а не на актах.

- timestamp: 2026-09-03T09:55:00Z
  checked: |
    Живой UAT на Windows, сборка 1.4.0-phase40 (draft untagged-e8e506645b4ab18903ec),
    ПОСЛЕ применения плана 40-27, который реализовал фикс по диагнозу этой же сессии.
    Проверено чтением кода, что фикс на месте: PdfPreviewModal.svelte
    printViaTopLevel() делает activePolisher?.destroy() + printRoot.innerHTML=''
    БЕЗУСЛОВНО в начале каждого прогона (~строки 393-395), handlePrint() имеет
    re-entrancy guard `printing` (~строки 542-556).
  found: |
    Дефект ВОСПРОИЗВОДИТСЯ. Пользователь: «По прежнему при работе через браузер по LAN
    и экспорте PDF с отчётом о перемещениях, дублируется первая страница.»
  implication: |
    ГИПОТЕЗА ОПРОВЕРГНУТА. Накопление копий в #act-print-root между прогонами — не
    причина (или не единственная причина). Обрати внимание: прежний диагноз ТРЕБОВАЛ
    двух запусков печати в одной сессии модалки; сейчас дубликат возникает и при
    обычном сценарии. Ищи механизм, дающий дубликат первой страницы за ОДИН прогон.
    Отдельно отметить как подозреваемое, но не проверенное живьём:
    (а) `_header.html` эмитит свой <style> ВНУТРИ <body> — printViaTopLevel собирает
        только `head > style`, поэтому CSS шапки не попадает полишеру, а сам узел
        <style> уходит в поток контента (зафиксировано как «скрытый риск» ранее);
    (б) WR-04 из код-ревью раунда 2: слушатель `afterprint` предыдущего прогона
        снимается только внутри самого cleanup;
    (в) registerHandlers(RepeatTableHeadHandler) теперь одноразовый, но класс
        переобъявляется на каждый вызов — зарегистрирован остаётся класс ПЕРВОГО
        прогона, с его снимком savedThead.

## Resolution

superseded_root_cause: |
  ОПРОВЕРГНУТ живым UAT 2026-09-03 (см. последнюю запись в Evidence). Оставлен для
  истории — фикс по нему применён планом 40-27 и корректен сам по себе (устраняет
  реальную неидемпотентность), но симптом не устраняет.
  Путь LAN-печати `printViaTopLevel()` (ui/src/features/acts/PdfPreviewModal.svelte)
  не идемпотентен: он рендерит Paged.js в ДОЛГОЖИВУЩИЙ разделяемый контейнер
  `#act-print-root`, не очищая его перед рендером, а `Chunker.setup()` в pagedjs 0.4.3
  всегда ДОПИСЫВАЕТ новый `<div class="pagedjs_pages">` через `renderTo.appendChild`.
  Единственная очистка висит на событии `afterprint`, а `handlePrint()` не защищён от
  повторного входа (кнопка «Печать» остаётся активной всё время пагинации).
  Поэтому второй запуск печати в той же сессии модалки складывает вторую копию документа
  поверх первой. К моменту, когда Chrome снимает layout для печати, второй прогон обычно
  успевает отрисовать только первую страницу — итог [стр.1][стр.2][стр.1], то есть
  «третий лист = дубликат первого».
  Десктопная ветка (printViaSystemBrowser) от этого свободна: каждый вызов пишет свой
  временный файл и печатает его в отдельном документе системного браузера — общего
  накапливающего DOM там нет.
root_cause: |
  `printViaTopLevel()` (ui/src/features/acts/PdfPreviewModal.svelte) calls
  `previewer.preview(bodyHtml, [{ 'act-preview.css': cssText }], printRoot)`
  with an explicit `stylesheets` argument. pagedjs 0.4.3's
  `Previewer.preview()` only runs its own `removeStyles()` (which finds and
  `element.remove()`s EVERY <style> tag anywhere in the document, head or
  body) when the caller does NOT pass a `stylesheets` argument. Both other
  render paths (preview iframe's bootstrapScript.js, desktop's
  printViaSystemBrowser) call `previewer.preview()` with no `stylesheets`
  arg, so pagedjs's own removeStyles() always strips every <style> tag for
  them. printViaTopLevel is the only path supplying its own stylesheets
  (built from `head > style` only), which meant `_header.html`'s <style>
  tag — emitted inside <body> BY DESIGN (see that file's own IN-02 doc
  comment: browsers apply <style> CSS correctly regardless of head/body
  placement, so this was never a problem for the other two paths) — was
  never removed and flowed into `bodyHtml` as literal PAGINATED CONTENT
  instead of CSS. pagedjs 0.4.3's Chunker, given a fed-content string whose
  first flowed node is a <style> element (zero rendered height), has a
  page/overflow-bookkeeping defect: it produces one extra leading page whose
  printed content duplicates the real first page. Root-caused via a minimal
  isolated repro against the real pagedjs 0.4.3 module (not the app): 3
  correctly-paginated pages became 4 (with the extra page a duplicate of the
  first) purely from prepending a <style> tag to the fed content.
fix: |
  In `printViaTopLevel()`, collect `Array.from(parsed.querySelectorAll('style'))`
  (whole document — head AND body, not just `head > style`), build `cssText`
  from their `textContent`, then `el.remove()` each one from the parsed DOM
  BEFORE reading `bodyHtml = parsed.body.innerHTML`. This mirrors pagedjs's
  own `removeStyles()` behaviour exactly, so the stray <style> tag is
  removed from the content stream before pagedjs's Chunker ever sees it —
  eliminating the duplicate-page defect — and as a side effect also fixes a
  previously-flagged issue (header CSS never reaching the Polisher, since it
  used to be collected from `head > style` only).
verification: |
  Live reproduction (NOT a synthetic harness): built ui/dist, ran the real
  `trackly` binary in server mode against a throwaway SQLite DB (fictional
  org/people only — "ООО Ромашка", "Иванов И.И.", "Петров П.П." — never the
  user's real DB), seeded via direct SQL with 60 place_movements rows to
  force multi-page pagination. Drove a real headless Chrome (puppeteer-core)
  through the actual app UI: login, Отчёты -> Перемещения -> «Печать /
  Экспорт PDF». window.print() was overridden (installed before any app
  script runs) to snapshot the live #act-print-root DOM instead of blocking
  on a native dialog puppeteer can't drive, and separately the SAME DOM
  state was rendered to a real PDF via Chrome's own print pipeline
  (puppeteer page.pdf(), print media — what "Save as PDF" actually
  produces).
  BEFORE fix: preview footer said "4 страницы"; live DOM had 5
  `.pagedjs_page` nodes; rendered PDF had 5 pages with page 0 and page 1
  BYTE-IDENTICAL (2791 chars each, same header + same first table row) —
  reproducing the user's exact symptom.
  AFTER fix (rebuilt ui/dist, restarted throwaway server, re-ran the
  identical script 3x consecutively for stability): preview footer "4
  страницы"; live DOM has 4 `.pagedjs_page` nodes; rendered PDF has 4 pages,
  no two adjacent pages share identical text; page 0 renders the header
  (with its CSS now correctly applied — flex-column layout, borders,
  alternating row shading all present in a rendered screenshot) followed by
  real table rows, no leaked CSS text anywhere.
  Additional gates run and green: `svelte-check` (0 errors — an intermediate
  edit briefly introduced a "script left open" parse error caused by writing
  a literal `<style>` tag spelling inside a `<script>`-block comment, which
  Svelte's compiler tokenizes even inside comments; fixed by rewording the
  comment to avoid the literal angle-bracket spelling, same reason the
  pre-existing `pagedjsScript` string a few lines above is built via `'<' +
  'script>'` concatenation), `eslint` (clean), `pnpm --dir ui build`
  (clean), `node scripts/check-privacy.mjs` (PASS, 0 violations).
  NOT verified: real Windows LAN-browser environment (this investigation ran
  on macOS dev against a throwaway server instance) and the act/acceptance
  print paths specifically (they share the exact same printViaTopLevel code
  path and the exact same `_header.html` partial, so the same defect and fix
  apply structurally, but a live UAT round on an act was not separately run
  — noted as a blind spot). User confirmation on their real environment is
  the remaining checkpoint before this session can be archived.
files_changed:
  - ui/src/features/acts/PdfPreviewModal.svelte
