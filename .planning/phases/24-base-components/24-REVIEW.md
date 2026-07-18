---
phase: 24-base-components
reviewed: 2026-07-18T00:00:00Z
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
  critical: 3
  warning: 8
  info: 6
  total: 17
status: issues_found
---

# Phase 24: Code Review Report

**Reviewed:** 2026-07-18
**Depth:** standard
**Files Reviewed:** 21
**Status:** issues_found

## Summary

Просмотрены примитивы дизайн-системы (Button, Badge, Input, Select, Textarea, Checkbox, Radio, Modal, Tabs), витрина компонентов и её подключение к роутингу/сайдбару. `svelte-check` проходит без ошибок, токены (`--tr-*`, `--modal-max-width*`) все определены в `_tokens.scss` — визуальный слой в целом согласован.

Тем не менее найдены три дефекта уровня BLOCKER, два из которых воспроизводимы прямо сейчас:

1. **`bind:value` у Input/Select/Textarea не работает в обратную сторону** — `$bindable` объявлен, но props деструктурированы через `const` и компонент никогда не присваивает `value`. Витрина уже использует эти биндинги (FieldsSection, ModalSection), состояние родителя молча не обновляется. Это контракт API, на который будут опираться фазы 25–30.
2. **`:global()` в обычном `.scss`-файле** — `global.scss` обрабатывается sass, а не компилятором Svelte, поэтому `:global(...)` попадает в собранный CSS дословно (подтверждено в `ui/dist/assets/index-*.css`). Селектор невалиден → подавление transition при переключении темы (D-09) не работает вообще, класс `theme-switching` навешивается вхолостую.
3. **Modal не управляет фокусом** — нет начальной установки фокуса, нет focus trap, нет возврата фокуса на триггер. Для `role="dialog" aria-modal="true"` это нарушение контракта ARIA.

Отдельно: admin-гейт `/showcase` заявлен, но фактически косметический — карта `routes` общая для admin и manager, а роль проверяется только при отрисовке пункта сайдбара.

## Narrative Findings (AI reviewer)

## Critical Issues

### CR-01: `bind:value` у Input/Select/Textarea односторонний — состояние родителя не обновляется

**File:** `ui/src/lib/components/Input.svelte:13-22`, `ui/src/lib/components/Select.svelte:18-25`, `ui/src/lib/components/Textarea.svelte:12-20`
**Issue:** Во всех трёх компонентах props объявлены как `const { value = $bindable(''), ... } = $props()`. `$bindable` синхронизирует значение в родителя **только когда дочерний компонент присваивает проп**. Здесь присваивания нет (и не может быть — `const`), а обработчик `oninput`/`onchange` лишь вызывает колбэк. Итог: `<Input bind:value={x} />` никогда не обновит `x`.

Дефект уже проявляется в коде фазы: `FieldsSection.svelte:37,41,45,55,62,69,81,85,89` и `ModalSection.svelte:20` используют `bind:value`. В `ModalSection` `demoValue` навсегда останется `'Пример значения'`, что бы ни ввёл пользователь. Визуально это незаметно (DOM-значение меняет сам браузер), поэтому баг переживёт ручную проверку и уедет в фазы 25–30, где на этих примитивах будут строиться формы (акты, заявки) — там расхождение обернётся отправкой на бэкенд устаревших данных.

Сравните с `Checkbox.svelte:13` и `Radio.svelte:13`, где корректно использованы `let` + `bind:checked`/`bind:group`.

**Fix:**
```svelte
<!-- Input.svelte -->
let {
  type = 'text',
  value = $bindable(''),
  /* ... */
  oninput,
}: Props = $props();
</script>

<input
  {type}
  bind:value
  oninput={(e) => oninput?.((e.currentTarget as HTMLInputElement).value)}
  ...
/>
```
Аналогично: `Textarea` — `bind:value` на `<textarea>`; `Select` — `bind:value` на `<select>` плюс `let` вместо `const`. После правки прогнать витрину и убедиться, что состояние в `FieldsSection` действительно меняется (добавить временный вывод значения рядом с полем — это к тому же полезно как демонстрация).

---

### CR-02: `:global()` в обычном SCSS-файле → подавление transition при смене темы (D-09) не работает

**File:** `ui/src/styles/global.scss:64-67`
**Issue:** `global.scss` импортируется напрямую из `main.ts` и обрабатывается только sass + Vite. `:global()` — синтаксис компилятора Svelte для scoped-стилей компонентов; в обычном `.scss` он ничем не удаляется и попадает в бандл как есть. Проверено на собранном CSS:

