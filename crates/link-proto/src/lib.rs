//! Общий протокол Trackly Link: одна реализация для клиента и сервера.

pub mod spki;

pub use spki::{fmt_colon_upper, parse_colon_hex, spki_sha256, SpkiError};
