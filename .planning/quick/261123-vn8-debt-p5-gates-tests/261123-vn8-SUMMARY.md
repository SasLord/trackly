---
phase: quick-261123-vn8
plan: 01
status: complete
subsystem: tests-gates
tags: [debt-v1.4, P5, gates, mutation]
requirements: [DEBT-P5]
commits:
  - bbc1eccf  # Task 1 + WR-12
  - a45134e0  # Task 2 (Rust-гейты и тесты)
  - e6b03e92  # Task 2/3 (UI-гейты)
completed: 2026-11-23
---

# Quick 261123-vn8: пакет П5 «Гейты и тесты» (долг v1.4)

Продуктовый код П1-П4 не менялся. Единственная правка продуктового ресурса — по прямому указанию:
`<title>` в `templates/act_handover.html` (WR-12), с версионированием шаблона.

## Статус по строкам

| id | Статус | Что сделано |
|----|--------|-------------|
| users_crud FLAKY | исправлено | `TEST_BUDGET = 120 с` одна const на все 9 обёрток `timeout`; защита от зависания сохранена. Суита `app_auth`: 138 passed. |
| гонка TRACKLY_TEMPLATES_DIR | исправлено | Гонка была достижима: `template_service` (tokio Mutex) и `html_templates` (std Mutex) держали РАЗНЫЕ замки на одну переменную окружения в одном lib-бинарнике. Теперь один `pub(crate) TEMPLATES_ENV_GUARD` (tokio, без отравления; синхронные тесты берут `blocking_lock()`). Внешние `tests/template_edit.rs`, `templates_status.rs` — отдельные процессы, свои замки. |
| IN-01 (комментарий html_templates ~524) | исправлено | Комментарий говорил «только act_* имеют v22-снимок / report.html из двух элементов»; фактически `report.html` теперь v20,v21,v28. Переписан: индекс 2 = «третий элемент слайса». |
| 40.4 WR-12 (`<title>`) | исправлено | `{% if act.number %}` -> `{% if act.number_display %}`, новых переменных нет. Снимок `_legacy_defaults/v31/act_handover.html` (тело до правки), регистрация последним элементом в `KNOWN_LEGACY_DEFAULTS`, тест `upgrade_replaces_v31_act_handover_title_guard_body` (до-фикс и после-фикс различаются по замороженному снимку; upgrade v31 -> текущее). `app_acts` 346, `template_edit`, `templates_status` зелёные. |
| текст причины «Расположение» | исправлено | `check-place-path-short.mjs` стр. 135 -> «Место»; логика не тронута. |
| B-WR-01 | исправлено | `check-act-group-picker`: правило (C) требует `const free = ... MAX_ACT_ITEMS`, отказ `if (fresh.length > free) {...}` с `return;` и `pushToast`, порядок free < отказ < первый `onChange(`. Мутанты 13 (блок удалён) и 14 (убран `return;`). |
| B-WR-02 | исправлено | `check-act-group-confirm`: `confirmedItemsSignature = itemsSignature()` ДО `handleSubmit(` в `confirmGroupRelease` (порядок), `itemsSignature` читает items/device_id/quantity/group_ids и начинается с `return JSON.stringify(`. Мутанты 15 (строка удалена), 16 (ПЕРЕСТАНОВКА), 17 (константа), 18 (без quantity). Якоря по идентификаторам, не по номерам строк. |
| B-WR-05 | исправлено | `mutate_once` (assert matches==1) для мутанта режима в undo; для `WRITTEN_OFF_GUARD` (3 вхождения по природе) — `mutate_nth`, гасятся все три по очереди (строже прежнего «первого»). |
| B-WR-06 | исправлено | `group_write_sites`: `rejected>=3 && accepted>=5` -> точно (3, 6) и равенство счётам таблицы. `group_movements_journal`: `>=7` -> `total == 8`, `batched == 7`, `COUNT(DISTINCT entity)` == batched (значения сняты зондом по настоящему прогону: 6 устройств + 1 группа с batch, 1 вне групп). |
| B-WR-03 | перенесено | Продуктовая логика счётчиков вкладок (`PlaceContents`/сервер `member_count`) — П2/П3, не гейт. |
| B-WR-04 | перенесено | Контракт DTO (подтверждение — булево; нужен план/хэш в `GroupReleaseWarningDto`) — изменение продукта и bindings. |
| B-IN-01 | исправлено | `acts_group_plan_order_guard.rs` (вне перечня плана, но тестовый): первая запись = минимум по `WRITE_MARKERS` и по ЛЮБОМУ `*_in_tx(` вне явного списка читателей (`get_/fetch_/list_/plan_/ensure_` + 12 имён с причиной, `snapshot_keeping_*`). Реальный исходник зелёный; мутант 15 (неизвестный писатель выше `plan_in_tx`) красный, читатель с префиксом `get_` — нет; тест, что WRITE_MARKERS не в освобождениях и что освобождения не устарели. |
| B-IN-02 | исправлено | Добавлен мутант `update_return_with_split` (`if true {`) в `write_site_gate_act_release_is_move_aware`. Хвост «условие с подстрокой moved_device_ids, но инвертированное» — остаётся за поведенческими тестами (по самому ревью). |
| B-IN-03 | закрыто ранее | jcg (A-WR-01-пакет), не тронуто. |
| B-IN-04 | перенесено | Молчаливый `[]` источников пикера — в `ActFormItemsTable.svelte` (продукт). |
| B-IN-05 | исправлено | `check-act-group-picker`: сверка `MAX_ACT_ITEMS` с серверными `p.items.len() > N` (>=2 потолков, все равны); мутант 15. Правило счёта (клиент считает единицы, как `update`; `create` — строки) описано в doc функции. |
| B-IN-06 | исправлено | `strip_line_comment` (учёт строковых и символьных литералов) в `count_calls`/`code_of`/`code_of_phase_412`; `writes_place_id_in_sql` не зависит от регистра, `OR IGNORE/REPLACE`, `INTO`, продолжения строки; `devices_archive` не считается. Тест `text_scanning_helpers_edges`. |
| W4 (39, acts_suggest) | исправлено | Продукт ключ `location` не читает (реальный `op_payload_json` пишет `place_id`); фикстура приведена к реальной форме. |
| IN-03 (39.1, cartridges_search) | исправлено | Тест `search_returns_place_path_short_for_placed_cartridge`: трёхсегментный путь + `last` на листе, точное значение «Кабинет 12», у картриджа без места `None`. |
| IN-05 (41.10, group-vocabulary) | закрыто ранее + доказано | Гейт и так инвентарь от файла (ROOT_RE вне markers краснит). Добавлены 3 фикстуры по `historyEvents.ts` (две красные, одна зелёная) — раньше покрытия не было. |
| WR-14 (40.3) | исправлено | `check-act-number-no-regex-split`: `stripComments` до матча, обратная косая обязательна, lookbehind против деления. Строки НЕ вырезаются сознательно: `row['number_raw']` — настоящее употребление. 5 новых фикстур, в т.ч. номер строки после вырезания. |
| IN-07 (40.3, «асимметрия PRAGMA») | перенесено (не требует правки) | Это комментарий к `migrations.rs::run_to` (ветки `pending`/`else` по `foreign_keys`), по самому ревью «No fix required»; к гейту `check-act-number-no-regex-split` отношения не имеет, файл — продукт. |
| TD-2 (41.11) | исправлено | `check-device-list-mode`: счётчики `h7Caught == h7Mutants.length` (10/10) и зелёных форм (2/2) сверяются и печатаются. |
| TD-5 (41.11) | исправлено | Правило [D1] живёт в `check-device-form-quantity-gate.mjs`, не в `check-device-list-mode` (плановая отсылка неточна). Добавлены конъюнкты `!readonly`, `!lockedByGroup`, `isStoragePlace`, `storageStatusSuggested`, `storageId !== null`; 6 мутантов по реальному `DeviceFormBody.svelte` (инверсия каждого + дизъюнкт). |
| IN-02 (41.12) | исправлено | `check-device-type-labels`: T4 требует вызов `ensureLoaded` (в т.ч. псевдоним) у каждого потребителя стора; 4 новых случая (в т.ч. мутант по реальному `GroupAddDevicesModal`). |
| IN-05 (41.12, ROW_CELL) | исправлено | Файл — `check-place-path-short.mjs` (не `check-device-type-labels`, как в формулировке задачи): INV-5, ячейка `<td>` с full и без short в файле с вердиктом ROW_CELL краснит; реальное дерево чисто. |
| IN-01 (frontend 41, check-reorder) | исправлено | selftest мутирует реальный `reorder.ts` в памяти (якорь ровно 1, счёт 3/3), встроенный эталон удалён. |

