# Phase 28: Окна поддержки и администрирования — Pattern Map

**Mapped:** 2026-07-22
**Files analyzed:** 24 `.svelte` across 4 windows (Заявки 6, Отчёты 5, Настройки 8 + 1 route-orchestrator, Пользователи 4)
**Analogs found:** 24 / 24 (every file has a byte-close precedent already migrated in Фазы 26–27 — this phase repeats a proven playbook, it does not invent one)

> Фаза чисто визуальная (SC #1–4): **поля, действия, workflow, API не меняются**. Всё ниже —
> ре-токенизация и перевод на примитивы. Гейт `check-tokens.mjs` роняет сборку на несуществующем
> токене — все имена токенов ниже уже проверены по `ui/src/styles/_tokens.scss` (используются
> дословно в уже-мигрированных Фазой 26/27 файлах, которые я прочитал).

---

## File Classification

### Заявки (WIN-06) — структурно идентичны Фазе 27, плейбук применяется 1:1

| Файл | Роль | Data Flow | Ближайший аналог | Качество |
|------|------|-----------|-------------------|----------|
| `requests/RequestsMasterDetail.svelte` | layout (master-detail) | container | `acts/ActsMasterDetail.svelte` (уже на D-02) | **exact** |
| `requests/RequestsSearchAndTabs.svelte` | filter-bar | request-response (tabs, no debounce — no search input here) | `acts/ActsSearchAndTabs.svelte` (уже на `Tabs`) | exact (минус debounce-часть — Заявки не имеют поиска здесь) |
| `requests/RequestsList.svelte` | list | CRUD (list+empty+footer) | `acts/ActsList.svelte` (уже на `Table`) | exact |
| `requests/RequestListRow.svelte` | list-row | transform | `acts/ActListRow.svelte` (уже на `TableRow`) | role-match (Заявки — 4 колонки другие: тип/статус/автор/дата, не №/дата/получатель/кол-во) |
| `requests/RequestDetail.svelte` | detail-panel | transform (+ много модалок/lifecycle-кнопок) | `acts/ActDetail.svelte` + `printers/PrinterDetail.svelte` (оба уже на `DetailPanel`) | role-match — заголовок сложнее (badges+meta-row), см. Pattern Assignment ниже |
| `requests/RequestFormModal.svelte` | modal (form) | CRUD | уже на `Modal`/`Select`/`Textarea`; частично ре-токенизирована (см. ниже) | role-match |
| `requests/RequestsPage.svelte` | page (orchestrator) | container | `acts/ActsPage.svelte` (уже на `PageHeader`) | role-match — **сейчас bespoke `<header class="page-header">`, не мигрирован на `PageHeader`** (см. «Находка вне явного списка CONTEXT.md» ниже) |

### Отчёты (WIN-07) — реальная серая зона (D-06 nav, D-07 table), решена в UI-SPEC

| Файл | Роль | Data Flow | Ближайший аналог | Качество |
|------|------|-----------|-------------------|----------|
| `reports/ReportSubNav.svelte` | sub-nav (2 уровня) | event-driven | `Tabs.svelte` напрямую (`variant="underline"` + `"segmented"`) — **не форк, примитив уже покрывает** (UI-SPEC §6.1) | exact-after-adoption |
| `reports/PeriodSelector.svelte` | segmented switch + selects | event-driven | `Tabs.svelte` (`segmented`, `disabled` per-tab) + `Select.svelte` для месяц/год | exact-after-adoption |
| `reports/ReportFilters.svelte` | button-bar | event-driven | **уже целиком на `Button`** (GAP-R4 убрал все фильтры-поля) — просто re-audit, вероятно 0 правок | keep |
| `reports/ReportTable.svelte` | table (dynamic columns) | transform (batch render + group-separator) | `Table.svelte`/`TableRow.svelte` (динамический `head`/`children` snippet уже поддерживает произвольные колонки) | role-match — разделитель-строка НЕ через `TableRow group` (см. Pattern Assignment) |
| `reports/ReportsPage.svelte` | page (orchestrator) | container | `acts/ActsPage.svelte` (`PageHeader`) | role-match — **тоже bespoke `<header class="page-header">`** (см. находка ниже) |

### Настройки (WIN-08) — суб-нав серая зона решена (Tabs, без расширения); панели — чистый D-04

| Файл | Роль | Data Flow | Ближайший аналог | Качество |
|------|------|-----------|-------------------|----------|
| `settings/SettingsSubNav.svelte` | sub-nav (7 разделов, 1 уровень) | event-driven | `Tabs.svelte` (`variant="underline"`) | exact-after-adoption |
| `settings/NetworkSettings.svelte` | form panel | CRUD (settings get/set) | уже на `Button`; raw `<input type="checkbox">` для toggle | role-match, D-04 |
| `settings/OrgSettings.svelte` | form panel (+file upload) | CRUD | уже на `Button`; raw `<input class="form-input">` | role-match, D-04 |
| `settings/StorageSettings.svelte` | form panel (+confirm modal) | CRUD | уже на `Button`+`Modal` | role-match, D-04 (маленький — 203 стр) |
| `settings/BackupSettings.svelte` | form panel | CRUD | уже на `Button`; `.folder-code` = bare `font-family: monospace` → `--tr-text-mono` (UI-SPEC §9.3, обязательное) | role-match, D-04 |
| `settings/ThresholdSettings.svelte` | form panel (1 поле) | CRUD | raw `<input type="number">` + `<label>` — простейшая панель (120 стр) | role-match, D-04 |
| `settings/ActiveDirectorySettings.svelte` | form panel | CRUD | уже на `Button`; raw `<input>`/`<input type="checkbox">` (несколько disabled read-only полей) | role-match, D-04 |
| `settings/TemplateEditor.svelte` | form panel + code editor (462 стр) | CRUD + file-I/O (preview iframe) | уже на `Button`+`Modal`; raw `<select class="form-select">` для выбора kind | role-match, D-04 — **область `textarea`/превью НЕ трогать** (D-08) |
| `pages/SettingsPage.svelte` (route file, НЕ re-export — см. находка ниже) | page (orchestrator) | container | `acts/ActsPage.svelte` (`PageHeader`) | role-match — bespoke `<header class="page-header">`, живёт в другом каталоге чем остальные три Page |

### Пользователи (WIN-09) — простейшее окно, без master-detail

| Файл | Роль | Data Flow | Ближайший аналог | Качество |
|------|------|-----------|-------------------|----------|
| `users/UsersList.svelte` | list (raw `<table>`) | CRUD | `acts/ActsList.svelte` (уже на `Table`) — но проще: без пагинации, без loading-скелетона | role-match |
| `users/UserListRow.svelte` | list-row (raw `<tr>`, bespoke `.badge`) | transform | `acts/ActListRow.svelte` (уже на `TableRow`) + `Badge.svelte` для статуса | role-match |
| `users/UserFormModal.svelte` | modal (form) | CRUD | `users/UserFormModal.svelte` уже на `Modal`+`Button`; raw `<input>`/`<select>`/`<input type=checkbox>` внутри | role-match, D-04 — использовать `devices/DeviceFormBody.svelte` как эталон `Input`/`Select`+field-error разметки |
| `users/UsersPage.svelte` | page (orchestrator) | container | `acts/ActsPage.svelte` (`PageHeader`) | role-match — bespoke `<header class="page-header">` |

---

## Находка вне явного списка CONTEXT.md: 4 page-orchestrators всё ещё bespoke `<header class="page-header">`, НЕ на `PageHeader`

Проверено чтением: `RequestsPage.svelte` (строка 222), `ReportsPage.svelte` (423), `UsersPage.svelte`
(108), `pages/SettingsPage.svelte` (16) — все четыре держат собственный
`<header class="page-header"><h1 class="page-title">…</h1></header>`, а НЕ `PageHeader.svelte`
(извлечён Фазой 26, D-07; уже используется `DevicesPage.svelte` строка 231, `ActsPage.svelte`
строка 254, и аналогично Cartridges/Printers). CONTEXT.md перечисляет эти файлы в «Код, который
меняется (по окнам)» (`RequestsPage`, `ReportsPage`, `UsersPage`), но не называет причину явно —
скорее всего это тот самый компонентный разнобой, который SC #1–4 просит устранить («консистентность
компонентов поверх токенизированной поверхности»). Планировщику: включить перевод всех 4
page-заголовков на `PageHeader` (title-проп + `actions`-snippet при наличии кнопок в шапке) в
соответствующие волны — это тот же паттерн, что D-07 Фазы 26, только раньше пропущенный для этих
четырёх окон.

