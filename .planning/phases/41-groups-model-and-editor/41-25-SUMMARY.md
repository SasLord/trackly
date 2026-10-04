---
phase: 41-groups-model-and-editor
plan: 25
subsystem: ui
tags: [svelte, reports, movements, groups, batch, vocabulary-gate, places]
requires: [41-15, 41-17, 41-24]
provides:
  - "ReportTable: пакет группового перемещения — одна свёрнутая строка с шевроном (D-25/D-26), заголовок синтезируется из batch_label/batch_size, если строки группы нет в видимом наборе"
  - "PlaceContents: строка про переезжающие группы в модалке массового переноса и тост «Перенесено: N устройств и M группа» (D-23)"
  - "allowlist словарного гейта: узкие записи для ReportTable и PlaceContents"
affects: [41-26]
tech-stack:
  added: []
  patterns:
    - "ReportTable складывает строки пакета в BatchItem поверх существующего потока разделителей месяцев; reportType !== 'movements' возвращает rows как есть"
    - "тост массового переноса строится из серверных device_count/moving_group_count, прочитанных при открытии модалки"
key-files:
  created: []
  modified:
    - ui/src/features/reports/ReportTable.svelte
    - ui/src/features/places/PlaceContents.svelte
    - ui/scripts/check-group-vocabulary.mjs
key-decisions:
  - "Шеврон внутри ячейки «Предмет» обычной TableRow, а не TableRow group с colspan: колонка «Дата» стоит перед «Предметом», colspan потерял бы дату заголовка (отклонение от формулировки UI-SPEC §14.1)"
  - "Печать не тронута: в ReportTable нет ни одного @media print; экран и бумага расходятся сознательно (D-27)"
requirements-completed: [GRP-06, GRP-07]
completed: 2026-10-04
---

# Phase 41 Plan 25: пакеты перемещений в отчёте и группы в массовом переносе места Summary

Отчёт «Перемещения» показывает групповой перенос одной свёрнутой строкой «имя (N устройств)» с шевроном, а модалка массового переноса места предупреждает о переезжающих группах и пишет в тосте число устройств и групп из серверных данных.

## Tasks

| Task | Name | Commit |
|------|------|--------|
| 1 | ReportTable: свёртка пакета перемещений шевроном | 5927a2d9 |
| 2 | PlaceContents: группы в модалке и тосте (D-23), allowlist | c2c31640 |

## Что сделано

- **ReportTable (только `movements`):** строки с `batch_role === 'header'` образуют `BatchItem`, их члены (`member` с тем же `batch_id`) выводятся под заголовком при раскрытии (`TableRow indent`). Пакет без строки-заголовка в видимом наборе (первое размещение группы без места по D-21, фильтр типа устройства, LIMIT 1000, удалённая группа) получает синтезированный заголовок: имя из `batch_label`, число из `batch_size`, Тип «Группа», причина «перенос группы», Дата — первой строки, Откуда/Куда/Кем — значение, общее для всех видимых строк пакета, иначе «—». При пустом `batch_label` синтеза нет, строки остаются обычными. По умолчанию всё свёрнуто (`expandedBatches`, пересоздаваемый `Set`).
- Шеврон: `button`, `aria-expanded`, `aria-label` «Свернуть»/«Развернуть», `stopPropagation`; визуал скопирован из `.tr-row-chevron` (18px, поворот 90°, фокус-кольцо). Счётчик — `pluralizeRu`, `.tr-mono`. Разделители месяцев строятся по строке-заголовку (реальной или синтезированной).
- **PlaceContents:** в отменяемом `$effect` модалки параллельно с `places_contents` запрашивается `places_subtree_stats` (`moving_group_count`, `device_count`), оба под одним флагом `cancelled`; сбой статистики даёт `null` и прежний тост «Содержимое перенесено». Строка «Вместе с содержимым переедут N группа/группы/групп и весь их состав.» — только при N > 0 и после загрузки. Существующие тексты модалки и `notifyPlaceContentChanged` не менялись.
- **Словарный гейт GRD-06:** запись ReportTable с маркерами `'Группа'`, `'перенос группы'`; запись PlaceContents с маркером `'групп'` (ширина записи описана комментарием в гейте: покрывает строчное «группировать», не покрывает «Группировать» и «сгруппировать»). Selftest 12 -> 20 фикстур.

