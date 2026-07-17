---
phase: 23-design-tokens-foundations
plan: 06
subsystem: ui
tags: [design-tokens, ci-gate, prettier, verification, hand-off]

# Dependency graph
requires:
  - phase: 23-01
    provides: "--tr-* token layer in _tokens.scss + global.scss"
  - phase: 23-02
    provides: "check-tokens.mjs permanent CI gate (3 rules) + verify-value-map.mjs"
  - phase: 23-03
    provides: "color/shadow migration (0 remaining)"
  - phase: 23-04
    provides: "space/radius migration (0 remaining) + BASE_SHA for verify-value-map.mjs"
  - phase: 23-05
    provides: "typography migration + tr-mono application (0 remaining)"
provides:
  - "Подтверждённо зелёный whole-tree прогон всех статических гейтов фазы 23 одновременно: check-tokens.mjs (все 3 правила), verify-value-map.mjs, pnpm svelte-check, pnpm lint"
  - "pnpm lint зелёный впервые за всю фазу 23 (0 exit code) — закрыт последний из 7 pre-existing prettier-drift файлов, найденных планом 23-02"
  - "Явный hand-off чек-лист (4 пункта) непроверяемых вручную визуальных пунктов для /gsd-verify-work (D-09)"
affects: [24, 25, 26, 27, 28, 29, 30]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Whole-tree финальная верификация как отдельный план (не растворена в последнем sweep-плане) — единственная точка, где швы между 5 предыдущими планами (файлы, задетые несколькими sweep-планами последовательно) доказуемо не оставили пропуска"

key-files:
  created:
    - ".planning/phases/23-design-tokens-foundations/23-06-SUMMARY.md"
  modified:
    - "ui/src/features/acts/ActFormBody.svelte"
    - "ui/src/features/acts/PdfPreviewModal.svelte"
    - "ui/src/features/dashboard/ChartWidget.svelte"
    - "ui/src/lib/api/acts.ts"
    - "ui/src/lib/components/PersonAutocomplete.svelte"
    - "ui/src/styles/_tokens.scss"

key-decisions:
  - "Task 1 (полный check-tokens.mjs + verify-value-map.mjs прогон) не потребовал ни одного точечного фикса — baseline уже был чист (0 нарушений по всем 3 правилам, 0 расхождений value-map, 0 старых undefined-имён) на момент старта плана; это подтверждает, что планы 23-01..23-05 не оставили пропусков на стыках"
  - "pnpm prettier --write . выполнен буквально по инструкции плана Task 2 (не как discretionary Rule-3 фикс вне scope) — план прямо предписывает этот шаг при обнаружении prettier-несоответствий и требует подтвердить, что изменения ограничены форматированием"
  - "Каждый из 6 изменённых prettier-файлов проверен построчным git diff перед коммитом — все 6 диффов оказались чистым line-wrap/reflow (перенос длинных строк, схлопывание shorthand box-shadow в одну строку), ни одного изменения значения/логики"

requirements-completed: [DS-01, DS-02, DS-03, DS-04, QA-01]

# Metrics
duration: ~10min
completed: 2026-07-17
---

# Phase 23 Plan 06: Финальная верификация фазы + hand-off чек-лист Summary

**Whole-tree прогон подтвердил 0 нарушений во всех 4 статических гейтах фазы одновременно (check-tokens.mjs 3/3 правила, verify-value-map.mjs, svelte-check, lint); закрыт последний pre-existing prettier-drift файл из deferred-items.md, `pnpm lint` впервые за фазу вернул exit-код 0.**

## Performance

- **Duration:** ~10 min
- **Started:** 2026-07-17T15:25:58Z
- **Tasks:** 2/2 completed
- **Files modified:** 6 (Task 2 prettier fix) + 1 создан (SUMMARY.md)

