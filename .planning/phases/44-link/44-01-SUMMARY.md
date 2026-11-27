---
phase: 44-link
plan: 01
subsystem: supply-chain
tags: [dependencies, human-verify, T-44-SC]
requirements-completed: []
key-files:
  created: []
  modified: []
duration: 10min
completed: 2026-11-27
---

# Phase 44 Plan 01: Вердикт легитимности новых крейтов — Summary

**Пользователь проверил все 7 новых крейтов и разрешил их (плюс прямой reqwest 0.13). Зависимости в этом плане не устанавливались.**

## Таблица пакетов (данные crates.io API, 2026-11-27)

| Пакет | Версия | Репозиторий | Загрузки всего / за 90 дн. | Последнее обновление | В Cargo.lock | Вердикт человека |
|-------|--------|-------------|----------------------------|----------------------|--------------|------------------|
| keyring-core | 1.1.0 | github.com/open-source-cooperative/keyring-core | 9,59 млн / 7,70 млн | 2026-11-20 | нет | **разрешён** |
| windows-native-keyring-store | 1.1.0 | github.com/open-source-cooperative/windows-native-keyring-store | 5,18 млн / 4,03 млн | 2026-05-24 | нет | **разрешён** |
| apple-native-keyring-store | 1.0.3 | github.com/open-source-cooperative/apple-native-keyring-store | 5,06 млн / 3,95 млн | 2026-11-22 | нет | **разрешён** |
| zbus-secret-service-keyring-store | 1.0.1 | github.com/open-source-cooperative/zbus-secret-service-keyring-store | 5,58 млн / 4,61 млн | 2026-08-15 | нет | **разрешён** |
| cross-krb5 | 0.5.0 | github.com/estokes/cross-krb5 | 1,42 млн / 0,52 млн | 2026-05-31 | нет | **разрешён** |
| auto-launch | 0.6.0 | github.com/zzzgydi/auto-launch | 8,74 млн / 3,43 млн | 2026-01-10 | нет | **разрешён** |
| x509-parser | 0.18.1 | github.com/rusticata/x509-parser | 232 млн / 79,8 млн | 2026-02-05 | да (0.18.1) | **разрешён** |
| reqwest (прямая, 0.13) | 0.13.5 | github.com/seanmonstar/reqwest | 965 млн | 2026-09-08 | да (0.12.28 и 0.13.5) | **разрешён** |

Имена сверены побуквенно, двойников не найдено.

## Поправка к RESEARCH

В RESEARCH для auto-launch указан репозиторий `tauri-apps/auto-launch`. На самом деле крейт живёт в **`zzzgydi/auto-launch`**. Этот же крейт использует официальный `tauri-plugin-autostart`. Пользователь разрешил его, зная это расхождение.

## aws-lc-rs (Pitfall 2)

`cargo tree -p trackly-app -i aws-lc-rs` показывает, что **aws-lc-rs v1.19.0 уже есть в графе** trackly-app: его тянет цепочка `ldap3 0.12.1 → rustls 0.23.46` из trackly-infra. На link-app это не распространяется. План 05 обязан проверить, что `cargo tree -p link-app -i aws-lc-rs` пуст, а reqwest собирается на ring.

## Решения

- Хранилища ОС (keyring) **разрешены**. По D-27 `device_secret` при отсутствии хранилища лежит в файле 0600, токен входа хранится только в памяти. Список пакетов это не меняет.
- cross-krb5 разрешён. Совместимость с серверным acceptor проверяется только живьём на Windows+AD. Если не сработает, запасной путь — прямой `windows` FFI (план 08).

## Чекпоинт

`checkpoint:human-verify` (blocking-human) закрыт **ответом пользователя в чате**, а не авто-одобрением. Пользователь ответил «Разрешить» по всем четырём вопросам.

## Self-Check: PASSED
- 44-01-SUMMARY.md существует, cross-krb5 упомянут
- Cargo.toml не менялся
