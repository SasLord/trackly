//! Tests for `trackly_infra::config::AppConfig`. See PLAN 01-02 Task 2 §behavior.

use std::fs;
use std::path::PathBuf;

use tempfile::TempDir;
use trackly_core::error::AppError;
use trackly_infra::AppConfig;

fn write_fixture(dir: &TempDir, contents: &str) -> PathBuf {
    let path = dir.path().join("trackly.config.toml");
    fs::write(&path, contents).unwrap();
    path
}

#[test]
fn test_1_load_or_default_missing_file_returns_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("does-not-exist.toml");

    let cfg = AppConfig::load_or_default(&missing).expect("missing file → defaults, not error");

    // Should equal AppConfig::default()
    let defaults = AppConfig::default();
    assert_eq!(cfg.server.enabled, defaults.server.enabled);
    assert_eq!(cfg.server.host, defaults.server.host);
    assert_eq!(cfg.server.port, defaults.server.port);
    assert_eq!(cfg.logging.level, defaults.logging.level);
    assert_eq!(cfg.organization.timezone, defaults.organization.timezone);
}

#[test]
fn test_2_load_or_default_full_file_parses_all_sections() {
    let dir = tempfile::tempdir().unwrap();
    let body = r#"
[server]
enabled = true
host = "0.0.0.0"
port = 9000
cert_path = "/path/to/cert.pem"

[paths]
db_path = "/data/trackly.db"

[logging]
level = "debug"
format = "json"
retention_days = 30

[organization]
timezone = "Europe/Berlin"
"#;
    let path = write_fixture(&dir, body);

    let cfg = AppConfig::load_or_default(&path).expect("valid TOML parses");

    assert!(cfg.server.enabled);
    assert_eq!(cfg.server.host, "0.0.0.0");
    assert_eq!(cfg.server.port, 9000);
    assert_eq!(cfg.server.cert_path, "/path/to/cert.pem");
    assert_eq!(cfg.paths.db_path, "/data/trackly.db");
    assert_eq!(cfg.logging.level, "debug");
    assert_eq!(cfg.logging.format, "json");
    assert_eq!(cfg.logging.retention_days, 30);
    assert_eq!(cfg.organization.timezone, "Europe/Berlin");
}

#[test]
fn test_3_load_or_default_partial_file_uses_section_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let body = r#"
[server]
enabled = true
host = "10.0.0.5"
port = 8080
cert_path = ""
"#;
    let path = write_fixture(&dir, body);

    let cfg = AppConfig::load_or_default(&path).expect("partial TOML parses");

    // Server values from file
    assert!(cfg.server.enabled);
    assert_eq!(cfg.server.host, "10.0.0.5");
    assert_eq!(cfg.server.port, 8080);

    // Other sections use defaults
    let defaults = AppConfig::default();
    assert_eq!(cfg.paths.db_path, defaults.paths.db_path);
    assert_eq!(cfg.logging.level, defaults.logging.level);
    assert_eq!(cfg.logging.format, defaults.logging.format);
    assert_eq!(cfg.logging.retention_days, defaults.logging.retention_days);
    assert_eq!(cfg.organization.timezone, defaults.organization.timezone);
}

#[test]
fn test_4_load_or_default_malformed_toml_returns_validation_error() {
    let dir = tempfile::tempdir().unwrap();
    // Bad TOML: unterminated string.
    let body = r#"
[server
enabled = true
"#;
    let path = write_fixture(&dir, body);

    let err = AppConfig::load_or_default(&path).expect_err("malformed TOML must error");

    match err {
        AppError::Validation { field, message } => {
            assert!(
                field.contains("trackly.config.toml"),
                "field should reference file name, got: {field}"
            );
            assert!(
                !message.is_empty(),
                "message should contain TOML parser error"
            );
        }
        other => panic!("expected Validation, got {other:?}"),
    }
}

#[test]
fn test_5_default_values_match_d_config_01() {
    let cfg = AppConfig::default();

    assert!(!cfg.server.enabled, "server disabled by default");
    assert_eq!(cfg.server.host, "127.0.0.1");
    assert_eq!(cfg.server.port, 8443);
    assert_eq!(cfg.server.cert_path, "");
    assert_eq!(cfg.paths.db_path, "");
    assert_eq!(cfg.logging.level, "info");
    assert_eq!(cfg.logging.format, "compact");
    assert_eq!(cfg.logging.retention_days, 14);
    assert_eq!(cfg.organization.timezone, "Europe/Moscow");
}

#[test]
fn test_6_unknown_keys_ignored_gracefully() {
    // Defensive: unknown top-level keys / unknown section keys must NOT error
    // (forward-compat: future versions may add fields older binaries don't know).
    let dir = tempfile::tempdir().unwrap();
    let body = r#"
[server]
enabled = false
host = "127.0.0.1"
port = 8443
cert_path = ""
mystery_field = "ignored"

[unknown_section]
foo = "bar"
"#;
    let path = write_fixture(&dir, body);

    let cfg = AppConfig::load_or_default(&path).expect("unknown keys must not block parsing");
    assert!(!cfg.server.enabled);
    assert_eq!(cfg.server.host, "127.0.0.1");
}
