---
phase: 06-snmp
plan: 00
subsystem: testing
tags: [cargo-test, stub, nyquist, snmp, requests, printers]

requires:
  - phase: 05-auth-server-mode
    provides: trackly-app и trackly-infra workspace crates со всей тестовой инфраструктурой

provides:
  - 14 именованных #[ignore]-stub-тестов в crates/trackly-app/tests/phase06_stubs.rs
  - 1 именованный #[ignore]-stub-тест в crates/trackly-infra/tests/phase06_stubs.rs
  - Nyquist-compliance Wave-0 скаффолд для Phase 6

affects:
  - 06-01 (Wave 1 — реализует test_oid_profiles_seeded, test_mock_snmp)
  - 06-02 (Wave 2 — реализует большинство app-стабов)
  - 06-03 (Wave 3 — реализует test_snmp_mock_switch, test_ws_unauth_401)

tech-stack:
  added: []
  patterns:
    - "#[test] + #[ignore] stub pattern: все тела пусты ({}), нет импортов будущих модулей — compile-safe заглушки"

key-files:
  created:
    - crates/trackly-app/tests/phase06_stubs.rs
    - crates/trackly-infra/tests/phase06_stubs.rs
  modified: []

key-decisions:
  - "15 stub-тестов (14 trackly-app + 1 trackly-infra) — VALIDATION.md является авторитетным источником списка тест-имён (включая test_secret_debug, которого нет в PLAN.md task-секции)"
  - "Тела стабов — пустые {}, не panic! — оба варианта compile-safe, пустое тело чище для волновой замены"

patterns-established:
  - "Phase-N Wave-0 pattern: создаём все stub-тесты до реализации (Nyquist-rule); каждая волна заменяет заглушки на реальный код"

requirements-completed:
  - PRN-01
  - PRN-02
  - PRN-03
  - PRN-04
  - PRN-05
  - PRN-06
  - PRN-07
  - PRN-08
  - REQ-01
  - REQ-02
  - REQ-03
  - REQ-04
  - REQ-05
  - REQ-07

duration: 6min
completed: 2026-06-14
---

# Phase 06 Plan 00: Wave-0 Stub Tests Summary

**15 Nyquist-compliant #[ignore]-stub-тестов для Phase 6 SNMP + Requests созданы в двух crates; cargo check --workspace зелёный**

## Performance

- **Duration:** 6 min
- **Started:** 2026-06-14T14:36:19Z
- **Completed:** 2026-06-14T14:41:41Z
- **Tasks:** 1
- **Files modified:** 2

## Accomplishments

- Создан `crates/trackly-app/tests/phase06_stubs.rs` — 14 именованных `#[ignore]` stub-тестов (PRN-01..07, REQ-01..05, D-Mock-01, D-Retention-01, ASVS V4, CLAUDE.md Secret<T>)
- Создан `crates/trackly-infra/tests/phase06_stubs.rs` — 1 stub-тест (PRN-08 MockSnmpClient)
- `cargo check --workspace` — exit 0; `cargo test -p trackly-app --test phase06_stubs` — 14 ignored; `cargo test -p trackly-infra --test phase06_stubs` — 1 ignored
- VALIDATION.md статус `nyquist_compliant: true` / `wave_0_complete: true` подтверждён

## Task Commits

1. **Task 1: Создать stub-тестовые файлы Phase 6** - `5de0e07` (feat)

## Files Created/Modified

- `crates/trackly-app/tests/phase06_stubs.rs` — 14 #[ignore] stub-тестов Phase 6, trackly-app уровень
- `crates/trackly-infra/tests/phase06_stubs.rs` — 1 #[ignore] stub-тест Phase 6, trackly-infra уровень

## Decisions Made

- VALIDATION.md — авторитетный источник для списка тест-имён: содержит `test_secret_debug` (тест T-06-07-I), которого нет в секции `<action>` PLAN.md, но есть в таблице VALIDATION.md. Добавлен согласно VALIDATION.md — 15 тестов вместо 14 из PLAN.md.
- Тела stub-функций — пустые `{}` (не `panic!("stub")`): оба варианта compile-safe, пустые тела чище при волновой замене.

## Deviations from Plan

**1. [Rule 2 - Missing Critical] Добавлен 15-й stub-тест `test_secret_debug`**
- **Found during:** Task 1 (анализ VALIDATION.md vs PLAN.md)
- **Issue:** PLAN.md перечисляет 13 стабов для trackly-app, VALIDATION.md содержит 14 (включая `test_secret_debug` / T-06-07-I — Secret<T> Debug leak prevention). VALIDATION.md = авторитетный Nyquist-контракт фазы.
- **Fix:** Добавлен `test_secret_debug` в phase06_stubs.rs. Итого 14 стабов в trackly-app + 1 в trackly-infra = 15 всего.
- **Files modified:** crates/trackly-app/tests/phase06_stubs.rs
- **Verification:** `grep -c "#[test]" crates/trackly-app/tests/phase06_stubs.rs` → 14
- **Committed in:** 5de0e07

---

**Total deviations:** 1 auto-fixed (Rule 2 — missing test per VALIDATION.md)
**Impact on plan:** Необходимое дополнение для полного Nyquist-compliance. Никакого изменения сферы реализации.

## Issues Encountered

None — файлы скомпилировались с первого раза без ошибок.

## Known Stubs

По определению этот план является только Wave-0 скаффолдом: все 15 тестов являются намеренными стабами `#[ignore]`. Реализация заглушек распределена по волнам 1–3:

- Wave 1 (06-01): `test_oid_profiles_seeded`, `test_mock_snmp`
- Wave 2 (06-02): 11 остальных trackly-app тестов
- Wave 3 (06-03): `test_snmp_mock_switch`, `test_ws_unauth_401`

## Threat Flags

Нет новых threat surface — только .rs тестовые файлы, никаких сетевых endpoint'ов, БД изменений или auth-путей.

## Self-Check

- [x] `crates/trackly-app/tests/phase06_stubs.rs` существует
- [x] `crates/trackly-infra/tests/phase06_stubs.rs` существует
- [x] Коммит `5de0e07` существует в git log
- [x] `cargo check --workspace` — exit 0

## Self-Check: PASSED

## Next Phase Readiness

Wave-0 завершена. Phase 06 Plan 01 (Wave 1) может начаться: создаст миграции принтеров/заявок, OID-профили, MockSnmpClient — и заменит stubs `test_oid_profiles_seeded` + `test_mock_snmp` на реальные тесты.

---
*Phase: 06-snmp*
*Completed: 2026-06-14*
