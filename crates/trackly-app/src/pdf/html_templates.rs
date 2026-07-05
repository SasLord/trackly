//! HTML template file-resolver + materialize-on-startup + read-on-render
//! loader (Phase 16, D-05/D-06/D-07/D-08).
//!
//! Unlike `template_service.rs`'s DB-backed `document_templates` table (which
//! the krilla/DocSpec pipeline still uses, frozen but not removed — see
//! CONTEXT.md), the HTML act templates for Phase 16 live as plain, author-
//! editable files in `templates/` next to the executable:
//!
//! - **Resolve** (D-07): `TRACKLY_TEMPLATES_DIR` env var wins when set
//!   (dev/test override, mirrors `TRACKLY_AD_MOCK`/`TRACKLY_SNMP_MOCK`);
//!   otherwise falls back to `Paths::templates_dir()`
//!   (`<exe_dir>/templates`).
//! - **Materialize on startup** (D-05): if a template file is missing, write
//!   the embedded `include_str!` default so the user immediately sees a file
//!   to edit. This is idempotent insert-only — an existing file (even if
//!   hand-edited) is never overwritten, unlike `template_service`'s DB
//!   auto-upgrade-in-place branch.
//! - **Read-on-render** (D-06/D-08): the file is read fresh from disk on
//!   every render call; if it is missing (e.g. deleted after startup), the
//!   embedded default is used instead — generation never panics or fails
//!   because of a missing template file.

use std::path::{Path, PathBuf};

use trackly_core::error::AppError;
use trackly_infra::paths::Paths;

/// Embedded default HTML templates: `(filename, body)`. Mirrors the shape of
/// `template_service::DEFAULT_TEMPLATES` but file-backed, not DB-backed.
pub const DEFAULT_HTML_TEMPLATES: &[(&str, &str)] = &[
    (
        "act_handover.html",
        include_str!("../../templates/act_handover.html"),
    ),
    (
        "act_acceptance.html",
        include_str!("../../templates/act_acceptance.html"),
    ),
];

/// Resolves the templates directory: `TRACKLY_TEMPLATES_DIR` env var wins
/// when set (non-empty); otherwise falls back to `paths.templates_dir()`
/// (`<exe_dir>/templates`).
///
/// The env var is a developer/test-only override (D-07) — production always
/// uses the portable, `current_exe()`-derived default.
pub fn resolve_templates_dir(paths: &Paths) -> PathBuf {
    match std::env::var("TRACKLY_TEMPLATES_DIR") {
        Ok(val) if !val.is_empty() => PathBuf::from(val),
        _ => paths.templates_dir().to_path_buf(),
    }
}

/// Materializes any missing embedded default HTML templates into
/// `templates_dir` (D-05). Creates `templates_dir` if it does not exist.
///
/// Idempotent insert-only: a file that already exists (default OR
/// hand-edited by the user) is never overwritten — this is deliberately NOT
/// an auto-upgrade-in-place operation, unlike `template_service`'s DB seed
/// path.
pub fn materialize_defaults_on_startup(templates_dir: &Path) -> Result<(), AppError> {
    std::fs::create_dir_all(templates_dir).map_err(|e| AppError::Internal {
        source_chain: format!("create_dir_all({}) failed: {e}", templates_dir.display()),
    })?;

    for (filename, default_body) in DEFAULT_HTML_TEMPLATES.iter() {
        let path = templates_dir.join(filename);
        if !path.exists() {
            std::fs::write(&path, default_body).map_err(|e| AppError::Internal {
                source_chain: format!("write({}) failed: {e}", path.display()),
            })?;
            tracing::info!("Materialized default HTML template at {}", path.display());
        }
    }
    Ok(())
}