```
$ grep -o ":global([^)]*)" ui/dist/assets/index-CLsIRsCf.css
:global(.theme-switching)
:global(.theme-switching)
```

Браузер не знает псевдокласс `:global`, поэтому весь список селекторов признаётся невалидным и правило `transition: none !important` отбрасывается целиком. Следовательно `applyResolved()` в `theme.svelte.ts:34-38` добавляет и снимает класс `theme-switching` без какого-либо эффекта, а декларированное решение D-09 (отсутствие «размазывания» цветов при переключении light/dark) не выполнено.

**Fix:**
```scss
// global.scss — файл уже глобальный, обёртка :global() не нужна и вредна
.theme-switching,
.theme-switching * {
  transition: none !important;
}
```
Чтобы такой класс ошибок не повторялся, стоит добавить в CI grep-гейт: `grep -R ":global(" ui/src/styles/ && exit 1`.

---

### CR-03: Modal не управляет фокусом (нет initial focus, focus trap и возврата фокуса)

**File:** `ui/src/lib/components/Modal.svelte:41-66`
**Issue:** Контейнер помечен `role="dialog" aria-modal="true"`, но:
- при открытии фокус остаётся на кнопке-триггере **за** бэкдропом;
- `Tab`/`Shift+Tab` свободно уводят фокус в фоновый контент (сайдбар, таблицы) — пользователь клавиатуры и скринридера «проваливается» из модалки, при этом фон помечен как inert для AT только декларативно (`aria-modal`), но реально не заблокирован;
- при закрытии фокус не возвращается на элемент, открывший модалку, — он теряется на `<body>`, и следующий `Tab` начинает обход с начала страницы.

`tabindex="-1"` на бэкдропе (строка 50) сам по себе фокус не устанавливает — он лишь делает элемент программно фокусируемым. Для `aria-modal="true"` это нарушение WAI-ARIA Dialog Pattern, а не стилистическое замечание: модалка используется во всех формах приложения (акты, картриджи, настройки).

**Fix:**
```svelte
<script lang="ts">
  let dialogEl = $state<HTMLElement | null>(null);
  let prevFocus: HTMLElement | null = null;

  $effect(() => {
    if (!open) return;
    prevFocus = document.activeElement as HTMLElement | null;
    // фокус на первый интерактивный элемент, иначе на сам контейнер
    const first = dialogEl?.querySelector<HTMLElement>(
      'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
    );
    (first ?? dialogEl)?.focus();
    return () => prevFocus?.focus();
  });

  function trapTab(e: KeyboardEvent) {
    if (e.key !== 'Tab' || !dialogEl) return;
    const nodes = [...dialogEl.querySelectorAll<HTMLElement>(
      'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])'
    )].filter((n) => n.offsetParent !== null);
    if (nodes.length === 0) return;
    const first = nodes[0];
    const last = nodes[nodes.length - 1];
    if (e.shiftKey && document.activeElement === first) { e.preventDefault(); last.focus(); }
    else if (!e.shiftKey && document.activeElement === last) { e.preventDefault(); first.focus(); }
  }
</script>

<div class="modal-container modal-{size}" bind:this={dialogEl} onkeydown={trapTab} tabindex="-1">
```

---

## Warnings

### WR-01: Escape закрывает модалку двойным вызовом `onClose`

**File:** `ui/src/lib/components/Modal.svelte:39, 46`
**Issue:** `handleKeydown` навешан одновременно на `<svelte:window onkeydown=...>` и на сам бэкдроп (`onkeydown={handleKeydown}`). Когда фокус внутри модалки, событие сначала обрабатывается на div, затем всплывает до `window` — `onClose()` вызывается дважды. Для витрины это безобидно (`open = false` дважды), но потребители передают в `onClose` не только сброс флага: закрытие с откатом черновика, снятие блокировки, pop из стека модалок, отправка аналитики — всё это отработает дважды.

**Fix:** Оставить один источник. Проще всего убрать `onkeydown={handleKeydown}` со строки 46 (обработчик на `window` уже покрывает случай, когда фокус вне модалки), либо оставить только локальный обработчик после реализации focus trap из CR-03 и добавить `e.stopPropagation()`.

---

### WR-02: Tabs объявляет `role="tablist"`/`role="tab"`, но не реализует клавиатурный паттерн

