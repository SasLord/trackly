---
phase: 43-reliable-event-delivery
reviewed: 2026-11-27T01:38:51Z
depth: standard
files_reviewed: 47
files_reviewed_list:
  - crates/trackly-app/src/context.rs
  - crates/trackly-app/src/desktop_event_bridge.rs
  - crates/trackly-app/src/dto/im.rs
  - crates/trackly-app/src/dto/mod.rs
  - crates/trackly-app/src/dto/request.rs
  - crates/trackly-app/src/http/chat.rs
  - crates/trackly-app/src/http/health.rs
  - crates/trackly-app/src/http/im.rs
  - crates/trackly-app/src/http/mod.rs
  - crates/trackly-app/src/http/ws.rs
  - crates/trackly-app/src/main.rs
  - crates/trackly-app/src/services/chat_service.rs
  - crates/trackly-app/src/services/im_hub.rs
  - crates/trackly-app/src/services/im_service.rs
  - crates/trackly-app/src/services/mod.rs
  - crates/trackly-app/src/services/request_service.rs
  - crates/trackly-app/src/tauri_cmds/chat.rs
  - crates/trackly-app/src/tauri_cmds/health.rs
  - crates/trackly-core/src/domain/im.rs
  - crates/trackly-core/src/domain/mod.rs
  - crates/trackly-infra/src/repos/chat_sqlite.rs
  - crates/trackly-infra/src/repos/im_sqlite.rs
  - crates/trackly-infra/src/repos/mod.rs
  - crates/trackly-infra/src/repos/requests_sqlite.rs
  - migrations/V058__im_core.sql
  - migrations/V059__idempotency_keys.sql
  - packages/attachments/src/AttachmentsBlock.svelte
  - packages/attachments/src/attachmentEvents.svelte.ts
  - packages/transport/src/sendWithRetry.ts
  - packages/transport/src/ws.ts
  - ui/src/bindings-phase6.ts
  - ui/src/features/employee/EmployeeDevicesPage.svelte
  - ui/src/features/layout/EmployeeLayout.svelte
  - ui/src/features/layout/Layout.svelte
  - ui/src/features/printers/PrintersPage.svelte
  - ui/src/features/requests/RequestDetail.svelte
  - ui/src/features/requests/RequestFormModal.svelte
  - ui/src/features/requests/RequestsPage.svelte
  - ui/src/features/requests/chat/ChatComposer.svelte
  - ui/src/features/requests/chat/ChatPendingMessage.svelte
  - ui/src/features/requests/chat/RequestChat.svelte
  - ui/src/features/requests/chat/api.ts
  - ui/src/features/requests/chat/pendingStore.svelte.ts
  - ui/src/features/requests/chat/sendOutcomeText.ts
  - ui/src/features/requests/chat/unreadStore.svelte.ts
  - ui/src/features/settings/OrgSettings.svelte
  - ui/src/lib/components/NumberTemplateField.svelte
findings:
  critical: 3
  warning: 7
  info: 8
  total: 18
status: issues_found
---

# Phase 43: Code Review Report

**Reviewed:** 2026-11-27T01:38:51Z
**Depth:** standard
**Files Reviewed:** 47
**Status:** issues_found

## Narrative Findings (AI reviewer)

## Summary

Проверены изменения с `4f1e9a20` по 47 файлам: WS-протокол (hello/ping/idle/resync_required), ImHub/ImService, идемпотентность `chat_send`/`requests_create`/`im_send`, клиентский реестр ресинка, отложенный тост обрыва, очередь неотправленных сообщений чата. Код сверялся с разделом «Протокол доставки (Фаза 43)» в ADR 0003.

Что сделано правильно: auth на новых маршрутах (`session_identity` в обоих хендлерах IM, 403 без `user_id`). Получатели сигнала `im_changed` берутся из активных участников внутри транзакции записи. Replay не публикует события и не пишет аудит. Ветка `Lagged` в обоих каналах отдаёт `resync_required`. Подписки `onWsResync` во всех компонентах снимаются в cleanup.

Найдено три дефекта уровня Critical:
1. Исключённый участник, который успел дочитать беседу, никогда не узнаёт через `im_sync`, что беседа стала read-only. Это прямо противоречит D-05 в спецификации.
2. Отказ одной отправки в композере стирает черновики вложений у ДРУГОГО сообщения, которое лежит в очереди «Не отправлено». Повтор потом даёт 403, и текст сообщения теряется.
3. Таймер пробы живости в `ws.ts` переживает `onclose`. Это ведёт к двойному таймеру переподключения и двум живым сокетам, то есть к дублю событий (класс GAP-12-10).

