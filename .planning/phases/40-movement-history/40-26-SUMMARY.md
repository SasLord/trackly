---
phase: 40-movement-history
plan: 26
subsystem: devices
tags: [rusqlite, sqlite, svelte-5, specta, place-path]

# Dependency graph
requires:
  - phase: 39.1
    provides: "from_row_with_short_path / place_path_short формула сокращения пути, condition_distinct_count прецедент для distinct-count колонок"
provides:
  - "list_by_ids отдаёт place_path_short для каждого устройства (дети развёрнутой группы больше не показывают прочерк)"
  - "place_distinct_count во всех трёх SQL-ветках list_grouped + DeviceGroupRow (core) + DeviceGroup (dto)"
  - "DeviceGroupRow.svelte гасит место строки-группы при неоднородности мест"
affects: [devices, movement-history, reports]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "COUNT(DISTINCT COALESCE(col, sentinel)) AS x_distinct_count — второй прецедент (после condition_distinct_count), sentinel подбирается по типу колонки (' ' для текста, -1 для целочисленного FK)"

key-files:
  created: []
  modified:
    - crates/trackly-infra/src/repos/devices_sqlite.rs
    - crates/trackly-core/src/domain/devices.rs
    - crates/trackly-app/src/dto/device.rs
    - crates/trackly-app/src/services/device_service.rs
    - ui/src/features/devices/DeviceGroupRow.svelte
    - crates/trackly-app/tests/devices_grouping.rs

key-decisions:
  - "Sentinel для place_distinct_count = -1 (не ' ' как у condition_distinct_count), т.к. place_id — целочисленный FK, не текст; NULL-места считаются отдельным бакетом (WR-04 прецедент)"
  - "Инлайн-тернарник в разметке ячейки места (не именованная $derived-переменная) — сохраняет буквальные подстроки place_path_short/full_path, требуемые гейтом check-place-path-short.mjs"

requirements-completed: [HST-01]

# Metrics
duration: ~5min (эта сессия; Task 1 выполнен и доведён до коммита предыдущим исполнителем, прерванным лимитом провайдера)
completed: 2026-09-03
---

# Phase 40 Plan 26: Гэп grouped-device-list-place-inversion Summary

**Два независимых бэкенд-дефекта колонки «Место» в списке устройств закрыты: `list_by_ids` теперь использует `from_row_with_short_path` (дети развёрнутой группы видят своё место), и все три SQL-ветки `list_grouped` считают `place_distinct_count`, который фронтенд использует, чтобы гасить место строки-группы при неоднородности вместо показа места случайного члена.**

Эта сессия — ПРОДОЛЖЕНИЕ после прерывания предыдущего исполнителя лимитом провайдера
(quota limit). Task 1 был уже применён в рабочем дереве предыдущим исполнителем, но не
закоммичен; эта сессия проверила диф на соответствие плану (включая обязательный grep
по оставшимся голым `from_row` в файле), закоммитила его как Task 1, и выполнила Tasks
2-4 с нуля.

## Performance

- **Duration:** ~5 min (видимая часть сессии, Task 1 → SUMMARY)
- **Started:** 2026-09-03T01:00:00Z (resume)
- **Completed:** 2026-09-03T01:08:43Z
- **Tasks:** 4/4
- **Files modified:** 6

## Accomplishments
- `list_by_ids` больше не роняет `place_path_short` в `None` — дети развёрнутой группы устройств показывают собственное сокращённое место
- `place_distinct_count` (симметрично `condition_distinct_count`) считается во всех трёх SQL-ветках `list_grouped`, прокинут через `DeviceGroupRow` (core) → `DeviceGroup` (dto) → фронтенд
- Строка-группы гасит место («—»), только если места у всех членов группы разные; при одинаковом месте показывает его
- 4 новых интеграционных теста покрывают оба фикса, включая обе SQL-ветки (`group_by_condition` true/false)
- `cargo clippy --workspace -- -D warnings`, `pnpm lint` (включая `check-place-path-short.mjs`), `pnpm svelte-check` — все зелёные

## Task Commits

Each task was committed atomically:

1. **Task 1: Фикс A — list_by_ids отдаёт сокращённый путь** - `a9df9347` (fix) — диф применён предыдущим исполнителем, проверен и закоммичен этой сессией
2. **Task 2: Фикс B — place_distinct_count для неоднородных групп (backend)** - `1145eda7` (fix)
3. **Task 3: Фронтенд — гасить место группы при неоднородности** - `fa006236` (fix)
4. **Task 4: Интеграционные тесты для обоих фиксов** - `824ec722` (test)