**File:** `ui/src/lib/components/Tabs.svelte:26-54`
**Issue:** Для варианта `underline` выставлены `role="tablist"` и `role="tab"`, `aria-selected`, но отсутствуют обязательные части паттерна WAI-ARIA Tabs:
- нет навигации стрелками ←/→ (+ Home/End);
- нет roving `tabindex` — все вкладки попадают в tab-order, тогда как спецификация требует `tabindex="0"` только у активной и `-1` у остальных;
- нет `aria-controls` и связанного `role="tabpanel"` — скринридер объявляет «вкладка», но не может перейти к панели.

То есть заявленная семантика вкладок скринридеру обещана, а поведение — как у обычных кнопок; это хуже, чем `role="group"` (вариант `segmented` как раз сделан честно).

Дополнительно: `disabled` на `role="tab"` полностью убирает элемент из восприятия AT — по паттерну корректнее `aria-disabled="true"` с сохранением фокусируемости.

**Fix:**
```svelte
<button
  role={variant === 'segmented' ? undefined : 'tab'}
  tabindex={variant === 'segmented' ? undefined : (tab.key === active ? 0 : -1)}
  aria-disabled={tab.disabled ? 'true' : undefined}
  onkeydown={variant === 'segmented' ? undefined : onTabKeydown}
  ...
>
```
плюс функция `onTabKeydown`, переносящая `active` на следующую/предыдущую недизейбленную вкладку по ArrowLeft/ArrowRight/Home/End с вызовом `.focus()`. Либо — если панелей нет — понизить семантику до `role="group"` + `aria-pressed`, как в `segmented`.

---

### WR-03: `appearance="count"` реализован только для двух тонов из пяти

**File:** `ui/src/lib/components/Badge.svelte:177-196`
**Issue:** Стили для count-бейджа заданы у `.badge-m-count` (нейтральный) и `.badge-m-accent.badge-m-count`. Тонов же пять. `<Badge variant="success" appearance="count">`, `warning` и `destructive` получают классы `badge-m-success badge-m-count`, но правило `.badge-m-count` перекрывает фон/цвет нейтральным — три из пяти вариантов молча рендерятся серыми.

Это видно прямо в витрине: `BadgeSection.svelte:34, 44, 54` показывают success/warning/destructive count как неотличимые от default. Витрина существует ровно для того, чтобы такие дыры ловить — здесь она их демонстрирует, но как «норму».

**Fix:** Добавить count-правила в каждый тон-блок (по образцу `soft`/`solid`), например:
```scss
.badge-m-success {
  &.badge-m-count { background: var(--tr-success-soft); color: var(--tr-success-text); }
}
```
Либо, если по UI-SPEC count намеренно только neutral/accent, — ограничить типы (`appearance?: 'soft' | 'solid' | 'dot'` + отдельный `count`-контракт) и убрать несуществующие комбинации из витрины.

---

### WR-04: admin-гейт `/showcase` косметический — страница доступна manager'у по прямому хэшу

**File:** `ui/src/routes.ts:28`, `ui/src/features/layout/sidebar-config.ts:31`
**Issue:** `roles: ['admin']` в `sidebar-config.ts` влияет только на отрисовку пункта меню (`Sidebar.svelte:10` → `getVisibleItems`). Маршрутная карта `routes` (`App.svelte:67`) одна и та же для ролей `admin` и `manager`, поэтому пользователь с ролью manager, набрав `#/showcase`, получит страницу. Гейта уровня маршрута нет (в отличие от `employeeRoutes`, где неизвестные пути честно ведут на `AccessDenied`).

Данных на витрине нет, поэтому это не утечка, но заявленное в фазе ограничение не выполняется, и тот же паттерн уже действует для `/users` и `/settings` — там цена ошибки выше (страницы рендерят реальные данные и полагаются исключительно на серверную авторизацию).

Второй аспект: витрина безусловно входит в production-бандл (5 секций + примитивы), и в desktop-режиме с `desktop_lock_enabled=false` любой пользователь получает роль `admin` (`App.svelte:38-43`) — то есть на портативной сборке витрина видна всем.

**Fix:** Ввести явную проверку роли на уровне маршрута, например обёртку:
```ts
import { wrap } from 'svelte-spa-router/wrap';

'/showcase': wrap({
  asyncComponent: () => import('./pages/ComponentShowcasePage.svelte'),
  conditions: [() => authStore.user?.role === 'admin'],
}),
```
(`asyncComponent` заодно уберёт витрину из основного чанка). Условие `conditions` при провале уводит на `*` → `NotFound`. Если витрина мыслится как dev-инструмент — дополнительно гейтить через `import.meta.env.DEV`.

---

### WR-05: проп `invalid` у Select/Checkbox/Radio не транслируется в `aria-invalid`

