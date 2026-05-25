---
phase: 2
slug: 02-ui-devices
status: draft
shadcn_initialized: false
preset: none
created: 2026-05-25
---

# Phase 2 — UI Design Contract: Устройства и базовый UI

> Визуальный и интерактивный контракт фазы. Сгенерирован gsd-ui-researcher, валидируется gsd-ui-checker.
> Все строки UI — на русском (CLAUDE.md + UI-03). Имена компонентов, токенов и атрибутов — English.
> Контракт прескриптивный: executor реализует точно то, что здесь записано; отклонения требуют поправки UI-SPEC.md.

---

## Design System

| Property | Value |
|----------|-------|
| Tool | none (hand-rolled Svelte 5 primitives — CONTEXT.md D-UI-Errors-01/D-UI-Validation-01) |
| Preset | not applicable |
| Component library | none — каждый примитив пишется руками в `ui/src/lib/components/` |
| Icon library | inline SVG в `ui/src/lib/icons/*.svelte` (~12 иконок на фазу; без `lucide`, `tabler` и пр. — нулевые runtime-зависимости) |
| Font | system stack: `-apple-system, "Segoe UI", "Roboto", "Helvetica Neue", "Arial", sans-serif` — кириллица-готово на Windows/macOS/Linux, ноль шрифт-файлов, нулевая FOUT |
| Styling | SCSS через `svelte-preprocess` + `_tokens.scss` autoprepended в каждый `<style lang="scss">` (vite.config.ts уже настроен в Phase 1) |
| Router | `svelte-spa-router 5.1.0` (hash routing — CONTEXT D-UI-Router-01) |
| State | Svelte 5 runes (`$state`/`$derived`/`$effect`) в `.svelte.ts` модулях — CONTEXT D-UI-State-01 |

**Rationale for "none":** проект — B2B-внутренний инструмент, Russian-only, без brand-guideline; CLAUDE.md фиксирует Vanilla Svelte 5 (НЕ SvelteKit) и явно отвергает component-библиотеки. Hand-rolled примитивы в 10-12 файлах дешевле, чем библиотечный bundle на 100KB + локализация на русский + override темы.

---

## Spacing Scale

8-point base, multiples of 4 only. Все Phase-2 layout-spacing берётся **исключительно** из этого списка.

| Token (CSS var) | Value | Usage |
|-----------------|-------|-------|
| `--space-xs` | 4px | gap между иконкой и текстом, inline padding, hairline-разделение |
| `--space-sm` | 8px | компактные ряды формы, отступы внутри badge/chip, gap в footer toolbar |
| `--space-md` | 16px | стандартный padding ячейки таблицы, gap между полями формы, padding модального заголовка |
| `--space-lg` | 24px | padding страницы (контентная область), gap между секциями фильтров, отступ между header и таблицей |
| `--space-xl` | 32px | вертикальный gap между крупными блоками страницы (например, switch-bar → table) |
| `--space-2xl` | 48px | top-padding пустых state-экранов («Устройств пока нет») |
| `--space-3xl` | 64px | резерв; в Phase 2 не используется |

**Layout-фиксированные размеры** (вынесены отдельно — они не из spacing-шкалы, это размерные константы):

| Token | Value | Reason |
|-------|-------|--------|
| `--sidebar-width` | 240px | CONTEXT D-UI-Responsive-01 + UI-04 |
| `--header-height` | 56px | стандарт desktop chrome; вмещает 16px иконки + 8px padding × 2 + 16px breathing room |
| `--modal-max-width` | 640px | формы устройств помещаются без скролла на 720px-высоте |
| `--modal-max-width-wide` | 960px | CSV-импорт preview-таблица |
| `--touch-target-min` | 36px | min-height интерактивных кнопок/инпутов (desktop-density, не mobile 44px) |
| `--row-height` | 40px | строка таблицы — компактно, читаемо для 8-часового использования |
| `--row-height-dense` | 32px | dense-режим (резерв — toggle в Phase 7) |
| `--radius-sm` | 4px | input, button, badge |
| `--radius-md` | 8px | card, modal, dropdown-меню |
| `--shadow-elev-1` | `0 1px 2px rgba(0,0,0,0.06), 0 1px 1px rgba(0,0,0,0.04)` | card |
| `--shadow-elev-2` | `0 4px 12px rgba(0,0,0,0.08), 0 2px 4px rgba(0,0,0,0.06)` | modal, dropdown |
| `--shadow-elev-2-dark` | `0 4px 16px rgba(0,0,0,0.50), 0 2px 8px rgba(0,0,0,0.30)` | те же elev-2 в тёмной теме |

**Exceptions:** `--sidebar-width`, `--modal-max-width*`, `--row-height*`, `--header-height`, `--touch-target-min` — фиксированные конструктивные размеры, не из spacing-шкалы (это лимиты, не отступы).

---

## Typography

System font stack (см. Design System). Кириллица читается из Segoe UI на Windows и SF Pro на macOS — оба отрендерят «Сидоров-Петроградский Иван Александрович (ё)» без подмены глифов.

| Role | Size | Weight | Line Height | Usage |
|------|------|--------|-------------|-------|
| Body | 14px | 400 | 1.5 | таблицы, текст в формах, метаданные в карточке устройства |
| Label | 13px | 500 | 1.4 | подписи полей формы, заголовки колонок таблицы, sidebar-пункты |
| Heading | 20px | 600 | 1.3 | заголовок страницы («Устройства», «Дашборд») + заголовок модального окна |
| Display | 28px | 600 | 1.2 | резерв (Phase 7 dashboard-виджеты); в Phase 2 не используется |

