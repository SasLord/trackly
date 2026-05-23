# Pitfalls Research

**Domain:** Portable Tauri + Svelte desktop with embedded LAN HTTP server, SQLite WAL, AD bind, multi-vendor SNMP printer monitoring (Trackly)
**Researched:** 2026-05-24
**Confidence:** HIGH for Tauri/SQLite/AD/SNMP generic; MEDIUM for Pantum-specific (vendor support is sparse publicly); HIGH for Russian-locale traps (Cyrillic, encoding, TZ).

---

## Critical Pitfalls

### Pitfall 1: Portable mode leaks data into %APPDATA% / %LOCALAPPDATA%

**What goes wrong:**
"Portable" build still creates `%LOCALAPPDATA%\<app>\EBWebView` (WebView2 user data), and any Tauri code using `tauri::path::BaseDirectory::AppData / AppLocalData / AppConfig` writes outside the executable folder. The user copies `trackly.exe` to a USB stick, runs it on another machine, and discovers settings/cookies/DB orphans were left behind.

**Why it happens:**
- WebView2 defaults its user-data folder to `LOCALAPPDATA\<app>\EBWebView`. Tauri does not redirect this automatically.
- `BaseDirectory::AppData` and the `app_data_dir()` helper resolve to system AppData by default.
- Sqlite/migration code, the updater plugin, and log_dir defaults all silently choose AppData if not overridden.
- "Portable" is a build-mode myth in Tauri 2 — it must be engineered.

**How to avoid:**
- Set `WEBVIEW2_USER_DATA_FOLDER` env var to `<exe_dir>/data/webview` **before** Tauri builder runs (in `main()` first line, before `tauri::Builder`).
- Define a single `app_root()` function returning `std::env::current_exe()?.parent()` and route every path (DB, logs, config, backups, templates, cache, webview) through it.
- Never call `app_handle.path().app_data_dir()` etc. — ban it via clippy `disallowed-methods` lint.
- Disable Tauri updater plugin (or configure its cache dir under `app_root()`).
- Add an integration test that runs the app in a sandboxed temp folder and asserts no writes anywhere except `<exe_dir>`.

**Warning signs:**
- `Process Monitor` (Windows) shows writes to `\AppData\` paths after starting the app.
- A second user on the same machine sees the first user's settings/cache.
- Running from a read-only directory (e.g., USB with write-protect) crashes or silently fails.
- `dir %LOCALAPPDATA%\Trackly` returns anything other than "not found".

**Phase to address:**
Phase 1 (foundation) — establish `app_root()` discipline and the no-AppData rule before any feature code lands. Add the procmon-based test in CI Windows job.

---

### Pitfall 2: SQLite "database is locked" under server-mode concurrent writers

**What goes wrong:**
Admin desktop is editing an act while a specialist in the browser submits a return at the same instant. One transaction sees `SQLITE_BUSY`, the user gets a red error toast, and refreshes. Or worse: the desktop reads cached data, the browser commits, and the desktop's "save" silently overwrites with stale values.

**Why it happens:**
- WAL mode only solves **reader/writer** blocking; it does **not** allow concurrent writers.
- Default `busy_timeout = 0` returns `SQLITE_BUSY` immediately.
- Read-then-upgrade transactions (`BEGIN` → `SELECT` → `UPDATE`) deadlock when two upgraders race; `BEGIN IMMEDIATE` is required.
- A long transaction (e.g., bulk CSV import, PDF rendering inside a tx) blocks every other writer for the duration.
- App-level optimistic concurrency (row version / updated_at compare) is usually skipped in MVP.

**How to avoid:**
- On every connection: `PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000; PRAGMA synchronous=NORMAL; PRAGMA foreign_keys=ON; PRAGMA wal_autocheckpoint=1000;`
- Use a single writer pool (size = 1) and a separate reader pool (size = N) — `r2d2` or `sqlx` SqlitePool with `.max_connections(1)` for writes; route every mutating query through it.
- Use `BEGIN IMMEDIATE` for any tx that will write.
- Keep write transactions short — no I/O (PDF, HTTP, file write) inside a tx.
- Add row-level `version` column on `acts`, `cartridges`, `devices`; refuse update if version mismatch with 409-style error.
- Do not put the DB file on a network share — WAL requires shared memory between processes; over SMB it corrupts.

**Warning signs:**
- Intermittent "database is locked" in logs even at low load → busy_timeout missing.
- `-wal` file grows >100 MB → checkpoints aren't running → long-lived read tx holding back checkpoints.
- Users report "I saved but my change disappeared" → no optimistic concurrency.
- Random crashes when DB is on `\\server\share\trackly.db` → forbid this in Settings UI.

**Phase to address:**
Phase 1 (foundation / data layer). Pragmas, writer-pool, optimistic concurrency, and the "no network share" guard must ship with the schema.

---

### Pitfall 3: Pantum BM5100ADN spooler hang — SNMP cannot reliably detect it

**What goes wrong:**
The product's originating pain point. The Pantum BM5100ADN periodically stops printing in AD environments without a visible printer error — print jobs queue on the Windows host, the printer's own panel shows "Ready", and SNMP `hrPrinterStatus` returns `idle(3)` or `printing(4)` while nothing actually prints. The team builds a monitor that asks the printer "are you OK?" and the printer says "yes" — but jobs are stuck on the Windows spooler, not on the device.

**Why it happens:**
- The hang is on the **Windows print spooler service** (host side), not on the printer hardware. SNMP polls the printer, which is healthy.
- Pantum's SNMP firmware support is shallow — `hrDeviceStatus`, `prtAlertTable`, `prtMarkerSuppliesLevel` may return constants or `noSuchObject`, especially for the BM5100 series.
- Job queue depth (`prtConsoleDisplayBufferText`, `hrPrinterDetectedErrorState`) is often not implemented.
- Many "fix" articles point at clearing `C:\Windows\System32\spool\PRINTERS\` and restarting `Spooler` service — confirming the bug lives on Windows, not on the device.
- Driver re-installation is frequently the suggested workaround.

**How to avoid (detection strategy that actually works):**
Combine multiple signals — SNMP alone is insufficient for this printer:
1. **Spooler-side detection** (primary signal): poll the Windows host where the printer is shared. Two options:
   - **Local agent** on the print server (small Rust/Go binary, or a scheduled task) reporting via HTTP to Trackly.
   - **Remote WMI/RPC** from Trackly server to the print host (requires AD service account, firewall rules). Watch `Win32_PrintJob` count over time — same JobId stuck >N minutes = hang.
2. **SNMP cross-check** (secondary signal): poll device every 60s for `prtMarkerLifeCount` (page counter). If a job is queued (signal 1) AND page counter hasn't advanced in 5 min → confirmed hang.
3. **Network reachability** (sanity check): TCP 9100 (raw print) open + ICMP responds → device is alive; if dead, it's not the spooler bug.
4. **Alert, don't auto-fix in v1.** Restarting the spooler kills every other queued job for other printers; do it only on operator confirmation and only for the specific printer queue (`Restart-Service Spooler` is too broad — prefer `Remove-PrintJob` + driver restart per queue).

**Warning signs (telling the user something's wrong even before SNMP):**
- `Win32_PrintJob.JobStatus` includes `PRINTING|RETAINED` or `ERROR` for >5 minutes.
- `Get-Printer | Select-Object -ExpandProperty PrinterStatus` returns 4 (Error) or paused.
- Same JobId appears in consecutive polls with identical PagesPrinted.

**Phase to address:**
Phase "Принтеры" (printer monitoring). Detection mechanism is research-heavy — flag for prototype/spike before final design. Auto-restart is explicitly deferred to a later phase per PROJECT.md (correct decision; do not pull it forward).

---

### Pitfall 4: AD bind from non-domain dev machine (macOS) without realistic test path

**What goes wrong:**
LDAP code "works" in dev (no AD reachable → mocked), passes review, hits a Windows AD-joined test box, and fails with one of: TLS cert untrusted (corp CA), referral loop, `LDAP_OPERATIONS_ERROR` because `sAMAccountName=us100` was sent as the DN, channel binding requirement rejected, 60-second timeout because the firewall blackholes 389 and tester thinks the app is hung.

**Why it happens:**
- Active Directory commonly requires **LDAP channel binding** (post-2023 hardening). `ldap3` supports the tls-server-end-point token, but only on TLS connections — plain `ldap://` will be rejected silently or noisily.
- AD distinguishes between **bind DN** (`CN=User,OU=...,DC=corp,DC=local`) and **logon names** (`sAMAccountName=us100`, `userPrincipalName=us100@corp.local`). You can bind with UPN directly, but `sAMAccountName` requires either `DOMAIN\us100` (NTLM-style, not pure LDAP) or a search-then-bind two-step.
- Corp AD almost always uses an internal CA → `native-tls` rejects it unless the CA is trusted.
- Default `ldap3` timeouts are generous → blocking the app's UI thread if you call bind synchronously.
- Connecting to a single DC by IP fails when that DC reboots — you need DNS round-robin or SRV records (`_ldap._tcp.dc._msdcs.corp.local`).

