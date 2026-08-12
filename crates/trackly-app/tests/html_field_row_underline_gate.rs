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
//! modifies `crates/trackly-app/templates/*.html`. Phase 35 Plan 06 (G-04/
//! IN-01) extended this file to also cover `act_acceptance.html`'s equivalent
//! signature-line underline, closing the structural-gate gap the code review
//! flagged for that template.

const ACT_HANDOVER_HTML: &str = include_str!("../templates/act_handover.html");
const ACT_ACCEPTANCE_HTML: &str = include_str!("../templates/act_acceptance.html");

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

/// G-04/IN-01: structural DOC-07-equivalent gate for `act_acceptance.html`.
/// Unlike `act_handover.html`, this template has no `.field-row`/
/// `.value-blank` pattern (it uses `table.kv` for the details table), so
/// exactly ONE `border-bottom` source is legitimate: the signature line
/// (D-09), shared markup/CSS with `act_handover.html`'s signature block.
#[test]
fn acceptance_signature_line_css_has_exactly_one_legitimate_border_bottom() {
    let style = extract_style_block(ACT_ACCEPTANCE_HTML);

    let count = style.matches("border-bottom").count();
    assert_eq!(
        count, 1,
        "found {count} border-bottom occurrences in act_acceptance.html's <style>, \
         expected exactly 1 (.signature-field .signature-line)"
    );

    let signature_line_body = extract_rule_body(&style, ".signature-field .signature-line");
    assert!(
        signature_line_body.contains("border-bottom"),
        "`.signature-field .signature-line` must declare border-bottom (D-09). \
         Rule body: {signature_line_body}"
    );
}

/// Closes VERIFICATION.md missing item 1 (2026-08-12 re-verification,
/// DOC-08/SC#4, WR-02) with a structural CSS gate rather than a textual
/// `html.contains` check: `.signature-row .signature-name` in BOTH
/// `act_handover.html` and `act_acceptance.html` must permit wrapping
/// (`min-width: 0`, `white-space: normal`, `overflow-wrap: break-word`) and
/// must never fall back to a bare `nowrap` — otherwise a long Cyrillic ФИО
/// (double surname + patronymic) cannot shrink/wrap and overflows the print
/// width.
#[test]
fn signature_name_css_permits_wrap_for_long_names() {
    for (filename, html) in [
        ("act_handover.html", ACT_HANDOVER_HTML),
        ("act_acceptance.html", ACT_ACCEPTANCE_HTML),
    ] {
        let style = extract_style_block(html);
        let body = extract_rule_body(&style, ".signature-row .signature-name");

        assert!(
            body.contains("min-width: 0"),
            "{filename}: `.signature-row .signature-name` must declare min-width: 0 \
             so a long ФИО can shrink below its unbroken text width. Rule body: {body}"
        );
        assert!(
            body.contains("white-space: normal"),
            "{filename}: `.signature-row .signature-name` must declare white-space: normal \
             to permit wrapping. Rule body: {body}"
        );
        assert!(
            body.contains("overflow-wrap: break-word"),
            "{filename}: `.signature-row .signature-name` must declare \
             overflow-wrap: break-word so an unbreakable long word still wraps. \
             Rule body: {body}"
        );
        assert!(
            !body.contains("nowrap"),
            "{filename}: `.signature-row .signature-name` must not force nowrap — \
             long Cyrillic ФИО (DOC-08/SC#4, VERIFICATION.md gap) needs to wrap, \
             not overflow. Rule body: {body}"
        );
    }
}
