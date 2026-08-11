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
    (
        "_header.html",
        include_str!("../../templates/_header.html"),
    ),
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
///
/// WR-06: EVERY filename in `DEFAULT_HTML_TEMPLATES` must have an entry here,
/// even an empty one — enforced by
/// `every_default_template_has_a_known_legacy_defaults_entry`. Without the
/// empty-but-present entry the extension-point note above does not describe
/// what a maintainer has to do for a brand-new file (add a whole new top-level
/// tuple, not "a new entry in that filename's slice"), and both structural
/// upgrade tests silently `continue` past it — so a header change would ship
/// green while reaching no existing install: `materialize` skips the file (it
/// exists), `upgrade` finds no legacy match, and the D-16 branch calls every
/// install "user-customized".
pub const KNOWN_LEGACY_DEFAULTS: &[(&str, &[&str])] = &[
    (
        "act_handover.html",
        &[
            include_str!("../../templates/_legacy_defaults/v20/act_handover.html"),
            include_str!("../../templates/_legacy_defaults/v21/act_handover.html"),
            include_str!("../../templates/_legacy_defaults/v22/act_handover.html"),
        ],
    ),
    (
        "act_acceptance.html",
        &[
            include_str!("../../templates/_legacy_defaults/v20/act_acceptance.html"),
            include_str!("../../templates/_legacy_defaults/v21/act_acceptance.html"),
            include_str!("../../templates/_legacy_defaults/v22/act_acceptance.html"),
        ],
    ),
    (
        "report.html",
        &[
            include_str!("../../templates/_legacy_defaults/v20/report.html"),
            include_str!("../../templates/_legacy_defaults/v21/report.html"),
        ],
    ),
    // Phase 34 introduced `_header.html`; its CURRENT body is the first one
    // ever shipped, so there is no prior default to recognize yet — hence the
    // empty (but present, WR-06) slice. Before changing this file in a future
    // phase, snapshot THIS body into `_legacy_defaults/vNN/_header.html` and
    // add it here, or existing installs will never receive the new header.
    ("_header.html", &[]),
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
///
/// IN-05: write failures are logged and skipped, NOT propagated. `AppCtx::build`
/// calls this with `?`, so a read-only share or a locked install directory used
/// to make the whole application refuse to start — even though `load_template`
/// would happily have served the embedded defaults, i.e. printing would have
/// worked fine. Materializing files is a convenience (giving the user something
/// to edit), never a precondition for rendering. The signature keeps returning
/// `Result` for call-site compatibility; it is now always `Ok`.
pub fn materialize_defaults_on_startup(templates_dir: &Path) -> Result<(), AppError> {
    if let Err(e) = std::fs::create_dir_all(templates_dir) {
        tracing::warn!(
            "Cannot create templates directory {} ({e}) — printing will use the embedded \
             defaults; template files will not be available for editing.",
            templates_dir.display()
        );
        return Ok(());
    }

    for (filename, default_body) in DEFAULT_HTML_TEMPLATES.iter() {
        let path = templates_dir.join(filename);
        if !path.exists() {
            match std::fs::write(&path, default_body) {
                Ok(()) => {
                    tracing::info!("Materialized default HTML template at {}", path.display())
                }
                Err(e) => tracing::warn!(
                    "Cannot write default template {} ({e}) — printing will use the embedded \
                     default; this file will not be available for editing.",
                    path.display()
                ),
            }
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
///
/// IN-05: as in `materialize_defaults_on_startup`, a write failure is logged
/// and skipped rather than propagated — a read-only install directory must not
/// prevent the application from starting when rendering degrades cleanly to the
/// embedded defaults. Always returns `Ok`.
pub fn upgrade_untouched_defaults_on_startup(templates_dir: &Path) -> Result<(), AppError> {
    for (filename, current_default) in DEFAULT_HTML_TEMPLATES.iter() {
        let path = templates_dir.join(filename);
        let on_disk = match std::fs::read_to_string(&path) {
            Ok(body) => body,
            // Absent is the expected case here — `materialize_defaults_on_startup`
            // owns it and has already run.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            // WR-03: anything else (permissions, non-UTF-8 bytes) used to be
            // swallowed identically, with no signal at all. The realistic
            // trigger on the target platform is a Windows admin editing the
            // file in Notepad and saving it as Windows-1251/ANSI — Cyrillic
            // content guarantees non-UTF-8 bytes. Their edits then silently do
            // nothing (the embedded default renders instead) and nothing in
            // the app ever says why.
            Err(e) => {
                tracing::warn!(
                    "Cannot read template {} ({e}) — skipping auto-upgrade and falling back \
                     to the embedded default on render. If you edited this file, make sure \
                     it is saved as UTF-8 (не ANSI/Windows-1251).",
                    path.display()
                );
                continue;
            }
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
            match std::fs::write(&path, current_default) {
                Ok(()) => tracing::info!(
                    "Auto-upgraded untouched default HTML template at {}",
                    path.display()
                ),
                Err(e) => tracing::warn!(
                    "Cannot auto-upgrade untouched default template {} ({e}) — the on-disk \
                     copy stays on its older (but valid) body.",
                    path.display()
                ),
            }
        } else {
            tracing::warn!(
                "Skipped auto-upgrade of {} — on-disk content matches neither current \
                 default nor any known legacy default (user-customized); manual review \
                 needed if a header/layout upgrade was expected.",
                path.display()
            );
        }
    }
    Ok(())
}

/// Reads `templates_dir.join(filename)` from disk; falls back to
/// `embedded_default` if the file is absent or unreadable (D-06/D-08).
/// Never panics, never returns an `Err` — generation must not fail because a
/// template file was deleted after startup.
///
/// WR-03: "absent" is silent (the normal, expected fallback), but "present
/// and unreadable" — permissions, or non-UTF-8 bytes from a Notepad
/// ANSI/Windows-1251 save — is logged at `warn`. Without this the user's edits
/// simply have no effect and the app offers no diagnosis from the inside.
pub fn load_template(templates_dir: &Path, filename: &str, embedded_default: &str) -> String {
    let path = templates_dir.join(filename);
    match std::fs::read_to_string(&path) {
        Ok(body) => body,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => embedded_default.to_string(),
        Err(e) => {
            tracing::warn!(
                "Cannot read template {} ({e}) — rendering the embedded default instead. \
                 If you edited this file, make sure it is saved as UTF-8 \
                 (не ANSI/Windows-1251).",
                path.display()
            );
            embedded_default.to_string()
        }
    }
}

/// Classifies a template file on disk for the status endpoint (D-17) and for
/// `load_template`'s siblings: `Ok(Some(body))` — readable; `Ok(None)` —
/// absent (not yet materialized); `Err(io::Error)` — present but unreadable
/// (WR-03: permissions or non-UTF-8 bytes).
///
/// Exists so callers stop conflating "absent" with "unreadable" — the D-17
/// endpoint whose entire purpose is flagging hand-edited files used to report
/// a Notepad-ANSI-mangled file as `Current`.
pub fn read_template_if_present(
    templates_dir: &Path,
    filename: &str,
) -> Result<Option<String>, std::io::Error> {
    match std::fs::read_to_string(templates_dir.join(filename)) {
        Ok(body) => Ok(Some(body)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
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

        // Pre-populate disk with the OLD (pre-Phase-20) content for each file
        // that has a registered legacy snapshot. `_header.html` (Phase 34) is
        // a brand-new file with no legacy predecessor — its
        // `KNOWN_LEGACY_DEFAULTS` entry exists (WR-06) but is EMPTY (a missing
        // file is `materialize_defaults_on_startup`'s job, not this upgrade
        // path's), so it is skipped here rather than treated as a failure.
        for (filename, _current) in DEFAULT_HTML_TEMPLATES.iter() {
            let Some(legacy) = KNOWN_LEGACY_DEFAULTS
                .iter()
                .find(|(name, _)| name == filename)
                .and_then(|(_, bodies)| bodies.first())
            else {
                continue;
            };
            std::fs::write(dir.path().join(filename), legacy).expect("write legacy body");
        }

        upgrade_untouched_defaults_on_startup(dir.path()).expect("upgrade ok");

        for (filename, current) in DEFAULT_HTML_TEMPLATES.iter() {
            let has_legacy_body = KNOWN_LEGACY_DEFAULTS
                .iter()
                .find(|(name, _)| name == filename)
                .is_some_and(|(_, bodies)| !bodies.is_empty());
            if !has_legacy_body {
                continue;
            }
            let contents = std::fs::read_to_string(dir.path().join(filename)).expect("file exists");
            assert_eq!(
                &contents, current,
                "{filename} must be upgraded to the current bundled body"
            );
        }
    }

    /// WR-06 invariant gate: every filename in `DEFAULT_HTML_TEMPLATES` must
    /// have an entry in `KNOWN_LEGACY_DEFAULTS` — empty is fine for a
    /// brand-new file, ABSENT is not.
    ///
    /// Without this, a future body change to a file with no registered slice
    /// ships green while reaching zero existing installs: `materialize` skips
    /// it (the file exists), `upgrade` finds no legacy match, the D-16 branch
    /// warns that every install is "user-customized", and both structural
    /// upgrade tests above `continue` past the gap forever. `_header.html` —
    /// now the single point of layout for all three printed forms, and so the
    /// file most likely to change next — was in exactly that state.
    #[test]
    fn every_default_template_has_a_known_legacy_defaults_entry() {
        for (filename, _) in DEFAULT_HTML_TEMPLATES.iter() {
            assert!(
                KNOWN_LEGACY_DEFAULTS
                    .iter()
                    .any(|(name, _)| name == filename),
                "{filename} has no KNOWN_LEGACY_DEFAULTS entry — add one (an empty \
                 slice is correct for a brand-new file; before CHANGING an existing \
                 body, snapshot the pre-change body into _legacy_defaults/vNN/ and \
                 append it to that filename's slice)"
            );
        }
    }

    /// Phase 34 D-15: proves the NEW v21 slice element specifically (not just
    /// `.first()`/v20) drives a real upgrade — unlike the `.first()`-based
    /// test above, this pulls index `1` (the v21 element) from each
    /// filename's `KNOWN_LEGACY_DEFAULTS` slice.
    #[test]
    fn upgrade_replaces_v21_legacy_default_with_current_bundled_body() {
        let _guard = ENV_GUARD.lock().unwrap();
        let dir = tempfile::tempdir().expect("tempdir");

        for (filename, current) in DEFAULT_HTML_TEMPLATES.iter() {
            let Some(bodies) = KNOWN_LEGACY_DEFAULTS
                .iter()
                .find(|(name, _)| name == filename)
                .map(|(_, bodies)| *bodies)
            else {
                continue; // e.g. _header.html — no legacy slice registered
            };
            let Some(v21_body) = bodies.get(1) else {
                continue; // filename has no v21 element (shouldn't happen for the 3 main files)
            };

            // Precondition guard (D-15/Pitfall 5): if the v21 snapshot had
            // been taken AFTER the rewrite instead of before, it would
            // already equal the current bundled body and the upgrade
            // assertion below would pass trivially without ever exercising
            // a real upgrade. This makes that failure mode impossible to
            // pass unnoticed.
            assert_ne!(
                v21_body, current,
                "{filename}: v21 legacy snapshot must NOT equal the current bundled \
                 default — otherwise the snapshot was taken after the rewrite and this \
                 test cannot prove a real upgrade happened"
            );

            std::fs::write(dir.path().join(filename), v21_body).expect("write v21 body");
        }

        upgrade_untouched_defaults_on_startup(dir.path()).expect("upgrade ok");

        for (filename, current) in DEFAULT_HTML_TEMPLATES.iter() {
            let has_v21 = KNOWN_LEGACY_DEFAULTS
                .iter()
                .find(|(name, _)| name == filename)
                .map(|(_, bodies)| bodies.len() > 1)
                .unwrap_or(false);
            if !has_v21 {
                continue;
            }
            let contents = std::fs::read_to_string(dir.path().join(filename)).expect("file exists");
            assert_eq!(
                &contents, current,
                "{filename} must be upgraded from its v21 legacy body to the current bundled body"
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

    /// Captures `tracing` output into a shared byte buffer so a test can assert
    /// on what the application actually logged. Cloneable + `io::Write`, which
    /// is what `fmt().with_writer(|| ...)` needs.
    #[derive(Clone)]
    struct CapturedLogs(std::sync::Arc<Mutex<Vec<u8>>>);

    impl std::io::Write for CapturedLogs {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// D-16: the fail-closed skip must be OBSERVABLE, not silent.
    ///
    /// `upgrade_leaves_user_customized_file_untouched` above only proves the
    /// bytes survive; that is exactly the state a Phase-20/34 header upgrade
    /// looks like when it silently never reaches an install. This test asserts
    /// the operator gets told: a `warn` naming the skipped file. Deleting the
    /// `else` branch in `upgrade_untouched_defaults_on_startup` keeps the
    /// sibling test green and turns this one red.
    ///
    /// Uses a THREAD-LOCAL scoped subscriber (`set_default`, not
    /// `set_global_default`) — the lib test binary runs many tests in one
    /// process and a global default can only ever be installed once.
    #[test]
    fn upgrade_warns_when_it_skips_a_user_customized_file() {
        let _guard = ENV_GUARD.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::tempdir().expect("tempdir");

        // Hand-edited file → matches neither the current default nor any known
        // legacy default → must be skipped AND warned about. Fictional
        // placeholder content only (public repo, no real org/personal data).
        let custom = "<html><p>hand-edited</p></html>";
        std::fs::write(dir.path().join("act_handover.html"), custom).expect("write custom body");

        // Control: a file already on the current bundled body is a no-op and
        // must NOT be warned about — otherwise the "warn" assertion below
        // would pass for an implementation that warns indiscriminately.
        let current_report = DEFAULT_HTML_TEMPLATES
            .iter()
            .find(|(name, _)| *name == "report.html")
            .map(|(_, body)| *body)
            .expect("report.html default present");
        std::fs::write(dir.path().join("report.html"), current_report).expect("write current body");

        let buffer = std::sync::Arc::new(Mutex::new(Vec::<u8>::new()));
        let writer = CapturedLogs(buffer.clone());
        let subscriber = tracing_subscriber::fmt()
            .with_writer(move || writer.clone())
            .with_max_level(tracing::Level::WARN)
            .with_ansi(false)
            .finish();

        {
            let _log_guard = tracing::subscriber::set_default(subscriber);
            upgrade_untouched_defaults_on_startup(dir.path()).expect("upgrade ok");
        }

        let captured = String::from_utf8(buffer.lock().unwrap_or_else(|e| e.into_inner()).clone())
            .expect("captured logs are UTF-8");

        assert!(
            captured.contains("Skipped auto-upgrade"),
            "skipping a user-customized template must emit a warn, captured logs were: \
             {captured:?}"
        );
        assert!(
            captured.contains("act_handover.html"),
            "the warn must name the skipped file, captured logs were: {captured:?}"
        );
        assert!(
            !captured.contains("report.html"),
            "an already-current file must not be reported as skipped, captured logs were: \
             {captured:?}"
        );

        // Non-vacuous: the skip is still a real skip — the bytes are preserved.
        let contents =
            std::fs::read_to_string(dir.path().join("act_handover.html")).expect("still exists");
        assert_eq!(contents, custom);
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

    /// IN-05: a read-only `templates/` directory must not fail startup.
    ///
    /// `AppCtx::build` calls both startup functions with `?`, so before this
    /// fix a portable install on a read-only share or a locked install
    /// directory refused to start — even though `load_template` would have
    /// served the embedded defaults and printing would have worked.
    #[test]
    #[cfg(unix)]
    fn readonly_templates_dir_does_not_fail_startup() {
        use std::os::unix::fs::PermissionsExt;

        let _guard = ENV_GUARD.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::tempdir().expect("tempdir");
        let templates_dir = dir.path().join("templates");
        std::fs::create_dir_all(&templates_dir).expect("create templates dir");

        // r-xr-xr-x — traversable and readable, but not writable.
        std::fs::set_permissions(&templates_dir, std::fs::Permissions::from_mode(0o555))
            .expect("chmod read-only");

        let materialize = materialize_defaults_on_startup(&templates_dir);
        let upgrade = upgrade_untouched_defaults_on_startup(&templates_dir);

        // Restore write permission so TempDir::drop can clean up.
        let _ = std::fs::set_permissions(&templates_dir, std::fs::Permissions::from_mode(0o755));

        assert!(
            materialize.is_ok(),
            "a read-only templates dir must not fail startup, got {materialize:?}"
        );
        assert!(
            upgrade.is_ok(),
            "a read-only templates dir must not fail startup, got {upgrade:?}"
        );

        // Non-vacuous: rendering still degrades cleanly to the embedded default.
        let embedded = DEFAULT_HTML_TEMPLATES
            .iter()
            .find(|(name, _)| *name == "act_handover.html")
            .map(|(_, body)| *body)
            .expect("act_handover.html default present");
        assert_eq!(
            load_template(&templates_dir, "act_handover.html", embedded),
            embedded,
            "rendering must still work off the embedded default"
        );
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
