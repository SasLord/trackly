# Фаза 43: Надёжная доставка событий - Research

**Researched:** 2026-11-26
**Domain:** WebSocket-доставка (axum 0.8 ws), курсорная синхронизация поверх SQLite (единственный писатель), идемпотентность записи, клиент транспорта (Svelte/TS + Tauri-мост)
**Confidence:** HIGH для внутренних фактов (всё прочитано из репозитория), MEDIUM для деталей поведения браузера при полуоткрытых сокетах (разд. «Мёртвые соединения»)

Внешних библиотек фаза не добавляет (см. «Standard Stack»): Context7/WebSearch не понадобились, все факты по коду — `[VERIFIED: codebase grep]`, помеченные иначе — оговорены.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** В Фазе 43 создаётся минимальное ядро `im_*`: беседы (`im_conversations` с `last_seq`), участники (`im_members`), сообщения (`im_messages` с `seq`, `rev`, `client_msg_id`, `UNIQUE(conversation_id, seq)`, `UNIQUE(conversation_id, author_user_id, client_msg_id)`) — по схеме из `.planning/research/ARCHITECTURE.md` §3–§4. Фаза 47 НАРАЩИВАЕТ эту схему, а не создаёт заново. Колонки, которые понадобятся 47, можно заложить сразу, если это дёшево и не требует логики (на усмотрение планировщика). Тело сообщения в 43 — простой текст в обёртке версионируемого документа; валидатор rich-документа — Фаза 47/49.
- **D-02:** Наружу как настоящие маршруты `/api/v1/*` (с авторизацией по членству) выставляются только `im_send` и `im_sync`: их потребят Фазы 44 и 48. Создание бесед и изменение состава (добавить/исключить участника) в 43 — сервисные функции, вызываемые из тестов; маршруты для них появятся в Фазе 47.
- **D-03:** Клиентская сторона в 43 — общий JS-пакет `packages/transport` (браузер) и мост десктопа Trackly (`main.rs`, `trackly-event`). Протокол (кадры `hello`, `resync_required`, сигналы IM, ping/pong, тайминги) фиксируется документом (ADR или раздел в ADR 0003) и серверными тестами так, чтобы Rust-клиент Link в Фазе 44 реализовал его без догадок.
- **D-04:** Доставка — «сигнал + догрузка» по ADR 0003: сигнал несёт только `{conversation_id, rev}` и тип, без тела. `seq`/`rev` назначаются внутри замыкания единственного писателя. Получатели определяются в той же транзакции, публикация — после `.await?` (правило D-04 из `entities_broadcast.rs`). Маршрутизация по пользователям — отдельный компонент (`ImHub`, канал на пользователя); старый `WsEvent`/`ws_broadcast` для IM НЕ расширяется, `is_visible_to` для IM не используется.
- **D-05:** После исключения из беседы новые события этой беседы пользователю НЕ приходят сразу (следующий же сигнал после коммита исключения не попадает в его канал).
- **D-06:** Исключённый участник сохраняет **историю до момента исключения**: беседа остаётся у него в списке только для чтения, сообщения до момента исключения видны, новые не приходят и в `im_sync` не отдаются. Ядро обязано это заложить (например, граница видимости по `seq` на момент выхода в `im_members`, мягкий выход `left_at_utc`). Отличие от исследования: там предлагалось отдавать такую беседу в `removed:[ids]` — это решение пользователя отменяет. Изменения сообщений, сделанные ПОСЛЕ исключения (правка/удаление старых), исключённому не доставляются — его копия заморожена на момент выхода.
- **D-07:** Ветка `Lagged(n)` перестаёт молча продолжать — в ОБОИХ местах: `http/ws.rs` и мост десктопа `main.rs` (~стр. 327). Сервер шлёт клиенту `resync_required`; мост десктопа — аналогичный сигнал в webview. Увеличение буфера 128 — не решение, не делать.
- **D-08:** Клиент на `resync_required` И на каждый успешный `onopen` после обрыва вызывает ресинк всех подписчиков. Перечитывается **всё, что слушает WS-события** (списки, открытые карточки, счётчики, колокольчик, чат заявки), а не только текущий раздел. Нужен гейт, который ловит подписчика без обработчика ресинка (по образцу реестровых гейтов проекта).
- **D-09:** Ресинк **тихий**: никакого отдельного тоста «данные обновлены». При `Lagged` без обрыва пользователь не видит ничего.
- **D-10:** Ресинк **не трогает несохранённый ввод** в открытых формах (карточка устройства, акт и т.п.) — так же, как обычное событие сегодня. Обнаружение конфликта правок — вне фазы.
- **D-11:** Идемпотентная отправка по клиентскому id применяется в трёх местах: `im_send`, чат заявок (`chat_service::send`) и **создание заявки** (`request_service::create`). Повтор с тем же id возвращает уже созданную сущность, а не ошибку и не дубликат. Для чата заявок и заявок — миграции с уникальным ключом (автор + клиентский id), как у `im_messages`.
- **D-12:** Поведение клиента при сбое сети: несколько тихих автоповторов с тем же id; если не удалось — сообщение остаётся в ленте с пометкой «Не отправлено» и кнопкой «Повторить» (тот же id). Очередь — только в памяти (живёт, пока открыта вкладка); оффлайн-режим вне ТЗ. Для формы создания заявки — тот же принцип: id держится формой до успеха, повторное нажатие уходит с тем же id.
- **D-13:** Сервер шлёт `hello` первым кадром, ping каждые **25 с**, закрывает соединение без ответа **60 с**. Значения — константы в коде, не настройки.
- **D-14:** Тост «Соединение с сервером потеряно» показывается **с задержкой ~5 с**: если переподключились быстрее (проснулся ноутбук, мигнул Wi-Fi) — тоста нет. Существующие инварианты из `packages/transport/src/ws.ts` сохраняются: не больше одного тоста за эпизод, тост залипающий и снимается на `onopen`, при остановке сервера тост появляется гарантированно.
- **D-15:** Переподключение: backoff 1 с → ×2 → 30 с как сейчас, плюс **сброс ожидания и немедленная попытка** по событиям браузера «сеть появилась» (`online`) и возврату на вкладку (`visibilitychange`).

### Claude's Discretion
- Формат кадров и сигналов (имена полей, типы), версия протокола в `hello`.
- Порог `reload:true` в `im_sync` и размер страницы.
- Число и интервалы автоповторов отправки.
- Точная форма границы видимости исключённого (`left_seq` или иное) — при условии D-06.
- Как именно мост десктопа передаёт ресинк в webview (отдельное событие или кадр в `trackly-event`).
- Нужен ли отдельный ADR или дополнение ADR 0003 для протокола (D-03).

