---
phase: 43
slug: reliable-event-delivery
status: verified
threats_open: 0
asvs_level: 1
created: 2026-11-27
---

# Фаза 43 — Безопасность

> Контракт безопасности фазы: реестр угроз, принятые риски, журнал аудита.
> Реестр составлен при планировании (`register_authored_at_plan_time: true`), 21 план, 84 угрозы.
> Каждая угроза с решением `mitigate` проверена по коду HEAD (`837fb760`), а не по SUMMARY и не по планам.
> Поздние планы 43-13..43-21 закрывали пробелы и меняли поведение ранних планов. Поэтому доказательства
> ниже взяты из текущего кода.

---

## Границы доверия

| Граница | Описание | Что пересекает |
|---------|----------|----------------|
| клиент -> WS-цикл (`/api/v1/ws`) | клиент присылает произвольные кадры; сервер шлёт в сокет, у которого может не быть читателя | служебные кадры ping/pong/hello/resync_required, сигналы `im_changed` (без тела) |
| клиент -> `/api/v1/im_send`, `/api/v1/im_sync` | id беседы, `clientMsgId`, курсоры, limit, текст приходят от пользователя | текст сообщений, состав бесед (конфиденциально в пределах членства) |
| клиент -> `chat_send`, `chat_discard_drafts`, `requests_create` | клиентские id идемпотентности и id черновиков | текст чата заявки, вложения, заявки |
| ImHub -> сокет пользователя | сигнал маршрутизируется по списку участников | `{conversationId, rev}` |
| БД -> окно видимости | граница исключённого (`left_seq`) определяет, что может прочитать бывший участник | история беседы |
| сервер/прокси -> клиент HTTP | тело ответа не доверяется | коды ошибок |
| артефакты фазы -> публичный репозиторий | ADR, VALIDATION, SUMMARY, протоколы UAT | риск утечки реальных данных |

---

## Реестр угроз

Статус: `closed` — защита найдена в коде (файл:строка); для `accept` — запись в журнале принятых рисков ниже.

