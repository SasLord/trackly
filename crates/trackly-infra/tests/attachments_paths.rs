//! Каталог вложений (Phase 41.3 Plan 01, D-21 / D-28). Только вымышленные данные.

use std::path::{Path, PathBuf};

use trackly_infra::paths::resolve_attachments_dir;
use trackly_infra::AppConfig;

#[test]
fn empty_value_defaults_next_to_db() {
    let got = resolve_attachments_dir(Path::new("/srv/trackly/trackly.db"), "");
    assert_eq!(got, PathBuf::from("/srv/trackly/attachments"));
    let got = resolve_attachments_dir(Path::new("/srv/trackly/trackly.db"), "   ");
    assert_eq!(got, PathBuf::from("/srv/trackly/attachments"));
}

#[test]
fn configured_value_is_used_verbatim() {
    let got = resolve_attachments_dir(Path::new("/srv/trackly/trackly.db"), "files/att");
    assert_eq!(got, PathBuf::from("files/att"));
}

#[test]
fn unc_value_is_accepted() {
    let got = resolve_attachments_dir(Path::new("/srv/trackly/trackly.db"), r"\\server\share\att");
    assert_eq!(got, PathBuf::from(r"\\server\share\att"));
}

#[cfg(windows)]
#[test]
fn db_path_still_rejects_unc() {
    let err = trackly_infra::Paths::resolve_for_exe_dir(PathBuf::from(r"\\server\share"));
    assert!(err.is_err());
}

#[test]
fn config_without_attachments_path_still_parses() {
    let cfg: AppConfig = toml::from_str("[paths]\ndb_path = \"x.db\"\n").expect("old config");
    assert_eq!(cfg.paths.attachments_path, "");

    let cfg: AppConfig =
        toml::from_str("[paths]\ndb_path = \"x.db\"\nattachments_path = \"D:/att\"\n")
            .expect("new config");
    assert_eq!(cfg.paths.attachments_path, "D:/att");
}