### Deferred Ideas (OUT OF SCOPE)
- Предупреждение о конфликте правок, если открытая в форме запись изменилась на сервере во время редактирования.
- Идемпотентность прочих создающих мутаций (кроме заявки и сообщений).
- Оффлайн-очередь отправки, переживающая закрытие вкладки.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| DLV-01 | После обрыва/переполнения клиент догружает пропущенное по курсору | `resync_required` вместо `Lagged => continue` в `http/ws.rs` и `main.rs`; реестр ресинков подписчиков `onWsEvent` (13 мест, см. таблицу); `im_sync` по `rev`-курсору |
| DLV-02 | События только своих бесед; после исключения перестают приходить сразу | `ImHub` с каналом на пользователя, получатели из `im_members` внутри транзакции писателя; граница `left_seq`/`left_at_utc` (D-06); `im_sync` режет по границе |
| DLV-03 | Повторная отправка не создаёт дубликат | `client_msg_id` + UNIQUE (im_messages, chat_messages), `client_request_id` + частичный UNIQUE (requests); проверка «уже есть» внутри замыкания писателя до проверки статуса |
| DLV-04 | Сервер закрывает мёртвые соединения, клиенты переподключаются | `hello`, ping 25 с / таймаут 60 с (константы, но инжектируемые для тестов), `online`/`visibilitychange` в `ws.ts`, отложенный тост 5 с |
</phase_requirements>

## Summary

Фаза состоит из трёх почти независимых контуров. (1) **Серверный транспорт** (`http/ws.rs`): к существующему `select!` (shutdown / broadcast / recv) добавляются ветка `ImHub`, интервал ping и дедлайн по последней входящей активности; `Lagged` превращается в кадр `resync_required`; первым кадром уходит `hello`. (2) **Ядро `im_*`**: две миграции (ядро + ключи идемпотентности), сервис `ImService` на единственном писателе, `ImHub`, два маршрута `im_send`/`im_sync`. (3) **Клиент**: `packages/transport/src/ws.ts` получает реестр ресинков, отложенный тост, `online`/`visibilitychange`; 13 подписчиков `onWsEvent` в `ui/` и `packages/` регистрируют ресинк; десктоп-мост шлёт отдельное Tauri-событие.

Главные нетривиальные выводы: (а) в `tokio` проекта нет фичи `test-util`, поэтому тайминги D-13 нужно делать инжектируемыми (параметры `WsTimings`), иначе тесты ждут по 60 с; (б) гейт паритета `http_route_parity.rs` потребует записей в `ROUTE_WITHOUT_CMD` для `im_send`/`im_sync`; (в) `resync_required` нельзя класть в `WsEvent` (specta-биндинги, `is_visible_to`, `packages-compat.ts`) — это отдельный служебный кадр; (г) идемпотентность обязана проверять «уже есть» ДО проверки статуса заявки (иначе повтор после закрытия заявки вернёт `Closed`, а не созданное сообщение); (д) `ADD COLUMN ... UNIQUE` в SQLite невозможен — нужен `CREATE UNIQUE INDEX`.

**Primary recommendation:** Сначала протокол и серверные тесты на настоящем WS (по образцу `chat_ws_fanout.rs`), затем ядро `im_*`, затем клиент и гейты. Все новые константы и кадры зафиксировать в разделе ADR 0003 «Протокол доставки (Фаза 43)».

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Назначение `seq`/`rev`, определение получателей | API / Backend (замыкание писателя) | Database | Монотонность без блокировок гарантирует единственный писатель |
| Маршрутизация сигналов по пользователям (`ImHub`) | API / Backend | — | Авторизация фан-аута структурна: сигнал не попадает в канал не-получателя |
| `resync_required`, `hello`, ping/pong, закрытие мёртвых | API / Backend (`http/ws.rs`) | Browser / Client | Сервер единственный видит `Lagged` и полуоткрытые сокеты |
| Мост десктопа (`Lagged` → webview) | Desktop shell (`main.rs`) | Browser / Client | Webview не имеет WS; мост — единственный канал |
| Ресинк экранов, реестр ресинков | Browser / Client (`packages/transport`) | — | Экраны сами знают, что перечитывать |
| Backoff, `online`/`visibilitychange`, отложенный тост | Browser / Client | — | Только браузер знает о сети и видимости |
| Идемпотентность создания | Database (UNIQUE) + API (replay-ответ) | Client (держит id) | Гарантия уникальности должна быть на уровне БД |
| Автоповторы и «Не отправлено» | Browser / Client | — | Очередь только в памяти (D-12) |
| Курсорный `im_sync` | API / Backend | Database (читающий пул) | Чтение не блокирует писателя в WAL |

## Standard Stack

Новых зависимостей не требуется. `[VERIFIED: Cargo.toml]`

### Core (уже в проекте)
| Library | Version | Purpose | Why |
|---------|---------|---------|-----|
| axum (feature `ws`) | 0.8 | WS-цикл, `Message::Ping/Pong/Text/Close` | Уже используется в `http/ws.rs` |
| tokio | 1 (`time`, `sync`, без `test-util`) | `interval`, `timeout`, `broadcast`, `mpsc` | Уже в стеке |
| rusqlite / refinery | 0.38 / 0.9 (по Cargo.toml, не по CLAUDE.md) | Запись, миграции | Единственный писатель уже реализован |
| uuid (v4) | 1 | Клиентские id (на Rust-стороне и в тестах); в браузере — `crypto.randomUUID()` | Есть в workspace |
| tokio-tungstenite 0.29 + futures-util | dev | Настоящий WS-клиент в тестах | Уже в `[dev-dependencies]`, используется `chat_ws_fanout.rs` |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `std::sync::Mutex<HashMap<i64, broadcast::Sender>>` в `ImHub` | `dashmap` (в ARCHITECTURE.md) | `dashmap` в workspace НЕТ; при ≤100 пользователях Mutex достаточен и не требует новой зависимости (иначе — slopcheck/human-verify). Рекомендация: Mutex, без новых крейтов |
| Протокольные ping-кадры WS | Текстовые `{"type":"ping"}` | См. «Мёртвые соединения»: протокольные — для сервера и Rust-клиента Link; текстовый нужен браузеру только как проба при пробуждении |

**Installation:** нет. **Package Legitimacy Audit:** фаза не устанавливает внешних пакетов — не требуется (slopcheck не запускался).

## Architecture Patterns

### System Architecture Diagram

