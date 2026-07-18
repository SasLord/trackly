# Phase 25: Таблицы и Dropdown - Context

**Gathered:** 2026-07-19
**Status:** Ready for planning

<domain>
## Phase Boundary

Фаза 25 доставляет **два переиспользуемых примитива дизайн-системы** и **по одному живому потребителю
для каждого** — и ничего больше:

- **Table / TableRow** (`ui/src/lib/components/`) — строка таблицы в состояниях обычная / наведение /
  выбрана + **строка-группа** (фон группы, шеврон свёртки, счётчик-пилюля, отступ вложенных строк),
  по значениям `TableRows.dc.html`. Требование **CMP-06**.
- **Dropdown** (`ui/src/lib/components/`) — комбобокс/селект с **drill-in по группам**, две формы
  поиска, полная клавиатурная модель, пустое/загружающееся состояния, по значениям
  `Dropdown.dc.html`. Требование **CMP-07**.
- **Две новые секции витрины** (`ui/src/features/showcase/sections/`) — по образцу пяти существующих
  секций Фазы 24; поверхность визуального UAT.
- **Два пилотных внедрения**, доказывающих отсутствие регресса: строки в Устройствах и
  drill-in-dropdown в форме Акта.

**В границах фазы:**
- Создание `Table`/`TableRow` и `Dropdown` + секции витрины для обоих.
- Добавление токена `--tr-group` в `_tokens.scss` (D-09).
- Пилот таблицы: `DeviceList.svelte` / `DeviceListRow.svelte` / `DeviceGroupRow.svelte` переводятся
  на новые компоненты (только строки и шапка таблицы).
- Пилот dropdown: групповой пикер устройства в `ActFormItemsTable.svelte` переводится на `Dropdown`.
- Полная клавиатурная/ARIA-модель комбобокса в новом компоненте (D-12).

**Вне границ:**
- **Окно Устройств целиком** (шапка окна, фильтры, layout, пустые состояния экрана) — **Фаза 26,
  WIN-02 НЕ переносится** (D-06). Роадмап и REQUIREMENTS.md этой фазой не правятся.
- Остальные 5 таблиц (`UsersList`, `ReportTable`, `DiscoveryResultsTable`, `CartridgeListRow`,
  `ModelListRow`) и остальные 6 селекторов (`Select`, `PrinterSelect`, `GroupedPrinterSelect`,
  `CartridgeSelect`, `LocationAutocomplete`, `PersonAutocomplete`, `DeviceAutocompleteField`) —
  фазы 26–28 (D-08).
- **Механика выделения строк в Устройствах** — новая функциональность, не редизайн (D-11).
- AA-контраст, focus ring по новому дизайну, паритет Tauri WebView vs LAN-браузер — QA-02/QA-03,
  Фаза 30.
- Любые изменения бизнес-логики, API, бэкенда.

</domain>

<decisions>
## Implementation Decisions

### Группы в Dropdown

- **D-01: Модель групп — drill-in, а не заголовки секций.** Клик по строке-группе **заменяет**
  содержимое панели на экземпляры группы, сверху появляется кнопка-шапка «← Назад · {название группы}»
  (`--tr-surface-sunken`, h=38px). Групповая строка справа несёт счётчик `×N` (`--tr-accent-text`,
  `tabular-nums`) и шеврон `›`.
  **Обоснование:** это одновременно и модель `Dropdown.dc.html`, и **уже работающая** механика
  `ActFormItemsTable.svelte` (`viewModeByRow: 'groups' | 'members'`) — нулевой риск поведенческой
  регрессии.
  **Действие для планировщика/верификатора:** Success Criteria #4 в ROADMAP.md сформулирован как
  «список с группами (**заголовки секций**)» — это не соответствует ни референсу, ни коду.
  **Обновить формулировку SC #4 на drill-in** (тот же приём, что D-06 Фазы 24), иначе верификация
  упадёт на расхождении, хотя код прав.