**Отдельная структурная особенность:** `SettingsPage.svelte` — **не** тонкий re-export как остальные
(`ui/src/pages/{RequestsPage,ReportsPage,UsersPage}.svelte` — 5–6-строчные обёртки над
`features/{requests,reports,users}/*Page.svelte`); он живёт напрямую в `ui/src/pages/SettingsPage.svelte`
(48 строк, содержит `SettingsSubNav` + переключение 7 панелей). Нет отдельного
`features/settings/SettingsPage.svelte` — это и есть файл окна Настроек, планировщик должен
адресовать правки по этому пути, а не искать эквивалент в `features/settings/`.

**Аналог для миграции (копировать форму из `acts/ActsPage.svelte` строки 254–258):**
```svelte
import PageHeader from '$lib/components/PageHeader.svelte';
...
<PageHeader title="Заявки">
  {#snippet actions()}<!-- кнопка «Создать заявку», если в шапке -->{/snippet}
</PageHeader>
```

---

## Pattern Assignments

### D-01 (Заявки): `RequestDetail.svelte` → `DetailPanel`/`DetailSection`/`DetailField`

**Аналоги:** `acts/ActDetail.svelte` (простой заголовок) + `printers/PrinterDetail.svelte` (заголовок
со значком-бейджем под шапкой — **это решение подходит RequestDetail лучше**, см. ниже).

