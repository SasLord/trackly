---
phase: 43-reliable-event-delivery
plan: 13
subsystem: im-sync
tags: [gap-closure, im_sync, im_send, adr]
requires: []
provides:
  - "read-only сводка исключённому на каждый im_sync (CR-01)"
  - "дельта исключённого в окне видимости (WR-01)"
  - "replay im_send до can_post (WR-06)"
affects: [44, 47]
key-files:
  modified:
    - crates/trackly-app/src/services/im_service.rs
    - crates/trackly-infra/src/repos/im_sqlite.rs
    - crates/trackly-infra/tests/im_repo.rs
    - crates/trackly-app/tests/im_sync.rs
    - crates/trackly-app/tests/im_idempotent_send.rs
    - crates/trackly-app/tests/im_membership.rs
    - docs/adr/0003-chat-model-fts-delivery.md
key-decisions:
  - "Признак «знаю о выходе» в курсор не вводится: ImSyncRequestDto не меняется (контракт Link, Фаза 44)"
  - "Окно member_window(.., Some(left_seq)) ограничивает и дельту: утечки правок и сообщений после выхода нет (D-05)"
  - "replay раньше can_post, но после проверки строки участника (404 сохранён)"
requirements-completed: [DLV-02, DLV-03]
completed: 2026-11-27
---

# Phase 43 Plan 13: исключённый участник и replay im_send Summary

Исключённый участник получает read-only сводку беседы на каждый im_sync, дельту только внутри окна видимости, а повтор im_send после исключения автора возвращает исходное сообщение.

## Что сделано
- **CR-01:** курсор >= left_seq даёт сводку read_only:true, leftSeq, messages пуст, reload:false вместо пропуска беседы. RED зафиксирован: тест `caught_up_then_removed_gets_read_only_summary` падал до правки.
- **WR-01:** курсор < left_seq даёт дельту rev > cursor в окне (reload:false), при превышении SYNC_RELOAD_THRESHOLD или без курсора - последняя страница окна с reload:true.
- **WR-06:** в send() порядок: строка участника (404) -> find_by_client_id_on (replay) -> can_post (403) -> запись.
- **IN-01:** is_unique_conflict переименован в is_constraint_conflict. **IN-02:** комментарий defence-in-depth в im_service::send.
- **ADR 0003:** разделы 6, 7, 8 и 11 приведены к коду.

## Тесты
- `cargo test -p trackly-app --test app_server -- im_sync`: 10 passed.
- `-- im_idempotent_send im_membership`: 10 passed.
- `cargo test -p trackly-infra --test infra_tests -- im_repo`: 7 passed.
- `cargo fmt --all -- --check`: чисто. Clippy и широкий прогон - в плане 43-21.

## Deviations from Plan
**1. [Rule 1 - устаревший контракт в тестах]** `im_membership::removed_member_cannot_send` утверждал «replay тоже 403», а `removed_member_gets_frozen_history_once` - «курсор = left_seq даёт пустой ответ» и «старый курсор даёт полную перезагрузку». Утверждения обновлены под D-06/D-11 из плана.

**2. Коммиты задач 1 и 2 объединены** в один (b6ad061f): обе правки в одном im_service.rs, раздельно проиндексировать без интерактивного `git add -p` нельзя. ADR - отдельным коммитом (cfc029e6).

Не делались по плану: IN-04 (Фаза 47), IN-05, IN-08.

## Commits
- b6ad061f: fix(43-13): read-only сводка исключённому, дельта в окне, replay im_send до can_post
- cfc029e6: docs(43-13): ADR 0003 разделы 6-8 и 11 согласованы с кодом

## Known Stubs
Нет.

## Self-Check: PASSED