## Accomplishments
- `node ui/scripts/check-tokens.mjs` (все 3 правила, без `--rules`) → **PASS — 0 нарушений** — подтверждено, точечных фиксов не потребовалось
- `node ui/scripts/verify-value-map.mjs 6425d30cf3e8acdc7d163c41dd3515d0eded88b5` (BASE_SHA из `23-04-SUMMARY.md`, план 23-04 обязал её записать — использована как есть, реконструкция не потребовалась) → **PASS — 578 хунков проверено, 0 нарушений**
- `git grep -n -- '--font-size-sm\|--radius-lg\b\|--shadow-md' ui/src` → 0 совпадений (все известные QA-01 сайты закрыты, включая bonus-находку `--shadow-md` из плана 23-03)
- `git grep -c -- '--tr-font-size-caption' ui/src/lib/components/PersonAutocomplete.svelte` → 1 (QA-01 фикс `--font-size-sm` подтверждён на месте)
- `pnpm svelte-check` → 0 errors, 48 pre-existing warnings (неизменный baseline всей фазы), exit 0
- `pnpm build` → успешно, exit 0 (только pre-existing предупреждение о dynamic import, не связанное с фазой)
- `pnpm lint` (eslint + prettier + check-tokens.mjs) → **впервые за всю фазу 23 вернул exit-код 0.** До фикса: prettier находил 6 файлов с форматированием не по стилю (7 pre-existing из `deferred-items.md` минус 1, `ActFormItemsTable.svelte`, случайно исправленный планом 23-05). Выполнен `pnpm prettier --write .` по прямой инструкции плана; каждый из 6 диффов проверен вручную — только перенос строк/схлопывание box-shadow shorthand, 0 изменений значений/логики
- Явный hand-off чек-лист (4 пункта, ниже) зафиксирован для `/gsd-verify-work`

## Task Commits

Каждая задача закоммичена атомарно:

1. **Task 1: Полный check-tokens.mjs + verify-value-map.mjs прогон** — без кода-изменений (baseline уже чист, 0 нарушений найдено), коммит не создавался
2. **Task 2: pnpm lint/svelte-check зелёные + prettier-фикс** - `a7c9bbe` (style)

**Plan metadata:** будет закоммичена ниже (final_commit)

## Files Created/Modified
- `ui/src/features/acts/ActFormBody.svelte` — prettier line-wrap длинного toast-сообщения (без изменения текста)
- `ui/src/features/acts/PdfPreviewModal.svelte` — prettier line-wrap атрибутов `<iframe>`
- `ui/src/features/dashboard/ChartWidget.svelte` — prettier multiline-объект + line-wrap SVG `<text>`/`<div>` атрибутов
- `ui/src/lib/api/acts.ts` — prettier line-wrap однострочного метода `updateReturn`
- `ui/src/lib/components/PersonAutocomplete.svelte` — prettier multiline-атрибуты `<div class="dropdown--person">`
- `ui/src/styles/_tokens.scss` — prettier схлопывание многострочного `box-shadow`-shorthand (`--tr-elev-1..4`, обе темы) в одну строку на объявление
- `.planning/phases/23-design-tokens-foundations/23-06-SUMMARY.md` — этот файл

## Decisions Made
- Task 1 не потребовал ни одного точечного фикса — задокументировано как явное подтверждение (не "пусто, потому что не искали"), все acceptance criteria плана выполнены буквально, включая bonus-грep на `--shadow-md`
- `pnpm prettier --write .` применён к whole tree (не точечно к 6 файлам) — прогон затронул только эти 6 файлов (остальные уже были каноничны), подтверждено `git status --short` после запуска
- Каждый из 6 диффов вручную проверен на "форматирование, не логика" перед стейджингом — план явно требовал это подтверждение как условие безопасности коммита

## Deviations from Plan

None (по духу Rules 1-3) — единственное расхождение с "нулевым изменением кода" (`pnpm prettier --write .`) прямо предписано текстом самого плана Task 2 ("если prettier находит несоответствия, прогнать `pnpm prettier --write .` и подтвердить, что изменения ограничены пробельным форматированием") — это не незапланированная работа, а буквальное выполнение шага плана.

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Known Stubs
None — план не создавал новых компонентов/UI-состояний, только запускал верификацию и точечный prettier-фикс.

