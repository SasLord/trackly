//! D-13 structural regression guard (Phase 33, Plan 02): the three shipped
//! HTML print templates (`act_handover.html`, `act_acceptance.html`,
//! `report.html`) must declare byte-identical `@page { size; margin }`
//! blocks. PRV-02's cross-document-consistency guarantee and Paged.js's
//! pagination (D-04) both depend on the three documents sharing the exact
//! same page geometry — a desync here would silently break either the
//! WYSIWYG preview/print match or make one document's pages a different
//! physical size than the other two.
//!
//! Reads the templates via `include_str!` (compile-time, relative to this
//! test file's own location) rather than a runtime `std::fs::read_to_string`
//! with a CWD-relative path, so the test is independent of `cargo test`'s
//! working directory. Per D-01, this test only READS the templates — it
//! never modifies `crates/trackly-app/templates/*.html`.

const ACT_HANDOVER_HTML: &str = include_str!("../templates/act_handover.html");
const ACT_ACCEPTANCE_HTML: &str = include_str!("../templates/act_acceptance.html");
const REPORT_HTML: &str = include_str!("../templates/report.html");

/// Extracts the first `@page { ... }` block from `text`, panicking with
/// `label` in the message if no match is found.
fn extract_page_block(text: &str, label: &str) -> String {
    let re = regex::Regex::new(r"(?s)@page\s*\{[^}]*\}").expect("valid regex");
    re.find(text)
        .unwrap_or_else(|| panic!("{label}: no @page block found in template"))
        .as_str()
        .to_string()
}

#[test]
fn all_three_templates_share_identical_page_block() {
    let handover = extract_page_block(ACT_HANDOVER_HTML, "act_handover.html");
    let acceptance = extract_page_block(ACT_ACCEPTANCE_HTML, "act_acceptance.html");
    let report = extract_page_block(REPORT_HTML, "report.html");

    assert_eq!(
        handover, acceptance,
        "act_handover.html and act_acceptance.html @page blocks differ:\n\
         act_handover.html:\n{handover}\n\
         act_acceptance.html:\n{acceptance}"
    );
    assert_eq!(
        acceptance, report,
        "act_acceptance.html and report.html @page blocks differ:\n\
         act_acceptance.html:\n{acceptance}\n\
         report.html:\n{report}"
    );
}
