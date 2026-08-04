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
    ("report.html", include_str!("../../templates/report.html")),
];

/// Registry of previously-shipped default bodies, keyed by filename (D-12).
///
/// `template_service.rs`'s DB-backed templates detect "untouched" via an
/// explicit `is_default` BOOLEAN column, flipped to 0 the moment a user calls
/// `update_body`. File-based templates have no such companion metadata slot —
/// a template is just bytes on disk. So `upgrade_untouched_defaults_on_startup`
/// detects "untouched" STRUCTURALLY: the on-disk body is byte-identical to the
/// current bundled default (already-current, no-op) OR to any body in this
/// registry (a known prior default → provably not user-customized → safe to
/// upgrade). Anything else is treated as user-customized and never overwritten.
///
/// **Extension point:** whenever a body in `DEFAULT_HTML_TEMPLATES` changes
/// again in a future phase, the PRE-CHANGE body MUST be captured as a new
/// snapshot (a new sibling directory, e.g. `_legacy_defaults/v21/`) and added
/// here as an additional entry in that filename's slice — otherwise installs
/// that predate THAT phase stop being recognized as untouched and silently
/// stop receiving upgrades. Forgetting this only causes a MISSED upgrade (file
/// stays on older-but-valid content), never a wrongful overwrite.
pub const KNOWN_LEGACY_DEFAULTS: &[(&str, &[&str])] = &[
    (
        "act_handover.html",
        &[include_str!(
            "../../templates/_legacy_defaults/v20/act_handover.html"
        )],
    ),
    (
        "act_acceptance.html",
        &[include_str!(
            "../../templates/_legacy_defaults/v20/act_acceptance.html"
        )],
    ),
    (
        "report.html",
        &[include_str!(
            "../../templates/_legacy_defaults/v20/report.html"
        )],
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

/// Auto-upgrades on-disk template files that are provably untouched by the
/// user, so bundled-default changes reach installs that already materialized
/// these files in a prior phase — not only fresh installs (D-12).
///
/// `materialize_defaults_on_startup` is insert-only and `load_template` always
/// prefers the on-disk copy, so without this pass a template edited in the
/// bundle (e.g. Phase 20's header/`address_line2` changes) never reaches any
/// existing install. This function runs immediately after materialize and, for
/// each embedded default:
///
/// - file missing/unreadable → `continue` (materialize owns the missing case);
/// - on-disk content == current bundled default → `continue` (already current);
/// - on-disk content == any `KNOWN_LEGACY_DEFAULTS` snapshot for this filename
///   → overwrite with the current default (provably untouched → safe upgrade);
/// - otherwise → leave untouched (user-customized; fail closed — D-12's core
///   safety property, never overwrite ambiguous content).
pub fn upgrade_untouched_defaults_on_startup(templates_dir: &Path) -> Result<(), AppError> {
    for (filename, current_default) in DEFAULT_HTML_TEMPLATES.iter() {
        let path = templates_dir.join(filename);
        let on_disk = match std::fs::read_to_string(&path) {
            Ok(body) => body,
            Err(_) => continue, // missing/unreadable — materialize owns this case
        };
        if &on_disk == current_default {
            continue; // already current — no write
        }
        let legacy_bodies = KNOWN_LEGACY_DEFAULTS
            .iter()
            .find(|(name, _)| name == filename)
            .map(|(_, bodies)| *bodies)
            .unwrap_or(&[]);
        if legacy_bodies.iter().any(|legacy| *legacy == on_disk) {
            std::fs::write(&path, current_default).map_err(|e| AppError::Internal {
                source_chain: format!("write({}) failed: {e}", path.display()),
            })?;
            tracing::info!(
                "Auto-upgraded untouched default HTML template at {}",
                path.display()
            );
        }
        // else: user-customized (matches neither current nor any known legacy
        // default) — leave untouched, fail closed.
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

    /// D-12 Test 1: a pre-materialized OLD (legacy) file — the exact shape a
    /// Phase 16/17 install has on disk — gets upgraded to the current bundled
    /// body. Uses pre-existing legacy content, NOT a fresh/empty dir, so it
    /// fails if `upgrade_untouched_defaults_on_startup` is reverted to a no-op.
    #[test]
    fn upgrade_replaces_untouched_legacy_default_with_current_bundled_body() {
        let _guard = ENV_GUARD.lock().unwrap();
        let dir = tempfile::tempdir().expect("tempdir");

        // Pre-populate disk with the OLD (pre-Phase-20) content for each file.
        for (filename, _current) in DEFAULT_HTML_TEMPLATES.iter() {
            let legacy = KNOWN_LEGACY_DEFAULTS
                .iter()
                .find(|(name, _)| name == filename)
                .and_then(|(_, bodies)| bodies.first())
                .expect("legacy snapshot registered for filename");
            std::fs::write(dir.path().join(filename), legacy).expect("write legacy body");
        }

        upgrade_untouched_defaults_on_startup(dir.path()).expect("upgrade ok");

        for (filename, current) in DEFAULT_HTML_TEMPLATES.iter() {
            let contents = std::fs::read_to_string(dir.path().join(filename)).expect("file exists");
            assert_eq!(
                &contents, current,
                "{filename} must be upgraded to the current bundled body"
            );
        }
    }

    /// D-12 Test 2: a user-customized file (matches neither the current default
    /// nor any known legacy default) is NEVER overwritten — the fail-closed
    /// safety guarantee (T-20-06-01 mitigation).
    #[test]
    fn upgrade_leaves_user_customized_file_untouched() {
        let _guard = ENV_GUARD.lock().unwrap();
        let dir = tempfile::tempdir().expect("tempdir");

        let custom = "<html>МОЙ КАСТОМНЫЙ ШАБЛОН</html>";
        let path = dir.path().join("act_handover.html");
        std::fs::write(&path, custom).expect("write custom body");

        upgrade_untouched_defaults_on_startup(dir.path()).expect("upgrade ok");

        let contents = std::fs::read_to_string(&path).expect("still exists");
        assert_eq!(
            contents, custom,
            "user-customized file must not be overwritten"
        );
    }

    /// D-12 Test 3: a file already on the current bundled body is a no-op —
    /// no wrongful rewrite, content stays identical.
    #[test]
    fn upgrade_is_noop_when_file_already_current() {
        let _guard = ENV_GUARD.lock().unwrap();
        let dir = tempfile::tempdir().expect("tempdir");

        let current = DEFAULT_HTML_TEMPLATES
            .iter()
            .find(|(name, _)| *name == "act_handover.html")
            .map(|(_, body)| *body)
            .expect("act_handover.html default present");
        let path = dir.path().join("act_handover.html");
        std::fs::write(&path, current).expect("write current body");

        upgrade_untouched_defaults_on_startup(dir.path()).expect("upgrade ok");

        let contents = std::fs::read_to_string(&path).expect("still exists");
        assert_eq!(contents, current, "already-current file must be unchanged");
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
