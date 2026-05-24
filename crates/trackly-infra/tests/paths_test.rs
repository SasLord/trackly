//! Tests for `trackly_infra::paths::Paths`. See PLAN 01-02 Task 1 §behavior.

use std::fs;
use std::path::PathBuf;

use tempfile::TempDir;
use trackly_core::error::AppError;
use trackly_infra::Paths;

/// Helper: create a sandbox dir that we'll pass to `Paths::resolve_for_exe_dir`.
fn sandbox() -> TempDir {
    tempfile::tempdir().expect("tempdir")
}

#[test]
fn test_1_resolve_with_portable_sentinel_marks_portable() {
    let dir = sandbox();
    fs::write(dir.path().join("portable.txt"), b"").unwrap();

    let paths = Paths::resolve_for_exe_dir(dir.path().to_path_buf())
        .expect("resolve_for_exe_dir succeeds with portable.txt");

    assert_eq!(paths.exe_dir(), dir.path());
    assert_eq!(paths.db_path(), dir.path().join("trackly.db").as_path());
    assert_eq!(
        paths.config_file(),
        dir.path().join("trackly.config.toml").as_path()
    );
    assert_eq!(
        paths.webview_data_dir(),
        dir.path().join("data/webview").as_path()
    );
    assert_eq!(paths.logs_dir(), dir.path().join("logs").as_path());
    assert!(paths.is_portable(), "portable.txt sentinel must mark portable");
}

#[test]
fn test_2_resolve_without_sentinel_is_not_portable() {
    let dir = sandbox();
    // No portable.txt and no trackly.config.toml present.
    let paths = Paths::resolve_for_exe_dir(dir.path().to_path_buf())
        .expect("resolve_for_exe_dir succeeds without sentinel");

    assert_eq!(paths.exe_dir(), dir.path());
    assert!(
        !paths.is_portable(),
        "without sentinel, is_portable() must be false"
    );
}

#[test]
fn test_3_resolve_with_config_file_marks_portable() {
    let dir = sandbox();
    // Sentinel rule is OR — config file alone is enough.
    fs::write(dir.path().join("trackly.config.toml"), b"# empty\n").unwrap();

    let paths = Paths::resolve_for_exe_dir(dir.path().to_path_buf())
        .expect("resolve_for_exe_dir succeeds with config file");

    assert!(
        paths.is_portable(),
        "trackly.config.toml sentinel must mark portable"
    );
}

#[test]
#[cfg(windows)]
fn test_4_resolve_rejects_unc_path_on_windows() {
    let unc = PathBuf::from(r"\\server\share\trackly");
    let err = Paths::resolve_for_exe_dir(unc).expect_err("UNC path must be rejected");

    match err {
        AppError::Validation { field, message } => {
            assert_eq!(field, "exe_dir");
            assert!(
                message.contains("UNC") || message.contains("SMB"),
                "error message must mention UNC or SMB, got: {message}"
            );
        }
        other => panic!("expected Validation error, got {other:?}"),
    }
}

#[test]
fn test_5_resolve_accepts_cyrillic_path() {
    // Pitfall #3: Cyrillic / non-ASCII paths must round-trip correctly.
    let dir = sandbox();
    let cyrillic = dir.path().join("Документы").join("Учёт").join("Trackly_test");
    fs::create_dir_all(&cyrillic).expect("create cyrillic dir");

    let paths = Paths::resolve_for_exe_dir(cyrillic.clone())
        .expect("resolve_for_exe_dir accepts cyrillic path");

    assert_eq!(paths.exe_dir(), cyrillic.as_path());
    assert_eq!(
        paths.webview_data_dir(),
        cyrillic.join("data/webview").as_path()
    );
    // Use Path-level comparison rather than to_string_lossy round-trip
    // (Pitfall #3: to_string_lossy can normalize silently on some platforms).
    let webview_components: Vec<_> = paths.webview_data_dir().components().collect();
    let expected_components: Vec<_> = cyrillic.join("data/webview").components().collect();
    assert_eq!(webview_components, expected_components);
}
