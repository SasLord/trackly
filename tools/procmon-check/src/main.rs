//! procmon-check — Windows-only orchestrator that wraps Sysinternals ProcMon
//! around `trackly --self-test` to assert NO writes leak outside a portable
//! sandbox (FOUND-11, BLD-06, ROADMAP Phase 1 success criterion #1).
//!
//! On non-Windows hosts the binary is a no-op so `cargo build --workspace`
//! succeeds on macOS / Linux dev machines (Plan 01-01 invariant).

#[cfg(not(windows))]
fn main() {
    println!("procmon-check is Windows-only; skipping on this host");
    std::process::exit(0);
}

#[cfg(windows)]
mod csv_check;
#[cfg(windows)]
mod procmon;
#[cfg(windows)]
mod sandbox;

#[cfg(windows)]
fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let trackly_exe = std::path::PathBuf::from(
        args.get(1)
            .ok_or_else(|| anyhow::anyhow!("usage: procmon-check <path-to-trackly.exe>"))?,
    );
    if !trackly_exe.exists() {
        anyhow::bail!("trackly.exe not found at {:?}", trackly_exe);
    }

    // Step 1: cyrillic sandbox at %TEMP%\trackly_procmon_<uuid>\Документы\Учёт\Trackly\
    // (proves success criterion #1 — cyrillic install path — AND FOUND-11 — no APPDATA
    // writes — in a single fixture.)
    let sandbox = sandbox::create_sandbox()?;
    let trackly_in_sandbox = sandbox::copy_trackly(&trackly_exe, &sandbox)?;
    eprintln!("[procmon-check] sandbox: {}", sandbox.display());
    eprintln!("[procmon-check] trackly: {}", trackly_in_sandbox.display());

    // Step 2: ensure Procmon.exe is on PATH (or download + extract).
    let procmon_exe = procmon::ensure_procmon_on_path()?;
    eprintln!("[procmon-check] procmon: {}", procmon_exe.display());

    // Step 3: run capture + export.
    let (pml, csv) = procmon::run_capture(&procmon_exe, &trackly_in_sandbox, &sandbox)?;
    eprintln!("[procmon-check] pml: {}", pml.display());
    eprintln!("[procmon-check] csv: {}", csv.display());

    // Step 4: assert no forbidden writes.
    csv_check::assert_no_forbidden_writes(&csv, &sandbox)?;

    eprintln!("[procmon-check] PASS \u{2014} no writes outside sandbox detected");
    Ok(())
}
