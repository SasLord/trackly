---
phase: 24-base-components
reviewed: 2026-07-18T12:00:00Z
depth: standard
files_reviewed: 21
files_reviewed_list:
  - ui/src/features/layout/sidebar-config.ts
  - ui/src/features/showcase/ShowcasePage.svelte
  - ui/src/features/showcase/sections/BadgeSection.svelte
  - ui/src/features/showcase/sections/ButtonsSection.svelte
  - ui/src/features/showcase/sections/FieldsSection.svelte
  - ui/src/features/showcase/sections/ModalSection.svelte
  - ui/src/features/showcase/sections/TabsSection.svelte
  - ui/src/lib/components/Badge.svelte
  - ui/src/lib/components/Button.svelte
  - ui/src/lib/components/Checkbox.svelte
  - ui/src/lib/components/Input.svelte
  - ui/src/lib/components/Modal.svelte
  - ui/src/lib/components/Radio.svelte
  - ui/src/lib/components/Select.svelte
  - ui/src/lib/components/Tabs.svelte
  - ui/src/lib/components/Textarea.svelte
  - ui/src/lib/stores/theme.svelte.ts
  - ui/src/pages/ComponentShowcasePage.svelte
  - ui/src/routes.ts
  - ui/src/styles/_tokens.scss
  - ui/src/styles/global.scss
findings:
  critical: 2
  warning: 15
  info: 0
  total: 17
status: issues_found
---

# Phase 24: Code Review Report (re-review after 24-08 / 24-09 / 24-10)

**Reviewed:** 2026-07-18
**Depth:** standard
**Files Reviewed:** 21
**Status:** issues_found

## Summary

