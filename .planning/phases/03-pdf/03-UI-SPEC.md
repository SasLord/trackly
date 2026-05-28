---
phase: 3
slug: 03-pdf-acts
status: draft
shadcn_initialized: false
preset: none
created: 2026-05-28
inherits_from: ../02-ui/02-UI-SPEC.md
---

# Phase 3 — UI Design Contract: Акты приёма-передачи и первая PDF-печать

> Визуальный и интерактивный контракт фазы. Сгенерирован gsd-ui-researcher, валидируется gsd-ui-checker.
> Все строки UI — на русском (CLAUDE.md + UI-03). Имена компонентов, токенов и атрибутов — English.
> Фаза 3 **наследует токены и примитивы Phase 2** без отклонений. Phase 3 расширяет компонентный инвентарь
> новой feature-папкой `features/acts/`, переиспользует `Modal/Input/Select/Button/Toast/Badge/Spinner`,
> добавляет 3 новых паттерна (master-detail split, wide-act-modal с табличными позициями, PDF preview iframe).
> Контракт прескриптивный: executor реализует точно то, что здесь записано; отклонения требуют поправки UI-SPEC.md.

---

## Design System

| Property | Value | Source |
|----------|-------|--------|
| Tool | none (hand-rolled Svelte 5 primitives) | inherited from Phase 2 |
| Preset | not applicable | inherited |
| Component library | none — расширяем `ui/src/lib/components/` + `ui/src/features/acts/` | inherited |
| Icon library | inline SVG в `ui/src/lib/icons/*.svelte` (~6 новых иконок в Phase 3) | inherited |
| Font (UI) | `-apple-system, "Segoe UI", "Roboto", "Helvetica Neue", "Arial", sans-serif` | inherited |
| Font (PDF) | **DejaVu Sans Regular + Bold** (embedded via `include_bytes!`) | Phase 3 new — D-PDF-Engine-01 |
| Styling | SCSS + `_tokens.scss` autoprepended | inherited |
| Router | `svelte-spa-router 5.1.0` | inherited |
| State | Svelte 5 runes (`$state`/`$derived`/`$effect`) | inherited |
| PDF preview | `pdfjs-dist ^4.x` в `<iframe src="blob:...">` | Phase 3 new — D-Print-UX-01 |

**PDF vs UI font разделение:** UI рендерится system-stack'ом (Segoe UI / SF Pro покрывают кириллицу).
PDF рендерится embedded DejaVu Sans (детерминированный output, кириллица + ё + диакритики, public-domain
лицензия без обязательной атрибуции). Это **намеренное разделение** — `_tokens.scss` цвета и шрифты
**НЕ влияют** на PDF (там монохромный layout, контролируемый DocSpec → krilla).

---

## Spacing Scale

Полностью наследуется из Phase 2 (`_tokens.scss`). **Никаких новых spacing-токенов в Phase 3.**

| Token | Value | Phase 3 usage |
|-------|-------|---------------|
| `--space-xs` (4px) | 4px | gap иконка↔текст в кнопке «Печать», hairline между sub-number badge и №акта |
| `--space-sm` (8px) | 8px | gap внутри switch-bar табов, gap между чекбоксом и текстом в Return-модале |
| `--space-md` (16px) | 16px | padding ячеек таблицы «Позиции», gap между полями шапки акта, gap между bulk-default и таблицей |
| `--space-lg` (24px) | 24px | padding страницы Acts, gap между switch-bar и master-detail, padding preview-модала |
| `--space-xl` (32px) | 32px | gap между шапкой акта и таблицей позиций в ActDetail; gap между разделами модала Создание |
| `--space-2xl` (48px) | 48px | top-padding пустого state «Выберите акт» |

**Layout-конструктивные размеры Phase 3:**

| Token | Value | Reason |
|-------|-------|--------|
| `--acts-list-width` | `minmax(360px, 35%)` | левая колонка master-detail (D-Acts-List-01) |
| `--acts-detail-width` | `1fr` (65%) | правая колонка master-detail |
| `--acts-list-min-width` | 320px | при viewport < 1100 — горизонтальный скролл (no responsive collapse, как и Phase 2) |
| `--modal-max-width-acts-create` | 1000px | широкий модал создания акта (D-Acts-Create-01) |
| `--modal-max-width-acts-return` | 880px | модал возврата с таблицей позиций |
| `--modal-max-width-pdf-preview` | `min(95vw, 1100px)` | preview-модал; чем шире, тем удобнее читать A4 в pdfjs viewer |
| `--pdf-preview-height` | `min(90vh, 920px)` | контейнер для `<iframe>` viewer |
| `--act-list-row-height` | 64px | компактная карточка списка актов (двухстрочная: №+дата сверху, ФИО+items count снизу) |
| `--act-items-row-height` | 44px | строка таблицы «Позиции» (чуть выше `--row-height`, так как содержит inline-controls в return-модале) |

**Все остальные layout-tokens** (`--sidebar-width`, `--header-height`, `--modal-max-width`,
`--touch-target-min`, `--row-height`, `--radius-sm`, `--radius-md`, `--shadow-elev-1/2`) — **из Phase 2 без изменений**.

**Exceptions:** `--acts-list-width`, `--modal-max-width-acts-*`, `--pdf-preview-*`, `--act-list-row-height`,
`--act-items-row-height` — конструктивные лимиты, не отступы; они расширяют, а не заменяют Phase 2 список.

---

## Typography

Полностью наследуется из Phase 2. **Никаких новых typography-токенов в Phase 3.**

| Role | Size | Weight | Line Height | Phase 3 usage |
|------|------|--------|-------------|---------------|
| Body | 14px | 400 | 1.5 | список актов (ФИО, дата), таблица позиций, поля модалов, тело PDF preview модала |
| Label | 13px | 500 | 1.4 | заголовки колонок таблицы позиций, label полей шапки акта, аннотации в Return-модале («(по умолчанию)») |
| Heading | 20px | 600 | 1.3 | заголовок страницы «Акты», заголовки модалов («Новый акт», «Возврат по акту №42», «Печать акта №42») |
| Display | 28px | 600 | 1.2 | резерв; в Phase 3 не используется |

**Phase 3 уточнения:**
- `font-variant-numeric: tabular-nums` обязательно на колонке «№» в списке актов и на колонке «Количество» в таблице позиций — числа должны выравниваться разрядами при сортировке.
- Суффикс возврата «в», «в1», «в2» — same weight как номер (regular в списке, semibold 600 в заголовке модала просмотра); **никогда не uppercase**, никогда не `letter-spacing`-разрядка.
- Override-badge «авто» / «override» в шапке модала создания — Label (13px/500), цвет `--color-text-muted` (idle) или `--color-warning` (override активирован).

