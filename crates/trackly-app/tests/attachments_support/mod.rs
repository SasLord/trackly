//! Byte generators for attachment tests (plans 41.3-02 .. 09).
//!
//! Everything is built in memory — no real files, no real names. The module is
//! shared: integration tests `mod attachments_support;` it, and the unit tests
//! in `services/attachment_media.rs` include it via `#[path]`.
#![allow(dead_code)]

use std::io::Cursor;

use image::{ImageFormat, Rgb, RgbImage, Rgba, RgbaImage};

fn encode(img: image::DynamicImage, fmt: ImageFormat) -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    img.write_to(&mut out, fmt).expect("encode fixture");
    out.into_inner()
}

fn rgb(w: u32, h: u32) -> image::DynamicImage {
    image::DynamicImage::ImageRgb8(RgbImage::from_fn(w, h, |x, y| {
        Rgb([(x % 251) as u8, (y % 241) as u8, 128])
    }))
}

pub fn tiny_png() -> Vec<u8> {
    encode(rgb(8, 8), ImageFormat::Png)
}
pub fn png_sized(w: u32, h: u32) -> Vec<u8> {
    encode(rgb(w, h), ImageFormat::Png)
}
/// Fully transparent PNG (alpha flattening check).
pub fn png_transparent(w: u32, h: u32) -> Vec<u8> {
    let img = RgbaImage::from_pixel(w, h, Rgba([0, 0, 0, 0]));
    encode(image::DynamicImage::ImageRgba8(img), ImageFormat::Png)
}
pub fn tiny_jpeg() -> Vec<u8> {
    encode(rgb(8, 8), ImageFormat::Jpeg)
}
pub fn jpeg_sized(w: u32, h: u32) -> Vec<u8> {
    encode(rgb(w, h), ImageFormat::Jpeg)
}
/// JPEG with an EXIF APP1 segment carrying the given orientation (1..=8).
pub fn jpeg_with_orientation(w: u32, h: u32, orientation: u16) -> Vec<u8> {
    let base = jpeg_sized(w, h);
    let mut tiff: Vec<u8> = Vec::new();
    tiff.extend_from_slice(b"MM\0\x2A\0\0\0\x08"); // big endian, IFD at 8
    tiff.extend_from_slice(&1u16.to_be_bytes()); // one entry
    tiff.extend_from_slice(&0x0112u16.to_be_bytes()); // Orientation
    tiff.extend_from_slice(&3u16.to_be_bytes()); // SHORT
    tiff.extend_from_slice(&1u32.to_be_bytes()); // count
    tiff.extend_from_slice(&orientation.to_be_bytes());
    tiff.extend_from_slice(&[0, 0]);
    tiff.extend_from_slice(&0u32.to_be_bytes()); // next IFD
    let mut seg = Vec::new();
    seg.extend_from_slice(b"Exif\0\0");
    seg.extend_from_slice(&tiff);
    let len = (seg.len() + 2) as u16;
    let mut out = vec![0xFF, 0xD8, 0xFF, 0xE1];
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(&seg);
    out.extend_from_slice(&base[2..]); // skip original SOI
    out
}
pub fn tiny_gif() -> Vec<u8> {
    encode(rgb(8, 8), ImageFormat::Gif)
}
pub fn tiny_webp() -> Vec<u8> {
    encode(rgb(8, 8), ImageFormat::WebP)
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// Header-only PNG that claims `w` x `h` pixels (IHDR + IEND, no pixel data).
pub fn png_claiming(w: u32, h: u32) -> Vec<u8> {
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&w.to_be_bytes());
    ihdr.extend_from_slice(&h.to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
    for (kind, data) in [(&b"IHDR"[..], ihdr), (&b"IEND"[..], Vec::new())] {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let mut body = kind.to_vec();
        body.extend_from_slice(&data);
        out.extend_from_slice(&body);
        out.extend_from_slice(&crc32(&body).to_be_bytes());
    }
    out
}

pub fn tiny_pdf() -> Vec<u8> {
    b"%PDF-1.4\n1 0 obj<</Type/Catalog>>endobj\ntrailer<</Root 1 0 R>>\n%%EOF\n".to_vec()
}

/// Stored (uncompressed) zip with local headers + central directory.
pub fn zip_with_entries(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, data) in entries {
        let offset = out.len() as u32;
        let crc = crc32(data);
        out.extend_from_slice(&[0x50, 0x4B, 0x03, 0x04, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(data);
        central.extend_from_slice(&[0x50, 0x4B, 0x01, 0x02, 20, 0, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        central.extend_from_slice(&crc.to_le_bytes());
        central.extend_from_slice(&(data.len() as u32).to_le_bytes());
        central.extend_from_slice(&(data.len() as u32).to_le_bytes());
        central.extend_from_slice(&(name.len() as u16).to_le_bytes());
        central.extend_from_slice(&[0u8; 12]);
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(name.as_bytes());
    }
    let cd_off = out.len() as u32;
    out.extend_from_slice(&central);
    out.extend_from_slice(&[0x50, 0x4B, 0x05, 0x06, 0, 0, 0, 0]);
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    out.extend_from_slice(&(central.len() as u32).to_le_bytes());
    out.extend_from_slice(&cd_off.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

pub fn docx() -> Vec<u8> {
    zip_with_entries(&[
        ("[Content_Types].xml", b"<Types/>"),
        ("_rels/.rels", b"<Relationships/>"),
        ("word/document.xml", b"<w:document/>"),
        ("word/styles.xml", b"<w:styles/>"),
    ])
}
pub fn xlsx() -> Vec<u8> {
    zip_with_entries(&[
        ("[Content_Types].xml", b"<Types/>"),
        ("_rels/.rels", b"<Relationships/>"),
        ("xl/workbook.xml", b"<workbook/>"),
        ("xl/styles.xml", b"<styleSheet/>"),
    ])
}
fn odf(mime: &str) -> Vec<u8> {
    zip_with_entries(&[
        ("mimetype", mime.as_bytes()),
        ("META-INF/manifest.xml", b"<manifest/>"),
        ("content.xml", b"<office:document-content/>"),
    ])
}
pub fn odt() -> Vec<u8> {
    odf("application/vnd.oasis.opendocument.text")
}
pub fn ods() -> Vec<u8> {
    odf("application/vnd.oasis.opendocument.spreadsheet")
}
pub fn plain_zip() -> Vec<u8> {
    zip_with_entries(&[("a.txt", b"hello")])
}
pub fn sevenz_header() -> Vec<u8> {
    let mut v = vec![0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C, 0, 4];
    v.extend_from_slice(&[0u8; 24]);
    v
}
/// Real OLE2 compound file whose root CLSID marks it as Word (`word`) or Excel.
pub fn ole2_minimal(word: bool) -> Vec<u8> {
    let mut cf = cfb::CompoundFile::create(Cursor::new(Vec::new())).expect("cfb");
    let clsid = if word {
        "00020906-0000-0000-c000-000000000046"
    } else {
        "00020820-0000-0000-c000-000000000046"
    };
    cf.set_storage_clsid("/", clsid.parse().expect("uuid"))
        .expect("clsid");
    cf.flush().expect("flush");
    cf.into_inner().into_inner()
}
pub fn ole2_doc_minimal() -> Vec<u8> {
    ole2_minimal(true)
}
pub fn pe_stub() -> Vec<u8> {
    let mut v = b"MZ".to_vec();
    v.extend_from_slice(&[0x90, 0, 3, 0, 0, 0, 4, 0, 0, 0]);
    v
}
pub fn elf_stub() -> Vec<u8> {
    vec![0x7F, b'E', b'L', b'F', 2, 1, 1, 0, 0, 0, 0, 0]
}
pub fn macho_stub() -> Vec<u8> {
    vec![0xCF, 0xFA, 0xED, 0xFE, 7, 0, 0, 1]
}
pub fn script_stub() -> Vec<u8> {
    b"#!/bin/sh\necho hi\n".to_vec()
}
pub fn html_stub() -> Vec<u8> {
    b"<html><body><script>alert(1)</script></body></html>".to_vec()
}
pub fn svg_stub() -> Vec<u8> {
    b"<svg xmlns=\"http://www.w3.org/2000/svg\" onload=\"alert(1)\"/>".to_vec()
}
pub fn heic_stub() -> Vec<u8> {
    let mut v = vec![0, 0, 0, 24];
    v.extend_from_slice(b"ftypheic");
    v.extend_from_slice(&[0, 0, 0, 0]);
    v.extend_from_slice(b"mif1heic");
    v
}
pub fn avif_stub() -> Vec<u8> {
    let mut v = vec![0, 0, 0, 24];
    v.extend_from_slice(b"ftypavif");
    v.extend_from_slice(&[0, 0, 0, 0]);
    v.extend_from_slice(b"mif1avif");
    v
}
