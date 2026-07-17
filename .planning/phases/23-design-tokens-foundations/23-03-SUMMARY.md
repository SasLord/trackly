---
phase: 23-design-tokens-foundations
plan: 03
subsystem: ui
tags: [scss, design-tokens, css-custom-properties, svelte, color, elevation]

# Dependency graph
requires:
  - phase: 23-01
    provides: "--tr-* token layer in ui/src/styles/_tokens.scss (color light+dark, elevation, spacing, radius, typography)"
  - phase: 23-02
    provides: "check-tokens.mjs permanent CI gate (3 rules) wired into pnpm lint"
provides:
  - "Все 103 файла ui/src, ссылавшиеся на --color-*/--shadow-elev-*/--shadow-md, переведены на --tr-*"
  - "0 hex-литералов внутри <style>-блоков во всём ui/src (DS-01 SC1 закрыт)"
  - "--shadow-md баг (3 сайта в cartridges/*) закрыт → --tr-elev-2 (тот же класс, что QA-01)"
  - "Bonus-находки --color-surface-hover (4 сайта) / --color-surface-muted (3 сайта) закрыты той же картой"
affects: [23-04, 23-05, 23-06]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Ordered literal-map sweep (25 записей, длинные имена раньше коротких substring-префиксов) применяется как последовательность точных string.split/join замен, не regex — исключает риск случайного regex-overmatch на границах имён токенов"
    - "После механического sweep токен-имён отдельным вторым проходом закрываются оставшиеся var(--tr-*, #hex)-fallback (унаследованы от старых --color-*, #hex паттернов) — --tr-* всегда определён, fallback лишний"

key-files:
  created: []
  modified:
    - "103 файла ui/src/**/*.svelte (полный список — git log c5ba66c..474fe07)"
    - "ui/src/lib/components/Button.svelte (доп. fix: #ffffff -> --tr-on-accent)"
    - "ui/src/lib/components/Badge.svelte (доп. fix: #ffffff -> --tr-on-accent)"
    - "ui/src/features/settings/NetworkSettings.svelte, ui/src/features/users/UserListRow.svelte (доп. fix: #27ae60/#1a7a40 -> --tr-success/--tr-success-text)"
    - "ui/src/features/acts/PdfPreviewModal.svelte (доп. fix: #fff -> --tr-n-0)"

key-decisions:
  - "var(--tr-text-inverse, #fff) в трёх auth-экранах (BlockedScreen/FirstRunWizard/LoginPage, .btn-submit на --tr-accent фоне) оставлен как --tr-text-inverse (fallback просто убран), а НЕ переименован в --tr-on-accent — сохраняет консистентность с уже существующим необновляемым в этом плане skip-link паттерном (Layout.svelte/EmployeeLayout.svelte используют --tr-accent+--tr-text-inverse без hex, вне scope Task 2); Button.svelte:78,103 и Badge.svelte .badge-accent, наоборот, явно указаны в UI-SPEC как --tr-on-accent-кейс, поэтому туда применён другой токен"
  - "NetworkSettings/UserListRow success-бейдж мигрирован на пару --tr-success (color-mix источник) / --tr-success-text (текст), а не на просто --tr-success для обоих — ближе к уже установленному -soft/-text triplet паттерну семантических токенов, чем к локальному прежнему hex-паттерну"

requirements-completed: [DS-01, QA-01]

duration: ~35min
completed: 2026-07-17
---

# Phase 23 Plan 03: Литеральный sweep --color-*/--shadow-* → --tr-* Summary

**Все 103 файла ui/src, использовавшие старые `--color-*`/`--shadow-elev-*`/`--shadow-md`, переведены на единый `--tr-*` слой по 25-пунктовой ordered-карте (Task 1), плюс устранены 12 оставшихся hardcoded hex-литералов внутри `<style>`-блоков через семантический маппинг по контексту (Task 2) — DS-01 (цветовой слой) и `--shadow-md`-часть QA-01 закрыты.**

## Performance

- **Duration:** ~35 min
- **Completed:** 2026-07-17
- **Tasks:** 2/2 completed
- **Files modified:** 115 (103 в Task 1, 12 в Task 2 — 5 файлов пересекаются между задачами)

