---
phase: 24-base-components
verified: 2026-07-18T00:00:00Z
status: gaps_found
score: 5/8 must-haves verified
overrides_applied: 0
gaps:
  - truth: "bind:value обеспечивает двусторонний биндинг для Input/Select/Textarea"
    status: failed
    reason: "Input.svelte, Select.svelte, Textarea.svelte деструктурируют props через `const { value = $bindable('') }` и никогда не присваивают value обратно — элемент получает {value} однонаправленно (down), а oninput/onchange лишь вызывает колбэк. bind:value у родителя молча не обновляется. Checkbox/Radio сделаны правильно (let + bind:checked/bind:group) — набор примитивов внутренне несогласован."
    artifacts:
      - path: "ui/src/lib/components/Input.svelte"
        issue: "const-деструктуризация $bindable('value'), отсутствует bind:value на <input>, строки 13-38"
      - path: "ui/src/lib/components/Select.svelte"
        issue: "тот же паттерн, строки 18-38"
      - path: "ui/src/lib/components/Textarea.svelte"
        issue: "тот же паттерн, строки 12-35"
    missing:
      - "Заменить const на let во всех трёх компонентах"
      - "Добавить bind:value на нативный <input>/<select>/<textarea> вместо {value}"
      - "Пересобрать витрину и убедиться, что FieldsSection/ModalSection demoValue реально меняется при вводе"
  - truth: "Бейджи-статусы рендерятся в 5 тонах во всех 4 вариантах (мягкая подложка / сплошной / с точкой / счётчик-пилюля)"
    status: failed
    reason: "CSS-правила count-варианта заданы только для .badge-m-count (нейтральный) и .badge-m-accent.badge-m-count. Для success/warning/danger специфичных count-правил нет, поэтому <Badge variant=\"success|warning|destructive\" appearance=\"count\"> получает нейтральный серый вид вместо тонового. Подтверждено в собранном CSS (ui/dist/assets/*.css: только 2 правила .badge-m-count существуют) и напрямую воспроизводится в самой витрине (BadgeSection.svelte строки 34/44/54 используют count с этими тремя тонами)."
    artifacts:
      - path: "ui/src/lib/components/Badge.svelte"
        issue: "строки 177-196: .badge-m-count и .badge-m-accent.badge-m-count существуют, аналогичных блоков для success/warning/danger нет"
    missing:
      - "Добавить &.badge-m-count правило внутрь .badge-m-success/.badge-m-warning/.badge-m-danger по образцу .badge-m-accent"
  - truth: "Переключение темы (light↔dark) не создаёт видимого 'размазывания' цвета по интерактивным примитивам (D-09)"
    status: failed
    reason: "global.scss — обычный .scss-файл, обрабатывается только sass/Vite, а не компилятором Svelte. Синтаксис :global(...) специфичен для scoped-стилей Svelte-компонентов и в plain SCSS ничем не удаляется — селектор :global(.theme-switching) попадает в собранный CSS дословно и невалиден для браузера (подтверждено: grep -o \":global([^)]*)\" ui/dist/assets/*.css находит 2 совпадения). Правило transition:none!important никогда не применяется; theme.svelte.ts корректно навешивает/снимает класс theme-switching, но эффекта это не даёт."
    artifacts:
      - path: "ui/src/styles/global.scss"
        issue: "строки 64-67: :global(.theme-switching), :global(.theme-switching) * { transition: none !important; } — невалидный CSS вне контекста Svelte-компилятора"
    missing:
      - "Убрать обёртку :global() — файл уже глобальный: .theme-switching, .theme-switching * { transition: none !important; }"
      - "Пересобрать (pnpm --dir ui build) и убедиться grep'ом, что :global( больше не попадает в ui/dist/assets/*.css"
