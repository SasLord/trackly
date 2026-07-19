---
phase: 26
slug: windows-with-mockup
status: draft
shadcn_initialized: false
preset: none
created: 2026-07-19
revised: 2026-07-19
---

# Phase 26 — UI Design Contract

> Визуальный и интеракционный контракт двух окон с готовым макетом (Дашборд, Устройства) и общей
> админской оболочки. Составлен gsd-ui-researcher, проверяется gsd-ui-checker.
>
> **Главный артефакт этого документа — §3 «Таблица значений» (D-18).** Верификатор сверяет её
> построчно с кодом. Всё остальное — рамка вокруг неё.

---

## Design System

| Property | Value |
|----------|-------|
| Tool | none — рукописный слой SCSS custom properties (стек Tauri + Svelte 5 + SCSS зафиксирован CLAUDE.md; это не React/shadcn-проект) |
| Preset | not applicable |
| Component library | none — собственные примитивы Фаз 24–25 (`Button`, `Input`, `Checkbox`, `Badge`, `Tabs`, `Table`, `TableRow`, `Dropdown`, `Modal`) |
| Icon library | none — только inline SVG на `currentColor` |
| Font | системный стек `--tr-font-family` (portable-ограничение: никаких внешних/CDN-шрифтов) |
| Token layer | `ui/src/styles/_tokens.scss` — **единственный** слой. Closed-world гейт `scripts/check-tokens.mjs` роняет сборку на ссылку на несуществующий `--tr-*` |
| Source of truth (значения) | `.planning/reference/design-system-v2/Окно · Дашборд.dc.html`, `… Окно · Список устройств.dc.html`, `… Foundations.dc.html` |
| shadcn gate | **не применяется** — проект не на React/Next/Vite-React; `components.json` отсутствует и не нужен. Registry safety gate: not applicable |

---

## 1. Как читать этот документ

1. Формат Design Canvas (`<x-dc>`, `DCLogic`, `{{ }}`) — **спецификация, а не переносимый код**.
   Разметка `.dc` НИКОГДА не копируется в Svelte. Извлекаются только числа и имена токенов из
   `renderVals()` и inline-`style`.
2. Все значения ниже приведены **дословно из `.dc`** с указанием файла и строки. Downstream не
   пересчитывает и не «улучшает» их.
3. Где макет и работающее приложение расходятся по **содержанию** — действует правило D-10 Фазы 25:
   **макет задаёт форму, приложение задаёт содержание**. Отсюда D-01/D-03/D-04/D-08 этой фазы.
4. Расхождения с уже утверждёнными компонентами Фаз 24–25 не «чинятся молча» — они перечислены в
   §10 «Принятые отклонения». Компоненты Фазы 24 — тоже утверждённый контракт, и переписывать их
   ради 2px в этой фазе дороже, чем зафиксировать отклонение.

Сокращения в колонке «Действие»: **NEW** — писать с нуля, **CHG** — менять существующее значение,
**OK** — в коде уже правильно, только подтвердить, **KEEP** — оставить как есть осознанно.

---

## 2. Границы: что этот контракт НЕ покрывает

- Три панели дашборда из макета — «Низкий остаток» (с %), «Последние заявки», «Мониторинг
  картриджей». Данных нет в `DashboardWidgetDto` (D-01). Их значения из `.dc` **намеренно не
  внесены** в §3 — верификатору нечего там сверять.
- Кнопка «+ Создать акт» в шапке дашборда (D-03).
- Строки и групповые строки таблицы устройств — закрыты Фазой 25 (D-12). `DeviceListRow`,
  `DeviceGroupRow` не трогаются.
- `EmployeeLayout` (D-09, Фаза 29).
- Семь неперенесённых окон (D-13) — их визуальный регресс принят.
- AA-контраст и focus-ring по новому дизайну — QA-02/QA-03, Фаза 30. Здесь фиксируется только
  то, что focus-ring **не удаляется** ни с одного интерактивного элемента.

---

## 3. ТАБЛИЦА ЗНАЧЕНИЙ (D-18) — машинно-проверяемый контракт

Источники: `Д` = `Окно · Дашборд.dc.html`, `У` = `Окно · Список устройств.dc.html`.
Ссылка вида `Д:41` — файл и номер строки.

### 3.1 Сайдбар — контейнер

| Свойство | Значение макета | Источник | Сейчас в коде | Действие |
|---|---|---|---|---|
| `width` | `236px` | Д:41, У:44 | `var(--sidebar-width)` = **240px** | **CHG** — `--sidebar-width: 236px` в `_tokens.scss` (layout-константа, не `--tr-*`, гейт токенов не затрагивается) |
| `background` | `var(--tr-bg)` | Д:41 | `var(--tr-surface)` | **CHG** — инверсия D-06 |
| `border-right` | `1px solid var(--tr-border)` | Д:41 | то же | OK |
| `flex` | `none`, `display:flex; flex-direction:column` | Д:41 | то же | OK |

### 3.2 Сайдбар — шапка с логотипом (NEW, D-08)

| Свойство | Значение макета | Источник | Действие |
|---|---|---|---|
| контейнер | `display:flex; align-items:center; gap:9px` | Д:42 | NEW |
| `height` | `56px` (= `var(--header-height)`) | Д:42 | NEW |
| `padding` | `0 16px` | Д:42 | NEW |
| `border-bottom` | `1px solid var(--tr-border)` | Д:42 | NEW |
| текст | `16px` / `600`, «Trackly», цвет наследуется (`--tr-text-primary`) | Д:42 | NEW |
| квадрат-логотип | `width:11px; height:11px; border-radius:3px; background:var(--tr-accent)` | Д:43 | NEW |

Квадрат — декоративный: `aria-hidden="true"`, текст «Trackly» несёт смысл.

### 3.3 Сайдбар — навигация

| Свойство | Значение макета | Источник | Сейчас в коде | Действие |
|---|---|---|---|---|
| `nav` контейнер | `flex:1; padding:8px; overflow-y:auto; display:flex; flex-direction:column; gap:1px` | Д:45 | `padding: 8px 0`, без `gap` | **CHG** |
| разделитель | `height:1px; background:var(--tr-border); margin:6px 8px` | Д:47 | `margin: 4px 16px` | **CHG** |
| пункт: `height` / `line-height` | `38px` / `38px` | Д:280 | `40px` (`--row-height`) | **CHG** |
| пункт: `padding` | `0 12px` | Д:280 | `0 16px` | **CHG** |
| пункт: `border-radius` | `6px` (`--tr-radius-sm`) | Д:280 | нет | **CHG** |
| пункт: `font-size` | `14px` | Д:280 | то же | OK |
| пункт неактивный | `color:var(--tr-text-secondary); background:transparent; font-weight:400; box-shadow:none` | Д:280 | то же | OK |
| пункт активный | `color:var(--tr-accent-text); background:var(--tr-accent-soft); font-weight:600; box-shadow:inset 3px 0 0 var(--tr-accent)` | Д:280 | `border-left:3px` + `color-mix(--tr-accent 10%)` + `weight 500` | **CHG** — акцентная полоса переезжает с `border-left` на `inset box-shadow` (иначе неактивные пункты нужны с прозрачной рамкой и ломается `padding`) |
| hover неактивного | в макете не задан | — | `color-mix(--tr-text-primary 5%)` | KEEP — hover обязателен, макет статичен |

