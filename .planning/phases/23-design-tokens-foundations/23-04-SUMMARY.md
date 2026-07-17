---
phase: 23-design-tokens-foundations
plan: 04
subsystem: ui
tags: [scss, design-tokens, css-custom-properties, svelte, spacing, radius]

# Dependency graph
requires:
  - phase: 23-01
    provides: "--tr-* token layer in ui/src/styles/_tokens.scss (11-level spacing scale, 5-level radius scale)"
  - phase: 23-02
    provides: "check-tokens.mjs permanent CI gate (3 rules) + verify-value-map.mjs one-shot value-preserving verifier"
  - phase: 23-03
    provides: "--color-*/--shadow-* fully migrated to --tr-* (0 remaining) — clean separation, no overlap with this plan's scope"
provides:
  - "Все --space-*/--radius-md/--radius-lg call-sites (105 файлов) переведены на --tr-* ПО ЗНАЧЕНИЮ — 0 нарушений в verify-value-map.mjs"
  - "--radius-sm полностью выведен из кодовой базы: 4 allowlist-файла (Button/Input/Select/Textarea) -> --tr-radius-sm (6px, намеренный сдвиг D-07), все остальные (56 файлов) -> --tr-radius-xs (4px, value-preserving)"
  - "--radius-lg QA-01 undefined-token баг закрыт в 4 auth-экранах (LoginPage/BlockedScreen/FirstRunWizard/PendingScreen)"
  - "BASE_SHA (до правок space/radius): 6425d30cf3e8acdc7d163c41dd3515d0eded88b5"
affects: [23-05, 23-06]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Ordered literal-map sweep (9-точечная карта из <interfaces> плана) через zero-dependency Node-скрипт с regex-alternation (longest-match-first, negative lookahead на границу имени) вместо построчного sed/perl — устойчиво к отсутствию substring-коллизий между суффиксами шкалы"
    - "verify-value-map.mjs как post-hoc git-diff верификатор (не встроен в постоянный CI-гейт) — доказывает value-preserving свойство миграции на самом диффе плана, а не на снапшоте кода"

key-files:
  created: []
  modified:
    - "105 файлов ui/src/**/*.svelte + 1 ui/src/lib/utils/dropdownAnchor.ts (doc-comment reference) — полный список: git diff 6425d30..HEAD -- ui/src"
    - "ui/scripts/verify-value-map.mjs (bug fix: RADIUS_EXCEPTION_FILES path prefix)"

key-decisions:
  - "verify-value-map.mjs (построен в 23-02) содержал баг: RADIUS_EXCEPTION_FILES сравнивался с git-diff путями без префикса 'ui/', хотя корень репозитория — родитель ui/, а не сама ui/ (git diff всегда выдаёт repo-root-relative пути). Это давало 4 false-positive нарушения РОВНО на ожидаемом исключении (--radius-sm -> --tr-radius-sm в 4 allowlist-файлах) при любом реальном запуске. Исправлено добавлением префикса 'ui/' к каждой записи набора — Rule 1 (блокирующий баг инструмента, найден при выполнении acceptance criteria этого плана)"
  - "3 сайта var(--radius-sm, 4px) в DeviceImportCsvModal.svelte (fallback-форма) не попали в первоначальный список файлов Task 2 (grep искал точную подстроку 'var(--radius-sm)' без учёта запятой перед fallback-значением) — найдены повторным прогоном check-tokens.mjs --rules=1 после Task 2 и докоммичены в тот же коммит (Rule 1 - блокирующий баг обнаружения файлов, не код проекта)"

requirements-completed: [DS-04, QA-01]

duration: ~50min
completed: 2026-07-17
---

# Phase 23 Plan 04: Литеральный sweep --space-*/--radius-* → --tr-* по значению Summary