**Никаких новых SCSS-переменных typography не добавляется.**

---

## Color

Полностью наследуется из Phase 2 light + dark тем. **Никаких новых color-токенов в Phase 3.**

### Phase 3 specific color usage

| Phase 3 element | Token | Role |
|-----------------|-------|------|
| Switch-bar Акты/Возвраты/Архив (active tab underline) | `--color-accent` | accent (соответствует Phase 2 status-switch-bar pattern) |
| Counter badge у активного таба | `--color-accent` (fill) + `--color-text-inverse` | accent — only on active |
| Counter badge у idle таба | `--color-surface-sunken` (fill) + `--color-text-secondary` | neutral |
| Selected act row (master-detail) | `border-left: 3px var(--color-accent)` + `background: color-mix(in srgb, var(--color-accent) 8%, transparent)` | accent — selection indicator |
| Hover act row (idle) | `background: var(--color-surface)` | neutral |
| ActDetail panel background | `--color-bg` | dominant (60%) |
| ActsList panel background | `--color-surface` | secondary (30%) |
| Empty state «Выберите акт» | `--color-text-muted` | muted |
| Archive badge in detail (when archived=1) | `Badge variant="default"` (gray), copy «Архив» | neutral, не accent |
| Return-row checkbox checked | `--color-accent` fill + white check | accent (Phase 2 checkbox pattern) |
| Override badge «override» (когда юзер ввёл свой номер) | `Badge variant="warning"` (`--color-warning` border-left) | warning |
| Auto badge «авто» (idle) | `Badge variant="default"` | neutral |
| Sub-number suffix «в»/«в1» | `--color-text-secondary` (приглушённо, шапка номера остаётся primary) | secondary |
| PDF preview модал — backdrop | `rgba(0,0,0,0.6)` (light) / `rgba(0,0,0,0.75)` (dark) | сильнее, чем стандартный modal (PDF — фокус-режим) |
| Loading overlay в preview модале | `--color-surface-raised` + Spinner.lg | neutral |
| Error state в preview модале | `--color-destructive` (текст ошибки) + Button.secondary «Повторить» | destructive |

### Accent reserved-for list (Phase 3 ADDITIONS to Phase 2 list)

Phase 2 определила 7 элементов под accent. Phase 3 **добавляет** 3 элемента:

8. **Active tab в Acts switch-bar** (Акты / Возвраты / Архив) — нижняя 2px граница accent + counter-badge с accent fill (Phase 2 pattern для статус-switch-bar повторяется 1:1).
9. **Selected act row** в `ActsList` — left-border 3px accent + tinted background (8% accent in light, 12% в dark).
10. **Primary CTA «Создать акт»** в header страницы Acts (как Phase 2 «+ Создать устройство»).

**По-прежнему запрещено:** accent на «Печать», «Редактировать», «Возврат» actions (это secondary
действия) — они идут как `Button.secondary` или `Button.ghost`. Accent остаётся CTA-сигналом.

**Destructive (`--color-destructive`) reserved-for Phase 3:**
- Кнопка «Удалить акт» в context-menu детали (как в Phase 2 «Удалить устройство»).
- Confirmation modal heading «Удалить акт?» + body warning о восстановлении state.
- Inline field errors в модалах (border `--color-destructive` + сообщение).

**Warning (`--color-warning`) reserved-for Phase 3:**
- Badge «override» на ручном номере (см. выше).
- Toast при override номера (info-level, не error): «Номер №42 будет записан с пометкой override».
- Confirmation modal warning text при удалении handover («Все устройства вернутся на склад в исходное Состояние и Расположение»).
- Warning toast при отсутствии `org.json` / `logo.png` (только при первом запуске).

**Никаких новых CSS-переменных color не добавляется.**

---

## Copywriting Contract

> Все строки — на русском. Backend `AppError.message` уже русский (Phase 1 invariant), UI **не переводит**.
> Phase 3 строки — только UI-собственные.

### Sidebar обновление (UI-01)

Раздел «Акты» — из placeholder становится **ACTIVE** (как «Устройства» в Phase 2).

| Position | Label | Route | Phase 3 state |
|----------|-------|-------|---------------|
| 4 | Акты | `#/acts` | **АКТИВНЫЙ — реализован в Phase 3** |

Все остальные пункты sidebar — без изменений из Phase 2.

### Acts page (ACT-01..14)

| Element | Copy |
|---------|------|
| Page heading | Акты |
| Page-level primary CTA (top-right) | + Создать акт |
| Search input placeholder (above master-detail) | Поиск по номеру, ФИО, наименованию устройства |
| Switch-bar tabs (ACT-02) | Акты ・ Возвраты ・ Архив |
| Counter chip format | `{label} ({count})` — пример: `Акты (124)`, `Возвраты (37)`, `Архив (89)` |

### Switch-bar tabs — точная семантика

| Tab | Backend filter | Show count source |
|-----|----------------|-------------------|
| Акты | `act_type='handover' AND archived=0 AND deleted_at_utc IS NULL` | `acts_counts().handover_active` |
| Возвраты | `act_type='return' AND deleted_at_utc IS NULL` | `acts_counts().returns` |
| Архив | `act_type='handover' AND archived=1 AND deleted_at_utc IS NULL` | `acts_counts().archived` |

### ActsList (master) — карточки

| Element | Copy / Format |
|---------|---------------|
| Row top line | `№{number}{suffix}` + точка-разделитель + `{date in «28 мая 2026»}` |
| Row bottom line | `{receiver_name}` + точка-разделитель + `{items_count} устр.` |
| Archived badge (только в табе «Архив») | `Badge variant="default"` copy «В архиве» |
| Empty list (нет актов вообще в табе «Акты») | Heading: «Актов пока нет» / Body: «Создайте первый акт приёма-передачи.» / Action: **+ Создать акт** |
| Empty list (таб «Возвраты», нет возвратов) | Heading: «Возвратов пока нет» / Body: «Возвраты появятся, когда какие-то устройства вернутся на склад.» |
| Empty list (таб «Архив», нет архивных) | Heading: «Архив пуст» / Body: «Сюда попадают акты после полного возврата всех устройств.» |
| Empty search result | Heading: «Ничего не найдено» / Body: «По запросу «{query}» ничего не нашлось. Проверьте написание или сбросьте поиск.» / Action link: **Сбросить поиск** |

### ActDetail (slave) — детальная панель

