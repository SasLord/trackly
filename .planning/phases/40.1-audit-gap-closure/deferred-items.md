# Deferred items — Phase 40.1 (audit-gap-closure)

## `services::report_service::tests::export_pdf_*` — parallel-test race with `pdf::html_templates::tests`'s `TRACKLY_TEMPLATES_DIR` env override

**Found during:** Plan 40.1-02, Task 3 (user-requested deviations — «Тип» column
omission + filter-summary line), running a narrowed multi-module `cargo test --lib`
filter (`reports:: report_service:: template_service:: html_templates::`) together
in one process.

**Out of scope** — `pdf::html_templates.rs`'s `ENV_GUARD: Mutex<()>` (pre-existing,
not introduced by this plan) only serializes tests *within*
`pdf::html_templates::tests`. It does not protect `services::report_service::tests`
from a concurrently-running `html_templates::tests` test that calls
`set_templates_dir_env("...")` — `std::env` is process-global, Rust test threads run
in parallel by default, and `report_service::tests::make_test_service` resolves its
templates dir via `Paths::resolve_for_exe_dir`, which reads the very same
`TRACKLY_TEMPLATES_DIR` env var through `resolve_templates_dir`. When the two
modules' tests interleave, `report_service`'s `export_pdf` tests transiently read
`_header.html` from an unrelated `html_templates::tests` tempdir (already dropped or
mid-write), producing a `Template parse error: ... unexpected end of input ...
(in _header.html:1)`.

**Symptom:** `export_pdf_renders_filter_summary_when_present`,
`export_pdf_omits_filter_summary_when_absent`,
`export_pdf_non_empty_report_renders_month_groups_and_rows`,
`export_pdf_renders_org_header_name`, `export_pdf_empty_report_renders_no_data_message`
(5 of the module's `export_pdf_*` tests) intermittently fail with the above template
parse error when run in the same test binary as `pdf::html_templates::tests`.

**Confirmed NOT a regression from this plan's code:** `cargo test -p trackly-app --lib
-- services::report_service:: --test-threads=1` passes all 51 tests reliably every
time; the full default invocation (`cargo test -p trackly-app -- --skip
login_remember_persistent_cookie`, the project's standard full-package gate) also
passed cleanly (0 failures) in this same session. The race only manifests under a
specific narrowed multi-module filter combined with default test-thread parallelism —
this plan added 2 more tests using the pre-existing `make_test_service` pattern (already
used by every sibling test in the module before this plan), which does not participate
in and was never protected by `html_templates::tests`'s `ENV_GUARD`.

**At fix time:** either (a) have `report_service::tests`' `make_test_service` set
`TRACKLY_TEMPLATES_DIR` explicitly to its own tempdir (removing ambiguity regardless of
what other tests do to the env var), or (b) share one process-wide env-var mutex between
the two test modules. Do not touch this under Plan 40.1-02's scope — the fix belongs to
whichever plan next touches template-resolution test infrastructure.
