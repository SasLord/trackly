//! Media layer for attachments: content-signature sniffing (D-08), thumbnails
//! (D-11), streaming SHA-256 and the HEIC→JPEG hook (D-10, D-24).
//!
//! Everything here is synchronous and database-free; the service wraps heavy
//! calls in `spawn_blocking`. The declared MIME / extension is never trusted:
//! the signature decides, the extension is only a hint for plain text.

// Consumed by the attachment service (plans 41.3-05..08); not wired yet.
#![allow(dead_code)]

use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use image::codecs::jpeg::JpegEncoder;
use image::{ExtendedColorType, ImageDecoder, ImageReader, Limits};
use sha2::{Digest, Sha256};

/// Coarse class of an accepted file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaKind {
    Photo,
    Pdf,
    Office,
    Archive,
    Text,
}

/// Result of a successful signature check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sniffed {
    pub mime: &'static str,
    pub kind: MediaKind,
    /// Can be shown inline (photos, PDF). HEIC is not viewable until converted.
    pub viewable: bool,
    pub is_heic: bool,
    /// Canonical extension for the type (hint only; never used for paths).
    pub ext_hint: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaReject {
    Empty,
    TypeNotAllowed,
}

const OLE2_MAGIC: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
const SEVENZ_MAGIC: [u8; 6] = [0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C];

fn ext_of(file_name: &str) -> String {
    file_name
        .rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default()
}

fn s(mime: &'static str, kind: MediaKind, viewable: bool, ext: &'static str) -> Sniffed {
    Sniffed {
        mime,
        kind,
        viewable,
        is_heic: false,
        ext_hint: ext,
    }
}

/// Executables and active content: rejected whatever the name says.
fn is_denied(head: &[u8]) -> bool {
    if head.starts_with(b"MZ")
        || head.starts_with(&[0x7F, b'E', b'L', b'F'])
        || head.starts_with(b"#!")
    {
        return true;
    }
    // Mach-O (thin, both endiannesses) and fat/universal.
    if matches!(
        head.get(..4),
        Some([0xFE, 0xED, 0xFA, 0xCE])
            | Some([0xFE, 0xED, 0xFA, 0xCF])
            | Some([0xCE, 0xFA, 0xED, 0xFE])
            | Some([0xCF, 0xFA, 0xED, 0xFE])
            | Some([0xCA, 0xFE, 0xBA, 0xBE])
    ) {
        return true;
    }
    // Text-ish active content (HTML / SVG / script) — look at the first 1 KiB.
    let window = &head[..head.len().min(1024)];
    let lower: Vec<u8> = window.iter().map(|b| b.to_ascii_lowercase()).collect();
    let trimmed = {
        let mut t: &[u8] = &lower;
        if t.starts_with(&[0xEF, 0xBB, 0xBF]) {
            t = &t[3..];
        }
        while let Some((first, rest)) = t.split_first() {
            if first.is_ascii_whitespace() {
                t = rest;
            } else {
                break;
            }
        }
        t
    };
    if trimmed.starts_with(b"<svg")
        || trimmed.starts_with(b"<html")
        || trimmed.starts_with(b"<!doctype html")
        || trimmed.starts_with(b"<script")
    {
        return true;
    }
    if trimmed.starts_with(b"<?xml") && lower.windows(4).any(|w| w == b"<svg") {
        return true;
    }
    false
}

