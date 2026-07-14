---
phase: 21
slug: cartridge-drum-codes
status: verified
threats_open: 0
asvs_level: 1
created: 2026-07-15
---

# Phase 21 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| Server-side auto-code generation | `assign_code_in_tx` (no-override branch) generates codes internally from a DB counter — no external input crosses this path | `cartridge_seq` / `drum_seq` counter value → formatted code string (non-sensitive) |
| `code_override` input path | Caller-supplied custom code validated for UNIQUE before insert | User-supplied `code` string (non-sensitive; uniqueness-controlled) |

*No new trust boundary introduced. The change is a format-width adjustment (`{seq:06}` → `{seq:04}`) inside an existing server-generated path; no new input surface, no auth/authz change, no external I/O.*

---

## Threat Register

| Threat ID | Category | Component | Disposition | Mitigation | Status |
|-----------|----------|-----------|-------------|------------|--------|
| T-21-01 | Repudiation/Integrity | `assign_code_in_tx` (auto-code branch) | accept | Change is format-width only (`{:04}` vs `{:06}`); code generation stays server-side with the retry-loop on UNIQUE collision unchanged — collision/counter-loss risk not increased. Verified `cartridges_sqlite.rs:148-163`. | closed |
| T-21-02 | Tampering | `code_override` (custom code input) | accept | `code_override` validation path (`SELECT EXISTS` UNIQUE check → `AppError::Conflict`) is untouched by this phase; existing ASVS L1 control preserved as-is. Verified `cartridges_sqlite.rs:124-138`. | closed |

*Status: open · closed*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-21-01 | T-21-01 | Auto-code format narrowed from 6 to 4-digit minimum width. `{:04}` is a *minimum* width — seq > 9999 naturally widens to 5+ digits, existing 6-digit codes remain valid distinct strings (`C-0001` ≠ `C-000001`), no migration or collision. Server-side generation + retry-loop preserved unchanged. | Phase 21 owner (Alexander Platov) | 2026-07-15 |
| AR-21-02 | T-21-02 | `code_override` UNIQUE-validation branch not modified by this phase; existing L1 duplicate-code control (`AppError::Conflict`) verified present and unchanged. | Phase 21 owner (Alexander Platov) | 2026-07-15 |

*Accepted risks do not resurface in future audit runs.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-07-15 | 2 | 2 | 0 | gsd-secure-phase (register verified against implementation; short-circuit — plan-time register, no mitigate dispositions) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-07-15