| Threat ID | Кат. | Компонент | Решение | Доказательство | Статус |
|-----------|------|-----------|---------|----------------|--------|
| T-43-01-01 | T | im_sqlite.rs | mitigate | `crates/trackly-infra/src/repos/im_sqlite.rs:248-421`: все значения передаются через `params!`; `format!` подставляет только константы `MSG_COLS`/`WINDOW_SQL` (:108-111) | closed |
| T-43-01-02 | I | ключи идемпотентности | mitigate | `migrations/V058__im_core.sql:57` UNIQUE(conversation_id, author_user_id, client_msg_id); `V059__idempotency_keys.sql:10-18` (request_id, author) и (requested_by, id); поиск `im_sqlite.rs:248-264` по тройке | closed |
| T-43-01-03 | I | окно исключённого | mitigate | `trackly-core/src/domain/im.rs:69-75` `member_window` (hi_seq = max_rev = left_seq); `WINDOW_SQL` `im_sqlite.rs:108-111` (`seq <= ?3 AND rev <= ?4`); тест с прямым UPDATE rev `trackly-infra/tests/im_repo.rs:127-139` | closed |
| T-43-01-04 | D | пагинация | mitigate | `domain/im.rs:14,50-55` `clamp_page_limit` -> 1..=200 | closed |
| T-43-01-SC | T | зависимости | accept | см. AR-01 | closed |
| T-43-02-01 | D | handle_ws_socket | mitigate | `crates/trackly-app/src/http/ws.rs:77` SEND_TIMEOUT 5 с, `:86` IDLE_TIMEOUT 60 с, `:109-114` `send_msg` через `timeout`, `:259-266` ветка idle закрывает сокет, `:344` сброс idle на любом кадре | closed |
| T-43-02-02 | T | клиентский текст | mitigate | `ws.rs:89` CLIENT_TEXT_MAX=256, `:121-126` `is_client_ping` (длина + `type=="ping"`), `:345-353` прочее игнорируется | closed |
| T-43-02-03 | I | resync_required | accept | см. AR-02 | closed |
| T-43-02-04 | D | Lagged-лавина | mitigate | `ws.rs:322-328` (общий канал) и `:290-296` (IM): один `resync_required` на каждое Lagged; ёмкость `im_hub.rs:15` = 256, не увеличена; клиентский дебаунс — T-43-03-01 | closed |
| T-43-02-05 | E | ws auth | mitigate | `ws.rs:138-145` `ws_auth_middleware` -> 401; подключён `route_layer` `ws.rs:368`, маршрут смонтирован `http/mod.rs:202` | closed |
| T-43-03-01 | D | resyncAll | mitigate | `packages/transport/src/ws.ts:92` RESYNC_DEBOUNCE_MS=500, `:161-174` схлопывание; на первом открытии ресинка нет (`:262` только при `reconnecting`); число подписчиков держит `ui/scripts/check-ws-resync-registry.mjs:24` | closed |
| T-43-03-02 | T | onmessage | mitigate | `ws.ts:235-255` `JSON.parse` в try/catch, неизвестный `type` уходит в `dispatch`, получатели фильтруют по типу | closed |
| T-43-03-03 | D | утечка слушателей/таймеров | mitigate | `ws.ts:439-460` `activeCleanup` снимает `online`/`visibilitychange`, пробу, reconnectTimer, таймер тоста; `:490-501` `teardown` гасит таймер дебаунса; гейт `check-ws-resync.mjs` (19 мутантов, прогон 2026-11-27 зелёный) | closed |
| T-43-03-04 | I | служебные кадры | accept | см. AR-03 | closed |
| T-43-04-01 | I | ImHub.publish | mitigate | получатели читаются в транзакции писателя `im_service.rs:174` (`active_member_ids_on`), публикация после коммита `:186-200`; канал на пользователя `im_hub.rs:41-56` (в несуществующий канал не пишет); сигнал без тела `im_hub.rs:18-23`; тест `tests/im_membership.rs:28` | closed |
| T-43-04-02 | E | send/sync | mitigate | членство в замыкании писателя `im_service.rs:127-133` -> NotFound (без исключения для администратора); sync строит список от членства `:351-352`, чужие id курсоров не используются; тест `tests/im_membership.rs:10` | closed |
| T-43-04-03 | I | sync исключённого | mitigate | `im_service.rs:363-394`: все выборки исключённого идут через `member_window(.., Some(left_seq))`; тесты `tests/im_sync.rs:159,201,275,299` | closed |
| T-43-04-04 | S | client_msg_id | mitigate | ключ (беседа, автор, id) `im_service.rs:139-143`; `to_dto` отдаёт `client_msg_id` только автору `:51-60`; тест `tests/im_idempotent_send.rs:54,80` | closed |
| T-43-04-05 | D | sync | mitigate | `clamp_page_limit` `im_service.rs:338`; MAX_CURSORS=500 `:29,340`; порог reload `SYNC_RELOAD_THRESHOLD` `:370,412`; очередь писателя `trackly-infra/src/db/writer_worker.rs:31,34` (mpsc 256, таймаут 5 с) | closed |
| T-43-04-06 | T | текст сообщения | mitigate | `normalize_text` + `exceeds_limit` `im_service.rs:111-118` (`domain/chat.rs:8` MAX_TEXT_CHARS=4000); тело через `serde_json::json!` `domain/im.rs:29-41` | closed |
| T-43-05-01 | I | ресинк | accept | см. AR-04 | closed |
| T-43-05-02 | D | лавина после реконнекта | mitigate | дебаунс `ws.ts:161-174`; MIN_SUBSCRIBERS=13 `check-ws-resync-registry.mjs:24`, гейт подключён в `ui/package.json` (lint) | closed |
| T-43-05-03 | R | забытый подписчик | mitigate | `check-ws-resync-registry.mjs:57-71` нижняя граница + проверка освобождений; selftest M1-M7 пойман (прогон 2026-11-27) | closed |
| T-43-06-01 | I | replay | mitigate | чат: `load_access` до replay `chat_service.rs:484`, поиск по (заявка, автор, id) `:514-517`; заявки: `authorize` `request_service.rs:351`, поиск по (автор, id) `:370,486`; тест `tests/chat_idempotent_send.rs:111`, `tests/request_idempotent_create.rs:108` | closed |
| T-43-06-02 | T | client id | mitigate | `is_valid_client_id` `domain/im.rs:20-25`; вызов `chat_service.rs:475-482`, `request_service.rs:357-363`; запросы параметризованы `chat_sqlite.rs:135-165`, `requests_sqlite.rs:136-162` | closed |
| T-43-06-03 | R | дубликат аудита | mitigate | replay возвращается до записи аудита и до `ws_tx.send`: `chat_service.rs:606-619`, `request_service.rs:520-523`; тесты `chat_idempotent_send.rs:34`, `request_idempotent_create.rs:44` (одна строка, один аудит, одно событие) | closed |
| T-43-06-04 | E | replay на закрытой заявке | mitigate | replay до проверки статуса отдаёт только сохранённое `chat_service.rs:512-517`; новая отправка -> `Written::Closed` `:532,621-625`; тест `chat_idempotent_send.rs:69` | closed |
| T-43-07-01 | I | ресинк чата | accept | см. AR-05 | closed |
| T-43-07-02 | R | освобождения в EXEMPTIONS | mitigate | `check-ws-resync-registry.mjs:25` MIN_REASON=20, `:62-71` короткая причина и мёртвая запись краснят гейт; мутант M7 `:149` | closed |
| T-43-07-03 | D | лавина запросов | mitigate | дебаунс `ws.ts:92,161-174` | closed |
| T-43-07-04 | T | ресинк затирает ввод | mitigate | обработчик ресинка чата только перечитывает ленту `ui/src/features/requests/chat/RequestChat.svelte:270-272` (`load(false)`), состояние композера не трогает; Layout только поднимает счётчики `Layout.svelte:35-37`; живая UAT 43-12 | closed |
| T-43-08-01 | I | IM-сигнал | mitigate | `im_hub.rs:18-23` только `{conversationId, rev}`; `ws.rs:273-302` сигнал из канала пользователя; тесты реального сокета `tests/im_ws_fanout.rs:125,167` | closed |
| T-43-08-02 | E | im_send/im_sync | mitigate | `http/im.rs:23,35` `session_identity` на входе; членство в сервисе (T-43-04-02); тесты 401/404 `tests/im_http.rs:104-172` | closed |
| T-43-08-03 | S | WS без сессии | mitigate | `ws.rs:138-145,368` (без изменений) | closed |
| T-43-08-04 | D | Lagged IM | mitigate | `ws.rs:290-296` resync_required; ёмкость `im_hub.rs:15` фиксирована; `prune` `ws.rs:209`, `im_hub.rs:58-60`; тест `im_ws_fanout.rs:209` | closed |
| T-43-08-05 | I | отсутствующий маршрут | mitigate | `http/mod.rs:294-305` JSON 404 для `/api/*`; регрессия `tests/im_http.rs:214` | closed |
| T-43-09-01 | T | newClientId | mitigate | `packages/transport/src/sendWithRetry.ts:59-72` (randomUUID / hex / base36 с `-`, обрезка до 64); сервер проверяет независимо (T-43-06-02) | closed |
| T-43-09-02 | D | автоповторы | mitigate | `sendWithRetry.ts:9` три паузы, `:14-19` повтор только NETWORK_ERROR/502/503/504, `:52` выход после исчерпания | closed |
| T-43-09-03 | R | дубль заявки при повторе | mitigate | `ui/src/features/requests/RequestFormModal.svelte:339-349` один `attemptId` в замыкании `sendWithRetry`; сервер `request_service.rs:376-378,484-489`; тест `request_idempotent_create.rs:155` | closed |
| T-43-10-01 | T | текст в пузыре | mitigate | `ChatPendingMessage.svelte:22` интерполяция `{item.text}`; `{@html` в `ui/src/features/requests/chat/` не найден (grep) | closed |
| T-43-10-02 | R | дубль сообщения при повторе | mitigate | `pendingStore.svelte.ts:51-54` тот же `clientMsgId` во всех попытках, `:82` id выдаётся один раз; сервер T-43-06-*; гейт `check-chat-pending-behavior.mjs` (18 мутантов, зелёный) | closed |
| T-43-10-03 | I | очередь в памяти | accept | см. AR-06 | closed |
| T-43-10-04 | D | двойной «Повторить» | mitigate | `pendingStore.svelte.ts:97-98` повтор только из статуса `failed` | closed |
| T-43-11-01 | I | ADR и VALIDATION | mitigate | `scripts/check-privacy.mjs` в `.githooks/pre-commit` и `.github/workflows/ci-fast.yml:31`; прогон по HEAD 2026-11-27: PASS, 0 нарушений; ручной поиск шаблона «Фамилия И.И.» в артефактах фазы и ADR 0003 нашёл только вымышленные имена | closed |
| T-43-11-02 | R | расхождение документа и кода | mitigate | `docs/adr/0003-chat-model-fts-delivery.md:34-63` сверены с кодом: 25/60/5 с (`ws.rs:77,83,86`), 256 байт (`ws.rs:89`), 500 мс / 5 с / 5 с (`ws.ts:90-94`), 64 символа, 50/200/200, 500 курсоров (`domain/im.rs:10-17`, `im_service.rs:29`) | closed |
| T-43-12-01 | I | протокол UAT | mitigate | `43-12-SUMMARY.md` обезличен; `check-privacy.mjs` PASS по HEAD | closed |
| T-43-12-02 | R | автоодобрение чекпоинтов | mitigate | `43-12-SUMMARY.md:13,21,48`: оба human-verify закрыты собственным ответом пользователя, исполнитель-агент не запускался | closed |
| T-43-13-01 | I | sync, ветка исключённого | mitigate | `im_service.rs:363-394` дельта, сводка и reload — все через `member_window(.., Some(left_seq))`; тесты `im_sync.rs:201,241,275,299` | closed |
| T-43-13-02 | I | im_sync, чужие беседы | mitigate | `im_service.rs:351-352` список от `conversations_of_user_on`; тест `im_sync.rs:111` `foreign_conversation_is_not_disclosed` в прогоне | closed |
| T-43-13-03 | E | im_send после исключения | mitigate | `im_service.rs:139-146` replay по (беседа, автор, id) до `can_post`, новая запись требует `can_post`; тесты `im_idempotent_send.rs:112`, `im_membership.rs:97` | closed |
| T-43-13-04 | R | replay без аудита и сигнала | accept | см. AR-07 | closed |
| T-43-13-SC | T | менеджер пакетов | accept | см. AR-01 | closed |
| T-43-14-01 | D | лавина переподключений | mitigate | `ws.ts:227` `connectBrowser` не открывает второй сокет; `:285-291,317` один `reconnectTimer`, прежний гасится в `handleLost`; R12/R13 в `check-ws-resync.mjs` | closed |
| T-43-14-02 | I | дубли WsEvent | mitigate | `ws.ts:233,259,275` обработчики замещённого сокета ничего не делают; `:359` проба не трогает чужой сокет | closed |
| T-43-14-03 | T | hello.protocol | mitigate | `ws.ts:241-248` предупреждение при `protocol !== WS_PROTOCOL_VERSION`, кадр не диспатчится | closed |
| T-43-14-SC | T | менеджер пакетов | accept | см. AR-01 | closed |
| T-43-15-01 | D | автоповторы на постоянный отказ | mitigate | `sendWithRetry.ts:9,14-19,41-56` | closed |
| T-43-15-02 | I | message синтетической AppError | mitigate | `packages/transport/src/client.ts:76-80` message только с кодом статуса, тело не включается | closed |
| T-43-15-03 | R | потеря хуков 401/403 | mitigate | `client.ts:84-90` хуки по `res.status`; гейт `check-transport-hooks.mjs` (5 мутантов, зелёный) | closed |
| T-43-15-SC | T | менеджер пакетов | accept | см. AR-01 | closed |
| T-43-16-01 | T | привязка чужих черновиков | mitigate | `crates/trackly-infra/src/repos/chat_sqlite.rs:517-538` `promote_drafts_on`: для каждого id проверяются parent_id, автор, `message_id IS NULL`, иначе Forbidden; привязываются только явно переданные id | closed |
| T-43-16-02 | I | текст в очереди | accept | см. AR-08 | closed |
| T-43-16-03 | I | вывод текста в пузыре | mitigate | `ChatPendingMessage.svelte:22` интерполяция, `{@html` нет | closed |
| T-43-16-04 | D | осиротевшие черновики | accept | см. AR-09 | closed |
| T-43-16-SC | T | менеджер пакетов | accept | см. AR-01 | closed |
| T-43-17-01 | T | подмена содержимого повтором | mitigate | `ui/src/features/requests/requestIdPolicy.ts` `requestSignature` + `createRequestIdIssuer` (новый id при смене подписи); `RequestFormModal.svelte:149,339`; гейт `check-request-create-idempotency.mjs` (10 мутантов, зелёный) | closed |
| T-43-17-02 | R | дубль заявки | mitigate | тот же id при неизменной подписи `requestIdPolicy.ts`; сервер V059 `uq_requests_client`; правила I4/I5 гейта | closed |
| T-43-17-03 | S | подбор чужого clientRequestId | accept | см. AR-10 | closed |
| T-43-17-SC | T | менеджер пакетов | accept | см. AR-01 | closed |
| T-43-18-01 | D | цикл эффектов | mitigate | `ui/src/features/places/PlaceTree.svelte:306-318` сброс внутри существующего эффекта, запись `statsCache = {}` без чтения (`:344-348`); INV-12 в `check-place-tree-invalidation.mjs:113,636-661`, прогон зелёный | closed |
| T-43-18-02 | I | устаревшие счётчики | mitigate | `Layout.svelte:35-37` -> `notifyEntitiesChanged([], {all:true})` -> `placeContentEvents.svelte.ts:88` `resetSeq += 1` -> `PlaceTree.svelte:312-316` `resetStatsCache()` | closed |
| T-43-18-03 | D | шторм запросов статистики | accept | см. AR-11 | closed |
| T-43-18-SC | T | менеджер пакетов | accept | см. AR-01 | closed |
| T-43-19-01 | I | clientMsgId чужих сообщений | mitigate | `crates/trackly-app/src/services/chat_service.rs:80,91-95` поле только при `is_own`; тест `tests/chat_feed.rs:340-369` (у не-автора и у администратора `None`) | closed |
| T-43-19-02 | I | расширение MSG_SELECT | accept | см. AR-12 | closed |
| T-43-19-03 | T | новая колонка в запросе | mitigate | `chat_sqlite.rs:66-67` константа `MSG_SELECT`, значения через `params!` `:143-147` | closed |
| T-43-19-SC | T | менеджер пакетов | accept | см. AR-01 | closed |
| T-43-20-01 | T | снятие записи очереди | mitigate | `pendingStore.svelte.ts:123-128` фильтр: та же заявка И `status === 'failed'` И совпавший `clientMsgId`; кейсы P15/P15b/P15c в `check-chat-pending-behavior.mjs` | closed |
| T-43-20-02 | R | потеря неотправленного текста | mitigate | сверка вызывается только с id из ответа ленты сервера `RequestChat.svelte:187-192`; сервер отдаёт `clientMsgId` только своим сообщениям (T-43-19-01) | closed |
| T-43-20-SC | T | менеджер пакетов | accept | см. AR-01 | closed |
| T-43-21-01 | I | протокол живой проверки | mitigate | `43-21-SUMMARY.md` описывает вымышленные данные и временную БД; `check-privacy.mjs` PASS по HEAD; временная доработка стенда в репозиторий не попала | closed |
| T-43-21-02 | R | самозакрытие чекпоинта | mitigate | `43-21-SUMMARY.md:14,66`: замена ручной проверки стендом сделана по прямой просьбе пользователя и записана как отклонение. Исполнитель чекпоинт сам не закрывал. Ограничение: десктопные WKWebView/WebView2 этой проверкой не покрыты (см. «Замечания») | closed |
| T-43-21-03 | T | тестовые данные в боевой БД | accept | см. AR-13 | closed |
| T-43-21-SC | T | менеджер пакетов | accept | см. AR-01 | closed |

