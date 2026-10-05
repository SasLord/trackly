---
phase: 41-groups-model-and-editor
plan: 33
subsystem: ui-reports
tags: [reports, truncation, gate, golden-fixture, gap-closure]
requires: ["41-30", "41-31", "41-32"]
provides:
  - "movementsTruncationNotice(shown, total) — JS-зеркало серверной movements_truncation_notice"
  - "check-report-truncation.mjs — исполняющий гейт по общей фикстуре + структурные правила S1-S3 + selftest M1-M8"
  - "баннер усечения над таблицей отчёта «Перемещения»"
affects: [ui/src/features/reports, ui/package.json]
tech-stack:
  added: []
  patterns: ["golden-фикстура на обе стороны (Rust + JS)", "selftest с мутантами и проверкой единственности якоря"]
key-files:
  created:
    - ui/src/features/reports/truncationNotice.ts
    - ui/scripts/check-report-truncation.mjs
  modified:
    - ui/src/features/reports/ReportsPage.svelte
    - ui/package.json
key-decisions:
  - "S3 проверяет оба аргумента вызова: второй обязан кончаться на `.total`, первый — на `.length` без `.total`; так ловятся и чтение не того поля, и перестановка аргументов"
  - "Баннер без собственных @media print правил: на бумагу идёт только серверная строка (план 41-30)"
requirements-completed: [GRP-06]
duration: ~15 мин
completed: 2026-10-05
---

# Фаза 41 План 33: баннер усечения отчёта «Перемещения» и гейт Summary

Экран отчёта «Перемещения» сообщает об усечении фразой, посимвольно совпадающей с серверной (печать и CSV), текст закреплён фикстурой `report-truncation/cases.json`, которую читают и Rust, и JS-гейт в `pnpm lint`.

## Коммиты

- `624ec7ba` — `truncationNotice.ts` + `check-report-truncation.mjs` (задача 1)
- `b7b56381` — баннер в `ReportsPage.svelte`, регистрация гейта в `ui/package.json` (задача 2)

## Что сделано

- `movementsTruncationNotice(shown, total)`: `null` при `total <= shown`, иначе «Показано записей: N из M (самые ранние). Сузьте период или фильтры, чтобы увидеть остальные.». Ни одного import.
- Гейт читает фикстуру С ДИСКА (`ui/scripts/fixtures/report-truncation/cases.json`, 7 кейсов), фикстура НЕ редактировалась. Структурные правила: S1 (импорт), S2 (нет литерала «Показано записей» в странице), S3 (вызов с `.length` первым и `.total` вторым аргументом).
- `ReportsPage.svelte`: `const truncationNotice = $derived(reportTypeKey() === 'movements' && rows ? movementsTruncationNotice(rows.rows.length, rows.total) : null)`; баннер `<div class="truncation-notice" role="status">` между `.controls-row` и `ReportTable`, `flex-shrink: 0`, только токены `--tr-*`, без `@html`, без `@media print`, без `$effect`.
- Гейт дописан в конец цепочки `lint` после `check-property-removal.mjs`.

## Красные сценарии (доказательство, что гейт не вакуумный)

Задача 1, до подключения баннера: `node scripts/check-report-truncation.mjs` код 1 — S1 (нет импорта) и S3 (нет вызова).

Selftest: эталон проходит, 8 мутантов ловятся (M1 всегда null, M2 `<` вместо `<=`, M3 «поздние», M4 числа переставлены, M5 литерал в странице, M6 `rows.rows.length` вместо `rows.total`, M7 аргументы переставлены, M8 убран импорт). Якорь каждого мутанта проверяется на единственность.

На реальных файлах (после коммита, откат `git checkout -- <файл>`, дерево чисто):

| Сценарий | Мутация | Код | Поймал |
|---|---|---|---|
| m1 | вызов в ReportsPage заменён на `null` | 1 | S3 |
| m1b | `rows.total` -> `rows.rows.length` (чтение не того поля) | 1 | S3 |
| m2 | литерал «Показано записей» в `.svelte` | 1 | S2 |
| m3 | `<=` -> `<` в truncationNotice.ts | 1 | кейсы 3/3, 1000/1000, 0/0, 999/999 |
| m4 | «самые ранние» -> «самые поздние» | 1 | кейсы усечения 1000/1005, 1000/1001, ... |

После отката: код 0.

Честная оценка дискриминирующей силы: четыре «без усечения» кейса различают `<=` и `<`; три кейса усечения различают текст и порядок чисел. Кейс 1000/1000 не отличает «потолок захардкожен как 1000» от `total <= shown` (мутант `total <= 1000` фикстурой НЕ ловится — все 7 кейсов проходят). Это остаточная слабость самой фикстуры (по условию задачи не правилась); на практике хардкод потолка в зеркале серверной функции не вводился.

## Гейты

- `pnpm --dir ui svelte-check` — 0 ошибок (68 предупреждений, ни одного в ReportsPage/truncationNotice, все ранее существовавшие)
- `pnpm --dir ui lint` — код 0 (включая check-report-type-parity, check-movements-type-filter, check-print-isolation, check-tokens, prettier, новый гейт)
- `pnpm --dir ui build` — код 0, `ui/dist` свежий для плана 41-34
- cargo test не запускался (фронтенд-план); `prebuild` прогнал только `export_bindings`.

## Отклонения от плана

Нет — план выполнен как написан. Расширение сверх плана: гейт S3 строже плановой формулировки (проверяет и первый аргумент, ловит перестановку), мутантов 8 вместо 5.

## НЕ ПРОВЕРЕНО (живая проверка, требует запущенного приложения и людей с данными)

Компиляционные гейты слепы к вёрстке и рантайму. Ничего из ниже в работающем приложении НЕ запускалось. Передать пользователю (в 41-HUMAN-UAT.md как открытые пункты):

Подготовка: КОПИЯ dev-БД; вставить 1005 строк в `place_movements` рекурсивным CTE `INSERT … SELECT` (как в тесте `report_trunc_total_counts_rows_beyond_limit` плана 41-30; вымышленные данные, скрипт в репозиторий не класть). `cargo tauri dev` с `main` + LAN-браузер (после `pnpm --dir ui build`, он выполнен).

Оба транспорта (десктоп Tauri и LAN-браузер), экран «Отчёты» (`ReportsPage`) -> «Перемещения» за период:
1. Над таблицей жёлтый баннер «Показано записей: 1000 из 1005 (самые ранние). …»; значок вкладки `ReportSubNav` показывает 1005.
2. После сужения периода/фильтра до ≤ 1000 строк баннер исчезает; на других отчётах баннера нет.
3. Таблица `ReportTable` остаётся единственным скролл-регионом, второго скролла нет (`.content` оболочки `overflow: hidden`).
4. «Печать» из десктопа и из LAN-браузера: под периодом серверная строка уведомления; ЭКРАННЫЙ жёлтый баннер на бумагу НЕ попадает ни в десктопной, ни в LAN-печати (иначе дублирование) — LAN-печать верстается прямо в DOM приложения.
5. CSV: последняя строка содержит то же уведомление.
6. Консоль без `effect_update_depth_exceeded`.

## Known Stubs

Нет.

## Threat Flags

Нет новых поверхностей (текст баннера — интерполяция Svelte, без `{@html}`).

## Self-Check: PASSED

- ui/src/features/reports/truncationNotice.ts — найден
- ui/scripts/check-report-truncation.mjs — найден
- коммиты 624ec7ba, b7b56381 — найдены