```
 Мутации экранов ──► Service ──► writer.execute(замыкание) ──► commit
                                       │  (im_*: seq/rev++, получатели из im_members)
                                       ▼ (после .await?, вне замыкания)
              ┌────────────────────────┴─────────────────────┐
              │ WsEvent (общий broadcast 128)                │ ImSignal -> ImHub[user_id] (broadcast 256)
              ▼                                              ▼
   http/ws.rs select! ◄── интервал ping 25с ── deadline 60с ── socket.recv() (любой кадр = жив)
      │ ветки: shutdown | ws_broadcast | im_hub | tick | recv
      │ Lagged(общий) или Lagged(im) ──► кадр {"type":"resync_required"}
      ▼
   браузер ws.ts ── onmessage ──► служебные кадры (hello/resync_required) ИЛИ dispatch(WsEvent)
      │ resync_required / onopen-после-обрыва ──► resyncAll() ──► реестр обработчиков ресинка
      │ online / visibilitychange ──► сброс backoff + немедленный connect
   Десктоп: main.rs bridge ws_broadcast ─► emit("trackly-event") ; Lagged ─► emit("trackly-resync")
      └─► ws.ts (isTauri) listen('trackly-resync') ──► resyncAll()

 Клиент IM (Фаза 44/48): POST im_sync{cursors} ◄─ сигнал {conversation_id, rev}; POST im_send{client_msg_id}
```

### Recommended Project Structure
```
migrations/V058__im_core.sql                  # im_conversations/members/messages, user_version=58
migrations/V059__idempotency_keys.sql         # chat_messages.client_msg_id, requests.client_request_id, user_version=59
crates/trackly-core/src/domain/im.rs          # чистые правила: видимость, граница исключённого, нормализация текста
crates/trackly-infra/src/repos/im_sqlite.rs   # SQL на &Connection/&Transaction (паттерн chat_sqlite.rs: *_on(tx))
crates/trackly-app/src/services/im_service.rs # send/sync + create_conversation/add_member/remove_member (для тестов)
crates/trackly-app/src/services/im_hub.rs     # ImHub, ImSignal
crates/trackly-app/src/http/im.rs             # /api/v1/im_send, /api/v1/im_sync
crates/trackly-app/src/http/ws.rs             # + WsTimings, hello, ping/deadline, resync_required, ветка ImHub
packages/transport/src/ws.ts                  # + onWsResync, resyncAll, отложенный тост, online/visibilitychange
docs/adr/0003-...md                           # раздел «Протокол доставки (Фаза 43)»
```

### Pattern 1: Запись `im_send` внутри замыкания писателя
**What:** одна транзакция: (1) ищем `(conversation_id, author, client_msg_id)` — нашли, возвращаем существующее (replay, без сигнала); (2) проверяем, что автор — активный участник (`left_at_utc IS NULL`) — внутри транзакции, не до неё (TOCTOU, как статус заявки в `chat_service::send`); (3) `UPDATE im_conversations SET last_seq = last_seq+1 WHERE id=? RETURNING last_seq`; (4) INSERT сообщения `seq = rev = last_seq`; (5) читаем список получателей (`SELECT user_id FROM im_members WHERE conversation_id=? AND left_at_utc IS NULL`) в той же транзакции; (6) commit; наружу `(MessageDto, recipients, is_replay)`. После `.await?`: `im_hub.publish(recipients, ImSignal::Changed{conversation_id, rev})`.
**Backstop:** при гонке двух одинаковых id писатель однопоточен, но UNIQUE остаётся страховкой; конфликт UNIQUE обрабатывать как replay (повторный SELECT), а не как ошибку.
**Source:** паттерн `chat_service::send` (стр. ~444–560) и `entities_broadcast.rs` (правило D-04). `[VERIFIED: codebase]`

### Pattern 2: Граница видимости исключённого (D-06)
Рекомендация (в рамках Discretion): в `im_members` добавить `left_seq INTEGER NULL` (значение `last_seq` беседы на момент выхода) вместе с `left_at_utc`. Видимость строки сообщения участнику: `m.seq <= COALESCE(left_seq, +inf)` И `m.seq >= visible_from_seq`. Для `im_sync`: фильтр по **`seq`, а не `rev`** для исключённого: он получает только сообщения с `seq <= left_seq`, а изменения `rev > left_seq` на старых сообщениях ему НЕ отдаются (копия заморожена, D-06) — это достигается тем, что для исключённого запрос не использует `rev > cursor` вовсе: беседа отдаётся один раз целиком (`seq <= left_seq`, последняя страница) с флагом `read_only:true`, а затем его курсор считается равным `left_seq` и повторные sync ничего не возвращают. Заложить `visible_from_seq` и `last_read_seq` сразу (дёшево, нужны 47).
**Важный нюанс (ловушка):** `rev` у правки старого сообщения превышает `left_seq` — наивный фильтр `rev > cursor AND seq <= left_seq` вернул бы изменённое после выхода сообщение с новым содержимым и нарушил бы «копию заморожена». Для исключённого нужен отдельный путь без `rev`-выборки (или хранить снимок; в 43 правок нет, но тест на этот инвариант писать сразу, симулируя правку прямым UPDATE `rev` в тесте).
**Ответ sync:** поле `read_only` и `left_seq` в сводке беседы; `removed:[ids]` НЕ используется.

### Pattern 3: ImHub
```rust
// Source: ARCHITECTURE.md §4 (упрощено: без dashmap)
pub struct ImHub { chans: std::sync::Mutex<HashMap<i64, broadcast::Sender<ImSignal>>> }
impl ImHub {
    pub fn subscribe(&self, user_id: i64) -> broadcast::Receiver<ImSignal>;   // лениво создаёт канал (ёмкость 256)
    pub fn publish(&self, recipients: &[i64], s: ImSignal);                   // только существующим каналам; пустой канал не создаёт
    pub fn prune(&self);                                                      // receiver_count()==0 → удалить (вызывать при отписке сокета)
}
```
Структурная авторизация D-05: исключение коммитится до `publish`, список получателей считан ПОСЛЕ изменения состава внутри той же транзакции, поэтому исключённый в списке отсутствует. Сам сокет не фильтрует сигналы — фильтрует получатель-список. Сигнал, уже лежащий в буфере канала исключённого к моменту коммита исключения, всё равно содержит только `{conversation_id, rev}` без тела, а `im_sync` его не отдаст — это допустимо и должно быть описано в протоколе (тест DLV-02 проверяет «следующий сигнал после коммита»).
**Lagged по каналу IM** → тот же `resync_required` (клиент делает `im_sync` по всем беседам).

