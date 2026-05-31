//! Phase 3.1 Plan 05 — G-9 logo aspect-ratio unit tests.
//!
//! `scale_logo_dimensions(orig_w, orig_h, max_w, max_h) -> (final_w, final_h)`
//! гарантирует contain-fit (равные scale по w/h) → aspect-ratio preserved.
//! Эти тесты документируют 4 граничных случая ровно того поведения, что
//! G-9 fix-pattern в 03-UAT.md и 03.1-CONTEXT.md ожидают.

use trackly_app::pdf::renderer::scale_logo_dimensions;

const MAX_W: f32 = 100.0;
const MAX_H: f32 = 50.0;

#[test]
fn square_constrained_equal_scale() {
    // 200x100 → scale=min(0.5, 0.5)=0.5 → 100x50 (равно constrained).
    let (w, h) = scale_logo_dimensions(200.0, 100.0, MAX_W, MAX_H);
    assert_eq!(w, 100.0);
    assert_eq!(h, 50.0);
}

#[test]
fn width_constrained_smaller_height() {
    // 200x50 → scale=min(0.5, 1.0)=0.5 → 100x25 (width hits max first).
    let (w, h) = scale_logo_dimensions(200.0, 50.0, MAX_W, MAX_H);
    assert_eq!(w, 100.0);
    assert_eq!(h, 25.0);
}

#[test]
fn height_constrained_smaller_width() {
    // 100x200 → scale=min(1.0, 0.25)=0.25 → 25x50 (height hits max first).
    let (w, h) = scale_logo_dimensions(100.0, 200.0, MAX_W, MAX_H);
    assert_eq!(w, 25.0);
    assert_eq!(h, 50.0);
}

#[test]
fn smaller_than_max_no_upscale() {
    // 50x25 уже меньше 100x50 — НЕ масштабируем вверх (защита от blur'а).
    // scale=min(2.0, 2.0).min(1.0)=1.0 → final = orig.
    let (w, h) = scale_logo_dimensions(50.0, 25.0, MAX_W, MAX_H);
    assert_eq!(w, 50.0);
    assert_eq!(h, 25.0);
}

#[test]
fn zero_dimensions_returns_zero() {
    // Безопасный exit для invalid inputs.
    let (w, h) = scale_logo_dimensions(0.0, 100.0, MAX_W, MAX_H);
    assert_eq!(w, 0.0);
    assert_eq!(h, 0.0);

    let (w2, h2) = scale_logo_dimensions(100.0, 100.0, 0.0, MAX_H);
    assert_eq!(w2, 0.0);
    assert_eq!(h2, 0.0);
}

#[test]
fn extreme_aspect_ratios_preserved() {
    // 1000x10 → scale=min(0.1, 5.0)=0.1 → 100x1.
    let (w, h) = scale_logo_dimensions(1000.0, 10.0, MAX_W, MAX_H);
    assert_eq!(w, 100.0);
    assert!((h - 1.0).abs() < 1e-3);

    // 10x1000 → scale=min(10.0, 0.05)=0.05 → 0.5x50.
    let (w2, h2) = scale_logo_dimensions(10.0, 1000.0, MAX_W, MAX_H);
    assert!((w2 - 0.5).abs() < 1e-3);
    assert_eq!(h2, 50.0);
}
