---
phase: 40-movement-history
plan: 30
subsystem: database
tags: [rusqlite, sqlite, place_movements, cartridges, tauri, axum, specta]

# Dependency graph
requires:
  - phase: 40-movement-history
    provides: "40-28 (CR-02): three-step fallback chain in last_known_storage_place_in_tx"
provides:
  - "Read-only endpoint cartridges_operation_default_place on both transports (Tauri invoke + axum POST) — server-computed place defaults for OperationModal's to_refill/from_refill dialogs"
  - "most_common_to_refill_destination — deterministic global most-frequent-refill-destination aggregate, tie-break on smaller to_place_id"
  - "last_known_storage_place_in_tx widened from &Transaction to &Connection (still pub, still callable from the sole in-tx call site via deref coercion) — reusable from a plain reader-pool connection"
  - "TO_REFILL_MOVEMENT_NOTE — single source of the write-side note literal, also read by the new aggregate"
affects: [40-movement-history, cartridge-ui-consumers]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "&Transaction -> &Connection widening via deref coercion to let a tx-scoped read-only helper serve both write-transaction call sites and plain reader-pool call sites without a second resolver"
    - "Shared literal-note const as the single source of truth linking a write-side tag and a read-side aggregate query, preventing write/read note-string drift"

key-files:
  created: []
  modified:
    - crates/trackly-infra/src/repos/cartridges_sqlite.rs
    - crates/trackly-app/src/services/cartridge_service.rs
    - crates/trackly-app/src/tauri_cmds/cartridges.rs
    - crates/trackly-app/src/http/cartridges.rs
    - crates/trackly-app/src/specta_export.rs
    - crates/trackly-app/tests/role_endpoint_matrix.rs
    - crates/trackly-app/tests/cartridges_lifecycle.rs

key-decisions:
  - "from_refill default reuses last_known_storage_place_in_tx (CR-02, Plan 40-28) verbatim, applied to the cartridge's own id — no second parallel resolver, per plan's explicit anti-goal"
  - "to_refill default: single global aggregate across ALL history, no time window, no per-user/model split (product decision recorded in 40-HUMAN-UAT.md); deterministic tie-break on smaller to_place_id"
  - "Doc-comment over from_refill states the ACTUAL behavior — 'most recent movement touching a storage place wins, usually ToRefill but overwritable by an unguarded manual place edit during refill' — not an idealized invariant. Pinned by a regression test, not just prose (checker finding from planning round 3, commit 7883f235, applied here as the implementation)."
  - "last_known_storage_place_in_tx widened to &Connection rather than duplicated — Transaction: Deref<Target=Connection> means the sole existing call site inside transition_in_tx keeps compiling unchanged"

requirements-completed: [HST-01]

# Metrics
duration: ~40min
completed: 2026-09-03
---

# Phase 40 Plan 30: Серверные дефолты места для диалогов заправки картриджа Summary

**Новый read-only эндпоинт `cartridges_operation_default_place` (Tauri + axum) вычисляет дефолт места для «Отправка на заправку» (самое частое историческое место назначения, детерминированный tie-break) и «Получение с заправки» (переиспользование резолвера `last_known_storage_place_in_tx` из плана 40-28/CR-02, без второго параллельного источника правды).**

## Performance

- **Duration:** ~40 min
- **Completed:** 2026-09-03T22:44:52+07:00
- **Tasks:** 3 (все выполнены)
- **Files modified:** 7

## Accomplishments

- `last_known_storage_place_in_tx` обобщена с `tx: &Transaction<'_>` на `conn: &Connection` и стала `pub` — единственный существующий вызывающий (`transition_in_tx`) компилируется без изменений благодаря `Transaction: Deref<Target = Connection>`.
- Новая функция `most_common_to_refill_destination(&self, conn: &Connection) -> Result<Option<i64>, AppError>` — самое частое место назначения ВСЕХ движений с меткой `TO_REFILL_MOVEMENT_NOTE`, по всей истории, детерминированный tie-break (`ORDER BY COUNT(*) DESC, to_place_id ASC`), архивные/удалённые места исключены.
- Общая константа `TO_REFILL_MOVEMENT_NOTE` устраняет дублирование литерала между write-путём (`transition_in_tx`) и read-путём (новая агрегатная функция).
- `CartridgeService::operation_default_place(op, cartridge_id)` — диспетчер на `"to_refill"` (агрегат, `cartridge_id` игнорируется) / `"from_refill"` (резолвер CR-02, `cartridge_id` обязателен) / любой другой `op` → `AppError::Validation`.
- Оба транспорта (`cartridges_operation_default_place` Tauri-команда, `POST /api/v1/cartridges_operation_default_place`) зарегистрированы в `specta_export.rs`, `ui/src/bindings.ts` содержит запись после `export_bindings`.
- Doc-комментарий над `from_refill`-веткой описывает ФАКТИЧЕСКОЕ поведение (последнее движение, затрагивающее складское место — обычно ToRefill, но перебиваемое более поздним ручным редактированием места через `CartridgeService::update`, у которого нет гейта по статусу), закреплённое регрессионным тестом, а не только прозой.
- Role-матрица (Case 60/61) и 2 интеграционных теста через реальный поток `CartridgeService` (без ручного посева `place_movements`) покрывают Employee-403 / Manager-200 и оба сценария дефолта места.

## Task Commits