**How to avoid:**
- **Architecture:** bind via UPN (`us100@corp.local`) — works universally and doesn't require a search-then-bind.
- For `sAMAccountName` flow: app-scoped service account binds first, runs `(sAMAccountName=us100)` search to get the user's `userPrincipalName`, then bind as that user.
- Always use `ldaps://` (636) or `StartTLS`; provide a UI setting for "Trust CA from PEM file" with explicit path.
- Set `LdapConnSettings { conn_timeout: 5s, no_tls_verify: false }`. Expose `conn_timeout` in settings; default 5s.
- Discover DCs via DNS SRV records, not hardcoded hostnames. Use `hickory-resolver` for SRV queries.
- **Never store AD passwords.** Use them only for the immediate bind, then zeroize (`zeroize` crate on the password buffer).
- For dev on macOS: use Samba/`389-ds` in Docker as a stand-in AD; CI runs against a real Windows Server 2022 in a Vagrant/Hyper-V box (slow but realistic).

**Warning signs:**
- "Strong authentication required" or `LDAP_STRONG_AUTH_REQUIRED` → channel binding / signing enforcement; you're on plain LDAP and AD is set to require sealing.
- Bind succeeds in dev mock, fails in prod with `LDAP_INVALID_CREDENTIALS` → you sent `sAMAccountName` as DN.
- 30+ second hangs at login → DC unreachable, no timeout configured.
- AD password appears in any log line or stack trace → critical, fix immediately.

**Phase to address:**
Phase "Пользователи — AD-вход" (late phase per PROJECT.md). However, the **constraint** that local user passwords never coexist with AD passwords in clear must be in Phase 1 (auth scaffolding: argon2 hashing, no `password` column anywhere).

---

### Pitfall 5: Backend authorization gap — UI-only role checks

**What goes wrong:**
The Svelte UI hides "Delete device" from the Specialist role. A curious user opens DevTools, calls the Tauri command or the axum endpoint directly with the request the Admin uses, and deletes anything. Or a Сотрудник (Employee) calls `POST /api/cartridges/issue` from `curl` and issues themselves cartridges.

**Why it happens:**
- Tauri's own docs explicitly state: "command exposure has no inherent security impact if the backend doesn't validate requests independently." Tauri capabilities prevent the frontend from *calling* a command, but in server mode the same logic is exposed over HTTP and the capability system doesn't apply.
- Developers assume "the desktop is the admin" and don't enforce role on Tauri commands either, then enable server mode and forget to add HTTP-side role enforcement.
- Permission code that looks like `if (user.role === 'admin')` lives in a Svelte component, not in axum middleware.

**How to avoid:**
- **Single source of truth for permissions in Rust** — a `Permission` enum and an `authorize(user, perm)` function. Every Tauri command **and** every axum route calls it before doing work.
- Axum middleware tower layer extracts the session from cookie/header, looks up the user, attaches `RequestUser` to extensions; every handler receives `RequestUser` and calls `authorize`.
- Tauri commands receive `tauri::State<SessionStore>` and resolve the current user; same `authorize` call.
- Write a single integration test per route: "Specialist calls DELETE /api/devices/:id → 403". Use a parameterized matrix (role × endpoint).
- UI hiding is a UX nicety, never a security control. Document this in the auth module's README.

**Warning signs:**
- A permission check lives in a `.svelte` file and nowhere else.
- `grep -r "authorize\|require_role" src-tauri/` returns < (number of mutating endpoints + Tauri commands).
- Specialist can hit admin endpoint via curl and get 200.

**Phase to address:**
Phase "Пользователи" (auth/roles). Must ship with the role system itself, not retrofitted later.

---

### Pitfall 6: Cyrillic + Windows paths + Tauri = silent breakage

