---
phase: 43-reliable-event-delivery
plan: 04
subsystem: api
tags: [im, hub, sync, idempotency, rusqlite, broadcast]
requires:
  - phase: 43-01
    provides: "V058/V059, domain::im, SqliteImRepository"
provides:
  - "ImHub/ImSignal: канал сигналов на пользователя"
  - "ImService: send (идемпотентность), sync (курсор по rev), create_conversation/add_member/remove_member"
  - "DTO im_send/im_sync; AppCtx.im и AppCtx.im_hub"
affects: [43-08, 47]
tech-stack:
  added: []
  patterns: ["публикация сигнала после .await? и вне замыкания писателя", "окно исключённого seq/rev <= left_seq, sync без выборки rev > cursor"]
key-files:
  created:
    - crates/trackly-app/src/services/im_hub.rs
    - crates/trackly-app/src/services/im_service.rs
    - crates/trackly-app/src/dto/im.rs
    - crates/trackly-app/tests/im_support/mod.rs
    - crates/trackly-app/tests/im_membership.rs
    - crates/trackly-app/tests/im_idempotent_send.rs
    - crates/trackly-app/tests/im_sync.rs
  modified:
    - crates/trackly-app/src/context.rs
    - crates/trackly-app/src/services/mod.rs
    - crates/trackly-app/src/dto/mod.rs
    - crates/trackly-app/src/http/health.rs
    - crates/trackly-app/src/tauri_cmds/health.rs
    - crates/trackly-app/tests/suites/server.rs
key-decisions:
  - "D-04: seq/rev, replay-поиск, получатели — в одной транзакции писателя; публикация после коммита, при replay её нет"
  - "D-06: исключённый получает замороженную историю один раз (reload=true, read_only=true, lastSeq=left_seq); изменённое после выхода сообщение не отдаётся — передано Фазе 47 (снимок ревизии или надгробие), комментарий в коде sync"
  - "add_member: новый и вернувшийся участник видят беседу с last_seq+1 (в плане это было задано только для вернувшегося; для нового выбрано то же консервативное правило)"
requirements-completed: [DLV-01, DLV-02, DLV-03]
duration: ~45 мин
completed: 2026-11-27
---

# Phase 43 Plan 04: ядро доставки IM на сервере Summary

ImHub (std Mutex, канал 256 на пользователя), ImService с идемпотентной отправкой, составом беседы и курсорным sync; членство строгое (админ без членства получает NotFound), исключённый видит замороженную историю.

## Задачи

1. ImHub/ImSignal, DTO, `im_hub` в AppCtx, 5 юнит-тестов (Lagged, точный JSON, ленивые каналы) — `d5e158dc`
2. ImService: send, create_conversation, add_member, remove_member; тесты im_membership (6) и im_idempotent_send (3) — `1d82d33d`
3. ImService::sync; тесты im_sync (6) — `992afbb3`

## Проверки

- `cargo test -p trackly-app --lib services::im_hub` — 5/5.
- `cargo test -p trackly-app --test app_server -- im_sync im_membership im_idempotent_send` — 15/15.
- `cargo clippy -p trackly-app --all-targets -- -D warnings` и `cargo fmt -p trackly-app --check` — чисто. unwrap вне тестов в im_service.rs нет; dashmap не добавлен; printer.rs (WsEvent) не тронут.

## Мутационные проверки (исходник восстановлен после каждой)

- Получатели из ВСЕХ членов вместо активных — краснеет `removed_member_stops_receiving`.
- Убран replay-поиск: тест остаётся зелёным, пока работает запасной путь по UNIQUE-конфликту; при отключении обоих — краснеет `replay_returns_original_without_row_or_signal`.
- Публикация сигнала при replay — краснеет тот же тест.
- Снято `rev <= left_seq` в окне исключённого — краснеет `edit_after_leaving_is_not_delivered`.
- Ветка `rev > cursor` для исключённого — краснеют `edit_after_leaving...` и `removed_member_gets_frozen_history_once`.
- Выборка бесед без фильтра по членству (SQL репозитория) — краснеют 4 теста, включая `foreign_conversation_is_not_disclosed`.

## Отклонения от плана

**1. [Rule 3 - блокер] Литералы `AppCtx {..}` в 8 местах** (http/health.rs, tauri_cmds/health.rs и 6 тестов) не собирались с новыми полями `im_hub`/`im`; поля добавлены, `im` собирается до литерала из клонов writer/readers/clock.

**2. [Rule 1 - тест] Правка в `tamper` двигает `last_seq`.** Настоящая правка поднимает счётчик беседы; без этого sync активного участника (курсор == last_seq) закономерно пропускал беседу.

**3. Наблюдение.** При проверке мутации я подменил SQL sed'ом и вернул `mv` — mtime файла остался старым, cargo пересобрал не всё и показал ложные красные тесты; после `touch` прогон 15/15. Исходник был чист (git status).

## Known Stubs

Нет. Маршруты send/sync наружу — План 08 (D-02).

## Threat Flags

Нет новых поверхностей: сервис в транспорты пока не подключён.

## Self-Check: PASSED

Файлы из key-files существуют, коммиты d5e158dc, 992afbb3 и коммит ImService в `git log`.
