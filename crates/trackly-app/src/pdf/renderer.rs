//! `PdfRenderer` — krilla 0.7 wrapper that turns a `DocSpec` into PDF bytes.
//!
//! Holds (a) the two embedded DejaVu Sans cuts as `Arc<Vec<u8>>` (cheap to
//! clone into per-render krilla `Font` instances) and (b) a long-lived MiniJinja
//! `Environment` (Phase 4 will use it; Phase 3 plan 01 leaves it dormant but
//! initialized so `PdfRenderer::new()` is the only place AppCtx talks to MiniJinja).
//!
//! ## Determinism guard (Pitfall 4)
//!
//! The same `DocSpec` must produce byte-identical output across consecutive
//! renders on a single machine (Wave-0 requirement; cross-OS stability is
//! verified later in CI). krilla 0.7 has two known sources of non-determinism:
//!
//! 1. **Metadata timestamps** — `Metadata::creation_date` is `None` by default,
//!    but PDFs are still allowed to carry a `/CreationDate` entry produced by
//!    other code paths. We pin a fixed `Metadata` with `creation_date =
//!    2026-01-01T00:00:00Z` and `producer = "Trackly Phase 3"`.
//! 2. **XMP metadata** — the XMP packet is regenerated at write time. We
//!    disable it via `SerializeSettings { xmp_metadata: false, .. }`.
//!
//! As a safety net (krilla version drift could reintroduce timestamps), the
//! final byte stream is post-processed with three regex replacements against
//! `/CreationDate (D:...)`, `/ModDate (D:...)`, and `/Producer (...)` PDF
//! dictionary entries. If any of these match, they are normalized to the same
//! pinned constants we pass via `Metadata`.

use std::sync::Arc;

use krilla::geom::{Point, Size, Transform};
use image::ImageReader;
use krilla::image::Image;
use std::io::Cursor;
use krilla::metadata::{DateTime as KrillaDateTime, Metadata};
use krilla::page::PageSettings;
use krilla::text::{Font, TextDirection};
use krilla::Document;
use krilla::SerializeSettings;
use minijinja::Environment;
use regex::bytes::Regex;
use trackly_core::error::AppError;

use super::docspec::{DocSpec, KvRow, Section, TextStyle};
use super::fonts::{DEJAVU_SANS_BOLD, DEJAVU_SANS_REGULAR};
use super::minijinja_env::build_safe_env;

/// Pinned creation date — January 1st 2026 00:00:00Z. Chosen so the PDF
/// `/CreationDate` entry is a constant string that's easy to spot in diffs and
/// not tied to any real-world build time.
const PINNED_CREATION_YEAR: u16 = 2026;
const PINNED_PRODUCER: &str = "Trackly Phase 3";

/// A4 portrait dimensions in PDF points (1 pt = 1/72 in).
const A4_WIDTH_PT: f32 = 595.276;
const A4_HEIGHT_PT: f32 = 841.890;

/// Page margin (uniform, all four sides).
const MARGIN_PT: f32 = 50.0;

/// Default font size for body text.
const BODY_SIZE_PT: f32 = 10.0;
/// Default font size for headings.
const HEADING_SIZE_PT: f32 = 14.0;

#[derive(Clone)]
pub struct PdfRenderer {
    pub font_regular_bytes: Arc<Vec<u8>>,
    pub font_bold_bytes: Arc<Vec<u8>>,
    pub minijinja_env: Arc<Environment<'static>>,
}

impl Default for PdfRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl PdfRenderer {
    /// Build a new renderer with embedded fonts and a safe-mode MiniJinja env.
    pub fn new() -> Self {
        Self {
            font_regular_bytes: Arc::new(DEJAVU_SANS_REGULAR.to_vec()),
            font_bold_bytes: Arc::new(DEJAVU_SANS_BOLD.to_vec()),
            minijinja_env: Arc::new(build_safe_env()),
        }
    }