### Pattern 4: Цикл `handle_ws_socket` (DLV-04)
Ветки `select!`: `shutdown.cancelled()` (СОХРАНИТЬ, включая `CLOSE_SEND_TIMEOUT`), `rx.recv()`, `im_rx.recv()`, `ping_tick.tick()`, `socket.recv()`, и проверка дедлайна. Реализация: `last_seen: Instant` обновляется на ЛЮБОМ входящем кадре (включая Pong); по тику: `if now - last_seen > idle_timeout { close; break }` иначе `socket.send(Message::Ping(..))` с `timeout(SEND_TIMEOUT)` (зависший `send` к «мёртвому» peer не должен вешать задачу — тот же урок, что `CLOSE_SEND_TIMEOUT`). Первым кадром до цикла — `hello` (`{"type":"hello","protocol":1,"pingIntervalSecs":25,"idleTimeoutSecs":60,"serverTime":...}`), чтобы Rust-клиент Link не угадывал тайминги.
**Тестируемость:** сигнатура `handle_ws_socket(..., timings: WsTimings)`, `WsTimings{ping_interval, idle_timeout}` с `Default = 25с/60с` (константы D-13) и пробросом через `Extension`/поле `AppCtx` (по образцу `Option<Extension<ServerShutdown>>`); в тестах — 100 мс/300 мс. `tokio::time::pause` недоступен: у tokio в workspace нет `test-util` (`[VERIFIED: Cargo.toml:31]`), включать фичу ради этого не нужно.
**Мёртвый peer в тесте:** клиент `tokio-tungstenite`, который просто не читает сокет (не отвечает Pong, поскольку pong формируется при чтении), либо сырой TCP, завершивший handshake и замолчавший. Ожидание: сервер закрывает за `idle_timeout`.
**Оговорка про браузер:** браузер автоматически отвечает Pong на протокольный Ping (JS этого не видит) — серверная сторона полноценна. Клиентская сторона полуоткрытого сокета (после сна ноутбука) через протокольные ping НЕ детектируется, JS их не наблюдает; поэтому при `online`/`visibilitychange` (D-15) клиент делает: если `ws.readyState !== OPEN` — сразу переподключение; если OPEN — отправляет текстовый `{"type":"ping"}` и, не получив `{"type":"pong"}` (или любого кадра) за ~5 с, закрывает и переподключается. Это рекомендация планировщику `[ASSUMED]` (не обсуждалась с пользователем, но вытекает из D-15 «немедленная попытка»; без неё D-15 при половинчатом сокете не сработает). Сервер на клиентский текст `ping` отвечает `pong`.

### Pattern 5: Клиент — реестр ресинка (D-08)
В `ws.ts`: `onWsResync(handler): () => void` и внутренний `resyncAll()`. Вызывается: (а) на служебном кадре `resync_required`; (б) на `onopen`, если соединение было потеряно ранее (флаг `hadOpen`, чтобы не ресинкать первое открытие; для первого открытия подписчики уже загрузили данные штатно); (в) в Tauri-ветке на событии `trackly-resync`. Служебные кадры разбираются ДО `dispatch` и не попадают в `WsEventHandler` — типизация `WsEvent` и `packages-compat.ts` не меняются. Вводится отдельный тип `WsControlFrame`.
Ресинк подписчика = та же функция, что вызывается при обычном событии/монтировании (перезагрузить данные), без сброса локального ввода (D-10).

### Anti-Patterns to Avoid
- **`resync_required` как вариант `WsEvent`:** потребует specta-перегенерации `bindings-phase6.ts`, ветки в `is_visible_to` и правок десктоп-фильтра. Служебный кадр — отдельный.
- **Увеличение ёмкости `broadcast(128)`:** запрещено D-07.
- **Публикация сигнала внутри замыкания писателя:** откат оставит ложный сигнал (правило D-04).
- **`Written::Closed` до проверки replay** в `chat_service::send`: повтор после закрытия заявки отдаст ошибку вместо созданного сообщения.
- **`ALTER TABLE ... ADD COLUMN x UNIQUE`:** SQLite запрещает; использовать `CREATE UNIQUE INDEX`.
- **Перестройка `requests`/`chat_messages`:** урок V042/V050 (FK OFF не действует внутри транзакции refinery) — только `ADD COLUMN` + `CREATE INDEX`.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Heartbeat | Свой кадр-пинг как единственный механизм | Протокольные `Message::Ping` + `last_seen` | Браузер и tokio-tungstenite отвечают Pong сами |
| Монотонные номера | Блокировки/мьютексы вокруг счётчика | `UPDATE ... SET last_seq=last_seq+1 RETURNING` в замыкании единственного писателя | Структурная монотонность, уже принятая модель |
| Уникальность повтора | Проверка «есть ли» только в коде | `UNIQUE`-индекс + replay-ветка | Гонка двух вкладок/устройств |
| Генерация клиентских id | Счётчик/`Date.now()` | `crypto.randomUUID()` (контекст безопасный: https и 127.0.0.1 — см. память про secure context; для LAN по голому `http://` `randomUUID` недоступен — fallback на `crypto.getRandomValues`) | Коллизии между вкладками |
| Ожидание в тестах | `sleep` для негативных проверок | Тотальный порядок событий в канале + контрольное событие (приём `chat_ws_fanout.rs`) | Нет флакинга |

**Key insight:** вся надёжность держится на курсоре и идемпотентности; поток WS — лишь подсказка. Любая «умная» доставка поверх неё добавляет только пути рассинхронизации.

## Runtime State Inventory

Фаза не переименование/миграция существующих данных. Затрагиваемое состояние: БД получает новые таблицы и колонки (`requests.client_request_id`, `chat_messages.client_msg_id` — NULL для старых строк, UNIQUE-индекс частичный `WHERE ... IS NOT NULL`, поэтому старые строки не конфликтуют). Прочие категории (живые сервисы, ОС-регистрации, секреты, артефакты сборки): **None** — verified анализом scope. Исключение: `ui/dist` пересобрать (`pnpm --dir ui build`) перед браузерной проверкой (память проекта).

## Common Pitfalls

### Pitfall 1: Миграция без `PRAGMA user_version`
**What goes wrong:** точечные тесты зелёные, в `ci-full` ~10 красных (`downgrade_protection`, `chat_migration` и др.).
**How to avoid:** последняя строка каждой миграции `PRAGMA user_version = N;` (58 и 59); добавить тест «файл заканчивается user_version» по образцу `history_events_repo.rs::v057_file_ends_with_user_version_57`; найти места, где захардкожено ожидаемое число (`grep -rn user_version crates --include='*.rs'`: `downgrade_protection.rs`, `desktop_system_admin.rs`, `acts_clone_handover.rs`, `context.rs`, `dto/health.rs`).
**Warning signs:** `user_version` на 1 меньше (признак устаревшей `embed_migrations`; build.rs уже пересобирает при новом .sql).

### Pitfall 2: Повтор не доходит до replay-ветки
Порядок в замыкании: replay-поиск → проверка статуса/членства → вставка. Для `request_service::create` replay-поиск `(requested_by_user_id, client_request_id)` — первым; найденная заявка возвращается через `self.get(...)`, а `WsEvent::NewRequest` при replay НЕ шлётся повторно (иначе дубль уведомления «новая заявка»). Валидации (`request_type`, scope принтера) можно выполнять до замыкания: повтор с теми же данными пройдёт их так же; но если принтер ушёл из области между попытками — вернётся Forbidden вместо replay. Рекомендация: replay-поиск раньше валидации scope, сразу после `authorize`; поиск по `(requested_by_user_id, client_request_id)` идёт через читающий пул, а окончательная гарантия — в замыкании писателя.

### Pitfall 3: Дыра в `seq` при откате
`last_seq` увеличивается внутри той же транзакции, что и вставка; откат откатывает оба — дыр нет. Нельзя инкрементировать `last_seq` отдельным запросом вне транзакции или в другом замыкании.

