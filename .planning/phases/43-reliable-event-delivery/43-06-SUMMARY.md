---
phase: 43-reliable-event-delivery
plan: 06
subsystem: api
tags: [idempotency, chat, requests, specta, rusqlite]
requires:
  - phase: 43-01
    provides: "V059 частичные UNIQUE-индексы client_msg_id / client_request_id, is_valid_client_id"
  - phase: 43-04
    provides: "образец идемпотентной отправки (replay до записи, UNIQUE как страховка)"
provides:
  - "ChatService::send(.., client_msg_id): повтор возвращает то же сообщение"
  - "RequestService::create с RequestCreateDto.client_request_id: повтор возвращает ту же заявку"
affects: [43-09, 43-10]
tech-stack:
  added: []
  patterns: ["replay до проверки статуса и до scope-проверок", "поиск повторяется в замыкании писателя; конфликт UNIQUE при UPDATE = replay"]
key-files:
  created:
    - crates/trackly-app/tests/chat_idempotent_send.rs
    - crates/trackly-app/tests/request_idempotent_create.rs
  modified:
    - crates/trackly-infra/src/repos/chat_sqlite.rs
    - crates/trackly-infra/src/repos/requests_sqlite.rs
    - crates/trackly-app/src/services/chat_service.rs
    - crates/trackly-app/src/services/request_service.rs
    - crates/trackly-app/src/dto/request.rs
    - crates/trackly-app/src/http/chat.rs
    - crates/trackly-app/src/tauri_cmds/chat.rs
    - crates/trackly-app/tests/suites/chat.rs
    - crates/trackly-app/tests/suites/server.rs
key-decisions:
  - "D-11: replay-поиск стоит до проверки статуса заявки (чат) и до allowlist/scope (заявки); id задаёт идентичность, текст повтора игнорируется"
  - "insert_message_on и insert_in_tx не менялись: id проставляется UPDATE'ом в той же транзакции (у insert_in_tx много вызывающих, включая AuthService)"
  - "client_request_id в RequestCreateDto: serde(default) + specta(optional), TS-тип clientRequestId?: string | null; существующие UI-вызовы не правились"
requirements-completed: [DLV-03]
duration: ~50 мин
completed: 2026-11-27
---

# Phase 43 Plan 06: идемпотентность chat_send и requests_create Summary

Повтор отправки сообщения чата и повтор создания заявки с тем же клиентским id возвращают уже созданную сущность без дубля строки, без второго audit_log и без повторного WsEvent; поле необязательно, старые клиенты работают как прежде.

## Задачи

1. Идемпотентный `ChatService::send` + транспорты (HTTP `SendPayload.clientMsgId`, Tauri `chat_send`) + тесты — `f87559e4`
2. Идемпотентный `RequestService::create` + DTO + тесты + проверка биндингов — `a03ae3e6`

## Проверки

- `cargo test -p trackly-app --test app_chat -- idempot chat_send` — 13/13 (5 новых + 8 существующих).
- `cargo test -p trackly-app --test app_server -- request_idempotent_create request_lifecycle` — зелёный (6 новых).
- `cargo test -p trackly-app --test export_bindings` — зелёный; `ui/src/bindings.ts` в gitignore (diff недоступен), сгенерированный файл содержит `clientRequestId?: string | null` и `clientMsgId: string | null` в `chatSend`.
- `pnpm --dir ui exec svelte-check` — 0 ошибок, 0 предупреждений (UI вызывает через `apiCall` с динамическим телом, отсутствующее поле = `None`).
- `cargo test -p trackly-app --test app_gates -- specta_roundtrip` — 1/1.
- `cargo clippy -p trackly-app -p trackly-infra --all-targets -- -D warnings`, `cargo fmt --check`, `cargo check --workspace --all-targets` — чисто.
- `promote_drafts_on` вызывается только на пути Sent (одна строка в chat_service.rs).

## Мутационные проверки (исходник восстановлен после каждой)

Чат:
- Событие `ChatMessage` и при replay — краснеет `idempotent_send_replays_without_duplicate_or_event`.
- Replay-поиск без `request_id` в SQL (`?1 = ?1`) — краснеет `key_includes_author_and_request`. Первая попытка (подстановка константы 0 вместо id заявки) тест «другая заявка» не краснила: константа даёт «не найдено», что для другой заявки и нужно; краснела проба «закрытая заявка», подтверждая и порядок «replay до статуса». Мутация «убрать фильтр» показала что тест несущий.
- Перенос поиска после проверки статуса отдельно не гонялся; порядок закреплён пробой `replay_after_request_closed_returns_message_new_id_rejected` (фикстуры различаются: новый id в закрытой заявке — `closed`), и она краснела при поломке поиска.

Заявки:
- Audit при replay — краснеют `replay_returns_same_request_one_row_one_audit_one_event` и `concurrent_creates...`.
- `NewRequest` при replay в писателе — краснеет `concurrent_creates_with_same_id_collapse_to_one`.
- Убран поиск в писателе вместе с UNIQUE-страховкой — тест гонки краснеет (ошибка Conflict).
- Убран только поиск в писателе, страховка оставлена — зелёный: конфликт UNIQUE превращается в replay, а не в ошибку (так и задумано, фиксируем результат).

## Отклонения от плана

**1. [Rule 1 - тест] Тест гонки через `tokio::join!` оказался вакуумным.** Обе мутации писателя оставались зелёными: второй вызов всегда находил заявку ранним поиском. Заменён на 16 параллельных `tokio::spawn` с одним id; после этого мутации краснеют. Урок: две корутины в `join!` гонку не создают.

**2. [Rule 3 - блокер] 25 литералов `RequestCreateDto {..}` в тестах** дополнены `client_request_id: None` (поле необязательное только на уровне serde/TS, в Rust конструктор требует его).

**3. Вложения replay-сообщения.** Загружаются в замыкании писателя тем же репозиторием, что и лента (`list_message_attachments_on`, только неудалённые, `from_message = true`). Для replay берётся актуальный `fresh`-доступ, как и при обычной отправке.

**4. Биндинги.** `ui/src/bindings.ts` не отслеживается git, поэтому «diff только clientRequestId» подтверждён чтением сгенерированного файла, а не `git diff`.

## Known Stubs

Нет.

## Threat Flags

Нет новых поверхностей: ключи включают автора (и заявку для чата), авторизация `load_access` / `authorize` выполняется до replay, id валидируется `is_valid_client_id`, запросы параметризованные (T-43-06-01..04 закрыты).

## Self-Check: PASSED

Оба тестовых файла существуют, коммиты f87559e4 и a03ae3e6 в `git log`.
