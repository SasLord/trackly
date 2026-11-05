---
status: diagnosed
trigger: "uat-41.2-act-delete-membership-restore: удаление акта не возвращает устройства в контейнерную группу (группа без места; группа перенесена после акта)"
created: 2026-11-05
updated: 2026-11-05
---

## Current Focus

hypothesis: CONFIRMED. Правило (c) в restore_membership_in_tx сравнивает `group.place_id != restored_place_id` как Option целиком; у группы без места (None) и устройства со своим местом (Some) — ложное «группа теперь в другом месте».
test: чтение кода + сверка с инвариантом D-21 фазы 41 (группа без места не запирает место членов)
expecting: n/a
next_action: вернуть диагноз оркестратору (find_root_cause_only)

## Symptoms

expected: (1) Группа «АРМ #3» без места; актом выведены 2 устройства; акт удалён -> устройства возвращаются в «АРМ #3». (2) Группа с местом перенесена после акта -> вернуть в группу или спросить пользователя (вернуть в группу на новом месте / оставить вне группы на прежнем месте).
actual: (1) Не вернулись; жёлтый тост «"Принтер #1" не вернулся в «АРМ #3»: группа теперь в другом месте.» при place_id группы = NULL. (2) Жёлтые тосты, членство не возвращено (текущее поведение по D-09/правилу (c)).
errors: нет; ложная причина в тосте для (1)
reproduction: тесты 3 и 4(c) в .planning/phases/41.2-groups-acts-places/41.2-UAT.md
started: найдено на UAT 2026-11-05

## Eliminated

- hypothesis: SQL `=` с NULL в запросе места группы
  evidence: сравнение делается в Rust (`Option<i64> != Option<i64>`), не в SQL; SQL-запросы get_group_in_tx/devices.place_id читают значения корректно
  timestamp: 2026-11-05

## Evidence

- timestamp: 2026-11-05
  checked: crates/trackly-app/src/services/group_membership.rs:245-247 (restore_membership_in_tx, правило (c))
  found: `if group.place_id != restored_place_id { skipped("группа теперь в другом месте.") }`; док-комментарий явно требует сравнивать Option целиком, «включая None против Some»
  implication: для группы без места (None) любое устройство со своим местом (Some) отбрасывается

- timestamp: 2026-11-05
  checked: crates/trackly-infra/src/repos/groups_sqlite.rs:334-349 (locked_group_for_device_in_tx) и group_membership.rs release_if_locked_device_in_tx
  found: «запирающая» группа = `g.place_id IS NOT NULL`; группа без места не запирает место членов (D-21 ф.41), члены живут на собственных местах
  implication: у членов группы без места place_id обычно Some(X); после отката снимка restored_place_id = Some(X) != None -> ложный пропуск. Обоснование в комментарии (группа Some против устройства None) применимо только к одной стороне; случай «группа None» туда попал ошибочно

- timestamp: 2026-11-05
  checked: crates/trackly-app/tests/acts_groups.rs:814-817 и group_membership.rs unit-тесты restore_*
  found: вынос части состава группы без места актом проходит и пишет эффект release; тестов отката для группы без места (group None, device Some) нет; unit-тест restore_moved_group_is_skipped проверяет только Some(1) vs Some(2)
  implication: дефект не покрыт тестами

- timestamp: 2026-11-05
  checked: group_act_plan.rs:1028-1088 (restore_released_members_in_tx) и act_service.rs:4469-4590 (undo_device_mutations_for_act)
  found: порядок отката: (1) revert_group_moves, (2) восстановление снимков устройств (+ release_if_locked Skip), (3) restore_released_members_in_tx читает ТЕКУЩЕЕ место устройства (уже из снимка, т.е. место до акта) и передаёт в правило (c), (4) удаление эффектов. Всё — в одной транзакции, без предварительного опроса пользователя. Эффект release (group_membership.rs:113-125) пишет from_place_id: None — место группы на момент акта не запоминается
  implication: случай (2): группа перенесена A->C, устройство восстановлено на A, правило (c) Some(C) != Some(A) -> пропуск. Так спроектировано (D-09/правило (c)). Чтобы вернуть в группу, надо после правила (b) переносить устройство на group.place_id (с записью журнала места) вместо пропуска; для выбора пользователем нужен предварительный read-only расчёт конфликтов + параметр решения в удалении акта

## Resolution

root_cause: restore_membership_in_tx (group_membership.rs:245) применяет правило (c) «место группы == место восстановленного устройства» и к группам без места. Группа без места не запирает места членов (locked_group = place_id IS NOT NULL), поэтому её члены имеют собственные места; после отката снимка устройство стоит на Some(X), группа на None -> ложный Skipped «группа теперь в другом месте». Случай (2) — не дефект кода, а правило (c)/D-09: при расхождении Some(C) != Some(A) членство не возвращается.
fix: (не применялся — режим диагностики)
verification:
files_changed: []