- **D-02: Поведенческий контракт Фазы 18 сохраняется дословно.** Оба правила остаются частью
  компонента, не пропами-опциями: (1) список раскрывается **при получении фокуса**, без ввода
  (AUTO-02); (2) если после фильтрации осталась **единственная группа**, она не показывается как
  группа — сразу плоский список её устройств (AUTO-05). Фаза 25 меняет **только визуал**.

- **D-03: Реализуются обе формы поиска из референса.**
  (а) **комбобокс** — ввод прямо в поле (`fieldStyle` с focus-состоянием), панель без строки поиска;
  (б) **селект** — поле показывает выбранное значение, строка поиска живёт **внутри панели**
  (`searchBoxStyle`: h=30px, `--tr-surface-sunken`, radius 5px, иконка ⌕).
  **Обоснование:** покрывает оба типа селекторов, реально существующих в приложении.

### Граница фазы и пилотные внедрения

- **D-04: Витрина + пилотные экраны**, а не только витрина. Отличие от D-07 Фазы 24 осознанное:
  строки-группы и drill-in **уже живут в бою**, и SC #5 требует доказать, что portal/anchor Фазы 18
  не сломан — в витрине это недоказуемо.

- **D-05: Пилотов два, по одному на компонент.** Таблица → Устройства (`DeviceList` +
  `DeviceListRow` + `DeviceGroupRow`). Dropdown → форма Акта (`ActFormItemsTable`, групповой пикер
  устройства в модалке — самое рискованное место portal-а).

- **D-06: WIN-02 остаётся Фазе 26.** Рассматривался перенос «Устройства целиком по макету» в эту
  фазу — **отклонено**. Фаза 25 в Устройствах трогает **только строки и шапку таблицы**; шапка окна,
  фильтры, layout, пустые состояния экрана — Фаза 26. ROADMAP.md и REQUIREMENTS.md (`WIN-02 | Phase 26`)
  не изменяются.

### Форма поставки

- **D-07: Настоящие Svelte-компоненты, не набор SCSS-миксинов.** `Table`/`TableRow` и `Dropdown`
  живут в `ui/src/lib/components/` рядом с примитивами Фазы 24.

- **D-08: Принимают их в Фазе 25 только витрина и два пилота.** Остальные 5 таблиц и 6 селекторов
  мигрируют в фазах 26–28. Это прямое следствие D-04/D-05 — граница «пилот, а не ретрофит».

### Токены и расхождения с референсом

- **D-09: Токен `--tr-group` добавляется в `_tokens.scss`.** Его сейчас **нет**, а `TableRows.dc`
  требует и задаёт значения: light `#e9edf5`, dark `#1a212b`. Берутся дословно, для обеих тем.
  **Критично:** closed-world гейт `check-tokens.mjs` роняет сборку при ссылке на несуществующий
  токен — ровно на этом фаза 24 обожглась (см. Lessons «Пометка [VERIFIED] не является проверкой»).
  Остальные нужные токены проверены и присутствуют: `--tr-row-hover`, `--tr-row-selected`,
  `--tr-surface-raised`, `--tr-elev-2`, `--tr-focus-ring`, `--tr-accent-soft`, `--tr-accent-text`,
  `--tr-border-strong`, `--tr-text-tertiary`.

- **D-10: Правило разрешения конфликтов — стили из `.dc`, содержание из приложения.** Референс —
  источник истины по высотам, цветам, шрифтам, отступам, радиусам. Тексты, набор колонок и данные
  берутся из реального экрана. Пример: 8-я колонка в `TableRows.dc` — пустой заголовок с «⋯», в
  `DeviceList` — «Действия»; остаётся «Действия».

### Состояния и доступность

- **D-11: Состояние «выбрана» живёт в компоненте, механика выбора — позже.** `TableRow` получает
  проп `selected` и показывается в витрине во всех трёх состояниях, чем и закрывается CMP-06.
  В Устройствах выделение строк **не вводится** — сейчас его там нет вообще (ни класса, ни
  `aria-selected`), и добавление было бы новой функциональностью, а не редизайном. Существующая
  реализация выделения в `CartridgeListRow` (master-detail) не трогается.

