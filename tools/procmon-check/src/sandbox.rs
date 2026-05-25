//! Cyrillic sandbox setup for ProcMon-check (Windows-only).
//!
//! Constructs `%TEMP%\trackly_procmon_<uuid>\Документы\Учёт\Trackly\` —
//! the cyrillic path doubles as the success-criterion-#1 fixture.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Create a fresh cyrillic-path sandbox under `%TEMP%` and return its absolute
/// path. The sandbox is intentionally NOT a `tempfile::TempDir` guard — the
/// CI runner is ephemeral and the `.pml` trace is useful for forensic
/// inspection if the assertion fails. Cleanup is GitHub's job.
pub fn create_sandbox() -> Result<PathBuf> {
    let temp = std::env::temp_dir();
    let id = uuid::Uuid::new_v4();
    // Intentional cyrillic path: covers ROADMAP success criterion #1 + FOUND-11
    // in one fixture. Если CreateFileW не справляется с UTF-16-кодированной
    // кириллицей — это и есть тот баг, который мы хотим увидеть.
    let sandbox = temp
        .join(format!("trackly_procmon_{id}"))
        .join("Документы")
        .join("Учёт")
        .join("Trackly");
    std::fs::create_dir_all(&sandbox)
        .with_context(|| format!("create cyrillic sandbox at {}", sandbox.display()))?;
    Ok(sandbox)
}

/// Copy `trackly.exe` into the sandbox so paths::resolve() roots all I/O
/// inside the cyrillic directory. Returns the destination path.
#[allow(
    clippy::disallowed_methods,
    reason = "std::fs::copy is banned for DB backup paths (rusqlite::backup::Backup); copying an .exe binary into a sandbox is the test-harness use case the clippy.toml comment explicitly allows."
)]
pub fn copy_trackly(src: &Path, sandbox: &Path) -> Result<PathBuf> {
    let dst = sandbox.join("trackly.exe");
    std::fs::copy(src, &dst)
        .with_context(|| format!("copy {} -> {}", src.display(), dst.display()))?;
    Ok(dst)
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn create_sandbox_returns_cyrillic_path_that_exists() {
        let s = create_sandbox().expect("create sandbox");
        assert!(s.exists(), "sandbox should exist on disk: {}", s.display());
        assert!(s.is_dir(), "sandbox should be a directory");
        let s_str = s.to_string_lossy();
        assert!(
            s_str.contains("Документы"),
            "sandbox path missing cyrillic 'Документы': {s_str}"
        );
        assert!(
            s_str.contains("Учёт"),
            "sandbox path missing cyrillic 'Учёт': {s_str}"
        );
        assert!(
            s_str.contains("Trackly"),
            "sandbox path missing 'Trackly': {s_str}"
        );
        // Clean up — best effort.
        let _ = std::fs::remove_dir_all(s.ancestors().nth(2).unwrap_or(&s));
    }
}