**Все 651+ site использований `--space-*` и 134+ `--radius-md`/`--radius-lg`/`--radius-sm` в 105 файлах `ui/src` переведены на `--tr-*` эквиваленты ПО ЗНАЧЕНИЮ (значения не меняются), кроме двух явно объявленных исключений — split `--radius-sm` по 4-файловому allowlist (D-07) и фикс QA-01 undefined-токена `--radius-lg` в 4 auth-экранах — подтверждено `verify-value-map.mjs` с 0 нарушений на полном диффе плана.**

## Performance

- **Duration:** ~50 min
- **Completed:** 2026-07-17
- **Tasks:** 2/2 completed
- **Files modified:** 105 (ui/src) + 1 (ui/scripts/verify-value-map.mjs bug fix)

## Accomplishments
- Task 1: mechanical ordered-map sweep 9-пунктовой карты из `<interfaces>` плана применён 4 пакетами по 26 файлов (104 файла, line-based `split -l 26`, не byte-based — избежан баг 23-03) с промежуточной верификацией `check-tokens.mjs --rules=1` после каждого пакета
- `--radius-lg` (QA-01 undefined-token баг) закрыт в 4 auth-экранах: LoginPage.svelte:175, BlockedScreen.svelte:125, FirstRunWizard.svelte:190, PendingScreen.svelte:37 — все получили `--tr-radius-lg` (12px)
- Task 2: `--radius-sm` split по allowlist — ровно 4 файла (Button/Input/Select/Textarea, по 1 сайту каждый) получили `--tr-radius-sm` (6px, намеренный сдвиг 4→6 по D-07), остальные 56 файлов (98 var()-вхождений) получили `--tr-radius-xs` (4px, value-preserving)
- `verify-value-map.mjs` подтверждает: **PASS — 578 хунков проверено, 0 нарушений** на полном диффе Task 1 + Task 2 относительно BASE_SHA
- Итог: `check-tokens.mjs --rules=1` не находит НИ ОДНОГО `--space-*`/`--radius-*` нарушения (517 оставшихся нарушений — исключительно `--font-size-*`/`--font-weight-*`/`--line-height-*`, вне scope этого плана, ожидают 23-05); `pnpm svelte-check` → 0 errors, 48 pre-existing warnings (неизменный baseline)

## Task Commits

Каждая задача закоммичена атомарно (Task 1 — 4 коммита-пакета; Task 2 — 1 коммит + 1 фикс-коммит инструмента):

1. **Task 1 (batch 1/4): acts/auth/cartridges** - `16244e2` (feat) — включает QA-01 radius-lg fix в 4 auth-файлах
2. **Task 1 (batch 2/4): cartridges/dashboard/devices/layout/printers** - `66705ee` (feat)
3. **Task 1 (batch 3/4): printers/reports/requests/settings** - `15d36a8` (feat)
4. **Task 1 (batch 4/4): settings/users/lib/pages** - `9a0fbb6` (feat)
5. **Task 2: split --radius-sm по allowlist** - `ea6af95` (fix)
6. **Инструментальный фикс: verify-value-map.mjs path prefix** - `ddce311` (fix)

_Плановая docs-метадата коммитится отдельно ниже (final_commit)._

## Files Created/Modified
- 104 файла `ui/src/features/**/*.svelte` + `ui/src/lib/**/*.svelte` + `ui/src/pages/*.svelte` — механический sweep 9-пунктовой карты (Task 1, коммиты `16244e2`..`9a0fbb6`)
- `ui/src/lib/utils/dropdownAnchor.ts` — doc-comment ссылка `--space-xs` -> `--tr-space-2xs` (не .svelte/.scss, но найден тем же grep-паттерном плана, который не ограничен расширением)
- `ui/src/features/auth/LoginPage.svelte`, `BlockedScreen.svelte`, `FirstRunWizard.svelte`, `PendingScreen.svelte` — QA-01 `--radius-lg` fix (Task 1) + radius-sm split (Task 2, все 4 попали в default `--tr-radius-xs`, не в allowlist по radius-sm — allowlist для radius-sm касается других 4 файлов: Button/Input/Select/Textarea)
- `ui/src/lib/components/Button.svelte`, `Input.svelte`, `Select.svelte`, `Textarea.svelte` — `--radius-sm` -> `--tr-radius-sm` (6px, единственное разрешённое value-shift исключение, D-07)
- `ui/src/features/devices/DeviceImportCsvModal.svelte` — 3 доп. сайта `var(--radius-sm, 4px)` fallback-формы, найдены повторной проверкой после Task 2, докоммичены в `ea6af95`
- `ui/scripts/verify-value-map.mjs` — фикс `RADIUS_EXCEPTION_FILES` (добавлен префикс `ui/` к 4 путям)

