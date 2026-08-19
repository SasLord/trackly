---
quick_id: 260819-ubv
slug: models-filter-row
phase: 260819-ubv
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - ui/src/features/cartridges/CartridgesSearchAndTabs.svelte
  - ui/src/features/cartridges/CartridgesPage.svelte
  - ui/src/features/cartridges/ModelListRow.svelte
autonomous: true
requirements: [UBV-01, UBV-02]
must_haves:
  truths:
    - "На вкладке «Модели» раздела «Картриджи» пользователь видит текстовое поле фильтра слева от переключателя вкладок «Картриджи | Модели», в одной горизонтальной строке с ним — тем же компонентом/паттерном, что уже используется на вкладке «Картриджи» (UBV-01)"
    - "Ввод текста в поле фильтра моделей регистронезависимо сужает список строк таблицы по бренду+модели (полю, показанному в колонке «Модель») и примечанию, без обращения к бэкенду — фильтрация клиентская, над уже загруженным списком моделей (UBV-01)"
    - "Очистка поля фильтра моделей возвращает полный список моделей"
    - "В таблице «Модели картриджей» ячейка «Модель» — одна строка: вертикальная полоска-индикатор типа расходника у левой границы, затем название модели (бренд+модель, обрезается многоточием при переполнении), затем — если применимо — чип цвета; отдельного чипа «Картридж»/«Фотобарабан» больше нет (UBV-02)"
    - "Тип расходника у полоски-индикатора доступен не только по цвету — у неё есть title и aria-label со значением «Картридж» или «Фотобарабан» (UBV-02)"
    - "td.cell-name таблицы моделей по-прежнему НЕ несёт display:flex напрямую (инвариант FIX B3 сохранён) — колонки таблицы «Модели картриджей» не разъезжаются и не наезжают друг на друга"
  artifacts:
    - path: "ui/src/features/cartridges/CartridgesSearchAndTabs.svelte"
      provides: "Поле фильтра моделей (id=models-search) рендерится в той же строке, что и Tabs, когда activeTab==='models' — по образцу существующего поля поиска картриджей (id=cartridges-search)"
      contains: "models-search"
    - path: "ui/src/features/cartridges/CartridgesPage.svelte"
      provides: "Состояние modelSearchQuery + клиентский derived filteredModels (фильтр по brand/model/notes), переданный в ModelsList вместо полного models"
      contains: "filteredModels"
    - path: "ui/src/features/cartridges/ModelListRow.svelte"
      provides: "Однострочная ячейка «Модель»: span.kind-indicator (title/aria-label) + span.name + опциональный Badge цвета, без отдельного Badge типа расходника; td.cell-name без display:flex"
      contains: "kind-indicator"
  key_links:
    - from: "ui/src/features/cartridges/CartridgesSearchAndTabs.svelte Input(id=models-search)"
      to: "ui/src/features/cartridges/CartridgesPage.svelte onModelSearchChange"
      via: "debounced oninput callback, тот же паттерн что и onSearchChange для cartridges-search"
      pattern: "onModelSearchChange"
    - from: "ui/src/features/cartridges/CartridgesPage.svelte filteredModels"
      to: "ui/src/features/cartridges/ModelsList.svelte models prop"
      via: "<ModelsList models={filteredModels} .../> вместо {models}"
      pattern: "models={filteredModels}"
    - from: "ui/src/features/cartridges/ModelListRow.svelte td.cell-name"
      to: "ui/src/lib/components/TableRow.svelte base td rule (FIX B3 invariant)"
      via: "td остаётся display:table-cell; flex-row layout живёт на вложенном span.cell-name-inner"
      pattern: "cell-name-inner"
---

<objective>
Две точечные доработки вкладки «Модели» раздела «Картриджи» (Svelte 5, ui/), продолжение квика
260819-thx, который только что починил раскладку колонок этой же таблицы (FIX B3). Бэкенд/БД не
трогаем.

