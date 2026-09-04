# Deferred items — Phase 40 (movement-history)

## `users_update_password_change` / `delete_then_recreate_revives_same_login` — intermittent 30s test-budget timeout

**Found during:** Plan 40-33, Task 3, full-package verification run
(`TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app -- --skip
login_remember_persistent_cookie`).

**Out of scope** — `crates/trackly-app/tests/users_crud.rs` is not touched by Plan 40-33
(cartridges/refill-default work only). Not auto-fixed per the executor's scope boundary.

**Symptom:** In two separate full-package runs, one or both of
`delete_then_recreate_revives_same_login` and `users_update_password_change` panicked with
`test exceeded 30s budget: Elapsed(())`. In isolation (`cargo test -p trackly-app --test
users_crud -- delete_then_recreate_revives_same_login users_update_password_change`) both tests
pass reliably in ~16s total.

**Likely cause:** argon2id hashing (`m=19456 KiB, t=2, p=1`) is CPU/memory-hard; when the full
104-file `trackly-app` test package runs back-to-back under machine load (multiple prior test
binaries still settling, disk I/O from the writer worker), the 30s per-test budget in these two
tests is tight enough to occasionally miss. Not a correctness bug — a resource-contention flake
in the test harness's own timeout, unrelated to argon2's actual production behavior.

**Recommendation:** Either raise the 30s budget for these two specific tests (they perform a
full argon2id hash + DB round-trip) or leave as a documented flake — do not touch
`users_crud.rs` under Plan 40-33's scope.

---

## Первый администратор недостижим через браузер по сети (HTTP/LAN-транспорт)

**Найдено:** 2026-09-04, при живой проверке чекпоинтов фазы 40 на одноразовом инстансе с пустой БД.

**Симптом:** завести самого первого администратора через LAN-браузер штатным путём невозможно.
`FirstRunWizard.svelte` дёргает эндпоинт `users_create`; на HTTP-транспорте `session_identity()`
всегда требует существующую сессию, поэтому вызов отвечает «unauthorized» — а сессии нет, потому
что нет ни одного пользователя. На десктопном (Tauri) транспорте проблемы нет:
`resolve_tauri_identity` выдаёт `trusted_admin()` при выключенном lock, и визард работает.

**Следствие:** свежую установку, доступную только по сети, штатно настроить нельзя. Все живые
проверки этой сессии заводили первого админа прямой SQL-вставкой в throwaway-БД.

**Не чинилось** — вне области фазы 40 (история перемещений). Требует продуманного решения, а не
механического: пока приложение находится в состоянии bootstrap, любой, кто дотянется до порта,
может занять учётную запись администратора. Стоит взвесить одноразовый токен, печатаемый в консоль
при первом запуске, или эквивалентную защиту, и решить осознанно.

**Покрытие при починке:** интеграционные тесты на HTTP-транспорте — bootstrap проходит на пустой
таблице пользователей и отвергается, как только появился хоть один пользователь.

---

## Очистка nullable-полей `DevicePatch` работает только для `place_id` (WR-03)

**Найдено:** 2026-09-03, при закрытии гэпа CR-03 в плане 40-28; зафиксировано как WR-03 в
`40-REVIEW.md`.

**Симптом:** `DeviceService::update` не может очистить nullable-поле — генерируемый SQL
`COALESCE(?, col)` не отличает «поле не передали» от «передали NULL» после того, как доменный
patch схлопывает `Option<Option<T>>` в `Option<T>`. Починено УЗКО, только для `place_id`
(`trackly-core/src/domain/devices.rs`, `trackly-app/src/dto/device.rs`,
`trackly-infra/src/repos/devices_sqlite.rs`).

**Осталось:** ещё шесть nullable-полей — `inventory_no`, `serial_no`, `model`, `specs`, `kit`,
`state` — по-прежнему молча игнорируют явную очистку. Docstring в DTO при этом обещает единообразную
семантику для всех семи, так что рабочий `place_id` теперь стоит рядом с шестью сломанными
двойниками, и это опаснее прежнего единообразно сломанного состояния.

**Сегодня пользователю не видно** — ни один экран не отправляет явную очистку этих полей.

**При починке решить осознанно**, каждое ли из шести полей вообще должно быть очищаемым, или часть
из них правильнее задокументировать как неочищаемые — и добавить регрессионные тесты на каждое
поле, которое станет очищаемым.