**What goes wrong:**
User saves the portable folder to `C:\Документы\Учёт\Trackly\` (Cyrillic path). On Windows, file APIs sometimes work, sometimes return "file not found", logs show `fo�.txt`-style replacement chars, and SQLite fails to open the DB because the path crossed a `&str` boundary and got mangled.

**Why it happens:**
- Windows filenames are UTF-16 (technically WTF-8 in Rust's internal representation), and Rust's `Path::to_str()` validates UTF-8 — non-UTF-8 paths return `None` or use replacement chars.
- Many crates take `&str` for paths (instead of `AsRef<Path>`), causing silent corruption.
- SQLite's `Connection::open(path)` does work with UTF-8, but if upstream code already mangled the string, it's too late.
- WebView2 cache path (set via `WEBVIEW2_USER_DATA_FOLDER`) is parsed via Windows APIs that don't always tolerate Cyrillic well in env vars unless the process is UTF-8 manifest-enabled.

**How to avoid:**
- Always pass `PathBuf` / `&Path`, never `&str`, between modules. Add a clippy/lint rule banning `.to_str().unwrap()` on paths.
- Add a Windows manifest setting `activeCodePage=UTF-8` (Windows 10 1903+) — Tauri's WiX/NSIS template should include this; verify it does for your version.
- Test from day 1 with paths containing `Документы`, `Учёт`, spaces, and `№` (the act-number character).
- For SQLite specifically, ensure `bundled` feature is used (so the SQLite library is the upstream version with UTF-8 path support, not the Windows-default one).
- Reject CR/LF/control characters in user-provided locations (e.g., DB-folder setting) with a clear error.

**Warning signs:**
- "File not found" errors mentioning paths with `?` or `�` characters.
- App works in `C:\Trackly\` but not in `C:\Документы\Trackly\`.
- Logs containing replacement chars where the user's path should be.

**Phase to address:**
Phase 1. Establish path-handling conventions and add Cyrillic-path test in CI Windows job before any feature ships.

---

### Pitfall 7: PDF generation with Cyrillic — missing glyphs, empty boxes, font bloat

**What goes wrong:**
The Act of Acceptance prints fine in dev (English placeholder text). In prod with `Сидоров А.А. передаёт Петрову Б.Б.`, every Cyrillic glyph renders as an empty rectangle or vanishes.

**Why it happens:**
- `genpdf`'s built-in fonts are Windows-1252 encoded — zero Cyrillic glyphs.
- `printpdf` requires explicit font embedding; if you embed a font without Cyrillic glyphs (e.g., default Helvetica from PDF standard), no Cyrillic renders.
- Some PDF libraries silently skip glyphs that aren't in the font's cmap instead of erroring loudly.
- Even when the font has glyphs, **subset embedding** can drop them if the library uses Latin-1 codepage extraction.
- Bundling a 5MB TTF per font face × bold × italic × monospace bloats the binary.

**How to avoid:**
- Pick a font family with full Cyrillic coverage: **DejaVu Sans**, **PT Sans**, **Roboto**, or **Inter** (verify the file actually contains U+0410–U+044F and `ё/Ё` U+0401/U+0451).
- Bundle the TTF as a Rust `include_bytes!` resource; don't rely on system fonts (portability).
- Use **Typst** (via `typst` crate) or **`weasyprint`-style HTML→PDF** (`headless_chrome` against the embedded webview is heavy but works) if `printpdf`/`genpdf` font handling becomes painful. **Recommendation:** start with `typst` — it's Rust-native, handles fonts and Unicode natively, and produces beautiful PDFs from declarative templates.
- Add a CI test: render a fixture PDF with known Cyrillic text, hash the output, fail if bytes mismatch.
- Strip the font to required subset offline to control binary size (use `fonttools subset --unicodes=U+0020-U+007E,U+0400-U+04FF`).

**Warning signs:**
- Empty boxes / missing glyphs in the rendered PDF.
- PDF opens with a font-substitution warning in Adobe Reader.
- File size of bundled fonts dominates the binary.

**Phase to address:**
Phase "Акты приёма-передачи" (first feature requiring PDF) — set up font infrastructure once, reuse for all later document templates.

---

### Pitfall 8: CSV roundtrip with Russian Excel — encoding mismatch

**What goes wrong:**
Export CSV from Trackly. Open in Russian Excel — every Cyrillic field is mojibake (`Ð¡Ð¸Ð´Ð¾Ñ€Ð¾Ð²`). User edits, re-imports. Either the import fails, or it imports correctly but silently double-converts.

**Why it happens:**
- Russian Excel on Windows defaults to Windows-1251 (CP1251) for CSV without a BOM.
- UTF-8 BOM (`EF BB BF`) is required for Excel to recognize the file as UTF-8.
- Excel's default CSV delimiter depends on regional settings — semicolon (`;`) in Russian locales, comma (`,`) in en-US.
- Quoting rules differ: Excel uses `""` doubling; some imports expect backslash escaping.

**How to avoid:**
- **Export:** write UTF-8 with BOM by default; use `;` as delimiter on Russian-locale (detect via OS locale or a Settings toggle). Provide a UI checkbox "Совместимость с Excel (Windows-1251, ;)" for users who insist on legacy.
- **Import:** detect encoding via BOM first (UTF-8 BOM, UTF-16 LE/BE BOM); if no BOM, try UTF-8 strict decode → on `UTF8Error` fall back to CP1251 (`encoding_rs`). Show a preview of the first 5 rows decoded in the chosen encoding before commit.
- Auto-detect delimiter by counting `;` vs `,` vs `\t` in the first 10 lines, pick highest; show in preview, allow override.
- Use the `csv` crate with `flexible(true)` so short rows don't fail import; show row-level errors with line numbers.
- Idempotency: define a "natural key" for each CSV entity (e.g., `serial_number` for devices) and use UPSERT semantics with explicit "what to do on conflict" UI choice.

**Warning signs:**
- Exports look fine in dev (TextEdit, VS Code) but mojibake in Excel.
- Users send back files with mixed encodings (some rows UTF-8, some CP1251 — happens after multi-step manual editing).
- Imports succeed with "0 errors" but data has `?` chars.

**Phase to address:**
Phase "Устройства" (first import/export). Build encoding/delimiter detection once as a shared utility.

---

### Pitfall 9: Sequential ID race — act №, cartridge code (C-000001)

**What goes wrong:**
Two specialists in the browser hit "Create act" at the same instant. Both UIs request "next available number", get `42`, both submit. Either one fails with a unique-constraint violation (bad UX) or — worse, if there's no unique index — both succeed and there are now two act №42s.

**Why it happens:**
- The naive flow: `SELECT MAX(number)+1` → return to UI → user edits → submit. The gap between read and write is wide open to races.
- AUTOINCREMENT works for surrogate IDs but the user-visible № may need to be editable (per PROJECT.md: "автопредложение следующего, с возможностью переопределить"), so you can't rely on it alone.
- SQLite gaps from rolled-back transactions create confusion ("Where's №43?") when the user expects gapless sequences.

**How to avoid:**
- Two-step strategy:
  1. UI shows a **suggested** next number from `MAX(number)+1` (or from a counter table). User can override.
  2. On submit, the backend wraps the INSERT in `BEGIN IMMEDIATE` transaction, re-computes the suggested number atomically (or re-validates uniqueness), and inserts. On unique-violation, retry once with the new max+1; if that fails, return the user a friendly "номер уже занят, предлагаем №X" with the new suggestion pre-filled.
- Use a **unique index** on `acts.number` and `cartridges.code` so the DB enforces correctness even if app logic fails.
- For cartridge code `C-000001` format: store the numeric part in a separate `cartridge_seq INTEGER NOT NULL UNIQUE` column; format `C-{:06}` on display.
- Gap policy: document that gaps are allowed and normal; never try to "fill" them.
- For partial returns ("N в1", "N в2"…): store as a separate `return_index` column on a child table, computed within the same transaction as the parent return.

**Warning signs:**
- Unique-constraint violations under load testing with 2+ simulated clients.
- Users reporting "I clicked save and it disappeared without an error."
- Two acts in DB with the same `number`.

**Phase to address:**
Phase "Акты приёма-передачи" (first sequential ID need). Pattern reused for cartridge codes in the cartridges phase.

---

### Pitfall 10: Backup that "works" but restores corrupt

**What goes wrong:**
Auto-backup copies `trackly.db` every night with `std::fs::copy`. Six months in, the disk fails. Restore the latest backup → SQLite says "database disk image is malformed" or, more insidiously, opens fine but is missing the last hour of data (which was in the WAL file that wasn't backed up).

**Why it happens:**
- A naive file-copy of a WAL-mode DB during an active transaction captures an inconsistent state.
- The `-wal` and `-shm` files contain uncommitted-to-main pages; backing up only the main file loses recent data.
- Reverse: backing up main+wal but restoring on a different machine without proper checkpoint can leave the WAL referencing pages that no longer exist.

**How to avoid:**
- **Always** use SQLite's online backup API (`rusqlite::backup::Backup`) or `VACUUM INTO 'backup.db'` — they handle WAL contents and locking correctly.
- Before backup: `PRAGMA wal_checkpoint(TRUNCATE)` to fold WAL into main; then backup the single file. Acceptable for nightly backups during low-activity windows.
- After backup: verify by opening the backup file read-only and running `PRAGMA integrity_check` — if it returns anything other than `ok`, mark the backup as bad and keep the prior one.
- Restore procedure: app refuses to start; user picks a backup file; app deletes `trackly.db`, `trackly.db-wal`, `trackly.db-shm`; copies backup over; reopens.
- Retention: keep N daily + M weekly + K monthly; delete oldest with a clear log line each time.
- Test restore quarterly in CI (script that creates DB, backs up, corrupts main, restores, verifies row counts).

**Warning signs:**
- Backup file size identical day-over-day → WAL not being checkpointed.
- Backup file size = 0 or < 4KB → backup failed silently.
- `integrity_check` after restore returns non-`ok`.
- Restore on a different machine fails to open.

**Phase to address:**
Phase "Настройки — Бэкап" (backup feature itself). However, **schema versioning** (`PRAGMA user_version`) must be in Phase 1 so restore knows what schema the backup is from.

---

### Pitfall 11: HTTPS in server mode — self-signed cert UX trap

**What goes wrong:**
Server mode launches with a self-signed cert. Specialists open `https://192.168.1.42:8443/` — browser shows a giant red "NOT SECURE" warning. They click around it once, but every browser update re-prompts. Users start using `http://` (and Trackly happily accepts it), passwords go in clear over the wire.

