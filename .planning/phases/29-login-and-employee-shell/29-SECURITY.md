# SECURITY.md — Phase 29: login-and-employee-shell

**Audit date:** 2026-07-24
**ASVS Level:** 1
**Block-on:** high
**Result:** SECURED — 10/10 threats resolved (5 mitigate CLOSED, 5 accept documented)

UI-only phase: ports the login / blocked / pending / first-run / employee-shell screens onto the
design-system primitives (`AuthShell`, `FormField`, `Input`, `Button`, `Checkbox`). No new backend
data flow, no new trust boundary. Verification confirms every declared mitigation exists in the
implemented code, not just in the plan.

---

## Threat Verification (verified against implemented code)

| Threat ID | Category | Disposition | Status | Evidence (file:line) |
|-----------|----------|-------------|--------|----------------------|
| T-29-01 | Tampering | accept | CLOSED (documented) | `ui/src/lib/components/Input.svelte:6` — `type?: 'text' \| 'number' \| 'search' \| 'password'` additive union member; `{type}` spread unchanged at line 38. See accepted-risks log below. |
| T-29-02 | Information Disclosure | mitigate | CLOSED | `ui/src/lib/components/Input.svelte:38` — native `{type}` passthrough drives browser masking; no `console.log`/custom value rendering anywhere in Input/auth/layout (grep: none). |
| T-29-03 | Information Disclosure | mitigate | CLOSED | `ui/src/features/auth/LoginPage.svelte:17` `GENERIC_AUTH_ERROR`, `:18` `AD_UNREACHABLE_ERROR`, `:73-87` code-branching — the `else` branch collapses all non-infra failures to the single generic message (anti-enumeration D-Sec-01). |
| T-29-04 | Spoofing/EoP | mitigate | CLOSED | `ui/src/features/auth/LoginPage.svelte:151` — `<Button type="button" variant="ghost" disabled>` with NO `onclick` and NO `tabindex` (grep confirms zero of both on file). Native `disabled` removes it from tab order. |
| T-29-05 | Information Disclosure | mitigate | CLOSED | Password masking on both screens: `LoginPage.svelte:131` `type="password"`; `FirstRunWizard.svelte:136,151` `type="password"` (password + confirm). All via Input's `{type}` passthrough. |
| T-29-06 | Information Disclosure | mitigate | CLOSED | `ui/src/features/auth/BlockedScreen.svelte:44-60` `handleRestoreRequest` + `:64-102` 4-branch conditional (submitted/pending/rejection_reason/default) present and intact; only markup/style migrated to `AuthShell`/`Button`. |
| T-29-07 | Repudiation | accept | CLOSED (documented) | `ui/src/features/auth/BlockedScreen.svelte:86,98` — `onclick={handleRestoreRequest}` forwards to the same `request_ad_restore` call (`:48`); no new indirection. See accepted-risks log. |
| T-29-08 | Tampering | accept | CLOSED (documented) | `ui/src/features/layout/EmployeeLayout.svelte:47-108` — `handleEmployeeWsEvent`/`onMount`/`connectWs`/`onWsEvent`/`logout` intact; phase-29 edits + UAT fixes were CSS/markup-wrapper only (`.theme-switcher-slot`, `.user-name` flex-shrink), no script logic change. See accepted-risks log. |
| T-29-09 | Elevation of Privilege | mitigate | CLOSED | `ui/src/features/layout/EmployeeLayout.svelte:5-6,113-132` — header-only shell, no sidebar / no new navigable route; comment reaffirms the employee boundary is backend 403 (Phase 10), not client-side hiding. |
| T-29-SC | Tampering (supply chain) | accept | CLOSED (documented) | All 4 SUMMARY.md `tech-stack.added: []`; no npm/pip/cargo install introduced. See accepted-risks log. |

---

## Accepted Risks Log

| Threat ID | Category | Rationale | Residual risk |
|-----------|----------|-----------|---------------|
| T-29-01 | Tampering (Input type union) | Additive-only union member (`'password'`); `{type}` DOM passthrough unchanged; `pnpm build` proves the 19 existing consumers compile unaffected. No code path is created that could be tampered with. | None. |
| T-29-07 | Repudiation (restore-CTA) | The `<Button onclick>` forwards to the exact same `handleRestoreRequest` reference that fires `request_ad_restore`; identity is re-proven server-side by AD re-bind (the user has no session). No change to when/how the request fires. Audit trail is a backend concern, unchanged by this UI phase. | None new in-scope. |
| T-29-08 | Tampering (EmployeeLayout WS/logout) | Phase-29 change is 2 CSS token values plus two UAT-driven CSS/markup-wrapper fixes; the WS handler, refCount teardown, and `logout` session-clearing are byte-identical script logic. | None new in-scope. |
| T-29-SC | Supply chain | No new package-manager installs across any of the 4 plans; components are hand-written Svelte on already-vetted deps (Svelte 5 runes). Package Legitimacy Gate not triggered. | None. |

---

## Unregistered Flags (WARNING — informational, non-blocking)

| Flag | Where | Assessment |
|------|-------|------------|
| `autocomplete` prop added to `Input.svelte` | `ui/src/lib/components/Input.svelte:13,27,46` — `autocomplete?: HTMLInputAttributes['autocomplete']`, passed to native `<input autocomplete>` | Appeared during 29-02 (Rule-3 auto-fix) with no mapped threat ID. Slightly exceeds T-29-01's "single-line additive union" framing (a second additive edit to the same primitive). Assessment: benign hardening, not new attack surface — a DOM attribute passthrough that enables `autocomplete="current-password"` (password-manager-friendly) with no logging or value capture. No new consumer regressed (no prior consumer passed `autocomplete`). No action required. |
| Post-phase quick-task fixes (WR-01 WS refcount leak, WR-02 empty-string `rejection_reason`) | `EmployeeLayout.svelte:63-93` (`disposed` guard); `BlockedScreen.svelte:78` (`rejection_reason !== null`) | Applied by quick task `260724-pxf` AFTER phase-29 execution (git `17aecfc`/`97bf660`/`afc8645`). Both are security-relevant hardening (prevents WS refcount leak; corrects blocked-state misclassification). Present in current code, do not open any phase-29 threat. Informational only. |

No unregistered flag rises to BLOCKER. All are additive/hardening changes.

---

## Verdict

All 5 `mitigate` threats have their declared mitigation present in the implemented code (verified by
file:line, not by intent). All 5 `accept` threats are recorded in the accepted-risks log above. No
declared mitigation is absent or contradicted. No high-severity open threat exists.

**threats_open: 0** — phase clears the `block_on: high` gate.