    /// Render a `DocSpec` to PDF bytes.
    ///
    /// The output is post-processed for determinism — see the module-level
    /// docs and Pitfall 4 in 03-RESEARCH.md.
    pub fn render_docspec(&self, spec: &DocSpec) -> Result<Vec<u8>, AppError> {
        // SerializeSettings without XMP metadata — XMP carries timestamps that
        // are hard to neutralize without breaking parsers.
        let settings = SerializeSettings {
            xmp_metadata: false,
            ..SerializeSettings::default()
        };
        let mut doc = Document::new_with(settings);

        // Pin metadata so /Info dict timestamps and producer string are stable.
        let metadata = Metadata::new()
            .creation_date(KrillaDateTime::new(PINNED_CREATION_YEAR))
            .producer(PINNED_PRODUCER.to_owned());
        doc.set_metadata(metadata);

        let regular_data: krilla::Data = self.font_regular_bytes.as_ref().clone().into();
        let bold_data: krilla::Data = self.font_bold_bytes.as_ref().clone().into();
        let font_regular = Font::new(regular_data, 0).ok_or_else(|| AppError::Internal {
            source_chain: "krilla Font::new(regular): None — invalid TTF".into(),
        })?;
        let font_bold = Font::new(bold_data, 0).ok_or_else(|| AppError::Internal {
            source_chain: "krilla Font::new(bold): None — invalid TTF".into(),
        })?;

        let page_settings =
            PageSettings::from_wh(A4_WIDTH_PT, A4_HEIGHT_PT).ok_or_else(|| AppError::Internal {
                source_chain: "krilla PageSettings::from_wh: invalid A4 dimensions".into(),
            })?;

        // Build a single page. Pagination is out of scope for Phase 3 plan 01;
        // plan 04 may add page breaks if a real act overflows.
        {
            let mut page = doc.start_page_with(page_settings);
            let mut surface = page.surface();

            let mut y = MARGIN_PT + HEADING_SIZE_PT;

            // Header band — org name + act_label + date_label. Kept minimal so
            // the determinism fixture stays predictable.
            surface.draw_text(
                Point::from_xy(MARGIN_PT, y),
                font_bold.clone(),
                HEADING_SIZE_PT,
                &spec.header.org_name,
                false,
                TextDirection::Auto,
            );
            y += HEADING_SIZE_PT + 4.0;
            surface.draw_text(
                Point::from_xy(MARGIN_PT, y),
                font_regular.clone(),
                BODY_SIZE_PT,
                &spec.header.org_address,
                false,
                TextDirection::Auto,
            );
            y += BODY_SIZE_PT + 4.0;
            surface.draw_text(
                Point::from_xy(MARGIN_PT, y),
                font_regular.clone(),
                BODY_SIZE_PT,
                &format!("ИНН {}  КПП {}", spec.header.org_inn, spec.header.org_kpp),
                false,
                TextDirection::Auto,
            );
            y += BODY_SIZE_PT + 16.0;
            surface.draw_text(
                Point::from_xy(MARGIN_PT, y),
                font_regular.clone(),
                BODY_SIZE_PT,
                &spec.header.date_label,
                false,
                TextDirection::Auto,
            );
            y += BODY_SIZE_PT + 16.0;

            // Optional logo in the top-right corner. Graceful — a missing file
            // or unsupported mime is a tracing::warn, never an error: orgs
            // without a logo (or with a misconfigured path) must still print.
            // ACT-11 / CR-01: previous Phase-3 plans wired `safe_logo_canonical`
            // up to `spec.header.logo_path`, but the renderer ignored it. This
            // block consumes it via `std::fs::read` → `Image::from_png/jpeg`
            // → `surface.draw_image`.
            if let Some(logo_path_str) = &spec.header.logo_path {
                draw_logo_top_right(&mut surface, logo_path_str);
            }

            // Walk DocSpec sections.
            for section in &spec.sections {
                y = render_section(&mut surface, section, &font_regular, &font_bold, y);
            }

            surface.finish();
            page.finish();
        }

        let raw = doc.finish().map_err(|e| AppError::Internal {
            source_chain: format!("krilla doc.finish: {e:?}"),
        })?;

        Ok(normalize_pdf_for_determinism(&raw))
    }
}

