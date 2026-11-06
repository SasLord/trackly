---
status: awaiting_human_verify
trigger: "G8 (41.2 HUMAN-UAT): частичный возврат разбил группу (G6, вероятно MoveGroup), затем возврат отредактировали до полной группы; акт ушёл в архив (верно), но группа не восстановилась: устройство, выведенное первой версией возврата, в группу не вернулось"
created: 2026-11-07
updated: 2026-11-07
---

## Current Focus

reasoning_checkpoint:
  hypothesis: "Шаг 10 update_return_with_split планирует добавленные позиции по ТЕКУЩЕМУ составу групп. Члены C,D, выведенные первой версией возврата (MoveGroup: release-эффект этого акта), уже вне группы, планировщик видит их как свободные устройства; эффект release этого акта по ним никто не читает, членство не восстанавливается"
  confirming_evidence:
    - "тест update_return_after_move_group_split_to_whole_group_restores_membership: после правки group_devices по группе = 2, ожидалось 4"
    - "в шаге 10 нет ни одного чтения эффектов release для added; prior_release читается только для removed (шаг 9)"
  falsification_test: "если после восстановления членства по release-эффекту этого акта (с переносом на место группы) тест остаётся красным — гипотеза неверна"
  fix_rationale: "до плана добавленных вернуть в группу тех added, по которым есть release-эффект ЭТОГО акта и группа перенесена этим же актом (move-эффект), снять эффект, поставить устройство на место группы и исключить его из плана — ровно как если бы первая версия возврата включала его"
  blind_spots: "якорные группы (деталь, выведенная при убирании позиции) — та же ветка, отдельного теста нет; правка, одновременно меняющая место удержанных членов (шаг 11)"
next_action: живая проверка G8 пользователем (MoveGroup); решение по открытому вопросу Release

## Symptoms

expected: правка возврата до полной группы делает группу целой: члены, выведенные этим же актом (split), возвращаются в состав, группа целиком на месте возврата (D-44); выведенных не этим актом не трогать
actual: акт уходит в архив, но член, выведенный первой версией возврата, остаётся вне группы
errors: нет
reproduction: G6 (2 из 4, «Изменить расположение группы») -> правка возврата, добавить оставшиеся 2 -> сохранить
started: найдено на живой UAT 2026-11-07

## Eliminated

- hypothesis: вариант Release ломается тем же образом
  evidence: тест update_return_after_release_split_to_whole_group_moves_rest_of_group зелёный до фикса: оставшиеся C,D покрывают группу, D-44 переносит её на место возврата; A,B вне группы по выбору «Вывести из группы» — вопрос трактовки, не дефект кода
  timestamp: 2026-11-07

## Evidence

- timestamp: 2026-11-07
  checked: .planning/debug/knowledge-base.md
  found: совпадений по ключевым словам нет
  implication: известного паттерна нет

- timestamp: 2026-11-07
  checked: act_service.rs update_return_with_split шаг 10 + group_act_plan.rs apply_split_move_in_tx
  found: MoveGroup пишет release-эффект этого акта для kept-членов и move-эффект группы. Шаг 10 строит plan_for_return_in_tx(added) по текущему составу; C,D вне групп -> план пуст, цикл ставит их на место возврата как одиночек; release-эффекты C,D остаются висеть
  implication: правка до полной группы не «переписывает» вывод, сделанный этим же актом

- timestamp: 2026-11-07
  checked: красный прогон acts_groups (новые тесты)
  found: move_group-вариант: group_members=2 (ожидалось 4); контроль «выведен не этим актом»: выведенный актом не вернулся (0); release-вариант зелёный
  implication: дефект воспроизведён для MoveGroup

## Resolution

root_cause: "update_return_with_split, шаг 10: план добавленных позиций строится по текущему составу групп и не учитывает следы release ЭТОГО акта. Члены, выведенные первой версией возврата при MoveGroup (kept-члены, release-эффект + move-эффект группы), для правки — свободные устройства: они встают на место возврата одиночками, членство не восстанавливается, следы release висят"
fix: "group_act_plan::rejoin_released_by_act_in_tx: до плана добавленных возвращает в группу added-устройства со следом release этого акта (прочитан до правки, effects_before) при живом move-эффекте этого акта по группе; снимает след, ставит устройство на место группы; D-46: места вернувшихся и удержанных позиций группы обязаны совпасть (иначе Validation place_id до записи). В act_service: такие устройства исключены из плана добавленных, считаются group_placed (не выводятся из группы), в шаге 11 едут с группой как удержанные"
verification: "acts_groups: 4 новых MoveGroup-теста красные до фикса (stash src), зелёные после; release-вариант зелёный до и после (поведение не менялось); регресс: acts_groups 78/0, acts_update_return 20/0, acts_returns 20/0, acts_return_split_http 6/0, group_write_sites 34/0, entities_changed_acts 13/0, acts_group_plan_order_guard 17/0, acts_delete_choice_http 3/0, --lib group_act_plan:: 47/0, entities_changed_gate 45/4 (4 известных по device_service.rs); fmt и clippy -D warnings чистые"

release_decision: "D-49 (пользователь, 2026-11-07): вариант 2. Правка до полной группы отменяет и выбор Release. Реализовано group_act_plan::rejoin_released_retained_for_whole_group_in_tx: удержанные позиции со следом release ЭТОГО акта (effects_before) возвращаются в группу, если каждый юнит текущего состава группы есть в позициях правки; затем идут в план добавленных вместе с новыми, группа покрыта целиком и едет на место возврата (D-44), места проверяет план (D-46). Правка, не делающая возврат полным, ничего не меняет. Тест update_return_after_release_split_to_whole_group_moves_rest_of_group переименован в ..._restores_membership; красный на 91753cf1 (группа 2 вместо 4), зелёный после. Добавлены контроли: not_whole_keeps_released_members_out, skips_member_without_this_acts_trace, with_different_places_is_rejected; якорная деталь: re_adding_removed_part_of_moved_anchor_group_restores_membership (зелёный уже на 91753cf1 — покрыт первым фиксом)."
