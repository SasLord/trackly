---
phase: 40-movement-history
plan: 29
subsystem: database
tags: [rusqlite, sqlite, reader-pool, deadlock, place_movements, reports, regression-testing]

# Dependency graph
requires:
  - phase: 40-movement-history
    provides: "40-28 (CR-03, CR-02 closed); 40-VERIFICATION.md gap-closure contract (gaps 3 and 4: CR-01, WR-10)"
provides:
  - "compute_place_path_short_with_conn(&Connection, ...) — no-acquire sibling of compute_place_path_short, used by both get_timeline and query_movements_inner"
  - "query_movements_inner takes only &Connection — no ReaderPool parameter, no nested acquire inside its per-row loop"
  - "act_number_display::resolve_movement_act_number(&Connection, Option<act_id>) — single owner of the movement act display-number formula, shared by timeline and report"
  - "ReaderPool::acquire_timeout(Duration) — bounded acquire, defense-in-depth, not migrated onto by any existing call site"
  - "test_writer_and_readers_sized(pool_size) — reusable fixture for pool-size-bounded regression tests"
affects: [40-movement-history, reports, reader-pool-consumers]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "&Connection sibling function alongside a &ReaderPool-taking wrapper — for callers that already hold a connection and must not nest a second acquire() inside a per-row loop (CR-01 root-cause fix shape)"
    - "Deadlock regression tests against ReaderPool-backed async code must run their fixture + call on a dedicated std::thread + own tokio::runtime::Runtime, bounded via mpsc::recv_timeout on the test thread — NOT #[tokio::test] + tokio::time::timeout, which does not protect the test run: Tokio's Runtime::drop blocks forever waiting for a leaked spawn_blocking task even after the timeout future itself returns"

key-files:
  created:
    - crates/trackly-app/src/services/act_number_display.rs
  modified:
    - crates/trackly-app/src/services/place_path_display.rs
    - crates/trackly-app/src/services/place_movement_service.rs
    - crates/trackly-app/src/services/report_service.rs
    - crates/trackly-app/src/services/mod.rs
    - crates/trackly-infra/src/db/pools.rs
    - crates/trackly-infra/src/test_support/test_app_ctx.rs
    - crates/trackly-infra/src/test_support/mod.rs
    - crates/trackly-app/tests/place_movements_timeline.rs
    - crates/trackly-app/tests/report_movements.rs

key-decisions:
  - "compute_place_path_short(readers: &ReaderPool, ...) becomes a thin acquire()+delegate wrapper around the new compute_place_path_short_with_conn(&Connection, ...) — public signature and behavior unchanged, existing act-service call sites untouched"
  - "query_movements_inner drops its &ReaderPool parameter entirely (both call sites — list_movements and get_report_counts's movements branch — only ever needed it for the now-removed nested acquire)"
  - "ReaderPool::acquire_timeout added purely as defense-in-depth — no existing call site migrated onto it, since Task 1 already removes the only known nested-acquire call sites in the codebase (documented explicitly in the threat register, T-40-29-01)"
  - "CR-01 deadlock regression test deliberately does NOT use the plan's literally-specified #[tokio::test] + tokio::time::timeout pattern — verified empirically (by temporarily restoring the pre-fix nested-acquire code) that this combination hangs the ENTIRE cargo test process on regression, not just the one test: Tokio's Runtime::drop for the #[tokio::test]-generated runtime blocks waiting for the leaked spawn_blocking task, even though the inner tokio::time::timeout future itself correctly returns Err after 5s. Redesigned as a plain #[test] running the fixture + get_timeline call on its own std::thread with its own Runtime, bounded via mpsc::recv_timeout on the main test thread — a regression now fails fast and cleanly (5.00s, clear panic message) instead of hanging the whole run, satisfying the project's own regression-test-must-not-hang-the-run constraint more literally than the plan's suggested pattern would have"

requirements-completed: [HST-02, HST-03, HST-04]

# Metrics
duration: ~70min (across a session-limit resume)
completed: 2026-09-03
---

# Phase 40 Plan 29: Устранение риска дедлока reader-пула + единый номер акта в отчёте «Перемещения» Summary

**Убран вложенный `ReaderPool::acquire()` из `get_timeline` и `query_movements_inner` (CR-01), добавлен bounded `acquire_timeout` как defense-in-depth, и оба поверхностных места (таймлайн + отчёт «Перемещения») теперь резолвят номер акта через один общий `resolve_movement_act_number` (WR-10, «20в» вместо голого «20»).**

## Performance

- **Duration:** ~70 min (сессия была прервана лимитом и продолжена координатором с того же состояния — Task 1 уже был выполнен, но не закоммичен на момент возобновления)
- **Completed:** 2026-09-03T06:26:40Z (по времени завершения полного прогона пакета)
- **Tasks:** 3 (все выполнены)
- **Files modified:** 9 (+ 1 создан)