> Активное состояние в коде вешается через `use:active` → класс `.is-active`, который живёт вне
> scope-хэша компонента и поэтому объявлен как `:global(.nav-link.is-active)` **в `<style lang="scss">`
> Svelte-компонента** — это работает. Урок Фазы 24 про `:global()` касается только plain `.scss`
> файлов (`global.scss`), не scoped-блоков компонентов. Не «чинить» это.

### 3.4 Сайдбар — футер

| Свойство | Значение макета | Источник | Сейчас в коде | Действие |
|---|---|---|---|---|
| контейнер | `padding:14px 16px; border-top:1px solid var(--tr-border)` | Д:51 | `padding:16px` | **CHG** |
| блок пользователя | `flex-direction:column; padding-bottom:10px; margin-bottom:10px; border-bottom:1px solid var(--tr-border)` | Д:52 | `padding-bottom:8px; margin-bottom:4px` | **CHG** |
| имя | `14px` / `600` | Д:53 | `14px` / `500` | **CHG** |
| роль | `12px`, `var(--tr-text-tertiary)` | Д:53 | `13px` | **CHG** |
| блок темы | `flex-direction:column; gap:7px` | Д:55 | `gap:4px` | **CHG** |
| подпись темы | `12px`, `var(--tr-text-secondary)`, текст **«Оформление»** | Д:56 | `13px`, `--tr-text-tertiary`, текст «Тема» | **CHG** (D-08) |
| кнопка «Выйти» | в макете отсутствует | — | есть | **KEEP** — D-08, удаление = регресс функциональности |

### 3.5 Сегмент-контрол темы (`ThemeSwitcher`)

| Свойство | Значение макета | Источник | Сейчас в коде | Действие |
|---|---|---|---|---|
| обёртка | `display:flex; padding:2px; gap:2px; background:var(--tr-surface-sunken); border:1px solid var(--tr-border); border-radius:8px` | Д:214 | `height:32px; background:var(--tr-surface); radius:4px; overflow:hidden`, без padding/gap | **CHG** |
| кнопка | `flex:1; height:26px; border:none; border-radius:6px; padding:0 4px; font-size:12px; font-weight:600` | Д:215 | `height:100%; border-right:1px; font-size:13px` | **CHG** |
| кнопка активная | `color:var(--tr-text-primary); background:var(--tr-surface-raised); box-shadow:var(--tr-elev-1)` | Д:215 | `background:var(--tr-surface-raised)` | **CHG** (добавить тень + цвет) |
| кнопка неактивная | `color:var(--tr-text-tertiary); background:transparent; box-shadow:none` | Д:215 | `--tr-text-secondary` | **CHG** |
| переход | `background .12s, color .12s` | Д:215 | `transition: none` | **CHG** — совпадает с микро-переходами .12s Фазы 24 (D-09) |
| порядок кнопок | Светлая · Системная · Тёмная | Д:216 | Светлая · Тёмная · Системная | **CHG** |

Разделители `border-right` между сегментами макетом не предусмотрены — убрать, форму задаёт
`gap: 2px` на sunken-подложке.

### 3.6 PageHeader — вариант `fixed` (Дашборд)

| Свойство | Значение макета | Источник | Действие |
|---|---|---|---|
| контейнер | `display:flex; align-items:center; justify-content:space-between; gap:16px; flex:none` | Д:67 | NEW |
| `height` | `56px` | Д:67 | NEW |
| `padding` | `0 24px` | Д:67 | NEW |
| `border-bottom` | `1px solid var(--tr-border)` | Д:67 | NEW |
| титул `h1` | `margin:0; font-size:20px; font-weight:600` | Д:68 | NEW |
| блок действий | `display:flex; align-items:center; gap:10px` | Д:69 | NEW |

### 3.7 PageHeader — вариант `wrap` (Устройства)

| Свойство | Значение макета | Источник | Сейчас в коде (`DevicesPage`) | Действие |
|---|---|---|---|---|
| контейнер | `display:flex; align-items:center; justify-content:space-between; gap:16px; flex-wrap:wrap; flex:none` | У:71 | то же | OK (переезжает в `PageHeader`) |
| `padding` | `16px 24px` | У:71 | `24px 32px` | **CHG** |
| `border-bottom` | `1px solid var(--tr-border)` | У:71 | то же | OK |
| титул `h1` | `margin:0; font-size:20px; font-weight:600` | У:72 | `--tr-font-size-h3` (20px) / 600 | OK |
| блок действий | `display:flex; gap:8px` | У:73 | `gap:8px; flex-wrap:wrap` | OK |

### 3.8 Тело страницы

| Свойство | Дашборд | Устройства | Источник | Действие |
|---|---|---|---|---|
| контейнер | `flex:1; overflow-y:auto` | `flex:1; overflow:auto` | Д:80, У:80 | **CHG** |
| `padding` | `24px` | `20px 24px` | Д:80, У:80 | **CHG** |
| `background` | `var(--tr-surface)` | `var(--tr-surface)` | Д:80, У:80 | **CHG** (D-06) |

`Layout.svelte` **снимает** `padding: var(--tr-space-xl)` и `background: var(--tr-bg)` с `.content`
(D-07): отступ и фон теперь принадлежат телу страницы, шапка идёт до краёв.

### 3.9 Селекты периода (Дашборд)

| Свойство | Значение макета | Источник | Сейчас в коде | Действие |
|---|---|---|---|---|
| `height` | `32px` | Д:70 | `32px` | OK |
| `padding` | `0 8px` | Д:70 | `0 var(--tr-space-xs)` = 8px | OK |
| `border-radius` | `6px` (`--tr-radius-sm`) | Д:70 | `--tr-radius-xs` (4px) | **CHG** |
| `border` | `1px solid var(--tr-border-strong)` | Д:70 | `var(--tr-border)` | **CHG** |
| `background` | `var(--tr-surface)` | Д:70 | `var(--tr-bg)` | **CHG** |
| `font-size` | `13px` (`--tr-font-size-label`) | Д:70 | то же | OK |
| `gap` между контролами | `10px` | Д:69 | `8px` | **CHG** |

### 3.10 Карточка статистики (`StatWidget`)