**Why it happens:**
- Self-signed certs trigger browser security warnings; UX is awful but not bypassable cleanly.
- If both HTTP and HTTPS listen, users default to HTTP and the team forgets to disable it.
- Cert pinning / corp CA trust is a real operational burden the user shouldn't have to solve.

**How to avoid:**
- **No HTTP listener at all** in server mode. HTTPS only, port configurable (default 8443).
- Generate a self-signed cert on first server-mode toggle, store `cert.pem` + `key.pem` next to the DB. Include the device's hostname and all detected LAN IPs in the SAN field — minimizes warnings.
- In Settings, show a "Доверенный сертификат" section with instructions to install the generated cert into Windows trusted root store on each client machine — provide a one-line `certutil -addstore` command and a `.cer` download.
- Allow users to provide their own corp CA-signed cert (path to PEM in Settings).
- Detect HSTS-eligible hostname (mDNS `.local`) and prefer it over IP — fewer cert SAN problems.
- Document that for serious deployments, the user should put Trackly behind their own reverse proxy with their corp cert (out of scope to automate this).

**Warning signs:**
- Users complain about browser warnings on every load.
- Passwords visible in Wireshark capture from a LAN test.
- Settings allow toggling HTTPS off → remove the option.

**Phase to address:**
Phase "Сервер-режим" (when HTTP server is introduced). Cert generation and the no-HTTP rule must be in the initial server-mode work.

---

### Pitfall 12: Cross-compilation macOS → Windows — works locally, breaks in CI

**What goes wrong:**
Dev builds Windows release locally via `cargo-xwin`, ships, users say "smartscreen warning" or "ring/openssl link error" or "missing VC++ runtime". Team adds GitHub Actions Windows runner, builds work; signing doesn't; updater fails because the bundle isn't signed.

**Why it happens:**
- Tauri's signtool integration assumes Windows host; cross-compile from macOS requires `osslsigncode` and a custom sign command in `tauri.conf.json`.
- `ring` crate has historically been problematic to cross-compile (recent versions are better but pin to a known-good).
- `openssl` crate links to system libs — prefer `rustls` everywhere, with `aws-lc-rs` provider.
- Windows 7 32-bit support effectively died: recent Rust releases (1.78+) deprecated `i686-win7-windows-msvc` in some channels; ring/sqlcipher/etc may have minimum Win10 requirements.
- SmartScreen ("unverified publisher") warning persists even with EV cert until enough downloads accumulate.

