//! CSV import/export utilities for the Devices entity.
//!
//! Modules:
//! - `sniff`         — byte-level encoding detection (UTF-8 / CP1251) + delimiter sniff
//! - `decode`        — decode raw bytes to String using detected encoding
//! - `parse`         — CSV row parsing via the `csv` crate
//! - `session_store` — in-memory preview-then-commit token store (5-min TTL)
//!
//! Full implementations for `sniff`, `decode`, and `parse` land in Plan 05.
//! `session_store` is fully implemented here because `DeviceService` references it.

pub mod decode;
pub mod parse;
pub mod session_store;
pub mod sniff;
