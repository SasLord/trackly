---
phase: quick-261123-vn3
plan: 01
status: complete
requirements: [HST-02, HST-05, HST-06]
commits:
  - a2a2903c  # fix: R8-03, R8-05, 41.2 WR-04, IN-03
  - fb6af182  # test: IN-02 паритет видов, гейт parent_group_id
  - 1b1df627  # refactor: R8-02 мёртвый эндпоинт, WARNING-01 (40)
---

# Quick 261123-vn3: долг v1.4, пакет П1 «История и группы — бэкенд»

Миграций нет. Файлы П2–П5 не затронуты (кроме оговорённых ниже отклонений по границам).

## Итог по id

| id | вердикт | коммит / тест |
|---|---|---|
| R8-03 | исправлено | a2a2903c. Висячая ссылка достижима, когда позицию акта исключают повторно (исключили с настоящим изменением, вернули, исключили снова): строка возврата с `act_id` забывается, а оставшийся `group_left` держит её `movement_id`. Теперь `forget_excluded_device_movements_in_tx` обнуляет висячие ссылки событий акта по устройству (`clear_dangling_movement_refs_by_act_subject_in_tx`). Тест `history_act_edit_reexclude_leaves_no_dangling_group_left_movement` был красным (1 висячая ссылка), стал зелёным; в `skipped_rejoin` добавлена проверка того же инварианта (была зелёной — из первой правки висячей ссылки нет). |
| R8-05 = IN-01 | исправлено (путь списания через группу); хвост П2 | a2a2903c. Новый вариант `ReleaseHistory::ReasonInOperation` (существующий `Reason` не менялся, обратно совместим); `group_service::write_off_device` передаёт `wo_operation_id`. Утверждение `operation_id` совпадает в `history_status_write_off_device` было красным, стало зелёным. Хвост: `device_service.rs:1640` (выбор «вывести» при списании через форму устройства) — файл П2, остался на `Reason` и пишет `group_left` со своим operation_id. Для П2: заменить на `ReasonInOperation`, передав id операции со статусом. Путь `write_off_group` менять не нужно: у участников там только `group_left`, статуса нет. |
| R8-02 = IN-03 (41.10) | исправлено | 1b1df627. Вызовов вне `bindings.ts` не было. Удалены маршрут, Tauri-команда, регистрация в specta, оба файла (`http/place_movements.rs`, `tauri_cmds/place_movements.rs`) и `mod`/`merge`, кейсы 52/53 и строка в описи маршрутов матрицы. `PlaceMovementService::get_timeline` оставлен. `bindings.ts` в `.gitignore`: регенерирован штатным `export_bindings`, совпадает с ручной правкой; в коммит не входит. |
| IN-02 (41.10) | исправлено | fb6af182. Дубль убран: `event_segment_kind` выводится из `HistoryEventKind` исчерпывающим `match` без `_` (новый вид не скомпилируется без назначения сегмента). Модульный тест + гейт `history_write_sites_event_kinds_not_duplicated_in_history_service`. |
| слепая зона parent_group_id | исправлено | fb6af182. `writes_parent_group_id` (нормализация пробелов/`\`-переносов/регистра, учитывается только часть до `WHERE`) в `history_write_sites_raw_sql_stays_in_repositories` + selftest. Легитимных нарушителей в репозитории нет. |
| gap-wave IN-02 (41, DEFERRED-транзакции) | отклонено | Само ревью: недостижимо (единственный писатель), `transaction_with_behavior` нигде в кодовой базе не используется, «можно оставить как есть». Единообразие важнее; точечная правка одного сервиса создала бы расхождение. |
| gap-wave IN-03 (41, двойная точка) | исправлено | a2a2903c. Находка живёт в `tauri_cmds/reports.rs` (не в `group_type_service.rs`, как в доке): `join_summary_notice` с `trim_end_matches('.')`, юнит-тест. Отклонение от границ плана: файл вне списка, но П2–П5 его не планируют. Красной стадии не было (хелпер новый). |
| 41.2 WR-04 | исправлено | a2a2903c. Забывание переноса удаляет преемника, у которого после переписывания `from_place_id = to_place_id`. Тест `forget_act_entity_rows_drops_same_place_successor` был красным (`[(1,1,None)]`), стал зелёным. Остаточный риск: у удалённого преемника может остаться событие с его `movement_id` (источник — другой акт/ручное); лента такое событие показывает без привязки, ошибки нет. |
| 41.2 WR-01 | перенесено | Подтверждено по коду: `rebase_later_move_effects_in_tx` не исключает эффекты того же акта, а цепочка строк одного акта не сворачивается. Исправление требует нового параметра `act_id`, правки `revert_move_effect_in_tx` в `group_act_plan.rs` (вне границ) и фикстуры «два `move` одного акта + перенос после»; оценка M в доке. Не делалось, чтобы не оставить полуисправление. |
| WARNING-01 (40) | исправлено | 1b1df627. Кейсы 63a/63b: Tauri-путь `build_cartridges_operation_default_place` и `build_cartridges_to_refill_last_send` (Employee → Forbidden, Manager → не Forbidden). Зелёные сразу (тест-только). |

## Проверки (вывод)

- `cargo test -p trackly-app --test app_acts` — 344 passed, 0 failed
- `cargo test -p trackly-app --test app_groups` — 201 passed
- `cargo test -p trackly-app --test app_gates` — 290 passed, 1 ignored
- `cargo test -p trackly-app --test app_places` — 91 passed
- `cargo test -p trackly-app --test app_cartridges` — 88 passed
- `cargo test -p trackly-app --test app_auth role_endpoint_matrix` — 35 passed
- `cargo test -p trackly-infra --test infra_tests place_movements` — 23 passed
- `cargo test -p trackly-app --lib` (join_summary, segment_kind) — passed
- `cargo test -p trackly-app --test export_bindings` — passed
- `cargo clippy -p trackly-app -p trackly-infra --all-targets -- -D warnings` — чисто
- `cargo fmt --check -p trackly-app -p trackly-infra` — чисто
- `pnpm --dir ui lint` — зелёный (71 гейт); `svelte-check` — 0 ERRORS, 75 WARNINGS (прежние, не из этого пакета)
- check-privacy в pre-commit — PASS 0 нарушений на всех трёх коммитах. Фикстуры вымышленные.
- Целевые тесты файлов `history_act_edit`/`group_movements_journal`/`place_movements_repo` из плана идут через suite-бинарники (`app_acts`, `app_groups`, `infra_tests`), отдельных `--test`-целей нет.
- Полный `infra_tests` и сплошной прогон не запускались.

## Мутационные проверки (откачены, в коммитах нет)

1. В конец `group_type_service.rs` добавлен литерал `"UPDATE groups SET\ parent_group_id = NULL WHERE id = 1"` — `history_write_sites_raw_sql_stays_in_repositories` красный («прямой UPDATE groups ... parent_group_id вне groups_sqlite.rs»). Якорь уникален (дописывание в конец файла).
2. В конец `history_service.rs` добавлен литерал `"group_joined"` — `history_write_sites_event_kinds_not_duplicated_in_history_service` красный.
Файлы восстановлены из копий; `git status` после — без мутаций.

## Живая проверка для общего чек-листа (не выполнялась)

1. Устройство в группе, статус «на списание»: списать устройство с выбором «вывести из группы» через «Группы» (списание участника) — в «Истории» устройства одна запись операции (статус + вывод), не две.
2. Акт передачи с устройством из группы; правка акта: исключить устройство, ввести его вручную в группу на другом месте, исключить ещё раз после повторного добавления — лента устройства без пустых/пропавших сегментов, ошибок нет.
3. Отчёт «Перемещения», усечённый до лимита, с типом/местом, чьё имя кончается точкой, — печать без «..» в строке фильтров.
4. Акт перенёс устройство А→Б, затем ручное Б→А; удалить акт — в журнале мест нет строки «А → А».

## Отклонения от границ плана

- `group_service.rs` (списание устройства: передача operation_id) и `tauri_cmds/reports.rs` (IN-03) — вне списка файлов плана; изменения минимальны, другие пакеты их не планируют.
- `tests/history_status_groups.rs` — добавлено утверждение вместо нового теста в `group_movements_journal.rs`.

## Self-Check: PASSED
Коммиты a2a2903c, fb6af182, 1b1df627 существуют; удалённые файлы отсутствуют; миграций нет.
