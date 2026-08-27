# Deferred Items — quick 260827-ui3

## Task 1 verification (full `trackly-app` package run)

- **`users_crud.rs::users_update_password_change` flaky under full-suite parallel load.**
  Hit its internal 30s `tokio::time::timeout` budget twice when run as part of the full
  `cargo test -p trackly-app -- --skip login_remember_persistent_cookie` sweep (argon2
  hashing is CPU-heavy; contention with ~90 other test binaries running concurrently pushes
  it over budget). Passes standalone in ~13.6s
  (`cargo test -p trackly-app --test users_crud users_update_password_change` → `ok`).
  Not touched by this quick task — this task's files are `crates/trackly-infra/src/config.rs`,
  `crates/trackly-app/src/dto/auth.rs`, `crates/trackly-app/src/http/auth.rs`,
  `crates/trackly-app/src/tauri_cmds/auth.rs`, and frontend `place_path_display` wiring; none
  touch `users_crud.rs`, password hashing, or auth login timing. Out of scope per executor
  scope-boundary rule (pre-existing failure in an unrelated file, not caused by this task's
  changes). Not fixed — logged here instead.
