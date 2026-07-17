# Phase 24: Базовые компоненты (base-components) - Context

**Gathered:** 2026-07-18
**Status:** Ready for planning

<domain>
## Phase Boundary

Фаза 24 доставляет **пять базовых примитивов на дизайн-системе `--tr-*`** (из Фазы 23) и **постоянную
витрину-галерею** для их визуальной проверки — и ничего больше. Конкретно:

- **Button** (`ui/src/lib/components/Button.svelte`) — 5 вариантов (primary/secondary/destructive/ghost/link)
  × 2 размера (sm 28px / md 36px) × 6 состояний (обычное/наведение/фокус/нажатие/отключено/загрузка).
- **Поля ввода** — Input/Select/Textarea (существуют) + **Checkbox И Radio** (создаются с нуля) в состояниях
  обычное/фокус/ошибка/отключено.
- **Badge** (`ui/src/lib/components/Badge.svelte`) — 5 тонов × 4 стиля (soft/solid/dot/счётчик-пилюля).
- **Tabs** (создаётся с нуля) — **два стиля**: switch-bar с подчёркиванием+счётчиками И сегментированный.
- **Modal** (`ui/src/lib/components/Modal.svelte`) — оверлей + шапка + тело + футер; тень `--tr-elev-3`, радиус 12px.

Требования: **CMP-01, CMP-02, CMP-03, CMP-04, CMP-05**.

**В границах фазы:**
- Приведение 5 примитивов к значениям из `.dc.html`-референсов Claude Design (источник истины по стилям).
- Создание новых компонентов Checkbox, Radio, Tabs (их пока нет в проекте).
- Обратносовместимое расширение API Badge (новый `appearance`, см. D-08).
- **Постоянная витрина компонентов** — маршрут за логином админа, показывающий все 5 примитивов во всех
  состояниях/вариантах (зеркало `.dc`-галерей). Расширяется фазами 25–30.
- Возврат микро-переходов .12s с подавлением на время смены темы (D-09).

**Вне границ:**
- **Ретрофит существующих экранов** на новые Tabs/Checkbox (самодельные фильтр-бары в Заявках/Картриджах/
  Настройках, raw-чекбоксы) — фазы 26–28. Фаза 24 строит только компоненты + витрину.
- Таблицы и Dropdown — фаза 25. Окна — фазы 26–29.
- Форма focus-ring по новому дизайну, AA-контраст, паритет Tauri vs LAN-браузер — QA-02/QA-03, фаза 30.
- Печатные HTML-шаблоны актов — вне scope дизайн-системы (REQUIREMENTS.md).
- Любые изменения бизнес-логики/API.

</domain>

<decisions>
## Implementation Decisions

### Витрина компонентов (verification surface)

- **D-01:** **Строится постоянная страница-витрина** (не throwaway). Она — основная поверхность для UAT
  Фазы 24 и остаётся в проекте как живая галерея дизайн-системы для фаз 25–30.
  **Обоснование:** у Button 30 комбинаций «вариант×состояние», а Checkbox/Radio/Tabs вообще не имеют мест
  использования в приложении (ретрофит отложен, D-05) — без витрины их негде увидеть и проверить.
- **D-02:** **Доступ к витрине — за логином админа** (пункт навигации, видимый только роли admin; как
  существующая ролевая модель). Не dev-only-флаг и не публичный скрытый URL — пользователь выбрал ролевое
  ограничение. Точка интеграции с роутингом/навигацией/авторизацией — см. code_context.
- **D-03:** **Охват витрины в Фазе 24 — все 5 примитивов** со всеми состояниями/вариантами (зеркало пяти
  `.dc`-референсов). Витрина спроектирована так, чтобы фазы 25+ добавляли свои секции (таблицы, dropdown, окна).

### Объём сверх минимума требований

- **D-04:** **Radio-кнопки строятся в этой фазе**, хотя CMP-02 упоминает только Checkbox. Референс
  `Fields.dc` уже содержит radio (тот же 18px box-примитив, круглая рамка + точка `--tr-on-accent`) —
  делается заодно с Checkbox. Пригодится для выбора типа устройства и т.п. в фазах 26–28.