| Свойство | Значение макета | Источник | Сейчас в коде | Действие |
|---|---|---|---|---|
| `border` | `1px solid var(--tr-border)` | Д:84 | то же | OK |
| `border-radius` | `8px` (`--tr-radius-md`) | Д:84 | то же | OK |
| `padding` | `16px` (`--tr-space-md`) | Д:84 | `24px` | **CHG** |
| `background` | `var(--tr-surface)` | Д:84 | то же | OK |
| `box-shadow` | `var(--tr-elev-1)` | Д:84 | нет | **CHG** |
| `min-width` | `0` | Д:84 | нет | **CHG** (обязательно — иначе grid-колонка не сожмётся) |
| `min-height` | не задан | — | `120px` | **KEEP** — D-14: высота в состоянии загрузки не должна прыгать |
| заголовок | `font-size:13px; color:var(--tr-text-secondary)` | Д:85 | `14px` / `600` / `--tr-text-primary` | **CHG** |
| строка значения | `display:flex; align-items:baseline; gap:8px; margin-top:6px` | Д:86 | нет | **CHG** |
| число | `font-size:30px; font-weight:700; line-height:1; font-variant-numeric:tabular-nums` | Д:87 | `30px` / `600` / `lh 1.2`, без tabular | **CHG** |
| единица | `font-size:13px; color:var(--tr-text-tertiary)` | Д:88 | `13px` / `--tr-text-secondary`, отдельной строкой | **CHG** — переезжает на одну базовую линию с числом |
| ряд пилюль | `display:flex; flex-wrap:wrap; gap:6px; margin-top:14px` | Д:90 | `<ul>` списком | **CHG** |
| пилюля | `display:inline-flex; align-items:center; gap:5px; padding:3px 9px; border-radius:11px; background:var(--tr-surface-sunken); font-size:12px; color:var(--tr-text-secondary); white-space:nowrap` | Д:92 | — | **NEW** |
| значение в пилюле | `<strong>`: `color:var(--tr-text-primary); font-variant-numeric:tabular-nums` | Д:92 | — | **NEW** |

Пилюли строятся **локальной разметкой `StatWidget`, а не компонентом `Badge`**: `Badge` (Фаза 24)
даёт `height:20px; border-radius:10px; padding:0 8px` и не поддерживает пару «label + strong».
Натягивать его пропами дороже и хуже читается.

### 3.11 `warningItems` — предупреждение о низком остатке (D-04, содержание приложения)

Макет этот блок не показывает. Форма выводится из системы, а не изобретается:

| Свойство | Значение | Обоснование |
|---|---|---|
| контейнер | `margin-top:14px; padding:8px 10px; border-radius:6px` | тот же ритм, что ряд пилюль |
| `background` | `var(--tr-warning-soft)` | семантическая пара токенов Фазы 23 |
| `border` | `1px solid transparent` → **убрать сплошной `--tr-warning`** | сейчас 1px сплошной warning — слишком громко рядом с карточкой на `elev-1` |
| текст заголовка | `font-size:12px; font-weight:600; color:var(--tr-warning-text)` | шкала пилюль |
| элементы списка | `font-size:12px; color:var(--tr-text-secondary)`, `list-style:none`, `gap:2px` | тот же 12px-регистр |
| копирайт | «Низкий остаток:» — **не менять** | работающая функциональность SC #3 |

### 3.12 Карточка графика (`ChartWidget`)

| Свойство | Значение макета | Источник | Действие |
|---|---|---|---|
| карточка | `border:1px solid var(--tr-border); border-radius:8px; padding:18px; background:var(--tr-surface); box-shadow:var(--tr-elev-1); min-width:0` | Д:100 | **CHG** |
| шапка карточки | `display:flex; align-items:center; justify-content:space-between; margin-bottom:20px` | Д:101 | **CHG** |
| заголовок `h3` | `margin:0; font-size:16px; font-weight:600` | Д:102 | **CHG** |
| ряд переключателей периода | `display:flex; gap:14px` | Д:103 | **CHG** |
| кнопка периода | `background:transparent; border:none; padding:2px 1px 5px; font-size:13px; border-bottom:2px solid …` | Д:254 | **CHG** |
| период активный | `font-weight:600; color:var(--tr-accent-text); border-bottom-color:var(--tr-accent)` | Д:254 | **CHG** |
| период неактивный | `font-weight:500; color:var(--tr-text-secondary); border-bottom-color:transparent` | Д:254 | **CHG** |
| область графика | `display:flex; gap:10px` | Д:109 | **CHG** |
| ось Y | `flex-direction:column; justify-content:space-between; height:210px; padding-bottom:20px; font-size:11px; color:var(--tr-text-tertiary); text-align:right` | Д:110 | **CHG** |
| поле построения | `flex:1; position:relative; height:210px` | Д:113 | **CHG** |
| линии сетки | `position:absolute; height:1px; background:var(--tr-border)`; контейнер `top:0; bottom:20px` | Д:114, Д:239 | **CHG** |
| ширина столбца | `16px` при ≤3 мес., `10px` при ≤6, иначе `6px` | Д:240 | **CHG** |
| зазор столбцов | `5px` / `3px` / `2px` (те же пороги) | Д:241 | **CHG** |
| столбец | `border-radius:2px 2px 0 0` | Д:249 | **CHG** |
| подпись значения | `font-size:9px; font-weight:600; color:var(--tr-text-secondary)`, `bottom:h%`, `transform:translate(-50%,-3px)` | Д:250 | **CHG** — 9px требует отдельного подтверждения глазами, см. §5 и §12 п.6 |
| подпись месяца | `height:20px; line-height:20px; font-size:11px; color:var(--tr-text-tertiary)` | Д:128 | **CHG** |
| легенда | `display:flex; flex-wrap:wrap; gap:16px; margin-top:16px; padding-top:14px; border-top:1px solid var(--tr-border)` | Д:134 | **CHG** |
| элемент легенды | `display:inline-flex; align-items:center; gap:7px; font-size:13px; color:var(--tr-text-secondary)` | Д:136 | **CHG** |
| маркер легенды | `width:10px; height:10px; border-radius:50%` | Д:137 | **CHG** |
| палитра рядов | `#3b6fe0`, `#1a9d5f`, `#d8820e` | Д:232–234 | **CHG** — см. §7, согласованное исключение |

`PeriodToggle.svelte` перестаёт быть tab-подобной полосой и переходит на `pStyle` из Д:254.
Роль контейнера остаётся `role="group"` с `aria-label="Период графика"` — это не табы.

### 3.13 Фильтры устройств (`DeviceFilters`, D-10)

