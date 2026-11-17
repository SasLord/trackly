//! Suite `app_gates`: гейты-сканеры исходников и реестры (entities_changed, write-site, словари, паритет HTML/маршрутов).
//!
//! Корень бинарника; сами тесты лежат в `tests/*.rs` и подключены через `#[path]`.
//! Порядок строк — алфавитный; новый файл добавлять сюда же.

// Файлы тестов исторически сами объявляют `mod entities_support;` /
// `mod repair_support;` и т.п. — внутри одного бинарника такой модуль-помощник
// компилируется копией на каждый подключивший его файл. Это намеренно: файлы
// тестов остаются самодостаточными и не правились при сборке в suite.
#![allow(clippy::duplicate_mod)]

#[path = "../attachments_changed_gate.rs"]
mod attachments_changed_gate;
#[path = "../chat_audit_signal_inventory.rs"]
mod chat_audit_signal_inventory;
#[path = "../chat_changed_gate.rs"]
mod chat_changed_gate;
#[path = "../device_status_vocabulary_gate.rs"]
mod device_status_vocabulary_gate;
#[path = "../device_type_label_gate.rs"]
mod device_type_label_gate;
#[path = "../entities_changed_acts.rs"]
mod entities_changed_acts;
#[path = "../entities_changed_cartridges.rs"]
mod entities_changed_cartridges;
#[path = "../entities_changed_devices.rs"]
mod entities_changed_devices;
#[path = "../entities_changed_gate.rs"]
mod entities_changed_gate;
#[path = "../entities_changed_groups.rs"]
mod entities_changed_groups;
#[path = "../entities_changed_places.rs"]
mod entities_changed_places;
#[path = "../entities_changed_repairs.rs"]
mod entities_changed_repairs;
#[path = "../entities_changed_ws.rs"]
mod entities_changed_ws;
#[path = "../group_write_sites.rs"]
mod group_write_sites;
#[path = "../html_field_row_underline_gate.rs"]
mod html_field_row_underline_gate;
#[path = "../html_header_parity.rs"]
mod html_header_parity;
#[path = "../html_page_parity.rs"]
mod html_page_parity;
#[path = "../http_route_parity.rs"]
mod http_route_parity;
#[path = "../number_space_broadcast_gate.rs"]
mod number_space_broadcast_gate;
#[path = "../specta_roundtrip.rs"]
mod specta_roundtrip;
#[path = "../suite_wiring.rs"]
mod suite_wiring;