- **D-05:** **Tabs получает ОБА стиля** — `underline` (switch-bar с подчёркиванием + счётчики, CMP-04) и
  `segmented` (пилюли Список/Карта/Таблица, `.dc` §«Сегментированный вариант»). Оформить как проп
  `variant: 'underline' | 'segmented'`.
- **D-06:** **Badge — 5 тонов** (neutral/accent/success/warning/danger), НЕ 4. Референс `Badges.dc` и
  текущий `Badge.svelte` уже используют 5; «4 тона» в CMP-03/Success-Criteria #3 — недочёт формулировки
  требования. **Действие для планировщика/верификатора: обновить CMP-03 и Success Criteria #3 в
  REQUIREMENTS.md / ROADMAP.md с «4 тона» на «5 тонов»** (иначе верификация фазы упадёт на этом расхождении).

### Ретрофит (внедрение в экраны)

- **D-07:** **Ретрофита нет.** Фаза 24 создаёт только примитивы + витрину. Существующие ~5 самодельных
  фильтр-баров (`RequestsSearchAndTabs`, `CartridgesSearchAndTabs`, `SettingsSubNav`, вкладки в `ActsPage`,
  `CartridgeFilters`) и raw-чекбоксы НЕ мигрируются здесь — это работа фаз 26–28 (согласуется с решением
  Фазы 23 отдать экраны фазам 26–29). Граница чёткая, риск регресса в рабочих экранах = 0.

### API и обратная совместимость

- **D-08:** **Badge — обратносовместимое расширение API.** Добавляется проп
  `appearance: 'soft' | 'solid' | 'dot' | 'count'` с **дефолтом `soft`** (= текущий визуал). Существующий
  проп-тон сохраняется, чтобы 15 текущих вызовов (`variant="default"` ×6, `"success"` ×1, `"warning"` ×2)
  продолжали работать без правок. Внутренний маппинг имён тонов: `default → neutral`, `destructive → danger`
  (референс использует neutral/danger). Точная форма пропа тона (оставить имя `variant` или добавить алиас
  `tone`) — на усмотрение планировщика, критерий один: **ни один из 15 call-site не трогается**.
- **D-09:** **Микро-переходы возвращаются, глушатся на время смены темы.** На компоненты возвращается
  `transition: background .12s, box-shadow .12s` (как в референсах, для hover/focus/active). Чтобы не было
  «наплыва» цветов при переключении темы (ради чего Фаза 23 ставила `transition: none`), при смене темы на
  корень (`documentElement`) на ~1 кадр вешается класс, глушащий все переходы (`* { transition: none !important }`),
  затем снимается. Правится хук переключения темы (`ThemeSwitcher.svelte` / соответствующий стор). Блок
  `@media (prefers-reduced-motion: reduce)` в `global.scss` уже глушит переходы — сохранить его поведение.

### Claude's Discretion

- Структура витрины (один компонент vs секции-партиалы), её маршрут-имя и точное место в навигации/меню
  админа — при условии соблюдения D-02 (только admin).
- Форма пропа тона Badge (`variant` с алиасом vs новый `tone`) — при соблюдении D-08 (0 правок call-site).
- Механика однокадрового подавления переходов (rAF vs setTimeout, имя класса, где живёт хук) — при
  соблюдении D-09.
- Дробление фазы на планы/волны. Разумная развязка: план на витрину-каркас зависит от планов компонентов;
  либо компонент-план сразу добавляет свою секцию в витрину.
- Точные значения состояний берутся ДОСЛОВНО из `.dc`-референсов — не пересчитывать (см. canonical_refs).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Референсы Claude Design (источник истины по стилям — читать первыми)
Формат Design Canvas (`<x-dc>`, `DCLogic`, `{{ }}`, требует `support.js`) — **спецификация, а не
переносимый код**. Разметка НИКОГДА не копируется в Svelte; извлекаются только значения стилей из
`styleFor`/`renderVals`/`ctrlBase` в embedded-скриптах. Значения `--tr-*` в `:root`/`[data-theme]` блоках
этих файлов ДОЛЖНЫ совпадать с `ui/src/styles/_tokens.scss` (сверять при расхождении).

