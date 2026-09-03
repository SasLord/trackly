---
phase: 40-movement-history
plan: 28
subsystem: database
tags: [rusqlite, sqlite, place_movements, cartridges, devices, regression-testing]

# Dependency graph
requires:
  - phase: 40-movement-history
    provides: "40-21 (printer->cartridge place cascade), 40-22 (auto-return storage fallback), 40-VERIFICATION.md gap-closure contract (gaps 1 and 2)"
provides:
  - "Cascade gate: printer place clear (Some->None) no longer wipes attached cartridges' place (CR-03)"
  - "Three-step fallback chain in last_known_storage_place_in_tx covering the real install-then-replace lifecycle (CR-02)"
  - "Fix for a previously undiscovered blocking bug: DeviceService::update could never actually clear a nullable field (place_id) via DevicePatch — COALESCE(?, col) cannot distinguish 'field omitted' from 'field explicitly NULL'"
affects: [40-movement-history, device-patch-consumers]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Explicit provided-flag + CASE WHEN instead of COALESCE(?, col) when a nullable SQL column must be clearable via a double-Option DTO field"
    - "Two-query fallback chain (movement-history hit, then entity's own current state) for 'last known X' derivations"

key-files:
  created: []
  modified:
    - crates/trackly-app/src/services/device_service.rs
    - crates/trackly-infra/src/repos/cartridges_sqlite.rs
    - crates/trackly-infra/src/repos/devices_sqlite.rs
    - crates/trackly-core/src/domain/devices.rs
    - crates/trackly-app/src/dto/device.rs
    - crates/trackly-app/tests/place_movements_write_sites_devices.rs
    - crates/trackly-app/tests/cartridges_lifecycle.rs

key-decisions:
  - "Cascade to attached cartridges only fires when after.place_id.is_some() — clearing a printer's place leaves cartridge places untouched rather than wiping them (product decision recorded in 40-VERIFICATION.md)"
  - "last_known_storage_place_in_tx checks from_place_id storage hits in addition to to_place_id storage hits, then falls back to the cartridge's own current place_id, before giving up with None"
  - "Rule 3 (blocking issue) auto-fix: domain::devices::DevicePatch.place_id widened from Option<i64> to Option<Option<i64>>, and devices_sqlite.rs's two UPDATE statements switched from COALESCE to an explicit provided-flag CASE WHEN for place_id only — narrowly scoped to unblock this plan's CR-03 regression test; other nullable DevicePatch fields (inventory_no, serial_no, model, specs, kit, state) have the same latent COALESCE-can't-clear bug but were left untouched (out of scope, logged as a deferred item)"

requirements-completed: [HST-01]

# Metrics
duration: ~50min
completed: 2026-09-03
---

# Phase 40 Plan 28: Закрытие двух major-гэпов (CR-03, CR-02) Summary

**Гейт `after.place_id.is_some()` на каскад места принтера + трёхступенчатая цепочка fallback в `last_known_storage_place_in_tx`, плюс попутный фикс блокирующего бага — `DeviceService::update` не мог фактически очистить nullable-поле через `COALESCE`.**

## Performance

- **Duration:** ~50 min
- **Completed:** 2026-09-03T03:15:52Z
- **Tasks:** 2 (оба выполнены)
- **Files modified:** 7

## Accomplishments

- **CR-03 закрыт:** очистка места принтера (`Some -> None`) больше не каскадирует на прикреплённые картриджи — их место и `version` остаются нетронутыми, никакой потери данных без аудита.
- **CR-02 закрыт:** авто-возврат картриджа без явного `previous_cartridge_place_id` теперь находит последнее известное складское место через ПОЛНУЮ цепочку (движение-в-склад ИЛИ движение-из-склада, затем собственное складское место картриджа), а не только через `to_place_id`, который D-06 (не пишет строку для первого назначения места) делает недостижимым на самом обычном жизненном цикле.
- **Побочный блокирующий баг обнаружен и исправлен:** при написании регрессионного теста для CR-03 выяснилось, что `DeviceService::update` физически не может очистить `place_id` (или любое другое nullable-поле `DevicePatch`) — `Option<Option<T>>` на уровне DTO уплощался в одинарный `Option<T>` на уровне domain-патча, и `COALESCE(?, col)` не может отличить "поле не передано" от "поле явно передано как NULL" — оба превращаются в SQL NULL, и `COALESCE` молча сохраняет старое значение. Без этого фикса тест CR-03 физически не мог воспроизвести сценарий "очистка места принтера" ни при каких условиях. Исправлено ТОЧЕЧНО для `place_id` (не для остальных nullable-полей — вне объёма этого плана).
- Оба регрессионных теста подтверждены КРАСНЫМИ на нефиксированном коде (с точными сообщениями assert) и ЗЕЛЁНЫМИ после фикса.

