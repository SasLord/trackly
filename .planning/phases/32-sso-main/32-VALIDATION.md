---
phase: 32
slug: sso-main
status: complete
nyquist_compliant: true
wave_0_complete: true
created: 2026-08-03
validated: 2026-08-18
validated_by: /gsd-validate-phase 32 (retroactive audit)
---

# Phase 32 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Derived from `32-RESEARCH.md` §"Validation Architecture".
> Statuses below were re-audited retroactively on 2026-08-18 by re-running every
> listed command against `main` — see `## Validation Audit 2026-08-18`.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (workspace + per-crate/per-test targeted runs) |
| **Config file** | Inline `#[cfg(test)] mod tests` in `config.rs`/`auth.rs`; integration file `crates/trackly-app/tests/ad_admin_logins.rs` (mirrors `ad_directory_sso.rs`/`ad_register.rs`) |
| **Quick run command** | `cargo test -p trackly-infra --lib config::` + `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test ad_admin_logins` |
| **Full suite command** | `cargo test --workspace --no-fail-fast -- --test-threads=1` (matches `ci-fast`/`ci-full` invocation) |
| **Estimated runtime** | targeted runs ~1s after build; full workspace ~71 min |

> ⚠ Repo constraints (memory): never run two `cargo test` concurrently (contends on `target/` lock); `--workspace` is known to hang/crawl on the pre-existing `auth_remember_cookie` test — prefer targeted per-crate/per-test runs during task loops.
> ⚠ `trackly-app` tests require `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1`.
> ⚠ `cargo test -p trackly-infra config::` (without `--lib`) matches 0 tests — the config tests live in the lib target. Use `--lib`.

---

## Sampling Rate

- **After every task commit:** Run the targeted `cargo test -p <crate> <path>` for the crate touched.
- **After every plan wave:** `cargo test --workspace --no-fail-fast -- --test-threads=1` + `cargo clippy --workspace --all-targets -- -D warnings` + `cargo fmt --all -- --check`.
- **Before phase verify / `main` merge:** Full suite green AND `cargo fmt --all -- --check` green AND a real `ci-full` run green (via PR).
- **Max feedback latency:** targeted run (seconds); full suite per wave.

---

## Per-Task Verification Map

> Filled by the planner from `32-RESEARCH.md` §"Phase Requirements → Test Map". SSO-02 behaviors below are the required coverage.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | Test Name | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-----------|--------|
| 32-01-01 | 01 | 1 | SSO-02 | — | `admin_logins` TOML field parses (flat array), defaults to empty | unit | `cargo test -p trackly-infra --lib config::` | `config::tests::admin_logins_flat_array_deserializes_and_defaults_empty` | ✅ green |
| 32-02-01 | 02 | 2 | SSO-02 | T-32 V4 | Unknown login in list → INSERT active admin, no pending request | integration | `cargo test -p trackly-app --test ad_admin_logins` | `admin_logins_unknown_user_becomes_active_admin_no_pending_request` | ✅ green |
| 32-02-02 | 02 | 2 | SSO-02 | T-32 V9 | Pending user in list → activated admin + open `ad_register` auto-completed, audit_log row written | integration | same | `admin_logins_pending_user_activated_and_request_completed` | ✅ green |
| 32-02-03 | 02 | 2 | SSO-02 | T-32 V4 | Blocked/soft-deleted user in list → revived active admin (overrides manual block, D-07) | integration | same | `admin_logins_blocked_user_revived_as_admin` + `admin_logins_soft_deleted_user_revived_as_admin` | ✅ green |
| 32-02-04 | 02 | 2 | SSO-02 | — | Existing active non-admin in list → escalated to admin on next login (D-06) | integration | same | `admin_logins_active_non_admin_escalated_to_admin` | ✅ green |
| 32-02-05 | 02 | 2 | SSO-02 | — | Already active admin in list → idempotent no-op (version unchanged) | integration | same | `admin_logins_already_admin_is_idempotent_noop` | ✅ green |
| 32-02-06 | 02 | 2 | SSO-02 | — | Login NOT in list → Phase 31 behavior unchanged (regression) | integration | same | `admin_logins_not_in_list_phase31_behavior_unchanged` | ✅ green |
| 32-02-07 | 02 | 2 | SSO-02 | T-32 V4 | Forces admin even when `AdDirectory::resolve` is `Unreachable`/`NotConfigured` (D-10) | integration | same | `admin_logins_forces_admin_when_directory_unreachable` | ✅ green |
| 32-02-08 | 02 | 2 | SSO-02 | — | Case-insensitive + UPN/NetBIOS matching (`us100`/`US100@...`/`EXAMPLE\us100` → `us100`) | unit | `cargo test -p trackly-app --lib admin_login` | `normalize_login_for_admin_check_strips_upn_and_netbios_and_lowercases`, `is_admin_login_matches_upn_and_netbios_forms_after_with_admin_logins`, `is_admin_login_defaults_to_empty_set_when_builder_not_called` | ✅ green |
| — | 02 | 2 | SSO-02 | — | Forced-admin applies via the LDAPS password-bind path too (injection at `on_ad_bind_success`, not only `sso_login`) | integration | same | `admin_logins_forces_admin_on_ldaps_password_bind_path_too` | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

