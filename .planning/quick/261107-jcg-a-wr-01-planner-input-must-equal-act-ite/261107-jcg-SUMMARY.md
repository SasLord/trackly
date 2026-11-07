# Quick 261107-jcg: A-WR-01 — вход планировщика = состав act_items

**One-liner:** в `ActService::create` планировщик групп получает через хелпер `create_item_device_ids` ровно те устройства, что станут позициями акта; лишний `device_id` вне непустого `device_ids` больше не двигает и не выводит из состава свою группу.

## Изменения
- `crates/trackly-app/src/services/act_service.rs`: хелпер `create_item_device_ids(&ActItemNewDto)` (device_ids, при пустом — `[device_id]`); используется в блоке `act_device_ids` перед `plan_in_tx` и в `ids_to_check` у `validate_create` (семантика там та же). Писатель (`effective_device_ids`) и валидация `device_id ∈ device_ids` не тронуты (locked-решение).
- `crates/trackly-app/tests/acts_groups.rs`: тест `create_stray_device_id_does_not_touch_its_group` (вымышленные данные).

## Мутационная проверка
На старой строке `std::iter::once(item.device_id).chain(item.device_ids...)` тест КРАСНЫЙ:

```
panicked at crates/trackly-app/tests/acts_groups.rs:336:18:
ожидали Created, получили NeedsGroupRelease(GroupReleaseWarningDto { items: [GroupReleaseItemDto { name: "Член 1", group_name: "Группа Ж" }] })
test result: FAILED. 0 passed; 1 failed
```

После возврата фикса: `acts_groups` 83 passed, 0 failed.

## Гейты
- `cargo test -p trackly-app --test acts_groups` — 83/83
- `cargo test -p trackly-app --lib act_service` — 7/7
- `cargo clippy -p trackly-app --all-targets -- -D warnings` — чисто
- `cargo fmt --check` — чисто
- Грепа приватности по диффу — пусто

## Отклонения
- [Rule 3] Хелпер размещён перед `#[cfg(test)]`-модулем (не в конце файла), тип указан полным путём `crate::dto::act::ActItemNewDto` (импорта в файле нет).

**Commit:** см. `git log` (fix(quick-261107-jcg)).

## Self-Check: PASSED
