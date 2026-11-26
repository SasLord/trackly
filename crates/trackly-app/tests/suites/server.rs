//! Suite `app_server`: сервер, WebSocket, TLS, заявки.
//!
//! Корень бинарника; сами тесты лежат в `tests/*.rs` и подключены через `#[path]`.
//! Порядок строк — алфавитный; новый файл добавлять сюда же.

// Файлы тестов исторически сами объявляют `mod entities_support;` /
// `mod repair_support;` и т.п. — внутри одного бинарника такой модуль-помощник
// компилируется копией на каждый подключивший его файл. Это намеренно: файлы
// тестов остаются самодостаточными и не правились при сборке в suite.
#![allow(clippy::duplicate_mod)]

#[path = "../chat_ws_fanout.rs"]
mod chat_ws_fanout;
#[path = "../graceful_shutdown_drain.rs"]
mod graceful_shutdown_drain;
#[path = "../im_idempotent_send.rs"]
mod im_idempotent_send;
#[path = "../im_membership.rs"]
mod im_membership;
#[path = "../request_accept_assignee.rs"]
mod request_accept_assignee;
#[path = "../request_lifecycle.rs"]
mod request_lifecycle;
#[path = "../request_printer_options.rs"]
mod request_printer_options;
#[path = "../request_printer_scope.rs"]
mod request_printer_scope;
#[path = "../restore_request_visibility_http.rs"]
mod restore_request_visibility_http;
#[path = "../server_bind_effective_host.rs"]
mod server_bind_effective_host;
#[path = "../server_drain_on_stop.rs"]
mod server_drain_on_stop;
#[path = "../server_hot_toggle.rs"]
mod server_hot_toggle;
#[path = "../supervisor.rs"]
mod supervisor;
#[path = "../tls_server_smoke.rs"]
mod tls_server_smoke;
#[path = "../ws_broadcast_fanout.rs"]
mod ws_broadcast_fanout;
#[path = "../ws_close_on_server_stop.rs"]
mod ws_close_on_server_stop;
#[path = "../ws_heartbeat.rs"]
mod ws_heartbeat;
#[path = "../ws_http_single_broadcast.rs"]
mod ws_http_single_broadcast;
#[path = "../ws_resync_required.rs"]
mod ws_resync_required;
#[path = "../ws_upgrade_serve_connection.rs"]
mod ws_upgrade_serve_connection;
