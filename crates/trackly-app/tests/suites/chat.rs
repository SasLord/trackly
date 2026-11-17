//! Suite `app_chat`: чат заявок (доступ, лента, непрочитанное, отправка, черновики, HTTP).
//!
//! Корень бинарника; сами тесты лежат в `tests/*.rs` и подключены через `#[path]`.
//! Порядок строк — алфавитный; новый файл добавлять сюда же.

#![allow(clippy::duplicate_mod)]

#[path = "../chat_access.rs"]
mod chat_access;
#[path = "../chat_feed.rs"]
mod chat_feed;
#[path = "../chat_unread.rs"]
mod chat_unread;
