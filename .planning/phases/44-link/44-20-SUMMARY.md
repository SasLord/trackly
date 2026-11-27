---
phase: 44-link
plan: 20
subsystem: phase-close
tags: [docs, mutation-checks, ci-full, validation, live-uat]
requirements-completed: []
key-files:
  created:
    - docs/LINK-CLIENT.md
  modified:
    - docs/AD-SETUP.md
    - .planning/phases/44-link/44-VALIDATION.md
duration: 60min
completed: 2026-11-28
---

# Phase 44 Plan 20: закрытие фазы Link Summary

Документы администратора написаны, пять несущих тестов доказанно краснеют при снятии гейта, все автоматические гейты и сплошной прогон зелёные. **Задача 3 (живая проверка) — awaiting human**: требования LNK-02..09, LNK-11 остаются Pending, фаза не закрыта.

## Задача 1: документация и мутации (коммит 18576ef3)

- `docs/LINK-CLIENT.md`: `link.toml` (ключи server, pin_sha256, spn; пути по ОС; машинный побеждает целиком), SPKI из «Настройки → Сеть», выпуск кода (1 ч / 24 ч / 7 сут, одноразовый), отзыв и «Разрешить», автозапуск всегда включён и восстанавливается, хранение секретов (device_secret в хранилище ОС или файл 0600; токен только в памяти).
- `docs/AD-SETUP.md`: раздел «Автовход Trackly Link» (SPN HTTP/<имя>, IP не работает, CNAME, часы > 5 мин).
- Сверка с `config.rs`: пути `%ProgramData%\Trackly Link\link.toml`, `/Library/Application Support/Trackly Link/link.toml`, `/etc/trackly-link/link.toml` и ключи server/pin_sha256/spn совпадают; пользовательский файл `link.toml` лежит в `app_local_data_dir`, секреты-файл в подкаталоге `secrets`.
- Privacy: `node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256` PASS (без `--hashes` скрипт отказывается работать; команда в плане устарела). Ручной просмотр: только example.local / EXAMPLE.

### Мутационные проверки (якоря встречались ровно по одному разу, файл восстановлен `git checkout`, `git status --short` чист)

| Мутация | Файл | Результат |
|---------|------|-----------|
| M1 | crates/link-app/src/pin.rs (`ct_eq` -> всегда true; общий путь для verifier и `pin_matches`) | красный (pin_tls: 4 падения, в т.ч. pin_wrong_pin_rejected_before_first_byte, ws_wrong_pin_no_upgrade) |
| M2 | crates/trackly-app/src/http/mod.rs (`if !valid` в link_guard -> `if false && !valid`) | красный (app_server link_revoke: 5 падений) |
| M3 | crates/trackly-app/src/services/auth.rs (`revoke_link_logins` ранний return) | красный (app_server link_revoke: 3 падения, blocking_user_revokes_logins..., deleting_user_revokes_logins) |
| M4 | crates/link-app/src/authflow.rs (проверка `local.revoked` в `startup_auth` снята) | красный (auth_flow test5_revoked_blocks_sso_until_password_login_accepted) |
| M5 | crates/link-app/src/enroll.rs (запись конфига до ответа сервера) | красный (enroll_flow: test2, test4, test5 — «ничего не сохранено») |

Вакуумных тестов нет. Отклонение от формулировки плана: у `pin_matches` нет вызовов вне теста (verifier сравнивает через `ct_eq`), поэтому M1 применён к `ct_eq` — он ловит оба пути.

## Задача 2: гейты

| Гейт | Результат |
|------|-----------|
| `cargo fmt --all -- --check` | зелёный |
| `cargo clippy --workspace --all-targets -- -D warnings` | зелёный |
| `pnpm --dir ui lint` / `ui svelte-check` | зелёные |
| `pnpm --dir link-ui lint` / `link-ui svelte-check` | зелёные |
| `pnpm --dir ui build` / `link-ui build` | зелёные (свежий ui/dist) |
| check-privacy (с `--hashes`) | PASS, 0 нарушений |
| `cargo deny check` | не установлен (cargo-deny отсутствует на машине) |
| осиротевшие cargo-процессы | нет |

### ci-full

Эквивалент `cargo test --workspace --no-fail-fast -- --test-threads=1` с `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1`, выполнен в foreground по крейтам/бинарникам (каждая команда < 10 мин, фактически секунды-минута; сплошной прогон на этой машине теперь быстрый). Всё 0 failed:

trackly-core 201+1+3; link-proto 9+6 (1 ignored — генератор фикстуры); link-app 48 lib + 2 bin + 17 pin_tls + 9 auth_flow + 21 commands_contract + 7 connection_loop + 17 enroll_flow + 5 state_contract... (фактически по бинарникам: 48, 2, 17, 9, 21, 7, 17, 5); trackly-infra 177 + 307 (infra_tests) + 2 ignored; trackly-app lib 464; app_gates 291; app_auth 138; app_acts 346; app_attachments 78; app_cartridges 89; app_chat 50; app_devices 292; app_groups 201; app_places 91; app_repairs 142; app_reports 179; app_server 142; прочие (stand_ui_move, export_bindings, health_smoke, phase06_stubs 18, template_edit 6, templates_status 3, attachments_heic 2) зелёные. Не запускалось: `cargo build --release -p trackly-app` (шаг CI) и процесс procmon на Windows.

Link-наборы непустые: link_service 12, link_admin 18, link_enroll 5, link_auth 7, link_revoke 11, link_revoked_device 2 (в app_server), link_migration 4 (infra), link-app/link-proto — см. выше.

Красных тестов не было; в этом плане исправлять код не потребовалось. Сырые выводы: `/private/tmp/claude-501/ci/*.txt`.

### 44-VALIDATION.md

Все строки карты ✅ green (команды прошли; шаблоны фильтров вроде `net::pin` заменены фактическими наборами pin_tls/auth_flow/connection_loop), Wave 0 отмечен целиком, `nyquist_compliant: true`, `wave_0_complete: true`. **`status` оставлен `draft`**, Approval pending: Manual-Only проверки (SSPI, трей, автозапуск, RDS, GNOME, keyring) ещё не выполнены человеком.

## Задача 3: живая проверка — awaiting human

Статус по пунктам (все «не проверено», пока человек не подтвердил): 1–11, C, D — **не проверено**. Отметки ставит только исполнявший шаг; пересказ не засчитывается. Чеклист — в отчёте CHECKPOINT REACHED.

## Авто-одобренные чекпоинты

Нет: единственный чекпоинт (задача 3) blocking-human, не одобрялся.

## Deviations from Plan

1. ci-full выполнен не run_in_background, а в foreground, разбитым по бинарникам (указание оркестратора).
2. Команда privacy в плане без `--hashes` не работает; использован `--hashes scripts/privacy-tokens.sha256` (как в CI).
3. M1 на `ct_eq`, а не на `pin_matches` (см. выше).
4. `cargo deny` не установлен — не выполнялся.

## Self-Check: PASSED (задачи 1–2)
