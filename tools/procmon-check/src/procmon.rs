//! ProcMon orchestration: locate Procmon.exe, run capture, export PML to CSV.
//! Windows-only.
//!
//! Capture sequence (per RESEARCH Code Example 5):
//!   1. Spawn `Procmon.exe /AcceptEula /Quiet /Minimized /Runtime 30 /BackingFile <pml>`.
//!   2. Sleep 2s so the kernel driver attaches before we start the workload.
//!   3. Run `trackly.exe --self-test` and capture exit code (must be 0 — see
//!      T-06-04 mitigation: a cyrillic-path crash would mask all writes).
//!   4. Send `/Terminate` so ProcMon flushes the .pml.
//!   5. Convert: `Procmon.exe /OpenLog <pml> /SaveAs <csv> /Quiet /AcceptEula`.
//!
//! NOTE: `capture_args` / `export_args` are extracted as pure functions so
//! the argv list is unit-testable WITHOUT spawning ProcMon (the integration
//! test is the CI ProcMon job itself).

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

pub const PROCMON_URL: &str = "https://download.sysinternals.com/files/ProcessMonitor.zip";

/// Locate Procmon.exe (preferring 64-bit). If neither `Procmon.exe` nor
/// `Procmon64.exe` is on PATH, download ProcessMonitor.zip from the
/// Sysinternals canonical URL, extract, and return the path to the extracted
/// binary. The extracted directory is leaked via `mem::forget` so the files
/// outlive the function (the CI runner is ephemeral; no cleanup needed).
pub fn ensure_procmon_on_path() -> Result<PathBuf> {
    for name in &["Procmon64.exe", "Procmon.exe"] {
        if let Ok(out) = Command::new("where").arg(name).output() {
            if out.status.success() {
                if let Some(first) = String::from_utf8_lossy(&out.stdout).lines().next() {
                    let path = PathBuf::from(first.trim());
                    if path.exists() {
                        return Ok(path);
                    }
                }
            }
        }
    }

    // Not found — download + extract.
    eprintln!("[procmon-check] ProcMon not on PATH; downloading from {PROCMON_URL}");
    let dl_dir = tempfile::TempDir::new().context("create download tempdir")?;
    let zip_path = dl_dir.path().join("pm.zip");
    let bytes = reqwest::blocking::get(PROCMON_URL)
        .context("GET ProcessMonitor.zip")?
        .error_for_status()
        .context("ProcessMonitor.zip HTTP status")?
        .bytes()
        .context("read ProcessMonitor.zip body")?;
    std::fs::write(&zip_path, &bytes).context("write ProcessMonitor.zip")?;
    eprintln!("[procmon-check] downloaded {} bytes", bytes.len());

    // SHA256 audit log — Sysinternals does not publish stable checksums,
    // so this is NOT gated; useful for forensic comparison across CI runs.
    use sha2::{Digest, Sha256};
    let hash = format!("{:x}", Sha256::digest(&bytes));
    eprintln!("[procmon-check] ProcessMonitor.zip sha256={hash}");

    let extract_dir = dl_dir.path().join("pm");
    std::fs::create_dir_all(&extract_dir).context("create extract dir")?;
    let zip_file = std::fs::File::open(&zip_path).context("open downloaded zip")?;
    let mut archive = zip::ZipArchive::new(zip_file).context("parse ProcessMonitor.zip")?;
    archive.extract(&extract_dir).context("extract zip")?;

    for name in &["Procmon64.exe", "Procmon.exe"] {
        let candidate = extract_dir.join(name);
        if candidate.exists() {
            // Persist the extracted dir for the rest of the process so the
            // returned path stays valid; CI runner cleanup handles disposal.
            std::mem::forget(dl_dir);
            return Ok(candidate);
        }
    }
    bail!("ProcMon not found in extracted ProcessMonitor.zip")
}

/// Build the argv list for the capture invocation. Pure function for unit
/// testing (argv must include `/AcceptEula /Quiet /Minimized /Runtime 30
/// /BackingFile <pml>`).
pub fn capture_args(pml: &Path) -> Vec<String> {
    vec![
        "/AcceptEula".to_string(),
        "/Quiet".to_string(),
        "/Minimized".to_string(),
        "/Runtime".to_string(),
        "30".to_string(),
        "/BackingFile".to_string(),
        pml.to_string_lossy().to_string(),
    ]
}