## Critical Issues

### CR-01: Исключённый участник с актуальным курсором никогда не получает `read_only` через `im_sync`

**File:** `crates/trackly-app/src/services/im_service.rs:359-362` (и `remove_member`, строки 315-318)
**Issue:** При исключении `left_seq = last_seq` беседы на момент выхода. Клиент, дочитавший беседу, держит курсор `rev == last_seq == left_seq`. В ветке исключённого стоит `if cursor.is_some_and(|c| c >= left_seq) { continue; }`, поэтому беседа в ответе отсутствует. Клиент не получает ни `read_only: true`, ни `leftSeq` и продолжает считать беседу активной. Узнать о выходе он может только через 403 на `im_send`. При этом ADR (раздел 8, D-05) обещает: «исключённый узнаёт при следующем `im_sync` или реконнекте». Именно этот случай (дочитавший клиент, а это обычное состояние) обещание нарушает. После реконнекта ресинк идёт с теми же курсорами, так что и он не помогает. Тест `removed_member_gets_frozen_history_once` проверяет только путь «курсора нет», а потом «курсор = left_seq уже после применения reload». Случай «дочитал, затем исключили» не покрыт.
**Fix:** Беседу исключённого пропускать только тогда, когда клиент уже знает о выходе. Курсор этого не передаёт, поэтому нужен признак в курсоре или ответ хотя бы со сводкой:
```rust
Some(left_seq) if m.left_at_utc.is_some() => {
    let known_left = req_cursor.map(|c| c.known_left_seq == Some(left_seq)).unwrap_or(false);
    if known_left { continue; }
    // курсор >= left_seq: сообщений не слать, но сводку с read_only отдать
    let msgs = if cursor.is_some_and(|c| c >= left_seq) { vec![] }
               else { repo.messages_last_page_on(&tx, conv.id, &w, limit)? };
    (msgs, cursor.map_or(true, |c| c < left_seq), left_seq, true)
}
```
Как минимум: при `cursor >= left_seq` отдавать сводку с `read_only: true`, `messages: []`, `reload: false`. Если повтор сводки на каждом sync нежелателен, нужно хранить у клиента признак «знаю о выходе». ADR (раздел 7-8) и тест `im_sync.rs` нужно поправить вместе с кодом.

### CR-02: Отказ отправки в композере стирает черновики вложений у сообщений в очереди «Не отправлено», и их текст теряется

**File:** `ui/src/features/requests/chat/ChatComposer.svelte:176-194`; `ui/src/features/requests/chat/pendingStore.svelte.ts:55-62`; `crates/trackly-infra/src/repos/chat_sqlite.rs:533-535`
**Issue:** Ветки `result.kind === 'error'` и `'rejected'` вызывают `chat.discardDrafts(sentFor)`. Сервер удаляет ВСЕ черновики пользователя по заявке. Среди них и черновики, которые держит уже лежащее в очереди сообщение со статусом `failed` (его `draftIds` композер намеренно не трогал: «черновики вложений нужны повтору»). Сценарий:
1. Сообщение A с файлом упало по сети, в ленте пузырь «Не отправлено».
2. Сообщение B получило отказ `too_long` (или любую ошибку).
3. `discardDrafts` удаляет черновик A.
4. Пользователь жмёт «Повторить» у A. `promote_drafts_on` видит `n != 1`, приходит `AppError::Forbidden`.
5. В `dispatch` это не сетевой сбой, поэтому `removeItem(A)`, и пузырь пропадает вместе с текстом. Пользователь видит неверное «Нет доступа к переписке этой заявки».

Это потеря введённых пользователем данных.
**Fix:** Удалять только черновики текущей отправки, а не все черновики заявки. Либо исключать `draftIds` из очереди:
```ts
const held = new Set(pending.items.filter((p) => p.requestId === sentFor).flatMap((p) => p.draftIds));
const own = draftIds.filter((id) => !held.has(id));
await attachments.deleteDrafts(own); // точечное удаление вместо chat.discardDrafts(sentFor)
```
Дополнительно: в `dispatch` при доменной ошибке на ПОВТОРЕ не удалять элемент молча. Нужно оставить `failed` с текстом или вернуть текст в черновик, чтобы он не терялся.

