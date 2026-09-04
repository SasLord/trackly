# Deferred items — Phase 40 (movement-history)

## `users_update_password_change` / `delete_then_recreate_revives_same_login` — intermittent 30s test-budget timeout

**Found during:** Plan 40-33, Task 3, full-package verification run
(`TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app -- --skip
login_remember_persistent_cookie`).

**Out of scope** — `crates/trackly-app/tests/users_crud.rs` is not touched by Plan 40-33
(cartridges/refill-default work only). Not auto-fixed per the executor's scope boundary.

**Symptom:** In two separate full-package runs, one or both of
`delete_then_recreate_revives_same_login` and `users_update_password_change` panicked with
`test exceeded 30s budget: Elapsed(())`. In isolation (`cargo test -p trackly-app --test
users_crud -- delete_then_recreate_revives_same_login users_update_password_change`) both tests
pass reliably in ~16s total.

**Likely cause:** argon2id hashing (`m=19456 KiB, t=2, p=1`) is CPU/memory-hard; when the full
104-file `trackly-app` test package runs back-to-back under machine load (multiple prior test
binaries still settling, disk I/O from the writer worker), the 30s per-test budget in these two
tests is tight enough to occasionally miss. Not a correctness bug — a resource-contention flake
in the test harness's own timeout, unrelated to argon2's actual production behavior.

**Recommendation:** Either raise the 30s budget for these two specific tests (they perform a
full argon2id hash + DB round-trip) or leave as a documented flake — do not touch
`users_crud.rs` under Plan 40-33's scope.