## Accomplishments

- **CR-01 закрыт (major):** ни `PlaceMovementService::get_timeline`, ни `report_service.rs::query_movements_inner` больше не берут ВТОРОЕ соединение из `ReaderPool` изнутри цикла по строкам — оба держат ровно одно соединение на весь запрос. Новая функция `compute_place_path_short_with_conn(&Connection, ...)` — сиблинг без `acquire()`, используемый обоими читателями; старая `compute_place_path_short(&ReaderPool, ...)` осталась как тонкая обёртка (публичное поведение не изменилось).
- **Defense-in-depth:** `ReaderPool::acquire_timeout(Duration) -> Option<ReaderHandle>` — bounded-вариант untimed `acquire()`, покрыт двумя unit-тестами (`acquire_timeout_returns_none_when_pool_exhausted` доказывает, что метод реально ждёт ~90+ мс, а не мгновенно сдаётся; `acquire_timeout_succeeds_once_a_connection_is_returned` доказывает успешный путь через межпоточную передачу connection). Ни один существующий вызывающий код не переключён на него — сознательное решение, задокументированное в threat-register плана (T-40-29-01): Task 1 уже устраняет единственные известные вложенные `acquire()` в кодовой базе.
- **WR-10 закрыт (minor):** новый файл `act_number_display.rs` содержит `resolve_movement_act_number(&Connection, Option<act_id>) -> Option<String>` — единственный владелец формулы отображаемого номера акта для движения (D-Numbering-01), дословно перенесённый из инлайновой логики `get_timeline`. `report_service.rs::query_movements_inner` больше не селектит `a.number AS act_number` напрямую (и убран сопутствующий `LEFT JOIN acts a`) — вместо этого вызывает тот же `resolve_movement_act_number` на каждой строке. Отчёт «Перемещения» теперь показывает «20в» для возвратного акта вместо голого родительского «20», согласованно с таймлайном.
- Оба регрессионных теста подтверждены КРАСНЫМИ на предфиксовом коде (с точными сообщениями) и ЗЕЛЁНЫМИ после фикса — см. «Verification Evidence» ниже.
- Полный прогон `cargo test -p trackly-app -- --skip login_remember_persistent_cookie`: **863 passed, 0 failed, 2 ignored**. Полный прогон `cargo test -p trackly-core -p trackly-infra`: **309 passed, 0 failed, 2 ignored**. Ни одной регрессии.

## Task Commits

1. **Task 1: `&Connection`-сиблинг `compute_place_path_short` + устранение вложенного acquire в `get_timeline` и `query_movements_inner`** — `e87e37cd` (fix)
2. **Task 2: `acquire_timeout` defense-in-depth + regression-тест на пул из одного соединения (CR-01)** — `69c4e0f3` (feat)
3. **Task 3: Общий `resolve_movement_act_number` — таймлайн и отчёт «Перемещения» показывают один и тот же канонический номер (WR-10)** — `c78d0377` (fix)

**Plan metadata:** (этот коммит, после self-check)

## Files Created/Modified

- `crates/trackly-app/src/services/place_path_display.rs` — добавлена `compute_place_path_short_with_conn(&Connection, ...)`; `compute_place_path_short(&ReaderPool, ...)` теперь тонкая обёртка (`acquire()` + делегирование).
- `crates/trackly-app/src/services/place_movement_service.rs` — `get_timeline`: оба вызова path-shortening в цикле переведены на `compute_place_path_short_with_conn(&conn, ...)` (уже удержанное соединение, не второй `acquire()`); инлайновый блок резолва номера акта заменён вызовом `resolve_movement_act_number(&conn, row.act_id)`.
- `crates/trackly-app/src/services/report_service.rs` — `query_movements_inner`: сигнатура лишилась параметра `readers: &ReaderPool`; оба вызова path-shortening переведены на `conn`; SQL лишился `a.number AS act_number` и `LEFT JOIN acts a`; `query_map`/tuple переиндексированы; `movement_reason`'s `act_number` теперь `Option<&str>`, резолвится через `resolve_movement_act_number` на каждой строке. Оба call site (`list_movements`, `get_report_counts`) обновлены — убран лишний аргумент `&readers`.
- `crates/trackly-app/src/services/act_number_display.rs` (новый) — `resolve_movement_act_number(&Connection, Option<act_id>) -> Option<String>`, дословный перенос прежней инлайновой логики `get_timeline`.
- `crates/trackly-app/src/services/mod.rs` — `pub mod act_number_display;` (алфавитно, перед `act_service`).
- `crates/trackly-infra/src/db/pools.rs` — `ReaderPool::acquire_timeout(Duration) -> Option<ReaderHandle>`; два новых unit-теста в `mod tests`.
- `crates/trackly-infra/src/test_support/test_app_ctx.rs` — `test_writer_and_readers_sized(pool_size)`; `test_writer_and_readers()` теперь тонкая обёртка (`_sized(4)`).
- `crates/trackly-infra/src/test_support/mod.rs` — экспортирует `test_writer_and_readers_sized`.
- `crates/trackly-app/tests/place_movements_timeline.rs` — новый тест `get_timeline_does_not_deadlock_with_single_reader_slot` (пул размера 1, два движения одного устройства).
- `crates/trackly-app/tests/report_movements.rs` — новые хелперы `seed_act`/`seed_return_act`/`seed_movement_with_act`; новый тест `report_movements_return_act_shows_canonical_number`.

