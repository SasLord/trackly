//! Suite `app_places`: места и журнал перемещений.
//!
//! Корень бинарника; сами тесты лежат в `tests/*.rs` и подключены через `#[path]`.
//! Порядок строк — алфавитный; новый файл добавлять сюда же.

// Файлы тестов исторически сами объявляют `mod entities_support;` /
// `mod repair_support;` и т.п. — внутри одного бинарника такой модуль-помощник
// компилируется копией на каждый подключивший его файл. Это намеренно: файлы
// тестов остаются самодостаточными и не правились при сборке в suite.
#![allow(clippy::duplicate_mod)]

#[path = "../history_timeline.rs"]
mod history_timeline;
#[path = "../place_movements_act_link.rs"]
mod place_movements_act_link;
#[path = "../place_movements_bulk_move.rs"]
mod place_movements_bulk_move;
#[path = "../place_movements_group_fields.rs"]
mod place_movements_group_fields;
#[path = "../place_movements_timeline.rs"]
mod place_movements_timeline;
#[path = "../place_movements_write_sites_cartridges.rs"]
mod place_movements_write_sites_cartridges;
#[path = "../place_movements_write_sites_devices.rs"]
mod place_movements_write_sites_devices;
#[path = "../places_bulk_move_expand_guard.rs"]
mod places_bulk_move_expand_guard;
#[path = "../places_contents.rs"]
mod places_contents;
#[path = "../places_delete_blocked.rs"]
mod places_delete_blocked;
#[path = "../places_group_members.rs"]
mod places_group_members;
#[path = "../places_move_cycle.rs"]
mod places_move_cycle;
#[path = "../places_move_groups.rs"]
mod places_move_groups;
#[path = "../places_search.rs"]
mod places_search;
#[path = "../places_service_crud.rs"]
mod places_service_crud;