## Task Commits

1. **Task 1: Гейт каскада на Some->None (CR-03) + регрессионный тест** — `b5e9b55f` (fix)
2. **Task 2: Полная цепочка fallback в last_known_storage_place_in_tx (CR-02) + регрессионный тест через реальный поток** — `bc1f6110` (fix)

**Plan metadata:** (этот коммит, после self-check)

## Files Created/Modified

- `crates/trackly-app/src/services/device_service.rs` — гейт `after.place_id.is_some() && before_place_id != after.place_id` перед вызовом каскада; расширенный комментарий про CR-03.
- `crates/trackly-infra/src/repos/cartridges_sqlite.rs` — rustdoc-предупреждение над `cascade_place_for_printer_in_tx` о предусловии вызова; `last_known_storage_place_in_tx` переписан на трёхступенчатую цепочку (`p_to.is_storage=1 OR p_from.is_storage=1` через LEFT JOIN + фильтр `archived_at_utc`/`deleted_at_utc`, затем собственное `place_id` картриджа).
- `crates/trackly-infra/src/repos/devices_sqlite.rs` — оба места (`update_in_tx`, trait `update`) переведены с `COALESCE(?, place_id)` на `CASE WHEN <флаг> = 1 THEN <значение> ELSE place_id END` для колонки `place_id` (Rule 3 auto-fix, побочный блокер).
- `crates/trackly-core/src/domain/devices.rs` — `DevicePatch.place_id: Option<i64>` → `Option<Option<i64>>`.
- `crates/trackly-app/src/dto/device.rs` — конвертация DTO→domain теперь сохраняет различие "не передано"/"передано как NULL" (`p.place_id = Some(inner)` вместо `p.place_id = inner`).
- `crates/trackly-app/tests/place_movements_write_sites_devices.rs` — новый тест `update_clearing_printer_place_does_not_touch_cartridges`.
- `crates/trackly-app/tests/cartridges_lifecycle.rs` — новый хелпер `seed_printer_device_with_place`, новый тест `install_auto_return_falls_back_via_real_service_flow_no_hand_seed` (полностью через `CartridgeService`, без ручного SQL-посева `place_movements`).

## Decisions Made