**Импорт (копировать `acts/ActDetail.svelte` строки 10–16):**
```svelte
import Button from '$lib/components/Button.svelte';
import Spinner from '$lib/components/Spinner.svelte';
import DetailPanel from '$lib/components/DetailPanel.svelte';
import DetailSection from '$lib/components/DetailSection.svelte';
import DetailField from '$lib/components/DetailField.svelte';
```

**Заголовок — серая зона, решена прецедентом `PrinterDetail`, не `ActDetail`:** `DetailPanel`'s
`title` — простая строка (`title?: string`), а `RequestDetail`'s текущий заголовок (строки 410–425)
несёт ДВА бейджа (тип + статус) плюс meta-row (автор/дата) — не влезает в строковый `title` как есть.
`PrinterDetail.svelte` уже решал ровно эту задачу (свой статус-бейдж под заголовком, не в `title`):
рендерит `<div class="title-badges"><Badge …/></div>` как ПЕРВЫЙ элемент внутри `DetailPanel`'s
`children`, сразу после `{#snippet actions()}` (см. `printers/PrinterDetail.svelte` строки 214–231).
**Решение для RequestDetail:** `panelTitle` = что-то простое (например заголовок заявки/её №, если
есть, иначе просто `typeLabel`), а бейджи (`typeLabel`/`statusLabel`) + meta-row (автор/дата) —
bespoke `<div class="title-row">…</div>` + `<div class="meta-row">…</div>`, скопированные ДОСЛОВНО
(текущие строки 410–424 текущего файла: `Badge variant="default">{typeLabel}`,
`Badge variant={statusVariant}>{statusLabel}`, `meta-item`×2) как первый контент внутри `children`,
рядом с `PrinterAlertBanner`-эквивалентной позицией у Printers. **Claude's Discretion (CONTEXT.md):**
точный состав секций — при переносе секции `.section`→`DetailSection`, `.fields-grid`→свой grid внутри
`DetailSection` (Field-grid паттерн уже есть — `27-PATTERNS.md` D-01, копировать 1:1), `.field`→
`DetailField label=... value={... ?? null}`.

**Empty state (копировать `acts/ActDetail.svelte` строки 71–79, значения — из текущего
`RequestDetail.svelte` строк 404–407):**
```svelte
<DetailPanel
  title={panelTitle}
  empty={request === null}
  emptyTitle="Выберите заявку"
  emptyBody="Выберите заявку слева, чтобы увидеть детали и историю."
>
```

**Loading state — сиблинг-ветка вне `DetailPanel` (копировать `acts/ActDetail.svelte` строки 65–69 +
CSS 132–145), НЕ трогать существующую логику loading в `RequestDetail`:**
```svelte
{#if loading}
  <div class="detail-loading" aria-live="polite">
    <Spinner size="md" />
    <span>Загрузка заявки…</span>
  </div>
{:else}
  <DetailPanel …>…</DetailPanel>
{/if}
```

**Действия (`actions` snippet) — многочисленные lifecycle-кнопки (Подтвердить/Принять в
работу/Выполнить/Отклонить/Удалить/Отменить) остаются функционально идентичными, переносятся в
`{#snippet actions()}` буквально как условные `<Button>` (образец — `acts/ActDetail.svelte` строки
80–97, там же видно паттерн `disabled`-обёртки через `<span title="…">`).**

**История (`.history-loading`/список) → `DetailSection heading="История"` + существующая
`{#each historyEntries}`-разметка без изменений внутри (пустое состояние «История пуста» — KEEP,
UI-SPEC §7.1).**

**Модалки внутри (`OperationModal`, reject/approve/delete/cancel confirm) — НЕ являются частью
`DetailPanel` API, остаются как отдельные `<Modal>`-вызовы вне `DetailPanel`, их внутренности
подпадают под D-04, не D-01.**

---

### D-02 (Заявки): `RequestsMasterDetail.svelte` → `--tr-surface-raised` + рамка

**Аналог (побайтово копировать):** `acts/ActsMasterDetail.svelte` — уже прошёл именно эту миграцию
Фазой 27. Разница текущего `RequestsMasterDetail` от него:
- `.master` сейчас `background: var(--tr-surface)`, `.detail` — `var(--tr-bg)` (регресс D-13) →
  ОБА на `var(--tr-surface-raised)` + `box-shadow: var(--tr-elev-1)` (уже есть `border`).