| Свойство | Значение макета | Источник | Сейчас в коде | Действие |
|---|---|---|---|---|
| контейнер | `display:flex; flex-direction:column; gap:12px` | У:83 | `gap:8px` | **CHG** |
| | `padding-bottom:12px; border-bottom:1px solid var(--tr-border); margin-bottom:14px` | У:83 | `padding-bottom:8px; margin-bottom:16px` | **CHG** |
| обёртка поиска | `position:relative; display:flex; align-items:center` | У:84 | то же | OK |
| иконка-лупа | `position:absolute; left:12px; color:var(--tr-text-tertiary)`, SVG `16×16` | У:85–86 | `left:8px` | **CHG** |
| поле поиска | `width:100%; height:36px; padding:0 12px 0 34px; border-radius:6px` | У:88 | `padding-left: calc(8px*2+16px)`=32px, `radius:4px` | **CHG** → примитив `Input` |
| поле поиска: `border` | `1px solid var(--tr-border-strong)` | У:88 | `var(--tr-border)` | **CHG** (примитив `Input` уже даёт `--tr-border-strong`) |
| поле поиска: `background` | `var(--tr-surface)` | У:88 | `var(--tr-bg)` | **CHG** |
| ряд «табы + чекбокс» | `display:flex; align-items:center; justify-content:space-between; gap:16px; flex-wrap:wrap` | У:90 | то же | OK |
| контейнер табов | `display:flex; gap:2px; border-bottom:1px solid var(--tr-border); overflow-x:auto` | У:91 | `gap:2px; overflow-x:auto`, **без `border-bottom`** | **CHG** |
| таб | `height:34px; padding:0 12px; font-size:14px; border-bottom:2px solid …; margin-bottom:-1px; border-radius:6px 6px 0 0; gap:6px` | У:234 | самописный `.status-tab` | **CHG** → примитив `Tabs variant="underline"` |
| таб активный | `font-weight:600; color:var(--tr-accent-text); border-bottom-color:var(--tr-accent)` | У:234 | `color:var(--tr-accent)`, `weight 500` | **CHG** |
| счётчик в табе | `min-width:18px; height:18px; padding:0 5px; border-radius:9px; font-size:11px; font-weight:600` | У:235 | `padding:0 4px`, `weight 500` | **CHG** |
| счётчик активный | `background:var(--tr-accent-soft); color:var(--tr-accent-text)` | У:235 | `color-mix(--tr-accent 15%)` | **CHG** |
| счётчик неактивный | `background:var(--tr-surface-sunken); color:var(--tr-text-secondary)` | У:235 | то же | OK |
| чекбокс: бокс | `18×18; border-radius:5px; border:1.5px solid` | У:242 | нативный `16×16` `accent-color` | **CHG** → примитив `Checkbox` |
| чекбокс отмечен | `border-color:var(--tr-accent); background:var(--tr-accent)`, галочка `--tr-on-accent` | У:242 | — | **CHG** |
| чекбокс не отмечен | `border-color:var(--tr-border-strong); background:var(--tr-surface)` | У:242 | — | **CHG** |
| подпись чекбокса | `font-size:14px; color:var(--tr-text-secondary)`, gap `9px` | У:96–98 | `14px`, `--tr-text-secondary`, gap `4px` | **CHG** |

**Жёсткий контракт D-10 (риск, а не деталь):** поведение фильтров не меняется — FTS-поиск с
дебаунсом 250 мс, `onSearchChange` / `onStatusChange` / `onGroupedChange` сохраняют сигнатуры,
пять статусов и порядок `Все · На складе · В работе · На ремонте · Списано` неизменны, счётчик
«Все» по-прежнему сумма остальных.

### 3.14 Рамка таблицы (D-11) и её содержимое

| Свойство | Значение макета | Источник | Сейчас в коде | Действие |
|---|---|---|---|---|
| внешняя рамка | `border:1px solid var(--tr-border); border-radius:8px; overflow:hidden; box-shadow:var(--tr-elev-1)` | У:104 | нет | **NEW** → в `Table.svelte` |
| внутренний скроллер | `overflow-x:auto` | У:105 | `.tr-table-wrapper { overflow-x:auto }` | OK (становится внутренним слоем) |
| `<table>` | `width:100%; border-collapse:collapse; font-size:14px; min-width:860px` | У:106 | всё кроме `min-width` | **CHG** |
| `<th>` | `text-align:left; font-size:12px; font-weight:600; color:var(--tr-text-secondary); padding:0 10px; white-space:nowrap` | У:196 | то же | OK |
| `<th>` высота | `36px` | У:196 | `34px` (`.tr-thead-row`) | **CHG** |
| `<th>` подложка | `background:var(--tr-bg); border-bottom:2px solid var(--tr-border-strong)` | У:196 | то же | OK |
| последняя колонка | `width:40px` | У:197 | `.th-actions` в `DeviceList` | KEEP |
| футер таблицы | `padding:9px 14px; border-top:1px solid var(--tr-border); font-size:13px; color:var(--tr-text-secondary); background:var(--tr-bg)` | У:137 | `padding:8px 16px`, без `background`, без размера шрифта на контейнере | **CHG** — см. §6.5 |

**Важно:** `overflow:hidden` на рамке и `overflow-x:auto` на внутреннем слое — это два разных
элемента. Слить их в один нельзя: `overflow:hidden` нужен, чтобы скруглённые углы обрезали
`thead`-подложку, а `overflow-x:auto` — чтобы горизонтально ехала только таблица, а не рамка.

### 3.15 Кнопки шапки (Устройства)

| Свойство | Значение макета | Источник | Примитив `Button` (Фаза 24) | Действие |
|---|---|---|---|---|
| `height` | `36px` | У:241 | `.btn-md { height:36px }` | OK |
| `padding` | `0 16px` | У:241 | `0 var(--tr-space-md)` | OK |
| `border-radius` | `6px` | У:241 | `--tr-radius-sm` | OK |
| `font-size` / `weight` | `14px` / `600` | У:241 | то же | OK |
| primary | `background/border:var(--tr-accent); color:var(--tr-on-accent)` | У:252 | то же | OK |
| secondary | `background:var(--tr-surface); color:var(--tr-text-primary); border-color:var(--tr-border-strong)` | У:253 | сверить | **проверить** |

Кнопки шапки — единственный блок, где `.dc` и Фаза 24 сходятся почти без правок. Если
`Button` `variant="secondary"` уже даёт эти три значения — трогать нечего.

### 3.16 Фокальная точка каждого окна

Что глаз ловит первым — проектное решение, а не побочный эффект. Проверяется прищуром
(squint-тест) на обеих темах.

| Окно | Фокальная точка | Чем удерживается | Что НЕ должно её перебивать |
|---|---|---|---|
| **Дашборд** | Ряд из четырёх чисел статистики — `30px/700` на `line-height:1` | Самый крупный кегль на экране (следующий — 20px титул); плотный вес и `tabular-nums` дают четыре тяжёлых якоря сразу под шапкой | Заголовки карточек намеренно `13px --tr-text-secondary`; график ниже сгиба и построен на `--tr-text-tertiary`; сплошного акцента на дашборде нет вовсе |
| **Устройства** | Primary-кнопка «+ Создать устройство» в шапке | Единственная сплошная `--tr-accent`-заливка на всём окне; две соседние кнопки намеренно secondary (`--tr-surface` + `--tr-border-strong`) | Табы фильтров несут акцент только в активном состоянии и только как 2px-подчёркивание + `soft`-счётчик; рамка таблицы держится на `--tr-border` и `elev-1`, без цвета; выделение строк не вводится (D-11 Фазы 25) |