## Threat Flags
None — изменения ограничены форматированием существующего кода (line-wrap/box-shadow shorthand), новой поверхности (endpoints/auth/schema) не введено. `T-23-06-SC` (accept, 0 новых зависимостей) подтверждён: `pnpm prettier` — уже установленная dev-зависимость проекта, новых пакетов не добавлено.

## Hand-off чек-лист для `/gsd-verify-work` (не автоматически проверяемые пункты, D-09)

Жёсткое ограничение D-09 (locked): исполнитель НЕ запускает браузер. Большая часть UI — за логином и требует бэкенда с реальными данными. Это архитектурное ограничение, не пробел инструментария. Ниже — 4 пункта, требующие ручной проверки человеком через живой рендер:

1. **Переключение темы без артефактов (DS-02).** Проверить обе темы (светлая/тёмная) минимум на 3-4 плотных экранах: Устройства, форма акта, Настройки. Артефакты = флеш не той темы при загрузке, нестилизованные поверхности, нечитаемый текст. **Гоча:** если проверка идёт через LAN-браузер (не десктоп-Tauri-окно), сначала выполнить `pnpm --dir ui build` — сервер-режим раздаёт `ui/dist`, а `cargo tauri dev` хотрелоадит только desktop-webview, не LAN-браузер.
2. **Отсутствие сдвига вёрстки после space/radius миграции (DS-04).** `verify-value-map.mjs` доказывает, что замена математически value-preserving (те же px-значения), но НЕ доказывает, как каскад их отрендерит визуально. Сравнить до/после на тех же плотных экранах. **Важно:** инверсия поверхностей (`--tr-bg` #eef1f6 ↔ `--tr-surface` #ffffff) — это НАМЕРЕННОЕ изменение дизайна (D-10), не дефект миграции — не путать со сдвигом вёрстки.
3. **Визуальная иерархия типографики на правильном уровне (DS-03, визуальная часть).** Выбор уровня шкалы (h1..caption) зафиксирован в UI-SPEC и статически проверен (`check-tokens.mjs --rules=1` = 0 нарушений), но "выглядит правильно относительно соседних текстовых блоков" — вопрос визуального суждения. Точечно сверить заголовки/тело/подписи на тех же экранах против 9-уровневой шкалы.
4. **Точечная потеря контраста от инверсии поверхностей (DS-02 / D-11).** Белое-на-белом возможно локально там, где старый код полагался на прежний порядок поверхностей. Это НЕ обязательство этой фазы (обязательство — только сама миграция токенов), но полезный ожидаемый паттерн для UAT: чинится точечно через `--tr-border`/`--tr-elev-1`/`--tr-surface-sunken` (D-11), не через перетриаж каждого сайта поверхности по смыслу — тот перетриаж относится к фазам 24-28, не к этой.

## Next Phase Readiness
Вся статически проверяемая часть Phase 23 (DS-01, DS-02 нестатическая, DS-03 статическая, DS-04 статическая, QA-01) подтверждена зелёной одновременно, впервые за фазу: `check-tokens.mjs` (3/3 правила), `verify-value-map.mjs` (578 хунков, 0 нарушений), `pnpm svelte-check` (0 errors), `pnpm lint` (0 exit code, все три части — eslint/prettier/check-tokens.mjs). Явный hand-off чек-лист (4 пункта) передан для `/gsd-verify-work` — визуальная часть DS-02/DS-03/DS-04 не молчаливо считается done (D-09). Фаза 23 полностью готова к closing/verify-work; следующие фазы (24: базовые компоненты, 25: таблицы/Dropdown, 26-29: окна, 30: качество/доступность) наследуют чистый `--tr-*` слой и честный `pnpm lint` gate.

---
*Phase: 23-design-tokens-foundations*
*Completed: 2026-07-17*
