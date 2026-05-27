//! CSV import/export utilities for the Devices entity.
//!
//! Modules:
//! - `sniff`         — byte-level encoding detection (UTF-8 / CP1251) + delimiter sniff
//! - `decode`        — decode raw bytes to String using detected encoding
//! - `parse`         — CSV row parsing via the `csv` crate
//! - `session_store` — in-memory preview-then-commit token store (5-min TTL)

pub mod decode;
pub mod parse;
pub mod session_store;
pub mod sniff;

// Re-exports for convenience.
pub use decode::decode_to_string;
pub use parse::parse_rows;
pub use session_store::{ImportSession, ImportSessionStore};
pub use sniff::{detect, CsvProfile};