- **D-12: Полная клавиатурная модель комбобокса реализуется в этой фазе**, а не откладывается в
  Фазу 30. Нижняя граница — **паритет с текущим** `ActFormItemsTable` (`role="combobox"`/`listbox"`/
  `option`, `aria-autocomplete="list"`, `aria-selected`, `ArrowDown` на открытие и на навигацию);
  сверх того — фокус-менеджмент при входе в группу и возврате «назад», Esc, Home/End.
  Ничего из существующей ARIA-обвязки потерять нельзя.

- **D-13: Пустое и загружающееся состояния панели входят в компонент и в витрину.** Референс их не
  показывает; проектируются по токенам системы (текст `--tr-text-tertiary`, та же высота строки
  46px). Причина явной фиксации: иначе фазы 26–28 изобретут их по-разному в шести местах.

### Claude's Discretion

- Дробление фазы на планы/волны. Разумная развязка: токен + компонент + его секция витрины + его
  пилот; таблица и dropdown между собой независимы.
- Точная форма API компонентов (пропы vs snippets для ячеек, где живёт `columns`) — при соблюдении
  D-07/D-08 и того, что разметка шести таблиц различна и не унифицируется в этой фазе.
- Устройство секций витрины (один файл на компонент vs подсекции) — по образцу пяти существующих.
- Как именно `Table`/`TableRow` уживаются с разной разметкой (`<table>` в `DeviceList` vs
  `role="row"`-дивы в `ActFormItemsTable`) — решается планировщиком после чтения обоих.
- Механика фокус-менеджмента при drill-in (D-12) — конкретика на усмотрение.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Референсы Claude Design (источник истины по стилям — читать первыми)
Формат Design Canvas (`<x-dc>`, `DCLogic`, `{{ }}`) — **спецификация, а не переносимый код**.
Разметка НИКОГДА не копируется в Svelte; извлекаются только значения из `renderVals`/`rowStyle`.

- `.planning/reference/design-system-v2/TableRows.dc.html` — **основной референс CMP-06**.
  `tdBase`: padding `0 10px`, height 40px, `borderBottom 1px solid var(--tr-border)` (кроме последней),
  `whiteSpace nowrap`, `verticalAlign middle`. `mono`: ui-monospace + `tabular-nums` +
  `--tr-text-secondary` + 13px для инв./серийного №. Шапка `th`: 12px/600, `--tr-text-secondary`,
  h=34px, `borderBottom 2px solid var(--tr-border-strong)`. Фон строки: hover `--tr-row-hover`,
  selected `--tr-row-selected` + `borderLeft 3px solid var(--tr-accent)` (и паддинг слева 32px→29px
  для компенсации). Группа: фон `--tr-group`, `cursor pointer`, имя 600, шеврон 18px с
  `transform rotate(90deg)` при раскрытии + `transition transform .15s`, счётчик-пилюля h=20px,
  radius 10px, 11px/600, `--tr-accent-soft` + `--tr-accent-text` + `border 1px solid var(--tr-accent)`.
  Бейдж статуса: h=22px, radius 11px, 12px/600, 4 тона (neutral/accent/warning/danger).
- `.planning/reference/design-system-v2/Dropdown.dc.html` — **основной референс CMP-07**.
  `field`: h=36px, padding `0 12px`, `--tr-surface`, border `--tr-accent` при фокусе (иначе
  `--tr-border-strong`), radius 6px, focus-ring `0 0 0 3px var(--tr-focus-ring)`. `panel`:
  `marginTop 4px`, `--tr-surface-raised`, `border 1px solid var(--tr-border)`, radius 8px,
  `boxShadow var(--tr-elev-2)`. Строка опции: `minHeight 46px`, padding `8px 12px`,
  `borderBottom 1px solid var(--tr-border)`, hover `--tr-row-hover`, selected `--tr-row-selected`;
  двухстрочная (имя 14px/600 + meta 13px `--tr-text-tertiary` в baseline-ряду; sub 12px
  `--tr-text-tertiary` во второй строке, у плоского варианта — моно). Скролл-контейнер:
  `max-height 280px` (групповой) / `240px` (плоский). Шапка «Назад» и `searchBoxStyle` — см. D-01/D-03.