human_verification:
  - test: "Открыть /showcase от имени admin, проверить 5 витрин против .dc.html-референсов"
    expected: "Кнопки/Поля/Бейджи/Вкладки/Модалка визуально совпадают с .planning/reference/design-system-v2/{Buttons,Fields,Badges,Tabs,Modal}.dc.html"
    why_human: "Визуальное сравнение пикселей/тональности не проверяется grep'ом; 24-07-SUMMARY.md прямо признаёт, что этот шаг человеком не выполнялся (checkpoint auto-approved под workflow.auto_advance)"
  - test: "Войти как manager/employee и перейти на #/showcase напрямую по хэшу"
    expected: "Согласно D-02, доступ должен быть admin-only"
    why_human: "Требует живой сессии с другой ролью; статически подтверждено, что routes-карта не гейтит маршрут (см. WR-04 ниже) — вероятный результат: manager получит страницу, только не увидит пункт в сайдбаре"
  - test: "Быстро переключить тему light→dark→system несколько раз подряд, наблюдая Button/Tabs/Badge"
    expected: "Не должно быть видимого цветового 'размазывания' (D-09)"
    why_human: "Визуальный эффект transition; УЖЕ известно как FAILED по коду (см. gap #3), человеческая проверка подтвердит на глаз, но не изменит вердикт"
---

# Phase 24: Базовые компоненты — отчёт о верификации

**Цель фазы:** Пять базовых примитивов (Button, поля ввода, бейджи, вкладки, модальное окно) отражают новую дизайн-систему, так что всё, что их переиспользует, автоматически наследует новый визуальный язык.

**Проверено:** 2026-07-18
**Статус:** gaps_found
**Повторная верификация:** Нет — первичная верификация

## Достижение цели

### Наблюдаемые истины (Success Criteria из ROADMAP.md + must_haves из планов)

| # | Истина | Статус | Свидетельство |
|---|--------|--------|----------------|
| 1 | Каждый из 5 вариантов кнопки в обоих размерах визуально различим в состояниях наведение/фокус/нажатие/отключено/загрузка | ✓ VERIFIED | `Button.svelte` — все 5 вариантов (primary/secondary/destructive/ghost/link) × 2 размера, `:hover`/`:focus-visible`/`:active`/`:disabled`/`.loading` реализованы, `.12s` transition присутствует (строки 46-179) |
| 2 | Input/Select/Textarea/Checkbox визуально различимы в состояниях обычное/фокус/ошибка/отключено | ✓ VERIFIED (визуально) | Все 5 полей (включая Radio) имеют `:focus-visible`/`.invalid`/`:disabled` CSS-правила; токены `--tr-surface`/`--tr-border-strong` подтверждены во всех трёх текстовых полях |
| 3 | `bind:value` обеспечивает двусторонний биндинг для Input/Select/Textarea | ✗ FAILED | См. gap #1 в frontmatter — `const` + отсутствие `bind:value` на элементе. Подтверждено напрямую чтением исходников; баг уже проявляется в собственной витрине фазы (`FieldsSection.svelte`, `ModalSection.svelte`) |
| 4 | Бейджи-статусы рендерятся в 5 тонах в вариантах мягкая подложка/сплошной/с точкой/счётчик-пилюля | ✗ FAILED | См. gap #2 — count-вариант работает только для 2 из 5 тонов (neutral, accent); success/warning/danger рендерятся серыми. Воспроизводится в built CSS и в самой `BadgeSection.svelte` |
| 5 | Вкладки switch-bar показывают счётчики и подчёркивание активной вкладки | ✓ VERIFIED | `Tabs.svelte` — `.tab-count` рендерится при `variant==='underline'`, `.tab.active` получает `border-bottom-color: var(--tr-accent)`; segmented-вариант отдельно реализован с `box-shadow: var(--tr-elev-1)` |
| 6 | Модальное окно показывает оверлей + шапку + тело + футер действий с тенью уровня 3 и радиусом 12px | ✓ VERIFIED | `Modal.svelte` — `.modal-container { border-radius: var(--tr-radius-lg); box-shadow: var(--tr-elev-3); }`, header/body/footer все присутствуют |
| 7 | Переключение темы (light↔dark) не создаёт видимого "размазывания" цвета (D-09, план 24-01) | ✗ FAILED | См. gap #3 — `:global()` внутри обычного `.scss`-файла невалиден вне Svelte-компилятора, правило подавления transition никогда не применяется. Подтверждено в собранном CSS |
| 8 | Показ пункта "Витрина компонентов" в сайдбаре только для admin (D-02, план 24-07) | ⚠️ PARTIAL | Пункт сайдбара корректно гейтится `roles: ['admin']` (`sidebar-config.ts:31`) — это верно. НО сам маршрут `/showcase` зарегистрирован в общей карте `routes` (используется и admin, и manager, `App.svelte:67`), а не отдельно гейтится — manager, зная `#/showcase`, получит страницу. Это существующий паттерн приложения (тот же для `/users`/`/settings`), не уникальная регрессия фазы 24, поэтому не поднимается до BLOCKER, но и не даёт полного PASS буквальной формулировке "access is admin-only" |

