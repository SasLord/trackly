//! MiniJinja Environment in safe-mode + bounded render wrapper.
//!
//! Safe-mode invariants (RESEARCH §Pattern 4, threat T-03-01-01/02):
//! - `UndefinedBehavior::Strict` — any undefined variable surfaces as an error.
//! - `set_auto_escape_callback(|_| AutoEscape::None)` — templates emit JSON,
//!   not HTML, so escaping must be off (otherwise quotes get mangled).
//! - `set_recursion_limit(64)` — shallow templates only.
//! - `set_fuel(Some(100_000))` — hard CPU instruction cap.
//! - **No loader** — `env.set_loader` is intentionally never called so
//!   `{% include %}` / `{% extends %}` cannot reach the filesystem.
//!
//! Render boundary: `render_with_timeout` wraps a `spawn_blocking` join in
//! `tokio::time::timeout(5s)`. MiniJinja runs CPU-sync; offloading it onto the
//! blocking pool keeps the async runtime responsive, and the wall-clock timeout
//! gives the user feedback before `set_fuel` would otherwise unwind silently.
//!
//! Error mapping:
//! - Template parse / render failures → `AppError::Validation { field: "template", ... }`.
//! - 5 s wall-clock timeout → `AppError::Validation { field: "template", message: "Render timeout ..." }`.
//! - `JoinError` from the blocking pool → `AppError::Internal { source_chain }`.

use std::time::Duration;

use minijinja::{AutoEscape, Environment, HtmlEscape, UndefinedBehavior};
use trackly_core::error::AppError;

/// Prepare `org.full_name` for the `_header.html` `{{ org.full_name | safe }}`
/// interpolation (Plan 34-02/34-03) — mirrors the pre-existing
/// `org.logo_data_uri | safe` pattern.
///
/// D-03 requires the two-step order: escape HTML special characters FIRST
/// (via `minijinja::HtmlEscape`), THEN replace `'\n'` with the literal string
/// `"<br />"`. The reverse order is a stored-XSS vector because the org
/// full-name field is authenticated-write / broadcast-read (any LAN user
/// previewing an act/report can trigger a render of whatever an admin typed
/// into org_settings.full_name).
/// IN-04: line endings are normalized to `\n` FIRST, so CRLF input does not
/// leave a stray `\r` before the inserted `<br />` (`"a\r\nb"` used to yield
/// `"a\r<br />b"`). The HTML textarea API normalizes to LF, but the HTTP API
/// accepts raw JSON, so CRLF can genuinely reach the column. Normalizing
/// before escaping is safe: `HtmlEscape` does not touch `\r` or `\n`, and the
/// escape-then-insert ordering that the XSS mitigation depends on is
/// unchanged.
pub fn org_full_name_html(raw: &str) -> String {
    let normalized = raw.replace("\r\n", "\n").replace('\r', "\n");
    format!("{}", HtmlEscape(normalized.as_str())).replace('\n', "<br />")
}

/// Mime types accepted for the org logo. Mirrors `OrgDbService::save_logo`'s
/// write-side allowlist (T-07-02-01) — kept in sync deliberately.
const ALLOWED_LOGO_MIMES: [&str; 3] = ["image/png", "image/jpeg", "image/svg+xml"];

/// Build the `data:` URI for `_header.html`'s `<img src="{{ org.logo_data_uri
/// | safe }}">` sink, enforcing the logo mime allowlist on the READ side
/// (WR-01).
///
/// Phase 17 added this read-side check to `report_service::export_pdf` only.
/// Phase 34 then made all three printed forms share ONE `| safe` sink via the
/// header partial, which left the two act paths interpolating an unvalidated
/// `logo_mime` — read straight out of a mutable DB column — into an HTML
/// attribute explicitly marked `| safe`. A mime such as `png" onerror="…`
/// would break out of `src="…"`. Not currently exploitable (both write paths,
/// `OrgDbService::save_logo` and `migrate_from_org_json`, constrain the
/// value), but the defence-in-depth the project already chose must hold on
/// all three paths, not one of three.
///
/// A `None` mime is treated as "ok" (the historic `image/png` default
/// applies, and unmimed bytes still provably come from `OrgDbService`, never
/// from request input); an EXPLICIT disallowed mime drops the logo entirely
/// rather than embedding unverified bytes under a spoofed mime.
pub fn logo_data_uri(bytes: Option<Vec<u8>>, mime: Option<&str>) -> Option<String> {
    let mime_ok = mime
        .map(|m| ALLOWED_LOGO_MIMES.contains(&m.to_lowercase().as_str()))
        .unwrap_or(true);
    if !mime_ok {
        return None;
    }
    bytes.map(|bytes| {
        use base64::Engine;
        format!(
            "data:{};base64,{}",
            mime.unwrap_or("image/png"),
            base64::engine::general_purpose::STANDARD.encode(bytes)
        )
    })
}