/// Classifies a file by content.
///
/// `head` is the first bytes (pass at least 32 KiB when available); `full` is
/// the complete buffer, needed only for OLE2 (DOC/XLS), whose class lives in a
/// directory entry that can sit anywhere in the file. A truncated buffer is
/// never misclassified: without `full` an OLE2 file is rejected.
pub fn sniff(head: &[u8], full: Option<&[u8]>, file_name: &str) -> Result<Sniffed, MediaReject> {
    if head.is_empty() {
        return Err(MediaReject::Empty);
    }
    if is_denied(head) {
        return Err(MediaReject::TypeNotAllowed);
    }

    if infer::image::is_jpeg(head) {
        return Ok(s("image/jpeg", MediaKind::Photo, true, "jpg"));
    }
    if infer::image::is_png(head) {
        return Ok(s("image/png", MediaKind::Photo, true, "png"));
    }
    if infer::image::is_gif(head) {
        return Ok(s("image/gif", MediaKind::Photo, true, "gif"));
    }
    if infer::image::is_webp(head) {
        // D-09: stored as is, viewable like JPEG/PNG.
        return Ok(s("image/webp", MediaKind::Photo, true, "webp"));
    }
    if infer::image::is_heif(head) {
        return Ok(Sniffed {
            mime: "image/heic",
            kind: MediaKind::Photo,
            viewable: false,
            is_heic: true,
            ext_hint: "heic",
        });
    }
    if infer::image::is_avif(head) {
        return Err(MediaReject::TypeNotAllowed);
    }
    if head.starts_with(b"%PDF-") {
        return Ok(s("application/pdf", MediaKind::Pdf, true, "pdf"));
    }
    if head.starts_with(&OLE2_MAGIC) {
        let buf = full.unwrap_or(&[]);
        if infer::doc::is_doc(buf) {
            return Ok(s("application/msword", MediaKind::Office, false, "doc"));
        }
        if infer::doc::is_xls(buf) {
            return Ok(s(
                "application/vnd.ms-excel",
                MediaKind::Office,
                false,
                "xls",
            ));
        }
        return Err(MediaReject::TypeNotAllowed);
    }
    if infer::doc::is_docx(head) {
        return Ok(s(
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            MediaKind::Office,
            false,
            "docx",
        ));
    }
    if infer::doc::is_xlsx(head) {
        return Ok(s(
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            MediaKind::Office,
            false,
            "xlsx",
        ));
    }
    if infer::odf::is_odt(head) {
        return Ok(s(
            "application/vnd.oasis.opendocument.text",
            MediaKind::Office,
            false,
            "odt",
        ));
    }
    if infer::odf::is_ods(head) {
        return Ok(s(
            "application/vnd.oasis.opendocument.spreadsheet",
            MediaKind::Office,
            false,
            "ods",
        ));
    }
    if head.starts_with(&SEVENZ_MAGIC) {
        return Ok(s(
            "application/x-7z-compressed",
            MediaKind::Archive,
            false,
            "7z",
        ));
    }
    if infer::archive::is_zip(head) {
        return Ok(s("application/zip", MediaKind::Archive, false, "zip"));
    }

    // Text last: only by whitelisted extension and only if it looks like text.
    let ext = ext_of(file_name);
    if matches!(ext.as_str(), "txt" | "csv" | "log") {
        let probe = &head[..head.len().min(8 * 1024)];
        if !probe.contains(&0) {
            return Ok(s("text/plain", MediaKind::Text, false, "txt"));
        }
    }
    Err(MediaReject::TypeNotAllowed)
}

// ---------------------------------------------------------------- hashing

/// Lowercase hex SHA-256 of a file, streamed in 8 KiB chunks.
pub fn sha256_file(path: &Path) -> io::Result<String> {
    let mut f = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8 * 1024];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex(&hasher.finalize()))
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(out, "{b:02x}");
    }
    out
}

// -------------------------------------------------------------- thumbnails

pub const THUMB_MAX_W: u32 = 272;
pub const THUMB_MAX_H: u32 = 192;
const THUMB_QUALITY: u8 = 80;
const MAX_DIMENSION: u32 = 16_384;
const MAX_ALLOC: u64 = 512 * 1024 * 1024;

#[derive(Debug)]
pub enum ThumbError {
    Io(io::Error),
    Decode(String),
}

impl std::fmt::Display for ThumbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThumbError::Io(e) => write!(f, "io: {e}"),
            ThumbError::Decode(e) => write!(f, "decode: {e}"),
        }
    }
}

impl std::error::Error for ThumbError {}

impl From<io::Error> for ThumbError {
    fn from(e: io::Error) -> Self {
        ThumbError::Io(e)
    }
}

fn decode_err(e: impl std::fmt::Display) -> ThumbError {
    ThumbError::Decode(e.to_string())
}