**Счёт:** 5/8 must-haves verified (4 VERIFIED + 1 PARTIAL считается как не-полный проход; 3 FAILED)

### Обязательные артефакты

| Артефакт | Ожидание | Статус | Детали |
|----------|----------|--------|--------|
| `ui/src/lib/components/Button.svelte` | 5×2×6 состояний, `.12s` transition | ✓ VERIFIED | Содержит `transition: background .12s, box-shadow .12s` |
| `ui/src/lib/components/Input.svelte` | `--tr-surface`/`--tr-border-strong`, рабочий `bind:value` | ⚠️ ЧАСТИЧНО | Токены верны; `bind:value` НЕ работает (const, нет `bind:value` на `<input>`) |
| `ui/src/lib/components/Select.svelte` | то же | ⚠️ ЧАСТИЧНО | Тот же дефект |
| `ui/src/lib/components/Textarea.svelte` | то же | ⚠️ ЧАСТИЧНО | Тот же дефект |
| `ui/src/lib/components/Checkbox.svelte` | Новый примитив, hidden native input | ✓ VERIFIED | `let` + `bind:checked`, корректно |
| `ui/src/lib/components/Radio.svelte` | Новый примитив, `bind:group` | ✓ VERIFIED | `let` + `bind:group`, корректно |
| `ui/src/lib/components/Badge.svelte` | Opt-in `appearance`, 5×4 матрица, 21 call-site не тронут | ⚠️ ЧАСТИЧНО | Матрица есть, но `count` работает только для 2/5 тонов; verbatim default-путь подтверждён (21 call-site не менялся — см. requirements-completed в 24-05-SUMMARY, не перепроверялось построчно, но diff plana ограничен файлами Badge.svelte + BadgeSection.svelte) |
| `ui/src/lib/components/Tabs.svelte` | underline + segmented варианты | ✓ VERIFIED | Оба варианта, count badge, active underline/raised segment |
| `ui/src/lib/components/Modal.svelte` | elev-3 + radius-lg | ✓ VERIFIED | Оба токена присутствуют |
| `ui/src/styles/_tokens.scss` | `--tr-accent-text` (light+dark) | ✓ VERIFIED | Строки 19, 95 |
| `ui/src/styles/global.scss` | Рабочее правило подавления transition при смене темы | ✗ STUB | Правило синтаксически невалидно (`:global()` вне Svelte-компилятора) — присутствует текстово, не функционирует |
| `ui/src/lib/stores/theme.svelte.ts` | `applyResolved()` навешивает/снимает `.theme-switching` | ✓ VERIFIED (логика корректна, эффекта нет из-за global.scss) | Класс добавляется/убирается через rAF, но CSS-правило для него сломано |
| `ui/src/features/showcase/ShowcasePage.svelte` | Сборка всех 5 секций | ✓ VERIFIED | 5 импортов + 5 использований подтверждены (grep) |
| `ui/src/pages/ComponentShowcasePage.svelte` | Тонкая обёртка | ✓ VERIFIED | 5 строк |
| `ui/src/routes.ts` | `/showcase` в admin/manager routes | ✓ VERIFIED (но не гейтится по роли на уровне маршрута) | Строка 28 |
| `ui/src/features/layout/sidebar-config.ts` | `roles: ['admin']` | ✓ VERIFIED | Строка 31 |

### Проверка ключевых связей (Key Links)