*Статус: open · closed*
*Решение: mitigate (нужна реализация) · accept (задокументированный риск) · transfer (третья сторона)*

---

## Журнал принятых рисков

| Risk ID | Ссылка на угрозу | Обоснование | Принял | Дата |
|---------|------------------|-------------|--------|------|
| AR-01 | T-43-01-SC, T-43-13-SC, T-43-14-SC, T-43-15-SC, T-43-16-SC, T-43-17-SC, T-43-18-SC, T-43-19-SC, T-43-20-SC, T-43-21-SC | Новых зависимостей нет. С первого коммита фазы (`f62cf5c9~1..HEAD`) в `Cargo.toml`, `Cargo.lock`, `pnpm-lock.yaml` изменений нет. В `ui/package.json` изменилась только строка `lint`: в неё добавлены гейты фазы | планировщик фазы (register_authored_at_plan_time), подтверждено аудитором | 2026-11-27 |
| AR-02 | T-43-02-03 | `{"type":"resync_required"}` не несёт данных и не проходит видимость WsEvent (`ws.rs:293,326`) | планировщик фазы | 2026-11-27 |
| AR-03 | T-43-03-04 | Служебные кадры hello/pong/resync_required без данных; тело сообщений по WS не ходит (сигнал IM — только id и rev) | планировщик фазы | 2026-11-27 |
| AR-04 | T-43-05-01 | Ресинк перечитывает данные теми же авторизованными вызовами, что и обычная загрузка; новых данных клиенту не раскрывается | планировщик фазы | 2026-11-27 |
| AR-05 | T-43-07-01 | Перечитывание ленты чата авторизуется сервером по доступу к заявке (`authorize`), как и раньше | планировщик фазы | 2026-11-27 |
| AR-06 | T-43-10-03 | Очередь неотправленных сообщений живёт только в памяти вкладки и сбрасывается при смене пользователя (`pendingStore.svelte.ts:132`, `registerUserScopedReset`) | планировщик фазы | 2026-11-27 |
| AR-07 | T-43-13-04 | Replay IM не пишет аудит и не шлёт сигнал (ADR 0003, раздел 6). Повтор не меняет состояние | планировщик фазы | 2026-11-27 |
| AR-08 | T-43-16-02 | Как AR-06: текст в очереди только в памяти вкладки | планировщик фазы | 2026-11-27 |
| AR-09 | T-43-16-04 | Устаревшие черновики чистятся сервером при старте: `context.rs:351` -> `chat_service.rs:683` `purge_stale_drafts(24 ч)`. Объём ограничен лимитом вложений | планировщик фазы | 2026-11-27 |
| AR-10 | T-43-17-03 | Ключ привязан к автору (`requested_by_user_id, client_request_id`, V059). Чужую заявку по id не получить | планировщик фазы | 2026-11-27 |
| AR-11 | T-43-18-03 | Счётчики догружаются лениво и только для видимых узлов; ресинк дебаунсится (`RESYNC_DEBOUNCE_MS`) | планировщик фазы | 2026-11-27 |
| AR-12 | T-43-19-02 | Клиентский id не секрет и данных других пользователей не раскрывает. Наружу отдаётся только автору (T-43-19-01) | планировщик фазы | 2026-11-27 |
| AR-13 | T-43-21-03 | Живая проверка шла на временной БД стенда с вымышленными заявками и сообщениями | планировщик фазы | 2026-11-27 |

