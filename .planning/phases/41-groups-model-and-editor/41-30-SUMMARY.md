---
phase: 41-groups-model-and-editor
plan: 30
subsystem: reports
tags: [gap-closure, tdd, reports, movements, truncation, W-B03]
requires: ["41-29"]
provides:
  - истинный ReportResponse.total для отчёта «Перемещения» (потолок строк 1000 сохранён)
  - movements_truncation_notice — единственный источник текста уведомления на стороне Rust
  - общая golden-фикстура ui/scripts/fixtures/report-truncation/cases.json для JS-зеркала плана 41-33
affects: [ReportService, tauri_cmds::reports, build_reports_export_pdf, build_reports_export_csv, get_report_counts]
tech-stack:
  added: []
  patterns: ["второй COUNT(*) переиспользует те же with_prefix/where_clause/param_refs", "общая golden-фикстура Rust <-> JS с точным ожидаемым текстом"]
key-files:
  created:
    - crates/trackly-app/tests/report_movements_truncation.rs
    - ui/scripts/fixtures/report-truncation/cases.json
  modified:
    - crates/trackly-app/src/services/report_service.rs
    - crates/trackly-app/src/tauri_cmds/reports.rs
key-decisions:
  - "Сигнализировать усечение, не меняя потолок 1000 и порядок «старые первыми»: total = истинное число подходящих, усечено = total > rows.len()"
  - "Печать: уведомление дописывается к существующему filter_summary («{сводка}. {уведомление}» или одно уведомление); templates/report.html не тронут"
  - "CSV: одна последняя запись, уведомление в первой ячейке через csv_safe, остальные пустые"
  - "Значок вкладки «Перемещения» читает resp.total вместо rows.len()"
requirements-completed: [GRP-06]
metrics:
  duration: ~45 мин (включая 4.5 мин clippy)
  completed: 2026-10-05
---

# Phase 41 Plan 30: усечение отчёта «Перемещения» больше не молчит (W-B03) Summary

Закрыт GAP-5: отчёт «Перемещения» по-прежнему отдаёт не более 1000 строк (самые ранние), но `total` теперь истинный, а печать и CSV несут явное уведомление «Показано записей: 1000 из 1005 (самые ранние). Сузьте период или фильтры, чтобы увидеть остальные.». Значок вкладки показывает истинное число. Экранное уведомление остаётся за планом 41-33.

## Выполнено

| Задача | Коммит | Что сделано |
|--------|--------|-------------|
| 1 (RED) | 82699de5 | Фикстура `cases.json` (7 кейсов), 7 тестов `report_trunc_*`, заглушка `movements_truncation_notice` -> `None` |
| 2 (GREEN) | 772c1d34 | `MOVEMENTS_REPORT_LIMIT`, истинный `total`, уведомление, CSV-строка, объединение с `filter_summary`, значок вкладки |

## Что изменено

- `query_movements_inner`: литерал `LIMIT 1000` заменён константой `MOVEMENTS_REPORT_LIMIT`; при достижении потолка выполняется второй `COUNT(*)` с ТЕМИ ЖЕ `with_prefix`, `where_clause`, `param_refs` (условия не пересобираются). Перед правкой проверено чтением: условия ссылаются только на `pm.*` и `d.type_id`, поэтому в счётном запросе достаточно join к `devices`. Ниже потолка `total = rows.len()` (лишнего запроса нет).
- `movements_truncation_notice(shown, total)`: `None`, если `total <= shown`, иначе текст из плана.
- `get_report_counts`: ветка movements читает `resp.total`.
- `export_csv`: после строк данных, если усечено, пишется одна запись из `columns.len()` ячеек (маркер `// W-B03:csv`).
- `build_reports_export_pdf`: итоговая сводка считается ПОСЛЕ `fetch_report` (маркер `// W-B03:print`). Оба транспорта идут через эту функцию (`http/reports.rs:295` вызывает её же), отдельной правки не нужно.
- `grep -c "LIMIT 1000"` в `report_service.rs`: 7 -> 6 (уменьшилось ровно на 1, остальные пять отчётов не тронуты, плюс строка шапки модуля).

## Фикстура для JS-зеркала (план 41-33)