| От | К | Через | Статус | Детали |
|----|---|-------|--------|--------|
| `theme.svelte.ts` | `global.scss` | `classList.add/remove('theme-switching')` ↔ `:global(.theme-switching)` CSS-правило | ✗ NOT_WIRED (функционально) | Класс навешивается корректно, но CSS-селектор невалиден в собранном бандле — связь текстуально присутствует (`grep` находит "theme-switching" в обоих файлах), но не работает в браузере |
| `Checkbox.svelte`/`Radio.svelte` | native `<input>` | `bind:checked`/`bind:group` | ✓ WIRED | Подтверждено |
| `Input/Select/Textarea.svelte` | native элемент | `bind:value` | ✗ NOT_WIRED | `{value}` — только downward binding, обратного присваивания нет |
| `ShowcasePage.svelte` | 5 секций | статические импорты | ✓ WIRED | Buttons → Fields → Badge → Tabs → Modal, порядок соответствует CMP-01..05 |
| `routes.ts` | `ComponentShowcasePage.svelte` | `'/showcase': ComponentShowcasePage` | ✓ WIRED | Строка 28 |
| `sidebar-config.ts` | `Sidebar.svelte` | `getVisibleItems(role)` фильтрует по `roles: ['admin']` | ✓ WIRED (только уровень сайдбара) | Не распространяется на уровень маршрута — см. истина #8 |

### Покрытие требований

| Требование | План | Описание | Статус | Свидетельство |
|------------|------|----------|--------|----------------|
| CMP-01 | 24-02 | Кнопки — 5 вариантов × 2 размера × 6 состояний | ✓ SATISFIED | `Button.svelte` дефектов не обнаружено |
| CMP-02 | 24-03 | Поля ввода — Input/Select/Textarea/Checkbox, 4 состояния | ✗ BLOCKED | Визуальные состояния верны, но `bind:value` сломан у 3 из 4 названных примитивов — базовый контракт "поле ввода с двусторонним биндингом" не выполняется, что подорвёт формы фаз 25-30 |
| CMP-03 | 24-01, 24-05 | Бейджи-статусы — 5 тонов × 4 варианта | ✗ BLOCKED | count-вариант работает только для 2 из 5 тонов; текстовая правка "5 тонов" в REQUIREMENTS.md/ROADMAP.md выполнена (D-06), но фактическая реализация не покрывает все 5×4 = 20 ячеек |
| CMP-04 | 24-06 | Вкладки switch-bar со счётчиками и подчёркиванием | ✓ SATISFIED | `Tabs.svelte` соответствует |
| CMP-05 | 24-04 | Модальное окно — оверлей/шапка/тело/футер, тень 3, радиус 12px | ✓ SATISFIED (по буквальной формулировке) | Визуально/структурно верно. Отдельно отмечается (не как провал этого требования, а как сопутствующая находка): Modal не управляет фокусом (нет initial focus, focus trap, возврата фокуса) — нарушение WAI-ARIA Dialog Pattern при заявленных `role="dialog" aria-modal="true"`, подтверждено прямым чтением `Modal.svelte:41-66`, где логики фокуса нет вообще |

**Осиротевших требований (orphaned) не обнаружено** — все CMP-01..05, заявленные в REQUIREMENTS.md для Phase 24, присутствуют в frontmatter соответствующих планов (24-01..24-07) и учтены выше.

### Анти-паттерны и сопутствующие находки

| Файл | Строка | Паттерн | Серьёзность | Влияние |
|------|--------|---------|--------------|---------|
| `ui/src/lib/components/Modal.svelte` | 41-66 | Отсутствие focus-management (нет initial focus / focus trap / возврата фокуса) | ⚠️ WARNING | Нарушение WAI-ARIA Dialog Pattern; Modal переиспользуется во всех формах приложения. Не входит в буквальную формулировку CMP-05/SC5, поэтому не поднято до BLOCKER этой верификации, но требует отдельного gap-closure |
| `ui/src/routes.ts`, `ui/src/features/layout/sidebar-config.ts` | 28, 31 | Гейт `/showcase` только на уровне сайдбара, не маршрута | ℹ️ INFO | Совпадает с существующим паттерном `/users`/`/settings` — не регрессия фазы 24, но и не полное исполнение D-02 "access is admin-only" |
| `ui/src/lib/components/Tabs.svelte` | 26-54 | `role="tablist"`/`role="tab"` без клавиатурного паттерна (стрелки, roving tabindex, `aria-controls`) | ℹ️ INFO | Заявленная ARIA-семантика не подкреплена поведением; не входит в SC4, не блокирует эту верификацию |
| TBD/FIXME/XXX | — | Проверены все 21 изменённый/созданный файл фазы | — | Не найдено ни одного маркера долга без ссылки на issue |