Следствие для обоих окон: **на экране ровно одна сплошная акцентная заливка** (primary-кнопка на
Устройствах; на Дашборде — ни одной). Всё остальное акцентное — `--tr-accent-soft` и
`--tr-accent-text`. Появление второй сплошной заливки — дефект.

---

## 4. Spacing Scale

Объявленная шкала (`_tokens.scss`, Фаза 23, **не меняется в этой фазе**):

| Токен | Значение | Использование в фазе |
|---|---|---|
| `--tr-space-3xs` | 2px | зазор сегмент-контрола, зазор табов |
| `--tr-space-2xs` | 4px | — |
| `--tr-space-xs` | 8px | `padding` nav-контейнера, зазор кнопок шапки, `gap` строки значения карточки |
| `--tr-space-sm` | 12px | `gap` фильтров, `padding` табов, отступ иконки поиска |
| `--tr-space-md` | 16px | `gap` сетки дашборда, `padding` карточки статистики, `padding` шапки сайдбара |
| `--tr-space-lg` | 20px | вертикальный `padding` тела Устройств |
| `--tr-space-xl` | 24px | `padding` тела страниц, горизонтальный `padding` шапок окон |
| `--tr-space-2xl` … `5xl` | 32/40/48/64px | не используются в этой фазе |

