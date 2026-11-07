//! HEIC handling (D-10, D-24). The default build rejects HEIC; the conversion
//! test runs only with `--features heic` (Windows spike job, heic-spike.yml).
#![allow(clippy::unwrap_used)]

use trackly_app::services::attachment_media::{heic_to_jpeg, sniff, HeicError};

/// Synthetic `ftyp heic` header, no image payload.
fn heic_header() -> Vec<u8> {
    let mut v = vec![0, 0, 0, 24];
    v.extend_from_slice(b"ftypheic");
    v.extend_from_slice(&[0, 0, 0, 0]);
    v.extend_from_slice(b"mif1heic");
    v
}

#[test]
fn sniff_flags_heic_and_not_viewable() {
    let b = heic_header();
    let r = sniff(&b, Some(&b), "IMG_0001.HEIC").unwrap();
    assert!(r.is_heic);
    assert!(!r.viewable);
}

#[cfg(not(feature = "heic"))]
#[test]
fn default_build_rejects_heic() {
    assert_eq!(
        heic_to_jpeg(&heic_header()).unwrap_err(),
        HeicError::Unavailable
    );
}

#[cfg(feature = "heic")]
#[test]
fn garbage_heic_fails_cleanly() {
    assert_eq!(heic_to_jpeg(&heic_header()).unwrap_err(), HeicError::Failed);
}

/// No HEIC fixture is committed (no real photos in a public repo, and the
/// encoder side of libheif is deliberately not built). The spike workflow
/// passes a sample path in `TRACKLY_HEIC_SAMPLE`; without it the test is a
/// no-op with an explicit message.
#[cfg(feature = "heic")]
#[test]
fn sample_converts_to_jpeg() {
    let Ok(path) = std::env::var("TRACKLY_HEIC_SAMPLE") else {
        eprintln!("SKIP: TRACKLY_HEIC_SAMPLE not set; conversion not exercised");
        return;
    };
    let bytes = std::fs::read(path).unwrap();
    let jpg = heic_to_jpeg(&bytes).unwrap();
    assert_eq!(&jpg[..3], &[0xFF, 0xD8, 0xFF]);
    let img = image::load_from_memory(&jpg).unwrap();
    assert!(img.width() > 0 && img.height() > 0);
}