Повторный просмотр после трёх gap-closure планов. `svelte-check --threshold error` — 0 ошибок (252 файла, 49 warning'ов вне scope этой фазы).

### Статус находок прошлого раунда

| ID | Тема | Статус |
|----|------|--------|
| CR-01 | `bind:value` односторонний у Input/Select/Textarea | **Исправлено (24-08).** Все три компонента используют `let` + `bind:value` на нативном элементе (`Input.svelte:13,32`, `Select.svelte:18,34`, `Textarea.svelte:12,30`). Побочный эффект — см. WR-06 ниже. |
| CR-02 | `:global()` в plain-`.scss` ломал D-09 | **Исправлено (24-08).** `global.scss:64-67` — обычные селекторы `.theme-switching, .theme-switching *`. Grep-гейт в CI не добавлен (см. WR-15). |
| CR-03 | Modal не управляет фокусом | **Исправлено частично (24-10).** Initial focus, Tab-trap и восстановление фокуса появились (`Modal.svelte:23-67`), но реализация имеет дефекты — CR-02 (новый), WR-01…WR-04. |
| WR-03 | `appearance="count"` только для 2 тонов из 5 | **Исправлено частично (24-09).** Тон-специфичные count-правила добавлены и по специфичности выигрывают (0,2,0 > 0,1,0). Осталась поломка `size="sm"` и рассинхрон высот — WR-05. |
| WR-01, WR-02, WR-04…WR-08, IN-01…IN-06 | — | **Не тронуты.** Перенесены ниже (переформулированы там, где прошлая формулировка была неточна — см. WR-09). |

Два дефекта уровня BLOCKER — оба в новом коде focus-management'а или обострены им.

## Structural Findings (fallow)

Структурный пре-пасс для этого раунда не передавался — раздел пуст намеренно.

## Narrative Findings (AI reviewer)

## Critical Issues

### CR-01: Escape вызывает `onClose` дважды — после 24-10 это гарантировано на каждом закрытии

**Severity:** BLOCKER
**File:** `ui/src/lib/components/Modal.svelte:85, 92`
**Issue:** `handleKeydown` навешан одновременно на `<svelte:window onkeydown={open ? handleKeydown : undefined}>` (строка 85) и на сам бэкдроп (`onkeydown={handleKeydown}`, строка 92). Событие `keydown` от элемента внутри модалки обрабатывается на бэкдропе, затем всплывает до `window` → `onClose()` выполняется дважды.

В прошлом раунде это было отмечено как WARNING именно потому, что зависело от того, где находится фокус: при фокусе вне модалки срабатывал только window-обработчик. **24-10 устранил эту неопределённость в худшую сторону** — теперь `$effect` (строки 32-46) при каждом открытии переводит фокус *внутрь* `dialogEl`, то есть путь всплытия всегда проходит через бэкдроп. Двойной вызов стал детерминированным для всех ~25 потребителей `Modal` в приложении.

Сейчас большинство `onClose` идемпотентны (`onClose={() => (x = false)}`), поэтому визуально ничего не ломается — и именно поэтому дефект переживёт ручную проверку витрины. Но контракт компонента «onClose вызывается один раз на закрытие» нарушен: любой потребитель с побочным эффектом (откат черновика, `pushToast`, POST отмены брони, pop из стека модалок) отработает дважды. `DeviceImportCsvModal.svelte:201` уже передаёт не-тривиальный `handleClose`.

**Fix:** оставить один источник события. Минимальная правка — убрать обработчик с бэкдропа, window-обработчик покрывает оба случая:
```svelte
<div
  class="modal-backdrop"
  onmousedown={handleBackdropMousedown}
  onmouseup={handleBackdropMouseup}
  aria-modal="true"
  role="dialog"
  aria-labelledby={titleId}
  tabindex="-1"
>
```
Альтернатива (если нужен приоритет вложенных модалок) — оставить только локальный обработчик и добавить `e.stopPropagation()` перед `onClose()`.

---

### CR-02: Список узлов focus-trap неполон — iframe внутри модалки недостижим с клавиатуры

**Severity:** BLOCKER
**File:** `ui/src/lib/components/Modal.svelte:27-30, 48-67`
**Issue:** `TRAP_FOCUSABLE_SELECTOR` перечисляет `button/[href]/input/select/textarea/[tabindex]`. В списке нет `iframe`, `[contenteditable]`, `audio[controls]`, `video[controls]`, `summary`, `area[href]`. `iframe` — фокусируемый элемент по умолчанию и участвует в нативном tab-order.

Последствие воспроизводится на боевых экранах:
- `ui/src/features/acts/PdfPreviewModal.svelte:288` — `<iframe sandbox="" srcdoc={htmlContent} class="pdf-iframe">` внутри `Modal`;
- `ui/src/features/settings/TemplateEditor.svelte:267` — то же для превью шаблона.

До 24-10 пользователь клавиатуры доходил табом до iframe и мог прокручивать документ. Теперь `nodes` его не содержит, поэтому `last` — это кнопка футера, и `Tab` на ней принудительно возвращает фокус на `first` (`Modal.svelte:63-66`). Превью акта и превью шаблона стали недоступны с клавиатуры полностью. Это не деградация «в теории»: печать акта — основной сценарий приложения.

Смежный случай той же причины: дропдауны автокомплитов переносятся `use:portal` в `<body>` (`ui/src/lib/utils/portal.ts:24`, `LocationAutocomplete.svelte:154`, `PersonAutocomplete.svelte:221`), то есть физически лежат вне `dialogEl` и в trap не попадают вообще — ни в `nodes`, ни под обработчик `onkeydown` (он навешан на `dialogEl`). Автокомплиты используются в 5 модалках (`ReturnModal`, `DocumentAcceptanceModal`, `OperationModal`, `PrinterCreateModal`, `RequestFormModal`).

**Fix:**
```ts
const TRAP_FOCUSABLE_SELECTOR = [
  'button:not([disabled])',
  '[href]',
  'input:not([disabled])',
  'select:not([disabled])',
  'textarea:not([disabled])',
  'iframe',
  '[contenteditable]:not([contenteditable="false"])',
  'audio[controls]',
  'video[controls]',
  'summary',
  '[tabindex]:not([tabindex="-1"])',
].join(', ');
```
Для portal-контента — либо помечать перенесённые узлы (`data-modal-portal`) и включать их в `nodes` через `document.querySelectorAll`, либо (надёжнее) перенести обработчик trap на `document` с проверкой «фокус внутри dialogEl ИЛИ внутри зарегистрированного portal-узла». Как минимум задокументировать ограничение в шапке `Modal.svelte`, чтобы фазы 25-30 не считали trap полным.

---

## Warnings

### WR-01: Начальный фокус всегда попадает на кнопку «Закрыть»

**Severity:** WARNING
**File:** `ui/src/lib/components/Modal.svelte:36-41`, разметка `104-107`
**Issue:** `dialogEl.querySelector(FOCUSABLE_SELECTOR)` возвращает первый фокусируемый узел в DOM-порядке, а первым в разметке идёт `<button class="modal-close">` из `<header>`. То есть во **всех** модалках приложения (формы актов, картриджей, заявок, пользователей) фокус после открытия оказывается на «×», а не на первом поле формы. Пользователь клавиатуры при открытии формы обязан сделать лишний Tab, а `Enter`/`Space` сразу после открытия закрывает окно.

Это не придирка к стилю: 25-30 фазы строят все формы поверх этого примитива, поведение зафиксируется.

**Fix:** добавить опциональный проп `initialFocus?: string | HTMLElement` и использовать его как приоритетную цель; либо искать первый фокусируемый узел в `.modal-body`, а `.modal-close` использовать как фолбэк:
```ts
const target =
  (typeof initialFocus === 'string' ? dialogEl?.querySelector<HTMLElement>(initialFocus) : initialFocus) ??
  dialogEl?.querySelector<HTMLElement>('.modal-body ' + FOCUSABLE_SELECTOR.split(', ').join(', .modal-body ')) ??
  dialogEl?.querySelector<HTMLElement>(FOCUSABLE_SELECTOR) ??
  dialogEl;
target?.focus();
```

### WR-02: Селектор начального фокуса не исключает disabled/скрытые узлы — фолбэк не срабатывает

**Severity:** WARNING
**File:** `ui/src/lib/components/Modal.svelte:27-28, 36-41`
**Issue:** `FOCUSABLE_SELECTOR` (для initial focus) в отличие от `TRAP_FOCUSABLE_SELECTOR` не содержит `:not([disabled])` и не фильтрует `offsetParent === null`. Если первый совпавший узел — `disabled` или скрытый, `first.focus()` — no-op, но ветка `else { dialogEl?.focus() }` не выполняется, потому что `first` истинно. Итог: фокус остаётся на элементе-триггере **за** бэкдропом, то есть ровно тот дефект, который 24-10 закрывал. Сейчас скрыто тем, что первым узлом почти всегда оказывается всегда-активная кнопка «Закрыть» (WR-01) — то есть баг замаскирован другим багом и вскроется, как только WR-01 починят.

**Fix:** использовать один селектор + один фильтр видимости/доступности для обоих путей и проверять результат по факту:
```ts
const target = focusableNodes()[0] ?? dialogEl;
target?.focus();
if (!dialogEl?.contains(document.activeElement)) dialogEl?.focus();
```

### WR-03: Фильтр `offsetParent !== null` отбрасывает элементы с `position: fixed`

**Severity:** WARNING
**File:** `ui/src/lib/components/Modal.svelte:53`
**Issue:** `offsetParent` равен `null` не только у скрытых элементов, но и у любого `position: fixed` (а также у `<body>`). Внутри модалки fixed-позиционирование — не экзотика: тот же `dropdownAnchor.ts`-слой строится именно на fixed. Такие узлы молча выпадут из `nodes`, что сдвинет `first`/`last` и снова разорвёт круг табуляции.

**Fix:** проверять видимость через `getClientRects().length > 0` (устойчиво и к fixed, и к `display: none`):
```ts
.filter((n) => n.getClientRects().length > 0)
```

### WR-04: Восстановление фокуса зависит от DOM-порядка модалок — при цепочке «закрыть A → открыть B» фокус может уехать из B

**Severity:** WARNING
**File:** `ui/src/lib/components/Modal.svelte:32-46`
**Issue:** cleanup `prevFocus?.focus()` выполняется при перезапуске эффекта. Если в одном flush'е модалка A закрывается, а B открывается, порядок эффектов определяется порядком компонентов в дереве. Когда B объявлена **раньше** A, сначала отработает initial-focus B, затем cleanup A вернёт фокус на свой триггер — то есть за пределы только что открытой модалки B. Цепочки модалок в коде уже есть (`DevicesPage.svelte:300,312` — acceptanceDevice → acceptancePayload; `RequestDetail.svelte` — 5 модалок в одном файле); сейчас порядок объявления случайно «правильный», и любая перестановка разметки тихо ломает фокус.

Смежно: `prevFocus` может указывать на узел, который к моменту закрытия уже удалён (модалка открыта из контекстного меню — `DeviceContextMenu.svelte:144`), тогда `focus()` — no-op и фокус падает на `<body>`.

**Fix:** восстанавливать фокус только если он всё ещё внутри закрываемого диалога, и проверять «жив» ли узел:
```ts
return () => {
  const el = prevFocus;
  prevFocus = null;
  if (el && el.isConnected) queueMicrotask(() => {
    if (document.activeElement === document.body) el.focus();
  });
};
```

### WR-05: `size="sm"` молча игнорируется у `appearance="count"`, высоты count-бейджей рассинхронизированы

**Severity:** WARNING
**File:** `ui/src/lib/components/Badge.svelte:98-105, 147-155, 170-178, 193-201, 204-223`
**Issue:** Два следствия одного и того же приёма из 24-09:
1. `.badge-m-{tone}.badge-m-count` имеет специфичность 0,2,0 и переопределяет `height`/`font-size`/`padding`, тогда как `.badge-m-sm` — 0,1,0. Значит `<Badge size="sm" appearance="count">` рендерится в размере `md` для accent/success/warning/danger. Проп принят и молча проигнорирован — худший вид отказа для примитива дизайн-системы.
2. Базовый нейтральный `.badge-m-count` (строки 204-213) задаёт `height: 18px`, `min-width: 18px`, без рамки; тон-специфичные — `height: 20px` + `border: 1px solid`. В `BadgeSection` count-бейджи разных тонов стоят в одинаковых рядах и имеют разную высоту и разное наличие рамки. Витрина существует ровно для выявления таких расхождений — здесь она их закрепляет как норму.

**Fix:** вынести размерность count'а из тон-блоков (в тонах оставить только `background`/`color`/`border-color`), а `height`/`font-size`/`padding` задавать в `.badge-m-count` и `.badge-m-count.badge-m-sm` — тогда `size` снова работает, а тона отличаются только цветом. Одновременно решить, есть ли у нейтрального count'а рамка, и привести все пять тонов к одному ответу.

### WR-06: `type="number"` + внутренний `bind:value` возвращает `number | null` в проп, объявленный как `string`

**Severity:** WARNING
**File:** `ui/src/lib/components/Input.svelte:2-11, 32`
**Issue:** Побочный эффект фикса 24-08. Svelte для `bind:value` на числоподобном `<input>` применяет числовое приведение в рантайме (`'' → null`, иначе `+value`). Тип `type` здесь динамический, поэтому проверка идёт на элементе, а не на этапе компиляции. Значит `<Input type="number" bind:value={s} />` запишет в `s` число либо `null`, хотя контракт `Props` объявляет `value: string`. `svelte-check` этого не увидит — приведение происходит внутри рантайма Svelte.

Практический риск: любой потребитель, следующий образцу из витрины (`bind:value`), на числовом поле получит `null` при очистке поля, и первый же `value.trim()`/`value.length` бросит `TypeError`. Существующий потребитель `ActNumberField.svelte:74-80` пока спасён тем, что использует одностороннюю передачу `value={displayValue}` + `oninput`, то есть держится в стороне от собственного же биндинга компонента.

**Fix:** либо сузить `Props` до `type?: 'text' | 'search'` и завести отдельный `NumberInput` с `value: number | null`, либо объявить `value: string | number | null` и явно нормализовать перед вызовом `oninput`. Молчаливое расхождение типа и рантайма — худший из трёх вариантов.

### WR-07: Tabs объявляет `role="tablist"`/`role="tab"`, но не реализует клавиатурный паттерн (перенесено, не исправлено)

**Severity:** WARNING
**File:** `ui/src/lib/components/Tabs.svelte:26-54`
**Issue:** Нет навигации стрелками ←/→ и Home/End, нет roving `tabindex` (все вкладки в tab-order вместо одной активной), нет `aria-controls`/`role="tabpanel"`. AT'у обещана семантика вкладок, поведение — обычных кнопок. Дополнительно `disabled` на `role="tab"` полностью убирает вкладку из восприятия AT — по паттерну корректнее `aria-disabled="true"` с сохранением фокусируемости. Вариант `segmented` (`role="group"` + `aria-pressed`) сделан честно и служит контрпримером в этом же файле.

**Fix:** добавить `onkeydown` с переносом `active` на соседнюю недизейбленную вкладку + `tabindex={tab.key === active ? 0 : -1}`; либо понизить семантику underline-варианта до `role="group"`, как в `segmented`.

### WR-08: Проп `invalid` у Select/Checkbox/Radio не транслируется в `aria-invalid` (перенесено, не исправлено)

**Severity:** WARNING
**File:** `ui/src/lib/components/Select.svelte:29-39`, `ui/src/lib/components/Checkbox.svelte:25-32`, `ui/src/lib/components/Radio.svelte:25`
**Issue:** `Input.svelte:34` и `Textarea.svelte:31` выставляют `aria-invalid={invalid || undefined}`, остальные три — нет: `invalid` меняет только рамку и тень. Для скринридера три из пяти полей в состоянии ошибки неотличимы от валидных, а цвет остаётся единственным носителем информации (WCAG 1.4.1). Несогласованность внутри одного набора примитивов гарантирует, что формы фаз 25+ будут доступны наполовину.

**Fix:** добавить `aria-invalid={invalid || undefined}` на `<select>` и оба `<input>`; заодно добавить проп `aria-describedby` (сейчас есть только у `Input`) для связи с текстом ошибки.

### WR-09: theme store — невалидированное значение из localStorage, незащищённый доступ, повторные подписки

**Severity:** WARNING
**File:** `ui/src/lib/stores/theme.svelte.ts:15-22, 24-28, 30-39`
**Issue:** Три проблемы:
1. `localStorage.getItem(KEY) ?? 'system') as Preference` — приведение типа без проверки; постороннее значение попадёт в `themeStore.preference` и в `dataset.theme`. *Уточнение к прошлому раунду: «приложение отрендерится без цветов» — неверно, `:root` в `_tokens.scss:12` перечислен рядом с `[data-theme='light']`, поэтому светлые токены останутся. Реальное следствие мягче — тема молча деградирует в light, а `applyResolved()` возвращает это значение как `Resolved`, хотя тип обещает `'light' | 'dark'`.*
2. `localStorage` вызывается без `try/catch`, тогда как `window` в том же файле проверяется (строки 12-13). В браузере с заблокированным хранилищем `getItem` бросает `SecurityError`; `main.ts:7` вызывает `initTheme()` на верхнем уровне до `mount()` и без обработки — приложение не смонтируется вообще (белый экран). Для LAN-браузерного режима это достижимый сценарий, и он приводит к полной недоступности, а не к сбою темы. `setTheme` (строка 26) вызывает `setItem` **до** `applyResolved()`, поэтому там же исключение оставит тему непереключённой.
3. `initTheme()` каждый вызов добавляет новый listener к `mql` и ничего не снимает; функция экспортирована и от повторного вызова не защищена.

**Fix:** валидирующий `readPreference()` с `try/catch`, `try/catch` вокруг `setItem` (и вызов `applyResolved()` **до** записи), флаг `initialized` в `initTheme()`.

### WR-10: Radio без пропа `name` и без семантики группы (перенесено, не исправлено)

**Severity:** WARNING
**File:** `ui/src/lib/components/Radio.svelte:4-25`, `ui/src/features/showcase/sections/FieldsSection.svelte:117-120`
**Issue:** `bind:group` связывает радиокнопки на уровне JS, нативной группы нет: стрелочная навигация между кнопками одной группы не работает, каждая — отдельная точка табуляции, в `<form>` значения не сериализуются, скринридер не объявляет «1 из 2». Витрина оборачивает пару в `<div class="radio-group">` без `role="radiogroup"`/`<fieldset>`, закрепляя неправильный образец.

**Fix:** обязательный проп `name: string` с пробросом в `<input type="radio" {name}>`; в витрине — `role="radiogroup" aria-label="…"` на обёртке.

### WR-11: admin-гейт `/showcase` косметический (перенесено, не исправлено)

**Severity:** WARNING
**File:** `ui/src/routes.ts:28`, `ui/src/features/layout/sidebar-config.ts:31`
**Issue:** `roles: ['admin']` влияет только на отрисовку пункта меню (`Sidebar.svelte:10` → `getVisibleItems`). Карта `routes` одна для admin и manager, поэтому `#/showcase` открывается вручную любым не-employee пользователем (в отличие от `employeeRoutes`, где неизвестный путь честно ведёт на `AccessDenied`). Данных на витрине нет, но заявленное ограничение не выполняется, и тот же паттерн уже действует для `/users` и `/settings`. Дополнительно витрина безусловно входит в production-бандл, а в desktop-режиме с `desktop_lock_enabled=false` роль admin выдаётся всем.

**Fix:** `wrap({ asyncComponent: …, conditions: [() => authStore.user?.role === 'admin'] })` из `svelte-spa-router/wrap` — заодно вынесет витрину из основного чанка; опционально гейт по `import.meta.env.DEV`.

### WR-12: PINNED-комментарий в sidebar-config противоречит коду (перенесено, не исправлено)

**Severity:** WARNING
**File:** `ui/src/features/layout/sidebar-config.ts:14-15, 31`
**Issue:** Комментарий помечен «source of truth» и утверждает `11 items + 4 dividers = 15 entries`, тогда как в массиве 12 items + 4 dividers = 16. После четырёх gap-closure планов расхождение по-прежнему на месте. Пометка PINNED, которая врёт, обесценивает саму пометку. Плюс `/showcase` — единственный item без поля `phase`.

**Fix:** обновить счётчик на `12 items + 4 dividers = 16 entries` с явной оговоркой, что витрина — служебный пункт вне UI-SPEC §Copywriting; добавить `phase: 24`.

### WR-13: id заголовка модалки через `Math.random()` (перенесено, не исправлено)

**Severity:** WARNING
**File:** `ui/src/lib/components/Modal.svelte:15`
**Issue:** Нестабильный id — шум в снапшот-тестах, теоретические коллизии, разное значение при каждом создании. В Svelte 5 для этого есть штатный `$props.id()`, детерминированный и SSR-безопасный.
**Fix:** `const titleId = $props.id();`

### WR-14: блокировка скролла body через `<style>` внутри `<svelte:head>` (перенесено, не исправлено)

**Severity:** WARNING
**File:** `ui/src/lib/components/Modal.svelte:120-128`
**Issue:** Инъекция глобального `body { overflow: hidden }` стилевым тегом: приоритет относительно других глобальных правил неуправляем, ширина скроллбара не компенсируется (контент дёргается при открытии/закрытии), при цепочке/вложенности модалок в `<head>` оказываются дубликаты, а исходное значение `overflow` не восстанавливается — восстанавливается лишь удаление тега.
**Fix:** `$effect` с сохранением и возвратом `document.body.style.overflow`, либо счётчик открытых модалок в отдельном сторе.

### WR-15: ни один из четырёх фиксов не защищён регрессионным гейтом

**Severity:** WARNING
**File:** `ui/package.json:10-17`, `ui/scripts/check-tokens.mjs`
**Issue:** 24-08…24-10 закрыли четыре дефекта, каждый из которых был невидим глазом (`bind:value` не пробрасывался, `:global()` уезжал в бандл, count-бейджи молча серели, фокус не переводился). В `ui/` нет тест-раннера вообще (`scripts` содержит только dev/build/check/lint, файлов `*.test.ts` нет), а предложенный в прошлом раунде grep-гейт на `:global(` в `ui/src/styles/` в `check-tokens.mjs` не добавлен. Приёмка фазы (24-11) свелась к устной формулировке «Витрину проверил — всё хорошо работает», что по природе этих багов ничего не подтверждает: три из четырёх выглядят исправно и будучи сломанными.

**Fix:** минимум — расширить `check-tokens.mjs` двумя grep-правилами (`:global(` в `ui/src/styles/**`, `const {` рядом с `$bindable(` в `ui/src/lib/components/**`); полноценно — добавить vitest + `@testing-library/svelte` и по одному тесту на `bind:value`, focus-trap и Escape-однократность.

### WR-16: поля витрины не имеют программных меток (перенесено, не исправлено)

**Severity:** WARNING
**File:** `ui/src/features/showcase/sections/FieldsSection.svelte:35-46, 53-72, 79-90`
**Issue:** `<span class="state-tag">` — визуальная подпись, не связанная с полем. Ни одно поле витрины не имеет `<label for>` или `aria-label`, хотя `Input`/`Select`/`Textarea` принимают `id`. Витрина задаёт образец для фаз 25-30, и образец — без меток.
**Fix:** заменить `<span class="state-tag">` на `<label class="state-tag" for="fld-input-normal">` и передавать соответствующий `id` в компонент.

### WR-17: остаточные мелочи, перенесённые без изменений

**Severity:** WARNING
**Issue:**
- `ui/src/lib/components/Badge.svelte:21` — `TONE_MAP[variant]` без фолбэка: значение вне union (вызов из нетипизированного места, данные с бэкенда) даёт класс `badge-m-undefined` и бейдж без фона, молча. Фикс: `$derived(TONE_MAP[variant] ?? 'neutral')`.
- `ui/src/styles/global.scss:71-93` — `.skip-link` определён глобально и независимо продублирован в scoped-стилях `Layout.svelte:44` и `EmployeeLayout.svelte:172`; scoped-версии выигрывают по хэшу класса, глобальный блок — мёртвый код, который придётся править «на всякий случай».
- `ui/src/lib/components/Input.svelte:2-11` — нет проброса `name`, `required`, `maxlength`, `autocomplete`, `readonly`, `onblur`; набор пропов уже вынуждает часть экранов (`OrgSettings.svelte`, `LoginPage.svelte`) обходить примитив и использовать сырой `<input>`. Фикс: `interface Props extends HTMLInputAttributes` + `{...rest}`.

---

_Reviewed: 2026-07-18_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