/// Render a single Section against `surface`. Returns the y-cursor *after*
/// this section (with trailing padding included).
fn render_section(
    surface: &mut krilla::surface::Surface<'_>,
    section: &Section,
    font_regular: &Font,
    font_bold: &Font,
    mut y: f32,
) -> f32 {
    match section {
        Section::Paragraph { text, style } => {
            let font = match style {
                TextStyle::Regular => font_regular,
                TextStyle::Bold => font_bold,
            };
            surface.draw_text(
                Point::from_xy(MARGIN_PT, y),
                font.clone(),
                BODY_SIZE_PT,
                text,
                false,
                TextDirection::Auto,
            );
            y + BODY_SIZE_PT + 4.0
        }
        Section::Heading { level, text } => {
            let size = match level {
                1 => HEADING_SIZE_PT,
                2 => HEADING_SIZE_PT - 2.0,
                _ => HEADING_SIZE_PT - 4.0,
            };
            surface.draw_text(
                Point::from_xy(MARGIN_PT, y),
                font_bold.clone(),
                size,
                text,
                false,
                TextDirection::Auto,
            );
            y + size + 8.0
        }
        Section::KeyValueTable { rows } => {
            for KvRow { key, value } in rows {
                surface.draw_text(
                    Point::from_xy(MARGIN_PT, y),
                    font_bold.clone(),
                    BODY_SIZE_PT,
                    &format!("{key}:"),
                    false,
                    TextDirection::Auto,
                );
                surface.draw_text(
                    Point::from_xy(MARGIN_PT + 120.0, y),
                    font_regular.clone(),
                    BODY_SIZE_PT,
                    value,
                    false,
                    TextDirection::Auto,
                );
                y += BODY_SIZE_PT + 4.0;
            }
            y + 8.0
        }
        Section::ItemsTable { columns, rows } => {
            // Column header row.
            let col_count = columns.len().max(1);
            let usable_width = A4_WIDTH_PT - 2.0 * MARGIN_PT;
            let col_width = usable_width / col_count as f32;
            for (idx, col) in columns.iter().enumerate() {
                surface.draw_text(
                    Point::from_xy(MARGIN_PT + idx as f32 * col_width, y),
                    font_bold.clone(),
                    BODY_SIZE_PT,
                    col,
                    false,
                    TextDirection::Auto,
                );
            }
            y += BODY_SIZE_PT + 6.0;
            for row in rows {
                for (idx, cell) in row.iter().enumerate() {
                    surface.draw_text(
                        Point::from_xy(MARGIN_PT + idx as f32 * col_width, y),
                        font_regular.clone(),
                        BODY_SIZE_PT,
                        cell,
                        false,
                        TextDirection::Auto,
                    );
                }
                y += BODY_SIZE_PT + 4.0;
            }
            y + 8.0
        }
        Section::Signature {
            left_label,
            right_label,
            spacer_pt,
        } => {
            y += spacer_pt;
            let mid = A4_WIDTH_PT / 2.0;
            surface.draw_text(
                Point::from_xy(MARGIN_PT, y),
                font_regular.clone(),
                BODY_SIZE_PT,
                left_label,
                false,
                TextDirection::Auto,
            );
            surface.draw_text(
                Point::from_xy(mid + 10.0, y),
                font_regular.clone(),
                BODY_SIZE_PT,
                right_label,
                false,
                TextDirection::Auto,
            );
            y + BODY_SIZE_PT + 4.0
        }
        Section::Spacer { height_pt } => y + height_pt,
    }
}

/// Default rendered logo box size in PDF points (top-right corner of page).
const LOGO_WIDTH_PT: f32 = 100.0;
const LOGO_HEIGHT_PT: f32 = 50.0;