**Отклонение от правила «только кратные 4» — объявлено осознанно.** Макет содержит внутренние
величины компонентов, не лежащие на шкале: `3px`, `5px`, `6px`, `7px`, `9px`, `10px`, `11px`,
`14px`, `18px`, `26px`, `34px`, `36px`, `38px`, `40px`, `210px`, `860px`. При конфликте между
попиксельным соответствием макету (D-18, оно же SC #1/#2) и абстрактным 4-point-правилом
**побеждает макет**. Эти значения пишутся литералами и перечислены в §7 — новых токенов под них
не заводится.

---

## 5. Typography

Шкала Фазы 23 не меняется. Роли, задействованные в фазе:

| Роль | Размер | Вес | Line-height | Где |
|---|---|---|---|---|
| Число статистики | 30px | 700 | 1 | `StatWidget` значение (**вес 700 — литерал**, `--tr-font-weight-*` даёт максимум 600) |
| `--tr-text-h3` | 20px | 600 | 1.3 | титул окна в `PageHeader` |
| `--tr-text-subtitle` | 16px | 600 | 1.4 | «Trackly» в сайдбаре, `h3` карточки графика |
| `--tr-text-body` | 14px | 400 | 1.5 | пункты навигации, поле поиска, таблица, подпись чекбокса |
| `--tr-text-body-strong` | 14px | 600 | 1.5 | активный пункт навигации, имя пользователя, метки табов, кнопки |
| `--tr-text-label` | 13px | 500 | 1.4 | заголовок карточки, единица измерения, селекты, футер таблицы, легенда |
| `--tr-text-caption` | 12px | 500 | 1.35 | `<th>` (вес 600), пилюли, роль пользователя, подпись «Оформление» |
| `--tr-text-micro` | 11px | 500 | 1.3 | подписи осей графика, счётчики в табах |
| 9px | 9px | 600 | — | подпись значения над столбцом графика — **литерал вне шкалы, под условием** (ниже) |

Табличные числа: `font-variant-numeric: tabular-nums` обязателен на числе статистики, значениях
в пилюлях и mono-ячейках таблицы (уже есть — класс `.tr-mono`).

### 9px — единственный размер под условием приёмки

`9px` (Д:250) лежит на две ступени ниже нижней ступени шкалы (`--tr-text-micro`, 11px). Значение
берётся из макета дословно, но **проходит отдельный пункт визуального UAT** (§12, п.6): подписи
над столбцами графика читаются в **тёмной** теме на `--tr-text-secondary` `#9aa3b4` поверх
`--tr-surface` `#161b23`. Тёмная тема здесь риск-носитель: тонкие мелкие глифы на тёмном фоне
«съедаются» сильнее, чем на светлом.

Если UAT покажет, что цифры не читаются — **разрешённая правка ровно одна**: поднять до `11px`
(`--tr-text-micro`, тот же размер, что подписи месяцев и осей — визуально согласовано). Промежуточный
`10px` не использовать: он не даёт ни соответствия макету, ни попадания на шкалу. Правка
оформляется как отклонение O-11 в §10, а не как молчаливое изменение.

---

## 6. Разрешения по «Claude's Discretion»

### 6.1 API компонента `PageHeader` (NEW)

`ui/src/lib/components/PageHeader.svelte`:

```ts
interface Props {
  /** Титул окна — <h1>, единственный на странице. */
  title: string;
  /**
   * 'fixed' — height:56px; padding:0 24px  (Дашборд, Д:67)
   * 'wrap'  — padding:16px 24px; flex-wrap (Устройства, У:71)
   */
  variant?: 'fixed' | 'wrap';
  /** Кнопки/селекты справа. Отсутствует — блок действий не рендерится вовсе. */
  actions?: Snippet;
}
```

Решение и обоснование:
- **`title` — проп, а не snippet.** Во всех 11 окнах это простая строка; snippet добавил бы
  синтаксический шум на каждом вызове и увёл бы `<h1>` из компонента, а он должен быть ровно один.
- **`actions` — snippet, а не проп-массив.** Содержимое разнородно (кнопки, селекты, а в Фазах
  27–28 будут и дропдауны); описывать это данными — тупик.
- **`variant`, а не набор булевых пропов.** Макет даёт ровно две геометрии; булевы флаги допускают
  бессмысленные комбинации.
- Бургер-кнопка (§6.3) рендерится **внутри** `PageHeader` слева от титула и скрыта чистым CSS
  выше 1024px — без `matchMedia` и без JS-ветвления.
- `PageHeader` **не** содержит `padding` тела страницы: тело — сосед-элемент со своим
  `flex:1; overflow:auto` (D-07).

### 6.2 Брейкпоинты

CSS custom properties **не работают в `@media`** — брейкпоинты не могут быть `--tr-*` токенами.
Заводится `ui/src/styles/_breakpoints.scss` с SCSS-переменными; файл `@use`-ится точечно теми
компонентами, которым он нужен (`check-tokens.mjs` его не касается — там нет `--tr-*`).

| Переменная | Значение | Что происходит |
|---|---|---|
| `$bp-xl` | `1280px` | сетка статистики `repeat(4,1fr)` → `repeat(2,1fr)` |
| `$bp-lg` | `1024px` | сайдбар уходит из потока в выезжающую панель; появляется бургер; `padding` тела `24px` → `16px` |
| `$bp-md` | `768px` | `PageHeader` перестаёт быть однострочным: титул и действия в две строки, `variant="fixed"` теряет фиксированную высоту (`min-height:56px`); ряд фильтров становится колонкой |
| `$bp-sm` | `560px` | сетка статистики → `1fr`; `padding` тела `16px` → `12px`; кнопки шапки растягиваются `flex:1` |

Пороги выведены из системы, а не из макета (D-16): 1280px — точка, где четыре карточки по 16px
gap перестают держать 30px-число без переноса; 1024px — типичная граница планшета в портрете при
сайдбаре 236px; 768px/560px — стандартные планшет/телефон.

### 6.3 Механика сворачивания сайдбара

- **≥ 1024px:** как сейчас — `grid-template-columns: 236px 1fr`, сайдбар `position:sticky; height:100vh`.
- **< 1024px:** grid превращается в одну колонку; `<aside>` получает
  `position:fixed; inset-block:0; left:0; width:236px; z-index:60; transform:translateX(-100%);
  transition:transform .18s ease; box-shadow:var(--tr-elev-3)`. Открытая панель —
  `transform:translateX(0)`.
- **Подложка:** `position:fixed; inset:0; background:var(--tr-overlay); z-index:55`, рендерится
  только при открытой панели.
- **Состояние:** крошечный рун-стор `ui/src/features/layout/layout-state.svelte.ts`
  (`export const sidebarNav = $state({ open: false })`). Не стор темы, не глобальный контекст —
  одно булево, доступное `Layout` и `PageHeader`.
- **Закрытие:** клик по подложке, `Escape`, клик по любому пункту навигации, смена маршрута.
- **Доступность:** у бургера `aria-expanded` и `aria-controls`, `aria-label` «Открыть меню» /
  «Закрыть меню»; при открытии фокус переходит на первую ссылку навигации, при закрытии —
  возвращается на бургер; закрытая панель получает `inert`; при открытой панели `overflow:hidden`
  на `<body>`.
- **Motion:** `@media (prefers-reduced-motion: reduce) { transition: none }`.
- **Бургер:** `36×36; border-radius:6px; background:transparent; border:none; color:var(--tr-text-secondary)`,
  hover `background:var(--tr-row-hover)`, inline-SVG 18px. Скрыт `display:none` выше `$bp-lg`.

### 6.4 Во что превращается строка таблицы на узкой ширине

**Ни во что — таблица остаётся таблицей и едет горизонтально.** `min-width: 860px` на `<table>`
плюс `overflow-x:auto` на внутреннем слое рамки (У:105–106) — это и есть предписанное макетом
поведение.

Обоснование, а не удобство: превращение строки в карточку требует переписать `DeviceListRow` и
`DeviceGroupRow`, которые D-12 прямо запрещает трогать (они закрыты Фазой 25 по `TableRows.dc`).
Карточный режим — это новая функциональность, а не адаптив редизайна. Добавляется только
`-webkit-overflow-scrolling: touch` и `scrollbar-gutter: stable` на скроллере.

### 6.5 Как рамка встраивается в `Table` без поломки потребителей

**Фактическая карта потребителей `Table.svelte` (проверено grep, а не по памяти):**

| Потребитель | Файл | Риск |
|---|---|---|
| Список устройств | `ui/src/features/devices/DeviceList.svelte` | целевой |
| Витрина CMP-06 | `ui/src/features/showcase/sections/TableSection.svelte` | визуальный |
| ~~`ActFormItemsTable`~~ | `ui/src/features/acts/ActFormItemsTable.svelte` | **потребителем НЕ является** |

> **Поправка к 26-CONTEXT.md (D-11).** `ActFormItemsTable.svelte` импортирует `Button`, `Spinner`,
> `Dropdown` и рисует собственную разметку таблицы — `Table.svelte` он не использует. Потребителей
> два, а не три. Планировщику не нужно закладывать регресс-проверку этого файла.

**Решение:**

```ts
// добавляется к существующим пропам Table.svelte
/** Рамка по макету: border + radius 8px + overflow hidden + elev-1. */
framed?: boolean;   // default: true
/** Полоса итога внутри рамки, под скроллером таблицы. */
footer?: Snippet;   // default: undefined — полоса не рендерится
```

- `framed` по умолчанию `true`: рамка — это новая норма дизайн-системы, шесть таблиц Фаз 27–28
  получат её бесплатно. Витрина тоже получит рамку — это улучшение, а не регресс, и его нужно
  осмотреть глазами в UAT.
- `footer` нужен потому, что по макету полоса итога лежит **внутри** рамки (У:104 → У:137), а
  сейчас она живёт в `DeviceList.svelte` рядом с `<Table>`, снаружи.

**Ограниченное исключение из «не трогать `DeviceList.svelte`».** D-12 запрещает трогать *строки,
футер и пустые состояния как поведение*. Здесь требуется механическое перемещение уже существующей
разметки футера в `{#snippet footer()}` и передача его в `<Table>`. Границы правки для
верификатора — жёсткие:

- разрешено: обернуть существующий `<footer class="list-footer">…` в `{#snippet footer()}…{/snippet}`,
  передать `{footer}` в `<Table>`, удалить обёртку `.device-list-wrapper` (её `overflow-x` уезжает
  в рамку);
- запрещено: менять условие `{#if !skeletonLoading && !isEmpty}`, тексты «Показано N из M» /
  «Групп: N», `emptyMessage`/`emptySubtext`, любую логику `$derived`.

Если планировщик сочтёт это неприемлемым нарушением D-12 — **запасной вариант**: `Table` получает
только `framed`, полоса итога остаётся снаружи рамки, и это фиксируется в §10 как принятое
отклонение от У:104. Первый вариант точнее по макету; выбор — за планировщиком, но он должен быть
явным, а не случайным.

### 6.6 Расширение `Input` иконкой (следствие D-10)

Макетный поиск — это `Input` с иконкой слева (У:85–88), а у примитива Фазы 24 слота под иконку нет.
Добавляется обратносовместимо:

```ts
/** Иконка слева внутри поля. Задана — поле получает padding-left:34px. */
iconLeft?: Snippet;
```

Без пропа разметка и метрики не меняются ни на одном из существующих вызовов. Иконка позиционируется
`position:absolute; left:12px; color:var(--tr-text-tertiary); pointer-events:none`.

---

## 7. Токены: ничего нового

**Новых `--tr-*` токенов фаза не вводит.** Все цвета, тени и радиусы макета уже есть в
`_tokens.scss` после Фаз 23–25. `check-tokens.mjs` должен пройти без правок аллоу-листа.

Одно изменение вне `--tr-*`-пространства:

| Константа | Было | Стало | Почему |
|---|---|---|---|
| `--sidebar-width` | `240px` | `236px` | Д:41 / У:44. Layout-константа, не `--tr-*` — гейт токенов её не проверяет |

**Согласованные литералы (не токенизируются).** Заводить под каждое из них токен — это раздуть
слой ради одного места использования:

`3px` (радиус логотипа) · `5px` (радиус чекбокса) · `9px` `7px` `6px` `5px` (внутренние gap'ы) ·
`11px` (размер логотипа, радиус пилюли) · `10px` (padding ячеек, маркер легенды) · `14px` (`padding`
футера сайдбара, `gap` легенды) · `18px` (`padding` карточки графика, размер счётчика) · `26px`
(высота сегмента) · `34px` (высота таба, левый `padding` поиска) · `36px` (высота `th`, полей,
кнопок) · `38px` (высота пункта навигации) · `40px` (ширина колонки действий) · `210px` (высота
графика) · `860px` (`min-width` таблицы) · `9px`/`30px` (шрифты вне шкалы) · вес `700` (число
статистики).

**Палитра рядов графика — задокументированное исключение.** `#3b6fe0`, `#1a9d5f`, `#d8820e`
(Д:232–234) — это data-viz-палитра, намеренно живущая вне семантического слоя: ряды должны
различаться между собой, а не нести смысл «успех/предупреждение». Значения одинаковы в обеих темах
(так в макете). Записываются литералами в `ChartWidget`; `check-tokens.mjs` их не видит.

---

## 8. Color: 60/30/10 и обе темы

| Роль | Токен | Светлая | Тёмная | Использование |
|---|---|---|---|---|
| Доминанта (60%) | `--tr-surface` | `#ffffff` | `#161b23` | тело страницы, карточки, таблица, поля ввода |
| Вторичная (30%) | `--tr-bg` | `#eef1f6` | `#0e1218` | сайдбар, `thead` таблицы, футер таблицы |
| Вторичная (утопленная) | `--tr-surface-sunken` | `#e4e8f0` | `#0a0d12` | пилюли, подложка сегмент-контрола, неактивные счётчики |
| Акцент (10%) | `--tr-accent` / `-soft` / `-text` | `#2b5fd9` | `#5b8bff` | см. список ниже |
| Семантика | `--tr-warning-*` | `#b9720c` | `#e5a13a` | блок «Низкий остаток» в карточке картриджей |
| Семантика | `--tr-danger-text` | `#b02f2f` | `#ff8080` | текст ошибки загрузки виджета и графика |

**Акцент зарезервирован строго за:** фоном primary-кнопки · квадратом логотипа · активным пунктом
навигации (`soft`-фон + `inset 3px` полоса + `accent-text`) · активным табом фильтров (подчёркивание
+ `accent-text` + `soft`-счётчик) · активной кнопкой периода графика · отмеченным чекбоксом ·
`focus-ring`. **Не** за hover'ом строк, **не** за обычными ссылками, **не** за границами полей.

### Инверсия D-06 в двух темах — что реально проверять

Наивная формулировка «в тёмной теме инверсия читается наоборот» неверна и её не надо тиражировать:
`--tr-bg` темнее `--tr-surface` в **обеих** темах, поэтому сайдбар в обеих выглядит утопленным
относительно контента. Настоящий риск другой:

1. **Карточка на теле — один и тот же `--tr-surface`.** Карточка статистики, карточка графика и
   рамка таблицы стоят на теле того же цвета. Их отделяют только `1px --tr-border` и `--tr-elev-1`.
2. **В тёмной теме `--tr-elev-1` = `0 1px 2px rgba(0,0,0,.5)`** — на `#161b23` эта тень почти не
   видна. Значит всю работу по разделению делает `--tr-border` `#272e3a`. **UAT обязан отдельно
   подтвердить, что границы карточек и рамка таблицы читаются в тёмной теме** — это единственное
   место, где макет физически не может быть «просто скопирован».
3. **`thead` и футер таблицы на `--tr-bg`** дают контраст в обе стороны: светлее контента в тёмной,
   темнее — в светлой. Это работает и проверяется быстро.
4. **D-13 подтверждён кодом:** после перевода тела на `--tr-surface` карточки семи неперенесённых
   окон, нарисованные на `--tr-surface`, сольются с фоном. Регресс принят, шим не вводится.

---

## 9. Copywriting Contract

Всё — по-русски (ограничение v1). Контракт исчерпывающий: любая строка, которую видит пользователь
в двух окнах этой фазы, есть в таблице. Изменяется ровно четыре строки; остальное сохраняется
дословно.

| Элемент | Копирайт | Где в коде | Статус |
|---|---|---|---|
| Логотип сайдбара | «Trackly» | `Sidebar.svelte` (новый блок) | NEW (D-08) |
| Подпись переключателя тем | «Оформление» | `Sidebar.svelte:74` | **CHG** — было «Тема» (D-08) |
| Кнопки сегмент-контрола | «Светлая» · «Системная» · «Тёмная» | `ThemeSwitcher.svelte:4-8` | порядок CHG, тексты те же |
| Кнопка выхода | «Выйти» / «Выход…» | `Sidebar.svelte:69` | KEEP (D-08) |
| Primary CTA (Устройства) | «+ Создать устройство» | `DevicesPage.svelte` шапка | KEEP |
| Secondary (Устройства) | «Импорт CSV» · «Экспорт CSV» | `DevicesPage.svelte` шапка | KEEP |
| Primary CTA (Дашборд) | **отсутствует** | — | D-03 — «+ Создать акт» не заводится |
| Титулы окон | «Дашборд» · «Устройства» | обе страницы | KEEP |
| Пустое состояние таблицы (поиск) | «По вашему запросу ничего не найдено» / «Попробуйте изменить поисковый запрос или сбросить фильтр статуса.» | `DeviceList.svelte:51-57` | KEEP (Фаза 25) |
| Пустое состояние таблицы (нет данных) | «Устройств пока нет» / «Создайте первое устройство или импортируйте список из CSV.» | `DeviceList.svelte:51-57` | KEEP (Фаза 25) |
| Итог таблицы | «Показано N из M» / «Групп: N» | `DeviceList.svelte:110-114` | KEEP |
| **Ошибка загрузки виджета** | **«Не удалось загрузить. Смените период или обновите страницу.»** | **`StatWidget.svelte:41` И `ChartWidget.svelte:225` — ДВА файла** | **CHG** — было «Ошибка загрузки» в обоих |
| **Пустое состояние графика** | «Нет данных о расходе за выбранный период» | `ChartWidget.svelte:227` | KEEP |
| Предупреждение в карточке | «Низкий остаток:» + список моделей | `StatWidget.svelte:54` | KEEP (D-04) |
| Табы фильтра | «Все» · «На складе» · «В работе» · «На ремонте» · «Списано» | `DeviceFilters.svelte:41-47` | KEEP |
| Чекбокс группировки | «Группировать похожие» | `DeviceFilters.svelte:110` | KEEP |
| Периоды графика | «3 мес.» · «6 мес.» · «12 мес.» | `PeriodToggle.svelte:21` | KEEP |
| Бургер-меню | `aria-label` «Открыть меню» / «Закрыть меню» | `PageHeader.svelte` (новый) | NEW |
| Пропустить к контенту | «Перейти к основному содержимому» | `Layout.svelte:12` | KEEP |

### Почему строка ошибки меняется именно так

«Ошибка загрузки» (и равно «Не удалось загрузить») — констатация сбоя без пути решения. Кнопки
повтора в интерфейсе нет: `StatWidget.svelte:41` и `ChartWidget.svelte:225` рендерят голую строку,
а `reloadWidgets()` (`DashboardPage.svelte:128`) висит только на `onchange` селектов периода —
пользователь не догадается, что смена периода перезапросит данные.

Правка **чисто копирайтная**: в строку дописывается путь решения существующими средствами —
«Смените период или обновите страницу». Оба выхода реально работают уже сейчас (селект дёргает
`reloadWidgets()`, для графика — `$effect` на `windowMonths`). Кнопка «Повторить» **не добавляется**:
это новая функциональность, а D-14 фиксирует, что состояния загрузки и ошибок меняют только вид.

**Строка живёт в двух файлах.** Правка одного `StatWidget` оставит `ChartWidget` со старым текстом
— расхождение внутри одного окна. Планировщик обязан завести оба файла в одну задачу.

**Деструктивных действий в фазе нет.** Ни одно окно не получает удаления, сброса или необратимой
операции — фаза чисто визуальная. Диалогов подтверждения не добавляется и не убирается.

---

## 10. Принятые отклонения от макета

Каждое — осознанное. Верификатор не должен заводить их как дефекты.

| # | Отклонение | Почему |
|---|---|---|
| O-1 | Зазор табов: `Tabs` даёт `gap:4px`, макет — `2px` | `Tabs` — утверждённый контракт Фазы 24 и живёт в витрине; правка ради 2px регрессирует три места ради одного |
| O-2 | `gap` чекбокса: `Checkbox` даёт `10px`, макет — `9px` | там же |
| O-3 | Цвет подписи чекбокса: `Checkbox` даёт `--tr-text-primary`, макет — `--tr-text-secondary` | scoped-стили не переопределяются снаружи без `:global()`-хаков; 1px-разница в тоне не стоит хака |
| O-4 | Три панели дашборда не строятся; график — во всю ширину вместо сетки `1.7fr/1fr` | D-01/D-02: данных нет в `DashboardWidgetDto` |
| O-5 | Кнопки «+ Создать акт» нет | D-03 |
| O-6 | Блок `warningItems` в карточке «Картриджи» — макетом не предусмотрен | D-04: работающая функциональность SC #3 |
| O-7 | Кнопка «Выйти» и ролевая навигация — макетом не показаны | D-08: удаление = регресс |
| O-8 | Состав пунктов навигации отличается от `navItems` в `.dc` | `sidebar-config.ts` — ролевая, макет статичен |
| O-9 | `height:720px; overflow:hidden` корневого контейнера `.dc` | приём макетной витрины; в приложении — `100vh` |
| O-10 | Скроллбар-поведение и `hover`-состояния | макет статичен, hover обязателен |
| O-11 | *(условное, заводится только по итогам UAT)* подпись значения над столбцом графика `9px` → `11px` | §5: активируется, только если 9px не читается в тёмной теме. Если UAT пройден — отклонения нет |

---

## 11. Действия за пределами этой фазы (обязательны до планирования)

1. **`REQUIREMENTS.md` — WIN-01/WIN-02 не покрывают адаптивность.** D-15 (полная адаптивность до
   мобильных ширин, брейкпоинты, выезжающий сайдбар) — сознательное расширение объёма поверх
   формулировки «соответствие макету». Оба `.dc` десктопные, мобильных раскладок в них нет.
   Требуется новое требование (например `WIN-12: Адаптивность окон Дашборд и Устройства`) либо
   расширение формулировок WIN-01/WIN-02. **Без этого верификатор не найдёт основания для §6.2–6.4
   и обязан будет пометить их как не подтверждённые требованиями.**
2. **`ROADMAP.md` §Phase 26 — SC #1 буквально невыполним при D-01.** «Дашборд визуально
   соответствует макету (виджеты, отступы, тональность)» → «…соответствует макету **в части
   реализованных виджетов**». Тот же приём, что D-01 Фазы 25.
3. **`ROADMAP.md` — копипаста планов Фазы 25 сидит в ПЯТИ секциях, не в одной.** Проверено
   построчно: блок `**Plans**: 7 plans` + список `- [x] 25-01-PLAN.md … 25-07-PLAN.md` (все
   помечены выполненными) продублирован в секциях **Phase 26** (стр. ~535), **Phase 27** (~568),
   **Phase 28** (~601), **Phase 29** (~633) и **Phase 30** (~665). Чистить надо все пять: если
   почистить только Фазу 26, подсчёт прогресса соврёт уже на следующей фазе, и ровно так же на
   каждой последующей. Каждая из пяти секций должна получить пустой `Plans:` и корректный счётчик.

---

## 12. Процедура верификации (D-18)

1. **Построчная сверка §3** — каждая строка со статусом CHG/NEW проверяется по коду. Строки OK
   подтверждаются, а не переписываются.
2. **Гейты:** `pnpm lint`, `pnpm svelte-check`, `node scripts/check-tokens.mjs`, `pnpm --dir ui build`.
3. **Сборка перед браузерной проверкой:** `pnpm --dir ui build` обязателен — серверный режим отдаёт
   `ui/dist`, а `cargo tauri dev` хотрелоадит только desktop-webview (память проекта).
4. **Проверять собранный CSS, а не исходник** (урок Фазы 24): значения подтверждаются в
   `ui/dist/assets/*.css`. `grep -c` по минифицированному однострочному CSS считает строки —
   использовать `grep -o … | wc -l`.
5. **Визуальный UAT — оба окна × обе темы × четыре ширины** (1440 / 1200 / 900 / 480). Отдельным
   пунктом: читаемость границ карточек и рамки таблицы в тёмной теме (§8, п.2).
6. **Отдельный пункт UAT: подписи значений над столбцами графика (9px) в тёмной теме.** Смотреть
   на реальном экране, не на скриншоте с зумом. Провал → правка O-11 (9px → 11px), см. §5.
7. **Строка ошибки проверяется в ОБОИХ файлах** — `StatWidget` и `ChartWidget` (§9). Способ вызвать:
   остановить бэкенд / отключить сеть при открытом дашборде.
8. **Регресс-проверка двух потребителей `Table`:** список устройств и витрина CMP-06.
9. **Человеческий чекпоинт визуального сравнения с `.dc` — только живой подписью.**
   `workflow.auto_advance` для него отключается: Фаза 24 потеряла два раунда на авто-одобрении,
   маскировавшем непроверенную витрину.

---

## Checker Sign-Off

- [ ] Dimension 1 Copywriting: PASS
- [ ] Dimension 2 Visuals: PASS
- [ ] Dimension 3 Color: PASS
- [ ] Dimension 4 Typography: PASS
- [ ] Dimension 5 Spacing: PASS
- [ ] Dimension 6 Registry Safety: PASS (not applicable — реестры не используются)

**Approval:** pending