*Принятые риски не поднимаются повторно в следующих прогонах аудита.*

---

## Флаги угроз из SUMMARY

| Источник | Флаг | Сопоставление |
|----------|------|---------------|
| 43-01, 43-04 | «Нет новых поверхностей» | информационно |
| 43-02 | закрыты T-43-02-01/02/04 | соответствует реестру |
| 43-03 | закрыты T-43-03-01/02/03 | соответствует реестру |
| 43-06 | закрыты T-43-06-01..04 | соответствует реестру |
| 43-08 | закрыты T-43-08-01..05 | соответствует реестру |
| 43-11 | «Нет» | информационно |

Незарегистрированных флагов нет.

---

## Замечания аудитора (не блокируют, угроз реестра не открывают)

1. **T-43-21-02.** Живую проверку A–F провёл агент на стенде вместо человека. Так попросил сам пользователь (`43-21-SUMMARY.md:14,66`). Десктопные WebView (WKWebView/WebView2) не проверялись, поэтому сценарии gap-closure 43-13..43-20 в десктопе живьём не подтверждены. Десктоп по-прежнему проверен только UAT плана 43-12, на сборке до закрытия пробелов.
2. **CI.** На коммите `d8a749f4` (до планов 43-13..43-21) `ci-full`/`ci-fast` были красными на шаге `pnpm lint`, при этом `cargo test` на macOS прошёл. На HEAD (`837fb760`) прогоны ещё шли во время аудита. Локально 2026-11-27 зелёные все восемь гейтов фазы (`check-ws-resync`, `check-ws-resync-registry`, `check-send-retry`, `check-request-create-idempotency`, `check-chat-pending-behavior`, `check-chat-ui`, `check-place-tree-invalidation`, `check-transport-hooks`, все с `--selftest`) и `check-privacy`. `cargo test` аудитор не запускал: серверные защиты подтверждены чтением кода и наличием тестов.
3. Три WARNING повторного ревью (`43-REVIEW.md`: кнопка «В поле ввода» на закрытой заявке, стирание файлов при загрузке, сверка очереди только при загрузке ленты) касаются потери пользовательского ввода, а не границ доверия. Угроз реестра они не открывают; в фазе-получателе их стоит закрыть одним пакетом.

---

## Журнал аудита безопасности

| Дата аудита | Всего угроз | Закрыто | Открыто | Кто проводил |
|-------------|-------------|---------|---------|--------------|
| 2026-11-27 | 84 | 84 (62 mitigate + 22 accept) | 0 | gsd-security-auditor (ASVS L1, block_on: high) |

---

## Подпись

- [x] У всех угроз есть решение (mitigate / accept / transfer)
- [x] Принятые риски записаны в журнал
- [x] `threats_open: 0` подтверждено
- [x] `status: verified` выставлен во frontmatter

**Утверждение:** verified 2026-11-27
