//! Suite `app_acts`: акты: CRUD, нумерация, возвраты, группы, HTTP.
//!
//! Корень бинарника; сами тесты лежат в `tests/*.rs` и подключены через `#[path]`.
//! Порядок строк — алфавитный; новый файл добавлять сюда же.

// Файлы тестов исторически сами объявляют `mod entities_support;` /
// `mod repair_support;` и т.п. — внутри одного бинарника такой модуль-помощник
// компилируется копией на каждый подключивший его файл. Это намеренно: файлы
// тестов остаются самодостаточными и не правились при сборке в suite.
#![allow(clippy::duplicate_mod)]

#[path = "../act_picker_printers.rs"]
mod act_picker_printers;
#[path = "../acts_archived_at.rs"]
mod acts_archived_at;
#[path = "../acts_changed_place_ids.rs"]
mod acts_changed_place_ids;
#[path = "../acts_clone_handover.rs"]
mod acts_clone_handover;
#[path = "../acts_crud.rs"]
mod acts_crud;
#[path = "../acts_date_source.rs"]
mod acts_date_source;
#[path = "../acts_delete_choice_http.rs"]
mod acts_delete_choice_http;
#[path = "../acts_display_rule.rs"]
mod acts_display_rule;
#[path = "../acts_e2e_smoke.rs"]
mod acts_e2e_smoke;
#[path = "../acts_group_expansion_e2e.rs"]
mod acts_group_expansion_e2e;
#[path = "../acts_group_plan_order_guard.rs"]
mod acts_group_plan_order_guard;
#[path = "../acts_groups.rs"]
mod acts_groups;
#[path = "../acts_http_smoke.rs"]
mod acts_http_smoke;
#[path = "../acts_numbering.rs"]
mod acts_numbering;
#[path = "../acts_place_path_short.rs"]
mod acts_place_path_short;
#[path = "../acts_place_snapshot.rs"]
mod acts_place_snapshot;
#[path = "../acts_return_split_http.rs"]
mod acts_return_split_http;
#[path = "../acts_returns.rs"]
mod acts_returns;
#[path = "../acts_search.rs"]
mod acts_search;
#[path = "../acts_suggest.rs"]
mod acts_suggest;
#[path = "../acts_to_write_off_status.rs"]
mod acts_to_write_off_status;
#[path = "../acts_undo.rs"]
mod acts_undo;
#[path = "../acts_update.rs"]
mod acts_update;
#[path = "../acts_update_return.rs"]
mod acts_update_return;
#[path = "../acts_written_off_member.rs"]
mod acts_written_off_member;
#[path = "../group_act_expansion.rs"]
mod group_act_expansion;
#[path = "../history_act_undo.rs"]
mod history_act_undo;
#[path = "../history_group_act.rs"]
mod history_group_act;
#[path = "../history_status_acts.rs"]
mod history_status_acts;
