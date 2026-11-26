# Фаза 43: Надёжная доставка событий - Карта паттернов

**Составлено:** 2026-11-26
**Файлов в анализе:** 34 (новые и изменяемые)
**Аналоги найдены:** 31 / 34 (3 без точного аналога - см. «Аналога нет»)

Все примеры в тестах - вымышленные имена («Иванов И.И.», «Петров П.П.»). Реальных данных организации в документе нет.

## Сверка имён тестовых целей (research помечал LOW)

`autotests = false` в обоих крейтах. Цели ТОЛЬКО такие (проверено по `[[test]]`):

**crates/trackly-app/Cargo.toml**

| Цель (`--test`) | Файл |
|-----------------|------|
| `app_server` | `tests/suites/server.rs` (WS, TLS, заявки, shutdown, fanout) |
| `app_chat` | `tests/suites/chat.rs` (чат заявок) |
| `app_gates` | `tests/suites/gates.rs` (гейты; ПРОВЕРЕНО: сюда подключены `http_route_parity` и `suite_wiring`) |
| `app_auth`, `app_acts`, `app_attachments`, `app_cartridges`, `app_devices`, `app_groups`, `app_places`, `app_repairs`, `app_reports` | остальные suite |
| отдельные: `stand_ui_move`, `attachments_heic`, `export_bindings`, `health_smoke`, `phase06_stubs`, `template_edit`, `templates_status` | свои файлы |

**crates/trackly-infra/Cargo.toml**: одна цель `infra_tests` -> `tests/suites/infra.rs`.

Следствие: команды из research `--test server ...` НЕВЕРНЫ. Правильно:
- серверные WS/IM-тесты: `cargo test -p trackly-app --test app_server <фильтр>`
- идемпотентность чата: `cargo test -p trackly-app --test app_chat idempot`
- идемпотентность заявки: `--test app_server` (рядом с `request_lifecycle`)
- миграции: `cargo test -p trackly-infra --test infra_tests im_migration`
- паритет/suite_wiring: `cargo test -p trackly-app --test app_gates http_route_parity` / `suite_wiring` (подключены в `suites/gates.rs:48,56`)

Куда подключать новые тесты (строки `#[path = "../X.rs"] mod X;`, алфавитный порядок, шапка `#![allow(clippy::duplicate_mod)]` уже есть):

| Новый файл теста | Suite |
|------------------|-------|
| `tests/ws_heartbeat.rs` | `suites/server.rs` |
| `tests/ws_resync_required.rs` | `suites/server.rs` |
| `tests/im_ws_fanout.rs` | `suites/server.rs` |
| `tests/im_membership.rs` | `suites/server.rs` |
| `tests/im_sync.rs` | `suites/server.rs` |
| `tests/im_http.rs` | `suites/server.rs` |
| `tests/request_idempotent_create.rs` | `suites/server.rs` (рядом с `request_lifecycle`) |
| `tests/chat_idempotent_send.rs` | `suites/chat.rs` (рядом с `chat_send`) |
| `tests/desktop_resync_bridge.rs` (ветка Lagged моста) | `suites/chat.rs` либо `server.rs`; аналог `chat_desktop_bridge.rs` лежит в `suites/chat.rs` - класть туда же |
| `crates/trackly-infra/tests/im_migration.rs` | `suites/infra.rs` (рядом с `chat_migration`, `history_events_repo`) |
| `crates/trackly-infra/tests/im_repo.rs` (опц.) | `suites/infra.rs` |

Хелперы IM-тестов: `tests/im_support/mod.rs` по образцу `tests/chat_support/mod.rs` (в тесте `mod im_support;`).

## Классификация файлов