- Гейт на `after.place_id.is_some()` — единственная точка изменения продакшн-логики каскада, как и требовал план; сигнатура `cascade_place_for_printer_in_tx` не менялась, только предусловие вызова.
- Фикс COALESCE-бага сделан МИНИМАЛЬНО (только `place_id`, не другие nullable-поля `DevicePatch`), чтобы не расширять объём плана — задокументировано как отложенный пункт ниже.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 — Blocking] `DeviceService::update` не мог фактически очистить `place_id` (или любое nullable-поле `DevicePatch`) через `COALESCE`-based SQL**
- **Найдено во время:** Task 1, при написании регрессионного теста для CR-03. Первая версия теста (по плану: `svc.update(..., DevicePatch { place_id: Some(None), ..Default::default() })`) прошла ЗЕЛЁНОЙ ещё ДО фикса гейта — это был сигнал бага теста (правило проекта: "Если тест зелёный ДО фикса — это баг теста"). Добавив временную отладочную проверку, подтвердил: место принтера в БД оставалось `Some(place_a)`, никогда не становилось `None`.
- **Причина:** `domain::devices::DevicePatch.place_id: Option<i64>` — уплощение DTO-уровневого `Option<Option<i64>>` теряло различие "поле не передано" vs "поле явно передано как NULL". В SQL `place_id = COALESCE(?8, place_id)` оба случая биндятся как SQL `NULL` → `COALESCE` молча сохраняет старое значение. Тот же баг присутствует у остальных nullable-полей `DevicePatch` (`inventory_no`, `serial_no`, `model`, `specs`, `kit`, `state`) — не исправлялось, вне объёма этого плана.
- **Фикс:** `domain::devices::DevicePatch.place_id` → `Option<Option<i64>>`; DTO→domain конверсия сохраняет разницу; в `devices_sqlite.rs` (оба места — `update_in_tx` и trait `update`) заменил `COALESCE(?8, place_id)` на `CASE WHEN ?8 = 1 THEN ?9 ELSE place_id END` с явным флагом "поле передано" (`patch.place_id.is_some() as i64`) и значением (`patch.place_id.flatten()`).
- **Файлы:** `crates/trackly-core/src/domain/devices.rs`, `crates/trackly-app/src/dto/device.rs`, `crates/trackly-infra/src/repos/devices_sqlite.rs`.
- **Верификация:** после фикса регрессионный тест CR-03 стал КРАСНЫМ (место картриджа НЕ вернулось `None` до гейт-фикса — правильное поведение теста), затем ЗЕЛЁНЫМ после гейт-фикса; полный прогон `devices_crud.rs`, `devices_location_roundtrip.rs`, `devices_bulk_create.rs`, `devices_type_conversion.rs` (35 тестов) — без регрессий.
- **Committed in:** `b5e9b55f` (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (Rule 3, blocking issue)
**Impact on plan:** Необходимо для корректности собственно теста CR-03 — без этого фикса сценарий "очистка места принтера" физически недостижим через `DeviceService::update`, и заявленный продуктовый гейт (`after.place_id.is_some()`) никогда бы не сработал в реальности. Объём фикса намеренно сужен только до `place_id`; остальные nullable-поля оставлены как отложенный технический долг (см. ниже).

## Known Deferred Items (not fixed, logged for future plan)

- **Тот же COALESCE-can't-clear баг у остальных nullable-полей `DevicePatch`:** `inventory_no`, `serial_no`, `model`, `specs`, `kit`, `state` — попытка очистить любое из этих полей через `DeviceService::update` (`Some(None)` в DTO) молча не срабатывает, как и `place_id` до этого фикса. Вне объёма 40-28 (только `place_id` был блокером для CR-03). Заслуживает отдельного точечного плана — тот же паттерн фикса (`Option<Option<T>>` в domain-патче + `CASE WHEN <флаг>` вместо `COALESCE`) применим ко всем сразу.
- **UI-уровень:** `DeviceFormBody.svelte`'s `canSubmit` требует `placeId !== null` — то есть форма редактирования устройства СЕЙЧАС не позволяет пользователю отправить очистку места вообще (независимо от бэкенд-бага). Фикс в этом плане — defense-in-depth на уровне сервиса (API можно вызвать напрямую, минуя форму), но живой UI-сценарий "очистить место принтера" из `human_verification` пункта 2 40-VERIFICATION.md по-прежнему недостижим через штатную форму до отдельного UI-фикса.
- Остальные два гэпа из 40-VERIFICATION.md (CR-01 reader-pool deadlock risk, WR-10 movements-report act-number formatting) — вне объёма 40-28, ожидают отдельного плана (40-29 или далее).

## Issues Encountered

Тест для CR-03, написанный по буквальному тексту плана, изначально прошёл ЗЕЛЁНЫМ на нефиксированном коде — сигнал, что тест не тестирует заявленный сценарий (см. Deviations выше). Диагностировано добавлением временной `eprintln!`-проверки места принтера после "очистки", подтвердившей, что место принтера в БД не менялось. После фикса блокирующего бага тест стал корректно КРАСНЫМ до гейт-фикса и ЗЕЛЁНЫМ после.

## Verification Evidence (красный-до / зелёный-после)

### CR-03: `update_clearing_printer_place_does_not_touch_cartridges`

**До фикса гейта** (после фикса блокирующего COALESCE-бага, но с исходным безусловным `if before_place_id != after.place_id`):
```
thread 'update_clearing_printer_place_does_not_touch_cartridges' panicked at .../place_movements_write_sites_devices.rs:577:9:
assertion `left == right` failed: CR-03: очистка места принтера НЕ должна трогать место картриджа
  left: None
 right: Some(1)
```

**После фикса гейта:**
```
test update_clearing_printer_place_does_not_touch_cartridges ... ok
```
Полный файл (6 тестов, включая нетронутый `update_cascades_place_to_attached_cartridges`): `test result: ok. 6 passed; 0 failed`.

### CR-02: `install_auto_return_falls_back_via_real_service_flow_no_hand_seed`

**До фикса (единственный запрос на `to_place_id`):**
```
thread 'install_auto_return_falls_back_via_real_service_flow_no_hand_seed' panicked at .../cartridges_lifecycle.rs:1455:9:
assertion `left == right` failed: A's place_id must resolve to its last known STORAGE place via the from_place_id branch...
  left: None
 right: Some(1)
```

**После фикса (трёхступенчатая цепочка):**
```
test install_auto_return_falls_back_via_real_service_flow_no_hand_seed ... ok
```
Три теста по общему префиксу (`install_auto_return_falls_back*`): `test result: ok. 3 passed; 0 failed` — включая нетронутый hand-seeded `install_auto_return_falls_back_to_last_known_storage_place`.

## Full Verification Run

- `cargo test -p trackly-app --test place_movements_write_sites_devices` — 6/6 ok.
- `cargo test -p trackly-app --test cartridges_lifecycle` — 25/25 ok.
- Смежные device/cartridge/place_movements наборы (`devices_crud`, `devices_location_roundtrip`, `devices_bulk_create`, `devices_type_conversion`, `cartridges_search`, `cartridges_numbering`, `cartridges_crud`, `cartridges_history`, `place_movements_act_link`, `place_movements_timeline`, `place_movements_bulk_move`, `place_movements_write_sites_cartridges`, `report_cartridges`) — все зелёные, без регрессий.
- `cargo clippy -p trackly-app -p trackly-infra -p trackly-core -- -D warnings` — чисто.
- `cargo fmt --all --check` — чисто.
- `node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256` — 0 нарушений (x3, при каждом коммите).
- Полный прогон `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app -- --skip login_remember_persistent_cookie` — **861 passed, 0 failed, 2 ignored**. (Без `TRACKLY_AD_MOCK`/`TRACKLY_SNMP_MOCK` один несвязанный тест — `restore_request_visibility_http::blocked_user_restore_request_visible_to_admin_and_marks_pending_http` — падает из-за отсутствия mock AD/SNMP в окружении; это pre-existing требование CI (`.github/workflows/ci-fast.yml`), не регрессия этого плана.)
- `cargo test -p trackly-core -p trackly-infra` (с теми же mock-флагами) — все наборы зелёные (92, 132 и другие — без failed).

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- CR-03 и CR-02 закрыты и покрыты регрессионными тестами, привязанными к реальному потоку сервисов (не к ручному SQL-посеву).
- 40-VERIFICATION.md's `gaps:` block теперь содержит 2 закрытых из 4 (CR-03, CR-02); CR-01 (reader-pool deadlock risk) и WR-10 (report act-number formatting) остаются — план 40-29 или отдельный план должен их закрыть.
- Отложенный технический долг (см. Known Deferred Items) — COALESCE-can't-clear баг для остальных nullable-полей `DevicePatch` — стоит зафиксировать как отдельный будущий пункт (не блокирует текущую веху, но является реальным production-багом).

---
*Phase: 40-movement-history*
*Completed: 2026-09-03*

## Self-Check: PASSED

All 7 modified/created source files and this SUMMARY.md confirmed present on disk; both task
commits (`b5e9b55f`, `bc1f6110`) confirmed present in `git log --oneline --all`.