### Pitfall 4: Dedup по клиентскому id между авторами
Ключ — `(author, client_msg_id)` (и `(conversation_id, author, client_msg_id)` в `im_messages`), не глобальный `client_msg_id`: иначе чужой id блокирует отправку и раскрывает существование сообщения. Повтор с тем же id, но ДРУГИМ текстом: возвращать существующее (id — идентичность, тело не сравнивается) — зафиксировать в тесте и протоколе.

### Pitfall 5: Тихий ресинк затирает ввод
`resyncAll()` вызывает перечитывание данных, а не пересоздание компонентов; формы с локальным состоянием не должны сбрасываться (D-10). Для проверки: гейт/тест, что обработчик ресинка не трогает поля формы; ручная проверка в UAT.

### Pitfall 6: Ресинк на первом `onopen` / лавина запросов
13 подписчиков × каждый ресинк = пачка запросов после каждого переподключения. Это допустимо (≤100 пользователей), но: не ресинкать на первом открытии; дебаунсить `resyncAll` (≤1 запуска за ~500 мс), чтобы `resync_required` + `onopen` подряд не дублировали загрузку.

### Pitfall 7: Тост с отложенным показом ломает инварианты гейта
`scripts/check-ws-disconnect-toast.mjs` (W1–W6, мутанты M1–M15) проверяет структуру `ws.ts` текстом. Отложенный показ (`setTimeout` ~5 с, отменяемый на `onopen`) должен: сохранять `sticky:true` и флаг эпизода; **хранить таймер и отменять его** при `onopen`, `activeCleanup` и `teardown` (иначе класс дефекта WR-04: призрачный таймер показывает тост уже восстановленного соединения); при остановке сервера (`onclose` с `refCount>0`) таймер всё равно срабатывает — «гарантированно» выполняется за 5 с, если соединение не восстановилось. Гейт придётся обновить в том же плане (новый инвариант W7: таймер отложенного тоста отменяем; selftest-мутанты на «таймер не отменён на onopen»); `--selftest` запускается в `pnpm lint`.

### Pitfall 8: Каждый новый `.mjs`-гейт обязан быть подключён в `pnpm lint`
`ui/package.json` `lint` — одна длинная цепочка; `check-lint-selftest-wiring.mjs` и `check-gate-coverage.mjs` проверяют подключение. Новый гейт ресинка (`check-ws-resync-registry.mjs`) добавлять парой `--selftest` + боевой запуск.

### Pitfall 9: Новый `.rs`-файл теста не подключён к suite
`autotests = false`: файл `tests/*.rs` без `#[path]` в `tests/suites/*.rs` (или `[[test]]`) молча не запускается; `suite_wiring.rs` краснеет. Серверные ВС-тесты — в `suites/server.rs`, миграционные — в suite infra (смотреть `crates/trackly-infra/Cargo.toml`).

### Pitfall 10: Мост десктопа и `desktop_event_visible`
Служебный сигнал ресинка не проходит `is_visible_to` (он не `WsEvent`); эмитить его напрямую `app_handle.emit("trackly-resync", ())` в ветке `Lagged`. `trackly-event` остаётся только для `WsEvent`. Покрыть тестом: ветка `Lagged` вынесена в тестируемую функцию (по образцу `desktop_event_bridge.rs`), иначе `main.rs`-код без теста невидим для аудита.

## Code Examples

### Ветка Lagged в `http/ws.rs` (замена `continue`)
```rust
// Source: crates/trackly-app/src/http/ws.rs (текущая ветка) + D-07
Err(broadcast::error::RecvError::Lagged(n)) => {
    tracing::warn!("ws: client lagged {n} events — sending resync_required");
    if send_text(&mut socket, r#"{"type":"resync_required"}"#).await.is_err() {
        break;
    }
}
```
`send_text` = `timeout(SEND_TIMEOUT, socket.send(Message::Text(..)))`.

### Клиент: служебные кадры
```ts
// packages/transport/src/ws.ts (набросок)
type WsControlFrame = { type: 'hello'; protocol: number } | { type: 'resync_required' } | { type: 'pong' };
ws.onmessage = (e) => {
  let msg: WsEvent | WsControlFrame;
  try { msg = JSON.parse(e.data); } catch { return; }
  if (msg.type === 'resync_required') { resyncAll(); return; }
  if (msg.type === 'hello' || msg.type === 'pong') { return; }
  dispatch(msg as WsEvent);
};
```

### Идемпотентный INSERT (замыкание писателя)
```rust
// Source: паттерн chat_service::send; ключ — (author, client_msg_id)
if let Some(cid) = &client_msg_id {
    if let Some(existing) = repo.find_by_client_id_on(&tx, user_id, cid)? {
        return Ok(Written::Replay(existing));   // без сигнала, без аудита
    }
}
```

## Enumerated: все подписчики `onWsEvent` (D-08, инвентарь ОТ репозитория)

`grep -rn "onWsEvent" ui/src packages` — 13 вызовов; каждому нужен обработчик ресинка (или явное освобождение в реестре с причиной) `[VERIFIED: codebase grep]`:

| # | Файл | Что слушает / что перечитать на ресинк |
|---|------|-----------------------------------------|
| 1 | `ui/src/features/settings/OrgSettings.svelte:433` | `number_space_changed` (шаблоны номеров) |
| 2 | `ui/src/features/layout/EmployeeLayout.svelte:118` | события сотрудника, колокольчик/уведомления |
| 3 | `ui/src/features/layout/Layout.svelte:48` | `entities_changed` (оболочка Admin/Manager: инвалидации деревьев/счётчиков) |
| 4 | `ui/src/features/requests/RequestDetail.svelte:371` | открытая заявка |
| 5 | `ui/src/features/requests/RequestsPage.svelte:194` | список заявок, счётчики |
| 6 | `ui/src/features/requests/chat/chatNotifications.svelte.ts:61` | уведомления чата (ресинк = НЕ тосты; тихо) |
| 7 | `ui/src/features/requests/chat/RequestChat.svelte:243` | лента чата открытой заявки (`chat_feed` заново) |
| 8 | `ui/src/features/requests/chat/unreadStore.svelte.ts:134` | счётчики непрочитанного (`chat_unread_summary`) |
| 9 | `ui/src/features/printers/PrintersPage.svelte:205` | список принтеров/алертов |
| 10 | `ui/src/features/employee/EmployeeDevicesPage.svelte:109` | устройства сотрудника |
| 11 | `ui/src/lib/components/NumberTemplateField.svelte:406` | превью номера шаблона |
| 12 | `packages/attachments/src/attachmentEvents.svelte.ts:38` | события вложений |
| 13 | `packages/attachments/src/AttachmentsBlock.svelte:89` | список вложений открытой карточки |