| Новый/изменяемый файл | Роль | Поток данных | Ближайший аналог | Качество |
|-----------------------|------|--------------|------------------|----------|
| `migrations/V058__im_core.sql` | migration | CRUD | `migrations/V054__request_chat.sql` | точный |
| `migrations/V059__idempotency_keys.sql` | migration | CRUD | `V054__request_chat.sql` (ADD COLUMN + INDEX) | роль |
| `crates/trackly-core/src/domain/im.rs` | model (чистые правила) | transform | `crates/trackly-core/src/domain/chat.rs` (`normalize_text`, `exceeds_limit`) | роль |
| `crates/trackly-infra/src/repos/im_sqlite.rs` | repo | CRUD | `repos/chat_sqlite.rs` (`*_on(&Connection)`) | точный |
| `crates/trackly-app/src/services/im_service.rs` | service | CRUD, event-driven | `services/chat_service.rs::send` | точный |
| `crates/trackly-app/src/services/im_hub.rs` | service/hub | pub-sub | `ctx.ws_broadcast` (broadcast) + `entities_broadcast.rs` | частичный |
| `crates/trackly-app/src/http/im.rs` | route | request-response | `http/chat.rs` | точный |
| `crates/trackly-app/src/http/mod.rs` | route (merge) | - | строки `.merge(...)` ~176-185 | точный |
| `crates/trackly-app/src/http/ws.rs` | controller (WS) | streaming | сам файл (ветки `select!`) | точный |
| `crates/trackly-app/src/context.rs` | config (AppCtx) | - | поле `ws_broadcast` (стр. 112, 234) | точный |
| `crates/trackly-app/src/main.rs` (мост, ~296-330) | provider | event-driven | сам файл + `desktop_event_bridge.rs` | точный |
| `crates/trackly-app/src/desktop_event_bridge.rs` | utility | event-driven | сам файл | точный |
| `crates/trackly-app/src/services/chat_service.rs` (send) | service | CRUD | сам файл | точный |
| `crates/trackly-app/src/services/request_service.rs` (create) | service | CRUD | `chat_service::send` (replay-ветка) | роль |
| `crates/trackly-app/src/dto/request.rs`, `dto/chat.rs` | model (DTO) | request-response | сами файлы | точный |
| `crates/trackly-app/src/http/chat.rs`, `tauri_cmds/chat.rs` | route/command | request-response | сами файлы | точный |
| `crates/trackly-app/src/dto/im.rs` (DTO im_send/im_sync) | model | request-response | `dto/chat.rs` | роль |
| `crates/trackly-app/tests/http_route_parity.rs` | test (гейт) | - | запись `ws` в `ROUTE_WITHOUT_CMD` (стр. 181-193) | точный |
| `ui/scripts/fixtures/employee-allowed-commands.json` + тест `employee_allowed_fixture_sound` | config/test | - | сами файлы | точный |
| `packages/transport/src/ws.ts` | service (клиент) | streaming | сам файл | точный |
| `packages/transport/src/sendWithRetry.ts` (нов.) | utility | request-response | нет (см. «Аналога нет») | - |
| `ui/scripts/check-ws-disconnect-toast.mjs` (расширить) | test (гейт) | - | сам файл | точный |
| `ui/scripts/check-ws-resync-registry.mjs` (нов.) | test (гейт) | - | `check-transport-hooks.mjs`, `check-ws-disconnect-toast.mjs` | роль |
| `ui/scripts/check-ws-resync.mjs` (нов., исполняющий) | test (гейт) | - | `check-ws-disconnect-toast.mjs` (исполняет транспилированный ws.ts) | роль |
| `ui/package.json` (цепочка `lint`) | config | - | строка 16, пары `--selftest` + боевой запуск | точный |
| 13 подписчиков `onWsEvent` (список в RESEARCH) | component | event-driven | каждый сам; шаблон - `RequestChat.svelte:243` | точный |
| `ChatComposer.svelte`, `chat/api.ts`, `RequestFormModal.svelte`, `requests/api.ts` | component | request-response | сами файлы | точный |
| `docs/adr/0003-chat-model-fts-delivery.md` | doc | - | сам файл | точный |
| Тесты Rust (список выше) | test | streaming/CRUD | `chat_ws_fanout.rs`, `chat_send.rs`, `chat_migration.rs`, `history_events_repo.rs` | точный |

## Назначение паттернов

### `migrations/V058__im_core.sql`, `V059__idempotency_keys.sql` (migration)

**Аналог:** `migrations/V054__request_chat.sql` (стр. 1-44), `V057__history_events.sql` (последняя строка).