- `.planning/reference/design-system-v2/Buttons.dc.html` — Button: base(size), styleFor(variant,state,size)
  со всеми 5 вариантами × 6 состояний; spinner-стиль загрузки; sm=28px/13px/pad12, md=36px/14px/pad16;
  border 1px solid transparent на всех; secondary def bg=`--tr-surface`; focus добавляет focusBorder на
  secondary/ghost/link; disabled opacity .45; link — underline+offset, focus снимает подчёркивание+radius 4px.
- `.planning/reference/design-system-v2/Fields.dc.html` — Input/Select/Textarea + **Checkbox И Radio**:
  ctrlBase (36px, radius 6px, bg `--tr-surface`, border `--tr-border-strong`); focus=`--tr-accent`+ring 3px;
  error=`--tr-danger`+`--tr-danger-ring`; disabled=`--tr-surface-sunken`. Checkbox: 18px box, radius 5px,
  border 1.5px, checked bg `--tr-accent`, галочка `--tr-on-accent`; radio: круг + точка 8px.
- `.planning/reference/design-system-v2/Badges.dc.html` — 5 тонов (neutral/accent/success/warning/danger)
  × стили soft/solid/dot/counter-pill; pill h=22px, radius 11px, 12px/600; точка 7px; счётчик-пилюли (2 формы).
- `.planning/reference/design-system-v2/Tabs.dc.html` — tabStyle(state) для switch-bar (h=34px,
  borderBottom 2px, active=`--tr-accent` подчёркивание + `--tr-accent-text`, badge-счётчик со своими
  состояниями) + segStyle (сегментированный, h=28px, active=`--tr-surface`+shadow).
- `.planning/reference/design-system-v2/Modal.dc.html` — оверлей `--tr-overlay`, тень `--tr-elev-3`,
  радиус 12px, шапка с заголовком+закрытием, тело формы, футер (btnSecondary/btnPrimary).
- `.planning/reference/design-system-v2/Foundations.dc.html` — первоисточник токенов (для сверки спорных
  значений). `support.js` — рантайм Design Canvas (не трогать, только для открытия `.dc` в браузере).

### Контракт токенов (из Фазы 23 — читать перед стилизацией)
- `.planning/phases/23-design-tokens-foundations/23-CONTEXT.md` — решения по слою `--tr-*`, инверсия
  поверхностей, `.tr-mono`, греп-гейт/скрипт-гейт (D-04/D-08 Фазы 23 защищают эту фазу от старых токенов).
- `.planning/phases/23-design-tokens-foundations/23-UI-SPEC.md` — все hex `--tr-*` (light+dark), шкала
  типографики, motion-решение (micro-transitions разрешены и относятся к этой фазе).
- `ui/src/styles/_tokens.scss` — единственный слой токенов; здесь искать реальные имена/значения `--tr-*`
  (напр. `--tr-elev-3`, `--tr-overlay`, `--tr-danger-ring`, `--tr-accent-soft`, `--tr-radius-lg`=12px).

### Требования и роадмап
- `.planning/ROADMAP.md` §«Phase 24: Базовые компоненты» — цель, 5 Success Criteria, зависимость от Фазы 23.
- `.planning/REQUIREMENTS.md` — CMP-01..CMP-05 (**CMP-03 требует правки 4→5 тонов, см. D-06**).

### Код, который меняется/создаётся
- `ui/src/lib/components/Button.svelte` — сейчас `transition: none`, opacity .5, secondary transparent, нет
  active-состояний → привести к `Buttons.dc` (D-09 возвращает переходы).
- `ui/src/lib/components/Input.svelte` — сейчас bg `--tr-bg`/border `--tr-border` → референс хочет
  bg `--tr-surface`/border `--tr-border-strong`.
