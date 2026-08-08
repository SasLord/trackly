---
phase: 34
slug: document-header
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-08-08
---

# Phase 34 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Derived from `34-RESEARCH.md` § Validation Architecture (line 891+).

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (workspace). No separate test config — integration targets live in `crates/trackly-app/tests/*.rs`, unit tests inline in `src/pdf/*.rs`. |
| **Config file** | none — workspace `Cargo.toml` |
| **Quick run command** | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --lib pdf:: -- --test-threads=1` |
| **Full suite command** | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app -- --test-threads=1` (requires a real `pnpm --dir ui build` output in `ui/dist` — placeholder fails the `security_headers` SPA test) |
| **Estimated runtime** | quick ~20 s · full ~3–5 min |

**Hard constraints (project memory):**
- Never run two `cargo test` processes concurrently — they contend on the `target/` lock and look like a multi-minute hang (`cargo-no-concurrent-test`).
- `cargo test --workspace` hangs on the pre-existing `auth_remember_cookie` test — use the targeted `-p trackly-app` commands above (`workspace-test-hangs-auth-remember-cookie`).

---

## Sampling Rate

- **After every task commit:** `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --lib pdf:: -- --test-threads=1`
- **After every plan wave:** full suite command above, with `pnpm --dir ui build` run first
- **Before `/gsd-verify-work`:** full suite green **AND** the Level-2 human visual pass (below) done on both transports
- **Max feedback latency:** ~20 s (quick) / ~300 s (full)

---

## Per-Task Verification Map

> Task IDs are filled in by the planner. Rows below are the requirement→test contract the plan must satisfy.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| TBD | TBD | 1 | DOC-04 | — | N/A | structural | `cargo test -p trackly-app --test html_header_parity` — all three templates contain `{% include "_header.html" %}` | ❌ W0 — new file, model on `html_page_parity.rs` | ⬜ pending |
| TBD | TBD | 2 | DOC-04 | — | N/A | integration | `cargo test -p trackly-app --test html_header_parity -- render_header` — rendered header fragment byte-identical across the three forms | ❌ W0 | ⬜ pending |
| TBD | TBD | — | DOC-04 | — | N/A | **manual UAT** | — (Level 2, see Manual-Only) | N/A | ⬜ pending |
| TBD | TBD | 1 | DOC-05 | T-34-PRIV | Shipped templates carry no literal org name — the name node must be a Jinja expression | structural | `cargo test -p trackly-app --lib templates_have_no_literal_org_name` — **privacy-safe form: assert positively that the name element's content is `{{ org.* }}` / contains no non-placeholder Cyrillic literal. Do NOT write the real org name into the test as a negative assertion — that commits it to a public repo.** | ❌ W0 | ⬜ pending |
| TBD | TBD | 2 | DOC-05 | — | N/A | unit | extend `pdf_render_act.rs` with a case where `full_name` is non-empty | partial — file exists, new `#[tokio::test]` needed | ⬜ pending |
| TBD | TBD | 1 | DOC-05 / D-03 | T-34-XSS | HTML-escape runs **before** `<br>` insertion; `<script>` in the org field renders inert | unit | `cargo test -p trackly-app --lib org_full_name_html` — asserts output contains `&lt;script&gt;` and `<br />`, and does **not** contain a literal `<script>` | ❌ W0 | ⬜ pending |
| TBD | TBD | 3 | DOC-06 | — | Fail-closed preserved: unrecognized file is never overwritten | unit | `cargo test -p trackly-app --lib upgrade` — new v21-specific case alongside the 3 existing tests | partial — 3 tests exist, v21 case new | ⬜ pending |
| TBD | TBD | 3 | DOC-06 / D-16 | — | Silent skip becomes observable | manual (or unit if `tracing-test` is present) | `grep tracing-test Cargo.toml` first; else manual run with `RUST_LOG=warn` | N/A default | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/trackly-app/tests/html_header_parity.rs` — substring + render gate for DOC-04, modeled on the existing `html_page_parity.rs`
- [ ] Privacy-safe "no literal org name in shipped templates" structural test for DOC-05 — **the test itself must not contain the real name**; confirm the chosen form with `/gsd-secure-phase`
- [ ] Unit test for the Rust `full_name` escaping helper (escape → `<br>` order, D-03)
- [ ] v21-specific upgrade unit test in `html_templates.rs` (DOC-06)
- [ ] Framework install: **not required** — `cargo test` is already configured, no new dependencies

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Header geometry and typography render identically across all three forms | DOC-04 (Success Criterion #2) | Text-extraction tests cannot see overlap or overflow (project memory `act-pdf-word-fidelity`). Success Criterion #2 explicitly demands a real rendered PDF/preview, not a text test. | Render a real preview/PDF of act_handover, act_acceptance and report. Compare logo → name → requisites block visually. |
| Same, in the LAN browser transport | DOC-04 (Success Criterion #2) | Server mode serves `ui/dist`; the desktop webview HMRs separately (project memory `dev-browser-testing-needs-ui-build`). | Run `pnpm --dir ui build`, start server mode, open each form's print preview in a LAN browser, repeat the visual comparison. |
| Long legal name / long address does not overflow the 80 mm block | DOC-04 / D-06 | `overflow-wrap` / `hyphens` behavior depends on the engine's hyphenation dictionary (RESEARCH assumptions A2, A3 — `[ASSUMED]`, unverified). | Enter a deliberately long fictional multi-line name and a long fictional address, render on both transports, confirm no horizontal overflow. |
| Cyrillic serif fallback on Windows | DOC-04 / D-10 | Windows is not reachable from the dev macOS box (project memory `dev-environment-constraints`); RESEARCH assumption A1 is `[ASSUMED]`. | On the Windows test machine, render each form and confirm the text is serif and Cyrillic renders correctly. |
| Upgrade reaches an already-installed copy | DOC-06 (Success Criterion #4) | Requires a real on-disk `templates/` dir matching the v21 snapshot plus an app restart. | Seed a templates dir with the v21 bodies, launch, confirm the new header appears; repeat with a hand-edited file, confirm it is left alone and a `warn` is logged. |
| Post-upgrade appearance with an empty `full_name` (C-01) | DOC-05 | Consequence of D-04 that only a human can judge as acceptable. | After upgrade with `full_name` empty and short name set, confirm the header shows only the short name in parentheses; decide whether that is acceptable. |

---

## Validation Sign-Off

- [ ] All tasks have an `<automated>` verify or a Wave 0 dependency
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all ❌ MISSING references above
- [ ] No watch-mode flags; `--test-threads=1` used; no concurrent `cargo test`
- [ ] Feedback latency < 300 s
- [ ] Level-2 human visual pass completed on **both** transports
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