## Decisions Made

- `compute_place_path_short` и `query_movements_inner` оставлены с прежними публичными сигнатурами там, где это возможно (первая), либо сужены до минимально нужного (`&Connection` вместо `&ReaderPool` во второй) — никаких лишних параметров, никакой преждевременной миграции существующих вызывающих кодов на `acquire_timeout`.
- Регрессионный тест на дедлок (Task 2) переписан относительно буквального текста плана: вместо `#[tokio::test]` + `tokio::time::timeout` использован `#[test]` + `std::thread` + собственный `tokio::runtime::Runtime` + `mpsc::recv_timeout`. Причина и доказательство — в разделе «Issues Encountered» и «Deviations» ниже.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 — тест не соответствовал требованию «не вешать весь прогон»] Regression-тест CR-01 переписан с `tokio::test`+`tokio::time::timeout` на `std::thread`+`mpsc::recv_timeout`**
- **Found during:** Task 2, при написании `get_timeline_does_not_deadlock_with_single_reader_slot` по буквальному тексту плана (`#[tokio::test]` + `tokio::time::timeout(5s, ...)`, паттерн из `place_movements_write_sites_devices.rs`).
- **Issue:** Проектное правило прямо требует: «тест на дедлок reader-пула обязан иметь собственный таймаут/ограниченный пул, чтобы при регрессии он ПАДАЛ, а не вешал весь прогон навсегда». При проверке этого теста на предфиксовом коде (временный откат `place_movement_service.rs` на `HEAD~1`) выяснилось, что паттерн `#[tokio::test]` + `tokio::time::timeout` НЕ защищает прогон: внутренний `tokio::time::timeout` действительно возвращает `Err` через 5с, но `Drop` рантайма, который генерирует макрос `#[tokio::test]`, блокируется в ожидании завершения ВСЕХ blocking-pool задач — включая ту самую задачу `spawn_blocking`, что вечно ждёт на `Condvar::wait` внутри вложенного `acquire()`. В результате весь `cargo test` завис — пришлось убивать процессы вручную (PID теста, `cargo test`, родительский shell).
- **Fix:** Тест переписан на обычный синхронный `#[test]`, который поднимает СОБСТВЕННЫЙ `tokio::runtime::Runtime` на ОТДЕЛЬНОМ `std::thread` и передаёт результат через `mpsc::channel`; главный тестовый поток вызывает `recv_timeout(Duration::from_secs(5))`. При регрессии воркер-поток паркуется навечно (leaked, но не блокирует процесс), канал не получает сообщение за 5с — тест падает с понятным паникующим сообщением; `cargo test` НЕ виснет, так как `std::thread::spawn`-потоки не блокируют выход из процесса (в отличие от `Drop` Tokio-рантайма).
- **Files modified:** `crates/trackly-app/tests/place_movements_timeline.rs`.
- **Verification:** см. «Verification Evidence» ниже — красный (5.00s, явная паника) до фикса, зелёный (0.05s) после.
- **Committed in:** `69c4e0f3` (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (тестовая архитектура, не продакшн-код)
**Impact on plan:** Существенно для соблюдения буквы проектного правила «регрессия должна падать, а не вешать прогон» — план предполагал паттерн, который на практике этому правилу не удовлетворяет из-за особенности `Drop` для Tokio-рантайма при утечке blocking-задачи. Итоговый тест строже соответствует требованию, чем буквальный текст плана. Продакшн-код (Task 1, Task 3) выполнен точно по плану, без отклонений.

## Issues Encountered

При написании regression-теста Task 2 по буквальному тексту плана (`#[tokio::test]` + `tokio::time::timeout`) обнаружилось, что демонстрация "красного" состояния (временный откат `place_movement_service.rs` на `HEAD~1` для восстановления вложенного `acquire()`) приводила не к ожидаемому таймауту через 5с, а к зависанию всего процесса `cargo test` — фоновая команда была принудительно перемещена харнессом в background после 120с и её пришлось убивать вручную (`kill -9`) по PID теста, самого `cargo test` и родительского shell. Причина и решение — см. «Deviations» выше. После редизайна теста та же процедура (временный откат кода → запуск → восстановление) прошла штатно: тест упал за ровно 5.00с с понятным сообщением, `cargo test` завершился нормально.

Отдельно: во время работы над Task 3 однократно был случайно запущен ВТОРОЙ параллельный `cargo test` (для `trackly-core`/`trackly-infra`) поверх ещё не завершившегося полного прогона `trackly-app` — нарушение проектного правила «один `cargo test` за раз». Обнаружено сразу по низкому CPU-времени обоих процессов (оба ждали на `target/`-локе); лишний процесс убит (`kill -9`), первый прогон доведён до конца без дальнейших коллизий.

## Verification Evidence (красный-до / зелёный-после)

### CR-01: `get_timeline_does_not_deadlock_with_single_reader_slot`

**До фикса** (временный откат `place_movement_service.rs` на предфиксовую версию — восстановлен вложенный `readers.acquire()` внутри цикла `get_timeline`, пул размера 1):
```
thread 'get_timeline_does_not_deadlock_with_single_reader_slot' (30654902) panicked at crates/trackly-app/tests/place_movements_timeline.rs:585:19:
get_timeline exceeded 5 s budget — nested reader-pool acquire regressed (CR-01)

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 5 filtered out; finished in 5.00s
```
(Первая версия теста, буквально по плану — `#[tokio::test]` + `tokio::time::timeout` — на этом же предфиксовом коде не завершилась вообще: `cargo test` завис и был убит вручную; см. «Issues Encountered».)

**После фикса** (код Task 1 восстановлен):
```
test get_timeline_does_not_deadlock_with_single_reader_slot ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 0.05s
```
Полный файл (6 тестов): `test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`.

### WR-10: `report_movements_return_act_shows_canonical_number`

**До фикса** (временный откат `report_service.rs` на `HEAD` — то есть на состояние ДО Task 3, с сырым `a.number AS act_number` без `resolve_movement_act_number`):
```
thread 'report_movements_return_act_shows_canonical_number' (30696436) panicked at crates/trackly-app/tests/report_movements.rs:889:5:
WR-10: report must show the canonical return number "20в", got reason: "актом №20"

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 9 filtered out; finished in 0.05s
```

**После фикса:**
```
test report_movements_return_act_shows_canonical_number ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 9 filtered out; finished in 0.05s
```
Полный файл (10 тестов): `test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`.

## Full Verification Run

- `cargo build -p trackly-app -p trackly-infra` — чисто, без warnings в затронутых файлах.
- `cargo test -p trackly-infra --lib pools::` — 7/7 ok (5 существующих + 2 новых: `acquire_timeout_returns_none_when_pool_exhausted`, `acquire_timeout_succeeds_once_a_connection_is_returned`).
- `cargo test -p trackly-app --test place_movements_timeline` — 6/6 ok (5 существующих + 1 новый).
- `cargo test -p trackly-app --test report_movements` — 10/10 ok (9 существующих + 1 новый).
- `cargo clippy -p trackly-app -p trackly-infra -- -D warnings` — чисто.
- `cargo fmt --all --check` — чисто.
- Полный прогон `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app -- --skip login_remember_persistent_cookie` — **863 passed, 0 failed, 2 ignored** (107 test-result блоков по всем интеграционным файлам пакета).
- Полный прогон `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-core -p trackly-infra` — **309 passed, 0 failed, 2 ignored** (2 ignored — doc-тесты `ad`/`snmp`, pre-existing).
- `node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256` — 0 нарушений (при каждом из трёх коммитов).

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- `40-VERIFICATION.md`'s `gaps:` block теперь содержит 4 из 4 закрытых гэпов раунда 2 (CR-03, CR-02 — план 40-28; CR-01, WR-10 — этот план). Фаза 40 гап-клозур готова к финальной проверке/закрытию.
- Оставшийся отложенный технический долг из 40-28 (COALESCE-can't-clear баг у остальных nullable-полей `DevicePatch`, кроме `place_id`; UI-уровень `DeviceFormBody.svelte`'s `canSubmit` не пускает очистку места) не входил в объём этого плана и остаётся отложенным.
- Новый паттерн (`&Connection`-сиблинг рядом с `&ReaderPool`-обёрткой) и новый тестовый паттерн (deadlock regression через `std::thread`+`mpsc::recv_timeout`, не `tokio::test`+`tokio::time::timeout`) задокументированы во frontmatter `tech-stack.patterns` — доступны для переиспользования, если аналогичный риск всплывёт в других reader-pool-зависимых сервисах.

---
*Phase: 40-movement-history*
*Completed: 2026-09-03*
