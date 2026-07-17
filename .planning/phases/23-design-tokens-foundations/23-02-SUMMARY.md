---
phase: 23-design-tokens-foundations
plan: 02
subsystem: ui
tags: [tooling, ci-gate, eslint, lint, design-tokens]

# Dependency graph
requires: ["23-01"]
provides:
  - "ui/scripts/check-tokens.mjs — постоянный CI-гейт (D-04): old-name / hex-in-style / closed-world --tr-* existence, встроен в pnpm lint"
  - "ui/scripts/verify-value-map.mjs — one-shot git-diff верификатор value-preserving space/radius миграции (D-08), для планов 23-04/23-05"
  - "pnpm lint (eslint-часть) чист — D-15 закрыт, 0 из прежних 5 pre-existing ошибок"
affects: [23-03, 23-04, 23-05, 23-06]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Zero-dependency Node ESM dev-скрипты (node:fs/node:path/node:child_process) — грепает, не парсит (следуя явному отклонению stylelint в D-04)"
    - "Comment-stripping guard в closed-world gate (Rule 3) — документация с {role}-плейсхолдером в комментариях не должна триггерить false positive"

key-files:
  created:
    - ui/scripts/check-tokens.mjs
    - ui/scripts/verify-value-map.mjs
    - .planning/phases/23-design-tokens-foundations/deferred-items.md
  modified:
    - ui/package.json
    - ui/eslint.config.js
    - ui/src/features/acts/ActFormItemsTable.svelte

key-decisions:
  - "Rule 3 (closed-world gate) страйпит // и /* */ комментарии перед матчингом — иначе документация в _tokens.scss (`var(--tr-text-{role})`) даёт ложное 'undefined token' на самом токен-файле"
  - "scripts/**/*.mjs добавлен в тот же eslint.config.js file-pattern block, что vite.config.ts/eslint.config.js — иначе новые .mjs-скрипты падают на no-undef (console/process/URL), т.к. ни один существующий glob их не покрывает"
  - "Prettier-формат 7 файлов (включая ActFormItemsTable.svelte, _tokens.scss) — подтверждённо pre-existing до Phase 23 (git show на коммит до 23-01), логировано в deferred-items.md, не исправлено — вне scope D-15 (только 5 eslint-ошибок)"

patterns-established:
  - "Грep-гейты этой фазы (D-04) — единственная автоматическая защита от 'резолвится в ничто' для цвета/типографики; space/radius дополнительно защищены value-map верификатором (D-08)"

requirements-completed: [QA-01]

duration: ~20min
completed: 2026-07-17
---

# Phase 23 Plan 02: Гейты токенов и D-15 eslint-фикс Summary

**`check-tokens.mjs` (постоянный 3-правильный CI-гейт в `pnpm lint`) + `verify-value-map.mjs` (one-shot git-diff верификатор D-08) + фикс всех 5 pre-existing eslint-ошибок (D-15), разблокировавший `pnpm lint` как честный гейт для v1.2.**

## Performance

- **Duration:** ~20 min
- **Completed:** 2026-07-17
- **Tasks:** 2/2 completed
- **Files modified:** 6 (3 created, 3 modified)

## Accomplishments
- `check-tokens.mjs`: три независимо запускаемые проверки (old-name gate / hex-in-`<style>` gate / closed-world `--tr-*` existence gate), встроен в `pnpm lint` через `&&`
  - Rule 1 (`--rules=1`) находит 2315 нарушений на текущем домиграционном дереве (порядок величины research-замера подтверждён, порог ≥600 пройден)
  - Rule 2 (`--rules=2`) находит 47 hex-нарушений (совпадает с ground-truth 47:47)
  - Rule 3 (`--rules=3`) чист (0 нарушений) — после фикса comment-stripping bug (см. Deviations)
- `verify-value-map.mjs`: git-diff based, принимает `<base-ref>`, SPACE_MAP (7 записей) + RADIUS split-функция (D-07 allowlist), `PASS`/`FAIL` с описанием `count-mismatch`/`value-mismatch`; смок-тест `HEAD` (пустой diff) → `PASS — 0 хунков проверено, 0 нарушений`
- D-15: все 5 pre-existing eslint-ошибок исправлены (`browserGlobals` +4 имени, `ActFormItemsTable.svelte` useless-assignment убран) — `eslint . --ext .ts,.svelte` теперь возвращает 0 ошибок

## Task Commits

Каждая задача закоммичена атомарно:

1. **Task 1: check-tokens.mjs — постоянный греп-гейт + wiring в package.json** - `5456897` (feat)
2. **Task 2: verify-value-map.mjs + D-15 eslint-фикс** - `4ccba10` (feat)

_Плановая docs-метадата коммитится отдельно ниже (final_commit)._

## Files Created/Modified
- `ui/scripts/check-tokens.mjs` — новый, 3 независимые проверки, `--rules=1,2,3` флаг, устойчив к отсутствующей `ui/src`
- `ui/scripts/verify-value-map.mjs` — новый, one-shot git-diff верификатор D-08, НЕ встроен в постоянный `pnpm lint`
- `ui/package.json` — `"lint"` расширен третьим шагом `&& node scripts/check-tokens.mjs`
- `ui/eslint.config.js` — `browserGlobals` +4 (`HTMLUListElement`/`SVGRectElement`/`SVGSVGElement`/`btoa`); новый file-pattern block покрывает `scripts/**/*.mjs` node+browser globals
- `ui/src/features/acts/ActFormItemsTable.svelte` — `let filtered: DeviceGroup[] = [];` → `let filtered: DeviceGroup[];` (инициализатор никогда не читался)
- `.planning/phases/23-design-tokens-foundations/deferred-items.md` — новый, документирует 7 pre-existing prettier-файлов вне scope