**How to avoid:**
- Use GitHub Actions `windows-latest` runner for the canonical build; treat macOS local Windows builds as developer convenience only.
- Replace `openssl` with `rustls` + `aws-lc-rs` everywhere (`reqwest` features, `ldap3` features, `sqlx` features).
- Pin all crypto crate versions in workspace `Cargo.toml`.
- Code-sign with a real (cheap OV) certificate from day 1 — SmartScreen reputation builds with signed downloads over time.
- For Win7 32-bit: declare it best-effort, run a single CI job that just attempts the build; do not block release on its success.
- Bundle VC++ runtime via NSIS (Tauri's NSIS template can include it) or document the prerequisite clearly.

**Warning signs:**
- Build green in CI but the resulting `.exe` doesn't start on a fresh Windows VM ("VCRUNTIME140.dll missing").
- `windows-rs` linker errors that don't appear on `windows-latest` runner.
- Updater fails verification on signed builds → sign command wrong, signature stripped.

**Phase to address:**
Phase "Инфраструктура / выпуск" (CI/CD). Set up the canonical Windows runner build early; cross-compile from macOS is for dev iteration only.

---

### Pitfall 13: SNMPv2c community string in plaintext + UDP unreliability

**What goes wrong:**
The Settings UI has a field "SNMP Community: public". User saves. The string lands in the SQLite DB unencrypted, in logs when SNMP requests are traced, and is sent over UDP in clear text. A bad actor on the LAN sniffs it and can now query every printer.

**Why it happens:**
- SNMPv2c is unauthenticated and unencrypted by protocol design; "community string" is a misnomer for "weak shared secret".
- Most teams default to `public` and never change it.
- UDP packet loss + variable printer response times cause inconsistent monitoring data; users report "the printer randomly goes offline" when it didn't.

**How to avoid:**
- **Encrypt at rest** the community string in DB using a key derived from a config file (the file lives next to the DB, so portability is preserved). Not security against local-FS access (the user has the DB file), but prevents accidental log leakage and protects against backup theft.
- **Never log** the community string. Use a `Secret<String>` newtype with custom `Debug` impl that prints `***`. Apply to AD passwords too.
- **SNMP retry strategy:** initial timeout 2s, retry 3x with exponential backoff, mark printer as `degraded` (not `offline`) after first failure, `offline` only after 3 consecutive polling cycles fail (so 6+ minutes at 60s poll). Avoids flapping.
- **Connection pooling** — one socket per printer, reuse across polls; avoid socket churn at 60s × 100 printers.
- Future-proof Settings UI for SNMPv3 (auth+priv) — users may not enable it in v1, but make the schema accommodate it.
- Document in Settings: "Используйте уникальную community string на ваших принтерах вместо `public`."

**Warning signs:**
- `grep community .planning/.../trackly.db` succeeds (it's there in plaintext).
- Logs contain the community string in error messages.
- Printer "goes offline" several times a day without anyone touching it → tighten retry logic.

**Phase to address:**
Phase "Принтеры" (SNMP monitoring). Secret-handling pattern set in Phase 1 (auth scaffolding) and reused here.

---

### Pitfall 14: User-editable document templates → injection / silently broken upgrades

**What goes wrong:**
Admin edits the Акт template in the UI. They paste in `{{ system('rm -rf /') }}` or similar — if using a powerful engine, it executes. Or, less malicious: a future release adds the field `act.warehouse_returner_phone` to default templates; admins who customized their templates 6 months ago still don't see this field because their template overrides the default.

**Why it happens:**
- Tera, Handlebars, and similar Jinja-derived engines have escape mechanisms but historically have had sandbox escapes via reflection or filter abuse.
- "Just embed Tera" with full power = remote code execution from the admin role (which is less bad than from an anonymous user, but still wrong defense in depth).
- Template versioning is usually not designed in v1 — users get stuck on old versions.

**How to avoid:**
- Use **Tera in safe mode**: register only explicit filters/functions you control; do not expose any I/O or `get_env`-style functions.
- Or use **MiniJinja** (`minijinja` crate) with `set_undefined_behavior(UndefinedBehavior::Strict)` and a custom syntax set — it's leaner than Tera and explicitly designed for end-user editing scenarios.
- Or skip templating entirely and use **Typst** with parameter passing — Typst is a typesetting language but its scripting is bounded by design.
- Per-template version: store the engine version and template schema version in DB; on app upgrade, if user has customized a template, show a 3-way merge dialog ("default changed, your version, merged result"). At minimum, warn "your template hasn't been updated since v1.2, current is v2.0 — review here".
- Run template render in a separate thread with a 5s timeout; kill if it loops.
- Provide a "Reset to default" button in the template editor (per template).

**Warning signs:**
- Template engine call without a timeout.
- Template rendering can call `read_file` / `include` from arbitrary paths.
- No version field on stored templates → no upgrade path.

**Phase to address:**
Phase "Документы и шаблоны" / wherever the first template is editable (likely "Акты"). Set the safe-template pattern early.

---

### Pitfall 15: Time zones — Russia's quirky history breaks dates in reports

**What goes wrong:**
Reports for "March 2026" show acts created at `2026-03-31 23:30 +03:00` correctly. Historical reports for "October 2014" mis-attribute events to the wrong month because the codebase stored `chrono::Local` timestamps and the local TZ database has stale rules.

**Why it happens:**
- Russia abolished DST on 26 October 2014. Russia changed nominal offsets in 2011 (permanent DST → UTC+4 in Moscow) and 2014 (back to UTC+3, no DST).
- Some regions (Samara, Udmurtia, Magadan, Kamchatka, Chukotka, Zabaykalsky, Kemerovo) changed offsets independently in 2014.
- Storing local timestamps + assuming "system TZ" makes reports break when the DB is moved between machines in different regions.
- Server mode: the desktop and the specialist browser may be in different TZs; "today" means different things.

**How to avoid:**
- **Store all timestamps in UTC** in DB (`DATETIME NOT NULL` storing ISO 8601 UTC, or INTEGER unix seconds). Never store local time.
- Store the organization's primary TZ in Settings (`Europe/Moscow` by default). Use `chrono-tz` crate (vendored IANA database — keep updated via `cargo update` quarterly).
- All formatting for UI happens in the organization's TZ, not the OS's TZ.
- Reports filter by the organization's TZ boundaries: "March 2026" = `2026-03-01T00:00:00 Europe/Moscow` to `2026-04-01T00:00:00 Europe/Moscow`, converted to UTC for the SQL `WHERE`.
- Document the convention in CONTRIBUTING.md: any `chrono::Local` or `SystemTime::now()` without explicit TZ handling is a code review block.
- Russian DST quirks: `chrono-tz` handles them correctly **if** the tzdata it was compiled against is recent. Pin a version, audit on each release.

**Warning signs:**
- `chrono::Local::now()` anywhere in the codebase.
- Report counts off by 1 across midnight/month boundaries depending on which machine ran them.
- Historical dates before 2014-10-26 displaying with the wrong offset.

**Phase to address:**
Phase 1 (data layer). TZ discipline established at the schema level — costly to retrofit.

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Use `chrono::Local` for new timestamps because "it works in dev" | -1 day setup | Reports broken across TZ boundaries; entire DB needs rewrite | **Never** |
| Skip `BEGIN IMMEDIATE`, hope for the best with WAL | Less code | Random "database is locked" under any concurrent load | Single-user pure-desktop mode only — not Trackly |
| Store SNMP community string / AD bind password in plaintext for v1 | -1 hour | Logged in tracebacks, leaks via backup theft | Until first beta tester runs Wireshark — i.e., never to production |
| File-copy DB for backup instead of online backup API | -2 hours | Backups silently corrupt; restore is unreliable | Only with explicit `wal_checkpoint(TRUNCATE)` + locking; even then, use the API |
| UI-only role checks now, backend checks "later" | Faster UI iteration | Whole-system audit needed; any browser-accessible endpoint is open | Only for genuinely read-only public endpoints (e.g., a status page) — there are none in Trackly |
| Default `public` SNMPv2c community string with no warning | Easier discovery | LAN compromise → printer config tampering | Acceptable as default if Settings UI warns + supports change; never silent |
| Single tokio runtime for both Tauri and axum (default Tauri behavior) | Simpler code | Long-running handler can block UI thread responsiveness | **Always do this** — Tauri's runtime is fine; just don't block-on inside command handlers |
| Bundle "system" font for PDF, expecting Cyrillic to "just work" | -1 hour | Glyphs missing in production | **Never** for v1 — always embed |
| Single SQLite pool with `max_connections > 1` for writes | Simpler config | Writer/writer contention, SQLITE_BUSY | Never — split read pool (N) and write pool (1) |
| Hardcode AD bind URL to one DC | -30 minutes | Outage when that DC reboots | Acceptable only as initial fallback if SRV-record discovery fails |

---

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| **Tauri ↔ axum** | Spawning separate tokio runtimes (`tokio::runtime::Runtime::new()` inside Tauri) | Use `tauri::async_runtime::spawn` (single runtime shared); axum app via `axum::serve(listener, app)` inside the spawn |
| **Tauri webview ↔ axum (same machine)** | Trying to fetch from `https://localhost:8443` in the webview but blocking via Tauri's default CSP | Either use Tauri commands from webview (no HTTP at all on the desktop side), or configure CSP `connect-src` explicitly to allow localhost HTTPS |
| **axum ↔ browser over LAN** | Listening on `127.0.0.1` and being puzzled why LAN clients can't connect | Bind `0.0.0.0:8443` (or specific LAN IP); be aware Windows Firewall will prompt — pre-configure via WiX/NSIS install step if possible |
| **ldap3 ↔ AD** | Plain `ldap://` to a hardened AD that requires sealing | Always `ldaps://636` with proper CA trust; or StartTLS; verify `tls-server-end-point` channel binding for AD with strict mode |
| **SNMP ↔ Pantum** | Trusting `hrPrinterStatus` to indicate spooler hang | Cross-reference with `prtMarkerLifeCount` (page counter) over time + WMI on print host |
| **rusqlite ↔ Tauri** | Holding a `Connection` in a Tauri command's local scope → opening/closing per call | Use `r2d2_sqlite` pool stored in `tauri::State`; one writer + N readers |
| **WebView2 ↔ portable folder** | Defaulting cache to `LOCALAPPDATA\<app>\EBWebView` | Set `WEBVIEW2_USER_DATA_FOLDER` env var before Tauri builder |
| **Templates (Tera/MiniJinja) ↔ user input** | Admin renders user-supplied template with full engine power | Sandbox: pre-registered filters only, no I/O functions, render in separate thread with timeout |
| **CSV ↔ Russian Excel** | Plain UTF-8 without BOM + comma delimiter | UTF-8 with BOM + `;` delimiter for export; encoding/delimiter sniffing on import |
| **Cross-compile ↔ Tauri signtool** | Building from macOS, expecting code signing to "just work" | Configure custom sign command using `osslsigncode`, or sign only on Windows-host CI |

---

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| N+1 query on devices→cartridges→acts in dashboard | Dashboard loads in 2s, then 5s, then 15s as data grows | Eager-load with joins or batched IN-queries; profile every `Vec<T>` returned to the UI | ~500+ devices, ~2000+ acts |
| WAL file unbounded growth | `.db-wal` reaches 1+ GB; reads slow as WAL grows | `PRAGMA wal_autocheckpoint=1000`; periodic `wal_checkpoint(TRUNCATE)` during idle | 24+ hours of constant writes without idle window |
| SNMP polling all printers in serial | Polling takes longer than the poll interval | Parallel polling with a bounded semaphore (e.g., 10 concurrent); per-printer timeout 2s | 20+ printers at 60s interval |
| Bulk CSV import in one transaction (10k rows) | UI frozen 30s; SQLite locked for everyone | Chunk into transactions of 500 rows each; show progress; allow cancel | 5000+ row imports |
| PDF render on UI thread | UI freezes 2-5s when admin clicks "Print Act" | Spawn render onto a worker thread, stream result; debounce repeated clicks | Documents >5 pages or with embedded images |
| Full-text search via `LIKE '%foo%'` on devices.name | Slows linearly with table; 800ms at 5000 rows | Use SQLite FTS5 virtual table; reindex on write via triggers | 2000+ devices |
| Every Tauri command opens a fresh DB connection | High file-descriptor pressure; intermittent lock errors | Connection pool in `tauri::State`; share across commands | 5+ concurrent users in server mode |
| Loading entire act history into memory for reports | App memory grows to 500+ MB | Server-side pagination + cursor; render virtualized lists in Svelte | 10k+ historical records |

---

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| Storing AD password (even hashed) for any purpose | Catastrophic if DB stolen — AD account compromise; violates PROJECT.md explicit rule | Bind-and-discard; `Secret<String>` newtype with `Drop` zeroing; never persist |
| Local user passwords with weak hash (MD5/SHA1/bcrypt cost 4) | Offline cracking trivial | argon2id with sensible cost (`argon2 = { version = "0.5", features = ["std"] }`, m_cost=19456, t_cost=2, p=1 — OWASP recommended) |
| Session token in URL query string for browser server mode | Logged in browser history, proxies, logs | HttpOnly + Secure cookies; CSRF token in header for mutating ops |
| Webview can call privileged commands from server mode | If admin opens a malicious URL in webview, that page can call admin commands | Disable webview navigation to non-app origins; use Tauri's `dangerousDisableAssetCspModification: false`; lock CSP |
| Open ports without confirmation | Server mode binds 8443 without asking; Windows Firewall prompt confuses admin who clicks "Cancel" | UX: explicit "Запустить сервер" toggle, show "Windows может запросить разрешение"; provide a tested NSIS step that adds firewall rule |
| Logging full request body on errors | AD passwords, session tokens, community strings in logs | Structured logging with explicit allowlist of fields; `Secret<T>` for all sensitive |
| Self-signed cert accepted with `danger_accept_invalid_certs` in Rust HTTP client | TLS protection bypassed | Trust the user's own CA via Settings; never accept invalid certs |
| Path traversal in template-storage / backup-folder Settings | User sets `../../../Windows/System32/foo` as backup path → app writes there | Canonicalize all user-provided paths; reject if outside the app's working tree (or document and explicitly allow with a confirmation) |
| Storing IP camera / printer credentials (SNMPv3, future) in clear in DB | LAN credential exposure on DB theft | Same `Secret<String>` discipline + DB-at-rest encryption (or at least field-level XOR keyed from a file beside DB) |
| Trust Tauri capability allowlist as the only authz | Server-mode endpoints bypass capabilities entirely | Backend `authorize()` function called from every command **and** every axum handler |

---

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| Showing browser cert warning on every launch in server mode | Specialists train themselves to ignore security warnings | Generate cert with hostname + LAN IPs; provide one-click "install cert" instructions; document mDNS `.local` access |
| "Database is locked" red toast with no recovery | User panics, loses confidence | Auto-retry with exponential backoff (1-5 attempts), show subtle spinner; only surface error after all retries fail; preserve form state |
| Auto-suggesting act № that's already taken because race lost | User submits, sees "номер занят", re-edits, repeats | After loss, automatically advance the suggestion and preserve all other fields; never make user re-enter |
| Sequential cartridge codes that show gaps confusingly | "Where's C-000042?" support questions | Document in UI that gaps are normal (small "?" tooltip); never expose internal numbering as user-visible identity if the user shouldn't see gaps |
| CSV import errors line-by-line in a small modal | User can't fix 50 errors without exporting them | Show errors in a side panel with line numbers, allow filter/export of failed rows; provide "skip and continue" + "fix and retry" |
| Cyrillic name fields with min-width too small (UI clipping) | Truncated names in tables | Test all UI components with `Сидоров-Петроградский-Долгорукий` and `№` characters; use `text-overflow: ellipsis` with full-text tooltip |
| Switching `Tauri Local` ↔ `Server Mode` requires restart with confusing prompt | Users guess wrong, lose unsaved work | "Save first?" prompt; preserve open documents across mode switch if possible; clearly indicate mode in title bar |
| Sidebar/dashboard counts that don't match list views | Trust erosion | Both queries hit the same query layer; counters update from the same source as the lists; never compute separately |
| PDF preview that opens in system default app | User has no PDF reader installed, or it opens slowly | Embed a PDF viewer (e.g., PDF.js in Tauri webview); fall back to "Open in system app" button |
| "Update detected, restart now?" with no "later" option | Interrupts work | Defer to next launch; show non-intrusive badge in title bar; never auto-restart |

---

## "Looks Done But Isn't" Checklist

- [ ] **Portable mode:** Run Process Monitor while exercising every screen — verify zero writes outside `<exe_dir>`. Specifically check `WEBVIEW2_USER_DATA_FOLDER` is set before Tauri builder.
- [ ] **AD login:** Test against a real Windows Server 2022 with LDAP signing and channel binding enabled (post-2023 hardening). Mock-only AD tests pass everything.
- [ ] **Pantum monitoring:** Test against the actual BM5100ADN with the spooler hang scenario reproduced (paused queue + responsive SNMP). Verify alert fires.
- [ ] **SQLite concurrency:** Run a load test with 20 simulated clients doing reads + writes concurrently. Verify zero `SQLITE_BUSY` errors reach the UI.
- [ ] **Backup/restore:** Restore from a backup taken yesterday, verify `PRAGMA integrity_check = ok`, verify row counts match expectations.
- [ ] **PDF Cyrillic:** Render an Act with `Сидоров-Петроградский Иван Александрович (ё)`, open in Adobe Reader, Foxit, and Chrome — no missing glyphs.
- [ ] **CSV roundtrip:** Export → open in Russian Excel → save → re-import → verify no data corruption, no encoding loss.
- [ ] **Cyrillic paths:** Install Trackly to `C:\Документы\Учёт\` — verify everything works.
- [ ] **Role enforcement:** Run a curl-based test that hits every mutating endpoint as Specialist and Employee — verify 403.
- [ ] **HTTPS only:** Verify server mode refuses to start an HTTP listener; verify cert has all LAN IPs in SAN.
- [ ] **Time zones:** Generate a report spanning the 2014-10-26 boundary, verify correct date attribution for historical entries.
- [ ] **No secrets in logs:** `grep -ri "password\|community\|secret" logs/` returns nothing sensitive.
- [ ] **Cross-platform build:** Fresh `windows-latest` VM runs the installer, app launches, no missing DLLs.
- [ ] **Template safety:** Try injecting `{{ self }}`, `{% for x in range(99999999) %}`, file-read functions — none should succeed.
- [ ] **WAL hygiene:** After 24h of normal use, `-wal` file is < 4 MB (autocheckpoint working).
- [ ] **Sequential IDs:** Two browser tabs creating acts simultaneously — both get unique numbers, no constraint violation surfaced to user.
- [ ] **Auto-suggest dropdowns:** With 5000+ devices, autocomplete is still fast (<100ms); FTS index in place.
- [ ] **Dashboard counters match list views:** Same numbers shown in widget and on the section page.
- [ ] **Firewall prompt:** First server-mode start on a clean Windows shows the firewall dialog and instructions are visible to the user.
- [ ] **Database on network share:** Setting attempts to point DB to `\\server\share\trackly.db` is refused with a clear error.

---

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Portable mode writes to AppData discovered post-release | MEDIUM | Ship patch redirecting paths; on first launch of patched version, detect AppData remnants and prompt to migrate; document manual cleanup in release notes |
| SQLite database corruption | HIGH | 1) Stop app. 2) Try `sqlite3 trackly.db .recover > recover.sql` then import to fresh DB. 3) If recovery partial, restore latest backup. 4) Reconcile gap manually from CSV exports if any. |
| Backups all corrupt (file-copy strategy) | CRITICAL | Roll back to last known-good. Build forensic SQL `.recover` from corrupted file. Communicate data loss window to user. Implement online backup API immediately. |
| Pantum monitoring false positives flooding alerts | LOW | Tune retry/threshold in Settings; add per-printer suppression; ship update with smarter cross-check logic |
| AD bind locked out service account | LOW-MEDIUM | Document the bind throttling; ask user's AD admin to unlock; switch app to UPN bind to avoid name confusion |
| PDF glyphs missing in shipped templates | LOW | Hot-patch the bundled font; users update; existing PDFs that were saved with missing glyphs cannot be re-rendered automatically |
| CSV import corrupts data (encoding mis-detected, no preview) | MEDIUM | Implement preview step; for already-imported bad data, restore from last backup taken before import; provide an "undo last import" feature (`import_id` foreign key on imported rows) |
| Sequential ID duplicates in production DB | MEDIUM | Add unique index now; resolve duplicates manually (rename with suffix or assign new numbers); communicate to user |
| Secret leak in logs (community string, password) | HIGH | Rotate the exposed secret; audit log retention; ship patch with `Secret<T>` discipline; communicate to affected users |
| Server mode exposed without HTTPS by misconfig | HIGH | Force HTTPS-only in next release; communicate; suggest password rotation for users who logged in over LAN |
| Time zone bug discovered in historical reports | MEDIUM | One-time migration script converting `Local` timestamps to UTC based on stored TZ hint (or organization TZ default); document the cutover |

---

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| 1. Portable mode leaks | Phase 1 (foundation) | ProcMon test in CI; clippy disallowed-methods rule |
| 2. SQLite "database is locked" | Phase 1 (foundation) | Load test with 20 concurrent clients in CI |
| 3. Pantum spooler hang detection | Phase "Принтеры" (prototype/spike before design) | Manual reproduction on a real BM5100ADN; alert fires |
| 4. AD bind issues | Phase "Пользователи — AD" (late); core `Secret<T>` pattern in Phase 1 | Integration test against real WS2022 |
| 5. UI-only role checks | Phase "Пользователи" (auth/roles) | curl-based matrix test, role × endpoint |
| 6. Cyrillic paths | Phase 1 (foundation) | CI Windows job tests with `C:\Документы\Учёт\` |
| 7. PDF Cyrillic | Phase "Акты приёма-передачи" (first PDF) | Render fixture PDF, hash check; visual inspection in 3 readers |
| 8. CSV encoding | Phase "Устройства" (first import/export) | Roundtrip test with Russian Excel fixture |
| 9. Sequential ID race | Phase "Акты приёма-передачи" | Concurrent submit test; unique index in schema |
| 10. Backup correctness | Phase "Настройки — Бэкап"; schema versioning in Phase 1 | Quarterly CI restore test; integrity_check on every backup |
| 11. HTTPS UX | Phase "Сервер-режим" | No HTTP listener (assertion in startup); cert SAN includes LAN IPs |
| 12. Cross-compile | Phase "Инфраструктура / выпуск" | windows-latest CI runner; fresh-VM smoke test |
| 13. SNMP secret + UDP retry | Phase "Принтеры"; secret pattern in Phase 1 | Wireshark check; `grep` DB for community string |
| 14. Template injection | Phase "Документы и шаблоны" | Injection-attempt test fixture; render timeout enforced |
| 15. Time zones | Phase 1 (foundation) | `chrono::Local` clippy lint; 2014-boundary report test |

---

## Sources

- [Tauri 2 — File System Plugin](https://v2.tauri.app/plugin/file-system/) — webview folder denied by default, Windows
- [Tauri Discussion #8029 — How to clean webview cache?](https://github.com/orgs/tauri-apps/discussions/8029) — WEBVIEW2_USER_DATA_FOLDER env var pattern
- [Tauri Discussion #5557 — Storing application data](https://github.com/tauri-apps/tauri/discussions/5557) — AppData defaults
- [Tauri Issue #7491 — EBWebView created in AppData](https://github.com/tauri-apps/tauri/issues/7491) — confirms WebView2 writes to AppData even for "portable"
- [Tauri — Capabilities](https://v2.tauri.app/security/capabilities/) — explicit "does not protect against backend code", "command exposure has no inherent security impact if backend doesn't validate"
- [Tauri — Permissions](https://v2.tauri.app/security/permissions/)
- [Tauri Discussion #2751 — http server in tauri app](https://github.com/tauri-apps/tauri/discussions/2751) — axum embedding patterns
- [Tauri Issue #12331 — Self-signed cert hosting](https://github.com/tauri-apps/tauri/issues/12331)
- [Tauri v1 — Cross-Platform Compilation](https://v1.tauri.app/v1/guides/building/cross-platform/)
- [Ship Your Tauri v2 App Like a Pro — Code Signing](https://dev.to/tomtomdu73/ship-your-tauri-v2-app-like-a-pro-code-signing-for-macos-and-windows-part-12-3o9n)
- [SQLite — WAL documentation](https://sqlite.org/wal.html) — single writer rule, network FS warning
- [SQLite Forum — Hot backup database in WAL mode by copying](https://sqlite.org/forum/forumpost/2ea989bbe9)
- [SQLite — Autoincrement](https://sqlite.org/autoinc.html) — gaps and uniqueness semantics
- [Bert Hubert — SQLITE_BUSY errors despite timeout](https://berthub.eu/articles/posts/a-brief-post-on-sqlite3-database-locked-despite-timeout/)
- [Ten Thousand Meters — SQLite concurrent writes](https://tenthousandmeters.com/blog/sqlite-concurrent-writes-and-database-is-locked-errors/) — BEGIN IMMEDIATE rationale
- [PhotoStructure — How to VACUUM SQLite in WAL Mode](https://photostructure.com/coding/how-to-vacuum-sqlite/)
- [ldap3 crate docs](https://docs.rs/ldap3/latest/ldap3/) — TLS, channel binding, settings
- [ldap3 LdapConnSettings](https://docs.rs/ldap3/latest/ldap3/struct.LdapConnSettings.html) — timeout and TLS verify options
- [RFC 3805 — Printer MIB v2](https://datatracker.ietf.org/doc/html/rfc3805)
- [MPS Monitor — Printer Monitoring with SNMP MIB](https://www.mpsmonitor.com/snmp-mib/) — vendor proprietary MIBs reality
- [MIB Browser — Printer-MIB OID list](https://mibbrowser.online/mibdb_search.php?mib=Printer-MIB)
- [Sysadminwork — Monitoring printers via SNMP (Kyocera/HP)](https://sysadminwork.com/monitoring-printers-hp-kyocera-brother-via-snmp-with-zabbix/)
- [Pantum BM5100 Series User Manual](https://www.manualslib.com/manual/2115030/Pantum-Bm5100-Series.html)
- [Pantum Support Portal](https://support.pantum.com/index/productcenter/problem.html)
- [Microsoft Learn — Troubleshooting printing scenarios](https://learn.microsoft.com/en-us/troubleshoot/windows-server/printing/troubleshoot-printing-scenarios) — confirms spooler-side hang pattern
- [Microsoft Support — Fix print spooler service](https://support.microsoft.com/en-us/windows/fix-print-spooler-service-not-running-errors-in-windows-bb0de80a-8c4a-4938-a36a-f89a859113f0)
- [Svelte 5 migration guide](https://svelte.dev/docs/svelte/v5-migration-guide)
- [Svelte $effect docs](https://svelte.dev/docs/svelte/$effect)
- [genpdf — fonts module](https://docs.rs/genpdf/latest/genpdf/fonts/index.html) — Windows-1252 limitation
- [printpdf GitHub](https://github.com/fschutt/printpdf)
- [Redmine #7037 — CSV export with UTF-8 + BOM for Excel](https://www.redmine.org/issues/7037)
- [Rust Forum — Path, OsStr and non-UTF-8 paths](https://users.rust-lang.org/t/path-osstr-and-supporting-non-utf-8-paths-inputs/64826)
- [Rust Issue #56171 — OsStr need not be valid UTF-8 on Windows](https://github.com/rust-lang/rust/issues/56171)
- [OWASP — Testing for SSTI](https://owasp.org/www-project-web-security-testing-guide/v41/4-Web_Application_Security_Testing/07-Input_Validation_Testing/18-Testing_for_Server_Side_Template_Injection)
- [Time in Russia — Wikipedia](https://en.wikipedia.org/wiki/Time_in_Russia) — 2011/2014 DST and offset history
- [Moscow Time — Wikipedia](https://en.wikipedia.org/wiki/Moscow_Time)
- [Broadcom KB — SNMP Timeouts explained](https://knowledge.broadcom.com/external/article/307285/understanding-simple-network-management.html)
- Personal experience: portable Windows desktop apps deployed in mixed-locale environments; Tauri 2 production releases 2024-2026.

---
*Pitfalls research for: Trackly — Tauri + Svelte + Rust + SQLite portable desktop with LAN server, AD bind, multi-vendor SNMP printer monitoring*
*Researched: 2026-05-24*