| Element | Copy |
|---------|------|
| Empty state heading (master-detail без выбора) | Выберите акт |
| Empty state body | Выберите акт слева, чтобы увидеть подробности, или создайте новый. |
| Empty state action | **+ Создать акт** |
| Section heading «Шапка акта» | Шапка |
| Header field: № | № |
| Header field: Дата | Дата |
| Header field: Сдал | Сдал |
| Header field: Принял | Принял |
| Header field: Сроком до | Сроком до |
| Header field: Расположение | Расположение |
| Section heading «Позиции» | Позиции ({count}) |
| ItemsTable column headers | Устройство ・ Инв. № ・ Серийный № ・ Количество ・ Состояние ・ Возврат |
| Items column «Возврат» values | `—` (если не возвращено) / `вернулось {date}` (если возвращено, link на return-акт) |
| Section heading «История возвратов» | История возвратов |
| Return entry format | `№{parent}в{sub} от {date}` + статус (полный/частичный) |
| Action buttons row labels | Печать ・ Редактировать ・ Возврат ・ Удалить |
| Print button icon + label | (printer icon) Печать |
| Edit button | Редактировать |
| Return button (primary action) | Возврат |
| Delete button (destructive) | Удалить |

### ActFormModal (ACT-03, D-Acts-Create-01)

| Element | Copy |
|---------|------|
| Modal heading (create) | Новый акт |
| Modal heading (edit) | Редактирование акта |
| Section heading «Шапка» | Шапка акта |
| Field — № ⃰ | № |
| № field hint (auto-mode idle) | Badge «авто» рядом с инпутом + tooltip «Следующий по порядку. Можно изменить.» |
| № field hint (override active) | Badge «override» (warning) + tooltip «Будет записано в журнал событий.» |
| № field action button (right of input) | «Следующий» (link-button; возвращает в auto-mode и подставляет предсказанный номер) |
| Field — Дата ⃰ | Дата |
| Date default | сегодня (через инжектированный `Clock`, не `Date.now()`) |
| Field — Сдал ⃰ | Сдал |
| Sдал placeholder | Иванов Иван Иванович |
| Field — Принял ⃰ | Принял |
| Принял placeholder | Петров Пётр Петрович |
| Field — Сроком до | Сроком до |
| Срокм до hint | Необязательно. Например, «до конца проекта». |
| Field — Расположение (общее) | Расположение |
| Расположение placeholder | Куда передаются устройства |
| Section heading «Позиции» | Позиции |
| Items add row button | + Добавить позицию |
| Items column headers | # ・ Устройство ⃰ ・ Количество ⃰ ・ ⌧ |
| Items remove row aria-label | Удалить позицию {n} |
| Items empty (no rows yet) | Добавьте хотя бы одну позицию. |
| Items device autocomplete placeholder | Устройство со склада |
| Items quantity min/max | min=1, max={device.available_quantity} |
| Items quantity hint when device picked | «На складе: {available}» |
| Primary action (create) | Создать акт |
| Primary action (edit) | Сохранить |
| Secondary action | Отмена |
| Submit loading | На кнопке: спиннер + «Создание…» (edit: «Сохранение…») |

### ReturnModal (ACT-07, ACT-08, D-Acts-Return-01)

| Element | Copy |
|---------|------|
| Modal heading | Возврат по акту №{parent_number} |
| Subheading (под заголовком, label-style) | Создаст акт возврата №{parent}в{predicted_sub_number} |
| Section heading «Применить ко всем» | Применить ко всем выбранным позициям |
| Bulk Состояние label | Состояние |
| Bulk Состояние placeholder | Хорошее / Б/У / Среднее / Новое |
| Bulk Расположение label | Расположение на складе |
| Bulk Расположение placeholder | Куда вернуть на склад |
| Apply-to-all checkbox | Применить ко всем (по умолчанию ВКЛ — per D-Acts-Return-01) |
| Section heading «Позиции к возврату» | Позиции к возврату ({count}) |
| Items column headers | ☑ ・ Устройство ・ Кол-во к возврату ・ Состояние ・ Расположение |
| Per-row override hint (когда не override) | «(по умолчанию)» в `color: var(--color-text-muted)` 13px |
| Per-row override active hint | «(переопределено)» в `color: var(--color-warning)` 13px |
| Row not-checked state | inputs disabled + visual muted (opacity 0.5) |
| Primary action | Оформить возврат |
| Secondary action | Отмена |
| Submit loading | Спиннер + «Оформляем возврат…» |
| Success toast | Создан акт возврата №{number}{suffix}. {n} устр. вернулось на склад. |
| Auto-archive notification (toast после success) | Акт №{parent} переехал в Архив (все устройства вернулись). — показывается только если archived стало 1 |

### PdfPreviewModal (ACT-11, DEV-14, D-Print-UX-01)

| Element | Copy |
|---------|------|
| Modal heading (handover act) | Печать акта №{number} |
| Modal heading (return act) | Печать акта возврата №{number}{suffix} |
| Modal heading (acceptance document — DEV-14) | Печать документа приёма |
| Loading state heading (PDF rendering) | Готовим PDF… |
| Loading state body | Подождите пару секунд. Большие акты могут потребовать чуть больше времени. |
| Error state heading | Не удалось сформировать PDF |
| Error state body | {AppError.message} |
| Error state action | **Повторить** |
| Footer action: Save (Primary) | Сохранить как PDF |
| Footer action: Open in OS viewer | Открыть в системном просмотрщике |
| Footer action: Print | Печать |
| Footer action: Close (Ghost) | Закрыть |
| Save dialog default filename (handover) | `Акт_приёма-передачи_№{number}_{YYYY-MM-DD}.pdf` |
| Save dialog default filename (return) | `Акт_возврата_№{number}{suffix}_{YYYY-MM-DD}.pdf` |
| Save dialog default filename (acceptance) | `Документ_приёма_{device.name}_{YYYY-MM-DD}.pdf` |
| Save toast | PDF сохранён: {filename} |
| Save error toast | Не удалось сохранить PDF. {AppError.message} |
| Open-in-OS toast | Открыли PDF в просмотрщике. |
| Print: page-level event | Открыт системный диалог печати. |

### Document приёма entry point (DEV-14, DEV-15)

Точка входа — на **странице Устройства** (Phase 2 раздел): новая кнопка в `DeviceList` row context-menu **и** в DeviceDetail (если есть; иначе только в context-menu).

| Element | Copy |
|---------|------|
| Context-menu item label | Печать документа приёма |
| Context-menu item position | После «Редактировать», перед «Удалить» (отделено separator) |
| Modal heading (intermediate input modal) | Документ приёма устройства |
| Modal field — Кто передал ⃰ | Кто передал |
| Modal field — Кто принял ⃰ | Кто принял |
| Modal field — Дата | Дата (default: сегодня) |
| Modal primary action | Сформировать PDF |
| Modal secondary action | Отмена |
| После submit | Открывается PdfPreviewModal (heading «Печать документа приёма») |

### Destructive actions (Phase 3 additions)

