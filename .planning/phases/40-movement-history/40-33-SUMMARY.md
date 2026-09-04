---
phase: 40-movement-history
plan: 33
subsystem: database
tags: [rusqlite, sqlite, audit_log, cartridges, tauri, axum, specta]

# Dependency graph
requires:
  - phase: 40-movement-history
    provides: "40-30 (HST-01, UAT3-01): operation_default_place dispatcher + place_before_last_to_refill resolver"
provides:
  - "latest_to_refill_send — single audit_log-based resolver answering 'место/кто выдал/кому выдал для отправки на заправку' by recency, replacing the frequency-based most_common_to_refill_destination"
  - "Read-only endpoint cartridges_to_refill_last_send on both transports (Tauri invoke + axum POST) — feeds all three fields of the 'Отправка на заправку' dialog from one record"
  - "operation_default_place('from_refill', …) two-step fallback: own cartridge history -> global latest-send source place -> None (UAT4-03)"
affects: [40-movement-history, cartridge-ui-consumers]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "audit_log as the source of truth for a read-side aggregate when the write-side payload (given_by_name/given_to_name) has no dedicated column — single row read via ORDER BY created_at_utc DESC, id DESC LIMIT 1, no join, both source and destination place_id extracted from the SAME row's before_json/payload_json"
    - "Recency-over-frequency resolver: user decision to replace a GROUP BY COUNT(*) aggregate with ORDER BY created_at_utc DESC LIMIT 1 — deterministic without a tie-break needed (id DESC as secondary sort covers same-second inserts)"

key-files:
  created: []
  modified:
    - crates/trackly-infra/src/repos/cartridges_sqlite.rs
    - crates/trackly-app/src/dto/cartridge.rs
    - crates/trackly-app/src/services/cartridge_service.rs
    - crates/trackly-app/src/tauri_cmds/cartridges.rs
    - crates/trackly-app/src/http/cartridges.rs
    - crates/trackly-app/src/specta_export.rs
    - crates/trackly-app/tests/role_endpoint_matrix.rs
    - crates/trackly-app/tests/cartridges_lifecycle.rs
    - .planning/phases/40-movement-history/deferred-items.md

key-decisions:
  - "most_common_to_refill_destination (frequency rule, Plan 40-30) deleted entirely, together with its 4 tests — no 'fallback option', single owner of the question is now latest_to_refill_send (user decision 2026-09-04, 40-HUMAN-UAT.md UAT4-02)"
  - "latest_to_refill_send reads audit_log, not place_movements — given_by_name/given_to_name exist only in audit_log.payload_json; before_json.place_id (source) and payload_json.place_id (destination) of the SAME row avoid a fragile join-by-timestamp with place_movements"
  - "operation_default_place('to_refill', …) removed entirely (falls into the generic Validation catch-all) — the question it answered is now served by to_refill_last_send(), a purpose-built DTO endpoint, not a bare Option<i64>"
  - "from_refill fallback chain is a short-circuit, not a time comparison: step 2 (latest_to_refill_send) is only invoked when step 1 (place_before_last_to_refill) returns None — proven by a dedicated test where the global record is objectively more recent yet loses to the cartridge's own (older) history"

requirements-completed: [HST-01]

# Metrics
duration: ~50min (includes two full ~20min full-package verification runs)
completed: 2026-09-04
---

# Phase 40 Plan 33: Одна запись — три поля диалога «Отправка на заправку» (UAT4-02/UAT4-03) Summary

**Новый резолвер `latest_to_refill_send` читает единственную строку `audit_log` (самая свежая отправка на заправку ЛЮБОГО картриджа) и отдаёт «кто выдал»/«кому выдал»/«место» одним запросом на обоих транспортах; частотное правило `most_common_to_refill_destination` удалено без остатка, `from_refill` получил двухступенчатую цепочку fallback.**

## Performance

- **Duration:** ~50 min (основное время — два полных прогона пакета `trackly-app` по ~20 мин каждый, наблюдение за фоновым `cargo test`)
- **Completed:** 2026-09-04T12:07:00+07:00
- **Tasks:** 3 (все выполнены)
- **Files modified:** 8 (+ deferred-items.md)

## Accomplishments

