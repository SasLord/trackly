---
phase: 43-reliable-event-delivery
reviewed: 2026-11-27T12:00:00Z
depth: standard
review_kind: re-review (gap closure 43-13..43-21)
diff_base: d8a749f4
files_reviewed: 32
files_reviewed_list:
  - crates/trackly-app/src/dto/chat.rs
  - crates/trackly-app/src/services/chat_service.rs
  - crates/trackly-app/src/services/im_service.rs
  - crates/trackly-app/tests/chat_feed.rs
  - crates/trackly-app/tests/chat_idempotent_send.rs
  - crates/trackly-app/tests/im_idempotent_send.rs
  - crates/trackly-app/tests/im_membership.rs
  - crates/trackly-app/tests/im_sync.rs
  - crates/trackly-infra/src/repos/chat_sqlite.rs
  - crates/trackly-infra/src/repos/im_sqlite.rs
  - crates/trackly-infra/tests/im_repo.rs
  - packages/transport/src/client.ts
  - packages/transport/src/sendWithRetry.ts
  - packages/transport/src/ws.ts
  - ui/scripts/_rune-harness.mjs
  - ui/scripts/check-chat-pending-behavior.mjs
  - ui/scripts/check-chat-ui.mjs
  - ui/scripts/check-place-tree-invalidation.mjs
  - ui/scripts/check-request-create-idempotency.mjs
  - ui/scripts/check-send-retry.mjs
  - ui/scripts/check-ws-resync.mjs
  - ui/src/features/layout/Layout.svelte
  - ui/src/features/places/PlaceTree.svelte
  - ui/src/features/requests/RequestFormModal.svelte
  - ui/src/features/requests/chat/ChatComposer.svelte
  - ui/src/features/requests/chat/ChatPendingMessage.svelte
  - ui/src/features/requests/chat/RequestChat.svelte
  - ui/src/features/requests/chat/draftPolicy.ts
  - ui/src/features/requests/chat/pendingStore.svelte.ts
  - ui/src/features/requests/requestIdPolicy.ts
  - ui/src/lib/stores/placeContentEvents.svelte.ts
  - docs/adr/0003-chat-model-fts-delivery.md
findings:
  critical: 0
  warning: 3
  info: 8
  total: 11
previous_findings:
  closed: [CR-01, CR-02, CR-03, WR-01, WR-02, WR-05, WR-06, WR-07]
  partial: [WR-03, WR-04]
  open: []
status: issues_found
---

# Phase 43: Code Review Report (повторное ревью после закрытия пробелов)

**Reviewed:** 2026-11-27T12:00:00Z
**Depth:** standard
**Files Reviewed:** 32 (диапазон `d8a749f4..HEAD`)
**Status:** issues_found

## Narrative Findings (AI reviewer)

## Summary

Проверены исправления планов 43-13..43-20 по всем файлам диапазона, а также поведенческие гейты, которыми они закреплены. Прежнее ревью сохранено в git, коммит `cb93d2d5`. Шесть гейтов области (`check-chat-pending-behavior`, `check-chat-ui`, `check-place-tree-invalidation`, `check-request-create-idempotency`, `check-send-retry`, `check-ws-resync`) я запустил локально, все зелёные. `cargo test` не запускался: серверная часть проверена чтением кода и тестов.

Все три Critical прежнего ревью закрыты по существу:
- `im_sync` отдаёт исключённому сводку `read_only`.
- Композер больше не стирает черновики, которые удерживает очередь.
- Сокеты в `ws.ts` привязаны к своему экземпляру, а проба и таймер переподключения гасятся в `handleLost`. Гейт `check-ws-resync` держит это мутантами по каждому слою защиты.

Из семи Warning пять закрыты, два закрыты частично (WR-03, WR-04).

Новых Critical нет. Найдены три новых Warning, все в клиентской очереди чата:
1. Кнопка «В поле ввода» на закрытой заявке уничтожает текст сообщения.
2. Файлы, приложенные во время загрузки вложений, стираются, хотя так и не были отправлены.
3. Сверка очереди с лентой работает только в момент загрузки ленты. Поэтому «двойник» из WR-04 остаётся достижимым.

