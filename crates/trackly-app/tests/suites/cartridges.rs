//! Suite `app_cartridges`: картриджи, принтеры, дашборд, конкурентные записи.
//!
//! Корень бинарника; сами тесты лежат в `tests/*.rs` и подключены через `#[path]`.
//! Порядок строк — алфавитный; новый файл добавлять сюда же.

// Файлы тестов исторически сами объявляют `mod entities_support;` /
// `mod repair_support;` и т.п. — внутри одного бинарника такой модуль-помощник
// компилируется копией на каждый подключивший его файл. Это намеренно: файлы
// тестов остаются самодостаточными и не правились при сборке в suite.
#![allow(clippy::duplicate_mod)]

#[path = "../cartridges_crud.rs"]
mod cartridges_crud;
#[path = "../cartridges_history.rs"]
mod cartridges_history;
#[path = "../cartridges_lifecycle.rs"]
mod cartridges_lifecycle;
#[path = "../cartridges_low_stock.rs"]
mod cartridges_low_stock;
#[path = "../cartridges_numbering.rs"]
mod cartridges_numbering;
#[path = "../cartridges_search.rs"]
mod cartridges_search;
#[path = "../concurrent_writes.rs"]
mod concurrent_writes;
#[path = "../dashboard_widgets.rs"]
mod dashboard_widgets;
#[path = "../printers_admit.rs"]
mod printers_admit;
#[path = "../printers_soft_delete_visibility.rs"]
mod printers_soft_delete_visibility;
