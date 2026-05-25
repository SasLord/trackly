# procmon-check

Windows-only CI helper that wraps Sysinternals
[ProcMon](https://learn.microsoft.com/en-us/sysinternals/downloads/procmon)
around `trackly --self-test` and asserts no writes leak outside a portable
sandbox.

## Why this exists

Phase 1 success criterion #1 (cyrillic install path) and requirements
**FOUND-11** + **BLD-06** demand a behavioral proof — not a code-review
or clippy proof — that `trackly.exe` writes **only** to its install
directory, even when:

- the path contains cyrillic characters (`Документы\Учёт\Trackly\`),
- a transitive dependency silently calls `dirs::cache_dir()` or
  `dirs::data_local_dir()` (which clippy denies, but `#[allow]` could
  bypass),
- WebView2 misbehaves and tries to default-write to `%LOCALAPPDATA%`
  (Pitfall #1 in `.planning/research/PITFALLS.md`).

Clippy + the workspace `disallowed-methods` list catch the easy cases.
`procmon-check` is the authoritative gate.

## What it does

On Windows:

1. Creates a fresh sandbox at
   `%TEMP%\trackly_procmon_<uuid>\Документы\Учёт\Trackly\` — the cyrillic
   path doubles as the success-criterion-#1 fixture.
2. Copies `trackly.exe` into the sandbox.
3. Finds (or downloads + extracts) `Procmon.exe` /
   `Procmon64.exe`.
4. Spawns ProcMon with
   `/AcceptEula /Quiet /Minimized /Runtime 30 /BackingFile <pml>`.
5. Sleeps 2 seconds so the kernel driver attaches.
6. Runs `trackly.exe --self-test` and asserts exit code 0 (a crash would
   silently mask the "no writes" assertion).
7. Sends `/Terminate` to ProcMon and waits for it to exit.
8. Exports the PML to CSV via
   `Procmon.exe /OpenLog <pml> /SaveAs <csv> /Quiet /AcceptEula`.
9. Walks the CSV: for every row where
   `Process Name == trackly.exe AND Operation in {WriteFile, CreateFile,
   SetEndOfFileInformationFile, WriteFileGather, SetAllocationInformationFile,
   SetBasicInformationFile}`, asserts the `Path` either starts with the
   sandbox prefix or with `%TEMP%`. Any path containing
   `\AppData\Local\`, `\AppData\Roaming\`, `\AppData\LocalLow\`, or
   `\ProgramData\` after normalization (uppercased, back-slash) is a leak.
10. Exits 0 on a clean trace; non-zero with the offending CSV row(s) on
    a leak.

On non-Windows: prints
`procmon-check is Windows-only; skipping on this host` and exits 0 so
`cargo build --workspace` works on macOS / Linux dev boxes.

## Local run (Windows VM)

Prerequisites:

- Windows 10/11 with admin rights (ProcMon loads a kernel driver).
- Rust 1.88 (`rust-toolchain.toml` pins this).
- A built `trackly.exe`:
  `cargo build --release -p trackly-app`.

Run:

```pwsh
cargo run --release -p procmon-check -- target\release\trackly.exe
```

Expected output on success:

```
[procmon-check] sandbox: C:\Users\YOU\AppData\Local\Temp\trackly_procmon_<uuid>\Документы\Учёт\Trackly
[procmon-check] trackly: ...\trackly.exe
[procmon-check] procmon: C:\ProcMon\Procmon64.exe
[procmon-check] trackly stdout: ...
[procmon-check] trackly stderr: self-test OK: schema_version=12, portable=...
[procmon-check] pml: ...\trace.pml
[procmon-check] csv: ...\trace.csv
[procmon-check] inspected N trackly.exe write row(s); offenses=0
[procmon-check] PASS — no writes outside sandbox detected
```

Example failure output:

```
[procmon-check] inspected 142 trackly.exe write row(s); offenses=1
Error: portable-mode leak: 1 forbidden write(s) detected:
  row 87: WriteFile -> C:\Users\runner\AppData\Local\trackly\settings.json
```

## Inspecting a failed trace manually

The sandbox is **not** auto-cleaned (the `.pml` is useful for forensic
analysis). On CI, the `procmon` job uploads it as an artifact named
`procmon-failure-<run_id>` (14-day retention) — download from the
GitHub Actions UI.

Open the `.pml` in the ProcMon UI:

```pwsh
Procmon64.exe /OpenLog "<path-to-trace.pml>"
```

Filter `Process Name is trackly.exe` and `Operation is WriteFile` to see
what the assertion saw.

## CI integration

`.github/workflows/ci-full.yml` runs the `procmon` job on `windows-latest`
after the `matrix` job (ubuntu/macos/windows fmt+clippy+test+lint+release)
succeeds. It:

- downloads `ProcessMonitor.zip` from
  `https://download.sysinternals.com/files/ProcessMonitor.zip` and puts
  the extracted directory on PATH so `procmon-check` finds Procmon.exe
  immediately (avoids the in-process download fallback);
- builds `trackly-app` and `procmon-check` in release;
- runs
  `cargo run --release -p procmon-check -- target/release/trackly.exe`;
- on failure, uploads the `.pml` + `.csv` from
  `${{ runner.temp }}/trackly_procmon_*/**/` as the
  `procmon-failure-<run_id>` artifact.

## Why the cyrillic path

A single fixture covers two requirements:

- **Success criterion #1**: the application MUST run from a cyrillic
  install path without errors. If `CreateFileW` mishandles UTF-16
  cyrillic, the binary crashes; ProcMon-check detects this because
  `trackly --self-test` exits non-zero (T-06-04 mitigation).
- **FOUND-11**: zero writes to `%APPDATA%`, `%LOCALAPPDATA%`,
  `~\AppData\`, `\ProgramData\`. ProcMon captures every `WriteFile` and
  `CreateFile`; the CSV walker flags any leak.

## Filter template

`filter.pmc.template` documents the conceptual filter (process name +
operation list). The current implementation does NOT use ProcMon's
`/LoadConfig` flag — the CSV-level post-filter in `csv_check.rs` is the
authoritative gate. The template is kept as a documentation artifact so a
future maintainer can drop a stricter ProcMon-side filter (for
performance) without rewriting the Rust code.

## Troubleshooting

- **`Procmon driver failed to load`** — ProcMon needs admin rights on
  Windows. CI runners have them by default; local Windows VMs may need
  `runas`.
- **`trackly --self-test exited non-zero`** — the binary itself crashed
  inside the cyrillic sandbox; check ProcMon stderr for the cause. This
  is the cyrillic-encoding bug we want to catch.
- **`procmon CSV not produced`** — the `/SaveAs` export failed. Inspect
  the `.pml` manually; if it is well-formed, re-run the export step
  outside ProcMon to bisect.
- **CSV parsing fails with non-UTF-8 bytes** — Sysinternals ProcMon
  exports UTF-8 by default. If a future Windows locale flips this to
  Windows-1252, add `encoding_rs` decoding before the `csv` reader.
