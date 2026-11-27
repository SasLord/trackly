---
phase: 44-link
plan: 03
subsystem: link-proto
tags: [spki, invite-code, dto, shared-crate]
requirements-completed: [LNK-07, LNK-08]
key-files:
  created:
    - crates/link-proto/Cargo.toml
    - crates/link-proto/src/lib.rs
    - crates/link-proto/src/spki.rs
    - crates/link-proto/src/invite.rs
    - crates/link-proto/src/dto.rs
    - crates/link-proto/tests/spki_golden.rs
    - crates/link-proto/tests/fixtures/spki_fixture.crt.pem
    - crates/link-proto/tests/fixtures/spki_golden.txt
  modified:
    - Cargo.toml
    - Cargo.lock
duration: 15min
completed: 2026-11-27
---

# Phase 44 Plan 03: крейт link-proto — Summary

**Общий крейт: SPKI-отпечаток листа (SHA-256 от DER SubjectPublicKeyInfo, golden-фикстура), кодек кода-приглашения `TL1.` с контрольной суммой и JSON-контракт входа/enrollment.**

## Коммиты
- 33d6aea7 — каркас крейта, SPKI, golden-фикстура
- 929c8a63 — кодек приглашения и DTO

## Что сделано
- `spki_sha256`, `fmt_colon_upper` (95 символов, как `compute_fingerprint`), `parse_colon_hex`.
- Тесты: golden, «SPKI != хэш всего DER», один ключ с разными SAN даёт один SPKI, мусорный DER, формат hex.
- `InviteCode`: encode/decode, нормализация пробелов, `is_expired` (граница включена), `secret_hash`; ручной Debug скрывает секрет. Тот же приём применён к DTO с секретами (пароль, токены).
- DTO camelCase, константы `LINK_*` и заголовки `x-link-device` / `x-link-secret`. В enrollment нет сотрудника (D-04).
- Фикстура — одноразовый сертификат CN=fixture.example.local; приватного ключа в репозитории нет.

## Gates
- `cargo test -p link-proto`: 9 (lib) + 6 (golden) зелёные, 1 ignored (генератор фикстуры)
- `cargo clippy -p link-proto --all-targets -- -D warnings`: чисто
- `cargo fmt --check -p link-proto`: чисто
- pre-commit check-privacy: PASS 0 нарушений (дважды)

## Deviations from Plan
- В workspace.dependencies добавлены `link-proto`, `x509-parser = "0.18"` и `base64 = "0.22"` (base64 в workspace не было; версия та же, что у trackly-app). Cargo.lock получил только новое ребро. Правило 3.
- `secret_hash` экспортирован свободной функцией (не методом), как в формулировке поведения.

## Авто-одобренные чекпоинты
Нет.

## Self-Check: PASSED