`ui/scripts/fixtures/report-truncation/cases.json` — массив `{ name, shown, total, expected }`, где `expected` — ТОЧНЫЙ текст уведомления либо `null`. JS-зеркало должно воспроизвести текст дословно из `shown` и `total`, поэтому расхождение формулы (например, `>=` вместо `>`) или чтение не того поля (`rows.length` вместо `total`) ломает гейт. Кейсы: 1000/1005, 1000/1001, 1000/25000 (усечение) и 3/3, 1000/1000, 0/0, 999/999 (без усечения). Rust-тест `report_trunc_notice_matches_golden_fixture` читает тот же файл; тесты CSV и печати сверяются с литералом кейса 1000/1005 из фикстуры, а не с результатом функции.

## RED-прогон (до правки кода)

`TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test report_movements_truncation`:

```
test report_trunc_notice_matches_golden_fixture ... FAILED   (left: None, right: Some("Показано записей: 1000 из 1005 ..."))
test report_trunc_total_counts_rows_beyond_limit ... FAILED  (left: 1000, right: 1005)
test report_trunc_total_respects_filters ... FAILED          (left: 1000, right: 1001)
test report_trunc_counts_badge_uses_total ... FAILED         (left: 1000, right: 1005)
test report_trunc_csv_has_notice_row ... FAILED              (left: 1001, right: 1002)
test report_trunc_print_summary_has_notice ... FAILED        (уведомление в печати без фильтров)
test report_trunc_under_limit_total_equals_rows ... ok       (охранный)
test result: FAILED. 1 passed; 6 failed
```

`report_trunc_under_limit_total_equals_rows` — ОХРАННЫЙ тест, а не красный: он зелёный на старом коде и защищает от ложного усечения (3 строки и ровно 1000 строк). Ошибок компиляции (`error[E`) в RED нет. Первый RED-прогон упал на моей ошибке в хелпере сидирования (не передан `place`), это исправлено до коммита: в закоммиченном RED все шесть красных падают по утверждениям.

## GREEN

- `--test report_movements_truncation`: 7 passed.
- Регрессии по очереди: `group_report_batch` 10 passed, `html_report_render` 8 passed, `report_csv_export` 2 passed, `--lib report_service` 51 passed, `--lib tauri_cmds::reports` 6 passed.
- `cargo clippy -p trackly-app --all-targets -- -D warnings`: чисто.
- `rustfmt --check` по трём моим файлам: чисто (drift в `dto/act.rs`, `tests/acts_*` существовал до плана и не тронут).
- `node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256`: 0 нарушений до обоих коммитов; данные в фикстуре и тестах вымышленные («Склад А/Б/В», «Ноутбук»).
- `git diff --stat -- templates crates/trackly-app/templates` пуст: шаблоны печати не тронуты.

## Мутационная проверка (на закоммиченных файлах, якоря проверены на единственность, откат `git checkout -- <файл>`)

| Мутация | Красные тесты |
|---------|---------------|
| m1: `total = rows.len()` безусловно (якорь `// W-B03:count`) | `total_counts_rows_beyond_limit`, `total_respects_filters`, `counts_badge_uses_total` (а также `csv_has_notice_row` и `print_summary_has_notice`, т.к. зависят от `total`) |
| m2: CSV без записи уведомления (`// W-B03:csv`) | `csv_has_notice_row` |
| m3: печать без уведомления (`// W-B03:print`) | `print_summary_has_notice` |
| m4: «самые ранние» -> «самые поздние» | `notice_matches_golden_fixture` (и CSV/печать, т.к. сверяются с фикстурой) |

Все мутации отпущены, дерево чисто, повторный прогон: 7 passed.

## Deviations from Plan

None - plan executed exactly as written. (Единственная поправка — исправление моей же опечатки в тестовом хелпере до RED-коммита, см. выше.)

## Known Stubs

None. Заглушка `movements_truncation_notice` существовала только в RED-коммите и заменена реализацией в GREEN.

## Для плана 41-33 и ручной проверки

Экранное уведомление этим планом не создаётся. Ручных пунктов в запущенном приложении этот план не требует (изменения проверены интеграционными тестами на реальном `AppCtx`; в живом приложении НЕ проверялось). При UAT фазы стоит посмотреть: на 1000+ перемещений значок вкладки показывает истинное число, а печать и CSV содержат строку уведомления.

## Threat Flags

None - новых сетевых поверхностей, путей аутентификации и изменений схемы нет; гейт `ReadPlaces` отчёта не менялся.

## Self-Check: PASSED

- FOUND: crates/trackly-app/tests/report_movements_truncation.rs
- FOUND: ui/scripts/fixtures/report-truncation/cases.json
- FOUND коммиты 82699de5, 772c1d34