**File:** `ui/src/lib/components/Select.svelte:29-39`, `ui/src/lib/components/Checkbox.svelte:25-33`, `ui/src/lib/components/Radio.svelte:25`
**Issue:** `Input.svelte:34` и `Textarea.svelte:31` выставляют `aria-invalid={invalid || undefined}`, а `Select`, `Checkbox` и `Radio` — нет: у них `invalid` меняет только рамку/тень. Для скринридера три из пяти полей в состоянии ошибки неотличимы от валидных, а красная рамка — единственный носитель информации (нарушение WCAG 1.4.1 «Use of Color»). Несогласованность внутри одного набора примитивов гарантирует, что в формах фаз 25+ часть полей будет доступной, часть — нет.

**Fix:** Добавить `aria-invalid={invalid || undefined}` на `<select>`, `<input type="checkbox">` и `<input type="radio">`. Заодно предусмотреть проп `aria-describedby` (как в `Input`) для связи с текстом ошибки — сейчас его нет ни у одного компонента, кроме `Input`.

---

### WR-06: theme store — невалидированное значение из localStorage, незащищённый доступ и повторные подписки

**File:** `ui/src/lib/stores/theme.svelte.ts:15-22, 30-39`
**Issue:** Три проблемы в одном месте:
1. `const saved = (localStorage.getItem(KEY) ?? 'system') as Preference` — приведение типа без проверки. Любое постороннее значение в ключе `trackly:theme` (ручная правка, коллизия с другим приложением на том же origin, старый формат) попадёт в `themeStore.preference`, а затем в `document.documentElement.dataset.theme` (строка 35). Селекторов `[data-theme='light'|'dark']` в `_tokens.scss` не будет ни одного → все `--tr-*` переменные не разрезолвятся, приложение отрендерится без цветов. `applyResolved()` дополнительно возвращает это значение как `Resolved`, хотя тип обещает только `'light' | 'dark'`.
2. `localStorage` вызывается без защиты, тогда как `window` в этом же файле проверяется (строка 12-13). В браузере с заблокированным хранилищем (сторонние cookies/приватный режим/политика домена) `getItem` бросает `SecurityError`; `main.ts:7` вызывает `initTheme()` на верхнем уровне без `try/catch` — приложение не смонтируется вообще (белый экран). Для режима LAN-браузера это реальный сценарий.
3. `initTheme()` каждый вызов добавляет новый `mql` listener, ничего не снимая. Сейчас вызов один, но функция экспортирована и никак от повторного вызова не защищена.

**Fix:**
```ts
const PREFS = ['light', 'dark', 'system'] as const;
function readPreference(): Preference {
  try {
    const raw = localStorage.getItem(KEY);
    return (PREFS as readonly string[]).includes(raw ?? '') ? (raw as Preference) : 'system';
  } catch {
    return 'system';
  }
}

let initialized = false;
export function initTheme(): void {
  if (initialized) return;
  initialized = true;
  themeStore.preference = readPreference();
  applyResolved();
  mql?.addEventListener('change', () => { if (themeStore.preference === 'system') applyResolved(); });
}
```
и обернуть `localStorage.setItem` в `setTheme` в `try/catch` — падение записи не должно ломать переключение темы.

---

### WR-07: Radio без `name` и без семантики группы

**File:** `ui/src/lib/components/Radio.svelte:4-25`, `ui/src/features/showcase/sections/FieldsSection.svelte:117-120`
**Issue:** У `Radio` нет пропа `name`. `bind:group` в Svelte связывает радиокнопки на уровне JS, поэтому визуально всё работает, но:
- нативная группировка браузера отсутствует → навигация стрелками между радиокнопками одной группы (стандартное поведение) не работает, каждая кнопка отдельная точка табуляции;
- при использовании внутри `<form>` значения не сериализуются;
- скринридер не объявляет «1 из 2».

В витрине группа собрана в `<div class="radio-group">` без `role="radiogroup"`/`<fieldset><legend>`, что закрепляет неправильный паттерн использования как эталонный.

**Fix:** Добавить обязательный проп `name: string` и пробросить его в `<input type="radio" {name} ...>`. В `FieldsSection` обернуть группу:
```svelte
<div class="radio-group" role="radiogroup" aria-label="Вариант">
  <Radio name="showcase-normal" bind:group={radioGroupNormal} value="a">Вариант A</Radio>
  <Radio name="showcase-normal" bind:group={radioGroupNormal} value="b">Вариант B</Radio>
</div>
```

---

### WR-08: закреплённый комментарий-«источник истины» в sidebar-config противоречит коду