fn limits() -> Limits {
    let mut l = Limits::default();
    l.max_image_width = Some(MAX_DIMENSION);
    l.max_image_height = Some(MAX_DIMENSION);
    l.max_alloc = Some(MAX_ALLOC);
    l
}

/// Builds a JPEG thumbnail (<= 272x192, EXIF orientation applied, alpha
/// flattened on white, q80). Limits are installed before any pixel decoding;
/// failure leaves no output file. GIF contributes its first frame only.
pub fn make_thumbnail(src: &Path, dest: &Path) -> Result<(), ThumbError> {
    let reader = ImageReader::open(src)?
        .with_guessed_format()
        .map_err(ThumbError::Io)?;
    let mut decoder = reader.into_decoder().map_err(decode_err)?;
    decoder.set_limits(limits()).map_err(decode_err)?;
    let orientation = decoder.orientation().map_err(decode_err)?;
    let mut img = image::DynamicImage::from_decoder(decoder).map_err(decode_err)?;
    img.apply_orientation(orientation);
    if img.width() > THUMB_MAX_W || img.height() > THUMB_MAX_H {
        img = img.thumbnail(THUMB_MAX_W, THUMB_MAX_H);
    }

    // Flatten alpha onto white.
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let mut rgb = Vec::with_capacity(w as usize * h as usize * 3);
    for px in rgba.pixels() {
        let a = u32::from(px[3]);
        for c in 0..3 {
            let v = (u32::from(px[c]) * a + 255 * (255 - a) + 127) / 255;
            rgb.push(v as u8);
        }
    }
    let mut out = Vec::new();
    JpegEncoder::new_with_quality(&mut out, THUMB_QUALITY)
        .encode(&rgb, w, h, ExtendedColorType::Rgb8)
        .map_err(decode_err)?;

    let mut tmp = dest.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = std::path::PathBuf::from(tmp);
    if let Err(e) = std::fs::write(&tmp, &out).and_then(|()| std::fs::rename(&tmp, dest)) {
        let _ = std::fs::remove_file(&tmp);
        return Err(ThumbError::Io(e));
    }
    Ok(())
}

// -------------------------------------------------------------------- HEIC

/// Both variants map to the upstream code `heic_failed` (D-24); the
/// «сохраните как JPEG» text is UI-side copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeicError {
    /// Built without the `heic` feature.
    Unavailable,
    /// libheif could not decode the image.
    Failed,
}

#[cfg(not(feature = "heic"))]
pub fn heic_to_jpeg(_bytes: &[u8]) -> Result<Vec<u8>, HeicError> {
    Err(HeicError::Unavailable)
}

/// HEIC → JPEG q90 via libheif. libheif applies irot/imir itself, so no EXIF
/// is carried over. Final verification on Windows happens in plan 41.3-06.
#[cfg(feature = "heic")]
pub fn heic_to_jpeg(bytes: &[u8]) -> Result<Vec<u8>, HeicError> {
    use libheif_rs::{ColorSpace, HeifContext, LibHeif, RgbChroma};

    const MAX_PIXELS: u64 = 100_000_000;
    let ctx = HeifContext::read_from_bytes(bytes).map_err(|_| HeicError::Failed)?;
    let handle = ctx.primary_image_handle().map_err(|_| HeicError::Failed)?;
    let (w, h) = (handle.width(), handle.height());
    if u64::from(w) * u64::from(h) > MAX_PIXELS || w == 0 || h == 0 {
        return Err(HeicError::Failed);
    }
    let lib = LibHeif::new();
    let img = lib
        .decode(&handle, ColorSpace::Rgb(RgbChroma::Rgb), None)
        .map_err(|_| HeicError::Failed)?;
    let planes = img.planes();
    let inter = planes.interleaved.ok_or(HeicError::Failed)?;
    let mut rgb = Vec::with_capacity(w as usize * h as usize * 3);
    for y in 0..h as usize {
        let start = y * inter.stride;
        let row = inter
            .data
            .get(start..start + w as usize * 3)
            .ok_or(HeicError::Failed)?;
        rgb.extend_from_slice(row);
    }
    let mut out = Vec::new();
    JpegEncoder::new_with_quality(&mut out, 90)
        .encode(&rgb, w, h, ExtendedColorType::Rgb8)
        .map_err(|_| HeicError::Failed)?;
    Ok(out)
}