## Verification (что реально запускалось)

- `pnpm --dir ui run lint` — exit 0 на обоих коммитах (в цепочке check-print-isolation, check-report-type-parity, check-movements-type-filter, check-place-tree-invalidation, check-group-vocabulary selftest + живой, check-privacy).
- `pnpm --dir ui run svelte-check` — 0 ERRORS / 68 WARNINGS (база).
- `pnpm --dir ui exec vite build` — проходит. `pnpm --dir ui run build` (хук prebuild с cargo) НЕ запускался.
- Мутации гейта (якоря уникальны, count == 1, файлы восстановлены `cp`/`cmp`): убран маркер «перенос группы» у ReportTable -> красны живой прогон (`ReportTable.svelte:228`) и selftest; заменён маркер PlaceContents -> красны живой прогон (`PlaceContents.svelte:473`) и selftest (3 FAIL).
- `git diff --stat` по `ReportsPage.svelte`, `templates`, `crates` пуст: колонки, серверный шаблон отчёта, `columns_for("movements")`, CSV и `html_header_parity` / `report_csv_export` не затронуты и не ослаблены. Cargo-тесты не запускались (серверной правки нет).

## Deviations from Plan

**1. [Зафиксированное отклонение от UI-SPEC §14.1]** Шеврон лежит в ячейке «Предмет» обычной `TableRow`, а не в `TableRow group` с colspan (решение плана, колонка «Дата» стоит перед «Предметом»).

**2. [Уточнение] Фикстуры selftest.** Плану хватало одной «Группировать»-фикстуры для PlaceContents; добавлены ещё «Сгруппировать» и негатив для другого файла «Мест». Строчное «группировать» в PlaceContents гейт пропустит (известная ширина маркера, задокументирована в гейте).

Прочих отклонений нет.

## UNVERIFIED (ничего из этого не запускалось)

`svelte-check`, `eslint` и сборка слепы к рантайму рун. Не проверено ни в Tauri (WKWebView), ни в LAN-браузере; Playwright не использовался.
- Реальный вид и поведение свёрнутой строки пакета: шеврон, раскрытие/сворачивание, отступ строк устройств, выравнивание счётчика, строки пакета на границе месяцев; нет ли `effect_update_depth_exceeded`/ошибок рун в ReportTable (`display`/`grouped` — `$derived.by`, `expandedBatches` пишется только из обработчика клика).
- Синтезированный заголовок на живых данных: первое размещение группы без места, фильтр «тип устройства», пакет без строки группы.
- Печать десктоп и LAN: полный состав пакета независимо от раскрытия, отсутствие шевронов, многостраничность (в UI нет print-правил, но бумага не смотрелась).
- Модалка массового переноса с группами: строка про группы, тост «Перенесено: N устройств и M группа», склонения, обновление счётчиков дерева «Мест»; поведение при сбое `places_subtree_stats`.
- Пункты human-check обеих задач не пройдены.

## Known Stubs
None.

## Threat Flags
None. T-41-25-01: в ReportTable нет print-правил, печать идёт из плоских серверных строк (план 17). T-41-25-02: имена групп выводятся Svelte-интерполяцией, `{@html}` нет. T-41-25-03: пакет без заголовка получает синтезированный, строки не теряются. T-41-25-04: `notifyPlaceContentChanged` сохранён, check-place-tree-invalidation зелёный.

## Self-Check: PASSED
Три файла изменены, коммиты 5927a2d9 и c2c31640 существуют.
