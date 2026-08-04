//! Regression test (quick task 260804-lk0): the SHIPPED `trackly.config.toml.example` must,
//! once every commented-out config line is uncommented, parse into `AppConfig` without
//! error. Loaded via `include_str!` against the exact file that ships in the portable ZIP
//! (`.github/workflows/release.yml`'s "Assemble portable ZIP" step copies this same
//! repo-root file) — never a hand-copied duplicate, so this test cannot silently drift from
//! what users actually receive.

use trackly_infra::AppConfig;

const EXAMPLE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../trackly.config.toml.example"
));

/// True if `rest` (already stripped of its leading `#`/`# `) is a `[section]` or
/// `[[array.of.tables]]` header — every character is `[`, `]`, a word char, or `.`.
fn is_section_header(rest: &str) -> bool {
    !rest.is_empty()
        && rest.starts_with('[')
        && rest
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "[]._".contains(c))
}

/// True if `rest` is a `key = value` line — starts with an ASCII identifier
/// (`[A-Za-z_][A-Za-z0-9_]*`) immediately followed (after optional whitespace) by `=`.
/// Genuine prose comments in this file are Cyrillic-led or indented, so they can never
/// satisfy this — Cyrillic letters are outside `[A-Za-z_]`, and an indented line's first
/// char is whitespace, not a letter.
fn is_key_value(rest: &str) -> bool {
    let mut chars = rest.char_indices();
    let starts_ok = matches!(chars.next(), Some((_, c)) if c.is_ascii_alphabetic() || c == '_');
    if !starts_ok {
        return false;
    }
    let key_end = rest
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(rest.len());
    rest[key_end..].trim_start().starts_with('=')
}

/// Uncomments only lines that are commented-out TOML config — leaves genuine prose comments
/// (this file's explanatory text is entirely in Russian/Cyrillic, or indented continuation
/// lines) untouched. See `is_section_header`/`is_key_value` for the exact heuristic.
fn uncomment_example(src: &str) -> String {
    src.lines()
        .map(|line| {
            let Some(rest) = line.strip_prefix('#') else {
                return line.to_string();
            };
            let rest = rest.strip_prefix(' ').unwrap_or(rest);
            if is_section_header(rest) || is_key_value(rest) {
                rest.to_string()
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn shipped_example_fully_uncommented_parses_into_app_config() {
    let uncommented = uncomment_example(EXAMPLE);
    let cfg: AppConfig = toml::from_str(&uncommented).unwrap_or_else(|e| {
        panic!(
            "shipped trackly.config.toml.example, fully uncommented, failed to parse: {e}\n\
             --- uncommented content ---\n{uncommented}"
        )
    });

    // Sanity: prove uncommenting activated real lines, not just an empty document.
    assert_eq!(cfg.server.host, "127.0.0.1");
    assert_eq!(cfg.server.port, 8443);
    assert!(!cfg.server.enabled, "example ships server.enabled = false");
    assert_eq!(cfg.logging.level, "info");
    assert_eq!(cfg.organization.timezone, "Europe/Moscow");
    assert!(
        !cfg.ad.role_mapping.is_empty(),
        "example [[ad.role_mapping]] tables should have uncommented"
    );
    assert_eq!(
        cfg.ad.admin_logins,
        vec!["us100".to_string(), "us777".to_string()]
    );
}

#[test]
fn shipped_example_uses_real_paths_section_not_stale_storage() {
    // Regression guard for the original bug report: the example used to ship a [storage]
    // section that does not exist in AppConfig (PathsConfig lives under [paths]) — silently
    // ignored by serde's forward-compat unknown-key tolerance, so the typo never surfaced as
    // a parse error. Assert the corrected file uses the real section name.
    assert!(
        EXAMPLE.contains("[paths]"),
        "example must use the real [paths] section name"
    );
    assert!(
        !EXAMPLE.contains("[storage]"),
        "example must not reintroduce the non-existent [storage] section"
    );
}
