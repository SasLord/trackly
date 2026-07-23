# Phase 29: Вход и интерфейс сотрудника - Context

**Gathered:** 2026-07-23
**Status:** Ready for planning

<domain>
## Phase Boundary

Фаза 29 переводит **экраны входа** и **оболочку роли «Сотрудник»** на дизайн-систему
Фаз 23–25, продолжая плейбук Фаз 26–28, — при том, что эти поверхности живут **вне**
основной оболочки приложения (`Layout`/`Sidebar`):

- **Экраны входа** (`ui/src/features/auth/`) — `LoginPage`, `PendingScreen`, `BlockedScreen`,
  `FirstRunWizard`. Требование **WIN-10**.
- **EmployeeLayout** (`ui/src/features/layout/EmployeeLayout.svelte`) — минимальная header-оболочка
  роли «Сотрудник». Требование **WIN-11**.

**Ключевой факт из скаута (меняет характер фазы — как в Фазе 28):**
- Токены `--tr-*` уже мигрированы глобально (Фаза 23); legacy-токенов (`--space-/--radius-/
  --font-size-`) и захардкоженных hex во всех этих файлах — **ноль**. Значит Фаза 29 — это
  **адаптация примитивов и консистентность компонентов**, а не миграция токенов. SC говорит
  «токены/**компоненты**» — вес на компонентах.
- **Экраны входа сами рисуют форм-контролы** (`.form-input`, `.btn-submit`, `.checkbox-label`,
  `.btn-sso-reserved`) вместо примитивов `Input`/`Button`/`Checkbox` — это самый крупный
  bespoke-пласт фазы и её основная работа (прямой аналог D-04 Фаз 27/28).
- **EmployeeLayout уже на `--tr-*`** + использует примитивы `Button`/`ThemeSwitcher`; это
  **header-only** оболочка (без сайдбара — сознательное решение D-UI-01 Фазы 10). Остаётся лёгкий
  пасс консистентности по header-chrome.
- **Employee-«форма заявки / список своих заявок» = тот же `features/requests/`** (`RequestsPage`,
  own-requests, серверный фильтр `is_visible_to`), **уже мигрированный в Фазе 28 (WIN-06)**. В
  Фазе 29 **не переносится заново**.
- **Макета `.dc` для этих экранов нет** → визуальная истина выводится из системы + `Fields.dc.html`/
  `Buttons.dc.html` (плейбук «без макета» Фазы 27).

**В границах фазы:**
- Экраны входа: полная адопция примитивов `Input`/`Button`/`Checkbox` **повсеместно**, без остатков
  bespoke-классов (SC #1); минимальное расширение `Input` под `type='password'` (см. D-01).
- Извлечение **общей лёгкой auth-оболочки** (центр-карточка + field-паттерн label/error/hint),
  переиспользуемой всеми 4 экранами входа (D-02) — закрывает SC #3.
- EmployeeLayout: пасс консистентности header-chrome на токенах/примитивах (SC #2), **header-only**
  сохраняется (D-03).
- Обе темы (светлая и тёмная) — унаследовано от D-17 Фазы 26.

**Вне границ:**
- **Любое изменение полей, действий, workflow, бизнес-логики, API, бэкенда, auth-логики** — фаза
  чисто визуальная. Особенно строго — логика auth-роутинга `LoginPage` (screen state, коды ошибок
  `REGISTRATION_PENDING`/`ACCESS_BLOCKED`/`SERVICE_UNAVAILABLE`, anti-enumeration D-Sec-01), WS-логика
  EmployeeLayout, reserved-SSO как **нерабочая** заглушка (D-UX-03).
- **Повторный перенос `features/requests/`** (форма/список заявок сотрудника) — сделано Фазой 28
  (WIN-06); Фаза 29 только **визуально проверяет паритет** окна внутри employee-shell в обеих темах
  (D-04).
- **Добавление сайдбара в EmployeeLayout** — у сотрудника нет разделов навигации (D-UI-01);
  это новая функциональность (scope creep). SC-формулировка «(сайдбар...)» — описательный boilerplate,
  см. D-03.
- **Редизайн раскладок** (новая иерархия auth-экранов, перекомпоновка формы входа/визарда) —
  сознательно не берётся; риск нарушить «чисто визуальный» SC.
- **Мобильная переработка** этих экранов.
- AA-контраст, focus ring по новому дизайну, паритет Tauri vs LAN-браузер — QA-02/QA-03, Фаза 30.

</domain>

<decisions>
## Implementation Decisions

### Серая зона A — Форм-контролы экранов входа → примитивы

- **D-01:** `LoginPage`/`FirstRunWizard` рисуют форм-контролы вручную (`.form-input`, `.btn-submit`,
  чекбокс remember-me, `.btn-sso-reserved`). **Решение: полная адопция примитивов + минимальное
  расширение `Input`.**
  - Кнопки → `Button`: submit → `variant="primary"` + `loading` (заменяет ручной «Вход...»);
    reserved-SSO → `Button` disabled (`ghost`/`secondary`), **без onclick, без tabindex-фокуса** —
    сохранить нерабочую-заглушку семантику D-UX-03.
  - Чекбокс «Запомнить меня» → примитив `Checkbox` (label-слот, `bind:checked`).
  - Текст/пароль-поля → примитив `Input`, **расширив union `type` под `'password'`** (и `'email'`
    при надобности) — расширение примитива, **не форк** (правило milestone). `Input` уже даёт
    `invalid` + `aria-describedby`.
  - **label / inline-ошибка / format-hint** («Логин: us100…») — через field-обёртку поверх `Input`
    (`invalid` + `aria-describedby` → id ошибки/хинта). Форма field-обёртки — см. D-02 (кандидат в
    общий `FormField`-паттерн).
  **Обоснование:** только полная адопция закрывает SC #1 («компоненты», без остатков bespoke) честно;
  прямой аналог D-04 Фаз 27/28. Отклонено «ре-токенизация bespoke на месте» — оставляет крупнейший
  bespoke-пласт вне охвата.
  **Действие для планировщика:** сверить расширение `Input` (`type='password'`, при NULL-safe
  дефолтах) с существующими потребителями `Input` (Устройства/Заявки/Настройки/витрина) — не сломать;
  свериться с `Fields.dc.html` на канонический вид поля + состояния.

### Серая зона B — Общая оболочка экранов входа

- **D-02:** Все 4 экрана (`LoginPage`/`PendingScreen`/`BlockedScreen`/`FirstRunWizard`) центрируют
  собственную bespoke-карточку (`.login-card`, `.login-container` и аналоги). **Решение: извлечь
  лёгкую общую auth-оболочку** — центр-карточный shell (контейнер + карточка + заголовок) плюс,
  вероятно, field-паттерн `label/error/hint` из D-01 — переиспользуемую всеми 4 экранами. По образцу
  извлечения `PageHeader` (Фаза 26) / `DetailPanel` (Фаза 27, D-01).
  **Обоснование:** прямо закрывает SC #3 (единый визуальный язык, несмотря на отдельную оболочку);
  устраняет 4-кратное дублирование chrome карточки и риск расхождений. Отклонено «ре-токенизация
  каждого на месте».
  **Действие для планировщика:** решить форму общего паттерна (компонент vs набор классов/snippets) и
  где он живёт (`ui/src/lib/components/`); решить, входит ли `FormField`-паттерн D-01 в тот же
  извлечённый артефакт. Состав/поля каждого экрана НЕ меняются (SC — чисто визуально). Порядок волн:
  общий shell первым, затем 4 экрана параллельно (прецедент D-19 Фазы 26).

### Серая зона C — EmployeeLayout: header-only vs сайдбар

- **D-03:** SC #2 буквально пишет «EmployeeLayout (сайдбар, форма заявки, список собственных
  заявок)», но реализация — **header-only** (D-UI-01 Фазы 10: у сотрудника нет разделов навигации,
  реальная граница доступа — backend 403). **Решение: оставить header-only.** SC-упоминание сайдбара —
  описательный перенос из roadmap-boilerplate, НЕ требование добавить навигацию. Фаза 29 = пасс
  консистентности по существующему header-chrome (`.employee-header`, `.employee-brand`,
  `.user-name/.user-role`, `.skip-link`) на токенах/примитивах; `Button`/`ThemeSwitcher` уже на месте.
  **Обоснование:** добавление сайдбара — новая функциональность (scope creep), маршрутов/разделов у
  сотрудника нет. Отклонено «трактовать SC буквально и добавить сайдбар».
  **Действие для планировщика/верификатора:** зафиксировать рассогласование SC↔реализация явно, чтобы
  верификатор не отметил «отсутствующий сайдбар» как провал SC #2.

### Серая зона D — Граница переиспользуемого окна Заявок

- **D-04:** Форма заявки / список собственных заявок сотрудника = **тот же** `features/requests/`
  (`RequestsPage`, own-requests, серверный фильтр), **уже мигрированный в Фазе 28 (WIN-06)**.
  **Решение: Фаза 29 НЕ переносит его заново.** WIN-11 в объёме Фазы 29 = только employee-shell
  (D-03) + **визуальная проверка паритета**, что уже-мигрированное окно Заявок корректно смотрится
  внутри employee-оболочки в **обеих темах**.
  **Обоснование:** дублировать миграцию Фазы 28 незачем; отдельного employee-специфичного request-
  компонента нет (`employeeRoutes` → тот же `RequestsPage`).
  **Отклонено:** «перепроверить/дотронуть окно Заявок в Фазе 29».

### Claude's Discretion
- Точная форма общего auth-shell и `FormField`-паттерна (компонент vs snippets/классы) и его
  размещение в `ui/src/lib/components/` (D-01/D-02).
- Точный набор `type`-значений при расширении `Input` (`password`; `email`/`tel` — по надобности) и
  форма пробрасывания label/error/hint (D-01).
- Точный состав визуальных правок `PendingScreen`/`BlockedScreen` (статус-экраны без полей ввода) при
  переносе на общий shell — без изменения отображаемого содержимого/состояний.
- Дробление на волны/планы (прецедент D-19 Фазы 26: общие/разделяемые артефакты — shell, расширение
  `Input` — первыми; затем экраны параллельно).
- Конкретные значения ре-токенизации/полировки там, где нет прецедента — выводятся из `_tokens.scss`,
  `Fields.dc`/`Buttons.dc` и окон Фаз 26–28.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Прецедент-плейбук — Фазы 27–28 (читать первыми: сюда ложится D-04/D-01 1:1)
- `.planning/phases/28-support-admin-windows/28-CONTEXT.md` — источник D-04 (полная ре-токенизация/
  адопция примитивов внутренностей, без остатков bespoke), правило «расширять примитив, а не форкать»,
  порядок волн (общие артефакты первыми), урок обеих тем.
- `.planning/phases/27-core-workflow-windows/27-CONTEXT.md` — **D-01 (извлечение лёгкого общего
  паттерна — образец для auth-shell D-02 здесь)**, плейбук «без макета» (истина из системы, не из
  `.dc`), D-05 (bespoke-контролы → примитивы). Формула «форма из системы, содержание из приложения».
- `.planning/phases/27-core-workflow-windows/27-LEARNINGS.md` и
  `.planning/phases/28-support-admin-windows/28-LEARNINGS.md` (если созданы) — уроки исполнения.

### Прецедент без макета и правила переноса — Фаза 26
- `.planning/phases/26-windows-with-mockup/26-CONTEXT.md` — D-07 (`PageHeader` — образец извлечения
  общего артефакта, как auth-shell D-02), D-17 (проверка обеих тем), D-18 (чек-лист значений +
  визуальный UAT), D-19 (порядок волн: общие файлы первыми).

### Контракт дизайн-системы (Фазы 23–25) + эталон полей
- `ui/src/styles/_tokens.scss` — единственный слой токенов. **Closed-world гейт `check-tokens.mjs`
  роняет сборку при ссылке на несуществующий токен** (обжигалась Фаза 24).
- `.planning/reference/design-system-v2/Fields.dc.html` — **эталон поля ввода** (label/hint/error,
  состояния) для D-01; `.../Buttons.dc.html` — эталон кнопок (submit/disabled) для D-01.
- `.planning/phases/23-design-tokens-foundations/23-UI-SPEC.md` — все hex `--tr-*` (light+dark),
  шкала типографики, motion.
- `.planning/phases/24-base-components/24-LEARNINGS.md` — **обязательно:** `:global()` в plain SCSS
  попадает в собранный CSS дословно и не работает; авто-одобрение чекпоинта маскировало непроверенное;
  ложный `[VERIFIED]`. Ловушка `const` vs `let` при `$bindable()`.
- `.planning/phases/25-dropdown/25-CONTEXT.md` — D-10 (стили из системы, содержание из приложения).

### Примитивы, которые переиспользуются/расширяются
- `ui/src/lib/components/Input.svelte` — **D-01: расширить `type` под `'password'`** (сейчас
  `text|number|search`); есть `invalid` + `aria-describedby`, нет встроенных label/error/hint.
  **Осторожно: общий примитив** (Устройства, Заявки, Настройки, витрина) — не сломать потребителей.
- `ui/src/lib/components/Button.svelte` — `primary`+`loading` (submit), disabled `ghost`/`secondary`
  (reserved-SSO). `ui/src/lib/components/Checkbox.svelte` — remember-me (label-слот, `invalid`).
- `ui/src/lib/components/ThemeSwitcher.svelte` — уже в EmployeeLayout.
- Прецедент извлечённых общих артефактов: `PageHeader.svelte` (Ф.26), `DetailPanel.svelte`/
  `DetailField.svelte` (Ф.27) — образец формы auth-shell/FormField (D-02).

### Код, который меняется
- **Экраны входа (WIN-10):** `ui/src/features/auth/LoginPage.svelte` (304),
  `ui/src/features/auth/FirstRunWizard.svelte` (289; 4 сырых `<input>`),
  `ui/src/features/auth/PendingScreen.svelte` (76), `ui/src/features/auth/BlockedScreen.svelte` (197).
- **Employee-shell (WIN-11):** `ui/src/features/layout/EmployeeLayout.svelte` (189; header-only,
  пасс консистентности D-03).
- **Новый общий артефакт (D-02):** auth-shell (+ возможно FormField) в `ui/src/lib/components/`.
- **Не трогать:** `features/requests/` (мигрировано Ф.28, D-04 — только визуальная проверка паритета);
  auth-логику `LoginPage` (screen-роутинг, коды ошибок, anti-enumeration); WS-логику EmployeeLayout;
  потребителей `Input` при его расширении.

### Требования и роадмап
- `.planning/ROADMAP.md` §«Phase 29» — цель, 3 Success Criteria. SC #2 буквально упоминает «сайдбар» —
  трактуется описательно, см. D-03. SC #3 (единый язык, несмотря на отдельную оболочку) — якорь D-02.
- `.planning/REQUIREMENTS.md` — WIN-10, WIN-11 (Pending). Мобильная адаптивность не покрыта —
  вне объёма.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **Примитивы Фаз 24–25 покрывают почти всё:** `Input` (после расширения под `password`), `Button`
  (`primary`+`loading`, disabled), `Checkbox` (label-слот), `ThemeSwitcher`. Новый нужен максимум
  один — общий auth-shell/FormField (D-02), по образцу `PageHeader`/`DetailPanel`.
- **EmployeeLayout уже на `--tr-*` + `Button`/`ThemeSwitcher`** — работа минимальна (пасс
  консистентности), не редизайн.
- **`features/requests/` уже мигрирован (Ф.28)** — сотрудник переиспользует `RequestsPage`, отдельной
  работы нет (D-04).

### Established Patterns
- Legacy-токены и hex во всех файлах фазы = **0** (Фаза 23). Фаза 29 = адаптация примитивов, а не
  переименование токенов.
- Экраны входа **уже на `--tr-*`, но с bespoke-контролами** (`.form-input`/`.btn-submit`/чекбокс) —
  это и есть работа D-01.
- Стили — scoped `<style lang="scss">` в каждом компоненте.
- Пропсы — Svelte 5 runes (`$props`, `$bindable`, `$derived`), `Snippet`. **Ловушка Фазы 24:**
  `const` vs `let` при `$bindable()` — контракт, не стилистика.
- **Фронтенд-тестов нет.** Гейты: `pnpm lint`, `pnpm svelte-check`, `pnpm --dir ui build`,
  `check-tokens.mjs`. Проверка визуала — только глазами (D-18 Фазы 26).
- **`:global()` в plain `.scss` не работает** (урок Фазы 24). CI-гейта нет.
- **Гоча (память проекта):** перед проверкой через LAN-браузер нужен `pnpm --dir ui build` —
  серверный режим отдаёт `ui/dist`, `cargo tauri dev` хотрелоадит только desktop-webview.
  `prebuild` тянет `cargo test -p trackly-app --test export_bindings`.

### Integration Points
- **`Input` — общий примитив** нескольких потребителей (Устройства, Заявки, Настройки, витрина).
  D-01 расширяет его `type` — регресс проверять во всех.
- **App.svelte / routes.ts:** `LoginPage`/`FirstRunWizard` рендерятся напрямую (не в основной
  оболочке); `employeeRoutes` → `RequestsPage` внутри `EmployeeLayout`. Границы меняются только внутри
  перечисленных компонентов.
- **EmployeeLayout WS-логика** (`connectWs`/`onWsEvent`, статус-тосты собственных заявок) — не UX-
  визуал, НЕ трогать (T-11-03).

</code_context>

<specifics>
## Specific Ideas

- **Характер фазы: компоненты, не токены.** Токены уже мигрированы (0 legacy/hex); Фаза 29 — про
  адопцию примитивов и консистентность. SC «токены/**компоненты**» читать с весом на компонентах.
- **Правило расширения примитива, а не форка** (расширение `Input` под `password` D-01) — единые
  компоненты как цель milestone.
- **SC #3 — якорь извлечения общего auth-shell (D-02):** единый визуальный язык, несмотря на отдельную
  оболочку.
- **Строгость SC (чисто визуально):** auth-роутинг, коды ошибок/anti-enumeration, reserved-SSO как
  нерабочая заглушка, WS-логика EmployeeLayout — сохраняются.
- **Все 4 серые зоны решены рекомендованными опциями** — набор консервативный и последовательный с
  Фазами 26–28.

</specifics>

<deferred>
## Deferred Ideas

- **Полноценная мобильная адаптивность экранов входа / employee-shell** — WIN-10/WIN-11 её не
  покрывают. Понадобится — новое требование + правка ROADMAP/REQUIREMENTS.
- **Сайдбар/расширенная навигация в EmployeeLayout** — у сотрудника нет разделов; появится, если
  роль получит новые разделы (новая функциональность, не эта фаза). См. D-03.
- **Рабочий SSO-вход по учётной записи Windows** — заглушка D-UX-03 остаётся нерабочей; реальная
  реализация — отдельный AD/SSO-заход (v2).
- **Редизайн раскладок auth-экранов / визарда** — выходит за «чисто визуальный» SC; отдельный
  продуктовый заход.
- **AA-контраст, focus ring по новому дизайну, паритет Tauri vs LAN-браузер** — QA-02/QA-03, Фаза 30.
- **Grep-гейт на `:global(` в plain `.scss`** (WR-15 Фазы 24) — так и не добавлен в CI.

None beyond the above — обсуждение осталось в границах фазы.

</deferred>

---

*Phase: 29-login-and-employee-shell*
*Context gathered: 2026-07-23*
</content>
</invoke>
