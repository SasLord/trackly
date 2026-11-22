//! Suite `app_reports`: отчёты, HTML/PDF-рендер, шаблоны номеров.
//!
//! Корень бинарника; сами тесты лежат в `tests/*.rs` и подключены через `#[path]`.
//! Порядок строк — алфавитный; новый файл добавлять сюда же.

// Файлы тестов исторически сами объявляют `mod entities_support;` /
// `mod repair_support;` и т.п. — внутри одного бинарника такой модуль-помощник
// компилируется копией на каждый подключивший его файл. Это намеренно: файлы
// тестов остаются самодостаточными и не правились при сборке в suite.
#![allow(clippy::duplicate_mod)]

#[path = "../history_report_guard.rs"]
mod history_report_guard;
#[path = "../html_act_render.rs"]
mod html_act_render;
#[path = "../html_report_render.rs"]
mod html_report_render;
#[path = "../number_templates_compute_next.rs"]
mod number_templates_compute_next;
#[path = "../number_templates_crud.rs"]
mod number_templates_crud;
#[path = "../pdf_column_overflow.rs"]
mod pdf_column_overflow;
#[path = "../pdf_determinism.rs"]
mod pdf_determinism;
#[path = "../pdf_logo.rs"]
mod pdf_logo;
#[path = "../pdf_logo_aspect.rs"]
mod pdf_logo_aspect;
#[path = "../pdf_render_act.rs"]
mod pdf_render_act;
#[path = "../pdf_text_extract.rs"]
mod pdf_text_extract;
#[path = "../report_acts.rs"]
mod report_acts;
#[path = "../report_cartridges.rs"]
mod report_cartridges;
#[path = "../report_csv_export.rs"]
mod report_csv_export;
#[path = "../report_movements.rs"]
mod report_movements;
#[path = "../report_movements_truncation.rs"]
mod report_movements_truncation;
#[path = "../report_period_bounds.rs"]
mod report_period_bounds;
#[path = "../report_place_path_short.rs"]
mod report_place_path_short;
#[path = "../report_place_subtree.rs"]
mod report_place_subtree;
#[path = "../report_repairs.rs"]
mod report_repairs;
#[path = "../report_requests.rs"]
mod report_requests;
#[path = "../report_returns_sub_number.rs"]
mod report_returns_sub_number;
#[path = "../report_status_codes.rs"]
mod report_status_codes;
#[path = "../reports_period_required.rs"]
mod reports_period_required;
#[path = "../templates_seed.rs"]
mod templates_seed;
