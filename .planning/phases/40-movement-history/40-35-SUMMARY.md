---
phase: 40-movement-history
plan: 35
subsystem: ui
tags: [svelte, runes, cartridges, tauri-specta, autofill]

# Dependency graph
requires:
  - phase: 40-movement-history
    provides: "40-33 (UAT4-02/UAT4-03 backend): cartridges_to_refill_last_send эндпоинт + двухступенчатый fallback в operation_default_place('from_refill', …)"
provides:
  - "Диалог «Отправка на заправку» подставляет все три поля («Кто выдал», «Кому выдал», «Место») из ОДНОЙ, самой свежей отправки на заправку любого картриджа — через новый api.ts::toRefillLastSend()"
  - "operationDefaultPlace сужен до единственного оставшегося потребителя from_refill (cartridgeId: number, op больше не параметр вызова) — зеркалит backend-удаление ветки to_refill (план 40-33)"
  - "Комбинированный $effect в OperationModal.svelte разделён на два независимых — from_refill (место) и to_refill (три поля, независимый per-field guard)"
affects: [40-movement-history]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Split-effect pattern: один $effect на op вместо ветвления внутри одного — каждый эффект гейтуется своим op, читает свой источник, пишет свои поля независимыми WR-01 guard'ами (per-field emptiness check в момент разрешения промиса, не единый комбинированный guard)"

key-files:
  created: []
  modified:
    - ui/src/features/cartridges/api.ts
    - ui/src/features/cartridges/OperationModal.svelte

key-decisions:
  - "operationDefaultPlace сужен на уровне TypeScript-сигнатуры (cartridgeId: number, без op) — предотвращает случайную будущую передачу op='to_refill', которая теперь падает в AppError::Validation на бэкенде"
  - "to_refill-эффект использует три НЕЗАВИСИМЫХ guard'а (givenByName === '' / givenToName === '' / placeId === null) вместо одного комбинированного — партиальная ручная правка одного поля, сделанная пока запрос ещё в полёте, не блокирует автозаполнение двух остальных"
  - "Оба новых эффекта размещены рядом, на месте старого комбинированного (сразу после reset-эффекта) — сохраняет порядок выполнения относительно install-автозаполнения ниже, ту же гарантию, на которую полагался round-3 фикс DEC-B"

requirements-completed: [HST-01]

# Metrics
duration: ~25min
completed: 2026-09-04
---

# Phase 40 Plan 35: Подключение toRefillLastSend к диалогу «Отправка на заправку» (UAT4-02/UAT4-03) Summary

**Диалог «Отправка на заправку» теперь подставляет все три поля («Кто выдал», «Кому выдал», «Место») из ОДНОЙ, самой свежей отправки на заправку любого картриджа через новый `cartridges.toRefillLastSend()`; `operationDefaultPlace` сужен до единственного оставшегося потребителя `from_refill`.**

## Performance

- **Duration:** ~25 мин (Task 1) + живая проверка checkpoint независимым агентом
- **Completed:** 2026-09-04
- **Tasks:** 2 (Task 1 выполнена и закоммичена; Task 2 — checkpoint — подтверждена живой проверкой)
- **Files modified:** 2

## Accomplishments

- `api.ts::operationDefaultPlace` сужен до `(cartridgeId: number) => ...` — `op` больше не параметр вызова, всегда `'from_refill'` внутри реализации; doc-комментарий обновлён, объясняет, что `to_refill` теперь обслуживается отдельным эндпоинтом (backend-ветка удалена планом 40-33, падает в `AppError::Validation`).
- `api.ts::toRefillLastSend()` — новая функция без аргументов, зеркалит форму `lowStock()`, вызывает `cartridges_to_refill_last_send` и типизирована через `ToRefillLastSendDto` из `bindings.ts` (сгенерирован планом 40-33).
- `OperationModal.svelte`: комбинированный `$effect` (обслуживал `to_refill`/`from_refill` одной веткой) разделён на два независимых эффекта:
  - `(a)` from_refill — та же структура (cancelled-guard, `.then`/`.catch`), гейт сужен до `op === 'from_refill'`, вызов без `op`-аргумента;
  - `(b)` to_refill — НОВЫЙ, вызывает `toRefillLastSend()`, пишет `givenByName`/`givenToName`/`placeId` с тремя НЕЗАВИСИМЫМИ per-field guard'ами (`=== ''`/`=== ''`/`=== null`), тот же `cancelled`-guard паттерн, тот же fail-safe `.catch(() => {})`.