### Coverage added after phase close (v1.3.2, ФИО-синхронизация)

Three further cases landed in the same file after the phase closed; they extend SSO-02's
forced-admin path and run in the same command, so they are recorded here for traceability:

| Test Name | Behavior |
|-----------|----------|
| `admin_logins_already_admin_syncs_changed_name_from_directory` | Already-admin: ФИО пересинхронизируется из каталога при смене фамилии в AD |
| `admin_logins_already_admin_does_not_overwrite_name_with_untrusted_caller_supplied_name` | Caller-supplied (недоверенное) имя НЕ перезаписывает ФИО |
| `admin_logins_active_non_admin_escalation_also_syncs_changed_name` | Эскалация до admin также синхронизирует изменившееся ФИО |

---

## Wave 0 Requirements

- [x] `crates/trackly-infra/src/config.rs` — `admin_logins: Vec<String>` field + Default + parsing tests (`config.rs:289`, `:344`, `:511`)
- [x] `crates/trackly-app/src/services/auth.rs` — normalize/`is_admin_login` helpers, `with_admin_logins` builder, forced-admin provisioning + injection in `on_ad_bind_success` (`auth.rs:168`, `:233`, `:247`, `:508`, `:579`)
- [x] `crates/trackly-app/src/context.rs` — `.with_admin_logins(config.ad.admin_logins.clone())` on the `AuthService::new(...)` chain (`context.rs:336`)
- [x] Integration test file `crates/trackly-app/tests/ad_admin_logins.rs` covering the full state matrix (12 cases, all green)
- [x] `trackly.config.toml.example` — documents `admin_logins` next to `role_mapping` (+ "requires restart" note + TOML ordering warning, lines 115–125)
- [x] **Pre-existing, merge-blocking:** `cargo fmt --all` run + commit (fmt drift) before/at merge — done in Plan 32-04 (`bfb77a0`), confirmed by green `ci-full` on PR #1

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Status |
|----------|-------------|------------|--------|
| Real SPNEGO login from a listed domain login yields admin on Windows/AD | SSO-02 | No AD reachable from dev macOS (dev-env constraint) | ⚠️ **Не подтверждено явно.** Живой AD-UAT после `v1.3.0` состоялся (и вскрыл LDAP/ФИО-проблемы → `v1.3.1`/`v1.3.2`), но отдельного протокола «логин из `admin_logins` получил роль администратора на живом AD» не зафиксировано. Остаётся ручным пунктом. |
| `ci-full` matrix green on all 3 OSes before `main` merge | D-11/D-12 | CI-only | ✅ Выполнено — PR #1, head `079e0ee`: ubuntu / macos / windows + procmon (windows) + ci-fast — всё pass |
| Tag `v1.3.0` triggers `release.yml` build | D-11 | Release infra | ✅ Выполнено — аннотированный тег `v1.3.0` на merge-коммите `ab25d4c`, `release.yml` запущен |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency acceptable (targeted runs ~1s post-build)
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-08-18 (retroactive audit, `/gsd-validate-phase 32`)

---

## Validation Audit 2026-08-18

Ретроактивный аудит по требованию QA-04 (Фаза 38). Файл был оставлен в состоянии
`draft` планировщиком и ни разу не обновлялся во время исполнения Фазы 32 — при этом
код и тесты были написаны полностью. Аудит перепроверил каждую строку карты, повторно
запустив команды на `main`.

| Metric | Count |
|--------|-------|
| Gaps found | 0 |
| Resolved | 0 (нечего закрывать — тесты уже существовали) |
| Escalated | 0 |
| Rows re-verified green | 9/9 (+1 незапланированная строка + 3 пост-фазовых теста v1.3.2) |
| Tests written by this audit | 0 |

**Прогоны (2026-08-18, `main` @ `5552fa85`):**

| Command | Result |
|---------|--------|
| `cargo test -p trackly-infra --lib config::` | 10 passed; 0 failed |
| `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --lib admin_login` | 2 passed; 0 failed |
| `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --lib normalize_login_for_admin_check` | 1 passed; 0 failed |
| `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test ad_admin_logins` | **12 passed; 0 failed** |

**Расхождение с исходным черновиком:** карта содержала 9 строк со статусом `⬜ pending`
и пометкой `❌ W0` («файла ещё нет») — это состояние *до* исполнения. Все 9 поведений
реализованы и покрыты; `nyquist_compliant` переведён в `true`.

**Найдено попутно (вне области Фазы 32):** `cargo fmt --all -- --check` сейчас красный,
но дрейф целиком в файлах фаз 34–36 (`crates/trackly-app/src/pdf/html_templates.rs`,
`pdf/minijinja_env.rs`, `tests/html_act_render.rs`, `tests/html_header_parity.rs`,
`tests/org_settings.rs`). Merge-блокирующий пункт Фазы 32 был закрыт в своё время
(`bfb77a0` + зелёный `ci-full`); текущий дрейф — новый долг более поздних фаз.