Правила (шапка V054, стр. 1-5): только CREATE TABLE / CREATE INDEX / ALTER TABLE ADD COLUMN; ни DROP, ни перестройки (урок V042: `foreign_keys=OFF` в refinery-транзакции не действует). Без FK у «лёгких» таблиц допустимо (V054: «запись переживает удаление пользователя»), но для `im_members`/`im_messages` решать осознанно.

```sql
CREATE TABLE chat_messages (
  id                     INTEGER PRIMARY KEY AUTOINCREMENT,
  request_id             INTEGER NOT NULL,
  author_user_id         INTEGER NULL,
  body                   TEXT    NOT NULL DEFAULT '',
  created_at_utc         INTEGER NOT NULL,
  moved_from_request_id  INTEGER NULL
);
CREATE INDEX idx_chat_messages_request ON chat_messages (request_id, id);
ALTER TABLE attachments ADD COLUMN message_id INTEGER NULL;
CREATE INDEX idx_attachments_message ON attachments (message_id);
```

Последняя строка файла ОБЯЗАНА быть `PRAGMA user_version = 58;` (59 для V059), иначе ~10 красных в `ci-full`. V059: `ALTER TABLE ... ADD COLUMN client_msg_id TEXT NULL;` + `CREATE UNIQUE INDEX ... WHERE client_msg_id IS NOT NULL` (`ADD COLUMN ... UNIQUE` в SQLite запрещён). Ключи идемпотентности: `(author_user_id, client_msg_id)` для `chat_messages`, `(requested_by_user_id, client_request_id)` для `requests`.

Тест на «последняя строка» - скопировать форму (`crates/trackly-infra/tests/history_events_repo.rs:60-71`):

```rust
#[test]
fn v057_file_ends_with_user_version_57() {
    let sql = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../migrations/V057__history_events.sql"
    ))
    .unwrap();
    assert_eq!(
        sql.trim_end().lines().last().unwrap(),
        "PRAGMA user_version = 57;"
    );
}
```

Проверить хардкоды ожидаемой версии: `tests/downgrade_protection.rs`, `tests/desktop_system_admin.rs:180` (`v >= 53`), `acts_clone_handover.rs` (используют `>=`, обычно не ломаются); `context.rs`, `dto/health.rs` - `grep -rn user_version crates --include='*.rs'`. `build.rs` уже пересобирает крейт при новом .sql.

### `crates/trackly-infra/src/repos/im_sqlite.rs` (repo, CRUD)

**Аналог:** `crates/trackly-infra/src/repos/chat_sqlite.rs`.

Импорты и форма (стр. 1-40, 111-129): нулевой по размеру репозиторий, методы `*_on(&self, conn: &Connection, ...)` (`Transaction` разыменовывается), параметризованные запросы, `map_rusqlite`.

```rust
use rusqlite::{params, Connection, OptionalExtension, Row};
use trackly_core::error::AppError;
use crate::error_conversions::map_rusqlite;

#[derive(Debug, Default, Clone)]
pub struct SqliteChatRepository;

pub fn insert_message_on(&self, conn: &Connection, n: &ChatMessageNew<'_>) -> Result<i64, AppError> {
    conn.execute("INSERT INTO chat_messages (...) VALUES (?1, ?2, ?3, ?4, ?5)", params![...])
        .map_err(map_rusqlite)?;
    Ok(conn.last_insert_rowid())
}
```

Применить: `find_by_client_id_on`, `bump_last_seq_on` (`UPDATE ... SET last_seq = last_seq + 1 ... RETURNING last_seq`), `active_members_on`, `insert_message_on`. Зарегистрировать модуль в `repos/mod.rs` (сверить по соседям).

### `crates/trackly-app/src/services/im_service.rs` (service, CRUD + event-driven)

**Аналог:** `services/chat_service.rs::send` (стр. ~440-560).