- `.planning/reference/design-system-v2/Foundations.dc.html` — первоисточник токенов для спорных
  значений. `support.js` — рантайм Design Canvas, не трогать.

### Контракт дизайн-системы (фазы 23–24)
- `ui/src/styles/_tokens.scss` — единственный слой токенов; сюда добавляется `--tr-group` (D-09).
- `.planning/phases/23-design-tokens-foundations/23-UI-SPEC.md` — все hex `--tr-*` (light+dark),
  шкала типографики, motion-решение.
- `.planning/phases/24-base-components/24-CONTEXT.md` — D-01/D-03 (витрина расширяется фазами 25+),
  D-07 (граница ретрофита), D-09 (микро-переходы .12s + подавление при смене темы).
- `.planning/phases/24-base-components/24-LEARNINGS.md` — **обязательно к прочтению**: три ловушки,
  на которых фаза 24 потеряла раунды (ложный `[VERIFIED]` в RESEARCH.md; `:global()` в plain SCSS
  попадает в собранный CSS дословно и не работает; авто-одобрение человеческого чекпоинта маскировало
  непроверенную витрину).

### Контракт поведения из Фазы 18 (D-02, SC #5)
- `.planning/phases/18-autocomplete-dropdowns/18-CONTEXT.md` — решения по portal-рендерингу и
  групповому пикеру устройства.
- `ui/src/lib/utils/portal.ts`, `ui/src/lib/utils/dropdownAnchor.ts` — механика, которую нельзя
  сломать; используется 7 селекторами и обоими контекстными меню.

### Требования и роадмап
- `.planning/ROADMAP.md` §«Phase 25: Таблицы и Dropdown» — цель, 5 Success Criteria.
  **SC #4 требует правки формулировки, см. D-01.**
- `.planning/REQUIREMENTS.md` — CMP-06, CMP-07. **WIN-02 остаётся за Phase 26, не трогать (D-06).**

### Код, который меняется/создаётся
- **Новые:** `ui/src/lib/components/Table.svelte` + `TableRow.svelte` (форма API — на усмотрение),
  `ui/src/lib/components/Dropdown.svelte`, две секции в `ui/src/features/showcase/sections/`.
- `ui/src/features/showcase/ShowcasePage.svelte` — подключение двух новых секций.
- `ui/src/features/devices/DeviceList.svelte` (251 строк), `DeviceListRow.svelte` (117),
  `DeviceGroupRow.svelte` (308) — пилот таблицы.
- `ui/src/features/acts/ActFormItemsTable.svelte` — пилот dropdown; здесь живёт эталонная
  drill-in-механика (`viewModeByRow`, строки ~103–351) и полная ARIA-обвязка (строки ~521–675).
- `ui/src/styles/_tokens.scss` — `--tr-group` (D-09).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **Витрина уже существует и спроектирована под расширение** (`ShowcasePage.svelte` + 5 секций
  `ButtonsSection`/`FieldsSection`/`BadgeSection`/`TabsSection`/`ModalSection`) — новые секции
  добавляются по готовому образцу, каркас строить не нужно.
- **`portal.ts` + `dropdownAnchor.ts`** — рабочая, отлаженная Фазой 18 механика позиционирования;
  новый `Dropdown` **переиспользует** их, а не переписывает.
- **`ActFormItemsTable.svelte` — фактический прототип нового Dropdown**: drill-in, ARIA, клавиатура,
  группы, схлопывание единственной группы. Извлечение в компонент, а не проектирование с нуля.
- Примитивы Фазы 24 (`Badge`, `Checkbox`, `Spinner`) переиспользуются внутри строк/панелей —
  бейдж статуса в `TableRows.dc` совпадает с `Badge` по геометрии (h=22px, radius 11px, 12px/600).

### Established Patterns
- Стили — scoped `<style lang="scss">` в каждом компоненте; глобальных классов почти нет
  (`.skip-link`, `.tr-mono`).
