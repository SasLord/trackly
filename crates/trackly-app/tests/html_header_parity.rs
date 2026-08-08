//! Header-partial structural gate (Phase 34, Plan 02): the three shipped HTML
//! print templates (`act_handover.html`, `act_acceptance.html`, `report.html`)
//! must all pull in the shared header markup/CSS via
//! `{% include "_header.html" %}` (D-12) rather than duplicating it, and the
//! shared partial's `.orgName` node must never carry a hardcoded organization
//! name (DOC-05) — only Jinja expressions and markup.
//!
//! Reads the templates via `include_str!` (compile-time, relative to this
//! test file's own location), modeled on `html_page_parity.rs`'s style — no
//! tokio needed, this test only READS the templates, never modifies them.

const ACT_HANDOVER_HTML: &str = include_str!("../templates/act_handover.html");
const ACT_ACCEPTANCE_HTML: &str = include_str!("../templates/act_acceptance.html");
const REPORT_HTML: &str = include_str!("../templates/report.html");
const HEADER_HTML: &str = include_str!("../templates/_header.html");

#[test]
fn all_three_templates_include_header_partial() {
    assert!(
        ACT_HANDOVER_HTML.contains("{% include \"_header.html\" %}"),
        "act_handover.html must include the shared _header.html partial"
    );
    assert!(
        ACT_ACCEPTANCE_HTML.contains("{% include \"_header.html\" %}"),
        "act_acceptance.html must include the shared _header.html partial"
    );
    assert!(
        REPORT_HTML.contains("{% include \"_header.html\" %}"),
        "report.html must include the shared _header.html partial"
    );
}

/// DOC-05, privacy-safe positive form: proves the `.orgName` node in the
/// shared header partial holds no hardcoded organization-name text, without
/// ever writing the real organization name into this test file. Extracts the
/// `<div class="orgName">...</div>` fragment (non-greedy — safe here because,
/// unlike `.header`, `.orgName`'s own children are only `{{ }}` / `{% %}` /
/// `<br />` / `(` / `)`, never a nested `<div>`), strips every Jinja
/// expression/statement span, then asserts the remainder contains no Unicode
/// letter character.
#[test]
fn header_partial_org_name_node_has_no_hardcoded_literal() {
    let org_name_re = regex::Regex::new(r#"(?s)<div class="orgName">.*?</div>"#)
        .expect("valid orgName extraction regex");
    let org_name_block = org_name_re
        .find(HEADER_HTML)
        .unwrap_or_else(|| panic!("no <div class=\"orgName\">...</div> block found in _header.html"))
        .as_str();

    let jinja_expr_re = regex::Regex::new(r"(?s)\{\{.*?\}\}").expect("valid Jinja expr regex");
    let jinja_stmt_re = regex::Regex::new(r"(?s)\{%.*?%\}").expect("valid Jinja stmt regex");

    let stripped = jinja_expr_re.replace_all(org_name_block, "");
    let stripped = jinja_stmt_re.replace_all(&stripped, "");

    let remainder: String = stripped
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .replace("<div", "")
        .replace("class=\"orgName\">", "")
        .replace("</div>", "")
        .replace("<br", "")
        .replace("/>", "")
        .replace('(', "")
        .replace(')', "");

    let has_letter = remainder.chars().any(|c| c.is_alphabetic());
    assert!(
        !has_letter,
        "_header.html's .orgName node must contain no hardcoded literal text \
         (only Jinja expressions/markup) — found leftover non-markup content: {remainder:?}"
    );
}