Ключевые приёмы для копирования: (1) валидация и нормализация ДО замыкания (`normalize_text`, `exceeds_limit`, reject-коды как `Ok`); (2) всё состояние-зависимое (статус/членство) читается ВНУТРИ замыкания писателя (комментарий «TOCTOU: the status is read here, in the writer, not before»); (3) enum результата замыкания (`Written::Closed | Written::Sent`) - добавить `Written::Replay`; (4) публикация только после `.await?`.

```rust
let written = self
    .writer
    .execute(move |conn| {
        let tx = conn.transaction().map_err(map_rusqlite)?;
        // TOCTOU: the status is read here, in the writer, not before.
        let status: Option<(String, Option<i64>)> = { use rusqlite::OptionalExtension; tx.query_row(
            "SELECT status, deleted_at_utc FROM requests WHERE id = ?1", params![request_id],
            |r| Ok((r.get(0)?, r.get(1)?))).optional().map_err(map_rusqlite)? };
        match status {
            Some((st, None)) if can_post_status(&st) => {}
            Some((_, _)) => return Ok(Written::Closed),
            None => return Err(AppError::NotFound { entity: "request", id: request_id }),
        }
        let message_id = repo.insert_message_on(&tx, &ChatMessageNew { .. })?;
        ...
        tx.commit().map_err(map_rusqlite)?;
        Ok(Written::Sent(row, promoted))
    })
    .await?;
// D-23: only after the writer returned Ok
if let Some(tx) = &self.ws_tx { let _ = tx.send(WsEvent::ChatMessage { .. }); }
```

Порядок в замыкании `im_send`: replay-поиск -> членство (`left_at_utc IS NULL`) -> `last_seq+1` -> INSERT (`seq = rev = last_seq`) -> получатели из `im_members` -> commit. 404 для чужой беседы (не раскрывать существование). Сервисные `create_conversation/add_member/remove_member` - те же правила, получатели считываются ПОСЛЕ изменения состава в той же транзакции (D-05).

Правило «публикация после записи, вне замыкания» (D-04) - `services/entities_broadcast.rs` стр. 1-12 (шапка контракта) и стр. 24-58 (`send_entities_changed`, best-effort: `None` отправитель = тихий пропуск, `let _ = tx.send(..)`). Если для IM нужен реестровый гейт «мутация -> рассылка» - сверить `chat_changed_gate.rs`, `chat_audit_signal_inventory.rs` (гейт читает исходники сервисов по имени метода и литеральной метке).

### `crates/trackly-app/src/services/im_hub.rs` (hub, pub-sub)

**Аналог:** `ctx.ws_broadcast` (`context.rs:112`, создание `:234`):

```rust
pub ws_broadcast: Arc<tokio::sync::broadcast::Sender<WsEvent>>,
let (ws_tx, _) = tokio::sync::broadcast::channel::<WsEvent>(128);
```

Скелет по RESEARCH Pattern 3 (`std::sync::Mutex<HashMap<i64, broadcast::Sender<ImSignal>>>`, ёмкость 256, без `dashmap`). Поле `pub im_hub: Arc<ImHub>` добавить в `AppCtx` рядом с `ws_broadcast`; инициализация в `AppCtx::new` рядом со стр. 234-235. Сервис получает хаб через `.with_im_hub(...)` по образцу `.with_ws_tx(ws_broadcast.clone())` (стр. 243, 323...).

### `crates/trackly-app/src/http/im.rs` (route, request-response)

**Аналог:** `crates/trackly-app/src/http/chat.rs` (стр. 1-60).

```rust
use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use tower_sessions::Session;
use crate::context::AppCtx;
use crate::error_axum::AppErrorResponse;
use crate::http::auth::session_identity;

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendPayload {
    pub request_id: i64,
    pub text: String,
    #[serde(default)]
    pub draft_attachment_ids: Vec<i64>,
}

pub async fn handler_send(State(ctx): State<AppCtx>, session: Session, Json(p): Json<SendPayload>)
    -> Result<Json<ChatSendOutcomeDto>, AppErrorResponse> {
    let who = session_identity(&session).await?;
    Ok(Json(build_chat_send(&ctx, &who, p.request_id, p.text, p.draft_attachment_ids).await?))
}
```