## Мутационные доказательства (прежняя версия гейта зелёная на мутанте)

| Гейт | Мутант | Прежняя | Новая |
|------|--------|---------|-------|
| check-act-group-picker | удалён блок `if (fresh.length > free)` | PASS 0 | FAIL (C) |
| check-act-group-confirm | убрана `confirmedItemsSignature = itemsSignature()` | PASS 0 | FAIL 1 |
| check-act-group-confirm | `itemsSignature` возвращает `''` | PASS 0 | FAIL 1 |
| check-device-form-quantity-gate | `!lockedByGroup` -> `lockedByGroup` | PASS 0 | FAIL 1 |
| check-place-path-short | `<td>{row.full_path}</td>` в DeviceListRow | PASS 0 | FAIL (INV-5) |
| check-act-number-no-regex-split | `/^data:/` (ложное срабатывание) | FAIL 1 | PASS 0 (истинный дефект `/^\d+/` по-прежнему FAIL) |
| check-reorder (selftest) | реальный reorder.ts с изменённой формой якоря | selftest OK (не смотрел реальный) | SELFTEST FAIL (якорь 0 раз) |

Для Rust-гейтов доказательство встроено в сами тесты: `mutate_once` ассертит уникальность, мутант с неизвестным писателем и `update_return` красят гейт, `text_scanning_helpers_edges` фиксирует прежнюю дыру (`//` в литерале).
Все якоря мутантов уникальны (`mutate()` валится при ином счёте).

