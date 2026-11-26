---
phase: 42-link-monorepo-foundation
plan: 02
subsystem: testing
tags: [stand, ui-move, baseline, tls, ignore-test]
requires: []
provides:
  - "#[ignore]-стенд stand_ui_move (настоящий build_router + ui/dist, HTTPS 127.0.0.1)"
  - "42-STAND-BASELINE.md (проверка ассетов; консоль браузера UNVERIFIED)"
affects: [42-19]
key-files:
  created:
    - crates/trackly-app/tests/stand_ui_move.rs
    - .planning/phases/42-link-monorepo-foundation/42-STAND-BASELINE.md
  modified:
    - crates/trackly-app/Cargo.toml
decisions:
  - "Стенд зарегистрирован отдельным [[test]] stand_ui_move (D-20), гейт suite_wiring живёт в suite app_gates"
metrics:
  completed: 2026-11-26
  tasks: 2
  files: 3
---

# Phase 42 Plan 02: стенд живой проверки UI Summary

Одноразовый `#[ignore]`-стенд поднимает настоящий сервер с `ui/dist` и вымышленным админом; baseline зафиксирован частично, консоль браузера не снята.

## Выполнено

1. Стенд `stand_serves_real_ui` (коммит b972b76c): HTTPS 127.0.0.1:${STAND_PORT:-18480}, самоподписанный сертификат, файл-флаг и предел времени. fmt, clippy `-D warnings`, `suite_wiring` (5/5 в `app_gates`) зелёные, без `--ignored` тест «1 ignored».
2. Baseline (коммит 6933692a): стенд реально запущен, `/`, `theme-init.js`, js и css отдают 200, 404 на ассеты нет, вход принимается в обёртке `{"req": ...}`.

## Отклонения

- План называет проверку `cargo test --test suite_wiring`: отдельной цели нет, тесты гейта в suite `app_gates` (`--test app_gates suite_wiring`).
- Baseline консоли НЕ снят (Rule: не выдумывать). Встроенного браузера у исполнителя нет, вместо него curl-проверка ассетов.
- Бинарник стенда запущен напрямую после `--no-run`, чтобы cargo не уходил в фон.

## Открытые пункты

- UNVERIFIED: обход разделов и `console.error`/`console.warn` до переезда. Для плана 19: снять вручную на стенде (принять самоподписанный сертификат) на коммите до переезда.
- Десктопная сборка и ветка Tauri-invoke: UNVERIFIED (допущение D-21).

## Self-Check: PASSED

Файлы и коммиты b972b76c, 6933692a существуют; privacy-гейт зелёный.