**File:** `ui/src/features/layout/sidebar-config.ts:14-15`
**Issue:** Комментарий гласит `PINNED: 11 items + 4 dividers = 15 entries — source of truth per UI-SPEC §Copywriting Sidebar`, но после добавления `/showcase` (строка 31) в массиве 12 items + 4 dividers = 16 записей. Комментарий помечен как PINNED и как источник истины — теперь он лжёт, и следующий разработчик либо удалит пункт витрины «чтобы сошлось», либо перестанет доверять пометке PINNED вообще. Кроме того, `/showcase` — единственный пункт без поля `phase`, что ломает единообразие структуры.

**Fix:** Обновить комментарий (`12 items + 4 dividers = 16 entries`) с явной пометкой, что витрина — служебный пункт вне UI-SPEC §Copywriting, и добавить `phase: 24` в запись строки 31.

---

## Info

### IN-01: id заголовка модалки генерируется через `Math.random()`

**File:** `ui/src/lib/components/Modal.svelte:15`
**Issue:** `Math.random().toString(36).slice(2)` даёт нестабильный id (шум в снапшот-тестах, теоретические коллизии, разное значение при повторном создании). В Svelte 5 для этого есть штатный `$props.id()`.
**Fix:** `const titleId = $props.id();`

### IN-02: блокировка скролла body через `<style>` внутри `<svelte:head>`

**File:** `ui/src/lib/components/Modal.svelte:69-77`
**Issue:** Инъекция глобального правила `body { overflow: hidden }` стилевым тегом — хрупкий приём: правило нельзя приоритизировать относительно других глобальных стилей, не компенсируется ширина скроллбара (контент дёргается при открытии), а при вложенных модалках в `<head>` попадают дубликаты.
**Fix:** Перевести на `$effect` с прямым управлением `document.body.style.overflow` и восстановлением прежнего значения в cleanup; либо на счётчик открытых модалок в отдельном сторе.

### IN-03: поля витрины не имеют программных меток

**File:** `ui/src/features/showcase/sections/FieldsSection.svelte:35-46, 53-72, 79-90`
**Issue:** `<span class="state-tag">Обычное</span>` — визуальная подпись, не связанная с полем. Ни одно поле в витрине не имеет `<label for>` или `aria-label`, хотя `Input`/`Select`/`Textarea` принимают `id`. Витрина задаёт образец использования для последующих фаз, и образец — без меток.
**Fix:** Заменить `<span class="state-tag">` на `<label class="state-tag" for="fld-input-normal">` и передавать соответствующий `id` в компонент.

### IN-04: `.skip-link` в global.scss дублирует стили обоих Layout-компонентов

**File:** `ui/src/styles/global.scss:71-93`
**Issue:** Правила `.skip-link` определены глобально и, независимо, в scoped-стилях `Layout.svelte:44` и `EmployeeLayout.svelte:172`. Scoped-версии выигрывают по специфичности класса-хэша, поэтому глобальный блок — фактически мёртвый код, который придётся править «на всякий случай» при каждом изменении skip-link.
**Fix:** Оставить одно определение — глобальное (и убрать из обоих layout'ов) либо scoped (и убрать из global.scss).

### IN-05: `Input` объявляет `type="number"`, но типизирует значение как `string`, и не пробрасывает атрибуты формы

**File:** `ui/src/lib/components/Input.svelte:2-11`
**Issue:** `type?: 'text' | 'number' | 'search'` при `value: string` — числовой ввод придётся парсить у каждого потребителя, из-за чего часть форм неизбежно будет обходить примитив и использовать сырой `<input>` (как это уже сделано в `OrgSettings.svelte`, `LoginPage.svelte`). Также нет проброса `name`, `required`, `maxlength`, `autocomplete`, `readonly`, `onblur` — набор пропов слишком узкий для форм фаз 25+.
**Fix:** Добавить `...rest` через `interface Props extends HTMLInputAttributes` и `{...rest}` на элементе, либо явно перечислить нужные атрибуты; для числового ввода предусмотреть отдельный проп/компонент.

### IN-06: неохраняемый доступ к `TONE_MAP` в Badge

**File:** `ui/src/lib/components/Badge.svelte:13-21`
**Issue:** `TONE_MAP[variant]` при значении вне union (вызов из нетипизированного места, данные с бэкенда) вернёт `undefined` → класс `badge-m-undefined`, бейдж без фона и без ошибки. Тихая деградация вместо явного сбоя.
**Fix:** `const tone = $derived(TONE_MAP[variant] ?? 'neutral');`

---

_Reviewed: 2026-07-18_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
