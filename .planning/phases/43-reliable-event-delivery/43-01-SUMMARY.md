---
phase: 43-reliable-event-delivery
plan: 01
subsystem: database
tags: [sqlite, migrations, im, idempotency, rusqlite]
requires: []
provides:
  - "V058: таблицы im_conversations / im_members / im_messages"
  - "V059: client_msg_id / client_request_id с частичными UNIQUE-индексами"
  - "domain::im: чистые правила (client id, документ-обёртка, окно видимости, пагинация)"
  - "SqliteImRepository: SQL ядра im_* на &Connection"
affects: [43-04, 43-06, 43-08, 47]
tech-stack:
  added: []
  patterns: ["окно видимости исключённого: seq <= left_seq И rev <= left_seq", "seq/rev одним UPDATE ... RETURNING"]
key-files:
  created:
    - migrations/V058__im_core.sql
    - migrations/V059__idempotency_keys.sql
    - crates/trackly-core/src/domain/im.rs
    - crates/trackly-infra/src/repos/im_sqlite.rs
    - crates/trackly-infra/tests/im_migration.rs
    - crates/trackly-infra/tests/im_repo.rs
  modified:
    - crates/trackly-core/src/domain/mod.rs
    - crates/trackly-infra/src/repos/mod.rs
    - crates/trackly-infra/tests/suites/infra.rs
key-decisions:
  - "D-06: left_seq и left_at_utc ставятся вместе; окно исключённого ограничено и по seq, и по rev"
  - "D-04: bump_last_seq_on — единственный путь выдачи seq/rev"
requirements-completed: [DLV-02, DLV-03]
duration: ~25 мин
completed: 2026-11-26
---

# Phase 43 Plan 01: ядро im_* и ключи идемпотентности Summary

Миграции V058/V059, чистые правила `domain/im.rs` и репозиторий `im_sqlite.rs` с невакуумными тестами; окно видимости исключённого участника закодировано и проверено мутацией.

## Задачи

1. V058 (ядро im_*) и V059 (ключи идемпотентности) + тесты миграций — `f62cf5c9`
2. `domain/im.rs`: is_valid_client_id, body_doc_json, dialog_key, clamp_page_limit, member_window, can_post (8 юнит-тестов) — `bdac6ae4`
3. `SqliteImRepository` + 7 тестов репозитория — `87557ad7`

## Проверки

- `cargo test -p trackly-infra --test infra_tests im_` — 12/12 зелёные (4 миграции + 7 репозиторий + 1 посторонний тест с «im_» в имени).
- `cargo test -p trackly-core --lib domain::im` — 8/8.
- `cargo clippy -p trackly-core / -p trackly-infra --all-targets -- -D warnings` и `cargo fmt --check` — чисто.
- Хардкоды user_version: `grep user_version` — проверки везде `>=` (desktop_system_admin >= 53, миграционные тесты >= N) или относятся к `run_up_to`; равенство `== 57` не найдено, править нечего.
- В SQL `format!` используется только с константами (`MSG_COLS`, `WINDOW_SQL`); пользовательские значения идут через параметры.

## Мутационные проверки

- Убрана `rev <= max_rev` из `WINDOW_SQL`: `window_hides_edits_after_leaving` краснеет; исходник возвращён.
- V059: индекс заменён на неуникальный — `v059_partial_unique_indexes` краснеет (тест несущий).

## Отклонения от плана

**1. [Наблюдение, не правка] Мутация «убрать WHERE client_msg_id IS NOT NULL» не краснит тест NULL.**
В SQLite два NULL в UNIQUE-индексе всегда различны, поэтому `WHERE ... IS NOT NULL` на допуск NULL не влияет (он лишь уменьшает индекс). Вместо этого проверена мутация «UNIQUE INDEX -> INDEX», которая тест краснит. Миграция возвращена, `diff` с копией пуст. Частичный индекс оставлен по плану.

Иных отклонений нет. Каждая миграция и тестовый файл создавались по плану; `im_repo.rs` сначала подключён заглушкой в задаче 1 (чтобы собрался suite), наполнен в задаче 3.

## Known Stubs

Нет.

## Threat Flags

Нет новых поверхностей (репозиторий пока не подключён к транспортам).

## Self-Check: PASSED

Файлы миграций, `im.rs`, `im_sqlite.rs`, тесты и коммиты f62cf5c9, bdac6ae4, 87557ad7 существуют.
