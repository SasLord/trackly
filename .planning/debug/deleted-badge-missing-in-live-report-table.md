---
status: diagnosed
trigger: "Маркер «удалено» (D-25) присутствует в экспорте CSV/PDF отчёта «Перемещения», но не отображается в живой таблице отчёта внутри приложения."
created: 2026-09-03T00:00:00Z
updated: 2026-09-03T00:00:00Z
---

## Current Focus

hypothesis: ПОДТВЕРЖДЕНА — `ReportTable.svelte::showDeletedBadge` сравнивает проп `reportType` со строкой `'movements'`, но `ReportsPage.svelte` передаёт в этот проп `activeReport`, который в домене «Перемещения» равен `'all'` (ключ единственной вкладки), а не `'movements'`. Условие всегда false → бейдж не рисуется никогда.
test: Трассировка обоих путей (живой список vs экспорт) от SQL до рендера.
expecting: Расхождение только на фронте; `is_deleted` доезжает до обоих.
next_action: Вернуть диагноз оркестратору (режим find_root_cause_only, фикс не применять).

## Symptoms

expected: |
  В отчёте «Перемещения» строки по мягко удалённым предметам помечаются бейджем
  «Удалено» в живой таблице и суффиксом «(удалено)» в CSV/PDF.
actual: |
  Экспорт CSV/PDF помечает корректно; живая таблица приложения — нет.
errors: нет
reproduction: Test 13, .planning/phases/40-movement-history/40-UAT.md
started: UAT фазы 40

## Eliminated

- hypothesis: "`is_deleted` не попадает в DTO/сериализацию живого списка (в отличие от экспортного пути)"
  evidence: |
    `ReportRow` (crates/trackly-app/src/dto/reports.rs:147) — одно поле `pub is_deleted: Option<bool>`
    для обоих путей; DTO без `rename_all = "camelCase"`, сгенерированный `ui/src/bindings.ts:2541`
    подтверждает snake_case `is_deleted: boolean | null` — контракт совпадает с чтением на фронте.
    Экспорт (tauri_cmds/reports.rs:513) сам вызывает тот же `reports.list_movements(...)`,
    что и живой список (build_reports_list_movements, стр. 263-270) → строки идентичны.
  timestamp: 2026-09-03

- hypothesis: "Бейдж привязан к ключу колонки, которого нет в COLUMNS_MAP.movements"
  evidence: |
    `showDeletedBadge` требует `col.key === 'device_name'`; COLUMNS_MAP.movements
    (ReportsPage.svelte:222-229) содержит `{ key: 'device_name', label: 'Предмет' }`.
    Ключ совпадает — это НЕ причина.
  timestamp: 2026-09-03

- hypothesis: "Проп showDeletedBadge/is_deleted вообще не передаётся из ReportsPage"
  evidence: |
    `showDeletedBadge` — не проп, а локальная функция ReportTable.svelte:174.
    Строки приходят в `rows` целиком, `is_deleted` объявлен в локальном интерфейсе
    ReportRow (ReportTable.svelte:36) — доступ есть.
  timestamp: 2026-09-03

## Evidence

- timestamp: 2026-09-03
  checked: ui/src/features/reports/ReportTable.svelte:174-176
  found: |
    function showDeletedBadge(row, col) {
      return reportType === 'movements' && col.key === 'device_name' && row.is_deleted === true;
    }
  implication: Единственное употребление пропа `reportType` во всём файле (стр. 59, 63, 175) — гейт бейджа.

- timestamp: 2026-09-03
  checked: ui/src/features/reports/ReportsPage.svelte:602-608 (<ReportTable ... />)
  found: "reportType={activeReport}"
  implication: В таблицу уходит ключ ВКЛАДКИ, а не ключ домена/бэкендного report_type.

- timestamp: 2026-09-03
  checked: ui/src/features/reports/ReportsPage.svelte:146-149 (MOVEMENT_REPORTS) и :548
  found: |
    MOVEMENT_REPORTS = [{ key: 'all', label: 'Все перемещения', cmd: 'reports_list_movements' }]
    при смене домена: activeReport = d === 'devices' ? 'acts' : d === 'cartridges' ? 'consumption' : 'all';
  implication: |
    В домене «Перемещения» activeReport ВСЕГДА === 'all'. Значит reportType==='movements'
    в showDeletedBadge не выполняется никогда → бейдж не рендерится ни для одной строки.

- timestamp: 2026-09-03
  checked: ui/src/features/reports/ReportsPage.svelte:336-374 (reportTypeKey()) и :487, :618
  found: |
    reportTypeKey() для activeDomain === 'movements' возвращает 'movements';
    оба экспорта (CSV стр. 487, PDF стр. 618) передают reportType: reportTypeKey().
  implication: |
    Вот и асимметрия: экспорт использует НОРМАЛИЗОВАННЫЙ ключ домена ('movements'),
    экран — сырой ключ вкладки ('all'). Один и тот же концепт «тип отчёта» имеет
    два разных значения в соседних вызовах одной страницы.

- timestamp: 2026-09-03
  checked: ui/src/features/reports/ReportsPage.svelte:212-221 (комментарий к COLUMNS_MAP.movements) и :378-381 (currentColumns())
  found: |
    Коллизия ключа 'all' между доменами «Заявки» и «Перемещения» уже была известна и
    обойдена в ДВУХ местах: currentColumns() и currentCmd() ветвятся на activeDomain.
  implication: |
    Обход коллизии применили к колонкам и к команде, но не к пропу reportType таблицы —
    классическая пропущенная третья площадка того же known-issue.

- timestamp: 2026-09-03
  checked: crates/trackly-app/src/services/report_service.rs:1482, 1507, 1586 и 1133-1141
  found: |
    SQL отдаёт CASE ... AS is_deleted (стр. 1482), читается как i64 (1507) и кладётся
    как is_deleted: Some(is_deleted != 0) (1586). row_field, ветка "device_name"
    (1133-1141) добавляет суффикс «(удалено)» при is_deleted == Some(true).
  implication: Бэкенд-половина D-25 корректна и общая для обоих путей; дефект целиком на фронте.

- timestamp: 2026-09-03
  checked: ui/scripts/*.mjs
  found: |
    9 check-*.mjs гейтов (contrast, focus-outline, print-isolation, place-path-short,
    placepath-parity, ...) — ни одного, покрывающего бейдж «Удалено» / гейт reportType.
  implication: |
    Ни svelte-check, ни eslint, ни build не видят логической ошибки в сравнении строк —
    расхождение «экран vs экспорт» не имеет автоматического гейта (повторение паттерна
    js_rust_mirror_needs_fixture_gate / compile_gates_miss_svelte_runtime).

## Resolution

root_cause: |
  ReportsPage.svelte передаёт в ReportTable проп reportType={activeReport} (ключ вкладки),
  а ReportTable.showDeletedBadge сравнивает его с 'movements' (ключ домена / бэкендный
  report_type). В домене «Перемещения» единственная вкладка имеет key 'all'
  (MOVEMENT_REPORTS, ReportsPage.svelte:147; присваивается на :548), поэтому гейт
  reportType === 'movements' ложен всегда и бейдж не рендерится ни для одной строки.
  Экспорт не затронут, потому что использует нормализованный reportTypeKey() === 'movements'.
fix: "не применялся (режим find_root_cause_only)"
verification: ""
files_changed: []