## Вердикты по находкам прежнего ревью

| ID | Вердикт | Обоснование |
|----|---------|-------------|
| CR-01 | **закрыт** | `im_service.rs:367`: при `cursor >= left_seq` беседа возвращается сводкой `(messages: [], reload: false, last_seq = left_seq, read_only: true)`, а не пропускается. Тест `caught_up_then_removed_gets_read_only_summary` (`im_sync.rs`) покрывает сценарий «дочитал, затем исключён», в том числе отсутствие утечки новых сообщений после выхода. ADR, раздел 8(в), и чек-лист Link (раздел 11) обновлены. |
| CR-02 | **закрыт** | `pendingStore.svelte.ts:104` `discardUnheldDrafts` не зовёт серверную очистку, пока хоть одна запись очереди по заявке держит черновики. `dispatch(..., keepOnDomainFailure = true)` на повторе не снимает запись при доменном отказе, так что текст не теряется. Живая проверка A/B в 43-21 пройдена. Остаточный эффект (осиротевшие черновики) описан в IN-03. |
| CR-03 | **закрыт** | `ws.ts`: в `connectBrowser` стоит `if (ws !== null) return`. Все обработчики проверяют `ws !== sock`. `handleLost` снимает пробу и висящий `reconnectTimer`. Таймер пробы сверяет `probed`. Кейсы R12/R12b/R13 с мутантами MA..MD и задокументированными эквивалентами одиночных снятий. |
| WR-01 | **закрыт** | Для исключённого с отстающим курсором выдаётся дельта `rev > cursor` внутри окна (`hi_seq`/`max_rev = left_seq`), `reload: true` только без курсора или выше порога. Это подтверждают тесты `removed_member_lagging_cursor_gets_windowed_delta` и `removed_member_over_threshold_reloads_within_window`. |
| WR-02 | **закрыт** | `requestIdPolicy.ts`: id выпускается заново, когда меняется подпись тела (тип, принтер или категория в зависимости от типа, `description.trim()`). Без правок повтор идёт с тем же id. Гейт `check-request-create-idempotency` исполняет политику. |
| WR-03 | **частично** | Текст очищается сразу и восстанавливается через `restoreText` при отказе, поэтому набранное за время автоповторов не стирается. Но снимок чипов `sentChipIds` берётся ПОСЛЕ цикла загрузки, и файлы, приложенные во время загрузки, удаляются неотправленными. См. новый WR-02. |
| WR-04 | **частично** | `ChatMessageDto.client_msg_id` отдаётся только автору (`chat_service.rs`, тест `feed_exposes_client_msg_id_to_author_only`). `load()` снимает записи `failed` по `clientMsgId`. Но сверка выполняется только в `load()`. Запись, перешедшая в `failed` ПОСЛЕ последней загрузки ленты, остаётся «Не отправлено» рядом с доставленным сообщением. См. новый WR-03. |
| WR-05 | **закрыт** | `notifyEntitiesChanged([], { all: true })` поднимает `resetSeq`. `PlaceTree` в том же эффекте сбрасывает `statsCache` и поколение `statsGen`. Самоподписки нет: `statsCache` в синхронной части эффекта только пишется. Живая проверка F пройдена. |
| WR-06 | **закрыт** | `im_service.rs:140`: replay ищется после проверки строки участника (404 для чужого), но до `can_post`. Новая отправка исключённого по-прежнему даёт 403. Тесты `replay_after_author_removed_still_returns_original` и обновлённый `removed_member_cannot_send`. ADR, раздел 6, обновлён. |
| WR-07 | **закрыт** | `client.ts`: отказ `fetch` превращается в `NETWORK_ERROR`, HTTP-ответ без формы AppError в `HTTP_<status>`. `isNetworkFailure` повторяет только `NETWORK_ERROR`, `HTTP_502/503/504` и `TypeError` отказа fetch. Остаток описан в IN-04. |

