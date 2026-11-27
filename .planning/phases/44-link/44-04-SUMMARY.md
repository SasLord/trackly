---
phase: 44-link
plan: 04
subsystem: server-tls
tags: [tls, spki, rcgen, link]
requirements-completed: [LNK-08]
key-files:
  modified:
    - crates/trackly-app/Cargo.toml
    - crates/trackly-app/src/server/tls.rs
    - crates/trackly-app/src/server/mod.rs
    - crates/trackly-app/src/context.rs
    - crates/trackly-app/src/main.rs
    - crates/trackly-app/src/http/settings.rs
    - crates/trackly-app/src/tauri_cmds/auth.rs
    - crates/trackly-app/src/dto/auth.rs
duration: 25min
completed: 2026-11-28
---

# Phase 44 Plan 04: SPKI-отпечаток сервера — Summary

**TlsBundle считает SPKI-отпечаток общей `link_proto::spki_sha256`; самоподписанный сертификат при смене SAN перевыпускается на сохранённом `server.key`, поэтому SPKI (пин Link) не меняется; ServerHandle хранит TlsInfo.**

## Коммиты
- 5d9a2526 — tls.rs: SPKI, перевыпуск на сохранённом ключе, 6 новых тестов
- 3140ad39 — TlsInfo в ServerHandle, `AppCtx::server_tls_info`, DTO, get-настройки и статус

## Что сделано
- `TlsBundle.spki_fingerprint_hex` заполняется в `generate_self_signed` и `load_from_pem` (в т.ч. пользовательский cert_path).
- `load_or_generate_self_signed`: при смене SAN, если `server.key` читается и разбирается (`KeyPair::from_pem`), выпускается новый сертификат на нём; ключ не перезаписывается (права 0600 сохраняются). Нет ключа или он мусор — прежняя генерация целиком.
- `TlsInfo { fingerprint_hex, spki_fingerprint_hex, host, port }` без PEM/ключа; все три конструктора ServerHandle заполняют `tls`.
- `AppCtx::server_tls_info()` — читатель для плана 16 (выпуск кода).
- Закрыт TODO «store fingerprint in server_ctl»: get-настройки (HTTP и Tauri) и `server_status` возвращают реальные `fingerprint` и `spki_fingerprint`; при остановленном сервере оба None. Статус Tauri теперь отдаёт и url.

## Gates
- `cargo test -p trackly-app --lib server::` — 18 зелёных (6 новых: SPKI = общая функция, смена SAN -> тот же SPKI/другой DER, мусорный ключ, нет ключа, 0600 после перевыпуска, пользовательские файлы)
- `cargo test -p trackly-app --test export_bindings` — зелёный
- `cargo test -p trackly-app --test app_server --no-run` — собирается
- `cargo clippy -p trackly-app --all-targets -- -D warnings` — чисто
- `cargo fmt --check` — чисто
- check-privacy в pre-commit — PASS

## Deviations from Plan
- Поля DTO названы `spki_fingerprint` (snake_case), а не `spkiFingerprint`: соседние поля этих DTO (`fingerprint`, `desktop_lock_enabled`) без rename; план называл camelCase по ошибке. Ключ в bindings: `spki_fingerprint`. Правило 1.
- `ui/src/bindings.ts` в .gitignore — файл регенерирован тестом export_bindings, но не закоммичен (в плане был «закоммитить diff»). Проверено: добавлено только `spki_fingerprint` в обоих типах.
- `save_cert_only` — отдельная функция записи без ключа вместо ветки в `save_self_signed`.

## Авто-одобренные чекпоинты
Нет.

## Self-Check: PASSED