| Action | Trigger | Confirmation modal heading | Confirmation modal body | Confirm button | Cancel button |
|--------|---------|----------------------------|--------------------------|----------------|---------------|
| Удалить акт приёма-передачи (handover) | Кнопка «Удалить» в ActDetail | Удалить акт №{number}? | Акт будет помечен как удалённый. Все устройства из акта вернутся на склад в **исходные** Состояние и Расположение (на момент выдачи). Действие можно отменить только восстановлением из бэкапа БД. | Удалить (destructive) | Отмена |
| Удалить акт возврата (return) | Кнопка «Удалить» в ActDetail у return-акта (либо в History секции handover'а) | Удалить акт возврата №{number}{suffix}? | Акт будет помечен как удалённым. Состояние и Расположение устройств вернутся к значениям **на момент выдачи** по акту №{parent_number}. Если parent был в Архиве — выйдет из архива. | Удалить (destructive) | Отмена |

**В Phase 3 — два новых destructive action**: удаление handover-акта и удаление return-акта. Оба soft-delete (через `deleted_at_utc`) с undo через `audit_log.before_json`.

### Loading states (Phase 3)

| Context | Copy |
|---------|------|
| Initial page load (skeleton) | (без текста — skeleton-блоки списка + плейсхолдер detail) |
| Acts list — пагинация/смена таба | (Spinner внутри list-panel, без затемнения) |
| ActDetail — загрузка после выбора | Spinner + «Загружаем акт…» (по центру правой панели) |
| Counts refresh (фоновый) | без UI; tab-badges обновляются on-success без skeleton |
| PDF generation | (см. PdfPreviewModal copy выше) |

### Error state copy (Phase 3 additions, Phase 2 общая обработка остаётся)

| Class | Heading | Body | Action |
|-------|---------|------|--------|
| Acts list load failure | Не удалось загрузить акты | {AppError.message} | Кнопка: **Повторить** |
| Counter conflict (override номера занят) | (toast + inline на поле №) | Акт №{N} уже существует. Выберите другой номер. | inline под полем |
| Counter conflict (auto race — practically impossible) | (toast) | Не удалось сгенерировать номер. Попробуйте снова. | — |
| Return: остаток после возврата отрицательный | (inline на поле Кол-во) | Можно вернуть не больше {available} устройств. | — |
| Return: ни одной позиции не отмечено | (inline под subheading таблицы) | Выберите хотя бы одну позицию для возврата. | — |
| Template render error (MiniJinja) | (PdfPreviewModal error state) | Шаблон документа не подготовлен. Обратитесь к администратору. {AppError.message} | **Закрыть** |
| Template timeout (MiniJinja 5s) | (toast) | Шаблон документа слишком сложный (превышено время рендера). Используйте дефолтный шаблон. | — |
| org.json не найден / placeholder | (warning toast при первом старте phase 3 / pdf render attempt) | Файл `org.json` не найден. Создан с временными данными — заполните рядом с приложением. | — |
| logo.png не найден | (warning toast при первом PDF render) | Логотип не найден. PDF сформирован без логотипа. | — |
| PDF render generic failure | (PdfPreviewModal error state) | Не удалось сформировать PDF | {AppError.message} | **Повторить** |

### Standard CTA pattern (Phase 3 reuse Phase 2)

| Element | Copy |
|---------|------|
| Primary CTA (create flow) | Создать акт (императивный глагол + объект, ≤2 слова) |
| Primary CTA (edit flow) | Сохранить |
| Primary CTA (return flow) | Оформить возврат |
| Primary CTA (PDF gen flow) | Сформировать PDF |
| Secondary CTA (cancel) | Отмена |
| Destructive CTA | Удалить (в красном; модальная подтверждалка обязательна) |
| Tertiary action (link-styled) | Сбросить поиск ・ Следующий (auto-number reset) ・ Повторить |

**Запрещено:** «Submit», «OK», «Ок», «Confirm», «Print», `→`, эмодзи. Кнопки — глаголы; ссылки — действие+объект.

---

## Component Inventory (Phase 3 additions)

Phase 3 наследует **все** компоненты Phase 2 без изменений. Ниже — только **новые** или **расширенные** компоненты.

### Reused from Phase 2 (no changes)

`Layout.svelte`, `Sidebar.svelte` (с обновлённым sidebar-config — Акты теперь активный),
`Modal.svelte` (с поддержкой нового size — см. ниже), `Input.svelte`, `Select.svelte`, `Textarea.svelte`,
`Button.svelte`, `Toast.svelte` + `ToastHost.svelte`, `Spinner.svelte`, `Badge.svelte`,
`ThemeSwitcher.svelte`, `Placeholder.svelte` (используется для оставшихся placeholder-разделов).

### Extended

#### `Modal.svelte` — расширение `size` prop

Phase 2: `size: 'md' | 'wide'` (md=640, wide=960).
Phase 3 добавляет: `size: 'md' | 'wide' | 'xwide' | 'pdf-preview'`.

| Size | max-width | Use case |
|------|-----------|----------|
| `md` | 640px (Phase 2) | формы устройств, confirm-диалоги |
| `wide` | 960px (Phase 2) | CSV-импорт preview |
| `xwide` | 1000px (NEW) | ActFormModal — широкий create-акт |
| `pdf-preview` | `min(95vw, 1100px)` (NEW) | PdfPreviewModal — viewer полноразмерный |

`pdf-preview` дополнительно:
- backdrop opacity сильнее (`rgba(0,0,0,0.6)` light / `rgba(0,0,0,0.75)` dark) — focus-mode для просмотра PDF.
- body padding `0` (iframe заполняет полностью).
- body NO scroll (PDF.js viewer внутри iframe скроллит сам).
- height: `var(--pdf-preview-height)` (90vh max).

### New components (in `ui/src/features/acts/`)

#### `ActsPage.svelte`
Route shell для `#/acts`. Иерархия:
```
<h1>Акты</h1>
<header-actions>[+ Создать акт]</header-actions>
<ActsSearchAndTabs/>       <!-- поиск + switch-bar над split'ом -->
<ActsMasterDetail>         <!-- CSS Grid 35% / 65% -->
  <ActsList/>              <!-- left panel -->
  <ActDetail/>             <!-- right panel -->
</ActsMasterDetail>
```

#### `ActsSearchAndTabs.svelte`
Объединяет поиск и switch-bar в одном горизонтальном блоке.
- Поиск-инпут (full-width or 50%, иконка лупы слева, debounce 250ms — reuse Phase 2 D-Search-01).
- Под/рядом — switch-bar tabs (Акты / Возвраты / Архив) с counter-badges. На viewport ≥ 1280 — в одну строку (search 50%, tabs 50%); иначе wrap.
- Click таба → reset pagination на page 1 + новый запрос.
- Counter badges — `Badge`, accent fill на активном, default на idle.

#### `ActsMasterDetail.svelte`
Layout-контейнер `display: grid; grid-template-columns: var(--acts-list-width) var(--acts-detail-width); gap: var(--space-md); align-items: stretch; min-height: calc(100vh - 200px);`.
- Левая колонка: `background: var(--color-surface); border-radius: var(--radius-md); border: 1px solid var(--color-border); overflow: hidden;`
- Правая колонка: `background: var(--color-bg); border-radius: var(--radius-md); border: 1px solid var(--color-border); overflow: auto;`
- На viewport < 1100 → horizontal scroll на `<main>` (как Phase 2 D-UI-Responsive-01); НЕ stacked.

#### `ActsList.svelte`
- Содержит `ActListRow.svelte` × N + footer-пагинацию.
- Selection-state: `selectedActId` через runes; передаётся в `ActDetail`.
- Pagination footer: «1–50 из 1240» слева; центр `< 1 2 3 … 25 >` (как Phase 2 D-UI-Pagination-01).
- Empty state: `Placeholder.svelte` (reuse) с heading/body/action из copy-таблицы.

#### `ActListRow.svelte`
Двухстрочная карточка:
- Высота `--act-list-row-height` (64px).
- Padding `var(--space-md) var(--space-md)`.
- Border-bottom `1px solid var(--color-border)`.
- Hover: `background: var(--color-surface-sunken)` (idle) или `--color-surface` (если выбран).
- Selected: `border-left: 3px solid var(--color-accent); background: color-mix(in srgb, var(--color-accent) 8%, transparent);` (dark: 12%).
- Top line (Body 14/400): `№{number}{suffix}` (semibold 600, tabular-nums) `·` `{date}` (text-secondary).
- Bottom line (Label 13/500): `{receiver_name}` (text-primary) `·` `{items_count} устр.` (text-secondary).
- В табе «Архив» — справа Badge «В архиве» (`Badge variant="default"`).
- Suffix `в`/`в1`/`в2` (для рядов в табе «Возвраты») — `--color-text-secondary` weight regular.
- Click → set selectedActId.

#### `ActDetail.svelte`
Правая панель. Иерархия:
```
<header>
  <h2>№{number}{suffix} от {date}</h2>
  <action-row>
    [Печать] [Редактировать] [Возврат] [⋯]  <!-- ⋯ kebab → Удалить -->
  </action-row>
</header>
<section class="act-header-fields">
  <ActHeaderField label="Сдал" value={giver_name}/>
  <ActHeaderField label="Принял" value={receiver_name}/>
  <ActHeaderField label="Дата"/>
  <ActHeaderField label="Сроком до"/>  <!-- nullable; «—» если null -->
  <ActHeaderField label="Расположение"/>
</section>
<section class="act-items">
  <h3>Позиции ({count})</h3>
  <ActItemsTable/>
</section>
<section class="act-returns" v-if-has-returns>
  <h3>История возвратов</h3>
  <ul><li>№{p}в{s} от {date} — {full|partial}</li>…</ul>
</section>
```
- Empty state (нет выбранного акта): `Placeholder.svelte` с heading «Выберите акт» + body + кнопка `+ Создать акт`.
- Padding: `var(--space-lg)`.
- Sections separated by `var(--space-xl)` margin.

#### `ActHeaderField.svelte`
Display-only поле в шапке акта детали.
- Layout: `<div class="field"><label>{label}</label><div class="value">{value}</div></div>`.
- Label: `font-size: var(--font-size-label); font-weight: 500; color: var(--color-text-secondary); margin-bottom: var(--space-xs);`
- Value: `font-size: var(--font-size-body); color: var(--color-text-primary);`
- Null value → «—» (`--color-text-muted`).

#### `ActItemsTable.svelte`
Read-only таблица позиций в `ActDetail`. Колонки: Устройство (25%), Инв. № (15%), Серийный № (15%), Количество (10% tabular-nums), Состояние (15%), Возврат (20%).
- Reuse table-styles из Phase 2 `DeviceList` (row 40px, header label 13/500, body 14/400).
- Колонка «Возврат» — для каждого item: `—` (не возвращено) или ссылка на return-акт (`color: var(--color-accent); text-decoration: underline on hover`).

#### `ActFormModal.svelte`
- Использует `Modal size="xwide"` (1000px).
- Heading: «Новый акт» / «Редактирование акта».
- Тело: вертикальный flex:
  1. Section «Шапка акта»: 2-column grid (`grid-template-columns: 1fr 1fr; gap: var(--space-md)`), 6 полей.
  2. Section «Позиции»: header «Позиции» + `ActFormItemsTable.svelte` + кнопка `+ Добавить позицию` (ghost variant).
- Footer: actions right-aligned, `[Отмена] [Создать акт]`.
- Submit blocked пока `canSubmit = $derived(...) === false`.
- При submit — button показывает spinner + «Создание…» (как Phase 2 pattern).
- Validation: inline error под каждым полем при `AppError::Validation`; toast для других ошибок.

#### `ActNumberField.svelte`
Специальный input для поля № в `ActFormModal`.
- Layout: `<Input type="number">` + справа inline `Badge` (`«авто»` / `«override»`) + ссылка-кнопка `Следующий` (показывается только в override-mode).
- При mount: запрашивает `acts_peek_next_number()` → подставляет предсказанное значение → badge «авто».
- При user-edit: badge становится `«override»` (warning), кнопка «Следующий» появляется.
- Click «Следующий»: возвращает предсказанное значение, badge «авто».
- Tooltip «авто»: «Следующий по порядку. Можно изменить.»
- Tooltip «override»: «Будет записано в журнал событий.»
- Validation: на blur — `acts_check_number_available(number)` → если занят → inline error «Акт №{N} уже существует».

#### `ActFormItemsTable.svelte`
Inline-editable таблица позиций в `ActFormModal`.
- Колонки: `#` (counter 32px), Устройство (45%), Количество (15% tabular-nums), `⌧` (remove 40px).
- Каждая row: `DeviceAutocompleteField` (reuse Phase 2; **расширить filter-prop'ом** `status_in=['на_складе']` — см. INTERFACE-CHANGES ниже) + `Input type="number" min=1 max={available}` + ghost-icon-button `×`.
- Под row, если device picked: hint «На складе: {available}» (`font-size: var(--font-size-label); color: var(--color-text-muted)`).
- Footer row: `<Button variant="ghost" size="sm">+ Добавить позицию</Button>`.
- Empty state (no rows): «Добавьте хотя бы одну позицию.» (text-muted, по центру, padding `var(--space-xl)`).
- Quantity validation: 1 ≤ qty ≤ available; ошибка → border destructive + inline hint.

#### `ReturnModal.svelte`
- Использует `Modal size="wide"` (960px) или новый `--modal-max-width-acts-return` (880px) — выбрать executor'у; рекомендую 880px (`size="wide"` с CSS-override через class — Modal Phase 2 это допускает).
- Heading: «Возврат по акту №{parent_number}»
- Subheading (под heading, Label 13/500 text-secondary): «Создаст акт возврата №{parent}в{predicted_sub_number}»
- Body:
  1. Section «Применить ко всем выбранным позициям» (bg `--color-surface`, padding `var(--space-md)`, radius `var(--radius-sm)`):
     - Checkbox «Применить ко всем» (default ON, per D-Acts-Return-01).
     - 2-column grid: Состояние (Select+Autocomplete combo or just Input) + Расположение на складе (`DeviceAutocompleteField` для location, фильтр «на складе»).
  2. Section «Позиции к возврату ({checked_count})»:
     - `ReturnItemsTable.svelte`.
- Footer: `[Отмена] [Оформить возврат]`.

#### `ReturnItemsTable.svelte`
Inline таблица возврата.
- Колонки: `☑` (checkbox 40px), Устройство (25%), Кол-во к возврату (15% tabular-nums), Состояние (25%), Расположение (35%).
- Per-row:
  - Checkbox (default ON для всех unrturned items).
  - Устройство — read-only (name + inv_no).
  - Количество — `Input type="number" min=1 max={items_remaining}`. Дефолт = `items_remaining`.
  - Состояние и Расположение:
    - Если row checked AND apply-to-all ON AND user не override этот row → показывается bulk-default, аннотация «(по умолчанию)» (`--color-text-muted` 13px).
    - Если user override → бойс-row inputs + аннотация «(переопределено)» (`--color-warning` 13px).
    - Если row not checked → inputs disabled, visual opacity 0.5.
- Row height `--act-items-row-height` (44px).

#### `PdfPreviewModal.svelte`
- Использует `Modal size="pdf-preview"`.
- Heading: «Печать акта №{number}{suffix}» / «Печать акта возврата ...» / «Печать документа приёма».
- Body: `<iframe src={blobUrl} title="PDF preview"/>` заполняет весь body (no padding).
- Footer:
  - Left: `[Закрыть]` (Button ghost).
  - Right: `[Сохранить как PDF] [Открыть в системном просмотрщике] [Печать]` (Сохранить — primary; остальные — secondary).
- States:
  - Loading: full-body overlay `--color-surface-raised` opacity 0.95 + Spinner.lg + «Готовим PDF…» heading + «Подождите пару секунд…» body. Центрировано (flex).
  - Error: full-body `--color-bg` + error heading «Не удалось сформировать PDF» + body `{AppError.message}` + кнопка `[Повторить]`. Центрировано.
  - Ready: iframe виден; footer actions активны.
- На mount: вызывает `acts_render_pdf(actId)` → `URL.createObjectURL(blob)` → set blobUrl → ready state.
- На unmount: `URL.revokeObjectURL(blobUrl)` для memory cleanup.
- Save: вызывает `tauri-plugin-dialog` save с дефолтным filename, на success — toast «PDF сохранён: {filename}».
- Open in OS: writes blob → `Paths::tmp_pdf()` → `tauri-plugin-shell::open(path)` → toast «Открыли PDF в просмотрщике.»
- Print: `iframe.contentWindow?.print()` → нативный диалог печати ОС → toast «Открыт системный диалог печати.»

#### `DocumentAcceptanceModal.svelte` (DEV-14 entry point)
- Используется со страницы Устройств (Phase 2 раздел).
- `Modal size="md"`.
- Heading: «Документ приёма устройства».
- Body: small recap «Устройство: {device.name} (инв. № {device.inventory_no | "—"})» + form:
  - Кто передал ⃰ — `Input` (free-text ФИО).
  - Кто принял ⃰ — `Input` (free-text ФИО).
  - Дата — `Input type="date"`, default = сегодня.
- Footer: `[Отмена] [Сформировать PDF]`.
- При submit: вызов `devices_render_acceptance_pdf(device_id, giver_name, receiver_name, date)` → onSuccess: закрытие этого модала + открытие `PdfPreviewModal` с heading «Печать документа приёма».
- Error: inline в полях / toast.

### Interface changes на Phase 2 компоненты

**`DeviceAutocompleteField.svelte` — расширить props:**

Add prop: `statusIn?: string[]` (массив status codes, фильтрующий backend autocomplete). Default `undefined` (все статусы).
- Используется в `ActFormItemsTable` со значением `['на_складе']` — autocomplete показывает только устройства на складе.
- Backend `devices_autocomplete` command принимает новый optional argument `status_in`.
- Если не задан — backend ведёт себя как сейчас.

**Backward compat:** существующие use-cases (DeviceFormModal) не передают `statusIn` → поведение прежнее.

**`DeviceContextMenu.svelte` — добавить пункт:**

Добавить пункт меню перед separator-Удалить:
```
| Редактировать                 |
| ─────────────────────────     |
| Печать документа приёма  ← NEW
| ─────────────────────────     |
| Удалить (destructive)         |
```
Click → открывает `DocumentAcceptanceModal` для текущего устройства.

**`sidebar-config.ts` — обновить:**

`{ section: 'Акты', route: '#/acts', icon: 'document', active: true }` (status `active=true` вместо placeholder).

---

## Interaction Patterns (Phase 3 additions)

Phase 3 наследует все Phase 2 interaction patterns (search debounce, theme application, error rendering, loading states, keyboard accessibility, density, responsive, motion). Ниже — **только новые** паттерны.

### Master-detail selection

1. На mount страницы Acts: автоматически выбирается первый акт в табе «Акты» (если есть).
2. Selection state: `selectedActId: number | null` в `acts.svelte.ts` runes-store.
3. Click row в `ActsList` → `selectedActId = row.id`; правая панель обновляется через `$derived`.
4. При смене таба → `selectedActId = null` (или = first row в новом табе — TBD; FLAG-001 ниже).
5. При delete акта → если deleted был selected → `selectedActId = null` (показывает empty state).
6. URL не отражает selection (no `#/acts/42` deep-link в Phase 3 — FLAG-002).

### Switch-bar tab behaviour

1. Click таба → reset pagination на page 1.
2. Counts (acts_counts) загружаются:
   - При mount страницы.
   - После любой successful mutation (create, update, delete, return) — refetch counts.
3. Counts во время рефетча — не показывать spinner; **показывать предыдущее значение** (stale-while-revalidate), чтобы не было визуального flicker.

### Act creation flow

1. Click `+ Создать акт` → `ActFormModal` open, focus → first input (№ поле).
2. На mount модала: `acts_peek_next_number()` → подставить в № + badge «авто».
3. Tab-навигация: № → Дата → Сдал → Принял → Сроком до → Расположение → Items[0].device → Items[0].quantity → + Добавить позицию → ... → [Отмена] [Создать акт].
4. Items autocomplete с фильтром `statusIn=['на_складе']`.
5. Quantity inputs: при выборе device — подставить `value=available`, set max=available.
6. Click `Создать акт`:
   - Button → loading «Создание…».
   - `apiCall('acts_create', payload)`.
   - On success:
     - Toast «Акт №{number} создан. {items_count} устр. в работе.»
     - Close modal.
     - List refresh + counts refresh.
     - Selected = newly created.
   - On error:
     - `Conflict { field:"number" }` → inline на поле № + toast.
     - `Validation` → inline под полем + toast.
     - Other → toast.
7. Esc → close с confirmation если dirty (как Phase 2).

### Override-number flow

1. На mount: peek_next → подставить → badge «авто».
2. User меняет значение в № input → badge становится «override» (warning) + ссылка-кнопка `Следующий` появляется справа.
3. На blur (300ms idle): `acts_check_number_available(N)` → если занят → inline error «Акт №{N} уже существует» + border destructive.
4. Click `Следующий` → reset to peeked value + badge «авто», ссылка скрывается.
5. На submit override → toast info: «Номер №{N} будет записан с пометкой override» (показывается **до** submit, не блокирует).

### Return flow

1. Click `Возврат` в `ActDetail` → `ReturnModal` open.
2. На mount: subheading «Создаст акт возврата №{parent}в{predicted_sub}» — `acts_peek_next_sub_number(parent_id)`.
3. Checkbox «Применить ко всем» = ON; bulk-default Состояние/Расположение пустые → user заполняет.
4. Per-row: все unreturned items checked by default; quantity = items_remaining.
5. User меняет bulk-default или per-row override.
6. Submit:
   - Button → loading «Оформляем возврат…».
   - `apiCall('acts_return', payload)`.
   - On success:
     - Toast «Создан акт возврата №{number}{suffix}. {n} устр. вернулось на склад.»
     - Если `result.parent_archived === true` → дополнительный toast «Акт №{parent} переехал в Архив (все устройства вернулись).»
     - Close modal.
     - List refresh + counts refresh + detail refresh.

### PDF preview flow

1. Click `Печать` (handover/return) или из ContextMenu Devices → `PdfPreviewModal` open в loading state.
2. На mount: `apiCall('acts_render_pdf', {act_id})` или `devices_render_acceptance_pdf`.
3. Result — `Vec<u8>` (PDF bytes) → `Uint8Array` → `Blob([bytes], {type:'application/pdf'})` → `URL.createObjectURL(blob)`.
4. Set `<iframe src={blobUrl}>`.
5. Loading overlay скрывается на `iframe.load` event ИЛИ через 200ms (whichever first).
6. Footer actions становятся активны.
7. На modal close: `URL.revokeObjectURL(blobUrl)`.

### Confirmation flow для destructive actions

1. Click `Удалить` в ActDetail / kebab → `ConfirmModal` (`Modal size="md"`).
2. Heading + body из copy-таблицы.
3. Focus default = `Отмена` (защита от случайного Enter — pattern из Phase 2).
4. Click `Удалить`:
   - Button → loading.
   - `apiCall('acts_delete', {act_id})`.
   - On success:
     - Toast «Акт №{number} удалён. {n} устр. вернулось на склад.»
     - Close confirm modal.
     - List refresh + counts refresh.
     - Selected = null.
5. Phase 3 **не реализует** undo-link в toast (как Phase 2 — out of scope, требует undo-stack в state).

### Search behaviour (Phase 2 D-Search-01 reuse)

1. Debounce 250ms.
2. Empty input → reset to current tab без FTS filter.
3. Non-empty → query через FTS5 join (`acts.number`, `acts.giver_name`, `acts.receiver_name`, `devices_fts` через `act_items.device_id`).
4. Search применяется внутри текущего таба (не cross-tab). FLAG-003: возможно, имеет смысл искать **во всех табах** и показывать «найдено: 3 в Акты, 1 в Архив» — но это complexity для Phase 7.

### Keyboard accessibility (additions)

- Master-detail: ↑/↓ в фокусированном `ActsList` — навигация по строкам (selectedActId меняется).
- Enter в фокусированной row → no-op (selection уже произошёл при focus); reserved для Phase 4+ details-open patterns.
- Esc в PdfPreviewModal — закрывает модал (но **не** прерывает render если в loading state — render продолжается на backend, blob просто не используется).

---

## Registry Safety

| Registry | Blocks Used | Safety Gate |
|----------|-------------|-------------|
| shadcn official | (не применимо) | not required — shadcn не инициализирован (carry-forward Phase 2) |
| third-party | (нет) | not applicable |

**Tool:** none. Phase 3 не использует генератор UI и не вытягивает компоненты из реестров. Все примитивы пишутся вручную или переиспользуются из Phase 2.

### Третьих-сторонних JS-пакетов фазы (новые в Phase 3)

| Package | Version | Source | Audit | Disposition |
|---------|---------|--------|-------|-------------|
| `pdfjs-dist` | `^4.x` (latest stable; точная версия в plan 04) | npm: mozilla | RESEARCH §Package Legitimacy Audit — Approved (Mozilla официально, >10 лет, миллионы downloads/неделю) | **Approved** — для blob-url iframe viewer per D-Print-UX-01 |

**Уже установлены (Phase 2 carry-forward, no re-install):** `svelte-spa-router 5.1.0`, `@tauri-apps/api 2.11.0`, `@tauri-apps/plugin-dialog 2.7.1`.

**Новый Tauri plugin в Phase 3 (Rust + npm):** `@tauri-apps/plugin-shell` (для «Открыть в системном просмотрщике»). Approved — official Tauri publisher.

**Никаких third-party реестровых блоков.** Registry-vetting gate не требуется.

---

## FLAGs (decisions auto-pickled by researcher; can be revisited by checker/executor)

В non-interactive auto-chain mode researcher не задаёт вопросов пользователю. Следующие решения приняты как «sensible defaults» и могут быть пересмотрены checker'ом или executor'ом без блокирования фазы:

- **FLAG-001 (Selection reset on tab switch):** при смене таба `selectedActId = null` (показывается empty state «Выберите акт»). Альтернатива — auto-select первый акт нового таба. Принят null-reset для предсказуемости (user явно выбирает); executor может изменить если UX testing покажет иное.
- **FLAG-002 (No deep-link in URL):** Phase 3 НЕ кодирует `selectedActId` в URL hash (т.е. `#/acts` не превращается в `#/acts/42`). Reload теряет selection. Это упрощает state-management в master-detail. Deep-link можно добавить в Phase 7 как UX-полировку.
- **FLAG-003 (Search scoped to current tab):** поиск применяется в пределах текущего таба (не показывает результаты из других табов). Альтернатива — cross-tab search с «найдено N в Акты, M в Архив» — отложено до Phase 7 если будет реальная необходимость.
- **FLAG-004 (ReturnModal size — 880px):** выбран новый размер `--modal-max-width-acts-return` = 880px (между Phase 2 `wide`=960 и `md`=640). Если executor'у проще использовать `wide` (960), это допустимо — таблица позиций сядет без проблем.
- **FLAG-005 (PDF preview load detection):** loading overlay скрывается на iframe `load` event OR через 200ms timeout (whichever first). Если на разных WebView2 versions iframe load event ненадёжен — fallback на timeout 200ms; executor может увеличить до 500ms если визуально требуется.
- **FLAG-006 (Save dialog filename slugification):** дефолтные filename содержат русские буквы (например, `Акт_приёма-передачи_№42_2026-05-28.pdf`). Tauri-plugin-dialog корректно обрабатывает Unicode на Windows/macOS/Linux. Если на Win7 32-bit (best-effort) возникнут проблемы — fallback ASCII через transliteration (отложено в WIN7-deferred).
- **FLAG-007 (Override toast timing):** info-toast «Номер №{N} будет записан с пометкой override» появляется при **изменении** № в инпуте (не только на submit). Это превентивный warning. Executor может перенести на submit если визуальный шум.
- **FLAG-008 (Auto-archive notification toast):** показывается дополнительным toast'ом после возврата (не интегрирован в основной toast). Альтернатива — объединить в один toast «Создан акт возврата №42в2. {n} устр. на склад. Акт №42 переехал в Архив.» — выбрано раздельно для ясности.
- **FLAG-009 (Items table в ActDetail — read-only):** колонка «Возврат» показывает текст «вернулось {date}» с возможной ссылкой на return-акт; ссылка переключает selectedActId на return-акт (внутрь таба «Возвраты»). Альтернатива — popover; выбрано переключение selection (проще).
- **FLAG-010 (DocumentAcceptanceModal entry-point):** размещён в `DeviceContextMenu` (kebab) на странице Устройств. Альтернатива — отдельная кнопка в `DeviceDetail` (если будет в Phase 3 — не запланировано). Если DeviceDetail появится в Phase 4+, кнопка может дублироваться там.
- **FLAG-011 (Selected row keyboard nav):** ↑/↓ в фокусированном ActsList навигирует по rows. Click vs keyboard — оба работают. Tab НЕ пропускает rows (каждый row tabbable, как Phase 2 pattern для table rows).
- **FLAG-012 (Print button reliability):** `iframe.contentWindow.print()` работает на WebView2 (Windows) и WKWebView (macOS). На WebKitGTK (Linux Tauri) может быть нестабильно. Если plan-execution выявит проблему — fallback: тот же путь, что у «Открыть в системном просмотрщике» (запись в tmp + shell.open) + toast hint.

---

## Checker Sign-Off

- [ ] Dimension 1 Copywriting: PASS (все строки на русском, CTA — verb+noun, empty/error states описаны для каждого таба и модала, два новых destructive действия с confirm)
- [ ] Dimension 2 Visuals: PASS (Component inventory: 10 новых компонентов + 2 расширения; master-detail layout, wide-create modal, PDF preview modal; motion reuse Phase 2; reduced-motion поддержан)
- [ ] Dimension 3 Color: PASS (наследует Phase 2 60/30/10; accent reserved-for list расширен на 3 элемента; контрастность carry-forward AA verified)
- [ ] Dimension 4 Typography: PASS (наследует Phase 2 14/13/20/28; никаких новых размеров; tabular-nums на № и Количество)
- [ ] Dimension 5 Spacing: PASS (наследует Phase 2 8pt scale; новые конструктивные размеры явно в exceptions; никаких произвольных `padding: 15px`)
- [ ] Dimension 6 Registry Safety: PASS (shadcn=none; новый npm `pdfjs-dist` — Mozilla официально, approved; tauri-plugin-shell — official Tauri)

**Approval:** pending (checker валидирует)

---

## Pre-Population Sources

| Section | Source | Fields |
|---------|--------|--------|
| Design System | CLAUDE.md + Phase 2 UI-SPEC + CONTEXT D-Print-UX-01 / D-PDF-Engine-01 | Tool=none, PDF font=DejaVu, UI font=system stack, pdfjs-dist для preview |
| Spacing | Phase 2 UI-SPEC inherit + новые конструктивные `--acts-list-width`, `--modal-max-width-acts-*`, `--pdf-preview-*`, `--act-list-row-height`, `--act-items-row-height` | full carry-forward + Phase 3 layout-сizes |
| Typography | Phase 2 UI-SPEC inherit (4 sizes, 2 weights) | без изменений |
| Color | Phase 2 UI-SPEC inherit (60/30/10 light + dark) + Phase 3 accent additions (3 new) + warning usage | full carry-forward + 3 accent + 4 warning use-cases |
| Sidebar update | UI-01 + CONTEXT (Акты активируется в Phase 3) | «Акты» из placeholder → active |
| Acts copy | REQUIREMENTS ACT-01..14 + DEV-14..15 + CONTEXT D-Acts-* | switch-bar labels, modal headings, error messages |
| Switch-bar semantics | CONTEXT D-Acts-List-01 | tab filters mapped to backend predicates |
| Master-detail layout | CONTEXT D-Acts-List-01 (35/65 split, fixed in Phase 3) | grid template, NO resizer (Phase 7 deferred) |
| Create modal | CONTEXT D-Acts-Create-01 (1000px width + override-badge) | `Modal size="xwide"` + ActNumberField |
| Return modal | CONTEXT D-Acts-Return-01 (bulk + per-row override + default ON apply-all) | ReturnModal + ReturnItemsTable |
| PDF preview | CONTEXT D-Print-UX-01 (pdfjs-dist iframe + 3 actions) | PdfPreviewModal + new Modal size |
| DEV-14 acceptance entry | REQUIREMENTS DEV-14 + CONTEXT (context-menu) | DeviceContextMenu пункт + DocumentAcceptanceModal |
| Destructive confirm | Phase 2 pattern + CONTEXT D-Undo-01 (restore from audit_log) | 2 new destructive actions with explicit warning copy |
| Registry safety | CLAUDE.md + RESEARCH §Package Legitimacy Audit | pdfjs-dist Mozilla approved |
| Interaction patterns | Phase 2 UI-SPEC inherit + CONTEXT D-Print-UX-01 + D-Acts-* | master-detail selection, switch-bar tab behaviour, PDF flow |

---

## UI-SPEC COMPLETE