/// Build a fresh MiniJinja `Environment` configured for Trackly safe-mode.
///
/// The returned environment has NO templates registered — call sites add
/// templates ad-hoc via `add_template_owned` inside `render_with_timeout`.
pub fn build_safe_env() -> Environment<'static> {
    let mut env = Environment::new();
    env.set_undefined_behavior(UndefinedBehavior::Strict);
    env.set_auto_escape_callback(|_| AutoEscape::None);
    env.set_recursion_limit(64);
    env.set_fuel(Some(100_000));
    // DO NOT set a loader — filesystem includes are not allowed.
    env
}

/// Build a fresh MiniJinja `Environment` configured for Trackly safe-mode,
/// with autoescape unconditionally ON (Phase 16, D-01/D-02, T-16-01).
///
/// Sibling to [`build_safe_env`] — every safe-mode invariant is identical
/// (`UndefinedBehavior::Strict`, `set_recursion_limit(64)`,
/// `set_fuel(Some(100_000))`, no loader). The only difference is
/// `AutoEscape::Html` instead of `AutoEscape::None`: every template rendered
/// through this environment is HTML output (act_handover.html /
/// act_acceptance.html), so `{{ var }}` interpolation must be HTML-escaped by
/// default — this is the sole mitigation for T-16-01 (Tampering/Injection via
/// device/org field interpolation). The sanctioned `| safe` exceptions are
/// `org.logo_data_uri` (server-constructed base64 + mime whitelist) and
/// `org.full_name` (server-side `org_full_name_html`-escaped before `<br>`
/// insertion) — `| safe` is permitted ONLY for values escaped or assembled
/// exclusively server-side from non-user-HTML input; never for raw
/// user/device/org field text.
pub fn build_safe_html_env() -> Environment<'static> {
    let mut env = Environment::new();
    env.set_undefined_behavior(UndefinedBehavior::Strict);
    env.set_auto_escape_callback(|_| AutoEscape::Html);
    env.set_recursion_limit(64);
    env.set_fuel(Some(100_000));
    // DO NOT set a loader — filesystem includes are not allowed (T-16-02).
    env
}