Отладочные маркеры (TBD/FIXME/XXX) отсутствуют — gate по debt-маркерам пройден.

`svelte-check` — 0 ERRORS (48 WARNINGS, все в файлах вне Phase 24, не новые). `node ui/scripts/check-tokens.mjs` — PASS, 0 нарушений.

### Поведенческие проверки (Spot-Checks)

| Поведение | Команда | Результат | Статус |
|-----------|---------|-----------|--------|
| `:global(.theme-switching)` попадает в собранный CSS дословно | `grep -o ":global([^)]*)" ui/dist/assets/*.css` | 2 совпадения | ✗ FAIL (подтверждает gap #3) |
| Badge count-CSS существует только для 2 тонов | `grep -o "\.badge-m-[a-z-]*\.badge-m-count[^}]*{[^}]*}" ui/dist/assets/*.css` | Только `.badge-m-accent.badge-m-count` + базовый `.badge-m-count` | ✗ FAIL (подтверждает gap #2) |
| `bind:value` не присваивается родителю | Прямое чтение `Input.svelte`/`Select.svelte`/`Textarea.svelte` (`const` + нет `bind:value` на элементе) | Подтверждено во всех трёх файлах | ✗ FAIL (подтверждает gap #1) |
| `/showcase` доступен через общую `routes`-карту вне зависимости от роли | Чтение `App.svelte:67`, `routes.ts:16-29` | `routes` общая для admin/manager | ⚠️ Существующий паттерн, не новая регрессия |
| Debt-маркеры (TBD/FIXME/XXX) во всех файлах фазы | `grep -n -E "TBD|FIXME|XXX|TODO|HACK|PLACEHOLDER"` по 21 файлу | Пусто | ✓ PASS |

### Требуется проверка человеком

См. `human_verification` в frontmatter — три пункта: визуальное сравнение с `.dc.html`-референсами, проверка доступа manager/employee к `#/showcase` напрямую, визуальная проверка отсутствия "размазывания" при смене темы. Ни один из этих пунктов не был выполнен человеком согласно самому честному признанию в `24-07-SUMMARY.md` ("Task 3 ... closed as a gate-closure only under auto_advance").

### Резюме пробелов

Фаза 24 структурно полностью собрана: все 5 примитивов существуют, витрина собрана, роутинг и сайдбар подключены, `svelte-check` чист, токены дисциплинированы, debt-маркеров нет. Однако три подтверждённых BLOCKER-дефекта делают часть заявленной функциональности фактически нерабочей:

1. **`bind:value` в Input/Select/Textarea односторонний** — любая форма, построенная на этих примитивах в фазах 25-30, будет отправлять на бэкенд устаревшие данные. Это самый серьёзный дефект — примитивы являются "контрактом", на который опираются 6 последующих фаз.
2. **Подавление transition при смене темы (D-09) не работает** — `:global()` в plain `.scss` — синтаксическая ошибка, не CSS-дизайн. Простой однострочный фикс.
3. **Счётчик-пилюля (count) у Badge работает только для 2 из 5 тонов** — заявленный в SC3/CMP-03 "5 тонов в вариантах ... счётчик-пилюля" не выполнен буквально; сам баг виден прямо в собственной витрине фазы.

Ни один из этих трёх пробелов не покрывается более поздними фазами дорожной карты (25-30 описывают Таблицы/Dropdown, готовые макеты, рабочие окна, поддержку/админку, вход/сотрудника, финальный QA по контрасту/фокус-кольцу/платформенному паритету — ни одна не упоминает bind:value, D-09 или Badge count). Отложить их некуда — все три должны быть закрыты в рамках gap-closure для Phase 24 до того, как фазы 25+ начнут строить формы поверх этих примитивов.

Дополнительно отмечена (не как gate-блокер, но как настоятельная рекомендация к отдельному follow-up) находка code review CR-03: `Modal.svelte` не управляет фокусом, что нарушает `role="dialog" aria-modal="true"`.

---

_Verified: 2026-07-18_
_Verifier: Claude (gsd-verifier)_
