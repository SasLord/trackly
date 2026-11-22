//! Suite `app_groups`: группы и типы групп.
//!
//! Корень бинарника; сами тесты лежат в `tests/*.rs` и подключены через `#[path]`.
//! Порядок строк — алфавитный; новый файл добавлять сюда же.

// Файлы тестов исторически сами объявляют `mod entities_support;` /
// `mod repair_support;` и т.п. — внутри одного бинарника такой модуль-помощник
// компилируется копией на каждый подключивший его файл. Это намеренно: файлы
// тестов остаются самодостаточными и не правились при сборке в suite.
#![allow(clippy::duplicate_mod)]

#[path = "../group_movements_journal.rs"]
mod group_movements_journal;
#[path = "../group_network_printer_ban_guard.rs"]
mod group_network_printer_ban_guard;
#[path = "../group_report_batch.rs"]
mod group_report_batch;
#[path = "../groups_anchor.rs"]
mod groups_anchor;
#[path = "../groups_membership.rs"]
mod groups_membership;
#[path = "../groups_move.rs"]
mod groups_move;
#[path = "../groups_move_status.rs"]
mod groups_move_status;
#[path = "../groups_move_status_transport.rs"]
mod groups_move_status_transport;
#[path = "../groups_property_removal_parity.rs"]
mod groups_property_removal_parity;
#[path = "../groups_service.rs"]
mod groups_service;
#[path = "../groups_types_service.rs"]
mod groups_types_service;
#[path = "../groups_values_card.rs"]
mod groups_values_card;
#[path = "../history_membership.rs"]
mod history_membership;
#[path = "../history_membership_release.rs"]
mod history_membership_release;
#[path = "../history_status_groups.rs"]
mod history_status_groups;
