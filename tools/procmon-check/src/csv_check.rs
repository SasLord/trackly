//! Parse ProcMon CSV export and assert no writes leak outside the sandbox
//! (FOUND-11 enforcement). Windows-only.
//!
//! The CSV column layout from `Procmon.exe /SaveAs trace.csv` is:
//!   `Time of Day,Process Name,PID,Operation,Path,Result,Detail`
//! We look up columns by NAME (resilient to ordering changes between ProcMon
//! versions) rather than by index.

use anyhow::{bail, Context, Result};
use std::path::Path;

/// Forbidden path fragments (uppercased; back-slash-normalized). Any row whose
/// `Path` column contains one of these AFTER normalization is a leak.
const FORBIDDEN_FRAGMENTS: &[&str] = &[
    "\\APPDATA\\LOCAL\\",
    "\\APPDATA\\ROAMING\\",
    "\\APPDATA\\LOCALLOW\\",
    "\\PROGRAMDATA\\",
];

/// Operations that count as a write (CreateFile only flags an offense if it
/// was opened with write access — ProcMon's CSV does not break that out
/// per-row, so we treat any CreateFile under a forbidden prefix as suspect).
const WRITE_OPERATIONS: &[&str] = &[
    "WriteFile",
    "CreateFile",
    "SetEndOfFileInformationFile",
    "WriteFileGather",
    "SetAllocationInformationFile",
    "SetBasicInformationFile",
];

/// Walk the ProcMon CSV and bail if any `trackly.exe` write lands outside
/// the sandbox. Returns Ok(()) on a clean run.
pub fn assert_no_forbidden_writes(csv_path: &Path, sandbox: &Path) -> Result<()> {
    let sandbox_norm = normalize(&sandbox.to_string_lossy());
    let temp_norm = std::env::var_os("TEMP")
        .map(|t| normalize(&t.to_string_lossy()))
        .unwrap_or_default();

    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_path(csv_path)
        .with_context(|| format!("open procmon CSV at {}", csv_path.display()))?;

    let headers = rdr.headers().context("read CSV header row")?.clone();
    let path_col = headers
        .iter()
        .position(|h| h.eq_ignore_ascii_case("Path"))
        .ok_or_else(|| anyhow::anyhow!("'Path' column not found in CSV headers: {headers:?}"))?;
    let op_col = headers
        .iter()
        .position(|h| h.eq_ignore_ascii_case("Operation"))
        .ok_or_else(|| {
            anyhow::anyhow!("'Operation' column not found in CSV headers: {headers:?}")
        })?;
    let proc_col = headers
        .iter()
        .position(|h| h.eq_ignore_ascii_case("Process Name"))
        .ok_or_else(|| {
            anyhow::anyhow!("'Process Name' column not found in CSV headers: {headers:?}")
        })?;

    let mut offenses: Vec<String> = Vec::new();
    let mut inspected: usize = 0;

    for (i, rec) in rdr.records().enumerate() {
        let rec = rec.with_context(|| format!("read CSV row {i}"))?;
        let proc_name = rec.get(proc_col).unwrap_or("");
        if !proc_name.eq_ignore_ascii_case("trackly.exe") {
            continue;
        }
        let op = rec.get(op_col).unwrap_or("");
        if !WRITE_OPERATIONS.iter().any(|w| op == *w) {
            continue;
        }
        let path = rec.get(path_col).unwrap_or("");
        inspected += 1;
        let path_norm = normalize(path);

        // Allow: writes inside the sandbox.
        if !sandbox_norm.is_empty() && path_norm.starts_with(&sandbox_norm) {
            continue;
        }
        // Allow: anything under %TEMP% (sandbox itself lives under %TEMP%;
        // transient temp files are not a portability violation).
        if !temp_norm.is_empty() && path_norm.starts_with(&temp_norm) {
            continue;
        }
        // Forbidden: any APPDATA / ProgramData fragment anywhere in path.
        for bad in FORBIDDEN_FRAGMENTS {
            if path_norm.contains(bad) {
                offenses.push(format!("row {i}: {op} -> {path}"));
                break;
            }
        }
    }

    eprintln!(
        "[procmon-check] inspected {inspected} trackly.exe write row(s); offenses={}",
        offenses.len()
    );

    if !offenses.is_empty() {
        bail!(
            "portable-mode leak: {} forbidden write(s) detected:\n  {}",
            offenses.len(),
            offenses.join("\n  "),
        );
    }
    Ok(())
}

/// Uppercase + flip forward slashes to back so the forbidden-fragment match
/// is case-insensitive AND slash-insensitive.
fn normalize(s: &str) -> String {
    s.replace('/', "\\").to_ascii_uppercase()
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn write_csv(body: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().expect("tmp csv");
        f.write_all(body.as_bytes()).expect("write csv");
        f.flush().expect("flush");
        f
    }

    #[test]
    fn detects_appdata_local_write() {
        let csv = write_csv(
            "Time of Day,Process Name,PID,Operation,Path,Result,Detail\n\
             10:00,trackly.exe,123,WriteFile,C:\\Users\\runner\\AppData\\Local\\trackly\\foo.dat,SUCCESS,\n\
             10:01,trackly.exe,123,WriteFile,C:\\Temp\\sandbox\\trackly.db,SUCCESS,\n\
             10:02,system,4,WriteFile,C:\\Users\\runner\\AppData\\Local\\sysstuff,SUCCESS,\n",
        );
        let sandbox = Path::new("C:\\Temp\\sandbox");
        let err = assert_no_forbidden_writes(csv.path(), sandbox).expect_err("must fail");
        let msg = format!("{err}");
        assert!(msg.contains("AppData"), "msg should mention AppData: {msg}");
        assert!(
            msg.contains("trackly\\foo.dat"),
            "msg should name offender: {msg}"
        );
        // The `system,4` row must NOT be flagged (process != trackly.exe).
        assert!(
            !msg.contains("sysstuff"),
            "non-trackly row leaked into offenses: {msg}"
        );
    }

    #[test]
    fn clean_csv_passes() {
        let csv = write_csv(
            "Time of Day,Process Name,PID,Operation,Path,Result,Detail\n\
             10:00,trackly.exe,123,WriteFile,C:\\Temp\\sandbox\\trackly.db,SUCCESS,\n\
             10:01,trackly.exe,123,WriteFile,C:\\Temp\\sandbox\\logs\\trackly.log,SUCCESS,\n",
        );
        let sandbox = Path::new("C:\\Temp\\sandbox");
        assert_no_forbidden_writes(csv.path(), sandbox).expect("clean csv must pass");
    }

    #[test]
    fn forward_slash_and_lowercase_still_caught() {
        let csv = write_csv(
            "Time of Day,Process Name,PID,Operation,Path,Result,Detail\n\
             10:00,trackly.exe,123,CreateFile,c:/users/x/appdata/roaming/trackly/cache,SUCCESS,\n",
        );
        let sandbox = Path::new("C:\\Temp\\sandbox");
        let err = assert_no_forbidden_writes(csv.path(), sandbox).expect_err("must fail");
        let msg = format!("{err}");
        assert!(
            msg.contains("appdata"),
            "lowercase/slashes must still be caught: {msg}"
        );
    }
}
