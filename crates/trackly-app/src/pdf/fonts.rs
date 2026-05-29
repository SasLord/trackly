//! Cyrillic-safe fonts embedded at compile time via `include_bytes!`.
//!
//! License: DejaVu Sans 2.37 — public-domain-derived (descended from Bitstream
//! Vera Fonts). See `crates/trackly-app/assets/fonts/README.md` for the full
//! provenance and rationale.
//!
//! Both cuts are embedded into the `trackly-app` binary so the portable bundle
//! does not have to ship the .ttf files alongside the executable. krilla 0.7
//! performs OpenType subsetting at PDF write time — only the glyphs actually
//! used by each rendered document are embedded in the output bytes, keeping
//! generated PDFs small even though the source fonts are ~700 KB each.

/// DejaVu Sans Regular cut — full Cyrillic coverage (Russian + Belarusian +
/// Ukrainian Cyrillic letters including `ё`, Latin + Latin Extended-A
/// diacritics, common punctuation, the `№` sign).
pub static DEJAVU_SANS_REGULAR: &[u8] = include_bytes!("../../assets/fonts/DejaVuSans.ttf");

/// DejaVu Sans Bold cut — same coverage as Regular; used for headings and
/// signatures so the master-detail PDF document has a clear visual hierarchy.
pub static DEJAVU_SANS_BOLD: &[u8] = include_bytes!("../../assets/fonts/DejaVuSans-Bold.ttf");