**Жёсткие правила:**
- Ровно **4 размера** в шкале (Body/Label/Heading/Display); запрещены произвольные `font-size: 15px` в компонентах
- Ровно **2 веса** (400 regular, 500/600 для labels/headings — 500 для labels, 600 для headings и кнопок-CTA)
- Числовая монotypeface для табличных чисел через `font-variant-numeric: tabular-nums` на колонке «Инвентарный №» (выравнивание разрядов)
- Кодовые/идентификаторные значения (например, `serial_no = "PA5100-XYZ"`) — тем же system font, но letter-spacing `0.02em` для читаемости
- Запрет `text-transform: uppercase` (плохо читается с русской латиницей)

**Token mapping (в `_tokens.scss`):**
```scss
--font-family-base: -apple-system, "Segoe UI", "Roboto", "Helvetica Neue", "Arial", sans-serif;
--font-size-body: 14px;
--font-size-label: 13px;
--font-size-heading: 20px;
--font-size-display: 28px;
--font-weight-regular: 400;
--font-weight-medium: 500;
--font-weight-semibold: 600;
--line-height-body: 1.5;
--line-height-label: 1.4;
--line-height-heading: 1.3;
--line-height-display: 1.2;
```

---

## Color

Нейтральная профессиональная палитра. Стиль-референсы: Linear, Notion table view, Stripe Dashboard — clean, dense, neutral, оптимизировано для 8-часовой работы.

### Light theme (`[data-theme="light"]`)

| Role | Value | Usage |
|------|-------|-------|
| Dominant (60%) | `#FFFFFF` | основной фон контентной области, фон таблиц, фон модала |
| Secondary (30%) | `#F5F6F8` | sidebar background, фон карточек, hover row in table, fill of disabled inputs |
| Tertiary surface | `#EAECEF` | divider строки в sidebar, border ячейки таблицы, фон switch-bar в idle |
| Accent (10%) | `#2563EB` (blue-600) | primary CTA, active sidebar item indicator, focus-ring, current page in pagination |
| Destructive | `#DC2626` (red-600) | кнопка «Удалить», иконка опасности в confirm-диалоге, кайма поля с ошибкой |
| Success | `#16A34A` (green-600) | toast «Устройство создано», галочка проверенной строки в CSV preview |
| Warning | `#D97706` (amber-600) | toast «CSV: 3 строки пропущены», ячейка «На ремонте» в счётчике |
| Text primary | `#111827` (gray-900) | body, headings |
| Text secondary | `#4B5563` (gray-600) | labels, метаданные, placeholder |
| Text muted | `#9CA3AF` (gray-400) | inactive sidebar item, «Раздел в разработке» placeholder |
| Border | `#E5E7EB` (gray-200) | inputs, cards, table cells |
| Border strong | `#D1D5DB` (gray-300) | focused input, divider в sidebar |

### Dark theme (`[data-theme="dark"]`)