## Проверки (вывод)

- `cargo fmt --check` — чисто; `cargo clippy -p trackly-app --all-targets -- -D warnings` — чисто.
- `cargo test -p trackly-app --lib` — 447 passed.
- Суиты (TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1, по одной): `app_gates` 291 passed/1 ignored; `app_acts` 346; `app_groups` 201; `app_cartridges` 89; `app_auth` 138; `template_edit` 6; `templates_status` 3.
- `--selftest` каждого тронутого гейта зелёный: picker 15 мутантов, confirm 19, number-no-regex-split 12 фикстур, quantity-gate 36 фикстур + 8 по реальному файлу, list-mode (H7 10/10), type-labels 44, group-vocabulary 61, place-path-short 26, reorder 3/3.
- `pnpm --dir ui lint` ПОСЛЕ последней правки (после `prettier --write` пяти скриптов) — зелёный целиком, `check-lint-selftest-wiring`: 72 гейта подключены. `svelte-check` не гонялся (.svelte не менялись).
- Сплошной прогон cargo не запускался.
- `check-privacy` на двух коммитах — PASS; в тестах только вымышленные данные.

## Отклонения от плана

1. [Rule 3] `acts_group_plan_order_guard.rs` (B-IN-01) и `check-device-form-quantity-gate.mjs` (TD-5), `check-place-path-short.mjs` (IN-05 ROW_CELL, текст причины) вне перечня `files_modified`, но это те же гейты по существу строк П5; граница П1-П4 (продуктовые файлы, `history_write_sites_gate.rs`, `role_endpoint_matrix.rs`, `check-history-timeline-refresh.mjs`) не нарушена.
2. WR-12 — единственная правка продуктового шаблона (по указанию оркестратора); механизм версий уложился в S.
3. TD-5/IN-05 в задаче были приписаны не тем файлам — сделаны в реальных.

## Перенесено

- B-WR-03 (счётчики вкладок: двойной учёт «забредшего» члена) — П2/П3, продукт.
- B-WR-04 (подтверждение вывода привязано к булеву, не к плану) — контракт DTO.
- B-IN-04 (пикер молча глотает два источника) — `ActFormItemsTable.svelte`.
- IN-07 (40.3, комментарий `run_to` про FK pragma) — комментарий продукта, по ревью правки не требует.
- B-IN-02, остаток (инвертированный `if` с подстрокой `moved_device_ids`) — держат поведенческие тесты.

## Self-Check: PASSED

Коммиты bbc1eccf, a45134e0, e6b03e92 найдены в `git log`; `_legacy_defaults/v31/act_handover.html` существует.