- `.master-detail` сейчас `min-height: calc(100vh - 240px)` (viewport-relative) — `ActsMasterDetail`
  заменил на `flex: 1 1 auto; min-height: 0;` (FIX B1) — **эта правка НЕ входит в D-02 буквально**
  (D-02 CONTEXT.md говорит «грид 35/65 сохраняется», не упоминает flex-fix), но так как
  `RequestsMasterDetail`/`RequestsList`/`RequestDetail` мигрируют на `Table`/`DetailPanel`
  (которые сами управляют внутренним скроллом — `fillHeight`), **скорее всего понадобится тот же
  flex-паттерн, иначе высота развалится** — Claude's Discretion при исполнении, сверить визуально.
- `.master { overflow: hidden }` уже совпадает с целевым; `RequestsList`/`RequestListRow` после D-03
  сами возьмут скролл через `Table fillHeight`.

**Копировать дословно (`acts/ActsMasterDetail.svelte` строки 35–48):**
```scss
.master, .detail {
  background: var(--tr-surface-raised);
  border: 1px solid var(--tr-border);
  border-radius: var(--tr-radius-md);
  box-shadow: var(--tr-elev-1);
}
```

**Caution (обе темы, урок D-17):** в светлой теме `--tr-surface-raised` == `--tr-surface` — граница+тень
несут разделение; в тёмной `--tr-surface-raised` СВЕТЛЕЕ `--tr-surface` — обязательный визуальный UAT
в обеих темах (см. `27-PATTERNS.md` D-02, сверено по `_tokens.scss`).

---

### D-05 (Заявки): `RequestsSearchAndTabs.svelte` → `Tabs`

**Аналог:** `acts/ActsSearchAndTabs.svelte` (уже мигрирован на `Tabs variant="underline"`).
**Ключевое отличие от Acts:** `RequestsSearchAndTabs` **не имеет поля поиска** (нет debounce/`Input`)
— только табы статуса + кнопка «Создать заявку». Взять из `ActsSearchAndTabs` только Tabs-часть, НЕ
искать несуществующий `Input`.

**Ключевой адаптер (текущий `TABS` уже `{key: StatusTab, label}` — нужен только `count`, которого
СЕЙЧАС НЕТ в `RequestsSearchAndTabs` вообще — свериться с UI-SPEC/CONTEXT, считает ли фаза добавление
счётчиков объёмом; текущий код не передаёт `counts`, поэтому либо оставить без `count` (Tabs
поддерживает `count?: number` как опциональный), либо — как D-06 делает для отчётов — не
изобретать нового поведения, раз в CONTEXT.md не зафиксировано):**

```svelte
import Tabs from '$lib/components/Tabs.svelte';

const tabItems = $derived(
  TABS.map((t) => ({ key: String(t.key), label: t.label })), // no count — RequestsSearchAndTabs currently has none
);

<Tabs
  variant="underline"
  tabs={tabItems}
  active={String(filter.status)}
  ariaLabel="Статус заявок"
  onchange={(key) => handleTabClick(key === 'null' ? null : (key as StatusTab))}
/>
```

**Строковый адаптер обязателен** — `StatusTab` включает `null` (для «Все»), `Tabs` требует
string-ключ (паттерн `String(tab.key)`/`key === 'null'` — идентичен `DeviceFilters.svelte` строки
61–65, см. `27-PATTERNS.md` D-05).

**Кнопка «Создать заявку» остаётся `<Button variant="primary">` рядом с `Tabs`, вне разметки табов**
(как сейчас) — не трогается.

---

### D-03 (Заявки + Пользователи): списки → `Table`/`TableRow`

**Заявки — аналог:** `acts/ActsList.svelte` + `acts/ActListRow.svelte` (уже на `Table`/`TableRow`).

**Копировать обёртку (`acts/ActsList.svelte` строки 90–139) — колонки другие:** RequestListRow
сейчас 2-строчная карточка (`.top`: type-badge + desc + status-badge; `.bottom`: author + date) →
переносится на плоскую таблицу, 4 колонки: **Тип** (badge) · **Описание/№** · **Автор** ·
**Статус** (badge) + relative-date как вторичный текст в одной из ячеек (Claude's Discretion —
финальная раскладка колонок, поля не меняются, только их таблично-колоночное расположение вместо
2-строчной карточки).

```svelte
{#snippet tableHead()}
  <th>Тип</th>
  <th>Описание</th>
  <th>Автор</th>
  <th class="th-status">Статус</th>
{/snippet}

<Table
  columns={4}
  loading={skeletonLoading}
  empty={isEmpty}
  emptyTitle={emptyConfig.heading}
  emptyBody={emptyConfig.body}
  head={tableHead}
  {footer}
  framed={false}
  fillHeight
>
  {#each items as r (r.id)}
    <RequestListRow request={r} selected={r.id === selectedId} onSelect={() => onSelect(r.id)} />
  {/each}
</Table>
```