1. Добавить текстовое поле фильтра списка моделей над таблицей «Модели картриджей» — в одной
   строке с переключателем вкладок «Картриджи | Модели», слева от него (поле — на том же месте,
   что и spacer сейчас). Переиспользовать существующий компонент/паттерн: тот же `Input`
   (`type="search"`), тот же debounce-приём, что уже реализован для вкладки «Картриджи» в
   `CartridgesSearchAndTabs.svelte` (`id="cartridges-search"`). Фильтрация — клиентская
   (`models` уже загружены целиком через `refreshModels()` в `CartridgesPage.svelte` и не
   пагинируются), по полям бренд+модель (то, что показано в колонке «Модель») + примечание,
   регистронезависимо. Решение зафиксировано: клиентский фильтр, не бэкенд-эндпоинт.

2. В `ModelListRow.svelte` свернуть ячейку «Модель» в одну строку: убрать отдельный чип
   «Картридж»/«Фотобарабан», заменить его вертикальной полоской-индикатором у левой границы
   ячейки, цвет которой кодирует тип расходника; чип цвета (если есть) остаётся и идёт ПОСЛЕ
   названия модели в той же строке. Полоска обязана иметь `title`/`aria-label` с текстовым
   значением типа — не только цветовое кодирование. Обрезание длинного названия многоточием
   сохраняется; полоска и чип цвета не схлопываются (`flex-shrink: 0`).

   Инвариант FIX B3 (задокументирован в 260819-thx-PLAN.md, уже применён в этом же файле):
   `display:flex` не должен стоять напрямую на `<td>` — иначе ячейка выпадает из колоночной
   модели таблицы и все колонки разъезжаются. Раскладка живёт на вложенном
   `span.cell-name-inner`, который уже существует в файле после 260819-thx — этот план его не
   удаляет, а меняет с колоночной (`flex-direction: column`) на однострочную (`row`).

Purpose: вкладка «Модели» получает тот же UX поиска, что и вкладка «Картриджи»; ячейка «Модель»
становится компактнее и однороднее по высоте строки, тип расходника остаётся доступным
(a11y), не занимая отдельного чипа.

Output: рабочее поле фильтра моделей над таблицей; однострочная ячейка «Модель» с доступным
вертикальным индикатором типа вместо чипа.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@CLAUDE.md
@.planning/STATE.md

<interfaces>
CartridgeModelDto — ui/src/bindings.ts (READ-ONLY, do not edit bindings.ts):

  export type CartridgeModelDto = {
    id: number; version: number; brand: string; model: string; kind_id: number;
    color: string | null; notes: string | null; created_at_utc: number; updated_at_utc: number;
    compatibility: string[];
    instance_count?: number;
  };