/// Shared byte generators (also used by integration tests).
#[cfg(test)]
#[path = "../../tests/attachments_support/mod.rs"]
mod fx;

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::fx;
    use super::*;

    fn kind(b: &[u8], name: &str) -> Result<Sniffed, MediaReject> {
        sniff(b, Some(b), name)
    }

    #[test]
    fn photos_are_viewable() {
        for (b, mime) in [
            (fx::tiny_jpeg(), "image/jpeg"),
            (fx::tiny_png(), "image/png"),
            (fx::tiny_gif(), "image/gif"),
            (fx::tiny_webp(), "image/webp"),
        ] {
            let r = kind(&b, "x.bin").unwrap();
            assert_eq!(r.mime, mime);
            assert_eq!(r.kind, MediaKind::Photo);
            assert!(r.viewable);
            assert!(!r.is_heic);
        }
    }

    #[test]
    fn webp_kept_as_is_and_viewable() {
        let r = kind(&fx::tiny_webp(), "photo.webp").unwrap();
        assert!(r.viewable && !r.is_heic);
        assert_eq!(r.ext_hint, "webp");
    }

    #[test]
    fn pdf_viewable_office_and_archives_not() {
        let r = kind(&fx::tiny_pdf(), "a.pdf").unwrap();
        assert_eq!((r.mime, r.viewable), ("application/pdf", true));
        for (b, ext) in [
            (fx::docx(), "docx"),
            (fx::xlsx(), "xlsx"),
            (fx::odt(), "odt"),
            (fx::ods(), "ods"),
        ] {
            let r = kind(&b, "f.dat").unwrap();
            assert_eq!(r.ext_hint, ext, "{ext}");
            assert_eq!(r.kind, MediaKind::Office);
            assert!(!r.viewable);
        }
        assert_eq!(
            kind(&fx::plain_zip(), "a.zip").unwrap().mime,
            "application/zip"
        );
        assert_eq!(
            kind(&fx::sevenz_header(), "a.7z").unwrap().mime,
            "application/x-7z-compressed"
        );
    }

    #[test]
    fn ole2_needs_full_buffer_and_truncated_is_not_doc() {
        let doc = fx::ole2_doc_minimal();
        assert_eq!(sniff(&doc, Some(&doc), "a.doc").unwrap().ext_hint, "doc");
        let xls = fx::ole2_minimal(false);
        assert_eq!(sniff(&xls, Some(&xls), "a.xls").unwrap().ext_hint, "xls");
        // Head only / truncated buffer: never classified as doc.
        assert_eq!(
            sniff(&doc[..512], None, "a.doc"),
            Err(MediaReject::TypeNotAllowed)
        );
        assert_eq!(
            sniff(&doc[..512], Some(&doc[..512]), "a.doc"),
            Err(MediaReject::TypeNotAllowed)
        );
    }

    #[test]
    fn text_by_extension_only() {
        let cp1251 = [0xCF, 0xF0, 0xE8, 0xE2, 0xE5, 0xF2, b'\n']; // not UTF-8
        assert_eq!(kind(&cp1251, "n.txt").unwrap().mime, "text/plain");
        assert_eq!(kind(b"a;b;c\n1;2;3\n", "t.csv").unwrap().mime, "text/plain");
        assert_eq!(kind(b"line\n", "x.log").unwrap().mime, "text/plain");
        for name in ["a.bat", "a.ps1", "a.js", "a.html", "a.svg", "noext"] {
            assert_eq!(
                kind(b"echo hi\n", name),
                Err(MediaReject::TypeNotAllowed),
                "{name}"
            );
        }
        assert_eq!(kind(b"ab\0cd", "n.txt"), Err(MediaReject::TypeNotAllowed));
    }

    #[test]
    fn extension_lies_signature_wins() {
        for b in [
            fx::pe_stub(),
            fx::elf_stub(),
            fx::macho_stub(),
            fx::script_stub(),
            fx::svg_stub(),
            fx::html_stub(),
        ] {
            for name in ["x.pdf", "x.txt", "x.jpg"] {
                assert_eq!(kind(&b, name), Err(MediaReject::TypeNotAllowed), "{name}");
            }
        }
        assert_eq!(
            kind(b"  <HTML>hi", "a.txt"),
            Err(MediaReject::TypeNotAllowed)
        );
        assert_eq!(
            kind(b"<?xml version=\"1.0\"?><svg/>", "a.txt"),
            Err(MediaReject::TypeNotAllowed)
        );
        let r = kind(&fx::tiny_png(), "a.pdf").unwrap();
        assert_eq!(r.mime, "image/png");
    }

    #[test]
    fn heic_flagged_avif_rejected_empty() {
        let r = kind(&fx::heic_stub(), "IMG.HEIC").unwrap();
        assert!(r.is_heic && !r.viewable);
        assert_eq!(
            kind(&fx::avif_stub(), "a.avif"),
            Err(MediaReject::TypeNotAllowed)
        );
        assert_eq!(kind(b"", "a.txt"), Err(MediaReject::Empty));
    }

    fn tmp() -> tempfile::TempDir {
        tempfile::TempDir::new().unwrap()
    }

    fn thumb_of(
        bytes: &[u8],
    ) -> (
        tempfile::TempDir,
        std::path::PathBuf,
        Result<(), ThumbError>,
    ) {
        let d = tmp();
        let src = d.path().join("src.bin");
        std::fs::write(&src, bytes).unwrap();
        let dest = d.path().join("out.jpg");
        let r = make_thumbnail(&src, &dest);
        (d, dest, r)
    }

    #[test]
    fn thumb_fits_and_keeps_aspect() {
        let (_d, dest, r) = thumb_of(&fx::png_sized(1000, 500));
        r.unwrap();
        let img = image::open(&dest).unwrap();
        assert!(img.width() <= THUMB_MAX_W && img.height() <= THUMB_MAX_H);
        assert_eq!(img.width(), 272);
        assert!((img.height() as i32 - 136).abs() <= 1);
    }

    #[test]
    fn thumb_applies_exif_orientation() {
        let (_d, dest, r) = thumb_of(&fx::jpeg_with_orientation(600, 300, 6));
        r.unwrap();
        let img = image::open(&dest).unwrap();
        assert!(
            img.height() > img.width(),
            "{}x{}",
            img.width(),
            img.height()
        );
        assert!(img.height() <= THUMB_MAX_H);
    }

    #[test]
    fn thumb_bomb_is_error_without_output() {
        let (_d, dest, r) = thumb_of(&fx::png_claiming(20_000, 20_000));
        assert!(r.is_err());
        assert!(!dest.exists());
    }

    #[test]
    fn thumb_garbage_is_error_not_panic() {
        let (_d, dest, r) = thumb_of(b"definitely not an image");
        assert!(r.is_err());
        assert!(!dest.exists());
    }

    #[test]
    fn thumb_flattens_alpha_on_white() {
        let (_d, dest, r) = thumb_of(&fx::png_transparent(16, 16));
        r.unwrap();
        let img = image::open(&dest).unwrap().to_rgb8();
        let p = img.get_pixel(8, 8);
        assert!(p.0.iter().all(|&c| c > 240), "{p:?}");
    }

    #[test]
    fn sha256_known_value_streaming() {
        let d = tmp();
        let p = d.path().join("f");
        std::fs::write(&p, b"abc").unwrap();
        assert_eq!(
            sha256_file(&p).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        let big = vec![b'a'; 100_000];
        std::fs::write(&p, &big).unwrap();
        let direct = hex(&Sha256::digest(&big));
        assert_eq!(sha256_file(&p).unwrap(), direct);
    }

    #[cfg(not(feature = "heic"))]
    #[test]
    fn heic_unavailable_without_feature() {
        assert_eq!(heic_to_jpeg(b"...").unwrap_err(), HeicError::Unavailable);
    }
}
