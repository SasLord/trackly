//! DOC-07 structural regression guard (Phase 35, Plan 04): auto-filled
//! `.field-row` values in `act_handover.html` must not carry an underline
//! (`border-bottom`). Underlines are reserved for exactly two places: the
//! blank "Сроком до" fallback (`.value-blank`, D-03/D-10) and the signature
//! line (`.signature-field .signature-line`, D-06). Anywhere else, a
//! resurrected `border-bottom` on `.field-row` (or a sibling rule) would
//! silently reintroduce the underlines D-10 removed.
//!
//! This gate checks CSS **by selector**, not by markup range. A naive
//! "text between `{% include "_header.html" %}` and `<div class="signatures">`"
//! range is unusable by construction here: the whole `<style>` block —
//! including both `border-bottom` declarations — sits *before* the
//! `{% include %}` marker in the current template, so that range would
//! always match zero occurrences regardless of a real regression. Instead:
//! extract the `<style>` block content first, then extract the body of a
//! specific CSS rule by its exact selector.
//!
//! Reads the template via `include_str!` (compile-time, relative to this
//! test file's own location), so the test is independent of `cargo test`'s
//! working directory. This test only READS `act_handover.html` — it never
//! modifies `crates/trackly-app/templates/*.html`.

const ACT_HANDOVER_HTML: &str = include_str!("../templates/act_handover.html");

/// Extracts the content of the first `<style>...</style>` block from `text`,
/// panicking if no `<style>` block is found.
fn extract_style_block(text: &str) -> String {
    let re = regex::Regex::new(r"(?s)<style>(.*?)</style>").expect("valid regex");
    re.captures(text)
        .unwrap_or_else(|| panic!("no <style> block found"))
        .get(1)
        .expect("capture group 1")
        .as_str()
        .to_string()
}

/// Extracts the body (content between `{` and `}`) of the CSS rule whose
/// selector is exactly `selector`, panicking with `selector` in the message
/// if no matching rule is found.
fn extract_rule_body(css: &str, selector: &str) -> String {
    let pattern = format!(r"(?s){}\s*\{{([^}}]*)\}}", regex::escape(selector));
    let re = regex::Regex::new(&pattern).expect("valid regex");
    re.captures(css)
        .unwrap_or_else(|| panic!("no CSS rule found for selector {selector:?}"))
        .get(1)
        .expect("capture group 1")
        .as_str()
        .to_string()
}

#[test]
fn field_row_css_has_no_border_bottom_and_only_two_legit_exceptions_remain() {
    let style = extract_style_block(ACT_HANDOVER_HTML);

    // 1. The `.field-row` rule itself must not declare an underline (D-10).
    let field_row_body = extract_rule_body(&style, ".field-row");
    assert!(
        !field_row_body.contains("border-bottom"),
        "`.field-row` must not declare border-bottom (D-10). Rule body: {field_row_body}"
    );

    // 2. Exactly two `border-bottom` declarations may exist in the whole
    //    <style> block: the blank-deadline fallback and the signature line.
    let count = style.matches("border-bottom").count();
    assert_eq!(
        count, 2,
        "found {count} border-bottom occurrences in <style>, expected exactly 2 \
         (.value-blank and .signature-field .signature-line)"
    );

    // 3. The first legitimate source: the blank "Сроком до" fallback
    //    (D-03/D-10).
    let value_blank_body = extract_rule_body(&style, ".value-blank");
    assert!(
        value_blank_body.contains("border-bottom"),
        "`.value-blank` must declare border-bottom (D-03/D-10). Rule body: {value_blank_body}"
    );

    // 4. The second legitimate source: the signature line (D-06).
    let signature_line_body = extract_rule_body(&style, ".signature-field .signature-line");
    assert!(
        signature_line_body.contains("border-bottom"),
        "`.signature-field .signature-line` must declare border-bottom (D-06). \
         Rule body: {signature_line_body}"
    );
}
