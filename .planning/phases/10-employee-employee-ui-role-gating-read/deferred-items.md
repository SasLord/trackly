# Deferred Items — Phase 10

Issues discovered during execution that are out of scope for the current plan
(pre-existing, unrelated to the files/behavior being changed). Logged per the
deviation-rules scope boundary — not fixed here.

## From Plan 10-02 execution

- **`crates/trackly-app/src/services/template_service.rs:379,430`** — clippy
  `len_zero` warning (`bytes.len() > 0` should be `!bytes.is_empty()`). Pre-existing,
  unrelated to the ReadData gating work in 10-02. Two occurrences.
- **`crates/trackly-app/tests/backup_service.rs:168`** — clippy `disallowed_methods`
  error (`std::fs::copy` used in a test; project lint requires `rusqlite::backup::Backup`
  for DB backups, but the lint also fires on this test's incidental file copy).
  Pre-existing, unrelated to 10-02. This causes `cargo clippy -p trackly-app --tests`
  to fail at the workspace level even though all 10-02-relevant files (and the
  `role_endpoint_matrix` test specifically) are clippy-clean in isolation.
- **`crates/trackly-app/tests/ws_upgrade_serve_connection.rs`** — minor `cargo fmt`
  formatting drift (line-wrapping nits), pre-existing from an earlier phase, unrelated
  to 10-02's read-gating changes. Left untouched to avoid scope creep.
