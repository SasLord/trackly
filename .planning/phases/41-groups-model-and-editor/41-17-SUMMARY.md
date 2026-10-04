---
phase: 41-groups-model-and-editor
plan: 17
subsystem: reports
tags: [rust, reports, movements, groups, batch, print, csv]
requires: ["41-05", "41-10"]
provides:
  - "ReportRow: batch_id, batch_role ('header'|'member'|None), batch_size, batch_label"
  - "Отчёт «Перемещения»: тип «Группа», имя группы из снимка entity_label, причины «перенос группы» / «в составе группы «имя»»"
affects: [41-25 UI свёртки пакета]
tech-stack:
  added: []
  patterns:
    - "batch_size — подзапрос по batch_id (только entity_type='device'), а не по видимым строкам"
    - "печать и CSV идут из тех же плоских строк, шаблон и колонки не тронуты"
key-files:
  created:
    - crates/trackly-app/tests/group_report_batch.rs
  modified:
    - crates/trackly-app/src/dto/reports.rs
    - crates/trackly-app/src/services/report_service.rs
    - crates/trackly-app/tests/html_header_parity.rs
    - crates/trackly-app/tests/html_report_render.rs
    - crates/trackly-app/tests/report_csv_export.rs
key-decisions:
  - "batch_label заполняется и в строке заголовка, и в строках состава одним и тем же снимком entity_label"
  - "movement_reason получил параметры entity_type и entity_label; ветка Group больше не отдаёт заглушку «группой»"
requirements-completed: [GRP-06]
duration: ~25 min
completed: 2026-10-04
---

# Phase 41 Plan 17: Отчёт «Перемещения» и пакеты группового переноса Summary

Отчёт «Перемещения» отдаёт поля пакета (`batch_id`, `batch_role`, `batch_size`, `batch_label`), читаемую строку группы и причины с именем группы; печать и CSV по-прежнему содержат полный состав пакета без правки шаблона.

## Tasks

| Task | Name | Commit |
|------|------|--------|
| 1 | Поля пакета в ReportRow, правки query_movements_inner и movement_reason | 67185094 |
| 2 | Печать и CSV: полный состав пакета (D-27) | 67185094 |

Обе задачи закрыты одним коммитом (см. отклонения).

## Что сделано

- `ReportRow` +4 необязательных поля; все конструкторы в `report_service.rs` (7 штук, а не 9, как в плане) и литералы в трёх тестах (`html_header_parity`, `report_csv_export` x2, `html_report_render`) получили `None`, кроме `query_movements_inner`.
- SQL: `COALESCE(d.name, c.code, pm.entity_label)`, `WHEN 'group' THEN 'Группа'`, колонки `pm.batch_id`, `pm.entity_type`, `pm.entity_label`, подзапрос `batch_size` (NULL при `batch_id IS NULL`). `ORDER BY`, `LIMIT 1000`, `columns_for("movements")` и `templates/report.html` не менялись.
- `batch_role` вычисляется в Rust: `header` для `entity_type='group'`, `member` для остальных строк с `batch_id`.
- 41-02 оставила в `movement_reason` заглушку «группой» — здесь заменена по D-26/D-28: «перенос группы» для строки группы, «в составе группы «{имя}»» для состава (без имени — «в составе группы»).

## Verification

- `group_report_batch` — 10 passed. Прежние цели по одной: `report_movements` 18, `report_csv_export` 2, `html_report_render` 8, `html_header_parity` 5, `report_acts` 2, `report_place_subtree` 12, `report_period_bounds` 3, `export_bindings` 1 — зелёные.
- `cargo clippy -p trackly-app --all-targets -- -D warnings` — чисто; `rustfmt --check` по трём своим файлам чисто.
- `ui/src/bindings.ts` (генерируется, в git не лежит) содержит `batch_role`.
- check-privacy — PASS; имена в тестах вымышленные.
- Мутация (якорь `AND pm2.entity_type = 'device') \` уникален, count == 1): замена на `AND 1 = 1` краснит `report_batch_printer_cartridge_row_not_counted`, `report_batch_type_filter_drops_header_keeps_size`, `report_batch_full_move_header_and_members`; файл восстановлен.
- Фикстура фильтра по типу расходится нарочно: 2 из 6 устройств переведены в тип 2, видимых строк 4, `batch_size` обязан остаться 6 (иначе тест вакуумный).

## Deviations from Plan

**1. [Организационное] Две задачи — один коммит.** Тесты задач 1 и 2 лежат в одном новом файле и писались вместе; раздельный коммит потребовал бы искусственного разбиения файла. Содержание обеих задач выполнено полностью.

**2. [Уточнение плана] Число конструкторов ReportRow в `src` — 7, не 9.** Обновлены все найденные `grep "ReportRow {"`; `cargo check` и clippy проходят.

Других отклонений нет.

## Для следующих планов

- План 25 (UI свёртки): при отсутствии строки `header` в видимом наборе синтезировать заголовок из `batch_label` и `batch_size`.
- Шаблон отчёта и `columns_for` не менялись, предпросмотр шаблонов затронут не был: новых переменных в `templates/*.html` нет (тест `report_batch_template_untouched` фиксирует отсутствие `batch_`).

## Known Stubs

None.

## Threat Flags

None. T-41-17-01: вывод идёт через экранирующий шаблонизатор (существующая логика). T-41-17-02: тесты на 7 строк в HTML и CSV. T-41-17-04: новые части SQL без пользовательского ввода.

## Self-Check: PASSED

- group_report_batch.rs на месте, коммит 67185094 существует
