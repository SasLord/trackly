---
phase: 40-movement-history
plan: 34
subsystem: database
tags: [rusqlite, sqlite, audit_log, suggest_person, autocomplete]

# Dependency graph
requires:
  - phase: 40-movement-history
    provides: "Фаза 12 (GAP-12-06): given_by_name_arm — прецедент UNION ALL арки по audit_log.payload_json внутри suggest_person"
provides:
  - "given_to_name_arm — симметричная given_by_name_arm ветка в suggest_person, читает audit_log.payload_json->given_to_name для custom:install/custom:to_refill, активна только для SuggestPersonField::Receiver"
affects: [40-movement-history, ui-cartridge-consumers]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Симметричные UNION ALL арки по одному format!() шаблону: given_by_name_arm (Giver) и given_to_name_arm (Receiver) идентичны по структуре SQL, различаются только JSON-ключом и enum-веткой, обе гейтуются match field { ... => \"\" } без string-interpolation от пользователя"

key-files:
  created: []
  modified:
    - crates/trackly-app/src/services/act_service.rs
    - crates/trackly-app/tests/acts_suggest.rs

key-decisions:
  - "Не расширять action IN (...) фильтр за пределы install/to_refill — GAP-12-12 auto-return (custom:return_to_stock) тоже пишет given_to_name в свой payload_json, но это сознательно вне области (симметрично уже принятому решению для given_by_name_arm)"
  - "Регрессионный тест на реальный симптом идёт через настоящий CartridgeService.transition() (ToRefill -> FromRefill -> Install), а не через ручной посев audit_log сырым SQL — раунд 1 этой фазы уже проваливался на таком посеве"

requirements-completed: [HST-01]

# Metrics
duration: ~20min
completed: 2026-09-04
---

# Phase 40 Plan 34: given_to_name_arm — автокомплит «Кому выдал» видит историю заправок (UAT4-01) Summary

**Симметричная `given_by_name_arm` арка `given_to_name_arm` добавлена в `suggest_person`, читает `audit_log.payload_json->given_to_name` для install/to_refill операций картриджа — автокомплит «Кому выдал» больше не теряет имена при перезаписи `cartridges.holder_name` следующей операцией того же картриджа.**

## Performance

- **Duration:** ~20 мин
- **Completed:** 2026-09-04T18:39:24+07:00
- **Tasks:** 2 (обе выполнены)
- **Files modified:** 2

## Accomplishments

- В `act_service.rs::suggest_person` добавлена вторая гейтованная UNION ALL арка `given_to_name_arm` — буквальное зеркало `given_by_name_arm` (тот же `action IN ('custom:install', 'custom:to_refill')`, тот же `escape_like`-экранированный `LIKE ?1`, тот же `GROUP BY json_extract(...)`), активна только для `SuggestPersonField::Receiver`, инвертирована относительно `given_by_name_arm` (та активна только для `Giver`).
- Doc-комментарий над `suggest_person` исправлен: убрано ложное утверждение, что `given_to_name` «уже покрыт» аркой `cartridges.holder_name` — заменено явным объяснением, что `holder_name` хранит только ТЕКУЩЕЕ значение картриджа и перезаписывается каждой новой install/to_refill операцией, поэтому более ранние получатели структурно терялись (UAT4-01, `40-HUMAN-UAT.md`).
- 4 новых прямых SQL-теста в `acts_suggest.rs`, зеркалящих существующие тесты `given_by_name_arm` (install/to_refill находят имя, нерелевантное действие исключает его, арка не протекает в Giver-поле) — переиспользуют существующий хелпер `seed_audit_log_given_by_name` без изменений (его payload_json уже несёт литерал `given_to_name: "Кому Выдал"`).
- 1 сквозной регрессионный тест `suggest_person_receiver_survives_holder_name_overwrite_by_later_transition`, доказывающий буквальный симптом UAT4-01 через реальный `CartridgeService.transition()`: `ToRefill` (given_to_name="Смирнов С.С.") → `FromRefill` → `Install` (given_to_name="Иной Получатель", перезаписывает `holder_name`) → `suggest_person(Receiver, "Смирнов", 20)` всё ещё находит «Смирнов С.С.».
- Полный прогон пакета `trackly-app` (`TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app -- --skip login_remember_persistent_cookie`) — 107 test-групп, 0 провалов, паник нет; известный посторонний флейк `users_crud.rs` в этом прогоне не сработал.

## Task Commits

1. **Task 1: given_to_name_arm в suggest_person + doc-фикс + прямые SQL-тесты** — `a33a6cba` (feat)
2. **Task 2: Регрессия на реальный симптом + полный прогон пакета** — `67d4bbb7` (test)

**Plan metadata:** (этот коммит, после self-check)

## Files Created/Modified

- `crates/trackly-app/src/services/act_service.rs` — `given_to_name_arm` (match по `SuggestPersonField`, идентичен `given_by_name_arm` с заменой JSON-ключа и enum-ветки), второй именованный placeholder `{given_to_name_arm}` в финальном `format!()`, исправленный doc-комментарий (4 арки вместо 3, явное объяснение ограничения `holder_name`).
- `crates/trackly-app/tests/acts_suggest.rs` — 4 прямых SQL-теста (Test 15-18, зеркало Test 11-14), 1 сквозной регрессионный тест через `CartridgeService` (Test 19), новый хелпер `make_acts_and_cartridge_services()` (ActService + CartridgeService на общем writer/readers) и `seed_cartridge_model_via_service()`.

## Decisions Made

- `action IN ('custom:install', 'custom:to_refill')` не расширен — GAP-12-12 auto-return (`custom:return_to_stock`) тоже пишет `given_to_name` в свой payload, но это сознательно вне области, симметрично уже принятому решению для `given_by_name_arm` (Фаза 12).
- Регрессионный тест на симптом UAT4-01 сделан через реальный `CartridgeService.transition()` (ToRefill → FromRefill → Install), а не ручным посевом `audit_log` сырым SQL — методическое требование плана, продиктованное провалом раунда 1 этой фазы на похожем посеве.

## Deviations from Plan

None — план выполнен как написан. Все acceptance criteria и must_haves покрыты буквально.

## Issues Encountered

Первый прогон полного пакета tests был случайно урезан командой `| tail -150` в фоновом вызове (общий вывод обрезался, но код завершения пайплайна не отражал реальный exit code `cargo test`). Перезапущен без урезания вывода напрямую в лог-файл — 107 групп тестов, 0 провалов, exit code 0 подтверждён явно. Не является дефектом кода, чисто артефакт инструментария прогона; ничего исправлять в коде не требовалось.

## Verification Evidence

- `cargo test -p trackly-app --test acts_suggest` — 19 passed, 0 failed (14 прежних + 4 новых Test 15-18 + 1 регрессия Test 19).
- `cargo test -p trackly-app --test acts_suggest suggest_person_receiver_survives_holder_name_overwrite_by_later_transition -- --exact` — зелёный отдельно.
- Полный прогон: `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app -- --skip login_remember_persistent_cookie` — 107 test result: ok групп, 0 test result: FAILED, 0 panicked at, exit code 0 (запущен напрямую в лог-файл после обнаружения урезанного первого прогона).
- `cargo clippy -p trackly-app --all-targets -- -D warnings` и `cargo clippy --workspace --all-targets -- -D warnings` — чисто.
- `cargo fmt --all --check` — чисто.
- `node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256` — 0 нарушений (проверено при каждом коммите через pre-commit hook).
- `grep -n "given_to_name_arm" crates/trackly-app/src/services/act_service.rs` — 3 вхождения (объявление + 2 места использования в format!()).
- `grep -n "уже покрыт" crates/trackly-app/src/services/act_service.rs` — пусто (ложное утверждение о полном покрытии удалено).

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Backend-часть UAT4-01 закрыта: `suggest_person(Receiver, ...)` теперь видит полную историю `given_to_name` из `audit_log`, симметрично уже работавшей ветке Giver.
- Готово к потреблению фронтендом без изменений на стороне UI — `PersonAutocomplete.svelte` уже вызывает `acts.suggestPerson` без изменения сигнатуры, новая арка активна прозрачно.
- План 40-35 (`OperationModal.svelte`, подключение `cartridgesToRefillLastSend()` из 40-33) остаётся следующим в очереди, независим от этого плана.

---
*Phase: 40-movement-history*
*Completed: 2026-09-04*

## Self-Check: PASSED

Все изменённые файлы (`act_service.rs`, `acts_suggest.rs`) и `40-34-SUMMARY.md` подтверждены на
диске. Оба коммита задач (`a33a6cba`, `67d4bbb7`) подтверждены в `git log --oneline --all`.
