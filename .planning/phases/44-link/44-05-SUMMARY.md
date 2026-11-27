---
phase: 44-link
plan: 05
subsystem: link-client
tags: [link-app, config, secrets, state-contract, keyring]
requires: [44-01, 44-03]
provides:
  - link-app как lib (link_app) + bin (trackly-link) с 16 заранее объявленными модулями
  - контракт LinkState / LinkEvent / LinkCmdError и golden-фикстуры для link-ui
  - config.rs (ConfigSource, resolve, LocalState) и secrets.rs (TokenStore, DeviceSecretStore)
affects: [44-07, 44-08, 44-09, 44-10, 44-12, 44-15, 44-17]
key-files:
  created:
    - crates/link-app/src/lib.rs
    - crates/link-app/src/state.rs
    - crates/link-app/src/config.rs
    - crates/link-app/src/secrets.rs
    - crates/link-app/tests/state_contract.rs
    - crates/link-app/tests/fixtures/link_state.json
    - crates/link-app/tests/fixtures/link_commands.json
  modified:
    - Cargo.toml
    - Cargo.lock
    - crates/link-app/Cargo.toml
decisions:
  - "Первый найденный источник конфига решает всё: невалидный машинный конфиг даёт Invalid, а не молчаливый откат на пользовательский (D-26)"
  - "TokenStore не содержит файлового хранилища вообще: токен физически не может попасть на диск (D-10)"
requirements-completed: [LNK-07, LNK-08]
duration: 35min
completed: 2026-11-28
---

# Phase 44 Plan 05: Каркас клиента Link Summary

link-app стал lib+bin с зависимостями без aws-lc-rs, зафиксированным контрактом состояний/команд (golden JSON), конфигом (машинный побеждает целиком) и хранилищем секретов с деградацией (keyring, память, файл 0600).

## Коммиты

- fd15994c: зависимости, lib.rs, state.rs, фикстуры, state_contract
- 97e669be: 16 модулей-заглушек
- c1616a45: config.rs
- ab3256ac: secrets.rs

## Проверки

- `cargo test -p link-app`: 16 юнит (config 8, secrets 8) + 2 main.rs + 5 state_contract, все зелёные
- `cargo clippy -p link-app --all-targets -- -D warnings`: чисто
- `cargo fmt --check -p link-app`: чисто
- **`cargo tree -p link-app -i aws-lc-rs`: пусто** («package ID specification `aws-lc-rs` did not match any packages»). link-app свободен от aws-lc-rs. tokio-rustls в link-app объявлен напрямую с `default-features = false, features = ["ring","tls12"]`, потому что workspace-запись `tokio-rustls = "0.26"` включает aws-lc-rs по умолчанию.
- `tray-icon` есть в link-app, в crates/trackly-app/Cargo.toml его нет.

## Отклонения от плана

1. **[Rule 3] tokio-tungstenite** в workspace записан с `default-features = false, features = ["connect","handshake"]` (без TLS-фич): TLS будет поверх tokio-rustls вручную (план 07), иначе тянется лишний провайдер. trackly-app держит свою прямую запись 0.29 и не затронут.
2. **[Rule 3] tokio-rustls** — не workspace=true, а прямая запись (см. выше).
3. tokio-rustls и rustls вынесены в обычные зависимости (план называл rustls; tokio-rustls нужен ядру, а не только тестам).
4. `user_config_dir` помечена `#[allow(clippy::disallowed_methods)]`: в clippy.toml запрещён `tauri::Manager::path`, но Link не portable (ADR 0005).
5. Добавлены `LinkCmdError::new/with_server` с готовыми русскими текстами из UI-SPEC §6.3/§6.5 и `file_store_dir`, `standard_sources` (хелперы для плана 17).

## Не проверено

- Ветки `#[cfg(windows)]` и `#[cfg(target_os = "linux")]` в `KeyringStore::init` и `machine_config_path` на macOS не компилируются; API (`Store::new() -> Result<Arc<Self>>`) сверено по исходникам крейтов, сборка на Windows/Linux будет проверена CI.
- Реальный Keychain macOS не опрашивался (тест идёт через keyring_core::mock).
- Feature unification: при `cargo build --workspace` feature `tray-icon` у tauri объединится и для trackly-app. Это не меняет Cargo.toml trackly-app, но бинарь сервера при сплошной сборке может получить tray-код; per-package сборки (`-p`, как в release/CI) не затронуты.

## Авто-одобренные чекпоинты

Нет (план полностью автономный).

## Threat Flags

Новых поверхностей вне threat_model нет.

## Known Stubs

Модули pin, http_client, ws_client, sso, backoff, enroll, authflow, connection, tray, autostart, window_policy, commands, runtime — намеренные однострочные заглушки (`//! Заполняется планом 44-NN.`), их наполняют планы 07, 08, 09, 12, 15, 17.

## Self-Check: PASSED