### CR-03: Таймер пробы живости переживает `onclose`, отсюда двойной таймер переподключения и два живых сокета

**File:** `packages/transport/src/ws.ts:263-296, 328-344`
**Issue:** `probeOrReconnect` ставит `probeTimer` на 5 с. Если за это время сокет закрылся сам, `onclose` вызывает `handleLost()`. Но `handleLost` не вызывает `clearProbe()`, а `connectBrowser` тоже не сбрасывает пробу. Дальше два варианта:
- **Проба срабатывает в окне backoff.** В этот момент `ws === null`, `reconnectTimer = T1`. Срабатывание пробы снова зовёт `handleLost()`, тот перезаписывает `reconnectTimer = T2`, а T1 остаётся без `clearTimeout`. Затем `connectNow()` гасит только T2 и открывает сокет A. Позже срабатывает T1 и `connectBrowser()` открывает сокет B. Сокет A осиротел, но его `onmessage` активен, и каждое событие диспатчится дважды. Это дубли тостов, тот же класс, что GAP-12-10. Вдобавок `onclose` сокета A обнулит `ws`, хотя B жив.
- **Проба срабатывает на новом сокете,** который ещё в CONNECTING. Она убивает здоровое переподключение.

Окно реально: возврат на вкладку (`visibilitychange`) при упавшем сервере. Браузер закрывает полуоткрытый сокет почти сразу, а backoff 2-4 с превышает остаток пробы.
**Fix:**
```ts
function handleLost(): void {
  clearProbe();
  if (reconnectTimer !== null) { clearTimeout(reconnectTimer); reconnectTimer = null; }
  ws = null;
  ...
}
// и в таймере пробы привязаться к конкретному сокету:
const probed = ws;
probeTimer = setTimeout(() => {
  probeTimer = null;
  if (ws !== probed) return; // сокет уже сменился
  ...
}, PROBE_TIMEOUT_MS);
```
Добавить в `check-ws-disconnect-toast.mjs` сценарий «onclose во время пробы».

## Warnings

### WR-01: `reload:true` для исключённого отдаёт только последнюю страницу, и клиент по контракту стирает более раннюю историю

**File:** `crates/trackly-app/src/services/im_service.rs:363-369`
**Issue:** Исключённому всегда отдаётся `messages_last_page_on(..., limit)` (по умолчанию 50, максимум 200) с `reload: true`. Контракт ADR (раздел 7) говорит: при `reload:true` клиент «заменяет локальную копию беседы». Если до выхода в беседе было больше `limit` сообщений, клиент удалит всё, что старше последней страницы. Дозагрузить это нельзя: маршрута постраничной истории в Фазе 43 нет, а при `cursor >= left_seq` беседа больше не возвращается. Получается, что D-06 «история до выхода видна» нарушается не только для изменённых сообщений (это принятое ограничение), но и для обычных. То же касается курсора, отставшего всего на 1: вместо дельты исключённый получает `reload` с усечённой страницей.
**Fix:** Для исключённого с курсором `< left_seq` отдавать дельту `rev > cursor` в окне (с тем же порогом `SYNC_RELOAD_THRESHOLD`), а не `reload`. `reload:true` ставить только при отсутствии курсора или превышении порога. Если оставить как есть, ADR должен явно предупредить Link не заменять копию для `read_only`.

### WR-02: Повтор создания заявки с тем же `clientRequestId` молча игнорирует правки формы

**File:** `ui/src/features/requests/RequestFormModal.svelte:145-154, 337-346`
**Issue:** `clientRequestId` живёт от открытия формы до успеха. Если сеть упала после коммита на сервере (ответ потерян), пользователь видит ошибку, меняет тип заявки, принтер или описание и жмёт «Отправить» снова. Сервер возвращает ИСХОДНУЮ заявку, тело повтора не сравнивается. Показывается тост «Заявка отправлена», а введённые изменения пропадают без предупреждения.
**Fix:** Перевыпускать `clientRequestId` при любом изменении полей после неудачной попытки (через `$effect` на `requestType/printerDeviceId/categoryId/description`, сбрасывающий id, если `lastAttemptFailed`). Либо сравнивать на клиенте ответ с отправленным и сообщать «заявка уже была создана ранее».