## Decisions Made
- Rule 3 comment-stripping guard: `//`/`/* */` вырезаются перед DEFINE_RE/USE_RE матчингом, с защитой от обрезания `http://`/`https://`/`file://` URL-схем (эвристика «не резать `//`, если непосредственно перед ним `:`»)
- `scripts/**/*.mjs` добавлен в существующий node-config file-pattern block (не новый отдельный блок) — минимальное изменение eslint.config.js
- 7 pre-existing prettier-несоответствий (включая файл, который я редактировал) — подтверждённо предшествуют Phase 23 целиком (проверено `git show <pre-23-01-коммит>:<файл> | prettier --check`), задокументированы, не исправлены (вне scope D-15)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Rule 3 (closed-world gate) ложно триггерился на собственной документации в _tokens.scss**
- **Found during:** Task 1, ручная проверка `node scripts/check-tokens.mjs --rules=3`
- **Issue:** Комментарий в `_tokens.scss` (`... composite shorthand (font: var(--tr-text-{role})) AND ...`, введён планом 23-01) содержит `var(--tr-text-{role})` — символ `{` не входит в `[a-z0-9-]+`, поэтому `USE_RE` матчит усечённое `--tr-text-` как "использованный" токен, которого нет в `defined`-множестве → ложный violation прямо на файле-источнике токенов
- **Fix:** Добавлена функция `stripCommentsForRule3()` — вырезает `/* */`-блоки и `//`-строчные комментарии (с защитой от обрезания `http://`-подобных URL-схем) перед матчингом DEFINE_RE/USE_RE в Rule 3 only (Rule 1/2 не тронуты — их дизайн явно требует сканировать файл целиком/весь `<style>`-блок)
- **Files modified:** ui/scripts/check-tokens.mjs
- **Verification:** `node scripts/check-tokens.mjs --rules=3` → `PASS — 0 нарушений`
- **Committed in:** `5456897` (часть коммита Task 1)

**2. [Rule 3 - Blocking] Новые .mjs-скрипты падали на eslint no-undef (console/process/URL)**
- **Found during:** Task 2, запуск `pnpm lint` целиком после фикса D-15
- **Issue:** Ни один существующий `eslint.config.js` file-pattern glob не покрывал `ui/scripts/**/*.mjs` — файлы попадали только под `js.configs.recommended` без globals, из-за чего `console`/`process`/`URL` репортились как `no-undef` (22 новые ошибки в двух новых скриптах)
- **Fix:** Добавлен `scripts/**/*.mjs` в существующий node-config file-pattern block (тот же, что уже покрывает `vite.config.ts`/`svelte.config.js`/`eslint.config.js`), даёт `nodeGlobals`+`browserGlobals`
- **Files modified:** ui/eslint.config.js
- **Verification:** `npx eslint . --ext .ts,.svelte` → 0 ошибок
- **Committed in:** `4ccba10` (часть коммита Task 2)

---

**Total deviations:** 2 auto-fixed (1 bug, 1 blocking issue)
**Impact on plan:** Оба фикса — прямое следствие написания новой инфраструктуры этим же планом (не затрагивают чужой код), устранены до коммита, задача выполнена полностью по духу плана.

## Issues Encountered
- `pnpm lint` (полностью, включая prettier-шаг) сегодня всё ещё возвращает non-zero — но причина сменилась с «5 eslint-ошибок» на «7 pre-existing prettier-файлов» (оба класса предшествуют этому плану; prettier-класс не входил в scope D-15 — см. Decisions + `deferred-items.md`).

## User Setup Required
None - no external service configuration required.

## Known Stubs
None — оба скрипта полностью функциональны, не заглушки.

## Threat Flags
None — оба скрипта строго dev-tooling (читают исходники репозитория и вывод `git diff`), новых trust boundaries не вводят; T-23-02-01/T-23-02-SC из threat_model плана остаются единственными применимыми и уже отражены в дизайне (`execSync` с фиксированной командой, ноль новых npm-пакетов).

## Next Phase Readiness
`check-tokens.mjs` готов как честный постоянный гейт для планов 23-03..23-06 (цвет/space-radius/типографика/финальная проверка) — уже сейчас корректно детектирует все домиграционные нарушения по всем трём правилам. `verify-value-map.mjs` готов к использованию планом 23-04 (space/radius sweep) сразу после первого sweep-коммита (`node scripts/verify-value-map.mjs <sha-до-sweep>`). `pnpm lint` (eslint-часть) — честный зелёный baseline для всех последующих фаз v1.2; prettier-часть остаётся red по независимой, задокументированной причине (`deferred-items.md`).

---
*Phase: 23-design-tokens-foundations*
*Completed: 2026-07-17*

## Self-Check: PASSED

- FOUND: ui/scripts/check-tokens.mjs
- FOUND: ui/scripts/verify-value-map.mjs
- FOUND: .planning/phases/23-design-tokens-foundations/deferred-items.md
- FOUND: 5456897 (feat(23-02): add check-tokens.mjs permanent CI gate, wire into pnpm lint)
- FOUND: 4ccba10 (feat(23-02): add verify-value-map.mjs, fix D-15 pre-existing eslint errors)
- FOUND: 865aafb (docs(23-02): add plan summary)