Info прежнего ревью: IN-01 (переименование в `is_constraint_conflict`), IN-02 (комментарии defence-in-depth), IN-03 (удалён параметр `repo`) и IN-06 (проверка `hello.protocol`) закрыты. IN-04 (`kind DEFAULT 'user'` в V058), IN-05 (`messages_window_on` вызывается только из тестов), IN-07 (двойной `refreshUnread` в `EmployeeLayout.svelte:97,124`) и IN-08 (`im_hub.subscribe` до `on_upgrade`, `http/ws.rs:167`) остаются открытыми. Они вне диапазона исправлений и сведены в IN-08 ниже.

## Warnings

### WR-01: «В поле ввода» на закрытой заявке уничтожает текст неотправленного сообщения

**File:** `ui/src/features/requests/chat/RequestChat.svelte:321-333`; `ui/src/features/requests/chat/ChatComposer.svelte:55-59`; `ui/src/features/requests/chat/ChatPendingMessage.svelte:28-33`
**Issue:** После 43-16 запись, которой повтор вернул `rejected: closed`, остаётся в очереди со статусом `failed` (`keepOnDomainFailure`), и у пузыря видна кнопка «В поле ввода». `onToDraft` сначала снимает запись (`returnToDraft` удаляет её из очереди), затем пишет текст в `draft.text`. На закрытой заявке композер смонтирован в read-only виде, а его эффект `if (!canPost && (draft.text !== '' || …)) clearDraft()` срабатывает на изменение `draft.text` и сразу стирает текст. В итоге пузырь пропадает, поле скрыто, текст уничтожен без следа. До клика текст хотя бы был виден в пузыре и его можно было скопировать.

Второй путь потери текста в `onToDraft`: запись снимается ДО проверки `draft.requestId !== requestId`. Если черновик привязан к другой заявке, текст возвращается из стора и выбрасывается.

С этим связано наблюдение 1 из 43-21: причина отказа повтора уходит в `draft.errors` скрытого композера, а «Повторить» на закрытой заявке бессмысленна навсегда.
**Fix:** Не предлагать «В поле ввода» и «Повторить», когда писать нельзя, и снимать запись только после успешной вставки текста:
```ts
function onToDraft(clientMsgId: string) {
  if (!canPost || draft.requestId !== requestId) return; // запись остаётся в очереди
  const item = pendingFor(requestId).find((p) => p.clientMsgId === clientMsgId);
  const text = returnToDraft(clientMsgId);
  if (text === null) return;
  draft.text = restoreText(text, draft.text);
  ...
}
```
В `ChatPendingMessage` передавать `canPost` и на закрытой заявке показывать вместо кнопок «Заявка закрыта — сообщение не отправлено» (текст остаётся в пузыре для копирования). Добавить кейс в `check-chat-pending-behavior`: «повтор → closed → В поле ввода» не должен терять текст.

### WR-02: Файлы, приложенные во время загрузки вложений, снимаются из композера неотправленными (остаток WR-03)

**File:** `ui/src/features/requests/chat/ChatComposer.svelte:154, 174, 181, 210`
**Issue:** Цикл загрузки идёт по снимку `[...draft.chips]` (строка 154), но `sentChipIds` вычисляется уже ПОСЛЕ цикла (строка 174) из текущего `draft.chips`. `sending` блокирует только кнопку. Вставка (`handlePaste`), выбор файлов и перетаскивание во время загрузки работают (`addFiles` проверяет лишь `canPost`). Если пользователь приложил файл, пока грузились предыдущие, этот чип:
- не загружен и в `draftIds` не попал, то есть с сообщением не уходит;
- входит в `sentChipIds` и при исходах `sent` и `failed` удаляется `removeChip`.

Пользователь видел приложенный файл, а в отправленном сообщении его нет, и из поля он исчез. Исправление WR-03 обещает «набранное за время отправки не стирается», но для вложений это не выполнено. Гейт `check-chat-pending-behavior` этот порядок не проверяет.
**Fix:** Снимок брать один раз до цикла и использовать его и для загрузки, и для очистки:
```ts
const sentChips = [...draft.chips];
const sentChipIds = new Set(sentChips.map((c) => c.id));
for (const chip of sentChips) { ... }
```
И добавить кейс «чип добавлен во время uploadDraft остаётся в композере».