### WR-03: Композер стирает текст, набранный за время автоповторов

**File:** `ui/src/features/requests/chat/ChatComposer.svelte:170-174, 195-198`
**Issue:** `submitMessage` ждёт до 1+2+4 с пауз плюс время запросов. Поле ввода всё это время редактируемо, блокируется только кнопка (`sending`). По завершении ветка `failed` (а также `sent`) вызывает `clearDraft()`, и текст, набранный пользователем после нажатия «Отправить», теряется. До фазы 43 окно ограничивалось одним запросом, теперь оно вытянуто ретраями. Заодно на всё время повторов текст виден дважды: в пузыре «Отправляется…» и в поле ввода.
**Fix:** Очищать поле сразу после постановки в очередь (текст уже в `PendingMessage`). При последующих исходах не вызывать `clearDraft()`, если `draft.text` изменился после отправки:
```ts
const sentText = draft.text;
draft.text = ''; // сразу, текст живёт в очереди
const result = await submitMessage(sentFor, sentText, draftIds);
```

### WR-04: Очередь неотправленных не сверяется с лентой, поэтому возможен «двойник»

**File:** `ui/src/features/requests/chat/pendingStore.svelte.ts:43-63`; `ui/src/features/requests/chat/RequestChat.svelte:258-260, 478-480`
**Issue:** Если все повторы исчерпаны, а сервер сообщение сохранил (потерян ответ), событие `chat_message` или ресинк перечитывает ленту, и сообщение появляется в ней. Пузырь «Не отправлено» с тем же текстом остаётся. `ChatMessageDto` не несёт `clientMsgId` (в биндингах его нет), так что сверить их невозможно. Пользователь видит «не отправлено» и пишет сообщение заново, получается реальный дубль.
**Fix:** Отдавать `clientMsgId` автору в `ChatMessageDto`, как это сделано в `ImMessageDto`. При `load()` или ресинке снимать из очереди элементы, чей `clientMsgId` уже есть в ленте.

### WR-05: Ресинк в `Layout.svelte` не сбрасывает счётчики мест в дереве

**File:** `ui/src/features/layout/Layout.svelte:35-37`; `ui/src/lib/stores/placeContentEvents.svelte.ts:75-78`; `ui/src/features/places/PlaceTree.svelte:302-307, 326`
**Issue:** `handleResync` вызывает `notifyEntitiesChanged([])`. Это только поднимает `reloadSeq`, а тот перечитывает дерево (`loadTree`). Кэш `statsCache` сбрасывается лишь в `refreshTree()` или по непустому списку `placeIds`. Потерянное при `Lagged`/обрыве событие `entities_changed` с `placeIds` приводит к тому, что счётчики содержимого мест остаются устаревшими до ручного «Обновить». Требование D-08 «перечитать всё» не выполняется.
**Fix:** Ввести в сторе отдельный `resetSeq`, на который `PlaceTree` вызывает `refreshTree()`. Либо дать `notifyEntitiesChanged` флаг `{ all: true }`, который сбрасывает `statsCache`.

### WR-06: `im_send`: повтор после исключения отвечает 403 вместо replay, и клиент не может подтвердить доставку

**File:** `crates/trackly-app/src/services/im_service.rs:136-143`
**Issue:** Проверка `can_post` стоит ДО поиска по `client_msg_id`. Если сообщение было записано, ответ потерян, а автора затем исключили, повтор с тем же id получает 403. Клиент (по чек-листу Link «повторять с тем же id») пометит доставленное сообщение как неотправленное или удалит его. В `chat_send` для аналогичного случая (закрытая заявка) replay сознательно решается раньше проверки статуса (`chat_service.rs:508-509`), то есть два контракта расходятся.
**Fix:** Искать replay сразу после проверки, что строка участника существует (это всё ещё «после проверки членства»), а `can_post` проверять только для новой вставки. Обновить ADR, раздел 6.

### WR-07: `isNetworkFailure` считает сетевым сбоем любой ответ без формы AppError

