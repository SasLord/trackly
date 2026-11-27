---
phase: 44-link
plan: 08
subsystem: link-app
tags: [sso, backoff, reconnect]
requires: [44-05]
provides: [SsoProvider, plan_sso, classify, can_retry, Backoff, SleepDetector]
affects: [44-12, 44-15]
key-files:
  modified:
    - crates/link-app/src/sso.rs
    - crates/link-app/src/backoff.rs
requirements: [LNK-06, LNK-09]
completed: 2026-11-28
---

# Phase 44 Plan 08: SSO-логика и backoff Summary

Чистая логика автовхода за трейтом `SsoProvider` (план попытки, классификация отказов, Windows-провайдер на cross-krb5 под `cfg(windows)`) и чистые функции переподключения (1,2,4,8,16,30 с с джиттером, индикация через 5 с, детект сна).

## Что сделано

- sso.rs: `NegotiateToken` со скрытым Debug, `SsoProvider`, `UnsupportedSso` (не-Windows, D-15), `MockSso`, `WindowsSso` (cfg(windows); NTLMSSP в токене даёт `NtlmFallback`), `plan_sso` (IP v4/v6 в скобках -> `Skip(IpAddress)`, иначе `HTTP/<host>` или spn из конфига), `classify`, `can_retry`, `SsoOutcome`. Пути авто-пароля нет, слова «password» в модуле нет.
- backoff.rs: `Backoff`, `should_show_offline`, `SleepDetector`, `OFFLINE_INDICATION_DELAY` = 5 с, `MAX_DELAY` = 30 с.

## Сверка с packages/transport/src/ws.ts

Старт 1000 мс, удвоение, потолок 30000 мс, `RECONNECT_TOAST_DELAY_MS` = 5000: значения совпадают. Джиттер ±20 % в браузере отсутствует, добавлен по D-23.

## Гейты

- `cargo test -p link-app --lib sso`: 6/6; `... backoff`: 5/5
- `cargo clippy -p link-app --all-targets -- -D warnings`: зелёный
- `cargo fmt --check -p link-app`: зелёный

## Важно (T-44-29)

Windows-провайдер не компилировался локально (target Windows не установлен) — проверяется в CI/на Windows. Реальный SPNEGO проверяется только живым прогоном на Windows с AD (план 20); мок и юнит-тесты этой проверкой не являются.

## Deviations from Plan

Нет. Замечание: `classify` принимает собственный `HttpOutcome` (определён в sso.rs), authflow (план 12) должен отобразить на него ответ LinkHttp.

## Авто-одобренные чекпоинты

Нет.

## Self-Check: PASSED