- `LatestToRefillSend { given_by_name, given_to_name, from_place_id, to_place_id }` + `pub fn latest_to_refill_send(&self, conn: &Connection) -> Result<Option<LatestToRefillSend>, AppError>` — один запрос к `audit_log` (`action = 'custom:to_refill'`, `ORDER BY created_at_utc DESC, id DESC LIMIT 1`), без join с `place_movements`: `before_json.place_id` (источник) и `payload_json.place_id` (назначение) читаются из ОДНОЙ строки той же транзакции.
- `most_common_to_refill_destination` и её 4 теста удалены целиком; doc-комментарии над `TO_REFILL_MOVEMENT_NOTE` и соседними хелперами обновлены, чтобы больше не ссылаться на удалённую функцию.
- 5 новых тестов в `cartridges_sqlite.rs` доказывают: пустая история → `None`; «свежее побеждает частое» (не «самое частое»); имена читаются из `payload_json`; фильтр по `action = 'custom:to_refill'`; архивное место назначения обнуляется, но имена остаются.
- `ToRefillLastSendDto` (dto/cartridge.rs) + `CartridgeService::to_refill_last_send()` — все три поля диалога «Отправка на заправку» из одной записи, не три независимых агрегата; пустая история → все три поля `None`.
- `operation_default_place`: ветка `"to_refill"` удалена целиком (падает в `AppError::Validation`, как любой неизвестный `op`); ветка `"from_refill"` теперь двухступенчатая — `place_before_last_to_refill` (собственная история картриджа) → при `None` → `latest_to_refill_send().from_place_id` (место-источник самой свежей отправки ЛЮБОГО картриджа) → иначе `None`. Один `reader.acquire()` на обе попытки внутри одного `spawn_blocking`.
- Новый эндпоинт `cartridges_to_refill_last_send` на обоих транспортах (Tauri-команда + `POST /api/v1/cartridges_to_refill_last_send`, без тела запроса), зарегистрирован в `specta_export.rs`, `ui/src/bindings.ts` содержит `cartridgesToRefillLastSend` после `export_bindings`.
- Role-матрица: `operation_default_place_payload` теперь `op: "from_refill"` (не `"to_refill"` — эндпоинт больше его не обслуживает); новые Case 62/63 (Employee 403 / Manager not-403 для `cartridges_to_refill_last_send`), зеркало Case 60/61.
- `cartridges_lifecycle.rs`: старый тест `operation_default_place_to_refill_resolves_via_real_service_flow` репурпозирован в `operation_default_place_to_refill_now_returns_validation_error` (регрессионная защита удаления ветки); добавлены 5 новых интеграционных тестов через реальный поток `CartridgeService` (без ручного посева `audit_log`/`place_movements` сырым SQL) — правило «от последней отправки» против «самое частое», имена, и оба сценария fallback-цепочки `from_refill` (глобальный fallback при отсутствии своей истории; собственная история побеждает даже более свежую глобальную запись — короткое замыкание, не сравнение времени).

## Task Commits

1. **Task 1: latest_to_refill_send резолвер + удаление most_common_to_refill_destination** — `6838b689` (feat)
2. **Task 2: Сервисный слой (to_refill_last_send + from_refill fallback) + оба транспорта + specta** — `b5553347` (feat)
3. **Task 3: Role-матрица + интеграционные тесты через реальный поток CartridgeService** — `d75b587d` (test)

**Plan metadata:** (этот коммит, после self-check)

## Files Created/Modified

- `crates/trackly-infra/src/repos/cartridges_sqlite.rs` — `LatestToRefillSend` struct; `latest_to_refill_send`; `most_common_to_refill_destination` и её 4 теста удалены; 5 новых inline-тестов.
- `crates/trackly-app/src/dto/cartridge.rs` — `ToRefillLastSendDto`.
- `crates/trackly-app/src/services/cartridge_service.rs` — `to_refill_last_send()`; `operation_default_place` переписан (ветка `"to_refill"` удалена, `"from_refill"` — двухступенчатая цепочка).
- `crates/trackly-app/src/tauri_cmds/cartridges.rs` — `build_cartridges_to_refill_last_send` + `#[tauri::command] cartridges_to_refill_last_send`.
- `crates/trackly-app/src/http/cartridges.rs` — `handler_to_refill_last_send`, маршрут `/api/v1/cartridges_to_refill_last_send`.
- `crates/trackly-app/src/specta_export.rs` — регистрация новой Tauri-команды.
- `crates/trackly-app/tests/role_endpoint_matrix.rs` — payload `op: "from_refill"`; Case 62/63.
- `crates/trackly-app/tests/cartridges_lifecycle.rs` — репурпозированный regression-тест + 5 новых интеграционных тестов.
- `.planning/phases/40-movement-history/deferred-items.md` — новый файл, задокументирован не относящийся к плану флейк `users_crud.rs`.

