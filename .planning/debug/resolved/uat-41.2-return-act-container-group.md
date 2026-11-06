---
status: resolved
trigger: "uat-41.2-return-act-container-group: Возврат актом устройств контейнерной группы, перенесённой ранее актом, опустошает группу. Нужно продуктовое изменение — опишите текущий механизм и точки, где его менять."
created: 2026-11-05T00:00:00Z
updated: 2026-11-05T00:00:00Z
---

## Current Focus

hypothesis: CONFIRMED. `plan_for_return` (group_act_plan.rs:319) по замыслу (D2) переносит только ЯКОРНЫЕ группы; контейнер никогда не попадает в `moves`, поэтому каждый возвращаемый член контейнера идёт в `releases` и выводится из группы, а место получает из формы возврата (по умолчанию первое складское место). У `do_return`/`update_return` нет исхода «подтверждение».
test: чтение кода (cargo test запрещён ограничениями)
expecting: n/a
next_action: вернуть ROOT CAUSE FOUND оркестратору

## Symptoms

expected: Возврат ВСЕЙ группы сохраняет группу (переезжает целиком на место возврата). Частичный возврат, разбивающий группу, показывает попап: «Вывести из группы» / «Изменить расположение группы».
actual: После частичного или полного возврата группа пустая (или без возвращённых устройств); устройства получают места из формы возврата (по умолчанию складские).
errors: None
reproduction: Тест 14 в 41.2-UAT.md. Акт переносит «АРМ #3» целиком в «Кабинет Б»; затем «Вернуть» — все или часть позиций.
started: найдено на UAT 2026-11-05; поведение по замыслу D2/D6 фазы 41.2.

## Eliminated

- hypothesis: D6 (stationary_device_ids) удерживает членов контейнера на месте группы
  evidence: apply_return_plan_in_tx заполняет stationary_device_ids только из plan.moves с anchor_device_id, у которого нет места возврата; контейнер в moves не попадает никогда
  timestamp: 2026-11-05

## Evidence

- timestamp: 2026-11-05
  checked: group_act_plan.rs plan_for_return (319-404)
  found: covered_anchor_groups = только группы с anchor_device_id ∈ returned; контейнеры пропускаются; все прочие возвращаемые члены групп -> releases. Doc-комментарий: «Контейнер не переезжает… Подтверждение для возврата не требуется».
  implication: полный возврат контейнера = N выносов, группа остаётся пустой на месте акта передачи.
- timestamp: 2026-11-05
  checked: group_act_plan.rs apply_return_plan_in_tx (517-576)
  found: цель переноса берётся только по якорю (return_places[anchor]); releases применяются через apply_plan_in_tx с эффектом Release на акт возврата.
  implication: место переноса для контейнера сейчас вычислить негде; нужна новая логика «общее место возвращаемых членов».
- timestamp: 2026-11-05
  checked: act_service.rs do_return (1815+), строки 1929-1935, 2046-2055, 2098-2224
  found: plan_for_return_in_tx считается до insert акта; затем цикл позиций: location_for_device = effective_location (форма), если не group_placed; release_device_in_tx если !group_placed && !is_anchor. Возвращаемый тип Result<ActDto>, нет confirm-флага в ActReturnDto.
  implication: точка вставки подтверждения — сразу после plan_for_return_in_tx (до insert_act_in_tx), как в create (act_service.rs:499-514).
- timestamp: 2026-11-05
  checked: act_service.rs update_return шаги 9/10/11 (2744-3220)
  found: шаг 9 (removed) снимает Move-эффекты по coverage = units_of_group_in_tx — работает и для контейнеров; шаг 10 (added) — plan_for_return_in_tx + apply_return_plan_in_tx; шаг 11 (retained_with_change) — moving_anchors только для якорей.
  implication: при изменении планировщика шаг 9 уже готов, шаги 10 и 11 нужно расширить на контейнеры.
- timestamp: 2026-11-05
  checked: undo_device_mutations_for_act (4469+), revert_move_effect_in_tx (858+), anchor_restore_conflict_in_tx (group_membership.rs:282), delete_soft каскад LIFO возвратов
  found: откат обходит эффекты Move/Detach акта (любой группы, не только якорной), возвращает группу на from_place, если она стоит на to_place и корневая; снимки устройств восстанавливаются; anchor_restore_conflict для контейнеров всегда None.
  implication: если возврат пишет Move-эффект контейнера через apply_plan_in_tx, удаление акта возврата вернёт группу в «Кабинет Б» без нового кода; если группу переместили после возврата — уведомление, а снимки членов контейнера восстановятся в «Кабинет Б» мимо группы (расхождение, нужен страж как у якоря).
- timestamp: 2026-11-05
  checked: ReturnModal.svelte 96-141, 255-347; GroupReleaseConfirmPopup.svelte; ActFormBody.svelte 495-563; ActFormModal.svelte 110-125
  found: bulkPlaceId по умолчанию = первое складское место (cartridge_storage_place_ids); acts.doReturn/updateReturn ждут ActDto. Попап выноса презентационный (items, onConfirm, onClose, две кнопки), подтверждение идёт через needs_group_release + confirm_group_release + подпись состава.
  implication: попап можно переиспользовать/обобщить, но у возврата нужен трёхисходный выбор (Отмена / Вывести из группы / Изменить расположение группы).

## Resolution

root_cause: plan_for_return в group_act_plan.rs переносит при возврате только якорные группы (решение D2). Контейнерная группа никогда не становится GroupMove, поэтому каждый возвращаемый член контейнера попадает в releases, выводится из группы без вопроса и получает место из формы возврата (по умолчанию складское). Путь возврата (do_return/update_return) не имеет исхода подтверждения.
fix:
verification:
files_changed: []


## Закрыто 2026-11-07
Подтверждено живой UAT 41.2 (G1–G12).
