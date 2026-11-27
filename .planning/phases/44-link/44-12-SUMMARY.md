---
phase: 44-link
plan: 12
subsystem: link-client
tags: [link-app, enroll, auth, sso, resume, logout, revoked]
requires: [44-07, 44-08, 44-11]
provides: [enroll::setup_with_invite, authflow::AuthFlow, tests/common мок-сервер]
affects: [44-13, 44-14, 44-15]
requirements: [LNK-06, LNK-07, LNK-08]
key-files:
  created:
    - crates/link-app/tests/enroll_flow.rs
    - crates/link-app/tests/auth_flow.rs
    - crates/link-app/tests/common/mod.rs
  modified:
    - crates/link-app/src/enroll.rs
    - crates/link-app/src/authflow.rs
decisions:
  - "ConfigPaths {user_dir, machine_config} определён в enroll.rs (в плане упомянут, но не существовал); при машинном конфиге ввод кода -> EnrollError::AlreadyConfigured без сетевого вызова."
  - "authflow отображает ответы LinkHttp на sso::HttpOutcome: Server(503) -> Status503, прочие Server/Network/Decode -> Network, Api{401|иное, code} -> Unauthorized{code}."
  - "Вход паролем и resume с кодами LINK_REVOKED/LINK_DEVICE_BANNED -> mark_revoked (токен стёрт, revoked=true на диске); retry_sso при revoked возвращает Revoked."
  - "Выход вне Windows флаг logged_out не ставит: автовхода нет, блокировать нечего."
metrics:
  tasks: 2
  completed: 2026-11-28
---

# Phase 44 Plan 12: потоки первой настройки и входа Link Summary

`setup_with_invite` регистрирует устройство по коду с пином из кода (чужой сертификат обрывает рукопожатие до запроса, конфиг и `device_secret` не пишутся), `AuthFlow` реализует resume -> SSO -> форма, пароль только по явному вызову, выход и отзыв по D-09..D-14, D-25.

## Коммиты

- `8ca0fa60` enroll.rs + tests/common + 7 тестов enroll_flow
- `40bd779b` authflow.rs + 17 тестов auth_flow

## Гейты

| Гейт | Результат |
|------|-----------|
| `cargo test -p link-app --test enroll_flow` | 7/7 |
| `cargo test -p link-app --test auth_flow` | 17/17 |
| `cargo clippy -p link-app --all-targets -- -D warnings` | чисто |
| `cargo fmt --check -p link-app` | чисто |
| мутация: снят гейт `revoked` в `startup_auth` | test5 краснеет, после отката зелёно |
| `grep -n "password_login(" authflow.rs` | единственное определение; внутренних вызовов нет |

## Deviations from Plan

1. **[Rule 3] ConfigPaths не существовал** — введён в enroll.rs. `local` принимается как `&LocalState` (изменять нечего), `now_unix` берётся из системных часов.
2. **Test 7 (enroll) без tracing-subscriber:** крейта в зависимостях нет, перехват сделан ручной реализацией `tracing::Subscriber` в тесте; подписчик глобальный (кэш интереса вызовов делал `set_default` недетерминированным при параллельных тестах).
3. **LINK_DEVICE_EXISTS** на enroll отображается в Internal (нет подходящего кода CmdErrorCode); отозванный код приходит как `LINK_INVITE_INVALID` (план 11) и показывается как «не распознан».
4. Пароль копируется в `LinkLoginRequest.password` и обнуляется после отправки (`zeroize`); промежуточный `serde_json::Value` внутри `post_json` не зануляется — остаточный риск в памяти процесса.

## Заметки

- SSO-ветка протестирована только на `MockSso`/`CountSso`; реальный SPNEGO и Windows-провайдер (`WindowsSso`) проверяются живьём на Windows с AD в плане 20.
- Test 6 из enroll (машинный конфиг) реализован как отказ без записи и без сети.
- Мок-сервер `tests/common/mod.rs` (TLS на rcgen, журнал запросов) пригоден для планов 13-15.

## Авто-одобренные чекпоинты

Нет (план автономный). Для человека на Windows с доменным ПК: запустить Link при включённом SSO, убедиться в автовходе; нажать «Выйти», перезапустить — автовход не срабатывает до «Войти автоматически»; отозвать устройство в админке — Link показывает «отключён», автовход не повторяется.

## Known Stubs

Нет.

## Threat Flags

Новых поверхностей вне threat_model нет (T-44-43..46 закрыты тестами: enroll Test 2, отсутствие вызова пароля, Secret/zeroize, revoked-гейт; T-44-47 принят).

## Self-Check: PASSED

Файлы enroll.rs, authflow.rs, enroll_flow.rs, auth_flow.rs, common/mod.rs существуют; коммиты в истории.