## Accomplishments
- Task 1: mechanical ordered sweep всех 103 файлов, найденных `git grep`-ом на момент выполнения (совпало с research-оценкой ~105), применён 4 пакетами по ~26 файлов с промежуточной верификацией `git grep`
- `--color-error`/`--color-warning-bg`/`-text`/`-border` fallback-выражения (`var(--color-x, #hex)`) заменены целиком на голый `var(--tr-x)` — 22+3 сайта, fallback больше не нужен (`--tr-*` всегда определён)
- `--color-surface-raised`/`-sunken`/`-hover`/`-muted` разведены на `--tr-surface-raised`/`-sunken`/`--tr-row-hover`/`--tr-surface-sunken` СТРОГО до generic `--color-surface`, чтобы более общий префикс не «съел» специфичные имена раньше своей очереди
- `--shadow-md` (3 сайта: ModelListRow.svelte:169, CompatibilityEditor.svelte:283, ModelFormModal.svelte:539) → `--tr-elev-2` — QA-01-класс баг закрыт
- Task 2: 12 оставшихся hex-сайтов из `check-tokens.mjs --rules=2` — большинство оказались leftover `var(--tr-x, #hex)` fallback-паттернами (унаследованы от Task 1's substring-only переименования старых `--color-x, #hex` выражений, которые не входили в явный список из 4 "целиком выражение" замен), плюс 5 именных случаев (`Button.svelte`/`Badge.svelte` → `--tr-on-accent`, `NetworkSettings`/`UserListRow` success-бейдж → `--tr-success`/`--tr-success-text`, `PdfPreviewModal` paper background → `--tr-n-0`)
- Итог: `git grep -c 'var(--color-'` / `'var(--shadow-'` по всему `ui/src` → 0 файлов; `check-tokens.mjs --rules=2,3` → `PASS — 0 нарушений`; `pnpm svelte-check` → 0 errors, 48 pre-existing warnings (неизменный baseline)

## Task Commits

Каждая задача закоммичена атомарно (Task 1 — 4 коммита-пакета по плану "чанковать по 20-30 файлов"):

1. **Task 1 (batch 1/4): acts/auth/cartridges×6** - `c5ba66c` (feat)
2. **Task 1 (batch 2/4): cartridges/dashboard/devices/layout/printers×1** - `fe4685b` (feat)
3. **Task 1 (batch 3/4): printers/reports/requests/settings×5** - `c469ea4` (feat)
4. **Task 1 (batch 4/4): settings/users/lib/pages** - `474fe07` (feat)
5. **Task 2: оставшиеся hex-литералы** - `7ee20cc` (fix)

_Плановая docs-метадата коммитится отдельно ниже (final_commit)._

## Files Created/Modified
- 103 файла `ui/src/features/**/*.svelte` + `ui/src/lib/components/*.svelte` + `ui/src/pages/*.svelte` + `ui/src/App.svelte` — механический sweep токен-имён по ordered-карте (Task 1, коммиты `c5ba66c`..`474fe07`)
- `ui/src/features/acts/ActFormItemsTable.svelte`, `ReturnItemsTable.svelte`, `ReturnModal.svelte`, `PdfPreviewModal.svelte` — сняты hex-fallback'и / paper-background hex (Task 2)
- `ui/src/features/auth/BlockedScreen.svelte`, `FirstRunWizard.svelte`, `LoginPage.svelte` — снят `#fff`-fallback у `--tr-text-inverse` (Task 2)
- `ui/src/features/devices/DeviceImportCsvModal.svelte` — сняты 6 hex-fallback'ов (`--tr-border`/`--tr-accent`×2/`--tr-success`/`--tr-surface`×2/`--tr-accent-hover`/`--tr-danger`) (Task 2)
- `ui/src/features/settings/NetworkSettings.svelte`, `ui/src/features/users/UserListRow.svelte` — success-бейдж hex → `--tr-success`/`--tr-success-text` (Task 2)
- `ui/src/lib/components/Button.svelte`, `Badge.svelte` — `#ffffff` → `--tr-on-accent` (Task 2)

## Decisions Made
- `--tr-text-inverse` в трёх auth-экранах не переименован в `--tr-on-accent` несмотря на визуально идентичный "текст на accent-фоне" паттерн — сохранена консистентность с необновляемым в этом плане skip-link-паттерном в `Layout.svelte`/`EmployeeLayout.svelte`; переименование затронуло бы файлы вне заявленного `<files>`-скоупа Task 2 (только файлы с hex по выводу check-tokens), что было бы избыточным архитектурным решением за пределами точечного hex-фикса
- NetworkSettings/UserListRow success-бейдж — `color-mix(..., var(--tr-success) 15%, ...)` + `var(--tr-success-text)` вместо buквального `var(--tr-success)` для обоих полей — ближе к уже установленному в кодовой базе `-soft`/`-text` triplet-паттерну семантических токенов

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Byte-based `split -n 4` порвал одну строку списка файлов пополам**
- **Found during:** Task 1, подготовка первого пакета (chunk_00)
- **Issue:** Изначальная попытка нарезать список из 103 файлов на 4 пакета через `split -n 4` (без префикса `l/`) на macOS-версии `split` режет по БАЙТАМ, а не по строкам — путь `ui/src/features/cartridges/CartridgesMasterDetail.svelte` оказался разорван посередине на границе пакета (`ui/src/features/cartridge` + `sMasterDetail.svelte` в разных файлах), что привело к `ENOENT` при попытке прочитать несуществующий путь
- **Fix:** Пересчитаны пакеты через `split -l 26` (line-based), что гарантирует целостность каждой строки-пути; уже применённый частичный sweep (26 файлов chunk_00, все успешно обработанные ДО падения на разорванной строке) оказался идентичен первому line-based пакету — переделывать не потребовалось, только продолжить с правильной нарезкой оставшихся
- **Files modified:** нет файлов кодовой базы, инструментальная правка временного bash-скрипта нарезки
- **Verification:** `wc -l` по всем 4 line-based пакетам суммарно даёт 103 (совпадает с исходным списком); `git status --short` после первого пакета показал ровно 26 изменённых файлов, совпадающих построчно с первым line-based пакетом
- **Committed in:** не отдельный коммит — обнаружено и исправлено ДО первого коммита `c5ba66c`

---

**Total deviations:** 1 auto-fixed (1 blocking, инструментальное — не код проекта)
**Impact on plan:** Чисто механическая правка процесса нарезки списка файлов на пакеты; не затронула ни одного файла кодовой базы и не повлияла на итоговый результат (все 103 файла обработаны корректно, что подтверждено финальной сквозной проверкой `git grep`).

## Issues Encountered
None (помимо описанной выше инструментальной правки нарезки на пакеты, не затронувшей код).

## User Setup Required
None - no external service configuration required.

## Known Stubs
None — это чисто CSS-токен sweep, новых компонентов/данных/UI-состояний не добавлено.

## Threat Flags
None — изменения ограничены переименованием CSS custom-property ссылок и заменой нескольких hex-литералов на уже определённые в `_tokens.scss` (план 23-01) токены; новых trust boundaries, эндпоинтов или обработки пользовательского ввода не введено. Соответствует threat_model плана (T-23-03-01/02/SC — все `accept`, значения копируются дословно, sweep — только имена ссылок).

## Next Phase Readiness
Цветовой и elevation-слой в `ui/src` полностью на `--tr-*` — планы 23-04 (space/radius by value) и 23-05 (типографика by role) могут стартовать без пересечения по scope: остающиеся 1291 нарушение `check-tokens.mjs --rules=1` — исключительно `--space-*`/`--radius-*`/`--font-size-*`/`--font-weight-*`/`--line-height-*`, ни одного `--color-*`/`--shadow-*`. `pnpm svelte-check` остаётся на чистом baseline (0 errors, 48 pre-existing warnings, не связанных с этим планом).

---
*Phase: 23-design-tokens-foundations*
*Completed: 2026-07-17*

## Self-Check: PASSED

- FOUND: c5ba66c (feat(23-03): sweep --color-*/--shadow-* to --tr-* in acts/auth/cartridges (batch 1/4))
- FOUND: fe4685b (feat(23-03): sweep --color-*/--shadow-* to --tr-* in cartridges/dashboard/devices/layout/printers (batch 2/4))
- FOUND: c469ea4 (feat(23-03): sweep --color-*/--shadow-* to --tr-* in printers/reports/requests/settings (batch 3/4))
- FOUND: 474fe07 (feat(23-03): sweep --color-*/--shadow-* to --tr-* in settings/users/lib/pages (batch 4/4))
- FOUND: 7ee20cc (fix(23-03): remove remaining hardcoded hex from <style> blocks)
- FOUND: .planning/phases/23-design-tokens-foundations/23-03-SUMMARY.md
