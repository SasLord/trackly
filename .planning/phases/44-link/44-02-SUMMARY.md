---
phase: 44-link
plan: 02
subsystem: database
tags: [sqlite, migration, link]
requires: []
provides:
  - "V060: таблицы link_devices, link_logins, link_invites"
affects: [44-link]
key-files:
  created:
    - migrations/V060__link_devices.sql
    - crates/trackly-infra/tests/link_migration.rs
  modified:
    - crates/trackly-infra/tests/suites/infra.rs
key-decisions:
  - "Запрет устройства (link_devices.revoked_at_utc) отделён от отзыва входа (link_logins.revoked_at_utc), D-13"
  - "Секреты только SHA-256 хэшами; без FK на users и без DROP"
requirements-completed: [LNK-07]
duration: 10min
completed: 2026-11-27
---

# Phase 44 Plan 02: Миграция V060 Summary

V060 создаёт три таблицы Link (устройства, входы, коды-приглашения) с хэш-only секретами, индексами по user/device и `PRAGMA user_version = 60`.

## Tasks

1. Task 1: V060 + тест `link_migration` (4 теста, зелёные, зарегистрированы в suite `infra_tests`) — коммит 16bc11a2.

## Проверки

- `cargo test -p trackly-infra --test infra_tests link_migration`: 4 passed.
- `cargo fmt`, `cargo clippy -p trackly-infra --all-targets -- -D warnings`: чисто (отдельными командами).
- Жёстко зашитых ожиданий «последняя версия = 59» нет, кроме проверки последней строки V059 в im_migration.rs (она по плану остаётся).
- Privacy-гейт pre-commit: PASS.

## Deviations from Plan

Корректив clippy в собственном тесте (`rfind` вместо `filter().next_back()`) — косметика. Иное: план выполнен как написан.

## Self-Check: PASSED