**Ловушка для (6):** ресинк чата не должен породить всплывающие уведомления о «новых» сообщениях, пришедших за время обрыва — иначе нарушается D-09 (тихий ресинк). Проверить логику `chatNotifications` (она реагирует на событие, ресинк — это перечитывание счётчика, не событие).
**Гейт (D-08):** `ui/scripts/check-ws-resync-registry.mjs` — для каждого файла из `ui/src` и `packages` (кроме `ws.ts`), где есть `onWsEvent(`, требует `onWsResync(` в том же файле либо запись в явном списке освобождений с причиной; плюс нижняя граница счёта (≥13) против «зелёного по пустоте»; selftest-мутанты: удаление `onWsResync` из одного файла → красный; пустой список файлов → красный. Образец: `check-transport-hooks.mjs`, `check-ws-disconnect-toast.mjs`. Мутационная проверка гейта на настоящем исходнике (урок «якорь мутации уникален»).

## Идемпотентность — места и схема (D-11)

| Место | Ключ | Миграция | Клиент |
|-------|------|----------|--------|
| `ImService::send` | `UNIQUE(conversation_id, author_user_id, client_msg_id)` (в V058) | — | Фазы 44/48 |
| `ChatService::send` (`chat_service.rs` ~444) | `chat_messages.client_msg_id TEXT NULL` + `CREATE UNIQUE INDEX uq_chat_messages_client ON chat_messages(author_user_id, client_msg_id) WHERE client_msg_id IS NOT NULL` | V059 (ADD COLUMN + индекс) | `ChatComposer.svelte` (`chat.send` на стр. 170), `chat/api.ts` (`chat_send`) |
| `RequestService::create` (~345) | `requests.client_request_id TEXT NULL` + `CREATE UNIQUE INDEX uq_requests_client ON requests(requested_by_user_id, client_request_id) WHERE client_request_id IS NOT NULL` | V059 | `RequestFormModal.svelte` → `requests/api.ts: create` |

**Цепочка изменений DTO (двухтранспортность):** `RequestCreateDto` (+`client_request_id: Option<String>`, serde `camelCase`), `SendPayload` в `http/chat.rs` (+`client_msg_id`, `#[serde(default)]`), `build_chat_send` и Tauri-команда `chat_send` (+`client_msg_id: Option<String>`), регенерация specta-биндингов (`ui/src/bindings.ts`, `bindings-phase6.ts`; тесты `export_bindings.rs`, `specta_roundtrip.rs`), `ui/src/features/requests/chat/api.ts`. Поле опционально — старые клиенты работают без идемпотентности (обратная совместимость с Link-раньше-релиза). Tauri-вызов `invoke` с `Option` принимает отсутствие поля.
**Поведение replay для `chat_send`:** возвращается `ChatSendOutcomeDto{message: Some(existing)}` — тот же DTO, что при успехе; WS `ChatMessage` НЕ шлётся повторно. Нюанс: `draft_attachment_ids` при replay игнорируются (уже привязаны).
**Клиентская часть (D-12):** помощник в `packages/transport` (рекомендация, чтобы не дублировать в трёх местах): `sendWithRetry(fn, {attempts, delays})` — повторяет ТОЛЬКО сетевые сбои (нет ответа / `TypeError` fetch / код сети), НЕ повторяет доменные отказы (`rejectCode`, 4xx). Число/интервалы по умолчанию: 3 повтора, 1 с → 2 с → 4 с (Discretion; `[ASSUMED]`). Состояние сообщения: `pending | failed` в памяти хранилища чата; элемент ленты «Не отправлено» + кнопка «Повторить» (тот же `clientMsgId`); для формы заявки — id создаётся при открытии формы, обнуляется только после успеха.

## UI-поверхность (для design-contract / UI-spec, если включён)

1. **Лента чата заявки (`RequestChat.svelte`, `ChatMessage.svelte`, `ChatComposer.svelte`):** новое состояние сообщения «Не отправлено» (приглушённый стиль + текст + кнопка «Повторить»); в проекте требование: кастомные компоненты, не нативные (память: Dropdown вместо `<select>`; кнопка — существующий компонент kit). Решить: показывать ли пометку ВНУТРИ списка сообщений (D-12: «остаётся в ленте»). Токены контраста — гейт `check-contrast.mjs`.
2. **Форма заявки (`RequestFormModal.svelte`):** повторное нажатие «Отправить» после сбоя уходит с тем же id; текст сбоя — существующим механизмом ошибок модалки (не новый тост).
3. **Тост обрыва (D-14):** задержка ~5 с; текст прежний «Соединение с сервером потеряно. Переподключение…».
4. **Ресинк (D-09):** UI-поверхности нет (намеренно).
5. Все русские строки — только русский (CLAUDE.md); примеры в тестах — вымышленные имена («Иванов И.И.», «Петров П.П.»).

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Rust | `cargo test` (1 процесс за раз — память: нельзя параллельные cargo test; сплошной прогон 4,5–6 ч — НЕ запускать на каждом плане) |
| Схема целей | `autotests = false`; тесты подключаются в `crates/trackly-app/tests/suites/*.rs` (`#[path]`) — серверные WS-тесты в `suites/server.rs`; бинарник-цель называется как suite |
| UI-гейты | Node-скрипты `ui/scripts/check-*.mjs`; `pnpm --dir ui lint` (гейты ESLint/prettier/`.mjs`) — **НЕ включает** `svelte-check` |
| Отдельные команды | `pnpm --dir ui svelte-check`, `cargo clippy`, `cargo fmt --check` — каждая отдельно (исполнители сдают зелёным только то, что гоняли) |
| Linker на этой машине | при exit 69 — префикс `DEVELOPER_DIR=/Library/Developer/CommandLineTools` |
| Эталон сплошного прогона | команда `ci-full` (один раз на границе волны, в фоне, без `timeout` — его на машине нет) |

