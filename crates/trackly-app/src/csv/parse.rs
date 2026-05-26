//! CSV row parser using the `csv` crate.
//!
//! Full implementation lands in Plan 05 (CSV import/export).
//! Supports configurable delimiters (`,` / `;`) and handles flexible/ragged rows.

#![allow(dead_code)]

// TODO: Plan 05 — implement parse_rows(text: &str, delimiter: u8) -> Result<(Vec<String>, Vec<Vec<String>>), csv::Error>