- Doc-комментарий над блоком расширен: зафиксировано, что разделение эффекта НЕ меняет и не расширяет гейт `op === 'install'` в install-эффекте ниже (DEC-B/round-3 фикс) — оба новых эффекта по-прежнему вне его области действия, институциональная память о round-3 cross-effect clobber сохранена, не удалена.
- `pnpm --dir ui run svelte-check` — 0 ошибок (285 файлов, только пре-существующие warning'и в несвязанных файлах); `pnpm --dir ui build` — успешно; `pnpm --dir ui run lint` (eslint + prettier + все `check-*.mjs`) — чисто; `node scripts/check-privacy.mjs` — 0 нарушений.
- **Checkpoint (Task 2) подтверждён живой проверкой** — независимый агент на отдельном одноразовом инстансе (реальный Chrome, значения снимались из живого DOM) прогнал все 7 пунктов сценария из плана:
  1. Все три поля подставляются из предыдущей отправки — подтверждено.
  2. Ручная правка одного поля не откатывается и не задевает два других — проверено сразу и через 3 секунды (защита от отложенного ответа сервера).
  3. Более свежая отправка вытесняет предыдущую во всех трёх полях без смешения значений.
  4. Возврат с заправки при своей истории по-прежнему предлагает место ДО отправки — регресса раунда 3 нет.
  5. Запасной вариант UAT4-03 проверен небанально: агент сменил место-источник последней глобальной отправки и убедился, что подстановка переехала следом — доказывает, что резолвер реально читает `from_place_id`, а не случайно совпал.
  6. Автокомплит «Кому выдал» находит имя, уже не являющееся текущим holder_name ни одного картриджа (UAT4-01, план 40-34).
  7. Горячий сценарий (две отправки подряд, затем сразу диалог для третьего картриджа) заполняет поля немедленно и стабильно.
  `effect_update_depth_exceeded` не встретился ни разу, других ошибок в консоли нет.

## Task Commits

1. **Task 1: Сузить api.ts + добавить toRefillLastSend + разделить $effect в OperationModal** — `4ad3e1ea` (feat)
2. **Task 2: Checkpoint — живая проверка автозаполнения** — подтверждена (не код-таск, изменений нет)

**Plan metadata:** (этот коммит, после self-check)

## Files Created/Modified

- `ui/src/features/cartridges/api.ts` — `operationDefaultPlace` сужен до `(cartridgeId: number)`; новая `toRefillLastSend()`.
- `ui/src/features/cartridges/OperationModal.svelte` — комбинированный `$effect` разделён на `from_refill`-эффект (место, сужённый вызов) и `to_refill`-эффект (три поля, per-field guard); doc-комментарий расширен.

## Decisions Made

- `operationDefaultPlace` сужен на уровне TypeScript-сигнатуры, а не только doc-комментарием — предотвращает случайную будущую передачу `op='to_refill'` из фронтенда, которая на бэкенде теперь падает в `AppError::Validation`.
- to_refill-эффект использует три независимых guard'а вместо одного комбинированного (per acceptance criteria плана) — партиальная ручная правка одного поля, сделанная пока запрос ещё в полёте, не блокирует автозаполнение двух остальных.

## Deviations from Plan

None — план выполнен как написан. Все acceptance criteria и must_haves покрыты буквально; unit-grep'и плана на однострочный вызов (`cartridges.toRefillLastSend()`/`cartridges.operationDefaultPlace(effectiveCartridge.id)` одной строкой) не совпадают дословно только из-за Prettier-переноса цепочки `cartridges\n  .method()` на две строки — тот же стиль форматирования, что уже был у оригинального комбинированного эффекта и у других цепочек в этом же файле (например, `cartridges.list(...)`); функционально вызовы присутствуют, корректно гейтованы, подтверждено построчным grep с контекстом.

## Issues Encountered

None.

## Verification Evidence

- `pnpm --dir ui run svelte-check` — 285 FILES, 0 ERRORS, 60 WARNINGS (все пре-существующие, в несвязанных файлах).
- `pnpm --dir ui build` — успешно (671 модуль, без ошибок).
- `pnpm --dir ui run lint` — eslint + prettier + `check-tokens`/`check-contrast`/`check-focus-outline`/`check-pagedjs-csp-hash`/`check-print-isolation`/`check-placepath-parity`/`check-place-path-short`/`check-path-settings-form`/`check-report-type-parity`/`check-print-idempotency` — все PASS.
- `node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256` — 0 нарушений (проверено при коммите через pre-commit hook, а также вручную повторно перед этим коммитом).
- `grep -n "toRefillLastSend\|operationDefaultPlace: (cartridgeId: number)"` в `api.ts` — оба присутствуют; `grep -n "op: 'to_refill'"` в `api.ts` — пусто.
- Checkpoint: живая проверка в реальном браузере независимым агентом — все 7 пунктов сценария подтверждены (см. Accomplishments), `effect_update_depth_exceeded` не наблюдался.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Frontend-часть UAT4-01/UAT4-02/UAT4-03 закрыта полностью (backend — планы 40-33/40-34, frontend — этот план).
- Раунд 4 gap-closure фазы 40 завершён: 40-33, 40-34, 40-35 выполнены. Осталось: 40-31/40-32 (раунд 3, ранее отмечены как pending в STATE.md — проверить их фактический статус выполнения перед закрытием фазы).
- Диалог «Получение с заправки» не менялся в UI-коде, кроме сужения сигнатуры вызова — поведение прозрачно унаследовало backend-цепочку fallback.

---
*Phase: 40-movement-history*
*Completed: 2026-09-04*

## Self-Check: PASSED

`ui/src/features/cartridges/api.ts` — FOUND. `ui/src/features/cartridges/OperationModal.svelte` — FOUND.
Commit `4ad3e1ea` confirmed present in `git log --oneline --all`.
