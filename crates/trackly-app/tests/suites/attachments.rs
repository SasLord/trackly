//! Suite `app_attachments`: вложения, резервные копии, реквизиты организации.
//!
//! Корень бинарника; сами тесты лежат в `tests/*.rs` и подключены через `#[path]`.
//! Порядок строк — алфавитный; новый файл добавлять сюда же.

// Файлы тестов исторически сами объявляют `mod entities_support;` /
// `mod repair_support;` и т.п. — внутри одного бинарника такой модуль-помощник
// компилируется копией на каждый подключивший его файл. Это намеренно: файлы
// тестов остаются самодостаточными и не правились при сборке в suite.
#![allow(clippy::duplicate_mod)]

#[path = "../attachments_access.rs"]
mod attachments_access;
#[path = "../attachments_draft.rs"]
mod attachments_draft;
#[path = "../attachments_http.rs"]
mod attachments_http;
#[path = "../attachments_integrity.rs"]
mod attachments_integrity;
#[path = "../attachments_lifecycle.rs"]
mod attachments_lifecycle;
#[path = "../attachments_move_db.rs"]
mod attachments_move_db;
#[path = "../attachments_service.rs"]
mod attachments_service;
#[path = "../attachments_unavailable_root.rs"]
mod attachments_unavailable_root;
#[path = "../backup_service.rs"]
mod backup_service;
#[path = "../org_settings.rs"]
mod org_settings;
#[path = "../organization_io.rs"]
mod organization_io;