/// Render a template against `ctx` with both a hard fuel cap (set on the
/// Environment) and a soft 5 s wall-clock timeout.
///
/// The environment is cloned per-render so concurrent renders don't pollute
/// each other's template cache.
///
/// `extra_templates` registers additional named templates (e.g. the shared
/// `_header.html` partial) alongside the main template BEFORE render — D-13:
/// both `add_template_owned` calls happen before `env.get_template`/
/// `tmpl.render`, and this remains the ONLY registration mechanism (`env
/// .set_loader` is never called, so `{% include %}` cannot reach the
/// filesystem). Call order between extras and the main template does not
/// matter — MiniJinja resolves `{% include %}` at render time, not
/// registration time.
pub async fn render_with_timeout(
    env: &Environment<'static>,
    name: &str,
    template_src: &str,
    ctx: serde_json::Value,
    extra_templates: &[(&str, &str)],
) -> Result<String, AppError> {
    let env_owned = env.clone();
    let name_owned = name.to_owned();
    let template_src_owned = template_src.to_owned();
    let extra_owned: Vec<(String, String)> = extra_templates
        .iter()
        .map(|(n, s)| (n.to_string(), s.to_string()))
        .collect();

    let join = tokio::task::spawn_blocking(move || -> Result<String, AppError> {
        let mut env = env_owned;
        for (extra_name, extra_src) in extra_owned {
            env.add_template_owned(extra_name, extra_src)
                .map_err(|e| AppError::Validation {
                    field: "template".into(),
                    message: format!("Template parse error: {e}"),
                })?;
        }
        env.add_template_owned(name_owned.clone(), template_src_owned)
            .map_err(|e| AppError::Validation {
                field: "template".into(),
                message: format!("Template parse error: {e}"),
            })?;
        let tmpl = env
            .get_template(&name_owned)
            .map_err(|e| AppError::Validation {
                field: "template".into(),
                message: format!("Template lookup error: {e}"),
            })?;
        tmpl.render(ctx).map_err(|e| AppError::Validation {
            field: "template".into(),
            message: format!("Template render error: {e}"),
        })
    });

    let timed = tokio::time::timeout(Duration::from_secs(5), join)
        .await
        .map_err(|_| AppError::Validation {
            field: "template".into(),
            message: "Render timeout (5s) — шаблон слишком сложный".into(),
        })?;

    timed.map_err(|e| AppError::Internal {
        source_chain: format!("spawn_blocking joined: {e}"),
    })?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn env_renders_simple_template() {
        let env = build_safe_env();
        let out = render_with_timeout(
            &env,
            "ok",
            "Hello, {{ name }}!",
            serde_json::json!({ "name": "мир" }),
            &[],
        )
        .await
        .expect("render ok");
        assert_eq!(out, "Hello, мир!");
    }

    #[tokio::test]
    async fn env_rejects_undefined() {
        let env = build_safe_env();
        let result = render_with_timeout(
            &env,
            "undef",
            "Hello, {{ missing_var }}!",
            serde_json::json!({}),
            &[],
        )
        .await;
        match result {
            Err(AppError::Validation { field, .. }) => {
                assert_eq!(field, "template");
            }
            other => panic!("expected Validation, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn env_render_timeout_returns_validation() {
        // Either the 100k fuel cap trips (Validation) or the 5s timeout trips
        // (also Validation). Both code paths must map to Validation{field:"template"}.
        let env = build_safe_env();
        let result = render_with_timeout(
            &env,
            "loop",
            "{% for i in range(10000000) %}x{% endfor %}",
            serde_json::json!({}),
            &[],
        )
        .await;
        match result {
            Err(AppError::Validation { field, .. }) => {
                assert_eq!(field, "template");
            }
            other => panic!("expected Validation, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn env_rejects_parse_error() {
        let env = build_safe_env();
        let result = render_with_timeout(
            &env,
            "broken",
            "{% if unclosed",
            serde_json::json!({}),
            &[],
        )
        .await;
        match result {
            Err(AppError::Validation { field, .. }) => assert_eq!(field, "template"),
            other => panic!("expected Validation, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn env_render_with_timeout_registers_extra_partial_before_render() {
        let env = build_safe_env();
        let out = render_with_timeout(
            &env,
            "main",
            "{% include \"partial\" %}Hi {{ name }}",
            serde_json::json!({ "name": "мир" }),
            &[("partial", "PARTIAL-")],
        )
        .await
        .expect("render ok");
        assert_eq!(out, "PARTIAL-Hi мир");
    }

    #[tokio::test]
    async fn env_render_with_timeout_missing_partial_fails_cleanly() {
        let env = build_safe_env();
        let result = render_with_timeout(
            &env,
            "main",
            "{% include \"missing\" %}",
            serde_json::json!({}),
            &[],
        )
        .await;
        match result {
            Err(AppError::Validation { field, message }) => {
                assert_eq!(field, "template");
                assert!(
                    message.contains("render") || message.contains("template"),
                    "expected render/template error message, got: {message}"
                );
            }
            other => panic!("expected Validation, got {other:?}"),
        }
    }

    #[test]
    fn org_full_name_html_replaces_newline_with_br() {
        assert_eq!(
            org_full_name_html("Строка1\nСтрока2"),
            "Строка1<br />Строка2"
        );
    }

    #[test]
    fn org_full_name_html_escapes_before_inserting_br() {
        let out = org_full_name_html("<script>x</script>\nстрока2");
        assert!(
            out.contains("&lt;script&gt;"),
            "expected escaped script tag, got: {out}"
        );
        assert!(out.contains("<br />"), "expected literal <br />, got: {out}");
        assert!(
            !out.contains("<script>"),
            "literal <script> must never survive, got: {out}"
        );
    }

    #[test]
    fn org_full_name_html_empty_input_is_empty_output() {
        assert_eq!(org_full_name_html(""), "");
    }

    /// IN-04: CRLF input must not leave a stray `\r` before the `<br />`.
    #[test]
    fn org_full_name_html_normalizes_crlf() {
        assert_eq!(org_full_name_html("a\r\nb"), "a<br />b");
    }

    /// Bare CR (classic-Mac line endings, and what a `\r`-only paste produces)
    /// is a line break too, not a character to print.
    #[test]
    fn org_full_name_html_normalizes_lone_cr() {
        assert_eq!(org_full_name_html("a\rb"), "a<br />b");
    }

    #[test]
    fn logo_data_uri_builds_uri_for_allowed_mime() {
        let out = logo_data_uri(Some(vec![1, 2, 3]), Some("image/png")).expect("some uri");
        assert!(
            out.starts_with("data:image/png;base64,"),
            "unexpected uri: {out}"
        );
    }

    #[test]
    fn logo_data_uri_is_case_insensitive_on_mime() {
        assert!(logo_data_uri(Some(vec![1]), Some("IMAGE/JPEG")).is_some());
    }

    #[test]
    fn logo_data_uri_drops_logo_for_disallowed_mime() {
        // WR-01: an attribute-breaking mime must never reach the `| safe`
        // `<img src="...">` sink — the whole logo is dropped instead.
        assert_eq!(
            logo_data_uri(Some(vec![1, 2, 3]), Some("image/png\" onerror=\"x")),
            None
        );
        assert_eq!(logo_data_uri(Some(vec![1]), Some("text/html")), None);
    }

    #[test]
    fn logo_data_uri_defaults_to_png_when_mime_absent() {
        let out = logo_data_uri(Some(vec![1]), None).expect("some uri");
        assert!(
            out.starts_with("data:image/png;base64,"),
            "unexpected uri: {out}"
        );
    }

    #[test]
    fn logo_data_uri_is_none_without_bytes() {
        assert_eq!(logo_data_uri(None, Some("image/png")), None);
    }

    #[test]
    fn org_full_name_html_escapes_ampersand() {
        let out = org_full_name_html("A & B");
        assert!(out.contains("&amp;"), "expected escaped ampersand, got: {out}");
    }
}
