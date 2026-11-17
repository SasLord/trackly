//! Suite `app_auth`: вход, AD, пользователи, матрица ролей, заголовки безопасности.
//!
//! Корень бинарника; сами тесты лежат в `tests/*.rs` и подключены через `#[path]`.
//! Порядок строк — алфавитный; новый файл добавлять сюда же.

// Файлы тестов исторически сами объявляют `mod entities_support;` /
// `mod repair_support;` и т.п. — внутри одного бинарника такой модуль-помощник
// компилируется копией на каждый подключивший его файл. Это намеренно: файлы
// тестов остаются самодостаточными и не правились при сборке в suite.
#![allow(clippy::duplicate_mod)]

#[path = "../ad_admin_logins.rs"]
mod ad_admin_logins;
#[path = "../ad_auth.rs"]
mod ad_auth;
#[path = "../ad_directory_sso.rs"]
mod ad_directory_sso;
#[path = "../ad_register.rs"]
mod ad_register;
#[path = "../auth_bootstrap.rs"]
mod auth_bootstrap;
#[path = "../auth_remember_cookie.rs"]
mod auth_remember_cookie;
#[path = "../auth_smoke.rs"]
mod auth_smoke;
#[path = "../desktop_system_admin.rs"]
mod desktop_system_admin;
#[path = "../downgrade_protection.rs"]
mod downgrade_protection;
#[path = "../requests_ad_register.rs"]
mod requests_ad_register;
#[path = "../requests_ad_register_http.rs"]
mod requests_ad_register_http;
#[path = "../requests_ad_register_visibility_manager.rs"]
mod requests_ad_register_visibility_manager;
#[path = "../role_endpoint_matrix.rs"]
mod role_endpoint_matrix;
#[path = "../security_headers.rs"]
mod security_headers;
#[path = "../session_survives_restart.rs"]
mod session_survives_restart;
#[path = "../settings_ad.rs"]
mod settings_ad;
#[path = "../users_crud.rs"]
mod users_crud;
#[path = "../users_http_camelcase.rs"]
mod users_http_camelcase;
