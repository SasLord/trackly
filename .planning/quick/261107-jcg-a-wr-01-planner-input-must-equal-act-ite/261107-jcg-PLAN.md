---
phase: quick-261107-jcg
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/trackly-app/src/services/act_service.rs
  - crates/trackly-app/tests/acts_groups.rs
autonomous: true
requirements: [A-WR-01]
must_haves:
  truths:
    - "Вход планировщика групп в ActService::create = ровно те устройства, что становятся act_items"
    - "device_id вне непустого device_ids не переносит и не выводит из состава живую группу"
    - "validate_create НЕ получает проверку device_id ∈ device_ids (locked решение оркестратора)"
    - "Поведение писателя (effective_device_ids на ~:593) не изменено"
  artifacts:
    - path: "crates/trackly-app/tests/acts_groups.rs"
      provides: "Регрессионный тест A-WR-01"
      contains: "create_stray_device_id_does_not_touch_its_group"
  key_links:
    - from: "act_service.rs create (планировщик ~:494)"
      to: "новый хелпер create_item_device_ids"
      via: "вызов вместо once(device_id).chain(device_ids)"
      pattern: "create_item_device_ids"
---

<objective>
Исправить A-WR-01: в `ActService::create` планировщик групп `plan_in_tx` должен получать те же устройства, что реально станут позициями акта.

Purpose: устройство `device_id`, не входящее в непустой `device_ids`, не должно менять место или членство без строки act_items и снимка before_json.
Output: правка act_service.rs + регрессионный тест в acts_groups.rs.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@CLAUDE.md
@.planning/phases/41.2-groups-acts-places/41.2-REVIEW.md

Факты (проверены):
- act_service.rs ~:304-308 (validate_create): `ids_to_check = if it.device_ids.is_empty() { vec![it.device_id] } else { it.device_ids.clone() }`.
- act_service.rs ~:494-503: `act_device_ids` строится через `once(item.device_id).chain(item.device_ids...)` с дедупом по `contains`, затем `plan_in_tx(&tx, &act_device_ids)`.
- ~:593 писатель: `effective_device_ids` (локальная переменная) берёт только `device_ids`, если он непуст. НЕ ТРОГАТЬ.
- Свободная функция `effective_device_ids` (~:5254) принимает `&ActReturnItemDto` — другой тип, не переиспользовать.
- Элемент payload.items имеет тип `ActItemNewDto` (поля device_id: i64, device_ids: Vec<i64>, quantity: i64).
- Хелперы теста (acts_groups.rs): make_test_ctx, timed, seed_place, seed_group, seed_device(dev(name, place, group)), dto(number, ids, place, confirm) (строит items с device_id=id, device_ids=[id]), create_ok, create_warning, group_place, group_members, member_rows, effects(ctx, act, kind), scalar_i64, int. Только вымышленные данные.
- ЛОКАЛЬНОЕ РЕШЕНИЕ (locked): не добавлять валидацию device_id ∈ device_ids — UI шлёт device_ids = group_ids.slice(0, quantity).
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Регрессионный тест (красный на старом коде) и фикс входа планировщика</name>
  <files>crates/trackly-app/src/services/act_service.rs, crates/trackly-app/tests/acts_groups.rs</files>
  <behavior>
    - Группа G (место A) с членами g1, g2. Свободные устройства f1, f2 без группы (место A).
    - Акт с одним item: device_id = g1, device_ids = [f1, f2], quantity = 2, place = B, confirm_group_release = false.
    - Ожидание: create возвращает Created без запроса подтверждения (create_ok не паникует); group_place(G) остаётся A; group_members(G) == 2; member_rows(g1) == 1; effects(act,"move") == 0 и effects(act,"release") == 0; place g1 не изменён; act_items только для f1, f2.
  </behavior>
  <action>
    1. В act_service.rs добавить приватную свободную функцию-хелпер `create_item_device_ids(item: &ActItemNewDto) -> Vec<i64>`: при пустом `item.device_ids` возвращает `vec![item.device_id]`, иначе `item.device_ids.clone()`. Комментарий: правило «какие устройства входят в позицию акта» (A-WR-01), совпадает с effective_device_ids писателя.
    2. Использовать хелпер в блоке построения `act_device_ids` (~:494-502): заменить `std::iter::once(item.device_id).chain(...)` на `create_item_device_ids(item)`, сохранив дедуп через `contains`. Использовать его же в `ids_to_check` в validate_create (~:304-308) — поведение там идентично, меняется только источник правила. Писатель (~:593) не трогать.
    3. Добавить тест `create_stray_device_id_does_not_touch_its_group` в acts_groups.rs (в стиле существующих: `#[tokio::test(flavor = "multi_thread", worker_threads = 2)]`, `timed(async {...}).await`). Собрать ActCreateDto через `dto("G-STRAY", &[], Some(b), false)` и затем присвоить `items = vec![ActItemNewDto { device_id: g1, device_ids: vec![f1, f2], quantity: 2 }]` (ActItemNewDto уже импортирован в файле; если нет — добавить в use). Имена устройств/мест вымышленные. Ассертить пункты из <behavior>.
    4. Мутационная проверка: СНАЧАЛА написать тест, временно вернуть старую строку (`std::iter::once(item.device_id).chain(item.device_ids.iter().copied())`) в блоке планировщика, запустить тест и убедиться, что он КРАСНЫЙ (падает по ожидаемой причине: перенос группы или запрос подтверждения). Зафиксировать результат в SUMMARY. Затем вернуть фикс и убедиться, что тест зелёный. Якорь мутации уникален — одна строка `std::iter::once(item.device_id)` в файле; проверить grep'ом перед подменой.
  </action>
  <verify>
    <automated>DEVELOPER_DIR=/Library/Developer/CommandLineTools TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test acts_groups create_stray_device_id</automated>
  </verify>
  <done>Тест красный на старом коде (зафиксировано), зелёный на фиксе; планировщик и dedup используют один хелпер; писатель и validate_create-семантика не изменены.</done>
</task>

<task type="auto">
  <name>Task 2: Прицельные гейты (последовательно, по одному cargo)</name>
  <files>crates/trackly-app/src/services/act_service.rs, crates/trackly-app/tests/acts_groups.rs</files>
  <action>
    Запускать строго по одной, без параллельных cargo и без полного workspace-прогона (команды `timeout` нет): 1) `cargo test -p trackly-app --test acts_groups` целиком; 2) `cargo test -p trackly-app --lib act_service` если юнит-тесты есть; 3) `cargo clippy -p trackly-app --all-targets -- -D warnings`; 4) `cargo fmt --check` (при расхождении — `cargo fmt` и повторить). Перед любым коммитом проверить, что в диффе нет реальных данных организации/людей.
  </action>
  <verify>
    <automated>DEVELOPER_DIR=/Library/Developer/CommandLineTools TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test acts_groups</automated>
  </verify>
  <done>acts_groups зелёный целиком, clippy -D warnings и fmt --check чисты.</done>
</task>

</tasks>

<verification>
Все тесты acts_groups зелёные; регрессионный тест подтверждённо красный на старой строке; clippy и fmt чисты.
</verification>

<success_criteria>
Планировщик групп при create получает ровно effective-состав позиций акта; stray device_id не двигает и не выводит из состава свою группу; validate_create не отвергает такой ввод.
</success_criteria>

<output>
Создать `.planning/quick/261107-jcg-a-wr-01-planner-input-must-equal-act-ite/261107-jcg-SUMMARY.md` (включить результат мутационной проверки).
</output>