1. **Task 1: Обобщить last_known_storage_place_in_tx + новая агрегатная функция + общая константа** — `36953384` (feat)
2. **Task 2: Сервисный метод + оба транспорта (Tauri + HTTP) + регистрация specta** — `2210d206` (feat)
3. **Task 3: Role-матрица + интеграционный тест через реальный поток CartridgeService** — `8c01b4d4` (test)

**Plan metadata:** (этот коммит, после self-check)

## Files Created/Modified

- `crates/trackly-infra/src/repos/cartridges_sqlite.rs` — `TO_REFILL_MOVEMENT_NOTE` const; `last_known_storage_place_in_tx` обобщена и публична; новая `most_common_to_refill_destination`; 6 новых inline-тестов.
- `crates/trackly-app/src/services/cartridge_service.rs` — `CartridgeService::operation_default_place(op, cartridge_id)`.
- `crates/trackly-app/src/tauri_cmds/cartridges.rs` — `build_cartridges_operation_default_place` + `#[tauri::command] cartridges_operation_default_place` (i32↔i64 boundary conversion).
- `crates/trackly-app/src/http/cartridges.rs` — `OperationDefaultPlacePayload`, `handler_operation_default_place`, маршрут `/api/v1/cartridges_operation_default_place`.
- `crates/trackly-app/src/specta_export.rs` — регистрация новой Tauri-команды.
- `crates/trackly-app/tests/role_endpoint_matrix.rs` — Case 60/61 (Employee 403 / Manager not-403), зеркало Case 13/14.
- `crates/trackly-app/tests/cartridges_lifecycle.rs` — регрессионный тест `operation_default_place_from_refill_reflects_manual_edit_during_refill` + 2 интеграционных теста через реальный поток сервиса.

## Decisions Made

- `from_refill` — переиспользование `last_known_storage_place_in_tx`, никакого второго резолвера (буквальное требование `must_haves.key_links` плана).
- `to_refill` — вся история, без окна времени, без разбивки (зафиксировано в `40-HUMAN-UAT.md`), tie-break на меньший `to_place_id` для детерминизма тестов.
- Гейт авторизации — `Action::ReadData`, тот же круг пользователей, что и у `storage_place_ids`/`get_history` (read-side поддержка того же диалога, что и мутация `transition`).
- Doc-комментарий над `from_refill` намеренно НЕ формулирует ложный инвариант "ничто не двигает место во время заправки" — фактическое поведение (последнее движение побеждает) закреплено и прозой, и тестом.

## Deviations from Plan

None — план выполнен как написан. Все acceptance criteria и must_haves покрыты буквально; план уже содержал исправленную (после checker'а раунда 3, коммит `7883f235`) формулировку doc-комментария и регрессионного теста, реализация им точно следует.

## Issues Encountered

None.

## Verification Evidence

- `cargo test -p trackly-infra --lib` — 139 passed, 0 failed (включая 6 новых тестов `cartridges_sqlite::tests`).
- `cargo test -p trackly-app --test export_bindings` — зелено; `grep -n cartridges_operation_default_place ui/src/bindings.ts` — присутствует.
- `cargo test -p trackly-app --test cartridges_lifecycle operation_default_place_from_refill_reflects_manual_edit_during_refill -- --exact` — зелено.
- `cargo test -p trackly-app --test cartridges_lifecycle -- operation_default_place` — 3 passed (регрессионный + 2 интеграционных через реальный сервис).
- `cargo test -p trackly-app --test role_endpoint_matrix` — зелено, включая Case 60/61.
- Полный прогон пакета: `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app -- --skip login_remember_persistent_cookie` — **866 passed, 0 failed**, без регрессий (запущено в фоне под наблюдением, один `cargo test` за раз, до полного завершения).
- `cargo clippy --workspace --all-targets -- -D warnings` — чисто.
- `cargo fmt --all --check` — чисто.
- `node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256` — 0 нарушений (x3, при каждом коммите).

Примечание про "красный до фикса": резолвер `last_known_storage_place_in_tx` не менялся в этом плане (только расширена сигнатура и видимость) — его корректность уже была закреплена регрессионными тестами плана 40-28 (CR-02). Регрессионный тест Task 2 (`operation_default_place_from_refill_reflects_manual_edit_during_refill`) фиксирует композицию нового сервисного метода с уже верным резолвером — он был зелёным сразу после написания реализации Task 2, что ожидаемо (это не re-discovery старого бага, а закрепление нового публичного контракта, использующего уже проверенную логику).

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Backend-часть UAT3-01 закрыта: оба дефолта места (`to_refill`/`from_refill`) доступны на обоих транспортах, авторизованы, покрыты тестами.
- Готово к потреблению фронтендом — план 40-31 (`OperationModal.svelte`) может вызывать `cartridges_operation_default_place` для подстановки дефолта в поле «Место» при открытии диалогов «Отправка на заправку»/«Получение с заправки».
- Известное ограниченное (не блокирующее) поведение `from_refill`, зафиксированное doc-комментарием и тестом: ручное редактирование места картриджа во время «На заправке» (доступно через контекстное меню в любом статусе) перебивает дефолт — подставится последнее место, но не обязательно "место до заправки". Оператор может исправить вручную; вне объёма этого плана дальнейшее ужесточение.

---
*Phase: 40-movement-history*
*Completed: 2026-09-03*

## Self-Check: PASSED

All 7 modified source files and this SUMMARY.md confirmed present on disk; all 3 task commits
(`36953384`, `2210d206`, `8c01b4d4`) confirmed present in `git log --oneline --all`.