### WR-03: Сверка очереди с лентой только в `load()`, поэтому «двойник» WR-04 остаётся достижимым

**File:** `ui/src/features/requests/chat/pendingStore.svelte.ts:66, 123-130`; `ui/src/features/requests/chat/RequestChat.svelte:186-192`
**Issue:** `reconcileWithFeed` вызывается только из `load()` и снимает лишь записи `failed`. Записи `sending` пропускаются намеренно. Типичная последовательность при потере ответа:
1. Сервер сохранил сообщение, ответ потерян.
2. Событие `chat_message` → `load()`: сообщение в ленте, запись ещё `sending`, сверка её не трогает.
3. Автоповторы исчерпаны (HTTP-путь недоступен, например прокси отвечает 502, а WS жив): `setStatus(..., 'failed')`.
4. Новых событий и ресинка нет, `load()` не вызывается.

В итоге доставленное сообщение стоит в ленте, а рядом пузырь «Не отправлено» с тем же текстом. Это ровно сценарий WR-04. Если теперь нажать «В поле ввода» (новая кнопка из 43-16) и отправить, сообщение уйдёт с НОВЫМ `clientMsgId`, и получится реальный дубль. «Повторить» в этом случае безопасна (replay), а «В поле ввода» нет.
**Fix:** Помнить последний набор `clientMsgId` ленты по заявке в сторе и сверять запись в момент перехода в `failed`:
```ts
const seenByRequest = new Map<number, Set<string>>();
export function reconcileWithFeed(requestId: number, ids: Iterable<string>) {
  const seen = new Set(ids);
  seenByRequest.set(requestId, seen);
  ...
}
// в dispatch, ветка сетевого сбоя:
if (seenByRequest.get(requestId)?.has(clientMsgId)) { removeItem(clientMsgId); return { kind: 'sent-unknown' } }
```
Либо после перехода в `failed` инициировать `load()` ленты. Добавить кейс гейта «лента загружена, пока запись sending; затем failed → пузыря нет».

## Info

### IN-01: Сводка `read_only` по каждой когда-либо покинутой беседе на каждый `im_sync`

**File:** `crates/trackly-app/src/services/im_service.rs:367`
**Issue:** После CR-01 исключённый получает сводку по беседе бессрочно, и ответ `im_sync` растёт вместе с числом покинутых бесед. Признак «знаю о выходе» сознательно не введён (ADR, раздел 8в). Для корректности это не дефект, но для Link (Фаза 44) это постоянный шум.
**Fix:** Зафиксировать в ADR как точку расширения (например, отдельный флаг `includeLeft`) или отдавать сводку только при `cursor < left_seq + 1` с курсором, который клиент ставит в `left_seq + 1` после приёма сводки.

### IN-02: Replay для исключённого автора отдаёт текущую ревизию строки

**File:** `crates/trackly-app/src/services/im_service.rs:138-142`
**Issue:** После WR-06 replay идёт до `can_post` и возвращает `existing` целиком. Окно видимости `rev <= left_seq` здесь не применяется. В Фазе 43 путей правки нет, так что утечки сейчас нет. Но когда Фаза 47 добавит правку или надгробие, повтор исключённого автора вернёт ревизию, созданную после выхода, а это противоречит D-05.
**Fix:** Внести в список точек расширения Фазы 47 (ADR, раздел 11): replay для `left_at_utc IS NOT NULL` обязан учитывать окно.

### IN-03: `discardUnheldDrafts` работает по принципу «всё или ничего» и оставляет сирот до перезапуска