/// Reads `templates_dir.join(filename)` from disk; falls back to
/// `embedded_default` if the file is absent or unreadable (D-06/D-08).
/// Never panics, never returns an `Err` — generation must not fail because a
/// template file was deleted after startup.
pub fn load_template(templates_dir: &Path, filename: &str, embedded_default: &str) -> String {
    std::fs::read_to_string(templates_dir.join(filename))
        .unwrap_or_else(|_| embedded_default.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Serializes tests that touch `TRACKLY_TEMPLATES_DIR` — `std::env` is
    /// process-global and Rust test threads run in parallel by default.
    static ENV_GUARD: Mutex<()> = Mutex::new(());

    fn set_templates_dir_env(val: &str) {
        // SAFETY: guarded by ENV_GUARD for the duration of the calling test;
        // no other thread touches TRACKLY_TEMPLATES_DIR concurrently.
        unsafe {
            std::env::set_var("TRACKLY_TEMPLATES_DIR", val);
        }
    }

    fn remove_templates_dir_env() {
        // SAFETY: see set_templates_dir_env.
        unsafe {
            std::env::remove_var("TRACKLY_TEMPLATES_DIR");
        }
    }

    #[test]
    fn materialize_creates_both_defaults_in_empty_dir() {
        let _guard = ENV_GUARD.lock().unwrap();
        let dir = tempfile::tempdir().expect("tempdir");

        materialize_defaults_on_startup(dir.path()).expect("materialize ok");

        for (filename, default_body) in DEFAULT_HTML_TEMPLATES.iter() {
            let contents = std::fs::read_to_string(dir.path().join(filename)).expect("file exists");
            assert_eq!(&contents, default_body);
        }
    }

    #[test]
    fn materialize_is_idempotent_and_does_not_clobber_hand_edits() {
        let _guard = ENV_GUARD.lock().unwrap();
        let dir = tempfile::tempdir().expect("tempdir");

        materialize_defaults_on_startup(dir.path()).expect("first materialize ok");

        // Hand-edit one of the materialized files.
        let handover_path = dir.path().join("act_handover.html");
        std::fs::write(&handover_path, "<html>CUSTOM EDIT</html>").expect("hand edit");

        // Second call must not clobber the hand-edited file.
        materialize_defaults_on_startup(dir.path()).expect("second materialize ok");

        let contents = std::fs::read_to_string(&handover_path).expect("still exists");
        assert_eq!(contents, "<html>CUSTOM EDIT</html>");
    }

    #[test]
    fn load_template_falls_back_to_embedded_default_when_file_absent() {
        let _guard = ENV_GUARD.lock().unwrap();
        let dir = tempfile::tempdir().expect("tempdir");
        // No materialize call — directory has no files, doesn't even need
        // to exist for load_template's fallback path.

        let embedded_default = "EMBEDDED_DEFAULT_BODY";
        let result = load_template(dir.path(), "act_handover.html", embedded_default);
        assert_eq!(result, embedded_default);
    }

    #[test]
    fn load_template_returns_on_disk_content_when_file_present() {
        let _guard = ENV_GUARD.lock().unwrap();
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("act_handover.html");
        std::fs::write(&path, "ON_DISK_CONTENT").expect("write file");

        let embedded_default = "EMBEDDED_DEFAULT_BODY";
        let result = load_template(dir.path(), "act_handover.html", embedded_default);
        assert_eq!(result, "ON_DISK_CONTENT");
        assert_ne!(result, embedded_default);
    }

    #[test]
    fn resolve_templates_dir_prefers_env_override_when_set() {
        let _guard = ENV_GUARD.lock().unwrap();
        let paths = Paths::resolve_for_exe_dir(PathBuf::from("/does/not/matter"))
            .expect("resolve_for_exe_dir ok");

        set_templates_dir_env("/tmp/trackly-test-templates-override");
        let resolved = resolve_templates_dir(&paths);
        assert_eq!(
            resolved,
            PathBuf::from("/tmp/trackly-test-templates-override")
        );

        remove_templates_dir_env();
        let resolved_default = resolve_templates_dir(&paths);
        assert_eq!(resolved_default, paths.templates_dir());
    }
}