CartridgesSearchAndTabs.svelte — CURRENT (as of 260819-thx close, unchanged since), full file
already read at planning time. Re-read before editing — line numbers may drift.

  Current markup (only cartridges tab gets a real Input; models tab gets a spacer):

    <div class="search-and-tabs">
      {#if activeTab === 'cartridges'}
        <div class="search-wrap">
          <Input
            id="cartridges-search"
            type="search"
            value={localQuery}
            placeholder="Поиск по коду, модели, расположению"
            oninput={handleInput}
          />
        </div>
      {:else}
        <div class="search-spacer"></div>
      {/if}
      <Tabs variant="underline" tabs={tabItems} active={activeTab} ariaLabel="Разделы картриджей"
        onchange={(key) => onTabChange(key as TabKey)} />
    </div>

  Current script (debounce pattern to mirror for the models field):

    let localQuery = $state(searchQuery);
    let debounceTimer: ReturnType<typeof setTimeout> | null = null;

    $effect(() => {
      if (searchQuery !== localQuery && document.activeElement?.id !== 'cartridges-search') {
        localQuery = searchQuery;
      }
    });

    function handleInput(v: string) {
      localQuery = v;
      if (debounceTimer !== null) clearTimeout(debounceTimer);
      debounceTimer = setTimeout(() => { onSearchChange(v); }, 250);
    }

  Current Props interface:

    interface Props {
      searchQuery: string;
      activeTab: TabKey;
      counts: CartridgeCountsDto;
      onSearchChange: (_q: string) => void;
      onTabChange: (_tab: TabKey) => void;
    }

  Current CSS (the .search-spacer rule this plan's markup change makes dead — remove it):

    .search-and-tabs {
      display: flex;
      flex-direction: row;
      align-items: center;
      justify-content: space-between;
      gap: var(--tr-space-md);
      margin-bottom: var(--tr-space-md);
    }
    .search-wrap {
      flex: 1;
      max-width: 480px;
    }
    .search-spacer {
      flex: 1;
      max-width: 480px;
      height: 36px; // Reserve height to avoid layout shift when switching tabs
    }

Input.svelte — props contract (READ-ONLY, do not edit), ui/src/lib/components/Input.svelte:

  interface Props {
    type?: 'text' | 'number' | 'search' | 'password';
    value: string;
    placeholder?: string;
    id?: string;
    autocomplete?: HTMLInputAttributes['autocomplete'];
    oninput?: (_value: string) => void;
    // ...disabled/invalid/aria-describedby/iconLeft also exist, not needed here
  }

CartridgesPage.svelte — CURRENT relevant slices (full file already read at planning time;
re-read before editing, line numbers may drift):

  State block (models are loaded whole, no pagination — client-side filter is correct here):

    let models = $state<CartridgeModelDto[]>([]);
    let modelsLoading = $state(false);
    ...
    let searchQuery = $state('');
    ...
    async function refreshModels() {
      modelsLoading = true;
      try {
        models = await cartridges.modelsList();
      } catch {
        // Non-fatal.
      } finally {
        modelsLoading = false;
      }
    }

  IMPORTANT: `models` (full/unfiltered) is ALSO passed to `<CartridgeFilters {models} .../>`
  (model dropdown filter options, «Картриджи» tab) and to `<CartridgeFormModal {models} .../>`
  (model picker in the add/edit cartridge form) — those two usages MUST keep receiving the full
  unfiltered `models` array. Only the `<ModelsList models={...} .../>` call (inside the
  `{:else}` branch, «Модели» tab) switches to the new filtered derived.

  CartridgesSearchAndTabs usage to extend with the two new props:

    <CartridgesSearchAndTabs
      {searchQuery}
      {activeTab}
      {counts}
      onSearchChange={(q) => (searchQuery = q)}
      onTabChange={(t) => (activeTab = t)}
    />

  ModelsList usage to switch from {models} to the filtered derived:

    <ModelsList
      {models}
      loading={modelsLoading}
      onCreateModel={handleCreateModel}
      onEditModel={handleEditModel}
      onDeleteModel={handleDeleteModel}
    />

ModelsList.svelte — Props contract (READ-ONLY, do not edit), confirms `models` prop is just an
array rendered verbatim (`{#each models as m (m.id)}`) — safe to swap the array passed in from
the parent without touching this file:

  interface Props {
    models: CartridgeModelDto[];
    loading: boolean;
    onCreateModel: () => void;
    onEditModel: (_model: CartridgeModelDto) => void;
    onDeleteModel: (_model: CartridgeModelDto) => void;
  }

ModelListRow.svelte — CURRENT (post-260819-thx) markup+CSS, full file already read at planning
time. Re-read before editing — line numbers may drift.

  Current markup (~line 54-66):

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

  Current CSS (the parts this plan's Task 2 touches):

    .cell-name {
      overflow: hidden;
      max-width: 0; // makes text-overflow work in table cells
    }
    .cell-name-inner {
      display: flex;
      flex-direction: column;
      justify-content: center;
      gap: 2px;
      min-width: 0;
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

  FIX B3 invariant (from 260819-thx, MUST remain true after this plan): `.cell-name` must never
  carry `display: flex` — that property lives only on the nested inner span. This plan changes
  `.cell-name-inner` from a `flex-direction: column` stack (name-over-badges) to a
  `flex-direction: row` single line — still on the INNER span, never on the `<td>` itself.

PrinterListRow.svelte — the established a11y-indicator precedent (accessible dot with
title+aria-label inside a single-line `.cell-name-inner`), READ-ONLY, do not edit. Mirror this
shape for the new `.kind-indicator` (bar instead of circle):

  <span class="cell-name-inner">
    {#if printer.hasAlert}
      <span class="alert-dot" aria-label="Есть проблема с принтером" title="Есть проблема"></span>
    {/if}
    <span class="name-lines">...</span>
  </span>

  .cell-name-inner {
    display: flex;
    align-items: center;
    gap: var(--tr-space-2xs);
  }
  .alert-dot {
    flex-shrink: 0;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--tr-danger);
  }

Design tokens to reuse for the two indicator colors (ui/src/styles/_tokens.scss, READ-ONLY —
do not add new tokens, these already exist and cover light+dark themes):

  --tr-accent          // Cartridge (kind_id === 1) — matches the OLD Badge variant="accent"
  --tr-border-strong    // Photobarrel/фотобарабан (kind_id !== 1) — neutral; matches the OLD
                         // Badge variant="default" semantics without reusing its exact
                         // background (badge bg was --tr-surface-sunken, too close to the row
                         // background for a thin solid bar to read against — --tr-border-strong
                         // is the nearest neutral token with enough contrast as a solid fill)
  --tr-space-2xs        // gap token, already used throughout this file
  --tr-radius-xs        // 4px; use a smaller value (e.g. 2px) for the bar's own border-radius
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Фильтр над таблицей моделей (вкладка «Модели»)</name>
  <files>ui/src/features/cartridges/CartridgesSearchAndTabs.svelte, ui/src/features/cartridges/CartridgesPage.svelte</files>
  <action>
Re-read оба файла перед правкой — см. interfaces выше для точного текущего состояния и точного
текста блоков, номера строк могли сдвинуться.

В `CartridgesSearchAndTabs.svelte`:

1. Расширить `Props`: добавить `modelSearchQuery: string;` и `onModelSearchChange: (_q: string) => void;` рядом с существующими `searchQuery`/`onSearchChange`. Деструктурировать оба новых пропа из `$props()`.

2. Добавить второй набор локального debounce-состояния, зеркальный существующему (`localQuery`/`debounceTimer`/`$effect`/`handleInput`), но для модели: `localModelQuery = $state(modelSearchQuery)`, отдельный `modelDebounceTimer`, отдельный `$effect` для внешнего сброса (проверка `document.activeElement?.id !== 'models-search'`), и `handleModelInput(v: string)` со своим `setTimeout(() => onModelSearchChange(v), 250)` — тот же паттерн, что `handleInput`, только с новым id/переменными, ничего не изобретать заново.

3. В разметке заменить ветку `{:else}` (сейчас — `<div class="search-spacer"></div>`) на реальное поле поиска, тот же `Input`-компонент, что и в ветке `cartridges`, с `id="models-search"`, `value={localModelQuery}`, `placeholder="Поиск по бренду, модели, примечанию"`, `oninput={handleModelInput}`, обёрнутое в `<div class="search-wrap">` (тот же класс, что у cartridges-поля — уже задаёт `flex: 1; max-width: 480px;`, свою вёрстку изобретать не нужно).

4. В CSS удалить теперь неиспользуемое правило `.search-spacer` (markup, использовавший его, заменён на `.search-wrap` в обеих ветках).

В `CartridgesPage.svelte`:

1. Добавить `let modelSearchQuery = $state('');` рядом с существующим `let searchQuery = $state('');`.

2. Добавить клиентский derived-фильтр рядом с `activeFilter`/`hasFilter`:

     const filteredModels = $derived.by(() => {
       const q = modelSearchQuery.trim().toLowerCase();
       if (!q) return models;
       return models.filter((m) => {
         const haystack = `${m.brand} ${m.model} ${m.notes ?? ''}`.toLowerCase();
         return haystack.includes(q);
       });
     });

   Фильтрует по бренду+модели (то же, что показано в колонке «Модель») и примечанию —
   регистронезависимо (`.toLowerCase()` с обеих сторон), клиентски, без сетевого запроса —
   `models` уже загружен целиком через `refreshModels()`/`onMount`.

3. В разметке передать в `CartridgesSearchAndTabs` два новых пропа: `{modelSearchQuery}` и
   `onModelSearchChange={(q) => (modelSearchQuery = q)}` — рядом с существующими
   `{searchQuery}`/`onSearchChange`.

4. В вызове `<ModelsList ... />` (внутри `{:else}` ветки, вкладка «Модели») заменить
   `{models}` на `models={filteredModels}`. НЕ трогать остальные потребители `models`
   (`<CartridgeFilters {models} .../>` и `<CartridgeFormModal {models} .../>`) — им
   по-прежнему нужен полный, нефильтрованный список для построения опций выбора модели.
  </action>
  <verify>
    <automated>cd /Users/madsas/Projects/trackly && grep -q "models-search" ui/src/features/cartridges/CartridgesSearchAndTabs.svelte && echo OK_MODELS_INPUT || echo FAIL_MODELS_INPUT; grep -q "modelSearchQuery" ui/src/features/cartridges/CartridgesSearchAndTabs.svelte && echo OK_PROP_TABS || echo FAIL_PROP_TABS; grep -q "search-spacer" ui/src/features/cartridges/CartridgesSearchAndTabs.svelte && echo FAIL_SPACER_STILL_PRESENT || echo OK_SPACER_REMOVED; grep -q "filteredModels" ui/src/features/cartridges/CartridgesPage.svelte && echo OK_FILTERED_DERIVED || echo FAIL_FILTERED_DERIVED; grep -q "models={filteredModels}" ui/src/features/cartridges/CartridgesPage.svelte && echo OK_MODELSLIST_WIRED || echo FAIL_MODELSLIST_WIRED; grep -c "{models}" ui/src/features/cartridges/CartridgesPage.svelte | grep -qE "^[2-9]|^[1-9][0-9]+" && echo OK_OTHER_MODELS_CONSUMERS_INTACT || echo FAIL_OTHER_MODELS_CONSUMERS_INTACT; pnpm --dir ui run svelte-check 2>&1 | tail -30 && pnpm --dir ui build 2>&1 | tail -20</automated>
  </verify>
  <done>«Модели»-вкладка показывает Input(id=models-search) в одной строке с Tabs, слева от него, зеркальный debounce-паттерну cartridges-search; CartridgesPage.svelte хранит modelSearchQuery и производный filteredModels (фильтр по brand/model/notes, регистронезависимо, клиентски); ModelsList получает filteredModels, CartridgeFilters/CartridgeFormModal по-прежнему получают полный models; pnpm --dir ui run svelte-check и pnpm --dir ui build проходят без ошибок.</done>
</task>

<task type="auto">
  <name>Task 2: Однострочная ячейка «Модель» с вертикальным индикатором типа</name>
  <files>ui/src/features/cartridges/ModelListRow.svelte</files>
  <action>
Re-read ui/src/features/cartridges/ModelListRow.svelte перед правкой — см. interfaces выше для
точного текущего состояния (после 260819-thx) и для эталонного a11y-индикатора PrinterListRow.svelte.

1. Добавить вычисляемую метку типа рядом с остальными переменными компонента:

     const kindLabel = $derived(model.kind_id === 1 ? 'Картридж' : 'Фотобарабан');

2. Заменить содержимое `<td class="cell cell-name" ...>` на:

     <td class="cell cell-name" title="{model.brand} {model.model}">
       <span class="cell-name-inner">
         <span
           class="kind-indicator"
           class:kind-indicator--drum={model.kind_id !== 1}
           title={kindLabel}
           aria-label={kindLabel}
         ></span>
         <span class="name">{model.brand} {model.model}</span>
         {#if model.kind_id === 1 && model.color}
           <Badge variant="default" size="sm">{model.color}</Badge>
         {/if}
       </span>
     </td>

   Отдельный Badge типа расходника («Картридж»/«Фотобарабан») и обёртка `<span class="badges">`
   удалены целиком — их заменяет `span.kind-indicator`. Badge цвета остаётся, идёт третьим
   элементом в строке (после индикатора и названия), рендерится по тому же условию, что и
   раньше (`model.kind_id === 1 && model.color`).

3. В CSS:
   - Изменить `.cell-name-inner`: убрать `flex-direction: column; justify-content: center; gap: 2px;`, оставить/добавить однострочную раскладку:

       .cell-name-inner {
         display: flex;
         align-items: center;
         gap: var(--tr-space-2xs);
         min-width: 0;
       }

   - Добавить новое правило `.kind-indicator` (полоска-индикатор, зафиксированной ширины/высоты — не растягивается и не схлопывается):

       .kind-indicator {
         flex-shrink: 0;
         width: 3px;
         height: 16px;
         border-radius: 2px;
         background: var(--tr-accent);
       }
       .kind-indicator--drum {
         background: var(--tr-border-strong);
       }

   - В правиле `.name` заменить `white-space: nowrap;`/`overflow: hidden;`/`text-overflow: ellipsis;`/`min-width: 0;` (они уже там, оставить как есть) и ДОБАВИТЬ `flex: 1 1 auto;` — чтобы `.name` был единственным элементом строки, который сжимается/растягивается, а индикатор и Badge цвета сохраняли свой размер.
   - Удалить правило `.badges` целиком (обёртка больше не рендерится).
   - Убедиться, что `.cell-name` НЕ содержит `display: flex` ни до, ни после правки — оставить как есть (`overflow: hidden; max-width: 0;`), это инвариант FIX B3.
   - Badge цвета (третий элемент строки) должен не схлопываться при недостатке места — добавить в `.cell-name-inner` правило-потомок через `:global()` (Badge.svelte — другой scope-hash, `<span class="badge ...">` — как и в остальных *ListRow-файлах, где стилизуются чужие потомки): `.cell-name-inner :global(.badge) { flex-shrink: 0; }`.

Не трогать `.cell-count`, `.cell-notes`, `.cell-actions`, кебаб-меню и остальную часть файла —
эта задача касается исключительно первой колонки.
  </action>
  <verify>
    <automated>cd /Users/madsas/Projects/trackly && grep -q "kind-indicator" ui/src/features/cartridges/ModelListRow.svelte && echo OK_INDICATOR_PRESENT || echo FAIL_INDICATOR_PRESENT; grep -c "aria-label={kindLabel}" ui/src/features/cartridges/ModelListRow.svelte | grep -qx 1 && echo OK_ARIA_LABEL || echo FAIL_ARIA_LABEL; grep -c "title={kindLabel}" ui/src/features/cartridges/ModelListRow.svelte | grep -qx 1 && echo OK_TITLE || echo FAIL_TITLE; grep -c "class=\"badges\"" ui/src/features/cartridges/ModelListRow.svelte | grep -qx 0 && echo OK_OLD_BADGES_WRAPPER_REMOVED || echo FAIL_OLD_BADGES_WRAPPER_STILL_PRESENT; grep -c "kind_id === 1 ? 'accent'" ui/src/features/cartridges/ModelListRow.svelte | grep -qx 0 && echo OK_OLD_KIND_BADGE_REMOVED || echo FAIL_OLD_KIND_BADGE_STILL_PRESENT; sed -n '/^  \.cell-name {/,/^  }/p' ui/src/features/cartridges/ModelListRow.svelte | grep -v '^#' | grep -c "display: flex" | grep -qx 0 && echo OK_TD_NOT_FLEX || echo FAIL_TD_STILL_FLEX; pnpm --dir ui run svelte-check 2>&1 | tail -30 && pnpm --dir ui build 2>&1 | tail -20</automated>
  </verify>
  <done>Ячейка «Модель» — одна строка (span.kind-indicator + span.name + опциональный Badge цвета) внутри span.cell-name-inner с flex-direction: row; отдельный Badge типа расходника и .badges-обёртка удалены; kind-indicator имеет title и aria-label со значением «Картридж»/«Фотобарабан», flex-shrink:0, цвет по --tr-accent/--tr-border-strong; .name растягивается/обрезается многоточием (flex:1 1 auto), Badge цвета не схлопывается; td.cell-name по-прежнему не несёт display:flex (FIX B3 инвариант сохранён); pnpm --dir ui run svelte-check и pnpm --dir ui build проходят без ошибок.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|--------------|
| Пользователь настольного/LAN-браузерного UI → вкладка «Модели» раздела «Картриджи» | Изменения этого плана — чисто фронтендовые (Svelte-состояние, клиентская фильтрация уже загруженного массива, CSS-layout). Ни одна задача не меняет бэкенд/БД/сетевую поверхность и не добавляет новый источник ввода данных, кроме текстового поля фильтра, которое НЕ отправляется на сервер — используется только для локального `Array.filter` в браузере/webview. |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|------------------|
| T-ubv-01 | Tampering (supply chain) | N/A | accept | Ни одна задача не добавляет новую зависимость и не запускает установку пакетов — только правки существующих .svelte-файлов, переиспользующие уже присутствующие в кодовой базе компоненты (Input.svelte, Badge.svelte) и токены (_tokens.scss). Package Legitimacy Gate не применим. |
| T-ubv-02 | Information Disclosure | Поле фильтра моделей (CartridgesSearchAndTabs.svelte) | accept | Фильтр работает над уже загруженным на клиент, уже видимым пользователю списком моделей (`cartridges.modelsList()`, вызывается один раз при монтировании страницы) — новых данных не запрашивается и не раскрывается, только клиентское сужение отображаемого подмножества. |
| T-ubv-03 | Denial of Service (client-side) | filteredModels $derived в CartridgesPage.svelte | accept | Линейный `Array.filter`/`.includes` по списку моделей картриджей одной организации (типично десятки-сотни записей, не выгружается постранично именно по этой причине) — при типичных объёмах не создаёт заметной задержки на каждый keystroke; при явном будущем росте объёма можно добавить debounce длиннее 250мс, вне скоупа этого плана. |
</threat_model>

<verification>
1. `pnpm --dir ui run svelte-check` — 0 ошибок, для обеих задач.
2. `pnpm --dir ui build` — успешная сборка, для обеих задач.
3. Визуальная проверка выполняется пользователем в живом приложении (UAT) — синтетические
   харнессы (Playwright/Chromium CSS-снапшоты) не считаются верификацией для Svelte/WKWebView-
   приложения; см. проектный урок «Synthetic harness not verification». Проверить вручную:
   поле фильтра над таблицей «Модели» сужает список по вводу (регистронезависимо), очистка поля
   возвращает полный список; ячейка «Модель» — одна строка с полоской слева и чипом цвета
   справа от названия; наведение/фокус на полоску показывает «Картридж»/«Фотобарабан».
</verification>

<success_criteria>
- На вкладке «Модели» раздела «Картриджи» над таблицей — рабочее поле фильтра, в одной строке
  с переключателем вкладок, тем же компонентом/паттерном, что и на вкладке «Картриджи».
- Фильтр сужает список моделей по бренду+модели+примечанию, регистронезависимо, клиентски.
- Ячейка «Модель» — одна строка: полоска-индикатор типа расходника (с title/aria-label) слева,
  название модели по центру (обрезается многоточием), опциональный чип цвета справа; отдельного
  чипа типа расходника больше нет.
- td.cell-name таблицы моделей не несёт display:flex напрямую — инвариант FIX B3 сохранён,
  колонки таблицы не разъезжаются.
- `pnpm --dir ui run svelte-check` и `pnpm --dir ui build` проходят чисто после обеих задач.
</success_criteria>

<output>
Create `.planning/quick/260819-ubv-models-filter-row/260819-ubv-SUMMARY.md` when done
</output>
