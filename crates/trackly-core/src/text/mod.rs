//! Pure (no I/O) text-processing helpers for numbering templates (Phase 40.2).
//!
//! - [`mask`] — mask grammar: parsing, validation, date-token expansion,
//!   rendering and reverse-parsing of numbers (NUM-02).
//! - [`homoglyphs`] — Cyrillic/Latin script-mix detection and skeleton
//!   comparison for doppelganger warnings (NUM-12, D-04).
//! - [`sequence`] — first-free / max+1 / overflow computation over a set of
//!   taken numbers (NUM-04/NUM-05).

pub mod homoglyphs;
pub mod mask;
pub mod sequence;