/// G-9 (Phase 3.1): scale logo proportionally to fit within `(max_w, max_h)`
/// box without distortion. Returns `(final_w, final_h)`.
///
/// scale = min(max_w / orig_w, max_h / orig_h) → contain-fit; равные scale
/// гарантируют aspect-ratio preservation.
///
/// Edge cases:
/// - orig dims ≤ 0 → return (0, 0) — caller treats как «skip render».
/// - scale > 1 (image smaller than max) → НЕ масштабируем вверх (keep orig);
///   практически — для очень маленьких логотипов на PDF лучше показать их в
///   натуральном размере, чем растянуть.
pub fn scale_logo_dimensions(
    orig_w: f32,
    orig_h: f32,
    max_w: f32,
    max_h: f32,
) -> (f32, f32) {
    if orig_w <= 0.0 || orig_h <= 0.0 || max_w <= 0.0 || max_h <= 0.0 {
        return (0.0, 0.0);
    }
    let scale = (max_w / orig_w).min(max_h / orig_h).min(1.0);
    (orig_w * scale, orig_h * scale)
}

/// Read a logo image from disk and emit a `draw_image` call into the
/// top-right corner of the page. Failures (missing file, unsupported mime,
/// invalid bytes) are logged at WARN and the function returns silently —
/// rendering must remain graceful so orgs without a valid logo still get a
/// document (ACT-11 / CR-01 design choice in 03-06-PLAN.md).
fn draw_logo_top_right(surface: &mut krilla::surface::Surface<'_>, logo_path_str: &str) {
    let bytes = match std::fs::read(logo_path_str) {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!(
                path = %logo_path_str,
                error = %e,
                "Logo file not readable — skipping"
            );
            return;
        }
    };

    // G-9 (Phase 3.1): parse intrinsic dimensions BEFORE moving `bytes`
    // into krilla. image crate returns u32 для w/h без фактического decoding
    // pixel data — копировать bytes не нужно (ImageReader работает по &[u8]).
    let (orig_w, orig_h) = match ImageReader::new(Cursor::new(bytes.as_slice()))
        .with_guessed_format()
    {
        Ok(reader) => match reader.into_dimensions() {
            Ok((w, h)) => (w as f32, h as f32),
            Err(e) => {
                tracing::warn!(
                    path = %logo_path_str,
                    error = %e,
                    "Logo intrinsic dimensions parse failed — skipping"
                );
                return;
            }
        },
        Err(e) => {
            tracing::warn!(
                path = %logo_path_str,
                error = %e,
                "Logo format guess failed — skipping"
            );
            return;
        }
    };

    // krilla::image::Image::from_png / from_jpeg expect `krilla::Data` (a
    // ref-counted byte container) — convert via `Into`. `interpolate = true`
    // gives smoother scaling for line-art logos.
    let lower = logo_path_str.to_lowercase();
    let image_result: Result<Image, String> = if lower.ends_with(".png") {
        Image::from_png(bytes.into(), true)
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        Image::from_jpeg(bytes.into(), true)
    } else {
        tracing::warn!(
            path = %logo_path_str,
            "Logo mime not supported (only .png / .jpg / .jpeg) — skipping"
        );
        return;
    };

    let image = match image_result {
        Ok(img) => img,
        Err(e) => {
            tracing::warn!(
                path = %logo_path_str,
                error = %e,
                "Logo bytes failed to parse — skipping"
            );
            return;
        }
    };

    let (final_w, final_h) =
        scale_logo_dimensions(orig_w, orig_h, LOGO_WIDTH_PT, LOGO_HEIGHT_PT);
    if final_w <= 0.0 || final_h <= 0.0 {
        tracing::warn!(
            path = %logo_path_str,
            orig_w, orig_h,
            "Logo scaled dimensions non-positive — skipping"
        );
        return;
    }

    let size = match Size::from_wh(final_w, final_h) {
        Some(s) => s,
        None => {
            tracing::warn!(final_w, final_h, "Logo size invalid — skipping");
            return;
        }
    };

    // `surface.draw_image` paints at the current transform origin with the
    // image's natural origin = (0,0). Translate so the logo lands at the
    // top-right corner with a uniform page margin. push_transform / pop is a
    // standard graphics-state save/restore.
    //
    // Right-align based on final_w (NOT LOGO_WIDTH_PT) — иначе при final_w <
    // LOGO_WIDTH_PT логотип будет смещён ВПРАВО от правильного anchor'а.
    let tx = A4_WIDTH_PT - MARGIN_PT - final_w;
    let ty = MARGIN_PT;
    surface.push_transform(&Transform::from_translate(tx, ty));
    surface.draw_image(image, size);
    surface.pop();
}