**Deviation fix (Rule 1):** `dd67bcb5` (fix) — устранение `clippy::doc_lazy_continuation` в doc-комментариях, добавленных Task 2

**Plan metadata:** committed as part of this response (docs: complete plan)

## Files Created/Modified
- `crates/trackly-infra/src/repos/devices_sqlite.rs` — `list_by_ids` использует `from_row_with_short_path`; все три SQL-ветки `list_grouped` (`sql_grouped_by_model_no_query`, `sql_grouped_by_model_with_query`, `sql_without_condition`) добавили `COUNT(DISTINCT COALESCE(d.place_id, -1)) AS place_distinct_count`; `GroupRowTuple`/`group_row_tuple()`/деструктуризация/конструктор `DeviceGroupRow` обновлены под новую колонку
- `crates/trackly-core/src/domain/devices.rs` — `DeviceGroupRow.place_distinct_count: i64`
- `crates/trackly-app/src/dto/device.rs` — `DeviceGroup.place_distinct_count: i64` (`#[specta(type = i32)]`, тот же паттерн что у `condition_distinct_count`)
- `crates/trackly-app/src/services/device_service.rs` — `list_grouped` прокидывает новое поле из `DeviceGroupRow` в `DeviceGroup`
- `ui/src/features/devices/DeviceGroupRow.svelte` — ячейка места: `group.place_distinct_count > 1 ? '—' : (group.repr.place_path_short ?? '—')`, `title` аналогично гасится
- `crates/trackly-app/tests/devices_grouping.rs` — 4 новых теста: `list_by_ids_returns_place_path_short_for_device_with_place`, `grouping_place_distinct_count_mixed`, `grouping_place_distinct_count_uniform`, `grouping_place_distinct_count_true_branch`

## Decisions Made
- Sentinel для `COUNT(DISTINCT COALESCE(d.place_id, ...))` = `-1`, а не `' '` (как у `condition_distinct_count`) — `place_id` целочисленный FK, не текстовое поле; NULL-места должны считаться отдельным бакетом (WR-04 прецедент, задокументировано в плане)
- Гейтинг ячейки места на фронтенде написан инлайн-тернарником прямо в разметке, а не через именованную `$derived`-переменную — того требует структурный гейт `check-place-path-short.mjs` (INV-1/2/3), который ищет буквальные подстроки `place_path_short`/`full_path` в тексте/title ячейки

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Устранён `clippy::doc_lazy_continuation` в doc-комментариях `place_distinct_count`**
- **Found during:** финальная проверка `cargo clippy --workspace --all-targets -- -D warnings` (часть `<verification>` плана) после завершения Task 4
- **Issue:** doc-комментарии к новому полю `place_distinct_count` (в `crates/trackly-core/src/domain/devices.rs` и `crates/trackly-app/src/dto/device.rs`) начинались со строки `> 1 означает...` — clippy парсит ведущий `>` как markdown-blockquote и требует `>`-маркер на каждой продолжающей строке; без него — ошибка компиляции под `-D warnings`
- **Fix:** переформулировано без ведущего `>` («Значение больше 1 означает...» / «A value greater than 1 means...»)
- **Files modified:** `crates/trackly-core/src/domain/devices.rs`, `crates/trackly-app/src/dto/device.rs`
- **Verification:** `cargo clippy --workspace --all-targets -- -D warnings` — чисто
- **Committed in:** `dd67bcb5`

---

**Total deviations:** 1 auto-fixed (1 Rule 1 — bug/lint)
**Impact on plan:** Косметическая правка doc-комментария, добавленного в рамках Task 2 этого же плана; не затрагивает поведение, только компилируемость под `-D warnings`. Без scope creep.

## Issues Encountered
None — план выполнен как написан, кроме одной мелкой clippy-правки собственного же doc-комментария (см. выше).

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Оба дефекта UAT-40 test 13 (grouped-device-list-place-inversion) закрыты: дети развёрнутой группы видят собственное место, строка-группы гасит место при неоднородности
- `ui/src/bindings.ts` (gitignored) перегенерирован через `cargo test -p trackly-app --test export_bindings` — содержит `place_distinct_count`; следующая сборка/dev-сессия должна перегенерировать его заново на чистом чекауте (`pnpm prebuild` hook делает это автоматически)
- Все остальные gap-closure планы волны (40-21, 40-24, 40-25) уже смёржены; этот план не пересекается с ними по файлам

---
*Phase: 40-movement-history*
*Completed: 2026-09-03*
