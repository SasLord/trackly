# Phase 27: Окна основного рабочего процесса — Pattern Map

**Mapped:** 2026-07-21
**Files analyzed:** 41 `.svelte` (Акты 16, Картриджи 17, Принтеры 12 — за вычетом `api.ts`/утилит)
**Analogs found:** 41 / 41 (все имеют внутренний прецедент из Фаз 24–26; новый артефакт — один: общий паттерн детальной панели D-01)

> Фаза чисто визуальная (SC #4): **поля, действия, workflow, API не меняются**. Всё ниже — про
> ре-токенизацию и перевод на примитивы, форма из системы Фаз 23–25, содержание из приложения.
> Гейт `check-tokens.mjs` роняет сборку при ссылке на несуществующий токен — все имена токенов
> в паттернах ниже уже проверены по `ui/src/styles/_tokens.scss`.

---

## File Classification

Роль здесь = слой в структуре окна; Data Flow = как данные текут через компонент.

### Акты (WIN-03)

| Файл | Роль | Data Flow | Ближайший аналог | Качество |
|------|------|-----------|------------------|----------|
| `acts/ActsMasterDetail.svelte` | layout (master-detail) | container | `devices` нет прямого — правка внутри файла (D-02) | self / role-match |
| `acts/ActsSearchAndTabs.svelte` | filter-bar | request-response (debounce+tabs) | `devices/DeviceFilters.svelte` | exact |
| `acts/ActsList.svelte` | list | CRUD (list+empty+footer) | `devices/DeviceList.svelte` | role-match (плоский, без групп) |
| `acts/ActListRow.svelte` | list-row | transform (row render) | `devices/DeviceListRow.svelte` | role-match (2-строчная → табличная) |
| `acts/ActDetail.svelte` | detail-panel | transform (read-only render) | shared-pattern D-01 (нов.) + сам себя | role-match |
| `acts/ReturnModal.svelte` | modal (form) | request-response | уже на `Modal`; ре-токенизация внутренностей | role-match |
| `acts/DocumentAcceptanceModal.svelte` | modal (form) | request-response | уже на `Modal` | role-match |
| `acts/PdfPreviewModal.svelte` | modal (preview) | file-I/O (PDF) | уже на `Modal` | role-match |
| `acts/ActFormModal.svelte` / `ActFormBody.svelte` | modal (form) | CRUD | `devices/DeviceFormModal/Body.svelte` | exact |
| `acts/ActFormItemsTable.svelte` | table (form-internal) | transform | **уже потребитель `Table`** — не ломать (D-03 caution) | keep |
| `acts/ActItemsTable.svelte`, `ReturnItemsTable.svelte` | table (detail-internal) | transform | ре-токенизация | role-match |
| `acts/ActHeaderField.svelte`, `ActNumberField.svelte` | field-widget | transform | field-паттерн D-01 | role-match |

### Картриджи (WIN-04)

| Файл | Роль | Data Flow | Ближайший аналог | Качество |
|------|------|-----------|------------------|----------|
| `cartridges/CartridgesMasterDetail.svelte` | layout | container | правка внутри (D-02) — идентичен Acts | self |
| `cartridges/CartridgesSearchAndTabs.svelte` | filter-bar | request-response | `devices/DeviceFilters.svelte` | exact |
| `cartridges/CartridgesList.svelte` / `CartridgeListRow.svelte` | list / row | CRUD / transform | `devices/DeviceList.svelte` / `DeviceListRow.svelte` | role-match |
| `cartridges/ModelsList.svelte` / `ModelListRow.svelte` | list / row | CRUD / transform | `devices/DeviceList.svelte` / `DeviceListRow.svelte` | role-match |
| `cartridges/CartridgeDetail.svelte` | detail-panel | transform | shared-pattern D-01 | role-match |
| `cartridges/OperationModal.svelte` (887) | modal (multi-step form) | request-response | уже на `Modal`; внутренности D-04 | role-match |
| `cartridges/ModelFormModal.svelte` (580) | modal (form) | CRUD | `devices/DeviceFormModal.svelte` | role-match |
| `cartridges/CartridgeFormModal.svelte` / `CartridgeFormBody.svelte` | modal (form) | CRUD | `devices/DeviceFormModal/Body.svelte` | exact |
| `cartridges/CompatibilityEditor.svelte` | form-widget | event-driven (add/remove) | ре-токенизация D-04 | role-match |
| `cartridges/CartridgeContextMenu.svelte` | menu-widget | event-driven | `devices/DeviceContextMenu.svelte` / `ActionMenu` | exact |
| `cartridges/LowStockBanner.svelte` | banner-widget | transform | `printers/PrinterAlertBanner.svelte` (взаимные близнецы) | exact |

### Принтеры (WIN-05)

| Файл | Роль | Data Flow | Ближайший аналог | Качество |
|------|------|-----------|------------------|----------|
| `printers/PrintersMasterDetail.svelte` | layout | container | правка внутри (D-02) — идентичен Acts | self |
| `printers/PrintersSearchAndTabs.svelte` | filter-bar | request-response | `devices/DeviceFilters.svelte` | exact |
| `printers/PrintersList.svelte` / `PrinterListRow.svelte` | list / row | CRUD / transform | `devices/DeviceList.svelte` / `DeviceListRow.svelte` | role-match |
| `printers/PrinterDetail.svelte` (603) | detail-panel | transform (+async readings) | shared-pattern D-01 | role-match (крупнейший bespoke) |
| `printers/PrinterCreateModal.svelte` | modal (form) | CRUD | `devices/DeviceFormModal.svelte` | exact |
| `printers/DiscoveryModal.svelte` | modal (scan) | streaming (SNMP) | уже на `Modal`; внутренности D-04 | role-match |
| `printers/DiscoveryResultsTable.svelte` | table (raw `<table>`) | transform | `devices/DeviceList.svelte` + `Table`/`TableRow` (D-04) | role-match |
| `printers/TonerGauge.svelte` | gauge-widget | transform | ре-токенизация (уже на токенах) D-04 | keep+audit |
| `printers/PrinterAlertBanner.svelte` | banner-widget | transform | `cartridges/LowStockBanner.svelte` | exact |

### Страницы-оркестраторы (наследуют оболочку Фазы 26, `PageHeader` + скроллящееся тело)

| Файл | Роль | Аналог |
|------|------|--------|
| `acts/ActsPage.svelte`, `cartridges/CartridgesPage.svelte`, `printers/PrintersPage.svelte` | page (orchestrator) | `devices/DevicesPage.svelte` — обёртка `PageHeader` + master-detail, менять только поверхности/токены |

---

## Pattern Assignments

### D-05: `*SearchAndTabs` → примитив `Tabs`

**Аналог (эталон переноса):** `ui/src/features/devices/DeviceFilters.svelte`
**Применить к:** `ActsSearchAndTabs`, `CartridgesSearchAndTabs`, `PrintersSearchAndTabs`

Все три сейчас держат **самописные `<button class="tab">` + `<Badge>` для счётчиков** (`ActsSearchAndTabs`
строки 54–72 и `<style>` 86–119). Ровно тот bespoke-пласт, что `DeviceFilters` уже снял, заменив на
`Tabs variant="underline"` со встроенным `count`.

**Импорт + разметка (копировать из `DeviceFilters.svelte` строки 5–7, 94–100):**
```svelte
import Input from '$lib/components/Input.svelte';
import Tabs from '$lib/components/Tabs.svelte';

<Tabs
  variant="underline"
  tabs={tabItems}
  active={String(statusFilter)}
  ariaLabel="Фильтр по статусу"
  onchange={(key) => onStatusChange(key === 'null' ? null : Number(key))}
/>
```

**Адаптер строкового контракта `Tabs` (когда ключ не строка — `DeviceFilters` строки 61–65):**
```ts
// Tabs требует string key + встроенный count — счётчик больше НЕ через <Badge>.
const tabItems = $derived(
  STATUSES.map((s) => ({ key: String(s.id), label: s.label, count: getCount(s.id) })),
);
```

Ключевые различия таб-ключей (не строки, нужен маппинг туда-обратно):
- **Acts** — `TabKey = 'handover' | 'returns' | 'archive'` (`ActsSearchAndTabs` строки 37–41);
  ключи уже строковые → адаптер тривиален, `count` берётся из `ActsCountsDto`.
- **Printers** — уже `role="tablist"` (строка 68), ближе всего к `underline`-семантике `Tabs`.
- **Cartridges** — `<nav class="tabs">` + `<Badge>` (строки 58–73), тот же паттерн.

**Контракт, который НЕ должен измениться (D-05 risk):** debounce поиска (`ActsSearchAndTabs` строки
19–35 — 250 мс + guard `document.activeElement?.id`), значения счётчиков, переключение вкладок.
Логику `<script>` не трогаем — меняется только разметка табов и удаляется bespoke `.tab`-CSS.
`Input` уже на месте — оставить.

---

### D-03: списки → семантика `<table>` + `Table`/`TableRow`

**Аналог (эталон):** `ui/src/features/devices/DeviceList.svelte` (обёртка) + `DeviceListRow.svelte` (строки)
**Применить к:** `ActsList`+`ActListRow`, `CartridgesList`+`CartridgeListRow`, `ModelsList`+`ModelListRow`,
`PrintersList`+`PrinterListRow`

**Структурный сдвиг:** текущие списки — НЕ таблицы. `ActListRow` (строки 52–76) — это
`<div class="row">` в **две строки** (номер/дата сверху, получатель/кол-во снизу), `<div class="rows">`
скроллер + bespoke `.pagination` футер (`ActsList` строки 96–114, `<style>` 117–164). Переносим на
плоскую табличную семантику как у Устройств (групп, как в `DeviceList`, здесь НЕТ — раскладка плоская).

**Обёртка списка — копировать форму из `DeviceList.svelte` строки 62–120:**
```svelte
{#snippet tableHead()}
  <th class="th-number">№</th>
  <th>Дата</th>
  <th>Получатель</th>
  <th class="th-count">Позиций</th>
{/snippet}

{#snippet footer()}
  {#if !skeletonLoading && !isEmpty}
    <footer class="list-footer">
      <span class="pagination-info">Показано {items.length} из {total}</span>
    </footer>
  {/if}
{/snippet}

<Table
  columns={4}
  loading={skeletonLoading}
  empty={isEmpty}
  emptyTitle={emptyMessage}
  emptyBody={emptySubtext}
  head={tableHead}
  {footer}
/>
```
`Table` уже владеет border/radius(8px)/`--tr-elev-1`/overflow-x (строки 86–91), skeleton (52–63) и
empty-state (64–72) — bespoke `.loading`/`.empty`/`.pagination` из `ActsList` **удаляются целиком**,
их роль забирает `Table`. `emptyConfig`-логику (`ActsList` строки 38–69) сохранить — она питает
`emptyTitle`/`emptyBody`.

**Строка — копировать форму из `DeviceListRow.svelte` строки 51–70:**
```svelte
<TableRow selected={act.id === selectedActId} class="...">
  <td class="cell cell-number"><span class="tr-mono">№{act.number}</span></td>
  <td class="cell">{dateLabel}</td>
  <td class="cell">{act.receiver_name}</td>
  <td class="cell cell-count">{itemsCount}</td>
</TableRow>
```
Клик-на-строку (сейчас `role="button"`/`onclick`/`onkeydown` на `.row`, `ActListRow` 52–60) переносится
на `<TableRow>`: `TableRow` принимает `selected` (даёт фон `--tr-row-selected` + 3px accent-бордер,
строки 90–93) и `class` pass-through. `onSelect` вешаем на `<TableRow>` через `class`/обёртку —
селект-состояние теперь через проп `selected`, не через bespoke `.row.selected` (`ActListRow` 98–102).

**Своя колоночная специфика по окнам (Claude's Discretion — сопоставление колонок):**
- **Acts:** № (`tr-mono`, tabular-nums) · дата · получатель · кол-во позиций · (badge «В архиве» при
  `activeTab==='archive'` — вместо строки 65–69 текущего `ActListRow`).
- **Cartridges/Models:** статус-`Badge` (маппинг status_id→variant уже есть в `CartridgeDetail` строки
  24–34 и `DeviceListRow` 40–48 — переиспользовать) · код (`tr-mono`) · модель.
- **Printers:** имя · IP (`font-variant-numeric: tabular-nums`) · статус-`Badge` · **`TonerGauge`
  инлайном в ячейке** (тонер-колонка).

**Ячейка `.cell` — копировать truncate-паттерн из `DeviceListRow.svelte` строки 84–91:**
```scss
.cell {
  font-size: var(--tr-font-size-body);
  color: var(--tr-text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 0; // makes text-overflow work in table cells
}
```

**Caution (D-03):** `Table`/`TableRow` — общие компоненты. Существующие потребители — Устройства
(`DeviceList`/`DeviceListRow`/`DeviceGroupRow` — **не трогать**, D-12 Фазы 26), витрина, и
`ActFormItemsTable` (уже потребитель — не ломать). Новые колонки не должны менять их поведение;
`minWidth` на `Table` — опциональный проп (`DeviceList` ставит `"860px"`), для узких master-панелей
задавать своё значение или не задавать вовсе. **Ловушка `:global()`** (см. Shared Patterns): любой
CSS, целящий в caller-`<td>` внутри `TableRow`, требует формы `.tr-row :global(> td)` — см.
`TableRow.svelte` строки 106–113 и `DeviceListRow.svelte` строки 72–82.

---

### D-01: детальные панели → лёгкий общий паттерн + ре-токенизация

**Прецедент формы извлечения:** `ui/src/lib/components/PageHeader.svelte` — как в Фазе 26 общий
кусок шапки вынесли в один компонент с `title` + `actions: Snippet`.
**Применить к:** `ActDetail.svelte` (201), `CartridgeDetail.svelte` (334), `PrinterDetail.svelte` (603)

**Общий словарь трёх панелей (это и есть кандидат на извлечение):** сравнение `<style>`-блоков
показывает, что все три повторяют один набор классов почти дословно:

| Общий элемент | ActDetail | CartridgeDetail | PrinterDetail |
|---------------|-----------|-----------------|---------------|
| Контейнер+скролл | `.act-detail` 123–128 | `.cartridge-detail` 207–212 | `.printer-detail` 394 |
| Loading/Empty блок | `.loading,.empty` 130–151 | 214–237 | 400–423 |
| Шапка | `.detail-header`+`.detail-title`+`.actions` 153–172 | `.detail-header`+`.title-row`+`.actions` 239–269 | `.detail-header`+`.title-row` 425–451 |
| Секция | `.section`+`.section-heading` 174–182 | 271–280 | `.detail-section`+`.section-heading` 461–480 |
| Field-grid | `.header-grid` (2col) 184–188 | `.fields-grid`+`.field`+`.field-label`+`.field-value` 282–306 | inline поля |
| История/список | `.returns-list` 190–195 | `.history-list`+`.history-row` 315–333 | `.readings-list`+`.reading-row` 541–577 |

**Empty-state блок — идентичен во всех трёх, вынести первым (копировать `ActDetail.svelte` 61–66 + CSS 130–151):**
```svelte
<div class="empty">
  <h2 class="empty-heading">Выберите акт</h2>
  <p class="empty-body">Выберите акт слева, чтобы увидеть подробности, или создайте новый.</p>
  <Button variant="primary" onclick={onCreate}>+ Создать акт</Button>
</div>
```
```scss
.loading, .empty {
  display: flex; flex-direction: column; align-items: center; justify-content: center;
  gap: var(--tr-space-md); min-height: 320px; text-align: center; color: var(--tr-text-secondary);
}
```

**Шапка детали — общий паттерн (копировать `ActDetail.svelte` 68–86 + CSS 153–172):**
```svelte
<header class="detail-header">
  <h2 class="detail-title">…</h2>
  <div class="actions"><!-- Button×N --></div>
</header>
```

**Field-grid — общий (копировать `CartridgeDetail.svelte` 163–186 + CSS 282–306):**
```svelte
<div class="fields-grid">
  <div class="field">
    <span class="field-label">Расположение</span>
    <span class="field-value">{cartridge.location ?? '—'}</span>
  </div>
</div>
```
`ActHeaderField.svelte` (36 строк) — уже маленький field-виджет с `label`/`value`; общий паттерн
D-01 должен либо поглотить его, либо согласоваться с ним (Claude's Discretion — компонент vs snippets).

**Действие для планировщика (D-01):** решить форму (новый компонент в `ui/src/lib/components/`,
напр. `DetailPanel.svelte` со `Snippet`-слотами header/sections — по образцу `PageHeader`; ИЛИ набор
общих классов/snippets). Разместить как разделяемый артефакт **первой волной** (D-19 Фазы 26).
**Поля и структура секций каждой панели НЕ меняются (SC #4)** — `PrinterDetail` с его `.counter-row`/
`.compat-agg-row`/`.meta-row` (строки 487–596) сохраняет все свои секции, меняется только их одежда.
`PrinterDetail` содержит async-загрузку readings/aggregates (`<script>` строки 38–120) — это данные,
не трогать, только визуал.

---

### D-02: master-detail → `--tr-surface-raised` + рамка (закрытие регресса D-13)

**Аналог языка поверхности:** `ui/src/lib/components/Table.svelte` строки 86–91 (framed-обёртка) +
карточки дашборда Фазы 26.
**Применить к:** `ActsMasterDetail.svelte`, `CartridgesMasterDetail.svelte`, `PrintersMasterDetail.svelte`
(три файла идентичны — `diff` показывает расхождение только в комментариях).

**Текущее (регресс D-13):** master на `--tr-surface`, detail на `--tr-bg` (`ActsMasterDetail.svelte`
строки 31–45). После D-06 Фазы 26 контент-область стала `--tr-surface` → панели сливаются с фоном.

**Целевая правка обеих панелей (`.master` и `.detail`):**
```scss
.master, .detail {
  background: var(--tr-surface-raised);
  border: 1px solid var(--tr-border);
  border-radius: var(--tr-radius-md);
  box-shadow: var(--tr-elev-1);
}
```
Grid 35/65 (строки 23–29) и `<1100px` horizontal-scroll fallback (строки 47–52) **сохраняются
дословно (SC #4)** — меняются только 2 свойства фона/тени на панель.

**Caution (обе темы, урок D-17):** проверено по `_tokens.scss` — в **светлой** теме
`--tr-surface-raised` (#ffffff строка 26) == `--tr-surface` (#ffffff строка 25), рамка+тень несут
всё разделение; в **тёмной** `--tr-surface-raised` (#1c222c строка 103) СВЕТЛЕЕ `--tr-surface`
(#161b23 строка 102) — панели «всплывают». Обязательный визуальный UAT в обеих темах.

**Согласовать с D-03/D-01:** `ActsList` сейчас сам красит фон `.acts-list { background: var(--tr-surface) }`
(строка 122) и `ActDetail` — `.act-detail { background: var(--tr-bg) }` (строка 128). После D-02 обёртка
даёт поверхность; внутренние фоны списка/детали убрать или согласовать, чтобы не было двойной заливки.

---

### D-04: полная ре-токенизация модалок и виджетов (без редизайна)

**Аналоги:** уже мигрированные `devices/DeviceFormModal.svelte`/`DeviceFormBody.svelte`,
`devices/DeviceContextMenu.svelte`, `ActionMenu.svelte`, и близнецы-баннеры между собой.
**Применить к:** внутренности `OperationModal` (887), `ModelFormModal` (580), `CartridgeFormBody`,
`CompatibilityEditor`, `CartridgeContextMenu`, `LowStockBanner`, `TonerGauge`, `PrinterAlertBanner`,
`DiscoveryResultsTable`, form-internal таблицы Актов.

Чрома модалок готова (все 9 на `Modal`), контролы на `Input`/`Select`. Остаётся внутренняя разметка
форм — переводится на токены/примитивы целиком, **функция и раскладка не меняются**.

**`DiscoveryResultsTable.svelte` — сырой `<table>` → `Table`/`TableRow` (частный случай D-03/D-04):**
Сейчас bespoke `<table class="results-table">` (строки 46–91) с ручными `th,td { padding … }` (CSS
119–156). Перенести на `Table` (columns=6, свой `head`-snippet с чекбоксом-select-all строки 49–57)
+ `TableRow` на каждую строку; `tr.duplicate` (color `--tr-text-tertiary`) → `class` pass-through на
`TableRow`. Пустое состояние (`.empty` строки 37–43) отдать `Table`-у (`emptyTitle`/`emptyBody`).
Чекбоксы заменить на `Checkbox`-примитив.

**Баннеры-близнецы `LowStockBanner` ↔ `PrinterAlertBanner`:** уже почти на токенах и идентичны по
структуре (warning SVG + `color-mix(in srgb, var(--tr-warning) 10%, transparent)` фон + `--tr-warning`
бордер). `LowStockBanner` строки 50–95 == `PrinterAlertBanner` строки 65–102 по CSS. Аудит на остаточные
не-токен значения; `color-mix`-фон — допустимый паттерн (встречается и в живом `DeviceFilters`).

**`TonerGauge.svelte`:** уже целиком на токенах (`--tr-accent`/`--tr-warning`/`--tr-danger`/
`--tr-surface`/`--tr-border`, строки 61–97). Только аудит: подтвердить 0 остаточных hardcode; порог-
цвета `<script>` 29–37 — логика, не трогать.

**Caution:** `check-tokens.mjs` — closed-world гейт (обжигалась Фаза 24): любой `var(--tr-*)`, которого
нет в `_tokens.scss`, роняет сборку. Сверять имена. `:global()` в plain `.scss` не работает (урок
Фазы 24) — внутри `.svelte`-scoped стилей ок.

---

## Shared Patterns

### Общий паттерн детальной панели (новый артефакт — D-01)
**Прецедент формы:** `ui/src/lib/components/PageHeader.svelte` (Snippet-слоты + одна ответственность).
**Применить к:** все три `*Detail.svelte`, переиспользуется Фазой 28 (ещё 4 окна с деталями).
**Где живёт:** `ui/src/lib/components/` (Claude's Discretion — компонент или snippets/классы).
Первая волна (D-19: общие файлы раньше окон).

### Примитив `Tabs` + строковый адаптер (D-05)
**Источник:** `ui/src/lib/components/Tabs.svelte` (`variant="underline"` со встроенным `count`),
эталон применения — `devices/DeviceFilters.svelte` строки 61–100.
**Применить к:** все три `*SearchAndTabs`.

### Обёртка `Table` (рамка/футер/skeleton/empty) (D-03)
**Источник:** `ui/src/lib/components/Table.svelte` (framed 86–91, skeleton 52–63, empty 64–72) +
`TableRow.svelte` (`selected` 90–93, base-`td` через `:global(> td)` 108–113).
**Применить к:** 4 master-списка + `DiscoveryResultsTable`.
**Ловушка `:global()` (контракт, не стиль):** целясь в caller-`<td>`, писать `.tr-row :global(> td)`
(внешняя часть в своём scope), НЕ `:global(.tr-row > td)` — вторая форма проигрывает по специфичности
(`TableRow.svelte` комментарий строки 102–107; живой пример `DeviceListRow.svelte` 72–82).

### Поверхность `--tr-surface-raised` + `--tr-elev-1` (D-02)
**Источник:** `Table.svelte` framed-обёртка (86–91), карточки дашборда Фазы 26.
**Применить к:** обе панели трёх `*MasterDetail`. **Проверять обе темы** (в светлой raised==surface,
в тёмной — светлее).

### Маппинг статус → `Badge` variant
**Источник:** `CartridgeDetail.svelte` 24–34, `DeviceListRow.svelte` 33–48 (`STATUS_LABELS`/
`STATUS_VARIANTS` records).
**Применить к:** списки/детали Картриджей и Принтеров (статус-колонки D-03, шапки деталей D-01).

### Svelte 5 runes контракт
**Источник:** любой мигрированный компонент (`$props`, `$bindable`, `$derived`, `Snippet`).
**Ловушка Фазы 24:** `const` vs `let` при `$bindable()` — контракт, не стилистика (`Tabs.svelte`
строка 17 использует `let … = $bindable()`).

---

## No Analog Found

| Файл/артефакт | Роль | Причина | Что делать |
|---------------|------|---------|-----------|
| Общий компонент детальной панели (D-01) | shared component | В `ui/src/lib/components/` нет готовой детальной панели — это НОВЫЙ разделяемый артефакт | Извлечь по прецеденту `PageHeader.svelte`; форма — Claude's Discretion; первая волна |

Все остальные 41 файла имеют внутренний прецедент (Устройства/примитивы/близнецы) — «no analog»
ограничивается единственным новым общим паттерном D-01.

---

## Metadata

**Analog search scope:** `ui/src/features/{acts,cartridges,printers,devices}`, `ui/src/lib/components`,
`ui/src/styles/_tokens.scss`
**Files scanned:** 41 целевых + 8 аналогов/примитивов прочитано целиком; `_tokens.scss` (токены D-02) сверены
**Порядок волн (D-19):** 1) общий паттерн детали D-01 + правки под колонки `Table`/`TableRow`;
2) три окна параллельно (Акты / Картриджи / Принтеры) — чтобы волны не дрались за общие файлы.
**Pattern extraction date:** 2026-07-21