- Пропсы — Svelte 5 runes (`$props`, `$bindable`, `$derived`), `Snippet` для children.
  **Ловушка Фазы 24:** `const` vs `let` при `$bindable()` — разные контракты, не стилистика.
- **Фронтенд-тестов нет** (ни vitest, ни playwright). Гейты: `pnpm lint`, `pnpm svelte-check`,
  `pnpm --dir ui build`, `check-tokens.mjs`. Проверка визуала — только глазами через витрину.
- **`:global()` в plain `.scss` не работает** — `global.scss` обрабатывается sass/Vite, а не
  компилятором Svelte (урок Фазы 24). Грep-гейт на это в CI так и не добавлен.
- **Гоча UAT (память проекта):** серверный режим отдаёт `ui/dist` — перед проверкой через
  LAN-браузер нужен `pnpm --dir ui build`; `cargo tauri dev` хотрелоадит только desktop-webview.
- **Гоча (память проекта):** `prebuild` тянет `cargo test -p trackly-app --test export_bindings` —
  сборка ui тянет cargo.

### Integration Points
- Разметка таблиц **неоднородна**: `DeviceList` использует настоящий `<table>`/`<th>`/`<td>`,
  а `ActFormItemsTable` — дивы с `role="row"`/`role="listbox"`. Общий `Table`/`TableRow` должен
  учитывать это (или явно ограничиться `<table>`-случаем — решение планировщика).
- 6 таблиц и 7 селекторов — потенциальные потребители в фазах 26–28; API проектируется с оглядкой
  на них, но мигрируются только два пилота (D-08).
- Витрина живёт за ролью admin (D-02 Фазы 24) — **известный долг:** гейт стоит на пункте меню,
  но не на маршруте (см. 24-LEARNINGS, и backlog-элемент 999.1 «ролевой гейт на уровне маршрутов»).

</code_context>

<specifics>
## Specific Ideas

- **`.dc`-референсы — это SPEC по значениям.** Ожидается попиксельное соответствие; downstream не
  «улучшает» и не пересчитывает стили, а извлекает их из embedded-скриптов (унаследовано от Фазы 24).
- **Существующее поведение важнее чистоты нового API.** И для таблицы, и для dropdown принят один
  и тот же приём: взять работающую механику как контракт (D-02), а расхождение с текстом роадмапа
  считать дефектом формулировки (D-01), а не поводом переписать код.
- **Пилот вместо ретрофита.** Пользователь сознательно отклонил и «только витрину» (недоказуемо),
  и «мигрировать всё сразу» (съедает фазы 26–28), и расширение до WIN-02.
- Приложение не релизится в середине v1.2 — промежуточный вид (две таблицы на новой системе, четыре
  на старой) принят.

</specifics>

<deferred>
## Deferred Ideas

- **Миграция остальных 5 таблиц** (`UsersList`, `ReportTable`, `DiscoveryResultsTable`,
  `CartridgeListRow`, `ModelListRow`) на `Table`/`TableRow` — фазы 26–28.
- **Миграция остальных 6 селекторов** (`Select`, `PrinterSelect`, `GroupedPrinterSelect`,
  `CartridgeSelect`, `LocationAutocomplete`, `PersonAutocomplete`, `DeviceAutocompleteField`) на
  `Dropdown` — фазы 26–28.
- **Окно Устройств целиком по макету** (`Окно · Список устройств.dc.html`) — Фаза 26, WIN-02.
- **Механика выделения строк в Устройствах** (что делает выбор строки, массовые операции) — новая
  функциональность, требует отдельного решения; не редизайн.
- **Сортировка колонок и липкая шапка таблицы** — в референсе отсутствуют, в требованиях v1.2 нет.
- **AA-контраст, focus ring по новому дизайну, паритет Tauri WebView vs LAN-браузер** — QA-02/QA-03,
  Фаза 30.
- **Грep-гейт на `:global(` в plain `.scss`** (WR-15 Фазы 24) — так и не добавлен в CI.

</deferred>

---

*Phase: 25-dropdown*
*Context gathered: 2026-07-19*