**File:** `ui/src/features/requests/chat/pendingStore.svelte.ts:104-108`; `ui/src/features/requests/chat/ChatComposer.svelte:161, 189, 201`
**Issue:** Если хоть одна запись очереди держит черновики, очистка пропускается целиком. Черновики текущей неудачной отправки (частично загруженные при отказе файла, черновики отказанного сообщения, черновики записи, снятой через «В поле ввода») остаются на сервере до запуска `purge_stale_drafts(24 ч)` (`context.rs:351`, только при старте). Обратный случай: запись `failed`, пережившая перезапуск сервера больше чем через сутки, получает на «Повторить» 403 `Forbidden` (черновик стёрт), и пользователь видит неверное «Нет доступа к переписке».
**Fix:** Точечное удаление по id (`attachments.deleteDrafts(own)`), как предлагалось в прежнем CR-02. Отказ `promote_drafts_on` различать отдельным кодом, а не `Forbidden`.

### IN-04: `FETCH_FAILURE_RE` ловит программные `TypeError` со словом «fetch»

**File:** `packages/transport/src/sendWithRetry.ts:21, 28`
**Issue:** `/fetch|network|load failed/i` совпадает, например, с `TypeError: x.fetchItems is not a function`. Такая программная ошибка будет повторена трижды и закончится «Не отправлено». В пределах `apiCall` отказ fetch уже превращён в `NETWORK_ERROR`, так что ветка нужна только обходящим `apiCall`.
**Fix:** Сузить до известных формулировок (`/^(Failed to fetch|NetworkError when attempting to fetch resource\.|Load failed|network error)$/i`) или убрать ветку, если обходящих вызовов нет.

### IN-05: Нет таймаута у `fetch`, поэтому зависший запрос держит композер

**File:** `packages/transport/src/client.ts:59-69`; `ui/src/features/requests/chat/ChatComposer.svelte:176`
**Issue:** На полуоткрытом TCP-соединении `fetch` может висеть минутами. `sendWithRetry` не видит отказа, запись остаётся «Отправляется…», а `sending` блокирует кнопку отправки во всём чате. Механизм D-12 (1/2/4 с) в этом случае не срабатывает.
**Fix:** `AbortController` с таймаутом (например, 15 с) для мутирующих вызовов. Отмену превращать в `NETWORK_ERROR`: повтор безопасен, потому что идемпотентен по `clientMsgId`.

### IN-06: Повтор с откатом правок формы заявки выпускает новый id

**File:** `ui/src/features/requests/requestIdPolicy.ts:41-53`
**Issue:** Выдатчик помнит только последнюю подпись. Возможна последовательность: тело A, ответ потерян после коммита; правка на B, сбой; возврат к A. Тогда A уходит с новым id и создаёт дубль первой заявки. Это редкий путь и осознанный компромисс WR-02.
**Fix:** Хранить `Map<signature, id>` в пределах открытия формы.

### IN-07: Дашборд и виджеты-счётчики не подписаны ни на события, ни на ресинк

**File:** `ui/src/features/dashboard/DashboardPage.svelte` (подписок `onWsEvent`/`onWsResync` нет)
**Issue:** Это наблюдение 2 из 43-21: счётчики не обновляются ни по событию, ни после ресинка. Дефект старше фазы, но реестр ресинка (`check-ws-resync-registry`) его не видит, потому что считает только уже подписанные экраны.
**Fix:** Отдельная задача: подписать дашборд и «Мои заявки» на `entities_changed`/`onWsResync` и внести их в реестр.

### IN-08: Открытые Info прежнего ревью

**File:** `migrations/V058__im_core.sql:45`; `crates/trackly-infra/src/repos/im_sqlite.rs:359`; `ui/src/features/layout/EmployeeLayout.svelte:97,124`; `crates/trackly-app/src/http/ws.rs:167`
**Issue:** Остаются открытыми:
- IN-04: `kind DEFAULT 'user'` против `'text'` в коде;
- IN-05: `messages_window_on` вызывается только из тестов;
- IN-07: двойной `refreshUnread` на ресинке у сотрудника;
- IN-08: подписка `ImHub` до `on_upgrade`.
**Fix:** См. прежнее ревью (коммит `cb93d2d5`). Ни одна из этих находок не блокирует фазу.

---

_Reviewed: 2026-11-27T12:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