Хендлер тонкий: идентичность + делегирование сервису; авторизация и WS-логика живут в сервисе (шапка `chat.rs`). Для `im_*` tauri-близнецов нет (чат в десктопе скрыт) - helper `build_im_*` можно держать в `http/im.rs`; в `ROUTE_WITHOUT_CMD` внести `im_send`, `im_sync`. Зарегистрировать: `.merge(im::router())` в `http/mod.rs` рядом со стр. 176-185, и `pub mod im;`.

### `crates/trackly-app/src/http/ws.rs` (WS-цикл, streaming)

**Аналог:** сам файл (стр. 113-182). Менять точечно, СОХРАНИТЬ ветку shutdown и `CLOSE_SEND_TIMEOUT`.

Что заменить:
- `ws.rs:158-162` - `Lagged(n)` с «continuing» -> отправка `{"type":"resync_required"}` (через `timeout(SEND_TIMEOUT, socket.send(..))`, при ошибке `break`).
- `ws.rs:175-177` - `Some(Ok(_)) => {}` -> обновлять `last_seen = Instant::now()` на любом входящем кадре; текстовый `ping` от клиента -> ответ `pong`; прочее игнорировать.
- Добавить ветки `im_rx.recv()` и `ping_tick.tick()` (проверка `idle_timeout`, иначе `Message::Ping`).
- `hello` отправлять до цикла.
- `ws_handler` (стр. 83-92): `Option<Extension<WsTimings>>` по образцу `Option<Extension<ServerShutdown>>`; `ctx.im_hub.subscribe(identity.user_id)`.
- Обновить докблок стр. 10-14 (утверждает «continue»).

Образец тайм-аутированной отправки (стр. 132-136):

```rust
let _ = tokio::time::timeout(CLOSE_SEND_TIMEOUT, socket.send(Message::Close(None))).await;
```

Тайминги инжектируемые (`WsTimings{ping_interval, idle_timeout}`, default 25 с/60 с): в tokio workspace нет `test-util`, `time::pause` недоступен.

### `crates/trackly-app/src/main.rs` (мост десктопа) и `desktop_event_bridge.rs`

**Аналог:** `main.rs:296-330` и `desktop_event_bridge.rs` (стр. 19-35).

```rust
Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
    // Slow consumer — skipped n events. Continue, don't exit (parity with http/ws.rs Pitfall 5).
    tracing::warn!("ws_broadcast->tauri bridge lagged {n} events — continuing");
}
```

Заменить на эмит `app_handle.emit("trackly-resync", ())` (не через `is_visible_to` - это не `WsEvent`). Чтобы код был виден тестам (урок «путь без теста невидим для аудита»): вынести ветку в функцию в `desktop_event_bridge.rs` (например, `pub fn on_lagged(emit: impl Fn())`), вызвать из `main.rs`, тест - по образцу `tests/chat_desktop_bridge.rs`. Комментарий «parity with http/ws.rs Pitfall 5» обновить.

### `chat_service::send` + `request_service::create` (идемпотентность D-11)

**Аналог:** `chat_service.rs:~444-560` (см. фрагмент выше). Добавить параметр `client_msg_id: Option<&str>`; в замыкании сразу после `conn.transaction()` и ДО чтения статуса - replay-поиск `(author_user_id, client_msg_id)`, возврат `Written::Replay(row)` без WS-события и без `promote_drafts_on` (иначе `Closed` заменит replay). Валидация длины/символов id (<=64) - до замыкания.

`request_service::create` (стр. ~345-380): replay-поиск `(requested_by_user_id, client_request_id)` сразу после `authorize(caller, &Action::CreateRequest)?` (стр. 351), раньше allowlist/scope-проверок; найденное вернуть через `self.get(...)`; `WsEvent::NewRequest` при replay НЕ слать. Окончательная гарантия - UNIQUE-индекс, конфликт трактовать как replay.

Цепочка DTO: `RequestCreateDto` (+`client_request_id: Option<String>`), `SendPayload` в `http/chat.rs` (+`#[serde(default)] client_msg_id`), `build_chat_send` (`tauri_cmds/chat.rs:21-31`), Tauri-команда `chat_send`, регенерация specta-биндингов (`tests/export_bindings.rs` — отдельная цель `export_bindings`; `specta_roundtrip`).