/// Apply regex post-processing on raw PDF bytes to normalize known
/// non-deterministic dictionary entries (Pitfall 4 safety net).
///
/// Even though `Metadata` pins `creation_date` and `producer`, future krilla
/// versions might reintroduce timestamps elsewhere. This function is the
/// defensive lower bound.
fn normalize_pdf_for_determinism(input: &[u8]) -> Vec<u8> {
    // Compile regexes per call. The regex crate's compile cost is small
    // relative to PDF generation, and using OnceLock here would add
    // multi-threading surface that we don't need.
    let re_creation = Regex::new(r"/CreationDate \(D:[^)]*\)").expect("creation date regex");
    let re_mod = Regex::new(r"/ModDate \(D:[^)]*\)").expect("mod date regex");
    let re_producer = Regex::new(r"/Producer \([^)]*\)").expect("producer regex");

    let mut out = input.to_vec();
    out = re_creation
        .replace_all(&out, &b"/CreationDate (D:20260101000000Z)"[..])
        .into_owned();
    out = re_mod
        .replace_all(&out, &b"/ModDate (D:20260101000000Z)"[..])
        .into_owned();
    out = re_producer
        .replace_all(&out, &b"/Producer (Trackly Phase 3)"[..])
        .into_owned();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf::docspec::{HeaderBlock, Section};

    fn hello_world_spec() -> DocSpec {
        DocSpec {
            title: "Привет".into(),
            header: HeaderBlock {
                org_name: "Org".into(),
                org_inn: "1".into(),
                org_kpp: "2".into(),
                org_address: "Addr".into(),
                logo_path: None,
                act_label: "Hello".into(),
                date_label: "Today".into(),
            },
            sections: vec![Section::Heading {
                level: 1,
                text: "Hello мир".into(),
            }],
        }
    }

    #[test]
    fn pdf_renderer_renders_hello_world() {
        let r = PdfRenderer::new();
        let bytes = r.render_docspec(&hello_world_spec()).expect("render");
        assert!(
            bytes.len() > 1000,
            "expected non-trivial PDF size, got {} bytes",
            bytes.len()
        );
        // PDF magic header
        assert_eq!(&bytes[..4], b"%PDF", "missing PDF magic header");
    }

    #[test]
    fn two_renders_produce_same_bytes() {
        let r = PdfRenderer::new();
        let a = r.render_docspec(&hello_world_spec()).expect("a");
        let b = r.render_docspec(&hello_world_spec()).expect("b");
        if a != b {
            // Surface a useful diagnostic on failure: show the first 12
            // differing byte positions so the determinism guard can be tuned.
            let diffs: Vec<usize> = a
                .iter()
                .zip(b.iter())
                .enumerate()
                .filter_map(|(i, (x, y))| if x != y { Some(i) } else { None })
                .take(12)
                .collect();
            panic!(
                "non-deterministic PDF output. First diff offsets: {:?}, len_a={}, len_b={}",
                diffs,
                a.len(),
                b.len()
            );
        }
    }

    #[test]
    fn normalize_replaces_creation_date_safety_net() {
        let raw = b"...prefix /CreationDate (D:20240715120000+02'00') suffix...".to_vec();
        let out = normalize_pdf_for_determinism(&raw);
        let s = String::from_utf8_lossy(&out);
        assert!(
            s.contains("/CreationDate (D:20260101000000Z)"),
            "post-process did not normalize CreationDate, got {s}"
        );
    }

    #[test]
    fn normalize_replaces_producer_safety_net() {
        let raw = b"...prefix /Producer (krilla 0.7.0) suffix...".to_vec();
        let out = normalize_pdf_for_determinism(&raw);
        let s = String::from_utf8_lossy(&out);
        assert!(
            s.contains("/Producer (Trackly Phase 3)"),
            "post-process did not normalize Producer, got {s}"
        );
    }
}
