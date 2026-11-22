//! Suite `app_repairs`: ремонтные заявки.
//!
//! Корень бинарника; сами тесты лежат в `tests/*.rs` и подключены через `#[path]`.
//! Порядок строк — алфавитный; новый файл добавлять сюда же.

// Файлы тестов исторически сами объявляют `mod entities_support;` /
// `mod repair_support;` и т.п. — внутри одного бинарника такой модуль-помощник
// компилируется копией на каждый подключивший его файл. Это намеренно: файлы
// тестов остаются самодостаточными и не правились при сборке в suite.
#![allow(clippy::duplicate_mod)]

#[path = "../employee_devices_list.rs"]
mod employee_devices_list;
#[path = "../history_status_repairs.rs"]
mod history_status_repairs;
#[path = "../repair_accept.rs"]
mod repair_accept;
#[path = "../repair_act_render.rs"]
mod repair_act_render;
#[path = "../repair_cancel_reject.rs"]
mod repair_cancel_reject;
#[path = "../repair_chat_merge.rs"]
mod repair_chat_merge;
#[path = "../repair_complete.rs"]
mod repair_complete;
#[path = "../repair_create.rs"]
mod repair_create;
#[path = "../repair_device_history.rs"]
mod repair_device_history;
#[path = "../repair_device_report.rs"]
mod repair_device_report;
#[path = "../repair_employee_state.rs"]
mod repair_employee_state;
#[path = "../repair_group_exception.rs"]
mod repair_group_exception;
#[path = "../repair_group_move.rs"]
mod repair_group_move;
#[path = "../repair_merge.rs"]
mod repair_merge;
#[path = "../repair_movement_source.rs"]
mod repair_movement_source;
#[path = "../repair_participants.rs"]
mod repair_participants;
#[path = "../repair_send_receive.rs"]
mod repair_send_receive;
#[path = "../repair_visibility.rs"]
mod repair_visibility;
#[path = "../repair_ws_signals.rs"]
mod repair_ws_signals;