### `tests/http_route_parity.rs` (гейт паритета)

**Аналог:** `tests/http_route_parity.rs:181-193`.

```rust
const ROUTE_WITHOUT_CMD: &[Verdict] = &[
    Verdict {
        name: "ws",
        reason: "WebSocket-upgrade для режима сервера (http/ws.rs), method any(); в десктопе \
                 инвалидация идёт внутрипроцессными событиями, команды Tauri здесь нет",
    },
```

Добавить `im_send`, `im_sync` с `reason` >=40 символов (проверка стр. 831-838); устаревшие позиции тоже краснят (стр. 424-430). Сверить карту employee-маршрутов (`ui/scripts/fixtures/employee-allowed-commands.json`) и `role_endpoint_matrix.rs`.

### `packages/transport/src/ws.ts` (клиент, streaming)

**Аналог:** сам файл (288 строк). Точки правок:

- `onWsEvent` (стр. 87-92) - образец для нового `onWsResync(handler): () => void` (тот же стиль: массив + возврат отписки).
- `connectBrowser().onmessage` (стр. 142-148): разбор служебных кадров ДО `dispatch`: `resync_required` -> `resyncAll()`; `hello`/`pong` -> return.

```ts
ws.onmessage = (e) => {
  try { dispatch(JSON.parse(e.data) as WsEvent); } catch { /* Malformed message — ignore. */ }
};
```

- `onopen` (стр. 150-158): добавить `hadOpen`-флаг; ресинк только на повторном открытии (после обрыва). Сейчас ветка `if (reconnecting) {...clearReconnectingToast()}` - сюда же `resyncAll()` и отмена таймера отложенного тоста.
- `onclose` (стр. 160-189): `showReconnectingToast()` вызывается сразу -> заменить на отложенный таймер ~5 с (D-14), хранить id таймера (как `reconnectTimer`, WR-04), отменять в `onopen`, `activeCleanup` (стр. 261-277) и `teardown`. Сохранить: `sticky: true`, один тост за эпизод (`reconnecting`), гарантированный показ при остановке сервера.
- Backoff: добавить `window.addEventListener('online' | 'visibilitychange')` - сброс `reconnectDelay = 1000`, `clearTimeout(reconnectTimer)`, немедленный `connectBrowser()`, если `ws === null`; слушатели ставятся в `connectBrowser`-ветке `connectWs` и снимаются в `activeCleanup` (иначе утечка, класс GAP-12-10).
- Tauri-ветка (стр. 223-228): рядом с `listen('trackly-event', ...)` добавить `listen('trackly-resync', () => resyncAll())`; обе отписки собирать в `activeCleanup` и учитывать `epoch` (WR-02, стр. 229-234).
- `resyncAll()` - дебаунс ~500 мс (Pitfall 6).

### 13 подписчиков `onWsEvent` (D-08)

**Образец для регистрации ресинка:** `ui/src/features/requests/chat/RequestChat.svelte:243` - `onWsEvent` в `onMount`/`$effect` с возвратом отписки; рядом добавить `onWsResync(() => <та же функция перечитывания>)` с общей отпиской. Ресинк перечитывает данные, формы не сбрасывает (D-10). Особо: `chatNotifications.svelte.ts:61` - ресинк НЕ порождает тосты (D-09).

### Гейты `.mjs`

- `ui/scripts/check-ws-disconnect-toast.mjs` (W1-W6, мутанты M1-M15): расширить инвариантом W7 «таймер отложенного тоста хранится и отменяется на `onopen`/`activeCleanup`/`teardown`» + selftest-мутанты; запуск `node ui/scripts/check-ws-disconnect-toast.mjs --selftest && node ui/scripts/check-ws-disconnect-toast.mjs`.
- `check-ws-resync-registry.mjs`: образец `check-transport-hooks.mjs` (+ нижняя граница >=13, мутации: удалить `onWsResync` из одного файла, пустой список).
- Подключение: `ui/package.json:16` цепочка `lint` - пары `--selftest` + боевой запуск; проверяют `check-lint-selftest-wiring.mjs`, `check-gate-coverage.mjs`.
- `pnpm lint` НЕ включает `svelte-check` - запускать отдельно (`pnpm --dir ui svelte-check`), так же clippy и fmt.