## Decisions Made
- `verify-value-map.mjs` баг: `RADIUS_EXCEPTION_FILES` содержал пути без `ui/`-префикса (`src/lib/components/Button.svelte`), хотя `git diff` всегда выдаёт repo-root-relative пути (`ui/src/lib/components/Button.svelte`), поскольку корень репозитория — `trackly/`, а не `trackly/ui/`. Это гарантированно давало 4 false-positive value-mismatch на РОВНО ожидаемом исключении при любом реальном прогоне скрипта. Исправлено добавлением префикса ко всем 4 записям (Rule 1 — блокирующий баг инструмента, обнаружен при выполнении acceptance criteria Task 2 этого плана, не преждевременная находка)
- Первоначальный список файлов для Task 2 (`git grep -l -- 'var(--radius-sm)' ui/src`) не поймал 3 сайта fallback-формы `var(--radius-sm, 4px)` в `DeviceImportCsvModal.svelte` (запятая перед fallback-значением ломает точное совпадение подстроки `var(--radius-sm)`) — найдено повторным прогоном `check-tokens.mjs --rules=1` сразу после Task 2 и докоммичено в тот же коммит `ea6af95` (не отдельный коммит, т.к. обнаружено до финализации Task 2)
- `BASE_SHA` (для `verify-value-map.mjs` и плана 23-06): `6425d30cf3e8acdc7d163c41dd3515d0eded88b5` — последний коммит перед началом Task 1 этого плана

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Blocking] `verify-value-map.mjs` (построен в 23-02) содержал path-prefix баг**
- **Found during:** Task 2 verification (запуск `node ui/scripts/verify-value-map.mjs <BASE_SHA>` после коммита `ea6af95`)
- **Issue:** `RADIUS_EXCEPTION_FILES` в скрипте сравнивался с `filePath`, извлечённым из `git diff` заголовков (`b/...` часть) — эти заголовки ВСЕГДА repo-root-relative (`ui/src/lib/components/Button.svelte`), а не relative к `ui/` (в котором физически лежит и выполняется скрипт), потому что корень git-репозитория — родительская директория `trackly/`, не `ui/`. Набор содержал пути без префикса `ui/` (`src/lib/components/Button.svelte`), поэтому сравнение никогда не совпадало — скрипт репортил 4 false-positive `value-mismatch` РОВНО на единственном разрешённом исключении плана (radius-sm allowlist), блокируя acceptance criteria «exit-код 0» из Task 2
- **Fix:** Добавлен префикс `ui/` ко всем 4 записям `RADIUS_EXCEPTION_FILES` + пояснительный комментарий о repo-root-relative природе git diff путей
- **Files modified:** `ui/scripts/verify-value-map.mjs`
- **Verification:** `node ui/scripts/verify-value-map.mjs 6425d30cf3e8acdc7d163c41dd3515d0eded88b5` → `PASS — 578 хунков проверено, 0 нарушений`
- **Committed in:** `ddce311` (fix)

