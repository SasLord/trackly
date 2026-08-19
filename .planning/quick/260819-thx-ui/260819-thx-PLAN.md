---
quick_id: 260819-thx
slug: ui
phase: 260819-thx
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - ui/src/features/cartridges/CartridgeFormBody.svelte
  - ui/src/features/cartridges/ModelFormModal.svelte
  - ui/src/features/cartridges/ModelListRow.svelte
  - ui/src/features/cartridges/CompatibilityEditor.svelte
autonomous: true
requirements: [THX-01, THX-02, THX-03]
must_haves:
  truths:
    - "Дропдаун типа расходника (Картридж/Фотобарабан) в попапах «Новый картридж/фотобарабан» (CartridgeFormBody.svelte) и «Новая модель картриджа» (ModelFormModal.svelte) больше НЕ показывает строку поиска над списком из двух пунктов; остальные Dropdown-инстансы (Модель, Цвет, Состояние и т.д.), где поиск нужен и полезен, продолжают показывать поле поиска как раньше — прочие потребители Dropdown.svelte не затронуты"
    - "На вкладке «Модели» страницы «Картриджи» первая колонка снова показывает название модели (бренд+модель) НАД бейджами типа/цвета — не только бейджи; колонка «Экземпляров» читаема и выровнена по паттерну остальных таблиц; колонки не наезжают друг на друга"
    - "В попапе «Новая модель картриджа» список подсказок «Совместимые принтеры» раскрывается ПОВЕРХ модалки через portal в body с fixed-позиционированием, привязанным к полю ввода строки, — а не внутри прокручиваемого контента попапа"
  artifacts:
    - path: "ui/src/features/cartridges/CartridgeFormBody.svelte"
      provides: "Dropdown «Что добавляем» (KIND_OPTIONS, 2 пункта) с searchable={false}"
      contains: "searchable={false}"
    - path: "ui/src/features/cartridges/ModelFormModal.svelte"
      provides: "Dropdown «Тип расходника» (KIND_OPTIONS, 2 пункта) с searchable={false}"
      contains: "searchable={false}"
    - path: "ui/src/features/cartridges/ModelListRow.svelte"
      provides: "Первая колонка (.cell-name) — плоская table-cell, flex-column layout вынесен на вложенный .cell-name-inner (FIX B3 pattern, уже применён в CartridgeListRow.svelte/.cell-code-inner и PrinterListRow.svelte/.cell-name-inner)"
      contains: "cell-name-inner"
    - path: "ui/src/features/cartridges/CompatibilityEditor.svelte"
      provides: "Панель подсказок «Совместимые принтеры» через use:portal + use:dropdownAnchor, namespaced класс .dropdown--compat (WR-03 конвенция)"
      contains: "dropdown--compat"
  key_links:
    - from: "ui/src/features/cartridges/CartridgeFormBody.svelte / ModelFormModal.svelte (Dropdown «Что добавляем» / «Тип расходника»)"
      to: "ui/src/lib/components/Dropdown.svelte searchable prop"
      via: "searchable={false} на flat+select Dropdown с KIND_OPTIONS"
      pattern: "searchable={false}"
    - from: "ui/src/features/cartridges/ModelListRow.svelte td.cell-name"
      to: "ui/src/lib/components/TableRow.svelte base td rule"
      via: "td больше не несёт display:flex — вложенный span.cell-name-inner держит flex-column, td остаётся display:table-cell"
      pattern: "cell-name-inner"
    - from: "ui/src/features/cartridges/CompatibilityEditor.svelte панель подсказок"
      to: "ui/src/lib/utils/portal.ts + ui/src/lib/utils/dropdownAnchor.ts"
      via: "use:portal use:dropdownAnchor вместо position:absolute внутри .autocomplete-wrapper"
      pattern: "use:dropdownAnchor"
---

<objective>
Три точечных UI-фикса в разделе «Картриджи» (Svelte 5, ui/), не затрагивающие бэкенд/БД:

1. Дропдаун выбора типа расходника (Картридж/Фотобарабан) в попапах «Новый картридж/фотобарабан»
   и «Новая модель картриджа» показывает поле «Поиск» над списком из двух пунктов — поиск для
   такого короткого списка избыточен. Dropdown.svelte уже имеет проп searchable (default true)
   именно для этого случая (его собственный doc-комментарий: "Set false for short, fully-visible
   option lists ... where a search field is noise rather than help") и уже используется как
   searchable={false} в CartridgeFilters.svelte, PeriodSelector.svelte, GroupedPrinterSelect.svelte.
   Решение — применить существующий проп к двум конкретным инстансам, не трогая компонент и не
   трогая другие дропдауны.

2. Таблица «Модели картриджей» — первая колонка пустая (видны только бейджи, названия модели
   нет), «Экземпляров» нечитаемо, колонки разъехались. Корневая причина: td.cell-name в
   ModelListRow.svelte несёт display:flex НАПРЯМУЮ на самом td, что переопределяет
   display:table-cell и ломает модель колонок всей таблицы — задокументированный баг "FIX B3",
   уже найденный и исправленный в этой же директории (CartridgeListRow.svelte,
   .cell-code/.cell-code-inner) и в PrinterListRow.svelte (.cell-name/.cell-name-inner, точный
   аналог: имя сверху, вторая строка снизу). Решение — вынести flex-layout с td на вложенный
   span.cell-name-inner, как уже сделано в обоих прецедентах.

3. Автокомплит «Совместимые принтеры» в попапе «Новая модель картриджа»
   (CompatibilityEditor.svelte) раскрывается position:absolute внутри контента модалки — отсюда
   внутренний скролл и неудобный выбор. Тот же класс проблемы уже решён для
   PersonAutocomplete.svelte, DeviceAutocompleteField.svelte, LocationAutocomplete.svelte через
   use:portal (перенос узла в body) + use:dropdownAnchor (fixed-позиционирование с привязкой к
   полю ввода, автоматический флип вверх/вниз — "актуально для дропдаунов внутри модалок" из
   doc-комментария самого dropdownAnchor.ts). Решение — переиспользовать этот механизм для
   CompatibilityEditor.svelte, не изобретая новый.

Purpose: убрать три конкретных визуальных дефекта в разделе «Картриджи», переиспользуя уже
существующие в кодовой базе решения (проп/паттерн/utility), а не изобретая новые.

Output: searchable={false} на двух Dropdown-инстансах; .cell-name/.cell-name-inner split в
ModelListRow.svelte; портированная (portal+dropdownAnchor) панель подсказок в
CompatibilityEditor.svelte.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@CLAUDE.md
@.planning/STATE.md

<interfaces>
Dropdown.svelte — searchable prop contract (already implemented, ui/src/lib/components/Dropdown.svelte).
Read-only reference — this plan does NOT modify Dropdown.svelte.

  interface Props {
    variant: 'combobox' | 'select';
    flat?: boolean;
    // select-variant only: show the in-panel search box (default true).
    // Set false for short, fully-visible option lists (e.g. month/year
    // pickers) where a search field is noise rather than help.
    searchable?: boolean;
  }

Existing precedent usages (grep confirmed): CartridgeFilters.svelte, PeriodSelector.svelte
(x3), GroupedPrinterSelect.svelte all already pass searchable={false}.

CartridgeFormBody.svelte — the "Что добавляем" Dropdown to fix (search the file for
KIND_OPTIONS — around line 76-79 for the const, around line 202-221 for the Dropdown block).
Re-read exact current line numbers before editing:

  const KIND_OPTIONS = [
    { id: 1, label: 'Картридж' },
    { id: 2, label: 'Фотобарабан' },
  ];
  ...
  <Dropdown
    variant="select"
    flat={true}
    value={kindLabel}
    placeholder="Выберите вид"
    searchPlaceholder="Поиск"
    loading={false}
    groups={KIND_OPTIONS}
    ...
    onPickGroup={(o) => handleKindChange(String(o.id))}
    onPickMember={() => {}}
  />

ModelFormModal.svelte — the "Тип расходника" Dropdown to fix (search the file for
KIND_OPTIONS — around line 32-35 for the const, around line 333-354 for the Dropdown block).
Re-read exact current line numbers before editing. Note: this same file ALSO has a "Цвет"
Dropdown (COLOR_DROPDOWN_OPTIONS, 6 options) around line 447-466 — DO NOT touch it, out of
scope for this plan:

  const KIND_OPTIONS = [
    { id: 1, label: 'Картридж' },
    { id: 2, label: 'Фотобарабан' },
  ];
  ...
  <Dropdown
    variant="select"
    flat={true}
    value={kindLabel}
    placeholder="Выберите тип"
    searchPlaceholder="Поиск"
    loading={false}
    groups={KIND_OPTIONS}
    ...
    onPickGroup={(o) => { kindId = Number(o.id); }}
    onPickMember={() => {}}
  />

ModelListRow.svelte — CURRENT (buggy) markup+CSS, ui/src/features/cartridges/ModelListRow.svelte.
Re-read the file before editing — line numbers below are as of planning time.

  markup, ~line 54-64:
  <td class="cell cell-name" title="{model.brand} {model.model}">
    <span class="name">{model.brand} {model.model}</span>
    <span class="badges">
      <Badge variant={model.kind_id === 1 ? 'accent' : 'default'} size="sm">
        {model.kind_id === 1 ? 'Картридж' : 'Фотобарабан'}
      </Badge>
      {#if model.kind_id === 1 && model.color}
        <Badge variant="default" size="sm">{model.color}</Badge>
      {/if}
    </span>
  </td>

  CSS, ~line 109-133 — THE BUG: display:flex directly on the .cell-name td:
  .cell-name {
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 2px;
    max-width: 0; // makes text-overflow work in table cells
  }
  .name {
    font-size: var(--tr-font-size-body);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }
  .badges {
    display: flex;
    align-items: center;
    gap: var(--tr-space-2xs);
    flex-shrink: 0;
  }

PrinterListRow.svelte — the ALREADY-FIXED precedent for this exact class of bug, in the
"Принтеры" table the task description references. READ-ONLY, do not edit — the shape below is
what ModelListRow.svelte's fix must mirror (td stays plain, inner span owns the flex):

  <td class="cell cell-name" ...>
    <span class="cell-name-inner">
      ...
      <span class="name-lines">
        <span class="name-text">{displayName}</span>
        ...
      </span>
    </span>
  </td>

  // FIX B3: display:flex on the <td> ITSELF overrides display:table-cell,
  // pulling the cell out of the table's column model — every column collapses/
  // overlaps. The <td> stays a normal table cell (ellipsis/shrink + cursor +
  // focus ring only); the flex layout lives on the inner span below.
  .cell-name {
    overflow: hidden;
    max-width: 0; // makes text-overflow work in table cells
  }
  .cell-name-inner {
    display: flex;
    align-items: center;
    gap: var(--tr-space-2xs);
  }
  .name-lines {
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 2px;
    min-width: 0;
  }

CompatibilityEditor.svelte — CURRENT (buggy) autocomplete block,
ui/src/features/cartridges/CompatibilityEditor.svelte. Re-read the file before editing.

  markup, ~line 153-195, inside {#each rows as row, i (i)}:
  <div class="autocomplete-wrapper">
    <input
      id="compat-name-{i}"
      ...
      oninput={(e) => handleInput(i, (e.currentTarget as HTMLInputElement).value)}
      onfocus={() => handleFocus(i)}
      onkeydown={(e) => handleKeydown(e, i)}
    />
    {#if openKey === getKey(i)}
      <div class="dropdown" role="listbox">
        ...suggestions...
      </div>
    {/if}
  </div>

  handleClickOutside, ~line 131-139 — relies on DOM containment via .closest('.compat-field');
  once the dropdown is portaled to body this no longer finds clicks INSIDE the dropdown:
  function handleClickOutside(e: MouseEvent) {
    if (!openKey) return;
    const el = e.target as HTMLElement | null;
    if (el && el.closest('.compat-field')) return;
    closeSuggestions();
  }

portal.ts / dropdownAnchor.ts — the utilities to reuse, ui/src/lib/utils/. READ-ONLY.

  // portal(node, target = 'body') — moves node into <body> on mount, removes on destroy.
  export function portal(node: HTMLElement, target?: HTMLElement | string): { destroy(): void };

  // dropdownAnchor(node, { anchorEl, gap?, maxHeight? }) — fixed-positions node relative to
  // anchorEl, re-repositions on scroll (capture phase, any ancestor) / resize, flips
  // up when there's no room below. "актуально для дропдаунов внутри модалок" (doc comment).
  export interface DropdownAnchorParams {
    anchorEl: HTMLElement | null;
    gap?: number;
    maxHeight?: number;
  }
  export function dropdownAnchor(
    node: HTMLElement,
    params: DropdownAnchorParams,
  ): { update(newParams: DropdownAnchorParams): void; destroy(): void };

PersonAutocomplete.svelte — the single-instance reference usage of portal+dropdownAnchor inside
a modal-form context (WR-03 namespaced-class convention). READ-ONLY.

  <div class="autocomplete-wrapper" bind:this={wrapperEl}>
    <input bind:this={inputEl} ... onfocus={handleFocus} />
    {#if open}
      <div class="dropdown--person" role="listbox" use:portal use:dropdownAnchor={{ anchorEl: inputEl }} bind:this={dropdownEl}>
        ...
      </div>
    {/if}
  </div>

  function handleClickOutside(e: MouseEvent) {
    const target = e.target as Node;
    const insideWrapper = wrapperEl?.contains(target) ?? false;
    const insideDropdown = dropdownEl?.contains(target) ?? false;
    if (!insideWrapper && !insideDropdown) open = false;
  }

  // WR-03: дропдаун портирован в <body> из НЕСКОЛЬКИХ компонентов — без namespace-класса
  // на корне глобальные правила .dropdown/.dropdown-item/... коллизируют между компонентами.
  :global(.dropdown--person) {
    position: fixed;
    z-index: 1000;
    background: var(--tr-surface-raised);
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-xs);
    box-shadow: var(--tr-elev-2);
    max-height: 240px;
    overflow-y: auto;
  }
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Отключить поиск в двухпунктовом Dropdown типа расходника</name>
  <files>ui/src/features/cartridges/CartridgeFormBody.svelte, ui/src/features/cartridges/ModelFormModal.svelte</files>
  <action>
Re-read both файла перед правкой, чтобы подтвердить текущие номера строк (могли сдвинуться) — см. interfaces выше для точного текста обоих Dropdown-блоков.

В CartridgeFormBody.svelte найти блок Dropdown, привязанный к groups={KIND_OPTIONS} (подпись поля «Что добавляем», внутри {#if !isEdit}). Добавить проп searchable={false} в этот Dropdown (например, сразу после searchPlaceholder="Поиск" — сам searchPlaceholder можно оставить как есть, он просто не рендерится при searchable={false}). НЕ трогать соседний Dropdown «Модель» (groups=modelOptions) и Dropdown «Состояние» (groups=stateOptions) — у них поиск нужен/уместен.

В ModelFormModal.svelte найти блок Dropdown, привязанный к groups={KIND_OPTIONS} (подпись поля «Тип расходника», первый Dropdown в форме). Добавить searchable={false} тем же способом. НЕ трогать Dropdown «Цвет» (groups=COLOR_DROPDOWN_OPTIONS, 6 пунктов) — вне зоны действия этого фикса.

Не изменять ui/src/lib/components/Dropdown.svelte — проп searchable уже существует и работает (default true), задача — только применить его на двух конкретных инстансах.
  </action>
  <verify>
    <automated>cd /Users/madsas/Projects/trackly && grep -c "searchable={false}" ui/src/features/cartridges/CartridgeFormBody.svelte | grep -qx 1 && echo OK_FORM_BODY || echo FAIL_FORM_BODY; grep -c "searchable={false}" ui/src/features/cartridges/ModelFormModal.svelte | grep -qx 1 && echo OK_MODEL_MODAL || echo FAIL_MODEL_MODAL; pnpm --dir ui run svelte-check 2>&1 | tail -30 && pnpm --dir ui build 2>&1 | tail -20</automated>
  </verify>
  <done>Оба файла содержат ровно по одному новому searchable={false} на Dropdown с groups={KIND_OPTIONS}; Dropdown «Модель»/«Состояние»/«Цвет» не изменены; pnpm --dir ui run svelte-check и pnpm --dir ui build проходят без ошибок.</done>
</task>

<task type="auto">
  <name>Task 2: Починить раскладку колонки «Модель» в таблице моделей картриджей</name>
  <files>ui/src/features/cartridges/ModelListRow.svelte</files>
  <action>
Re-read ui/src/features/cartridges/ModelListRow.svelte перед правкой — см. interfaces выше для текущего (буквенного) состояния и для эталонного прецедента PrinterListRow.svelte.

Применить паттерн FIX B3 (уже используется в CartridgeListRow.svelte/.cell-code+.cell-code-inner и в PrinterListRow.svelte/.cell-name+.cell-name-inner): обернуть содержимое td.cell-name (span.name + span.badges) в новый вложенный span.cell-name-inner. Markup должен стать:

  <td class="cell cell-name" title="{model.brand} {model.model}">
    <span class="cell-name-inner">
      <span class="name">{model.brand} {model.model}</span>
      <span class="badges">
        <Badge variant={model.kind_id === 1 ? 'accent' : 'default'} size="sm">
          {model.kind_id === 1 ? 'Картридж' : 'Фотобарабан'}
        </Badge>
        {#if model.kind_id === 1 && model.color}
          <Badge variant="default" size="sm">{model.color}</Badge>
        {/if}
      </span>
    </span>
  </td>

В CSS: убрать display:flex, flex-direction:column, justify-content:center, gap:2px из правила .cell-name (оставить там только overflow:hidden и max-width:0 — точь-в-точь как в PrinterListRow.svelte/.cell-name после FIX B3). Добавить новое правило .cell-name-inner с display:flex; flex-direction:column; justify-content:center; gap:2px; min-width:0; (перенесённые свойства). Правила .name и .badges оставить без изменений — они уже корректны (min-width:0/flex-shrink:0 и т.п.), проблема была исключительно в display:flex на самом td.

Не трогать .cell-count, .cell-notes, .cell-actions и остальную часть файла — они должны сами вернуться к нормальному виду, когда td.cell-name перестанет ломать table-layout колонок; численных изменений ширины им не требуется (тот же вывод справедлив для PrinterListRow.svelte — там .cell-status/.cell-toner не менялись при FIX B3).
  </action>
  <verify>
    <automated>cd /Users/madsas/Projects/trackly && grep -c "cell-name-inner" ui/src/features/cartridges/ModelListRow.svelte | grep -qx 2 && echo OK_INNER_SPAN_PRESENT || echo FAIL_INNER_SPAN_MISSING; sed -n '/\.cell-name {/,/}/p' ui/src/features/cartridges/ModelListRow.svelte | grep -c "display: flex" | grep -qx 0 && echo OK_TD_NOT_FLEX || echo FAIL_TD_STILL_FLEX; pnpm --dir ui run svelte-check 2>&1 | tail -30 && pnpm --dir ui build 2>&1 | tail -20</automated>
  </verify>
  <done>td.cell-name в разметке больше не несёт display:flex (правило .cell-name в CSS содержит только overflow/max-width); новый span.cell-name-inner (ровно 2 вхождения строки — разметка + CSS-правило) держит flex-column layout name+badges; pnpm --dir ui run svelte-check и pnpm --dir ui build проходят без ошибок.</done>
</task>

<task type="auto">
  <name>Task 3: Портировать список подсказок «Совместимые принтеры» за пределы модалки</name>
  <files>ui/src/features/cartridges/CompatibilityEditor.svelte</files>
  <action>
Re-read ui/src/features/cartridges/CompatibilityEditor.svelte перед правкой — см. interfaces выше для текущего состояния и для эталонного прецедента PersonAutocomplete.svelte.

1. Добавить импорты в начало script-блока: import { portal } from '$lib/utils/portal'; и import { dropdownAnchor } from '$lib/utils/dropdownAnchor';.

2. Добавить два новых state-поля рядом с существующими openKey/suggestions/activeIndex/loadingKey: let anchorEl = $state<HTMLInputElement | null>(null); и let dropdownEl = $state<HTMLDivElement | null>(null);. Это единый (не индексированный по строке) якорь/реф — панель подсказок у CompatibilityEditor всегда одна и та же, т.к. openKey гарантирует не более одной открытой панели одновременно; anchorEl обновляется на фокусе той строки, что сейчас активна.

3. Изменить сигнатуру handleFocus(index: number) на handleFocus(index: number, e: FocusEvent) и добавить первой строкой в теле функции: anchorEl = e.currentTarget as HTMLInputElement;. В шаблоне обновить вызов с onfocus={() => handleFocus(i)} на onfocus={(e) => handleFocus(i, e)}.

4. В handleClickOutside добавить проверку принадлежности клика самой (уже портированной) панели подсказок, аналогично PersonAutocomplete.svelte's insideDropdown — панель после use:portal физически покидает поддерево .compat-field, поэтому существующая проверка el.closest('.compat-field') больше не находит клики ВНУТРИ панели:

     function handleClickOutside(e: MouseEvent) {
       if (!openKey) return;
       const el = e.target as HTMLElement | null;
       if (el && el.closest('.compat-field')) return;
       if (dropdownEl?.contains(e.target as Node)) return;
       closeSuggestions();
     }

5. В разметке заменить `<div class="dropdown" role="listbox">` (внутри {#if openKey === getKey(i)}) на:

     <div
       class="dropdown--compat"
       role="listbox"
       use:portal
       use:dropdownAnchor={{ anchorEl, maxHeight: 200 }}
       bind:this={dropdownEl}
     >

   Внутреннее содержимое (loadingKey/suggestions/dropdown-item кнопки) оставить без изменений — меняется только открывающий тег контейнера.

6. В CSS: удалить старые scoped-правила .dropdown { position: absolute; ... }, .dropdown-loading, .dropdown-empty, .dropdown-item (те, что сейчас стилизуют старый position:absolute блок). Добавить namespaced global-правила по образцу PersonAutocomplete.svelte's :global(.dropdown--person ...) (см. interfaces), но под именем .dropdown--compat и с max-height: 200px (сохранить прежнюю высоту панели этого компонента, а не 240px из PersonAutocomplete):

     :global(.dropdown--compat) {
       position: fixed;
       z-index: 1000;
       background: var(--tr-surface-raised);
       border: 1px solid var(--tr-border);
       border-radius: var(--tr-radius-xs);
       box-shadow: var(--tr-elev-2);
       max-height: 200px;
       overflow-y: auto;
     }
     :global(.dropdown--compat .dropdown-loading),
     :global(.dropdown--compat .dropdown-empty) {
       padding: var(--tr-space-xs) var(--tr-space-md);
       color: var(--tr-text-tertiary);
       font-size: var(--tr-font-size-label);
     }
     :global(.dropdown--compat .dropdown-item) {
       display: block;
       width: 100%;
       padding: var(--tr-space-xs) var(--tr-space-md);
       background: transparent;
       border: none;
       text-align: left;
       color: var(--tr-text-primary);
       font-family: inherit;
       font-size: var(--tr-font-size-body);
       cursor: pointer;
     }
     :global(.dropdown--compat .dropdown-item:hover),
     :global(.dropdown--compat .dropdown-item.active) {
       background: var(--tr-row-hover);
     }

   .autocomplete-wrapper { position: relative; } оставить без изменений (используется для позиционирования input, дропдаун больше от него не зависит, но правило безвредно).

Не трогать ModelFormModal.svelte's собственные "Бренд"/"Модель" автокомплиты (там та же position:absolute-разметка, но они вне зоны этого фикса — задача касается только «Совместимые принтеры» / CompatibilityEditor.svelte).
  </action>
  <verify>
    <automated>cd /Users/madsas/Projects/trackly && grep -c "use:portal" ui/src/features/cartridges/CompatibilityEditor.svelte | grep -qx 1 && echo OK_PORTAL || echo FAIL_PORTAL; grep -c "use:dropdownAnchor" ui/src/features/cartridges/CompatibilityEditor.svelte | grep -qx 1 && echo OK_ANCHOR || echo FAIL_ANCHOR; grep -q "dropdown--compat" ui/src/features/cartridges/CompatibilityEditor.svelte && echo OK_NAMESPACED || echo FAIL_NAMESPACED; grep -q "dropdownEl?.contains" ui/src/features/cartridges/CompatibilityEditor.svelte && echo OK_CLICKOUTSIDE || echo FAIL_CLICKOUTSIDE; pnpm --dir ui run svelte-check 2>&1 | tail -30 && pnpm --dir ui build 2>&1 | tail -20</automated>
  </verify>
  <done>Панель подсказок «Совместимые принтеры» использует use:portal + use:dropdownAnchor с namespaced-классом .dropdown--compat (по конвенции PersonAutocomplete.svelte/DeviceAutocompleteField.svelte/LocationAutocomplete.svelte); handleClickOutside учитывает клики внутри портированной панели; ModelFormModal.svelte не изменён; pnpm --dir ui run svelte-check и pnpm --dir ui build проходят без ошибок.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| Пользователь настольного/LAN-браузерного UI → формы «Картриджи» | Изменения этого плана — чисто визуальные/структурные (CSS-layout, DOM-portal, проп существующего компонента). Ни один из трёх фиксов не меняет валидацию, не добавляет новый ввод данных, не открывает новую сетевую поверхность и не трогает бэкенд/БД. |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-thx-01 | Tampering (supply chain) | N/A | accept | Ни одна задача не добавляет новую зависимость и не запускает установку пакетов — только правки существующих .svelte-файлов и переиспользование уже присутствующих в кодовой базе утилит (portal.ts/dropdownAnchor.ts). Package Legitimacy Gate не применим. |
| T-thx-02 | Information Disclosure | CompatibilityEditor.svelte панель подсказок (портированная в body) | accept | Портирование в <body> — уже установленный в кодовой базе паттерн (PersonAutocomplete/DeviceAutocompleteField/LocationAutocomplete), никаких новых данных не открывается — тот же suggestFn/API-вызов, что и раньше, просто иначе спозиционирован в DOM. |
</threat_model>

<verification>
1. `pnpm --dir ui run svelte-check` — 0 ошибок, для всех трёх задач.
2. `pnpm --dir ui build` — успешная сборка, для всех трёх задач.
3. Визуальная проверка выполняется пользователем в живом приложении (UAT) — синтетические харнессы (Playwright/Chromium CSS-снапшоты) не считаются верификацией для Svelte/WKWebView-приложения; см. проектный урок "Synthetic harness not verification".
</verification>

<success_criteria>
- Дропдаун типа расходника в обоих попапах («Новый картридж/фотобарабан», «Новая модель картриджа») больше не показывает поле поиска над двумя пунктами; остальные Dropdown-инстансы не затронуты.
- Таблица «Модели картриджей» показывает название модели над бейджами в первой колонке, «Экземпляров» читаемо и выровнено, колонки не наезжают друг на друга.
- Список подсказок «Совместимые принтеры» раскрывается поверх модалки (portal + fixed-позиционирование), без внутреннего скролла контента попапа.
- `pnpm --dir ui run svelte-check` и `pnpm --dir ui build` проходят чисто после всех трёх задач.
</success_criteria>

<output>
Create `.planning/quick/260819-thx-ui/260819-thx-SUMMARY.md` when done
</output>