**Строка (копировать форму `acts/ActListRow.svelte` строки 63–82 — `<TableRow selected>` +
`<td>`×N с `onclick` на каждой ячейке + `role="button"`/`onkeydown` на первой):**
```svelte
<TableRow {selected} class="request-row">
  <td class="cell" role="button" tabindex="0" onclick={handleClick} onkeydown={handleKeydown}>
    <Badge variant="default" size="sm">{typeLabel}</Badge>
    {#if isAdRestore}<Badge variant="warning" size="sm">Восстановление доступа</Badge>{/if}
  </td>
  <td class="cell" onclick={handleClick}>{shortDesc}</td>
  <td class="cell" onclick={handleClick}>{request.requesterName ?? '—'}</td>
  <td class="cell cell-status" onclick={handleClick}>
    <Badge variant={statusVariant}>{statusLabel}</Badge>
  </td>
</TableRow>
```
Существующая логика (`statusVariant`/`statusLabel`/`typeLabel`/`shortDesc`/`relativeDate`/
`isAdRestore`) в `<script>` не меняется — переносится разметка.

**Пользователи — аналог:** `acts/ActsList.svelte` тоже (упрощённая версия — без пагинации/loading-
skeleton, `UsersList` сейчас не имеет loading-состояния вообще):
```svelte
{#snippet tableHead()}
  <th>Логин</th><th>ФИО</th><th>Роль</th><th>Email</th><th>Статус</th>
  <th class="th-actions">Действия</th>
{/snippet}

<Table columns={6} empty={items.length === 0} emptyTitle="Пользователи не найдены" head={tableHead}>
  {#each items as user (user.id)}
    <UserListRow {user} {onEdit} {onDelete} />
  {/each}
</Table>
```
**§6.4 UI-SPEC — обязательное сужение:** использовать ТОЛЬКО `emptyTitle` («Пользователи не
найдены»), **НЕ задавать `emptyBody`** — второй строки в текущем UsersList нет, добавлять её — вне
объёма (SC #4, новый копирайт не изобретается).

**`UserListRow` — статус через `Badge`, не bespoke `.badge`:**
```svelte
import Badge from '$lib/components/Badge.svelte';
…
{#if user.is_active}
  <Badge variant="success">Активен</Badge>
{:else}
  <Badge variant="default">Заблокирован</Badge>
{/if}
```
(`Badge` variant=`success` даёт `color-mix(--tr-success 15%)` фон + `--tr-success` текст — совпадает
по роли со старым `.badge--active`; `default` для «Заблокирован», роль как `.badge--blocked`.)
Inline-подтверждение удаления (`confirmDelete`/«Удалить?»/«Да»/«Нет», строки 47–56) **сохраняется
дословно** — UI-SPEC §7.4 явно требует не заменять модалкой.

**Caution (D-03, обе окна):** `Table`/`TableRow` — общие компоненты, использующиеся Устройствами,
витриной, `ActFormItemsTable`, окнами Фазы 27. Новые колонки Заявок/Пользователей не должны менять
их поведение. `:global()`-ловушка (см. Shared Patterns ниже) обязательна к соблюдению в новых
`*ListRow`.

---

### D-04 (везде): ре-токенизация внутренностей модалок и виджетов

**Аналог для форм с `Input`/`Select`/error-паттерном:** `devices/DeviceFormBody.svelte` (строки
196–260) — `<div class="field" class:has-error>` + `<label class="label">` + `<Input>`/`<Select>` +
`{#if fieldErrors[...]}<p class="field-error">`.

**`RequestFormModal.svelte`** — уже частично на `Select`/`Textarea`/`GroupedPrinterSelect` (строки
6–12); проверить остаточные raw `<input>` (если есть) и `.field`-стили на предмет полного покрытия
токенами — низкий риск, largely done.

**`UserFormModal.svelte`** — самый явный пример «raw HTML внутри Modal-обёртки» (строки 129–210):
6 полей — `login`/`full_name`/`password`/`role`/`email`/`is_active` через raw `<input>`,
`<select class="form-select">`, `<input type="checkbox">`. Переносится на `Input`/`Select`/`Checkbox`
буквально по образцу `DeviceFormBody`:
```svelte
import Input from '$lib/components/Input.svelte';
import Select from '$lib/components/Select.svelte';
import Checkbox from '$lib/components/Checkbox.svelte';
…
<div class="form-field" class:has-error={loginErr !== null}>
  <label class="form-label" for="uf-login">Логин</label>
  <Input id="uf-login" value={form.login} invalid={loginErr !== null}
    disabled={saving || mode === 'edit'} oninput={(v) => (form.login = v)} />
  {#if loginErr}<span class="field-error">{loginErr}</span>{/if}
</div>
…
<Select id="uf-role" value={form.role} invalid={roleErr !== null} disabled={saving}
  onchange={(v) => (form.role = v)}>
  {#each roleOptions as opt}<option value={opt.value}>{opt.label}</option>{/each}
</Select>
```
Валидация (`loginErr`/`passwordErr`/`roleErr`, `validate()`) не трогается — только разметка полей.

**Настройки — 8 панелей, все уже на `Button`/(частично) `Modal`, остаётся замена raw controls:**
- `NetworkSettings.svelte`, `OrgSettings.svelte`, `ActiveDirectorySettings.svelte` — raw
  `<input type="checkbox">` toggle → `Checkbox.svelte`; raw `<input class="form-input">` (текстовые
  поля) → `Input.svelte`.
- `ThresholdSettings.svelte` (120 стр, простейшая) — 1 raw `<input type="number">` → `Input`
  (`type` в текущем `Input.svelte` не включает `"number"` в типе — **проверить**, `Input.svelte`
  Props: `type?: 'text' | 'number' | 'search'` — **`'number'` УЖЕ поддержан**, безопасно).
- `TemplateEditor.svelte` (462 стр) — `<select id="template-kind" class="form-select">` → `Select`;
  **`textarea`/код-область и iframe-превью НЕ трогать** (D-08, TemplateEditor уже на `Modal`+
  `Button variant="destructive"` для confirm-сброса — см. UI-SPEC §6.3, подтверждено кодом).
- `BackupSettings.svelte` — `.folder-code { font-family: monospace }` → заменить на
  `--tr-text-mono` (обязательное DS-03, UI-SPEC §9.3): `font: var(--tr-text-mono)` или явные
  `font-family`/`font-size`/`font-weight`/`font-variant-numeric` компоненты токена (сверить точное
  имя свойства в `_tokens.scss` — `check-tokens.mjs` завалит сборку на опечатке).
- `StorageSettings.svelte` (203 стр) — уже на `Button`+`Modal`, наименьший объём работы.

**Caution:** `check-tokens.mjs` closed-world гейт — сверять каждое имя `var(--tr-*)` по
`_tokens.scss` перед использованием.

---

### D-06 (Отчёты + Настройки): суб-навигация и сегментные переключатели → `Tabs`, БЕЗ расширения примитива

UI-SPEC §6.1 подтвердил кодом: `Tabs.svelte`'s встроенный `count`-слот уже даёт accent-тон активному
счётчику (`.tab.active .tab-count { background: var(--tr-accent-soft); color: var(--tr-accent-text) }`)
— дословно совпадает с текущим `<Badge variant={active?'accent':'default'}>` в `ReportSubNav`.
**Не форкать/не расширять `Tabs` — принять примитив как есть.**

**`ReportSubNav.svelte` → ДВА экземпляра `Tabs` рядом (аналог — сама конструкция уже похожа на
Tabs.svelte's `tabs-segmented`+`tabs-underline` семантику один-в-один):**
```svelte
import Tabs from '$lib/components/Tabs.svelte';

<div class="report-sub-nav">
  <Tabs
    variant="segmented"
    tabs={DOMAINS.map((d) => ({ key: d.key, label: d.label }))}
    active={activeDomain}
    ariaLabel="Домен отчётов"
    onchange={(key) => onDomainChange(key as DomainKey)}
  />
  <Tabs
    variant="underline"
    tabs={activeReports.map((r) => ({
      key: r.key,
      label: r.label,
      count: statusCounts ? (statusCounts[r.key] ?? 0) : (r.key === activeReport ? rowCount : 0),
    }))}
    active={activeReport}
    ariaLabel="Тип отчёта"
    onchange={onReportChange}
  />
</div>
```
`Badge` больше не импортируется в `ReportSubNav` — `count`-слот `Tabs` заменяет его целиком.
**Внимание:** текущий код показывает `'–'` (тире) для неактивных табов без `statusCounts` — `Tabs`'s
`count` типизирован `number`, не строка; либо всегда передавать `statusCounts` (обычно так и есть —
`reports_get_report_counts` уже используется), либо согласовать fallback на `0` вместо `'–'`
(минорное визуальное отличие — Claude's Discretion, зафиксировать в исполнении).

**`SettingsSubNav.svelte` → `Tabs variant="underline"`, без count:**
```svelte
import Tabs from '$lib/components/Tabs.svelte';

<Tabs
  variant="underline"
  tabs={SECTIONS.map((s) => ({ key: s.key, label: s.label }))}
  active={activeSection}
  ariaLabel="Раздел настроек"
  onchange={onSectionChange}
/>
```

**`PeriodSelector.svelte` `period-buttons` → `Tabs variant="segmented"`, с `disabled` при
`isSnapshot` (уже поддержано `Tab.disabled` в примитиве):**
```svelte
import Tabs from '$lib/components/Tabs.svelte';

<Tabs
  variant="segmented"
  tabs={MODES.map((m) => ({ key: m.key, label: m.label, disabled: isSnapshot }))}
  active={mode}
  ariaLabel="Режим периода"
  onchange={(key) => setMode(key as PeriodMode)}
/>
```
Месяц/год `<select class="period-select">` (строки 116–133) → `Select.svelte`; `DatePicker` уже на
месте (не трогать, UI-SPEC явно исключает).

**Контракт, который не должен измениться:** debounce/переключение домена-типа-раздела-режима,
значения счётчиков, ARIA (`tablist`/`aria-selected` для `underline`, `role="group"` для `segmented`
— `Tabs` уже расставляет корректно по варианту, см. `Tabs.svelte` строки 33–36, 46–54).

---

### D-07 (Отчёты): `ReportTable.svelte` → `Table`/`TableRow`, динамические колонки без расширения примитива

UI-SPEC §6.2 подтвердил кодом: `Table.svelte`'s `head`/`children` — оба `Snippet`, рендерят
произвольную разметку → динамический набор `Column[]` ложится напрямую:
```svelte
import Table from '$lib/components/Table.svelte';
import TableRow from '$lib/components/TableRow.svelte';

{#snippet tableHead()}
  {#each columns as col}<th>{col.label}</th>{/each}
{/snippet}

<Table columns={columns.length} {loading} empty={rows.length===0 && !loading}
  emptyTitle="Нет данных за выбранный период"
  emptyBody="Измените диапазон дат или выберите другой тип отчёта."
  head={tableHead}>
  {#each grouped as item}
    {#if 'type' in item && item.type === 'separator'}
      <tr class="report-separator" aria-hidden="true">
        <td colspan={columns.length}>{item.label}</td>
      </tr>
    {:else}
      {@const row = item}
      <TableRow>
        {#each columns as col}
          <td title={formatCellValue(row, col.key)}>{formatCellValue(row, col.key)}</td>
        {/each}
      </TableRow>
    {/if}
  {/each}
</Table>
```

**Разделитель-строка — НЕ через `TableRow group` (настоящая серая зона, решена UI-SPEC §6.2):**
голый `<tr class="report-separator">` внутри `Table`'s `children`-сниппета, рядом с обычными
`<TableRow>`. `group`-режим `TableRow` — контракт сворачивания (`groupExpanded`/`onToggleGroup`) с
chevron-иконкой; разделитель месяца/локации статичен и не сворачивается — форсировать в `group` было
бы либо лишними обязательными пропами, либо фиктивным `onToggleGroup`. Стили разделителя переносятся
**дословно** из текущего `.month-separator` (`ReportTable.svelte` строки 234–242):
```scss
.report-separator td {
  padding: var(--tr-space-2xs) var(--tr-space-md);
  height: var(--row-height-dense);
  background: var(--tr-surface-sunken);
  font-size: var(--tr-font-size-body);
  font-weight: var(--tr-font-weight-semibold);
  border-top: 1px solid var(--tr-border-strong);
  color: var(--tr-text-primary);
}
```

**Ошибка загрузки/error-state** (`error` prop, «Не удалось загрузить отчёт…») — не имеет прямого
эквивалента в `Table`'s loading/empty API (только 2 состояния: loading, empty) → остаётся
sibling-веткой ВНЕ `Table`, как loading в `RequestDetail`/`ActDetail` (см. D-01 паттерн выше):
```svelte
{#if error}
  <div class="state state-error"><p class="error-text">{error}</p></div>
{:else}
  <Table …>…</Table>
{/if}
```

**Footer/итоги:** текущий `ReportTable.svelte` НЕ имеет итоговой строки (только заголовок+данные) —
`Table`'s `footer`-слот НЕ требуется вводить «на будущее» (SC #4 — не изобретать функциональность,
UI-SPEC §6.2 явно это исключает).

---

## Shared Patterns

### `Tabs` — четыре новых потребителя, примитив НЕ расширяется (D-05/D-06)
**Источник:** `Tabs.svelte` (variant `underline`/`segmented`, встроенный `count`-slot с
accent-тоном на active — уже даёт то, что раньше давал `<Badge variant="accent">`).
**Применить к:** `RequestsSearchAndTabs` (перенос D-05 из Фазы 27), `ReportSubNav` (2 экземпляра —
`segmented` для домена + `underline` для типа), `SettingsSubNav` (`underline`), `PeriodSelector`
(`segmented`, per-tab `disabled` при `isSnapshot`).

### `Table`/`TableRow` — framed-обёртка/skeleton/empty/`:global()`-контракт (D-03/D-07)
**Источник:** `Table.svelte` (framed 92–97, `fillHeight` mode 99–124, skeleton 165–183, empty
195–211) + `TableRow.svelte` (`selected` 90–93, base-`td` через `:global(> td)` 107–113).
**Применить к:** `RequestsList`+`RequestListRow`, `UsersList`+`UserListRow`, `ReportTable`
(динамические колонки, без `group`-режима — разделитель как голый `<tr>`).
**Ловушка `:global()` (контракт, не стиль, из `27-PATTERNS.md`):** целясь в caller-`<td>`, писать
`.request-row :global(> td)` / `.user-row :global(> td)`, НЕ `:global(.request-row > td)` —
специфичность проигрывает (`TableRow.svelte` комментарий строк 100–106).

### `DetailPanel`/`DetailSection`/`DetailField` — единственный потребитель этой фазы (D-01)
**Источник:** извлечены Фазой 27 (`ui/src/lib/components/{DetailPanel,DetailSection,DetailField}.svelte`).
**Применить к:** `RequestDetail.svelte` — единственная детальная панель среди 4 окон фазы.
**Заголовок со значками — используй прецедент `PrinterDetail.svelte`** (title-badges как первый
элемент `children`), НЕ `ActDetail.svelte` (простая строка title) — см. D-01 выше.

### `Badge` variant-маппинг для статусов
**Источник:** `Badge.svelte` variants (`default`/`accent`/`success`/`warning`/`destructive`).
**Применить к:** `UserListRow` (`success`=Активен, `default`=Заблокирован, взамен bespoke
`.badge--active`/`.badge--blocked`); `RequestListRow`/`RequestDetail` уже используют `Badge` —
только переносятся в новую разметку без изменения variant-логики.

### `PageHeader` — 4 page-orchestrators мигрируют (находка вне явного CONTEXT.md списка)
**Источник:** `PageHeader.svelte` (Фаза 26, D-07), эталон применения — `acts/ActsPage.svelte`
строки 254–258.
**Применить к:** `requests/RequestsPage.svelte`, `reports/ReportsPage.svelte`,
`users/UsersPage.svelte`, `pages/SettingsPage.svelte` (последний — НЕ re-export, реальный файл окна).

### `Input`/`Select`/`Checkbox` + field-error разметка (D-04)
**Источник:** `devices/DeviceFormBody.svelte` строки 196–260 (`.field`+`class:has-error`+`label`+
примитив+`{#if fieldErrors}`).
**Применить к:** `UserFormModal` (наибольший объём — 6 полей raw HTML), settings-панели (raw
`<input>`/`<select>`/checkbox), `RequestFormModal` (частично уже сделано — доверить финальный аудит).

### Svelte 5 runes контракт
**Ловушка Фазы 24:** `const` vs `let` при `$bindable()` — `Tabs.svelte` строка 17 (`active`)
использует `let … = $bindable()`; `Input`/`Select` аналогично (`value = $bindable('')`). Копировать
эту форму дословно при передаче `bind:` в новые места использования.

---

## No Analog Found

Ни один файл фазы не остался без прецедента — все 24 целевых файла (плюс 4 находки page-orchestrator)
имеют либо прямой Phase-27 эквивалент (Заявки/Пользователи), либо примитив уже покрывает целевой кейс
без расширения (Tabs — D-06, Table — D-07, подтверждено чтением кода в UI-SPEC §6.1/§6.2).

Единственная точка с неполной уверенностью — **`ReportSubNav`'s текущий fallback `'–'` для
неактивных табов без `statusCounts`**, который не имеет прямого эквивалента в `Tabs`'s
`count?: number` (строковый плейсхолдер не поддержан типом) — см. D-06 выше, решается на исполнении
(скорее всего `statusCounts` всегда присутствует на практике, тире-fallback можно уронить как
не-часто-достижимую ветку — не расширение примитива, просто разница в редком edge-case).

---

## Metadata

**Analog search scope:** `ui/src/features/{requests,reports,settings,users,acts,cartridges,printers,
devices}`, `ui/src/pages/{RequestsPage,ReportsPage,SettingsPage,UsersPage,ActsPage}.svelte`,
`ui/src/lib/components/{Tabs,Table,TableRow,DetailPanel,DetailSection,DetailField,Badge,Input,
Select,PageHeader}.svelte`
**Files scanned:** 24 целевых файла Фазы 28 + 4 route-shell `pages/*.svelte` + 11 аналогов/примитивов
прочитаны целиком или ключевыми диапазонами; `27-PATTERNS.md`/`27-CONTEXT.md`/`26-CONTEXT.md` сверены
для переносимых решений D-01…D-05.
**Порядок волн (рекомендация из CONTEXT.md Discretion, D-19 Фазы 26 / прецедент Фазы 27):**
1) любые правки, если понадобятся, к `Tabs`/`Table`/`TableRow` (в этой фазе не ожидается —
   UI-SPEC §6.1/§6.2 подтвердили: расширение НЕ требуется) — если планировщик всё же найдёт нужду,
   это первая волна;
2) четыре окна параллельно (Заявки/Отчёты/Настройки/Пользователи) — они не пересекаются по файлам,
   кроме общего чтения одних и тех же примитивов (не запись).
**Pattern extraction date:** 2026-07-22