**2. [Rule 1 - Blocking] Первоначальный список файлов Task 2 не поймал fallback-форму `var(--radius-sm, 4px)`**
- **Found during:** Task 2, повторная верификация `check-tokens.mjs --rules=1` после применения split (сразу после первого прогона скрипта на списке из `git grep -l -- 'var(--radius-sm)' ui/src`)
- **Issue:** Команда обнаружения файлов для Task 2 использовала точную подстроку `var(--radius-sm)` (с закрывающей скобкой сразу после имени токена), но 3 сайта в `DeviceImportCsvModal.svelte` использовали fallback-форму `var(--radius-sm, 4px)` — запятая перед закрывающей скобкой ломала точное совпадение, файл выпал из первого прохода split-скрипта
- **Fix:** Найдено через `git grep -n -F -- '--radius-sm' ui/src` (без требования закрывающей скобки сразу после), применена та же логика (файл не в allowlist → `--tr-radius-xs`, fallback-значение `4px` оставлено как есть — совпадает по значению)
- **Files modified:** `ui/src/features/devices/DeviceImportCsvModal.svelte`
- **Verification:** `check-tokens.mjs --rules=1` больше не находит `--radius-sm` нигде в `ui/src`; `git grep -n -F -- '--radius-sm' ui/src` пусто
- **Committed in:** `ea6af95` (часть коммита Task 2, найдено до его финализации — не отдельный коммит)

---

**Total deviations:** 2 auto-fixed (2 blocking — оба инструментальные/обнаружение-файлов, не логические ошибки в самой миграции кода)
**Impact on plan:** Оба отклонения — фикс инструмента верификации и расширение покрытия обнаружения файлов; ни одно не изменило саму value-map логику миграции. Итоговый результат (0 `--space-*`/`--radius-*` нарушений, `verify-value-map.mjs` PASS) достигнут корректно.

## Issues Encountered
None, помимо описанных выше 2 инструментальных отклонений.

## User Setup Required
None - no external service configuration required.

## Known Stubs
None — это чисто CSS-токен sweep, новых компонентов/данных/UI-состояний не добавлено.

## Threat Flags
None — изменения ограничены переименованием CSS custom-property ссылок (значения либо сохранены дословно, либо явно задокументированный сдвиг D-07/QA-01). Соответствует threat_model плана (T-23-04-01 mitigate закрыт `verify-value-map.mjs` PASS; T-23-04-SC accept, 0 новых зависимостей).

## Next Phase Readiness
Space/radius слой в `ui/src` полностью на `--tr-*` (кроме `_tokens.scss`, который легитимно вводит значения, не участвует в sweep). Единственное оставшееся семейство для `check-tokens.mjs --rules=1` — типографика (`--font-size-*`/`--font-weight-*`/`--line-height-*`, 517 нарушений) — план 23-05 закрывает его тем же паттерном (ordered value-map + verify по значению, если применимо). Baseline `pnpm svelte-check` не изменён (0 errors, 48 pre-existing warnings). `BASE_SHA` для плана 23-06: `6425d30cf3e8acdc7d163c41dd3515d0eded88b5` (последний коммит перед Task 1 этого плана — читается из этой SUMMARY, не реконструируется из git-истории).

---
*Phase: 23-design-tokens-foundations*
*Completed: 2026-07-17*

## Self-Check: PASSED

- FOUND: ui/src/styles/_tokens.scss
- FOUND: ui/scripts/verify-value-map.mjs
- FOUND: .planning/phases/23-design-tokens-foundations/23-04-SUMMARY.md
- FOUND: 16244e2 (feat(23-04): sweep batch 1/4)
- FOUND: 66705ee (feat(23-04): sweep batch 2/4)
- FOUND: 15d36a8 (feat(23-04): sweep batch 3/4)
- FOUND: 9a0fbb6 (feat(23-04): sweep batch 4/4)
- FOUND: ea6af95 (fix(23-04): radius-sm allowlist split)
- FOUND: ddce311 (fix(23-04): verify-value-map.mjs path prefix)
- FOUND: 26c60d0 (docs(23-04): add plan summary)