### Phase Requirements → Test Map
| Req | Behavior | Test Type | Automated Command | File |
|-----|----------|-----------|-------------------|------|
| DLV-01 | `Lagged` общего канала → клиент получает `resync_required` (настоящий WS, канал 128 переполнен) | integration | `cargo test -p trackly-app --test server ws_resync` (точное имя цели сверить в `Cargo.toml`) | ❌ Wave 0: `tests/ws_resync_required.rs` |
| DLV-01 | `Lagged` канала IM → `resync_required` | integration | то же | ❌ Wave 0 |
| DLV-01 | Мост десктопа: ветка Lagged эмитит `trackly-resync` (вынесенная функция) | unit | `cargo test -p trackly-app --lib desktop_event_bridge` | ❌ Wave 0 |
| DLV-01 | `im_sync`: отставший `rev` → сообщения `rev > cursor`; неизвестная беседа → сводка; порог → `reload:true` | integration | `cargo test -p trackly-app --test server im_sync` | ❌ Wave 0 |
| DLV-01 | Клиентский ресинк на `onopen`-после-обрыва и на `resync_required` | JS-исполняющий гейт | `node ui/scripts/check-ws-resync.mjs --selftest && node ui/scripts/check-ws-resync.mjs` | ❌ Wave 0 |
| DLV-01 | Реестр: каждый `onWsEvent` имеет ресинк | structural gate | `node ui/scripts/check-ws-resync-registry.mjs` | ❌ Wave 0 |
| DLV-02 | Не-участник не получает сигнал (настоящий WS, тотальный порядок + контрольное событие) | integration | `cargo test -p trackly-app --test server im_ws_fanout` | ❌ Wave 0 |
| DLV-02 | После исключения следующий сигнал не приходит; история до выхода читается; после — нет; правка после выхода не доставляется | integration | `cargo test -p trackly-app --test server im_membership` | ❌ Wave 0 |
| DLV-02 | `im_send`/`im_sync` отвергают не-участника (404/403, не 200 пустой) | HTTP | `cargo test -p trackly-app --test server im_http` | ❌ Wave 0 |
| DLV-03 | Повтор `im_send`/`chat_send`/`requests_create` с тем же id → тот же id сущности, строк ровно 1, сигнал/`NewRequest` не повторён | integration | `cargo test -p trackly-app --test <suite> idempot` | ❌ Wave 0 |
| DLV-03 | UNIQUE-страховка: прямой дубль INSERT отвергается | infra | `cargo test -p trackly-infra --test <suite> im_migration` | ❌ Wave 0 |
| DLV-03 | Проба «не зависит от статуса»: replay на закрытой заявке возвращает сообщение (не `Closed`) | integration | то же | ❌ Wave 0 |
| DLV-04 | `hello` первым кадром; ping приходит с интервалом; молчащий peer закрыт за `idle_timeout` (сокращённые тайминги) | integration | `cargo test -p trackly-app --test server ws_heartbeat` | ❌ Wave 0 |
| DLV-04 | Сохранена остановка сервера (`ws_close_on_server_stop`) | regression | `cargo test -p trackly-app --test server ws_close_on_server_stop` | ✅ |
| DLV-04 | Задержка тоста 5 с; ≤1 за эпизод; таймер отменён на `onopen`; `online`/`visibilitychange` сбрасывают backoff | JS-исполняющий гейт | `node ui/scripts/check-ws-disconnect-toast.mjs --selftest && node ui/scripts/check-ws-disconnect-toast.mjs` (расширить) | ✅ расширить |
| gates | Паритет http↔tauri: `im_send`,`im_sync` в `ROUTE_WITHOUT_CMD` с обоснованием | gate | `cargo test -p trackly-app --test <suite> http_route_parity` | ✅ править |
| gates | `suite_wiring` (новые файлы подключены) | gate | `cargo test -p trackly-app --test suite_wiring` | ✅ |
| gates | Дрейф `WsEvent` (`packages-compat.ts`) не нарушен | typecheck | `pnpm --dir ui svelte-check` | ✅ |
| gates | `ui/dist` для браузерной проверки | build | `pnpm --dir ui build` | ✅ |

Имена целей (`--test server` и пр.) планировщик обязан сверить с `[[test]]` в `crates/trackly-app/Cargo.toml` — в этом исследовании не сверялись.

### Sampling Rate
- **Per task commit:** скоуп-тест затронутой цели + соответствующий `node ui/scripts/check-*.mjs`
- **Per wave merge:** целевые suite (`server`, infra-suite с миграцией, `http_route_parity`, `suite_wiring`) + `pnpm --dir ui lint` + `pnpm --dir ui svelte-check` + `cargo clippy -D warnings` + `cargo fmt --check`
- **Phase gate:** один сплошной прогон `ci-full` в фоне на границе последней волны; затем `/gsd-verify-work`

### Wave 0 Gaps
- [ ] `crates/trackly-app/tests/ws_heartbeat.rs`, `ws_resync_required.rs`, `im_ws_fanout.rs`, `im_membership.rs`, `im_sync.rs`, `im_http.rs`, `idempotent_send.rs` (+ подключение в `suites/server.rs`)
- [ ] `crates/trackly-infra/tests/im_migration.rs` (V058/V059: таблицы, индексы, `user_version`, файл заканчивается `PRAGMA user_version`)
- [ ] хелперы IM-тестов (по образцу `chat_support/mod.rs`: `seed_user`, `cookie`, `serve`) — вынести в `im_support/mod.rs`
- [ ] `ui/scripts/check-ws-resync-registry.mjs`, `check-ws-resync.mjs` (исполняющий) с selftest, подключение в `pnpm lint` (+ `check-lint-selftest-wiring`/`check-gate-coverage`)
- [ ] Поведенческие проверки клиента запускать исполняющим гейтом (модуль транспилируется и вызывается с фейковым `WebSocket`) — компиляторные гейты рантайм рун/таймеров не видят (память: «compile gates miss runtime»)

**Требования к качеству тестов (уроки проекта):** негативные проверки — пробой с мутацией (снять охрану — тест краснеет), не удалением, а перестановкой; якорь мутации уникален; фикстура до- и после-фикса обязана различаться (иначе тест вакуумен).

## Security Domain

| ASVS | Applies | Control |
|------|---------|---------|
| V2/V3 Auth/Session | да | WS-маршрут за `ws_auth_middleware`; `im_send`/`im_sync` — `session_identity`; без сессии 401 |
| V4 Access Control | **да (ядро фазы)** | Членство проверяется в замыкании писателя (send) и в SQL-запросе (sync) — не после выборки; исключённый: границы `left_seq`; 404 для чужой беседы (не раскрывать существование) |
| V5 Input Validation | да | Длина `client_msg_id`/`client_request_id` (напр. ≤64, ASCII/UUID), длина текста (`normalize_text`/`exceeds_limit` из `domain/chat`), пустое тело; `limit` страницы sync ограничить сверху |
| V6 Cryptography | нет | — |

| Угроза | STRIDE | Митигация |
|--------|--------|-----------|
| Подписка на чужой сигнал | Info disclosure | Получатели из `im_members` в транзакции; канал на пользователя |
| Утечка тела через WS | Info disclosure | Сигнал только `{conversation_id, rev}` |
| Использование чужого `client_msg_id` для определения существования | Info disclosure | Ключ с автором; replay только своего |
| Исключённый читает новое/правки через `im_sync` | Info disclosure | Отдельный путь для исключённого, фильтр по `seq <= left_seq`, без выборки по `rev` |
| DoS: переполнение/зависший peer | DoS | Таймаут `send`, закрытие по `idle_timeout`, `Lagged` → resync, ограничение `limit` sync |
| Флуд `im_send` | DoS | Очередь писателя уже ограничена (mpsc 256, таймаут 5 с); лимит длины тела |
| Служебный кадр от клиента | Tampering | Серверу из клиентского текста интересен только `ping`; всё прочее игнорировать, размер кадра ограничить (axum по умолчанию ограничивает) |

