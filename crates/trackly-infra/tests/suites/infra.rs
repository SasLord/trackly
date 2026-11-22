//! Suite `infra_tests`: все интеграционные тесты trackly-infra.
//!
//! Корень бинарника; сами тесты лежат в `tests/*.rs` и подключены через `#[path]`.
//! Порядок строк — алфавитный; новый файл добавлять сюда же.

// Файлы тестов исторически сами объявляют `mod entities_support;` /
// `mod repair_support;` и т.п. — внутри одного бинарника такой модуль-помощник
// компилируется копией на каждый подключивший его файл. Это намеренно: файлы
// тестов остаются самодостаточными и не правились при сборке в suite.
#![allow(clippy::duplicate_mod)]

#[path = "../act_group_effects_migration.rs"]
mod act_group_effects_migration;
#[path = "../act_group_effects_repo.rs"]
mod act_group_effects_repo;
#[path = "../acts_number_text_migration.rs"]
mod acts_number_text_migration;
#[path = "../attachments_migration.rs"]
mod attachments_migration;
#[path = "../attachments_paths.rs"]
mod attachments_paths;
#[path = "../attachments_repo.rs"]
mod attachments_repo;
#[path = "../audit_log_schema.rs"]
mod audit_log_schema;
#[path = "../cartridges_code_unique_migration.rs"]
mod cartridges_code_unique_migration;
#[path = "../cartridges_place_path_short.rs"]
mod cartridges_place_path_short;
#[path = "../cartridges_place_search.rs"]
mod cartridges_place_search;
#[path = "../chat_migration.rs"]
mod chat_migration;
#[path = "../chat_repo.rs"]
mod chat_repo;
#[path = "../config_example_test.rs"]
mod config_example_test;
#[path = "../config_test.rs"]
mod config_test;
#[path = "../device_statuses_migration.rs"]
mod device_statuses_migration;
#[path = "../devices_anchor_flags.rs"]
mod devices_anchor_flags;
#[path = "../devices_container_entries.rs"]
mod devices_container_entries;
#[path = "../devices_inventory_dedup_migration.rs"]
mod devices_inventory_dedup_migration;
#[path = "../devices_list_all.rs"]
mod devices_list_all;
#[path = "../devices_list_filters.rs"]
mod devices_list_filters;
#[path = "../devices_place_path_short.rs"]
mod devices_place_path_short;
#[path = "../devices_place_search.rs"]
mod devices_place_search;
#[path = "../devices_status_origin.rs"]
mod devices_status_origin;
#[path = "../group_types_repo.rs"]
mod group_types_repo;
#[path = "../groups_anchor_migration.rs"]
mod groups_anchor_migration;
#[path = "../groups_anchor_repo.rs"]
mod groups_anchor_repo;
#[path = "../groups_migration.rs"]
mod groups_migration;
#[path = "../groups_repo.rs"]
mod groups_repo;
#[path = "../history_events_repo.rs"]
mod history_events_repo;
#[path = "../migration_idempotency.rs"]
mod migration_idempotency;
#[path = "../number_templates_migration.rs"]
mod number_templates_migration;
#[path = "../number_templates_repo.rs"]
mod number_templates_repo;
#[path = "../paths_test.rs"]
mod paths_test;
#[path = "../per_record_invariants.rs"]
mod per_record_invariants;
#[path = "../phase06_stubs.rs"]
mod phase06_stubs;
#[path = "../place_effective_variant.rs"]
mod place_effective_variant;
#[path = "../place_movements_batch_repo.rs"]
mod place_movements_batch_repo;
#[path = "../place_movements_migration.rs"]
mod place_movements_migration;
#[path = "../place_movements_repo.rs"]
mod place_movements_repo;
#[path = "../place_path_settings.rs"]
mod place_path_settings;
#[path = "../places_crud.rs"]
mod places_crud;
#[path = "../places_group_members.rs"]
mod places_group_members;
#[path = "../repair_act_number_unique_migration.rs"]
mod repair_act_number_unique_migration;
#[path = "../repairs_migration.rs"]
mod repairs_migration;
#[path = "../repairs_repo.rs"]
mod repairs_repo;
#[path = "../seed_data.rs"]
mod seed_data;