/// Build the argv list for the PML→CSV export invocation. Pure function for
/// unit testing.
pub fn export_args(pml: &Path, csv: &Path) -> Vec<String> {
    vec![
        "/OpenLog".to_string(),
        pml.to_string_lossy().to_string(),
        "/SaveAs".to_string(),
        csv.to_string_lossy().to_string(),
        "/Quiet".to_string(),
        "/AcceptEula".to_string(),
    ]
}

/// Run ProcMon, invoke `target_exe --self-test`, export the PML to CSV.
/// Returns `(pml_path, csv_path)` under `sandbox/`.
pub fn run_capture(
    procmon_exe: &Path,
    target_exe: &Path,
    sandbox: &Path,
) -> Result<(PathBuf, PathBuf)> {
    let pml = sandbox.join("trace.pml");
    let csv = sandbox.join("trace.csv");

    // Step A: spawn ProcMon in the background.
    let mut procmon = Command::new(procmon_exe)
        .args(capture_args(&pml))
        .spawn()
        .context("spawn Procmon for capture")?;

    // Step B: 2s for driver attach. If this is too short on slow runners,
    // bump (cheap — total budget is 30s /Runtime window).
    std::thread::sleep(Duration::from_secs(2));

    // Step C: run trackly --self-test. Exit code MUST be 0 (T-06-04 mitigation:
    // a cyrillic-path crash would silently mask "no writes detected").
    let out = Command::new(target_exe)
        .arg("--self-test")
        .output()
        .context("invoke trackly --self-test")?;
    eprintln!(
        "[procmon-check] trackly stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    eprintln!(
        "[procmon-check] trackly stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    if !out.status.success() {
        // Best-effort terminate ProcMon before bailing.
        let _ = Command::new(procmon_exe).arg("/Terminate").status();
        let _ = procmon.wait();
        bail!("trackly --self-test exited non-zero: {:?}", out.status);
    }

    // Step D: terminate ProcMon cleanly (writes the .pml header trailer).
    let _ = Command::new(procmon_exe).arg("/Terminate").status();
    let _ = procmon.wait();

    if !pml.exists() {
        bail!("procmon did not produce PML at {}", pml.display());
    }

    // Step E: export to CSV (filtering happens in csv_check, not ProcMon).
    let export_status = Command::new(procmon_exe)
        .args(export_args(&pml, &csv))
        .status()
        .context("export procmon PML to CSV")?;
    if !export_status.success() {
        bail!("procmon /SaveAs failed: {export_status:?}");
    }
    if !csv.exists() {
        bail!("procmon CSV not produced at {}", csv.display());
    }
    Ok((pml, csv))
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn capture_args_contains_required_flags() {
        let pml = PathBuf::from("C:\\sandbox\\trace.pml");
        let args = capture_args(&pml);
        assert!(args.iter().any(|a| a == "/AcceptEula"));
        assert!(args.iter().any(|a| a == "/Quiet"));
        assert!(args.iter().any(|a| a == "/Minimized"));
        assert!(args.iter().any(|a| a == "/Runtime"));
        assert!(args.iter().any(|a| a == "30"));
        assert!(args.iter().any(|a| a == "/BackingFile"));
        assert!(args.iter().any(|a| a.contains("trace.pml")));
    }

    #[test]
    fn export_args_contains_required_flags() {
        let pml = PathBuf::from("C:\\sandbox\\trace.pml");
        let csv = PathBuf::from("C:\\sandbox\\trace.csv");
        let args = export_args(&pml, &csv);
        assert!(args.iter().any(|a| a == "/OpenLog"));
        assert!(args.iter().any(|a| a == "/SaveAs"));
        assert!(args.iter().any(|a| a == "/Quiet"));
        assert!(args.iter().any(|a| a == "/AcceptEula"));
        assert!(args.iter().any(|a| a.contains("trace.pml")));
        assert!(args.iter().any(|a| a.contains("trace.csv")));
    }
}