- `ui/src/lib/components/Select.svelte`, `Textarea.svelte` — привести к `ctrlBase` из `Fields.dc`.
- `ui/src/lib/components/Badge.svelte` — расширить API по D-08.
- `ui/src/lib/components/Modal.svelte` — привести к `Modal.dc` (elev-3, radius 12px).
- **Новые:** `Checkbox.svelte`, `Radio.svelte`, `Tabs.svelte` + компонент(ы) витрины — аналогов в проекте нет.
- `ui/src/lib/components/Spinner.svelte` — переиспользуется в Button loading (уже так).
- `ui/src/lib/components/ThemeSwitcher.svelte` (или тема-стор) — точка правки для D-09 (подавление переходов).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **Button/Input/Select/Textarea/Badge/Modal уже существуют** и уже на `--tr-*` (после Фазы 23) — работа в
  основном визуальная доводка до референса, а не создание с нуля (кроме Checkbox/Radio/Tabs/витрины).
- `Spinner.svelte` — используется в Button при `loading`; референс спиннера в `Buttons.dc` совпадает по идее.
- Тема переключается через `[data-theme]` на корне + `color-scheme` per theme; theme-scoped `--tr-elev-*`.
- `@media (prefers-reduced-motion: reduce)` в `global.scss:47` уже глушит transitions — учесть при D-09.

### Established Patterns
- Стили — scoped `<style lang="scss">` в каждом компоненте; глобальные классы почти отсутствуют
  (`.skip-link`, `.tr-mono`). Витрина и подавление переходов (D-09) могут потребовать нового глобального
  класса — допустимо, по образцу `.tr-mono`.
- Пропсы через Svelte 5 runes (`$props`, `$bindable`, `$derived`), `Snippet` для children (см. Button/Input).
- Фронтенд-тестов нет (ни vitest, ни playwright); проверки — `pnpm lint` (eslint+prettier), `pnpm svelte-check`.
  Витрина (D-01) — сознательная замена отсутствующему storybook как поверхность визуальной проверки.
- **Гоча для UAT (память проекта):** серверный режим отдаёт `ui/dist` — перед проверкой через LAN-браузер
  нужен `pnpm --dir ui build`; `cargo tauri dev` хотрелоадит только desktop-webview.
- **Гоча (память проекта):** `prebuild` тянет `cargo test -p trackly-app --test export_bindings` — сборка ui
  тянет cargo.

### Integration Points
- **Роутинг + навигация + роль admin** (D-02): найти, как заведены маршруты (svelte-spa-router?) и как
  меню/навигация фильтруется по роли — витрина встраивается туда же как admin-only пункт.
- 15 call-site `<Badge>` в `ui/src` — контракт обратной совместимости D-08 (нельзя менять эти вызовы).
- `ui/src/main.ts` импортирует `global.scss` до `mount()` — порядок не меняется.

</code_context>

<specifics>
## Specific Ideas

- **`.dc`-референсы — это фактически SPEC по значениям.** Пользователь ожидает попиксельного соответствия
  им; downstream не «улучшает» и не пересчитывает стили, а извлекает их из embedded-скриптов референсов.
- **Безопасность обратной совместимости важнее чистоты API** (Badge, D-08): дефолт `appearance='soft'`
  сохраняет текущий вид, 15 вызовов не трогаются.
- **Витрина — за логином админа**, не dev-флаг: пользователь хочет видеть её в реальной сборке под своей ролью.
- Приложение не релизится в середине v1.2 — «полуготовый» вид экранов (компоненты новые, экраны старые) до
  фаз 26–28 принят.

</specifics>

<deferred>
## Deferred Ideas

- **Ретрофит экранов на Tabs/Checkbox** (самодельные фильтр-бары, raw-чекбоксы) — фазы 26–28.
- **Секции витрины под таблицы/строки-группы (Фаза 25), dropdown (Фаза 25), окна (26–29)** — витрина
  проектируется расширяемой, но наполняется соответствующими фазами.
- **Playwright / screenshot-diff** — обсуждалось ещё в Фазе 23; для фаз со стабильными значениями (24+)
  может быть ценно, но заводится отдельной задачей, не внутри этой фазы. Витрина (D-01) закрывает
  визуальную проверку вручную.
- **Компонент-обёртка `<Mono>`** — отложена из Фазы 23; при желании может появиться здесь или позже, но в
  scope Фазы 24 не входит (не в списке 5 примитивов).

</deferred>

---

*Phase: 24-base-components*
*Context gathered: 2026-07-18*