**File:** `packages/transport/src/sendWithRetry.ts:13-15`; `packages/transport/src/client.ts:62-64`
**Issue:** На HTTP-пути `apiCall` при `!res.ok` и не-JSON теле получает `{}`, и `parseAppError` возвращает `UnknownError`. Так выглядят: axum `JsonRejection` (422/400, text/plain), 413 от лимита тела, 502/504 прокси, а также любой программный `TypeError` в цепочке. Всё это повторяется трижды, а затем показывается «Не отправлено» с «Повторить», который никогда не пройдёт.
**Fix:** Различать реальный сетевой сбой (исключение `fetch`, `TypeError: Failed to fetch`) и HTTP-ответ с кодом. Для HTTP 4xx без JSON бросать AppError с синтетическим кодом (например `HTTP_4xx`) и не повторять. Повторять только отказ `fetch` и 502/503/504.

## Info

### IN-01: `is_unique_conflict` ловит любой `Conflict`, а не только UNIQUE

**File:** `crates/trackly-infra/src/repos/im_sqlite.rs:115-117`
**Issue:** `map_rusqlite` отображает в `Conflict` и CHECK/FK-нарушения. Имя функции вводит в заблуждение. Поведение безопасно, потому что поиск вернёт `None` и исходная ошибка пробросится.
**Fix:** Переименовать в `is_constraint_conflict` или различать код ограничения.

### IN-02: Ветки «страховки UNIQUE» недостижимы при единственном писателе

**File:** `crates/trackly-app/src/services/im_service.rs:159-168`, `chat_service.rs:546-563`, `request_service.rs:492-511`
**Issue:** Поиск по клиентскому id выполняется в той же транзакции писателя прямо перед вставкой, так что конфликт по ключу идемпотентности невозможен. Ветки не покрыты тестами и маскируют возможные будущие ошибки.
**Fix:** Оставить с комментарием «defence-in-depth, недостижимо при single-writer» или заменить на `debug_assert!`/`Internal`.

### IN-03: `replay_of` принимает неиспользуемый `repo`

**File:** `crates/trackly-app/src/services/chat_service.rs:103-116`
**Issue:** `let _ = repo;` подавляет предупреждение о мёртвом параметре.
**Fix:** Удалить параметр.

### IN-04: Дефолт `kind` в схеме (`'user'`) расходится с кодом (`'text'`)

**File:** `migrations/V058__im_core.sql:45`; `crates/trackly-app/src/services/im_service.rs:151`
**Issue:** Схема по умолчанию ставит `'user'`, сервис пишет `'text'`, CHECK нет. Фаза 47 (системные сообщения) получит два разных значения для одного вида.
**Fix:** Зафиксировать словарь `kind` в CHECK следующей миграцией, оставив V058 неизменной.

### IN-05: `messages_window_on` используется только в тестах

**File:** `crates/trackly-infra/src/repos/im_sqlite.rs:357-378`
**Issue:** Публичный метод без вызовов в продакшен-коде.
**Fix:** Пометить как точку расширения Фазы 47 или перенести под `#[cfg(test)]`.

### IN-06: Браузерный клиент не проверяет `hello.protocol`

**File:** `packages/transport/src/ws.ts:231-233`
**Issue:** `hello` молча отбрасывается. При смене версии протокола старая вкладка не узнает о несовместимости.
**Fix:** При `protocol !== 1` логировать или предлагать перезагрузку.

### IN-07: Двойной `refreshUnread` на ресинке у сотрудника

**File:** `ui/src/features/layout/EmployeeLayout.svelte:95-98`; `ui/src/features/requests/chat/unreadStore.svelte.ts:148-151`
**Issue:** `startChatRealtime()` через `startUnreadRealtime()` уже подписывает `refreshUnread` на ресинк, а `EmployeeLayout` добавляет второй такой же обработчик. В итоге два одинаковых запроса.
**Fix:** Убрать `handleResync` из `EmployeeLayout`.

### IN-08: Подписка ImHub создаётся до upgrade, и при сорванном рукопожатии канал не чистится

**File:** `crates/trackly-app/src/http/ws.rs:166-174`
**Issue:** `ctx.im_hub.subscribe(uid)` вызывается в `ws_handler` до `on_upgrade`. Если upgrade не завершился, `handle_ws_socket` не запускается и `prune()` не вызывается. Пустой канал остаётся в карте до следующего `prune` от другого соединения. Утечка ограничена числом пользователей.
**Fix:** Подписываться внутри замыкания `on_upgrade` или вызывать `prune` в `Drop`-гарде.

---

_Reviewed: 2026-11-27T01:38:51Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