### Тесты Rust

**`ws_heartbeat.rs`, `ws_resync_required.rs`, `im_ws_fanout.rs`**: образец `tests/chat_ws_fanout.rs` (стр. 1-90): TCP-сервер `build_router` + `RusqliteSessionStore`, куки сессии, `tokio_tungstenite`, негативные проверки без sleep (тотальный порядок + контрольное событие). Скопировать `cookie`, `serve`, `connect`, `frame`:

```rust
async fn serve(ctx: &AppCtx) -> SocketAddr {
    let app = build_router(ctx, RusqliteSessionStore::new(ctx.writer.clone(), ctx.readers.clone()));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let _ = axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await;
    });
    addr
}
```

Для таймингов: `build_router` поднимает без `WsTimings`; для heartbeat-теста нужен способ подать короткие тайминги (Extension-слой в тестовом роутере либо параметр) - решить в плане. Регрессия остановки сервера: `ws_close_on_server_stop.rs` должен остаться зелёным.

**Хелперы** (`im_support/mod.rs`): `seed_user` (INSERT в `users` через `ctx.writer.execute`), `ident(id, Role)` - копировать из `tests/chat_support/mod.rs:13-33`.

**Тесты миграции**: `trackly-infra/tests/chat_migration.rs`, `history_events_repo.rs` как образцы.

## Общие паттерны

### Правило «рассылка после записи»
**Источник:** `services/entities_broadcast.rs:1-12`; `chat_service.rs` (после `.await?`).
**Применять к:** `im_service`, `request_service::create`, `chat_service::send`. Публикация вне замыкания; при replay - не публиковать.

### Единственный писатель
**Источник:** `self.writer.execute(move |conn| {...}).await?` (`chat_service.rs`, `request_service.rs:450`), mpsc 256, таймаут 5 с. `seq`/`rev`/`last_seq` - только внутри замыкания, одним транзакционным блоком.

### Авторизация HTTP
**Источник:** `http/chat.rs` - `session_identity(&session).await?` + делегирование сервису; WS - `ws_auth_middleware` (`ws.rs:64-72`). Не-участник -> 404/403, не пустой 200.

### Ошибки
`AppError` (`trackly_core::error`), `AppErrorResponse` (`error_axum`), `map_rusqlite`; reject-коды домена возвращаются как `Ok(...)` (чат), `Forbidden`/`NotFound` - как `Err`.

### Приватность
Тестовые данные - вымышленные; в документах протокола и ADR реальных названий/ФИО нет. Перед коммитом - privacy-гейт (грепнуть реквизиты).

## Аналога нет

| Файл | Роль | Поток | Причина |
|------|------|-------|---------|
| Ping/pong и `idle_timeout` в `ws.rs` | controller | streaming | Сервер сейчас «только пушит» и не слушает клиента; ориентироваться на RESEARCH Pattern 4 |
| `packages/transport/src/sendWithRetry.ts` (автоповторы, состояние `pending/failed`) | utility | request-response | В проекте нет клиентского retry; RESEARCH «Идемпотентность», раздел клиента (3 повтора 1/2/4 с, только сетевые сбои) |
| `ui/scripts/check-ws-resync.mjs` (исполняющий гейт с фейковым `WebSocket`) | test | event-driven | `check-ws-disconnect-toast.mjs` исполняет структурно/текстом; полный исполняющий образец - сверить `check-transport-tauri.mjs` перед планом; при отсутствии - RESEARCH «Wave 0 Gaps» |

## Метаданные

**Область поиска аналогов:** `crates/trackly-app/{src,tests}`, `crates/trackly-infra/{src,tests}`, `migrations/`, `packages/transport/`, `ui/scripts/`, `ui/package.json`
**Просмотрено файлов:** ~16 (полностью или по диапазонам)
**Дата извлечения:** 2026-11-26
