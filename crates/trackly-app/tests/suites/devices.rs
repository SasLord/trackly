//! Suite `app_devices`: устройства: CRUD, CSV, группировка, сетевые/USB-принтеры.
//!
//! Корень бинарника; сами тесты лежат в `tests/*.rs` и подключены через `#[path]`.
//! Порядок строк — алфавитный; новый файл добавлять сюда же.

// Файлы тестов исторически сами объявляют `mod entities_support;` /
// `mod repair_support;` и т.п. — внутри одного бинарника такой модуль-помощник
// компилируется копией на каждый подключивший его файл. Это намеренно: файлы
// тестов остаются самодостаточными и не правились при сборке в suite.
#![allow(clippy::duplicate_mod)]

#[path = "../device_statuses_lifecycle.rs"]
mod device_statuses_lifecycle;
#[path = "../device_types_dictionary.rs"]
mod device_types_dictionary;
#[path = "../devices_anchor_list.rs"]
mod devices_anchor_list;
#[path = "../devices_autocomplete.rs"]
mod devices_autocomplete;
#[path = "../devices_bulk_create.rs"]
mod devices_bulk_create;
#[path = "../devices_container_hide.rs"]
mod devices_container_hide;
#[path = "../devices_container_printers.rs"]
mod devices_container_printers;
#[path = "../devices_container_rows.rs"]
mod devices_container_rows;
#[path = "../devices_create_origin_join.rs"]
mod devices_create_origin_join;
#[path = "../devices_crud.rs"]
mod devices_crud;
#[path = "../devices_csv_export.rs"]
mod devices_csv_export;
#[path = "../devices_csv_import.rs"]
mod devices_csv_import;
#[path = "../devices_csv_session.rs"]
mod devices_csv_session;
#[path = "../devices_filter_registry.rs"]
mod devices_filter_registry;
#[path = "../devices_group_children.rs"]
mod devices_group_children;
#[path = "../devices_grouping.rs"]
mod devices_grouping;
#[path = "../devices_http_smoke.rs"]
mod devices_http_smoke;
#[path = "../devices_location_roundtrip.rs"]
mod devices_location_roundtrip;
#[path = "../devices_numbering.rs"]
mod devices_numbering;
#[path = "../devices_printer_create_ip.rs"]
mod devices_printer_create_ip;
#[path = "../devices_printer_link_audit.rs"]
mod devices_printer_link_audit;
#[path = "../devices_printer_network.rs"]
mod devices_printer_network;
#[path = "../devices_search.rs"]
mod devices_search;
#[path = "../devices_status_only_patch.rs"]
mod devices_status_only_patch;
#[path = "../devices_type_conversion.rs"]
mod devices_type_conversion;
#[path = "../devices_usb_host.rs"]
mod devices_usb_host;
#[path = "../devices_write_off_via_form.rs"]
mod devices_write_off_via_form;