Отдельно (учтённая в памяти реальность): `spa_fallback` отдаёт 404 JSON для неизвестных `/api/*`; новые маршруты `im_*` обязаны быть зарегистрированы в `build_router` (живой тест `http_route_parity` ловит). Матрицу `role_endpoint_matrix.rs` и карту employee-маршрутов (`ui/scripts/fixtures/employee-allowed-commands.json`, тест `employee_allowed_fixture_sound`) проверить: `im_send`/`im_sync` доступны сотруднику по членству — фикстура и серверная проверка должны сойтись; фикстуру сверить с сервером тестом.

## State of the Art

| Old | Current | Impact |
|-----|---------|--------|
| `Lagged => continue` (молчаливая потеря) | `resync_required` + ресинк подписчиков | Закрывает долг F8/R7-06 (аудит v1.4) |
| Сервер только пушит, не слышит клиента | `hello`, ping/deadline, приём `ping` клиента | Мёртвые соединения закрываются |
| Событие = источник истины | Сигнал + курсор (`rev`) | Корректность не зависит от потери событий |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Браузер не даёт JS наблюдать протокольные ping/pong, поэтому клиентская проба полуоткрытого сокета нужна текстовым `ping`/`pong` | Pattern 4 | Если проба не нужна, лишний код; если нужна и не сделана — D-15 не лечит сон ноутбука с полуоткрытым сокетом |
| A2 | 3 автоповтора с 1/2/4 с | Идемпотентность | Мелкая настройка (Discretion) |
| A3 | `std::Mutex<HashMap>` достаточно вместо `dashmap` | Standard Stack | При росте нагрузки — заменить; новая зависимость требует slopcheck |
| A4 | Порог `reload:true` порядка 200 изменённых сообщений, страница ~50 | (Discretion) | Влияет на трафик, не на корректность |
| A5 | Имена целей `--test server` и пр. | Validation | Планировщик сверяет с `Cargo.toml` |
| A6 | `crypto.randomUUID` недоступен по `http://` в LAN; нужен fallback | Don't Hand-Roll | Если LAN-режим только https — fallback избыточен; без fallback id не создастся и идемпотентность тихо исчезнет |

## Open Questions (RESOLVED)

1. **Раздельные V058/V059 или одна миграция?** RESOLVED — две миграции, каждая со своим `PRAGMA user_version` (план 43-01).
   - Known: обе только CREATE/ADD COLUMN/INDEX.
   - Recommendation: две (ядро im_* и ключи идемпотентности чужих таблиц отдельно), каждая со своим `user_version`.
2. **Как исключённому отдать «беседу в списке только для чтения» в `im_sync`?** RESOLVED — сводка с `read_only:true, left_seq`, окно `seq<=left_seq` и `rev<=left_seq`, один раз; `removed` не вводится (план 43-04).
   - Recommendation: сводка с `read_only:true, left_seq`, один раз целиком, дальше пусто (Pattern 2); `removed:[ids]` не вводить.
3. **Где держать `WsTimings` — поле `AppCtx` или `Extension`?** RESOLVED — `Option<Extension<WsTimings>>`, по умолчанию 25/60 с, подставляется только в тестах (план 43-02).
   - Recommendation: `Option<Extension<WsTimings>>` по образцу `ServerShutdown` (роутер поднимают и без `start_server`), по умолчанию 25/60 с.
4. **Реестр ресинка: освобождения.** RESOLVED — все подписчики получают ресинк либо запись в EXEMPTIONS гейта с причиной (планы 43-05, 43-07). Нужны ли подписчики, которым ресинк не нужен (напр. тосты)? Рекомендация: все 13 получают ресинк либо явное освобождение с причиной в гейте.
5. **Тост 5 с на фоне `visibilitychange`:** RESOLVED — принято, тост появится при возврате на вкладку (план 43-03). вкладка в фоне троттлит таймеры браузера; допустимо — тост появится при возврате на вкладку. Принять.
6. **ADR:** RESOLVED — раздел «Протокол доставки (Фаза 43)» в ADR 0003 (план 43-11). рекомендовать раздел в ADR 0003 (протокол кадров, тайминги, семантика replay, правило исключённого) — единый документ для Фазы 44.

## Environment Availability

| Dependency | Required By | Available | Notes |
|------------|------------|-----------|-------|
| Rust toolchain / cargo | все Rust-тесты | да (по проекту) | линковка на macOS: префикс `DEVELOPER_DIR=/Library/Developer/CommandLineTools` при exit 69 |
| pnpm + Node | UI-гейты, `ui/dist` | да (по проекту) | `pnpm --dir ui build` перед браузерной проверкой |
| `timeout` (GNU) | — | **нет** | не использовать обёртку |
| Windows / AD | не требуется для фазы | — | десктоп-мост проверяется на macOS; UAT на Windows — по решению пользователя |

Статус наличия инструментов взят из памяти проекта; в этой сессии версии не пробовались (`[ASSUMED]`).

## Sources

### Primary (HIGH — репозиторий, прочитано в сессии)
- `crates/trackly-app/src/http/ws.rs`, `main.rs` (296–345), `desktop_event_bridge.rs`, `services/entities_broadcast.rs`, `services/chat_service.rs::send`, `services/request_service.rs::create`, `dto/request.rs`, `dto/chat.rs`, `tauri_cmds/chat.rs`, `http/chat.rs`
- `packages/transport/src/ws.ts`, `ui/scripts/check-ws-disconnect-toast.mjs`, `ui/src/packages-compat.ts`, `ui/package.json` (lint-цепочка)
- `crates/trackly-app/tests/http_route_parity.rs`, `suite_wiring.rs`, `chat_ws_fanout.rs`, `ws_close_on_server_stop.rs`
- `migrations/V054__request_chat.sql`, `V057__history_events.sql`
- `docs/adr/0003-chat-model-fts-delivery.md`, `.planning/research/ARCHITECTURE.md` §3–§4, `PITFALLS.md` C-5, `43-CONTEXT.md`, `REQUIREMENTS.md`
- `Cargo.toml` (tokio без `test-util`, axum 0.8 `ws`, uuid v4; нет dashmap)

### Secondary / Tertiary
- Поведение браузеров (Pong без доступа из JS, троттлинг таймеров фоновой вкладки) — знания обучения, внешне в сессии не проверялось (`[ASSUMED]`, A1).

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — зависимостей не добавляется, всё сверено с Cargo.toml
- Architecture: HIGH — следует принятому ADR 0003 и коду; решения D-01..D-15 зафиксированы
- Pitfalls: HIGH для проектных (источники — память проекта и код), MEDIUM для браузерных нюансов
- Целевые имена тестовых бинарников: LOW — сверить с `Cargo.toml`

**Research date:** 2026-11-26
**Valid until:** 2026-12-26 (код меняется между фазами; перепроверить номера строк и счётчик `onWsEvent` перед планированием)