## Decisions Made

- Признание правила пользователя от 2026-09-04 (`40-HUMAN-UAT.md` UAT4-02): «от предыдущей отправки», а не «самое частое» — распространено и на место, и на оба имени, все три поля из ОДНОЙ записи.
- `most_common_to_refill_destination` не оставлена «на всякий случай» — удалена без остатка, единственный резолвер вопроса подтверждён grep'ом (acceptance criterion плана).
- Источник для нового резолвера — `audit_log`, а не `place_movements`: имена физически существуют только в `payload_json`, а оба `place_id` (источник и назначение) одной транзакции ToRefill уже лежат в одной строке — join по временной метке не нужен и был бы хрупким при двух отправках в одну секунду.
- `operation_default_place("to_refill", …)` не оставлена как алиас нового эндпоинта — падает в `AppError::Validation`, явный сигнал вместо молчаливо устаревшего поведения (T-40-33-03 threat register).

## Deviations from Plan

None — план выполнен как написан. Все acceptance criteria и must_haves покрыты буквально.

## Issues Encountered

- **Пре-существующий флейк, вне области плана:** `users_crud.rs::users_update_password_change` (и один раз также `delete_then_recreate_revives_same_login`) превысили тестовый бюджет 30с при полном последовательном прогоне пакета `trackly-app` (argon2id-хеширование под нагрузкой от 104 тестовых бинарников подряд). В изоляции оба теста проходят стабильно (~16с суммарно на два). Файл `users_crud.rs` не входит в область этого плана — задокументировано в `deferred-items.md`, не исправлялось (scope boundary).

## Verification Evidence

- `cargo test -p trackly-infra --lib` — 143 passed, 0 failed (включая 5 новых тестов `latest_to_refill_send_*`).
- `cargo test -p trackly-app --test export_bindings` — зелено; `grep -n cartridgesToRefillLastSend ui/src/bindings.ts` — присутствует.
- `cargo test -p trackly-app --test role_endpoint_matrix` — зелено, включая Case 62/63.
- `cargo test -p trackly-app --test cartridges_lifecycle -- operation_default_place to_refill_last_send` — 9 passed, 0 failed.
- Полный прогон пакета: `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app -- --skip login_remember_persistent_cookie` — запущен ДВАЖДЫ (в фоне, один `cargo test` за раз, до полного завершения); единственные провалы в обоих прогонах — `users_crud.rs` тайминг-флейк (не в области плана, см. «Issues Encountered» и `deferred-items.md`); все файлы, изменённые этим планом (`cartridges_*`, `role_endpoint_matrix`, `cartridges_lifecycle`), зелены в обоих прогонах.
- `cargo clippy --workspace --all-targets -- -D warnings` — чисто.
- `cargo fmt --all --check` — чисто.
- `node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256` — 0 нарушений (при каждом коммите).

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Backend-часть UAT4-02/UAT4-03 закрыта: `cartridges_to_refill_last_send` отдаёт все три поля диалога «Отправка на заправку» одним запросом на обоих транспортах; `operation_default_place("from_refill", …)` реализует полную цепочку fallback из UAT4-03.
- Готово к потреблению фронтендом — план 40-35 (`OperationModal.svelte`) может подключить `cartridgesToRefillLastSend()` к диалогу «Отправка на заправку» вместо/в дополнение к существующему `cartridges_operation_default_place("from_refill", …)`.
- UAT4-01 (автокомплит имён из `audit_log.payload_json`) — вне области этого плана, отдельный gap.

---
*Phase: 40-movement-history*
*Completed: 2026-09-04*

## Self-Check: PASSED

All modified source files and this SUMMARY.md confirmed present on disk; all 3 task commits
(`6838b689`, `b5553347`, `d75b587d`) confirmed present in `git log --oneline --all`.