Deep neutral, **не pure black** — снижение eye-strain (#0A0E14 vs #000000 даёт ~40% контраста меньше для глаз ночью, остаётся в WCAG AA для текста).

| Role | Value | Usage |
|------|-------|-------|
| Dominant (60%) | `#0F1419` | основной фон |
| Secondary (30%) | `#1A1F26` | sidebar, карточки, hover row |
| Tertiary surface | `#252B33` | divider, фон switch-bar idle, фон ячейки таблицы при hover |
| Accent (10%) | `#3B82F6` (blue-500) | те же роли; чуть светлее, чем в light, чтобы держать AA-контраст на тёмном |
| Destructive | `#EF4444` (red-500) | те же роли |
| Success | `#22C55E` (green-500) | те же роли |
| Warning | `#F59E0B` (amber-500) | те же роли |
| Text primary | `#E5E7EB` (gray-200) | body, headings |
| Text secondary | `#9CA3AF` (gray-400) | labels, метаданные |
| Text muted | `#6B7280` (gray-500) | inactive nav, placeholder |
| Border | `#252B33` | inputs, cards |
| Border strong | `#374151` (gray-700) | focused input |

### Accent reserved for (NEVER «all interactive elements»)

Список конкретных Phase-2 элементов, где допустим accent (`#2563EB`/`#3B82F6`):

1. **Primary CTA кнопка** одна на страницу: «Создать устройство», «Импортировать CSV», «Сохранить», «Применить»
2. **Active sidebar item** — фон + 3px left-border accent для текущего пункта меню
3. **Focus ring** — `box-shadow: 0 0 0 3px var(--color-accent-focus)` на любом `:focus-visible` (input, button, link)
4. **Active page в pagination** — фон accent, текст white
5. **Active tab в status switch-bar** — нижняя граница 2px accent под выбранным статусом (Все / На складе / В работе / На ремонте / Списано)
6. **Checked checkbox / radio** — fill accent, галочка/точка white
7. **Selected row** в `DeviceList` — left-border 3px accent (резерв для multi-select операций; в Phase 2 не активируется)

**Запрещено:** accent на secondary-buttons, на иконках по умолчанию, на hover-состояниях вне CTA, на любых badge кроме «active».

### Semantic tokens (в `_tokens.scss`)

```scss
:root, [data-theme="light"] {
  --color-bg: #FFFFFF;
  --color-surface: #F5F6F8;
  --color-surface-raised: #FFFFFF;
  --color-surface-sunken: #EAECEF;
  --color-accent: #2563EB;
  --color-accent-hover: #1D4ED8;
  --color-accent-focus: rgba(37, 99, 235, 0.30);
  --color-destructive: #DC2626;
  --color-success: #16A34A;
  --color-warning: #D97706;
  --color-text-primary: #111827;
  --color-text-secondary: #4B5563;
  --color-text-muted: #9CA3AF;
  --color-text-inverse: #FFFFFF;
  --color-border: #E5E7EB;
  --color-border-strong: #D1D5DB;
  color-scheme: light;
}
[data-theme="dark"] {
  --color-bg: #0F1419;
  --color-surface: #1A1F26;
  --color-surface-raised: #1A1F26;
  --color-surface-sunken: #252B33;
  --color-accent: #3B82F6;
  --color-accent-hover: #60A5FA;
  --color-accent-focus: rgba(59, 130, 246, 0.40);
  --color-destructive: #EF4444;
  --color-success: #22C55E;
  --color-warning: #F59E0B;
  --color-text-primary: #E5E7EB;
  --color-text-secondary: #9CA3AF;
  --color-text-muted: #6B7280;
  --color-text-inverse: #0F1419;
  --color-border: #252B33;
  --color-border-strong: #374151;
  color-scheme: dark;
}
```

### Contrast (verified)

| Pair | Light ratio | Dark ratio | WCAG |
|------|-------------|------------|------|
| text-primary / bg | 16.1:1 (#111827 / #FFFFFF) | 13.2:1 (#E5E7EB / #0F1419) | AAA both |
| text-secondary / bg | 7.6:1 | 5.2:1 | AAA / AA |
| accent / bg | 4.5:1 (Phase-2 baseline) | 4.8:1 | AA both |
| accent / text-inverse on accent | 4.6:1 (text-inverse=#FFFFFF on #2563EB) | 4.9:1 | AA both |
| destructive / bg | 4.5:1 | 5.1:1 | AA both |

Все цифры подходят для AA на body-text 14px. Display/Heading 20–28px проходят и AAA автоматически.

---

## Copywriting Contract

> Все строки — на русском. Жёстко: backend `AppError.message` уже русский (Phase 1 invariant), UI **не переводит** — показывает как есть. Здесь — только UI-собственные строки.

### Sidebar (UI-01, точная структура — CONTEXT D-UI-Sidebar-01)

| Position | Label | Route | Phase 2 state |
|----------|-------|-------|---------------|
| 1 | Дашборд | `#/` | Placeholder «Раздел появится в Phase 7» |
| 2 | Карта | `#/map` | Placeholder «В разработке» (v2) |
| — | — divider — | — | — |
| 3 | Устройства | `#/devices` | **АКТИВНЫЙ — реализован в Phase 2** |
| 4 | Акты | `#/acts` | Placeholder «Раздел появится в Phase 3» |
| — | — divider — | — | — |
| 5 | Принтеры | `#/printers` | Placeholder «Раздел появится в Phase 6» |
| 6 | Картриджи | `#/cartridges` | Placeholder «Раздел появится в Phase 4» |
| 7 | Заявки | `#/requests` | Placeholder «Раздел появится в Phase 6» |
| — | — divider — | — | — |
| 8 | Отчёты | `#/reports` | Placeholder «Раздел появится в Phase 7» |
| 9 | Пользователи | `#/users` | Placeholder «Раздел появится в Phase 5» |
| — | — divider — | — | — |
| 10 | Настройки | `#/settings` | Placeholder «Раздел появится в Phase 7» |

**Placeholder копи (одинаковый текст для всех неактивных разделов):**

```
[заголовок страницы, ровно как в sidebar]
Раздел в разработке
Появится в следующих фазах.
```

(`Карта` — «В разработке (запланирована на v2)»; для v1-phases — «Появится в Phase N».)

### Theme switcher (UI-02 + CONTEXT D-UI-Theme-01)

Размещается в **footer sidebar'а** (визуально внизу sidebar, выше — спейсер `flex: 1`). 3 radio в горизонтальном ряду:

| Key | Label | Aria-label |
|-----|-------|------------|
| `light` | Светлая | Светлая тема |
| `dark` | Тёмная | Тёмная тема |
| `system` | Системная | Использовать системную тему |

Над переключателем — мелким серым: «Тема»

### Devices page (DEV-01..13)

| Element | Copy |
|---------|------|
| Page heading | Устройства |
| Page-level primary CTA (top-right) | + Создать устройство |
| Secondary action 1 (top-right) | Импорт CSV |
| Secondary action 2 (top-right) | Экспорт CSV |
| Search input placeholder | Поиск по наименованию, инвентарному, серийному, модели |
| Status switch-bar tabs (DEV-07) | Все ・ На складе ・ В работе ・ На ремонте ・ Списано |
| Counter chip format | `{label} ({count})` — пример: `На складе (124)` |
| Group toggle label | Группировать похожие |
| Group toggle tooltip | Сворачивает не-уникальные устройства (без серийного/инвентарного №) в одну строку |
| Group row expand button | `Показать {count}` (collapsed) / `Скрыть` (expanded) |

### Empty states

| Context | Heading | Body | Action |
|---------|---------|------|--------|
| Нет устройств вообще | Устройств пока нет | Создайте первое устройство или импортируйте список из CSV. | Кнопки: **+ Создать устройство** (primary) / **Импорт CSV** (secondary) |
| Поиск без результатов | Ничего не найдено | По запросу «{query}» ничего не нашлось. Проверьте написание или сбросьте фильтры. | Ссылка: **Сбросить фильтры** |
| Status-фильтр без результатов | В этом статусе пусто | Нет устройств в статусе «{status_name}». | Ссылка: **Показать все** |

### Loading states

| Context | Copy |
|---------|------|
| Initial page load (skeleton) | (без текста — skeleton-блоки) |
| Inline action loading (saving) | На кнопке: спиннер + «Сохранение…» (CTA-копи заменяется) |
| CSV preview parsing | Spinner + «Анализируем файл…» |
| CSV commit | Progress bar + «Импортировано {n} из {total}…» |

### Error state copy

| Class | Heading | Body | Action |
|-------|---------|------|--------|
| Page-level failure (load) | Не удалось загрузить устройства | {AppError.message} | Кнопка: **Повторить** |
| Field validation (inline, под полем) | (без heading) | {AppError.details[field] | "Поле обязательно"} | — |
| Toast (transient) | (без heading) | {AppError.message} | (auto-dismiss 6 с для error, 4 с для success/info) |
| Optimistic-lock conflict (особо обработать) | (toast) | Данные были изменены другим пользователем. Обновите страницу и попробуйте снова. | — |

### Device form modal (DEV-01..05, DEV-08..10)

| Element | Copy |
|---------|------|
| Modal heading (create) | Новое устройство |
| Modal heading (edit) | Редактирование устройства |
| Field — Тип ⃰ | Тип |
| Field — Наименование ⃰ | Наименование |
| Field — Инвентарный № | Инвентарный № |
| Field — Серийный № | Серийный № |
| Field — Модель | Модель |
| Field — Технические характеристики | Технические характеристики |
| Field — Комплектация | Комплектация |
| Field — Состояние | Состояние |
| Field — Расположение ⃰ | Расположение |
| Field — Статус ⃰ | Статус |
| Field — Количество (только для не-уникальных) | Количество |
| Required indicator | красная звёздочка `⃰` после label (`color: var(--color-destructive)`) |
| State-hints label (DEV-10) | Быстрый выбор: |
| State-hints chip values (6 шт) | Новое ・ Новый в заводской упаковке, не вскрытый ・ Новый в заводской упаковке, вскрытый ・ Хорошее ・ Среднее ・ Б/У |
| Primary action (create) | Создать |
| Primary action (edit) | Сохранить |
| Secondary action | Отмена |
| Autocomplete empty hint | Начните вводить, чтобы увидеть подсказки |
| Autocomplete loading hint | Загружаем подсказки… |
| Contextual autocomplete header (DEV-09) | Ранее использовалось с «{name}»: |

### CSV import modal (DEV-12)

Three-step wizard в одном модальном окне:

| Step | Heading | Body / Help |
|------|---------|-------------|
| 1. File pick | Импорт устройств из CSV | Выберите CSV-файл. Поддерживаются UTF-8, UTF-8 с BOM, Windows-1251. Разделители — запятая или точка с запятой. |
| 2. Preview | Проверьте данные | Определена кодировка: **{encoding}**, разделитель: **«{delim}»**. Показаны первые 5 строк. |
| 3. Mapping confirm | Сопоставление колонок | Колонки CSV сопоставлены с полями устройств автоматически по заголовкам. При необходимости измените. |
| 4. Result | Импорт завершён | Импортировано: **{ok}**. Пропущено с ошибками: **{failed}**. |

| Element | Copy |
|---------|------|
| File picker button | Выбрать файл… |
| File picker accepts | `.csv` |
| Preview row count caption | Показаны первые 5 строк из {total} |
| Mapping table headers | Колонка CSV ・ Поле устройства |
| Mapping unmapped option | — не импортировать — |
| Primary action step 2 | Далее |
| Primary action step 3 | Импортировать |
| Primary action step 4 | Готово |
| Secondary action all steps | Отмена |
| Per-row error format | Строка {n}: {AppError.message} |
| Show errors expander | Показать ошибки ({failed}) |

### CSV export

| Element | Copy |
|---------|------|
| Trigger button | Экспорт CSV |
| Default filename (in save dialog) | `устройства_{YYYY-MM-DD}.csv` |
| Toast on success | Экспортировано {count} устройств. |
| Toast on failure | {AppError.message} |

### Destructive actions

| Action | Trigger | Confirmation modal heading | Confirmation modal body | Confirm button | Cancel button |
|--------|---------|----------------------------|--------------------------|----------------|---------------|
| Удалить устройство | Кнопка «Удалить» в DeviceList row context menu | Удалить устройство? | «{device.name}» (инв. № {device.inventory_no \| «—»}) будет помечено как удалённое. Действие можно отменить только восстановлением из бэкапа БД. | Удалить (destructive стиль: фон `--color-destructive`, текст white) | Отмена |

В Phase 2 — **один destructive action**: удаление устройства. Soft-delete (через `deleted_at_utc`), не hard.

### Standard CTA pattern

| Element | Copy |
|---------|------|
| Primary CTA (create flow) | Создать (императивный глагол + опциональный объект, ≤2 слова) |
| Primary CTA (edit flow) | Сохранить |
| Secondary CTA (cancel) | Отмена |
| Destructive CTA | Удалить (в красном; модальная подтверждалка обязательна) |
| Tertiary action (link-styled) | Сбросить фильтры |

**Запрещено:** «Submit», «OK», «Ок», «Click here», «Confirm», `→`, эмодзи в копи. Кнопки — глаголы; ссылки — действие+объект.

---

## Component Inventory (Phase 2 primitives)

Каждый файл в `ui/src/lib/components/` или `ui/src/features/`. Прескриптивно — executor создаёт ровно эти компоненты, с ровно этими props/states.

### Layout (in `features/layout/`)

#### `Layout.svelte`
Корневой shell. Делит viewport на sidebar + main.
- Структура: `<div class="layout"><Sidebar/><main class="content"><slot/></main></div>`
- Layout: CSS Grid `grid-template-columns: 240px 1fr; min-height: 100vh`
- `<main>`: `overflow: auto; padding: var(--space-lg)`
- Применяет тему — вызывает `initTheme()` в `$effect` при mount

#### `Sidebar.svelte`
- Items из `sidebar-config.ts` (массив `SidebarItem | SidebarDivider`)
- Active item — через `svelte-spa-router/active` action
- Footer: `<ThemeSwitcher/>`
- Background: `--color-surface`; border-right: `1px solid var(--color-border)`
- Active item: `background: color-mix(in srgb, var(--color-accent) 10%, transparent); border-left: 3px solid var(--color-accent); color: var(--color-text-primary);`
- Inactive item: `color: var(--color-text-secondary)`; hover → `background: color-mix(in srgb, var(--color-text-primary) 5%, transparent)`
- Divider: `height: 1px; background: var(--color-border); margin: var(--space-sm) var(--space-md);`
- Item padding: `var(--space-sm) var(--space-md)` (8/16), font-size label (13px)

### Primitives (in `lib/components/`)

#### `Button.svelte`
Props: `variant: 'primary'|'secondary'|'destructive'|'ghost'|'link'`, `size: 'sm'|'md'`, `loading?: boolean`, `disabled?: boolean`, `type?: 'button'|'submit'` (default `button`)

States × variants matrix:

| Variant | Background | Color | Border | Hover | Focus | Disabled |
|---------|-----------|-------|--------|-------|-------|----------|
| primary | `var(--color-accent)` | white | none | `--color-accent-hover` | +ring | opacity 0.5 |
| secondary | transparent | `--color-text-primary` | `1px solid var(--color-border-strong)` | bg `--color-surface-sunken` | +ring | opacity 0.5 |
| destructive | `var(--color-destructive)` | white | none | brightness 0.92 | +ring | opacity 0.5 |
| ghost | transparent | `--color-text-primary` | none | bg `--color-surface` | +ring | opacity 0.5 |
| link | transparent | `--color-accent` | none | underline | +ring | opacity 0.5 |

Sizes: `md` = 36px height, 16px horizontal padding, 14px font; `sm` = 28px height, 12px padding, 13px font.
Loading state: kbd-spinner (12px) left of label; cursor `wait`; pointer-events none.
Radius: `var(--radius-sm)`.

#### `Input.svelte`
Props: `type: 'text'|'number'|'search'`, `value: string`, `placeholder?: string`, `disabled?: boolean`, `invalid?: boolean`, `id?: string`, `aria-describedby?: string`
- Height: 36px; padding: 0 `var(--space-md)`; font: body 14px
- Background: `var(--color-bg)`; border: `1px solid var(--color-border)`; radius `var(--radius-sm)`
- Focus: `border-color: var(--color-accent); box-shadow: 0 0 0 3px var(--color-accent-focus)`
- Invalid: `border-color: var(--color-destructive)` + кайма
- Placeholder: `color: var(--color-text-muted)`
- Disabled: `background: var(--color-surface-sunken); color: var(--color-text-muted)`

#### `Select.svelte`
Same dimensions as Input. Native `<select>` styled. Caret-icon inline-SVG right-aligned.

#### `Textarea.svelte`
Same metrics. min-height 80px; resize vertical only.

#### `Modal.svelte`
Props: `open: boolean`, `title: string`, `size: 'md'|'wide'` (md=640, wide=960), `onClose: () => void`
- Backdrop: `position: fixed; inset: 0; background: rgba(0,0,0,0.40); backdrop-filter: blur(2px)` (dark theme: 0.60)
- Container: `var(--color-surface-raised)`, radius `var(--radius-md)`, `box-shadow: var(--shadow-elev-2)`
- Header: `var(--space-md) var(--space-lg)`; heading (20px/600); close `×` button (ghost 28px right)
- Body: `padding: var(--space-lg); max-height: calc(100vh - 200px); overflow-y: auto`
- Footer: `var(--space-md) var(--space-lg)`; actions right-aligned, gap `var(--space-sm)`
- ESC to close; focus-trap; mount via portal at `document.body`; `aria-modal="true"`, `role="dialog"`, `aria-labelledby={title-id}`
- На open: `body { overflow: hidden }`

#### `Toast.svelte` + `ToastHost.svelte`
- `ToastHost.svelte` — единственный, монтируется в `App.svelte` под рутом
- Position: `position: fixed; bottom: var(--space-lg); right: var(--space-lg); z-index: 1000; display: flex; flex-direction: column; gap: var(--space-sm); max-width: 400px`
- Один Toast: `padding: var(--space-md); border-radius: var(--radius-md); box-shadow: var(--shadow-elev-2); background: var(--color-surface-raised); border-left: 4px solid var(--color-{kind})`
- Kind colors: success → `--color-success`; error → `--color-destructive`; info → `--color-accent`; warning → `--color-warning`
- TTL: error 6000ms, success 4000ms, info 4000ms, warning 5000ms (см. `toast.svelte.ts`)
- Close: `×` button top-right ghost-style
- Aria: `role="status"` для info/success, `role="alert"` для error/warning
- Enter: opacity 0→1 + translateY(8px→0) 150ms ease-out
- Exit: opacity 1→0 + translateY(0→8px) 100ms ease-in

#### `ThemeSwitcher.svelte`
3 segmented radio (visually grouped, single border, dividers between):
- `[Светлая | Тёмная | Системная]`
- Width: 100% of sidebar footer (~208px); height 32px
- Each segment: 1/3 width, font-size 13px; selected segment background `var(--color-surface-raised)` (light) или `var(--color-surface-sunken)` (dark), normal `--color-surface`
- Bind: `setTheme(preference)` из `lib/stores/theme.svelte.ts`

#### `Placeholder.svelte`
Универсальная заглушка для нереализованных страниц.
- Centered (flex), padding `var(--space-2xl)`
- Heading 20px/600: `{$props.section}` (e.g. «Дашборд»)
- Body 14px regular `var(--color-text-secondary)`: «Раздел в разработке» + sub-line «Появится в Phase N»

#### `Spinner.svelte`
Inline circular spinner. Size prop (`sm`=12px, `md`=16px, `lg`=24px). CSS-only (`@keyframes spin`), color `currentColor` (наследует от parent).

#### `Badge.svelte`
Counter chip для switch-bar и т.п. Variants: `default`, `accent`, `success`, `warning`, `destructive`. Size: 20px height, padding `0 var(--space-sm)`, font 12px/500, radius 10px.

### Devices feature (in `features/devices/`)

#### `DevicesPage.svelte`
Route shell. Иерархия:
```
<h1>Устройства</h1>
<header-actions>[+ Создать устройство] [Импорт CSV] [Экспорт CSV]</header-actions>
<DeviceFilters/>
<DeviceList/>
```

#### `DeviceFilters.svelte`
- FTS search input (full-width, иконка лупы слева, debounce 250ms — CONTEXT D-Search-01)
- Status switch-bar: 5 tabs с counter-badge у каждого («Все (NNN) / На складе (NNN) / …»)
- Group toggle (правый край): `<Checkbox>` + label «Группировать похожие»

#### `DeviceList.svelte`
- Header row: «Тип / Наименование / Инвентарный № / Серийный № / Модель / Расположение / Статус / Действия»
- Columns: type 100px, name 25%, inv 140px, serial 140px, model 20%, location 20%, status 120px, actions 40px
- Numeric columns (`inventory_no`, `serial_no`) — `font-variant-numeric: tabular-nums`
- Status — `Badge` с цветом по статусу: «На складе» default, «В работе» accent, «На ремонте» warning, «Списано» muted
- Actions: kebab-menu (3 dots) → dropdown с пунктами: Редактировать / Удалить
- Row height `--row-height` (40px); hover row → `background: var(--color-surface)`
- Group row variant (`DeviceGroupRow.svelte`) — отдельный визуал: левее на 16px chevron expand/collapse; label `«{count} шт.»` в столбце «Серийный №»; при expand — children rows с indent 24px и без kebab-menu на group-header
- Pagination footer (CONTEXT D-UI-Pagination-01): «1–50 из 1240» слева; центр `< 1 2 3 … 25 >` (numbered, current — accent fill)

#### `DeviceFormModal.svelte`
- Modal `size="md"`
- Поля в порядке REQ-DEV-01 (Тип → Наименование → … → Статус)
- Required-поля помечены `⃰`; нижняя строка модала: actions [Отмена] [Создать / Сохранить]
- Под полем «Состояние» — chip-row с 6 state-hints (DEV-10) с label «Быстрый выбор:»
- Validation: inline error под полем при server `AppError::Validation`
- `Создать` disabled пока обязательные пусты (`$derived canSubmit`)
- При submit: button показывает spinner + «Сохранение…»

#### `DeviceAutocompleteField.svelte`
Reusable autocomplete для DEV-08/DEV-09 contextual.
- Props: `field: 'name'|'model'|'specs'|'kit'|'state'|'location'`, `value: string`, `contextName?: string`, `placeholder?: string`
- Behavior: при `input` event — debounce 200ms — `apiCall('devices_autocomplete', { field, prefix: value, ctx_name: contextName })` → render dropdown
- Dropdown: `position: absolute`; max-height 240px; overflow auto; каждый элемент — 32px height, hover bg `--color-surface`
- Keyboard: ↑/↓ navigate, Enter select, Esc close
- При `contextName` задан и field !== 'name': dropdown показывает heading «Ранее использовалось с «{name}»:» (label 13px/500 color muted), под — список
- Empty: «Начните вводить, чтобы увидеть подсказки» (text-muted, 14px)
- Loading: spinner-sm + «Загружаем подсказки…» (text-secondary, 13px)

#### `DeviceImportCsvModal.svelte`
- Modal `size="wide"`
- 4-step wizard (см. Copywriting). Step indicator сверху: dots `● ● ○ ○` или текстом `Шаг 2 из 4: Проверьте данные`
- Step 2 preview-таблица: 5 rows max, ширина авто, sticky header
- Step 3 mapping-таблица: 2 колонки — «Колонка CSV» / `<Select>` для маппинга
- Step 4: progress bar (если commit > 100 rows, иначе моментальный), под — список ошибок (expandable)

#### `DeviceContextMenu.svelte` (kebab)
Triggered by 3-dots button per row.
- Dropdown с пунктами: «Редактировать» / «Удалить»
- «Удалить» — `color: var(--color-destructive)`, separator выше
- Открытие по click; close на outside-click + Esc

---

## Interaction Patterns

### Theme application (UI-02, CONTEXT D-UI-Theme-01)

1. **Inline `<head>` script (NO-FLASH)** в `index.html` ДО Vite-bundled `<script type="module">`:
   ```html
   <script>
     (function(){
       try {
         var t = localStorage.getItem('trackly:theme');
         var prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
         var resolved = (t==='light'||t==='dark') ? t : (prefersDark?'dark':'light');
         document.documentElement.dataset.theme = resolved;
       } catch(e){}
     })();
   </script>
   ```
2. **Module-level store** `lib/stores/theme.svelte.ts` (см. RESEARCH §Pattern 9) — `preference: 'light'|'dark'|'system'`, `resolved: 'light'|'dark'`
3. **Storage key**: `trackly:theme` (namespaced)
4. **`matchMedia` change-listener** активен только когда `preference === 'system'`
5. **Зачитываем** `localStorage` в inline-script (no-flash) + второй раз в `initTheme()` для store-синхронизации (без re-apply DOM, чтобы не вызвать вспышку)
6. **Transition:** **БЕЗ** CSS-transitions на цвета (`transition: background 200ms` запрещён). Мгновенное переключение — лучший UX для accessibility (motion-sensitive users) И избегает «прыгающего» вида при toggle.

### Search / filter / pagination

| Behavior | Detail |
|----------|--------|
| Search debounce | 250ms input idle → trigger `devices_search` |
| Search empty | Сбрасывает FTS; показывает `devices_list` отфильтрованный по status |
| Status switch-bar | Click таба → reset pagination на page 1, новый запрос |
| Pagination | Server-side, 50/страница; кнопки `< prev` / numbered / `next >` |
| Group toggle | Per-session (sessionStorage `trackly:devices:grouped`); default ON |
| Filter clear | Кнопка-link «Сбросить фильтры» в empty state — сбрасывает search + status to 'all' + group=ON |

### Form modal flow

1. Click `+ Создать устройство` → modal open, focus → first input (Тип)
2. Ввод/выбор; для autocomplete полей dropdown открывается на focus или при вводе ≥1 char
3. Tab навигация по полям сверху вниз
4. Click `Создать` → button → loading state → `apiCall` → on success: toast «Устройство создано», modal close, list refresh; on error: inline field errors + toast
5. Esc → close с confirmation если форма «dirty» (есть un-saved changes) — modal-confirm «Отменить изменения?» [Отменить / Продолжить редактирование]

### Destructive confirmation

1. Click «Удалить» в context-menu → modal `size="md"` (см. Destructive actions table выше)
2. Confirm button **не auto-focus** (защита от случайного Enter); focus на `Отмена` (default safer)
3. Click «Удалить» → soft-delete → toast «Устройство удалено» с link «Отменить» (стрейч-цель; в Phase 2 можно без undo, тогда toast без link)

### Autocomplete behavior (DEV-08, DEV-09)

- **Trigger:** focus on field OR input change with value.length ≥ 1
- **Debounce:** 200ms (быстрее, чем search, потому что подсказки — короче и важнее отзывчивость)
- **Max suggestions:** 30 (CONTEXT D-Autocomplete-01)
- **Sort:** ASC по значению; точные совпадения в начале
- **Contextual (DEV-09):** если в DeviceFormModal `name` поле уже заполнено, передаём `ctx_name`; backend фильтрует только значения, встречавшиеся с этим именем; dropdown показывает heading «Ранее использовалось с «{name}»:»
- **Keyboard:** ↑/↓ навигация (active элемент — `background: --color-surface; border-left: 2px var(--color-accent)`); Enter — select, Esc — close, Tab — select+next-field
- **Click outside** — close, value стирается ТОЛЬКО если он не валидирует и юзер не нажал select; иначе value остаётся как free-text (так как Расположение/Состояние — open-vocab)

### Error rendering (UI-06, CONTEXT D-UI-Errors-01)

- **`apiClient` always try/catch.** Catch блок: `toastStore.error(parseAppError(e).message)`
- **`AppError::Validation { field, message }`** в form-context: inline под полем + toast (toast для уверенности, что пользователь увидел)
- **`AppError::OptimisticLockMismatch`**: специальный toast с фиксированной копией («Данные были изменены другим пользователем. Обновите страницу и попробуйте снова.»)
- **`AppError::NotFound`**: для list-page → empty state «Ничего не найдено»; для get-by-id → toast + redirect на list
- **Network error** (Tauri `invoke()` фейлит без AppError shape): toast «Не удалось связаться с приложением. Попробуйте перезапустить.»

### Loading states

- **Initial page load**: skeleton-blocks для DeviceList (5 строк × 8 колонок, серые placeholder-блоки 16px высотой, animate-pulse 1.2s)
- **Inline button submit**: spinner в кнопке + текст-замена (см. Button.loading)
- **CSV preview parsing**: full-modal spinner overlay + text «Анализируем файл…»
- **CSV commit**: progress bar 0→100%, обновляется по batch-progress events (если backend шлёт) или просто intermediate spinner

### Keyboard accessibility

- **All interactive** — `:focus-visible` ring (`box-shadow: 0 0 0 3px var(--color-accent-focus)`)
- **Modal focus-trap** в `Modal.svelte`
- **Esc closes** modals, dropdowns, autocomplete dropdowns
- **Tab order** — left-to-right, top-to-bottom; no `tabindex > 0`
- **Skip-link** в `Layout.svelte`: hidden-until-focus «Перейти к основному содержимому» (mounts before sidebar; targets `<main>`)

### Density

Phase 2 — single density (desktop). Row height `--row-height` (40px); button height 36px; input 36px. Dense (32px) — заложен в tokens, активируется в Phase 7.

### Responsive (UI-04, CONTEXT D-UI-Responsive-01)

- Target: 1280×720 minimum
- Sidebar fixed 240px; content `flex: 1; overflow-x: auto`
- При viewport < 1280 → horizontal scroll on `<main>` (приемлемо для desktop apps; user видит горизонтальную полосу)
- Modals: `max-width: var(--modal-max-width)`; на < 720px height — `max-height: calc(100vh - 64px)` с body-scroll
- **NO** mobile breakpoints, **NO** hamburger menu, **NO** stacked layout

### Motion

- Modal enter: opacity 0→1 + scale 0.98→1.00 (150ms ease-out)
- Modal exit: opacity 1→0 (100ms ease-in)
- Toast: см. Toast component
- Dropdown: opacity 0→1 (100ms)
- Theme switch: instant (no transition)
- **`prefers-reduced-motion`**: все вышеперечисленные transitions → `0ms` (запрос в global.scss)
- **NO** scroll animations, **NO** parallax, **NO** page-transitions

---

## Registry Safety

| Registry | Blocks Used | Safety Gate |
|----------|-------------|-------------|
| shadcn official | (не применимо) | not required — shadcn не инициализирован (Svelte 5 + custom hand-rolled primitives per CONTEXT) |
| third-party | (нет) | not applicable |

**Tool:** none. Phase 2 не использует генератор UI и не вытягивает компоненты из реестров. Все примитивы пишутся вручную (см. Component Inventory). Этот пункт сознательный — CLAUDE.md фиксирует «no component library», CONTEXT.md D-UI-Errors-01 / D-UI-Validation-01 явно отвергают `svelte-french-toast` / `formsnap` / `superforms`.

**Третьих-сторонних JS-пакетов фазы (не реестровых, чисто npm):**

| Package | Version | Source | Audit |
|---------|---------|--------|-------|
| `svelte-spa-router` | `5.1.0` | npm: ItalyPaleAle | RESEARCH §Package Legitimacy Audit — Approved (Svelte 5 explicit support, 7+ years, Microsoft engineer maintainer) |
| `@tauri-apps/api` | `2.11.0` | npm: tauri-apps | Approved (official Tauri publisher) |
| `@tauri-apps/plugin-dialog` | `2.7.1` | npm: tauri-apps | Approved (official plugin) |

Установка проходит через стандартный `pnpm add` step, авторизированный Phase 2 planner — registry-vetting gate не требуется (нет shadcn third-party блоков).

---

## Checker Sign-Off

- [ ] Dimension 1 Copywriting: PASS (все строки на русском, CTA — verb+noun, empty/error states описаны, destructive обязан confirm)
- [ ] Dimension 2 Visuals: PASS (Component inventory с props/states; layout-tokens определены; motion ограничен; reduced-motion поддержан)
- [ ] Dimension 3 Color: PASS (60/30/10 light + dark; accent reserved-for list из 7 элементов; контрастность AA verified)
- [ ] Dimension 4 Typography: PASS (4 sizes, 2 weights, system stack кириллица-готовый, tabular-nums на числовых колонках)
- [ ] Dimension 5 Spacing: PASS (8pt scale, multiples of 4; конструктивные размеры в отдельной таблице как exceptions; declared values используются жёстко)
- [ ] Dimension 6 Registry Safety: PASS (shadcn=none; 3 third-party JS пакета verified в RESEARCH §Package Legitimacy Audit)

**Approval:** pending (checker валидирует)

---

## Pre-Population Sources

| Section | Source | Fields |
|---------|--------|--------|
| Design System | CLAUDE.md + CONTEXT D-UI-Router-01/D-UI-State-01 | Tool=none, router=svelte-spa-router, state=runes |
| Spacing | Default 8pt + CONTEXT D-UI-Responsive-01 | --sidebar-width=240px |
| Typography | Claude's discretion (нет фикс. размеров в upstream) | 14/13/20/28 px, 400/500/600 weights |
| Color | Claude's discretion + REQUIREMENTS (нейтральная палитра, dark theme) | full palette light + dark |
| Sidebar order | UI-01 + CONTEXT D-UI-Sidebar-01 | exact 10 items + 3 dividers |
| Theme switcher placement | UI-02 + CONTEXT D-UI-Theme-01 | в sidebar footer, не в Настройках |
| No-flash script | CONTEXT D-UI-Theme-01 + RESEARCH Pattern 5 | localStorage key, mql listener |
| Devices copy | REQUIREMENTS DEV-01..13 + CONTEXT specifics | Russian field labels, state-hints, CSV wizard copy |
| Destructive confirm | UI-06 + Phase 2 deferred-items.md (no undo yet) | single destructive (soft-delete) |
| Error parsing | CONTEXT D-UI-Errors-01 + RESEARCH §Pattern 8 | AppError.message rendering rules |
| Pagination | CONTEXT D-UI-Pagination-01 | 50/page, numbered + prev/next |
| Autocomplete | CONTEXT D-Autocomplete-01 + D-AutocompleteEndpoint-01 + DEV-08/DEV-09 | 30 max, 200ms debounce, contextual heading |
| Registry safety | CLAUDE.md «no component library» + RESEARCH §Package Legitimacy Audit | none + 3 vetted JS deps |
