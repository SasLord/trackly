# Phase 11: Заявки/employee UX gap-closure — Research

**Researched:** 2026-06-21
**Domain:** Trackly backend (Rust/axum/rusqlite) + Svelte 5 frontend — dual-transport DTO plumbing, WebSocket fan-out filtering, browser Notification/Page-Visibility APIs, custom grouped dropdown.
**Confidence:** HIGH (all three findings grounded in read of the actual code; no external dependency research needed — no new crates/npm packages.)

<user_constraints>
## User Constraints (from 11-CONTEXT.md)

### Locked Decisions

**D-CAT-01 — категория заявки текстом, не числом**
- Категория должна отображаться текстовым названием из `request_categories.name`.
- Список категорий должен возвращать `{ id, name }` (оба транспорта), чтобы форма слала корректный `category_id`.
- Read-DTO заявки (`RequestDto`) должен прокидывать имя категории: добавить `category_name: Option<String>` через LEFT JOIN `request_categories` в `requests_sqlite.rs` (по образцу `requester_name`).
- `RequestDetail.svelte` (и любой список заявок) рендерит `category_name`, а не `category_id`.

**D-WS-01 — ответ администратора сотруднику: WebSocket + тост + системная нотификация**
- Бэкенд УЖЕ шлёт `WsEvent::RequestStatusChanged` при `RequestService::transition`. Не хватает доставки сотруднику в браузере.
- Сотрудник в браузере подписывается на WS (как админ/десктоп) и при `RequestStatusChanged` по СВОЕЙ заявке показывает тост.
- Если вкладка свёрнута/неактивна (`document.hidden` / Page Visibility API) — показать системную нотификацию (Web Notification API); запросить разрешение деликатно.
- Поправить `WsEvent::is_visible_to(&identity)` так, чтобы автор заявки (employee) получал событие об изменении статуса СВОЕЙ заявки. Не раскрывать сотруднику чужие события.

**D-PRN-01 — дропдаун принтеров для заявки сотрудника**
- Отдельный эндпоинт списка принтеров для формы заявки, доступный сотруднику (гейт по `Action::CreateRequest`, НЕ `ReadData`/`ReadPrinters`). Возвращает минимум: id принтера, наименование, Расположение (location).
- На первое время — отдавать ВЕСЬ список принтеров (без фильтра по доступу), отсортированный по Расположению. Оба транспорта, бинды регенерировать.
- UI: кастомный дропдаун, сгруппированный по Расположению; заголовок группы = небольшая полоска с сереньким фоном и текстом Расположения. Сортировка групп по Расположению.
- `RequestFormModal.svelte` переключается с `devices.list` на новый эндпоинт.

### Claude's Discretion
- Точные имена полей/DTO; для free_form-заявок без категории показывать прочерк/пусто.
- Точные тексты тостов/нотификаций (RU); момент запроса разрешения на нотификации; формат payload `RequestStatusChanged` (добавить `requested_by_user_id` при необходимости).
- Имя эндпоинта/DTO принтер-пикера; вёрстка кастомного дропдауна; поведение при пустом списке; переиспользование для admin/manager (опционально).
- Где разместить логику подписки на WS для сотрудника (employee shell vs общий ws-клиент с ролевой фильтрацией).
- Нужны ли миграции БД — подтвердить (вероятно нет).

### Deferred Ideas (OUT OF SCOPE)
- Фильтрация принтеров по доступу/локации пользователя — позже («на первое время весь список»).
- Email/SMTP-уведомления — v2.
- Раннер фронтенд-тестов для WS/нотификаций/дропдауна — вне scope (проверка ручная).
- Изменения ролевой модели/гейтинга сверх необходимого.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| D-CAT-01 | Категория заявки — текстом, не числом | `request_categories` (V024) сидируется 4 RU-именами; `category_id` подтверждён как FK. Образец LEFT JOIN — `requester_name`/`printer_name` в `SELECT_REQUESTS` (requests_sqlite.rs). Список категорий уже хардкожен на UI в `RequestFormModal` — серверный список нужен только если планировщик решит убрать хардкод. |
| D-WS-01 | Ответ админа доходит до сотрудника: WS-тост + системная нотификация при скрытой вкладке | WS-инфраструктура существует (broadcast → `is_visible_to` фильтр → browser WS / Tauri bridge). Нужны 3 точечные правки: (1) `is_visible_to` пропускает `RequestStatusChanged` сотруднику; (2) payload расширяется `requested_by_user_id` для клиентской фильтрации; (3) employee-подписка + Notification/Visibility логика на фронте. **Secure-context ограничение Notification API — см. Pitfall 3.** |
| D-PRN-01 | Сотрудник снова может выбрать принтер в форме заявки | Phase 10 закрыл `devices_list` (`ReadData`) от employee → 403 → пустой список. Решение: новый узкий эндпоинт под `Action::CreateRequest`. Источник данных — `devices` type_id=2 + LEFT JOIN `locations` (паттерн в devices_sqlite.rs, индекс 15 = `l.name`). |
</phase_requirements>

## Summary

Все три находки — точечные доработки поверх уже существующей, проверенной инфраструктуры. Новых крейтов/npm-пакетов НЕ требуется. Миграции БД НЕ требуются (подтверждено: `request_categories` создана и засидена в V024; `devices`/`locations` существуют; всё остальное — DTO/JOIN/эндпоинты/UI).

- **D-CAT-01** — однострочная по сути правка: добавить `LEFT JOIN request_categories rc ON rc.id = r.category_id`, прокинуть `rc.name AS category_name` в `RequestRow`/`RequestDto`, отрендерить в `RequestDetail.svelte` (заменить `{request.categoryId}` на `{request.categoryName ?? '—'}`). Список категорий `{id,name}` — опционально (форма сейчас хардкодит 4 категории; серверный список устранит дублирование, но не блокирует находку).
- **D-WS-01** — три точечные правки: (1) `WsEvent::is_visible_to` для `RequestStatusChanged` должен пропускать события сотруднику-автору, что требует (2) добавить `requested_by_user_id` в payload `RequestStatusChanged` (иначе сервер не знает, кому событие принадлежит, и клиент не может отфильтровать своё). (3) Сотрудник должен подписаться на WS — сейчас `connectWs()/onWsEvent` живёт только в `RequestsPage` (которую employee видит). Логика тост-vs-нотификация по `document.hidden`.
- **D-PRN-01** — новый эндпоинт `request_printer_options` (или аналог), гейт `Action::CreateRequest`, минимальный DTO `{ id, name, location }`, source = `devices` type_id=2 + LEFT JOIN `locations`, сортировка по location. UI — кастомный сгруппированный дропдаун (новый компонент). Переключить `RequestFormModal.loadPrinters`.

**Primary recommendation:** Реализовать ровно по букве CONTEXT. Самая тонкая точка — D-WS-01: расширение payload `RequestStatusChanged` + изменение `is_visible_to` нужно делать вместе (одно без другого либо ничего не доставляет, либо доставляет сотруднику ВСЕ события). Notification API требует secure-context — это работает на штатном HTTPS-доступе (`0.0.0.0:8443` self-signed), но НЕ на HTTP-fallback первого запуска; нужно graceful-degrade на тост.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Имя категории в read-DTO | API / Backend (rusqlite JOIN) | — | Read-DTO собирается в `requests_sqlite.rs`; имя — производное от FK, считается на сервере (как `requester_name`). |
| Список категорий `{id,name}` | API / Backend (service+оба транспорта) | Browser/Client | «Один DTO, два транспорта». Форма потребляет. |
| Фильтрация видимости WS-событий по роли/автору | API / Backend (`is_visible_to`) | — | Источник истины безопасности — сервер; нельзя полагаться на клиентскую фильтрацию (D-RBAC-03). |
| Подписка на WS + тост/нотификация | Browser/Client (Svelte) + Tauri webview | — | UX-слой; работает в обоих webview через существующий dual-transport ws.ts. |
| Системная нотификация (Notification API) | Browser/Client | — | Браузерный стандарт; в Tauri webview работает (WebView2/WKWebView), но secure-context-зависим. |
| Список принтеров для формы заявки | API / Backend (новый CreateRequest-эндпоинт) | — | Гейт безопасности — сервер; employee не имеет ReadData, поэтому новый узкий гейт. |
| Сгруппированный дропдаун | Browser/Client (Svelte компонент) | — | Чистый UI; группировка по location, переданному с сервера. |

## Standard Stack

**Новых зависимостей нет.** Используется существующий стек (см. CLAUDE.md). Браузерные API ниже — веб-стандарты, не пакеты:

| API / тех | Где | Назначение | Подтверждение |
|-----------|-----|-----------|---------------|
| Web Notification API (`Notification`, `Notification.requestPermission`) | Frontend | Системная нотификация при скрытой вкладке | [CITED: MDN Notifications API] — secure-context only (Pitfall 3) |
| Page Visibility API (`document.hidden`, `visibilitychange`) | Frontend | Решить тост vs нотификация | [CITED: MDN Page Visibility API] |
| `WebSocket` / Tauri `listen('trackly-event')` | Frontend (`ui/src/lib/api/ws.ts`) | Доставка событий, уже реализовано dual-transport | [VERIFIED: код ws.ts прочитан] |
| `tokio::sync::broadcast` + axum WS | Backend (`http/ws.rs`) | Серверный fan-out, уже реализовано | [VERIFIED: код ws.rs прочитан] |
| `rusqlite` LEFT JOIN | Backend (`requests_sqlite.rs`, `devices_sqlite.rs`) | Имя категории / список принтеров | [VERIFIED: код прочитан] |
| `tauri_specta` / `specta::Type` | bindings | Регенерация `ui/src/bindings.ts` при новых командах/DTO | [VERIFIED: specta_export.rs прочитан] |

## Package Legitimacy Audit

**Не применимо к этой фазе.** Внешние пакеты не устанавливаются — все изменения на существующем стеке (rusqlite/axum/Svelte) и браузерных веб-стандартах. Новых строк в `Cargo.toml`/`package.json` не предвидится.

## Architecture Patterns

### System Architecture Diagram — D-WS-01 поток события сотруднику

```
[Admin/Manager жмёт "Принять/Отклонить/Выполнить" в RequestDetail.svelte]
        │  requests.transition(...)   (Tauri invoke ИЛИ POST /api/v1/requests_transition)
        ▼
[RequestService::transition]  ── authorize(TransitionRequests) ─→ writer tx (status update + audit)
        │
        │  ctx.ws_tx.send(WsEvent::RequestStatusChanged{ request_id, new_status, requested_by_user_id* })
        ▼
[tokio::sync::broadcast]  ─────────────────────────┬───────────────────────────────┐
        │ (browser path)                            │ (desktop path, main.rs bridge)  │
        ▼                                           ▼                                 │
[http/ws.rs handle_ws_socket]                [app.emit("trackly-event", evt)]        │
   per-client: evt.is_visible_to(&identity)?  (forwards same WsEvent to webview)     │
        │  ── ДА ──→ socket.send(JSON)                                               │
        ▼                                                                            ▼
[ui/src/lib/api/ws.ts] dispatch(event) ─→ onWsEvent handlers ──────────────────────┘
        ▼
[Employee handler]  event.type==='request_status_changed' && event.requestedByUserId === myUserId
        ├── document.hidden ?  ── ДА ──→ Notification (secure-context only; иначе fallback toast)
        └────────────────────  НЕТ ──→ pushToast('success'/'info', 'Ваша заявка ...')
```

`*` `requested_by_user_id` — новое поле, добавляется в payload (см. D-WS-01 ниже). Без него `is_visible_to` не сможет авторизовать «своё событие», и клиент не сможет отфильтровать.

### Pattern 1: Read-DTO производное поле через LEFT JOIN (D-CAT-01)

**What:** Имя из связанной таблицы прокидывается как `Option<String>` в read-DTO; пишущий путь не трогается.
**When to use:** D-CAT-01 — `category_name`.
**Example (точная точка вставки — `crates/trackly-infra/src/repos/requests_sqlite.rs`, const `SELECT_REQUESTS`):**

```rust
// Текущий SELECT_REQUESTS уже джойнит users и devices:
//   LEFT JOIN users u ON u.id = r.requested_by_user_id      -> u.full_name AS requester_name
//   LEFT JOIN devices d ON d.id = r.printer_device_id        -> d.name AS printer_name
// ДОБАВИТЬ:
//   rc.name AS category_name           (в список столбцов, ПЕРЕД r.created_at_utc — сдвиг индексов!)
//   LEFT JOIN request_categories rc ON rc.id = r.category_id
// + map_row_request: добавить row.get(N) для category_name (ВСЕ последующие индексы +1)
// + RequestRow (trackly-core/domain/requests.rs): pub category_name: Option<String>
// + RequestDto (dto/request.rs): pub category_name: Option<String> + строка в From<RequestRow>
```
**Source:** [VERIFIED: requests_sqlite.rs SELECT_REQUESTS + map_row_request прочитаны].
**Критично:** добавление столбца в середину SELECT сдвигает все индексы `row.get(n)` в `map_row_request`. Безопаснее добавить `category_name` ПОСЛЕДНИМ столбцом SELECT и индексом, чтобы не трогать существующие позиции. То же касается `fetch_in_tx`/`get`/`list`, которые все используют общий `SELECT_REQUESTS` + `map_row_request` — одна правка покрывает все пути чтения.

### Pattern 2: WS-payload расширение + ролевая видимость (D-WS-01)

**What:** Сервер обязан знать, кому принадлежит событие, чтобы и (а) пропустить его автору, и (б) НЕ раскрыть чужим сотрудникам.
**Example (`crates/trackly-app/src/dto/printer.rs`):**

```rust
// В enum WsEvent::RequestStatusChanged ДОБАВИТЬ поле:
RequestStatusChanged {
    #[specta(type = i32)]
    request_id: i64,
    new_status: String,
    #[specta(type = i32)]
    requested_by_user_id: i64,   // НОВОЕ — нужно is_visible_to + клиентской фильтрации
},

// is_visible_to: текущая реализация отдаёт RequestStatusChanged ТОЛЬКО Admin|Manager.
// Расширить: автор-сотрудник видит СВОЁ событие.
WsEvent::RequestStatusChanged { requested_by_user_id, .. } => {
    matches!(identity.role, Role::Admin | Role::Manager)
        || identity.user_id == Some(*requested_by_user_id)
}
// NewRequest оставить Admin|Manager (сотруднику чужие новые заявки не нужны).
```
**Source:** [VERIFIED: dto/printer.rs `WsEvent` + `is_visible_to` прочитаны — сейчас оба request-события идут только Admin|Manager].
**Точка отправки:** `RequestService::transition` (строка ~410), `approve_ad_register` (~543), `reject_ad_register` (~661) — ВСЕ три места отправляют `RequestStatusChanged` и должны заполнить новое поле. `transition` имеет `dto` (через `self.get`) с `requested_by_user_id`; остальные два — `current.requested_by_user_id`/`target_user_id`. Планировщик должен покрыть все три.

### Pattern 3: Узкий CreateRequest-гейтед эндпоинт (D-PRN-01)

**What:** Новая команда/handler, минимальный DTO, гейт `Action::CreateRequest` в сервис-слое.
**Source данных:** `devices` где `type_id = 2` + `LEFT JOIN locations l ON d.location_id = l.id` (паттерн уже в `devices_sqlite.rs`, `l.name` = индекс 15). Сортировка `ORDER BY l.name NULLS LAST, d.name`.
**Где разместить:** рядом с request-сервисом (узкий printer-picker) ИЛИ как метод в `device_service`/`printer_service` с CreateRequest-гейтом. CONTEXT даёт discretion. Рекомендация: метод в `RequestService` (например `printer_options(&self, caller)`) — семантически это «опции для формы заявки», гейт `CreateRequest` логичен там, и не загрязняет device/printer сервисы read-гейтом другого Action.
**DTO:**
```rust
#[derive(Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RequestPrinterOptionDto {
    #[specta(type = i32)]
    pub id: i64,          // device id (printer_device_id для RequestCreateDto)
    pub name: String,
    pub location: Option<String>,
}
```
**Регистрация (3 точки):**
1. Tauri-команда `request_printer_options` в `tauri_cmds/requests.rs` (тонкая обёртка над build_*).
2. axum handler + route в `http/requests.rs` (`POST /api/v1/request_printer_options`).
3. `crate::tauri_cmds::requests::request_printer_options` в `collect_commands!` (`specta_export.rs`).

### Recommended Project Structure (затрагиваемые файлы)
```
crates/trackly-core/src/
├── auth.rs                          # is_visible_to живёт в dto, НЕ тут; auth.rs не меняется (CreateRequest уже есть)
└── domain/requests.rs               # RequestRow += category_name
crates/trackly-app/src/
├── dto/request.rs                   # RequestDto += category_name; новый RequestPrinterOptionDto; (опц.) CategoryDto{id,name}
├── dto/printer.rs                   # WsEvent::RequestStatusChanged += requested_by_user_id; is_visible_to правка
├── services/request_service.rs      # 3 точки send(RequestStatusChanged) += поле; новый printer_options()
├── tauri_cmds/requests.rs           # новая команда request_printer_options; (опц.) categories {id,name}
├── http/requests.rs                 # новый handler+route
└── specta_export.rs                 # register new command
crates/trackly-infra/src/repos/
└── requests_sqlite.rs               # SELECT_REQUESTS += rc.name; map_row_request += get(N)
ui/src/
├── bindings.ts / bindings-phase6.ts # РЕГЕНЕРИРОВАТЬ (specta) — WsEvent += requestedByUserId; новый DTO
├── lib/api/ws.ts                    # без структурных изменений (union из bindings)
├── features/requests/RequestDetail.svelte    # {request.categoryName ?? '—'} вместо {request.categoryId}
├── features/requests/RequestFormModal.svelte  # loadPrinters → новый эндпоинт; новый дропдаун
├── features/requests/RequestsPage.svelte      # employee-ветка WS-обработчика (тост/нотификация по своей заявке)
├── features/layout/EmployeeLayout.svelte      # (опц.) запрос Notification permission деликатно
└── lib/components/<GroupedPrinterSelect>.svelte  # НОВЫЙ кастомный сгруппированный дропдаун
```

### Anti-Patterns to Avoid
- **Клиентская фильтрация чужих WS-событий как защита.** D-RBAC-03: UI-фильтр — только UX. Если `is_visible_to` пропустит `RequestStatusChanged` ВСЕМ сотрудникам, любой employee увидит статусы чужих заявок (утечка). Фильтр по `requested_by_user_id` ОБЯЗАН быть на сервере в `is_visible_to`.
- **Добавление столбца в середину `SELECT_REQUESTS`** без сдвига индексов `map_row_request` → молчаливый сбой/паника. Добавляй последним.
- **Хардкод списка категорий в форме и параллельно серверный список** — выбрать одно. Сейчас форма хардкодит (RequestFormModal.svelte стр. 41-46). Если планировщик добавляет серверный `{id,name}`, заменить хардкод, не дублировать.
- **Запрос Notification.requestPermission при загрузке страницы** — браузеры штрафуют/блокируют. Деликатно: при первой отправке заявки или по явному действию (см. Pitfall 3).
- **Двойная отправка WS на десктопе.** `tauri_cmds/requests.rs` уже НЕ зовёт `app.emit` напрямую (gap-closure) — bridge в main.rs форвардит. Не возвращать прямые emit.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Доставка WS-события сотруднику | Новый WS-канал/per-user сокет-роутинг | Существующий `broadcast` + `is_visible_to` фильтр | Инфраструктура fan-out + ролевой фильтр уже есть и протестирована (Pitfall 5/6 учтены в ws.rs). |
| Реконнект/бэкофф WS в браузере | Свой reconnect | `ws.ts` уже делает экспоненциальный бэкофф + единый «reconnecting» toast | Учтён баг toast-spam (debug session ui-ws-toast-reports-flicker). |
| Детект скрытой вкладки | Таймеры/`window.onblur` | `document.hidden` + `visibilitychange` (Page Visibility API) | Стандарт; `onblur` ложно срабатывает на смене фокуса внутри страницы. |
| Системная нотификация | Свой in-app «toast как окно» | Web Notification API | Нативная ОС-нотификация — то, что просит пользователь при свёрнутой вкладке. |
| Имя категории/принтера | Второй запрос с фронта на каждую заявку | LEFT JOIN на сервере (как `requester_name`) | Один запрос; консистентно с существующим паттерном. |

**Key insight:** Phase 11 — это «дозаполнить уже проложенные борозды», а не строить новое. Главный риск — рассинхрон серверного payload и клиентской фильтрации (D-WS-01), а не отсутствие инструментов.

## Common Pitfalls

### Pitfall 1: `is_visible_to` и payload меняются раздельно
**What goes wrong:** Меняют `is_visible_to`, но не добавляют `requested_by_user_id` — нечем авторизовать; ИЛИ добавляют поле, но `is_visible_to` всё ещё блокирует сотрудника → событие не доходит.
**Why:** Два изменения логически связаны, лежат в разных местах одного файла (`dto/printer.rs`).
**How to avoid:** Делать как одну атомарную правку DTO. Тест `is_visible_to`: employee-автор → true; employee-не-автор → false; admin/manager → true.
**Warning signs:** Сотрудник не получает тост вообще, ИЛИ получает тост о чужой заявке.

### Pitfall 2: Сдвиг индексов в `map_row_request`
**What goes wrong:** Вставка `rc.name` в середину `SELECT_REQUESTS` ломает все `row.get(n)`.
**How to avoid:** Добавить `category_name` ПОСЛЕДНИМ столбцом (после `r.ad_subtype`), индекс = последний+1. `ad_subtype` сейчас индекс 17 → `category_name` индекс 18.
**Warning signs:** Тесты репозитория падают на типах/панике `get`.

### Pitfall 3: Notification API требует secure context — на HTTP-fallback не работает
**What goes wrong:** Сотрудник зашёл по первому-запуску HTTP-fallback (CLAUDE.md: «fall back to HTTP for first-run if cert generation hasn't happened»). `Notification.requestPermission()`/`new Notification()` недоступны вне secure context (HTTPS / `localhost` / `127.0.0.1`).
**Why:** Спецификация: Notifications API доступен только в secure context. LAN-IP по HTTP (`http://192.168.x.x:port`) — НЕ secure context. Штатный путь — `https://0.0.0.0:8443` self-signed (secure context, даже с не-доверенным сертификатом после принятия).
**How to avoid:** Перед использованием проверять `window.isSecureContext` и `'Notification' in window`. Если недоступно — graceful degrade: всегда показывать тост (даже при `document.hidden`). Никогда не падать. В Tauri webview (WebView2/WKWebView) контекст считается доверенным — нотификации работают.
**Warning signs:** `Notification is not defined` / `requestPermission` отклоняется молча на LAN-HTTP.
**Source:** [CITED: MDN Notifications API — secure context requirement; localhost/127.0.0.1 trustworthy].

### Pitfall 4: Permission-prompt в неудачный момент
**What goes wrong:** Запрос разрешения на нотификации при первом рендере → браузер блокирует/штрафует домен, пользователь жмёт «Блокировать» навсегда.
**How to avoid:** Запрашивать `requestPermission()` ТОЛЬКО в ответ на жест пользователя — рекомендация: сразу после успешной отправки первой заявки (`pushToast('success','Заявка отправлена')` в RequestFormModal). Хранить, что уже спрашивали (флаг). Если `Notification.permission === 'default'` и пользователь только что отправил заявку — деликатный запрос.
**Warning signs:** `Notification.permission === 'denied'` массово.

### Pitfall 5: Сотрудник не подключён к WS
**What goes wrong:** `connectWs()`/`onWsEvent()` вызываются в `onMount` у `RequestsPage.svelte`. Employee-лендинг = «Мои заявки» (= RequestsPage), так что подписка происходит — НО если employee на другом экране (профиль) или RequestsPage размонтирован, событие пропущено.
**Why:** Подписка живёт в компоненте страницы, а не в оболочке.
**How to avoid:** Рассмотреть подъём `connectWs`/`onWsEvent` в `EmployeeLayout.svelte` (живёт всю сессию сотрудника), а тост/нотификация — оттуда. CONTEXT даёт это на discretion («employee shell vs общий ws-клиент»). Рекомендация: подписка в `EmployeeLayout` для employee, чтобы нотификация работала независимо от текущего экрана. Убедиться, что нет двойной подписки (RequestsPage уже подписан для admin/manager toast’ов «новая заявка»).
**Warning signs:** Нотификация приходит только когда открыт список заявок.

### Pitfall 6: Дев-среда не достаёт реальных принтеров/AD
**What goes wrong:** Проверка D-PRN-01/D-WS-01 на macOS без принтеров/AD.
**How to avoid:** D-PRN-01 не требует SNMP — `devices` type_id=2 это просто строки в БД (можно создать вручную в dev). D-WS-01 не требует AD — достаточно локального employee-пользователя (есть с Phase 5/10). Проверка ручная (frontend test runner вне scope). Серверная логика (`is_visible_to`, JOIN, новый эндпоинт) — покрывается Rust-тестами без внешних систем.
**Warning signs:** Попытка тестировать через реальный принтер/AD.

### Pitfall 7: LAN-браузер видит устаревший `ui/dist`
**What goes wrong:** После правок фронта server-mode отдаёт старый бандл.
**How to avoid:** После любых изменений фронта — `pnpm --dir ui build` (MEMORY: dev_browser_testing_needs_ui_build). `cargo tauri dev` HMR’ит только desktop webview, не LAN-бандл.

## Code Examples

### D-CAT-01 — рендер имени категории (RequestDetail.svelte, строки 387-392)
```svelte
<!-- Source: VERIFIED — текущий код рендерит {request.categoryId} -->
{#if request.categoryName}
  <div class="field">
    <span class="field-label">Категория</span>
    <span class="field-value">{request.categoryName}</span>
  </div>
{/if}
<!-- free_form без категории: блок не рисуется (или показать прочерк — discretion) -->
```

### D-WS-01 — employee-обработчик (тост vs нотификация)
```typescript
// Source: паттерн из RequestsPage.svelte handleWsEvent (VERIFIED), расширенный для employee.
function handleEmployeeWsEvent(event: WsEvent) {
  if (event.type !== 'request_status_changed') return;
  // Серверный is_visible_to уже гарантирует: сотрудник получает только СВОИ.
  // Клиентская проверка — подстраховка/выбор текста, НЕ безопасность.
  const text = statusToastText(event.newStatus); // 'Ваша заявка принята в работу' и т.п.
  const canNotify =
    'Notification' in window &&
    window.isSecureContext &&
    Notification.permission === 'granted';
  if (document.hidden && canNotify) {
    new Notification('Trackly', { body: text });
  } else {
    pushToast(event.newStatus === 'rejected' ? 'info' : 'success', text);
  }
}
// RU-тексты (discretion): in_progress→'Ваша заявка принята в работу';
//   completed→'Ваша заявка выполнена'; rejected→'Ваша заявка отклонена'.
```

### D-WS-01 — деликатный запрос разрешения (после первой отправки заявки)
```typescript
// Вызывать в onSuccess формы заявки, один раз.
function maybeRequestNotifyPermission() {
  if (!('Notification' in window) || !window.isSecureContext) return;
  if (Notification.permission === 'default') {
    void Notification.requestPermission(); // жест пользователя = отправка заявки
  }
}
```

### D-PRN-01 — сгруппированный дропдаун (структура данных)
```typescript
// options: RequestPrinterOptionDto[] с сервера, уже отсортированы по location.
// Группировка на клиенте по location для рендера заголовков-полосок.
const groups = $derived.by(() => {
  const map = new Map<string, RequestPrinterOptionDto[]>();
  for (const p of options) {
    const key = p.location ?? 'Без расположения';
    (map.get(key) ?? map.set(key, []).get(key)!).push(p);
  }
  return [...map.entries()]; // [location, printers[]]
});
// Заголовок группы: <div class="group-header"> с серым фоном (var(--color-surface-sunken)).
// Пустой список: показать 'Принтеры не найдены' (discretion).
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Категория = число в UI | Имя через серверный JOIN | Phase 11 | D-CAT-01 |
| `RequestStatusChanged` виден только Admin/Manager | + автор-сотрудник видит своё | Phase 11 | D-WS-01 |
| Принтеры в форме через `devices.list` (ReadData) | Узкий CreateRequest-эндпоинт | Phase 11 (регрессия Phase 10) | D-PRN-01 |

**Deprecated/outdated:** Прямой `app.emit("trackly-event")` в Tauri-командах заявок — уже удалён (gap-closure), bridge в main.rs единственный источник десктоп-emit. Не восстанавливать.

## Runtime State Inventory

> Phase 11 — НЕ rename/refactor/migration. Раздел применим частично (миграции/сид).

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | `request_categories` уже засеяна 4 RU-именами (V024). `requests.category_id` хранит FK (число «3» = id «Программное обеспечение»). | None — данные есть; нужен только JOIN на чтение. |
| Live service config | None — verified: WS/нотификации не имеют внешней конфигурации. | None. |
| OS-registered state | None — verified: фаза не трогает Task Scheduler/pm2/launchd. | None. |
| Secrets/env vars | None — verified: фаза не вводит новых секретов/env. | None. |
| Build artifacts | `ui/dist` бандл — устаревает после правок фронта. `ui/src/bindings.ts` — регенерируется при новых командах/DTO через `cargo test` (export_bindings) или `cargo run --bin export_bindings`. | `pnpm --dir ui build` для LAN; регенерация bindings обязательна. |

**Миграции БД:** НЕ требуются. Подтверждено: `request_categories` (V024) создана+засеяна; `devices`/`locations` существуют; `requests.category_id`/`printer_device_id` уже есть. Все изменения — DTO/JOIN/эндпоинты/UI.

## Common Pitfalls (Project Constraints from CLAUDE.md)

## Project Constraints (from CLAUDE.md)
- **Один DTO, два транспорта.** Новый printer-picker и любой новый payload — в сервис-слой; Tauri-команда и axum-handler тонкие, делегируют в общий `build_*`/сервис. (D-PRN-01, опц. категории.)
- **Источник истины безопасности — `authorize()`/`is_visible_to` в Rust.** UI-фильтрация — только UX (D-RBAC-03). Применяется к WS-видимости (D-WS-01).
- **Single-writer SQLite / reader-pool.** Новый read-эндпоинт (printer-picker) и JOIN идут через reader-pool в `spawn_blocking` (как существующие read-методы). Никаких прямых записей.
- **RU-only UI (v1).** Все тосты/нотификации/заголовки групп — по-русски.
- **rusqlite + параметризованные запросы.** Новый SELECT — через `params![]`, без конкатенации.
- **Portable mode.** Ничего в `%APPDATA%`; фаза не пишет файлов вне рабочего каталога.
- **specta/ts-rs регенерация bindings** при новых командах/DTO — иначе фронт не увидит типы.
- **GSD workflow enforcement (CLAUDE.md):** правки только через GSD-команду.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Tauri webview (WebView2/WKWebView) считается secure context → Notification API работает на десктопе без HTTPS. | Pitfall 3 | Если нет — десктоп-нотификация не сработает; mitigated тем, что десктоп-админ это не employee-сценарий, и есть fallback на тост. Проверить вручную на Windows. |
| A2 | Размещение employee-WS-подписки в `EmployeeLayout` — лучший вариант (vs RequestsPage). | Pitfall 5 | CONTEXT даёт это на discretion; неверный выбор = нотификация только на экране заявок, не блокер. Планировщик решает. |
| A3 | Серверный список категорий `{id,name}` опционален (форма хардкодит). | Summary / D-CAT-01 | Если пользователь добавит категории в БД, хардкод формы разойдётся с данными. Низкий риск в v1 (4 фикс. категории), но JOIN на чтение это уже закрывает для отображения. |
| A4 | Имя нового эндпоинта/DTO (`request_printer_options`/`RequestPrinterOptionDto`) — на discretion. | D-PRN-01 | Косметика; планировщик финализирует. |

## Open Questions (RESOLVED)

1. **Где запрашивать Notification permission?**
   - Что знаем: деликатно, по жесту пользователя (Pitfall 4). RequestFormModal `onSuccess` — естественный момент.
   - Что неясно: хочет ли пользователь явную кнопку «включить уведомления» вместо авто-промпта.
   - Рекомендация: авто-промпт один раз после первой успешной отправки заявки; не повторять если `denied`.
   - **RESOLVED:** план 11-03 запрашивает разрешение после первой успешной отправки заявки (gesture-gated), без повтора при `denied`.

2. **Сервер vs хардкод категорий.**
   - Что знаем: форма хардкодит 4 категории; отображение чинится JOIN-ом.
   - Рекомендация: реализовать JOIN (обязательно для D-CAT-01 display). Серверный `{id,name}`-список — по желанию планировщика (устраняет дублирование, согласуется с CONTEXT «список должен возвращать {id,name}»). Если делать — заменить хардкод в RequestFormModal.
   - **RESOLVED:** CONTEXT D-CAT-01 фиксирует `{id,name}` как решение → план 11-01 доставляет серверный `{id,name}`-список и заменяет хардкод в RequestFormModal (без дублирования).

3. **Переиспользовать ли printer-picker для admin/manager формы?**
   - Что знаем: admin/manager имеют `devices.list`, текущий код работает для них.
   - Рекомендация: переиспользовать единый эндпоинт для всех ролей (упрощает форму) — но не обязательно; CONTEXT допускает оставить admin/manager как есть.
   - **RESOLVED:** план 11-02 использует единый `request_printer_options`-эндпоинт для формы заявки всех ролей (employee имеет CreateRequest; admin/manager — тоже).

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain / cargo | Все backend-правки + тесты | ✓ (проект собирается) | ≥1.85 | — |
| pnpm + Vite | Сборка фронта / LAN-бандл | ✓ | — | npm |
| Реальные принтеры (SNMP) | D-PRN-01 | ✗ (dev macOS) | — | `devices` type_id=2 создаются вручную в БД; SNMP не нужен для списка |
| Active Directory | D-WS-01 | ✗ (dev macOS) | — | Локальный employee-пользователь (есть с Phase 5/10) |
| HTTPS secure context (LAN) | Notification API | ✓ штатно (`:8443` self-signed) | — | HTTP-fallback → graceful degrade на тост (Pitfall 3) |

**Missing dependencies with no fallback:** Нет — все находки проверяемы без AD/принтеров.
**Missing dependencies with fallback:** Принтеры (ручные записи в БД); AD (локальный пользователь).

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust встроенный `#[test]` (+ `cargo nextest` опционально); фронтенд-runner ВНЕ scope (CONTEXT deferred) |
| Config file | none для FE; Rust — Cargo workspace |
| Quick run command | `cargo test -p trackly-app -p trackly-infra -p trackly-core <module>` |
| Full suite command | `cargo test` (один за раз — MEMORY: no concurrent cargo test) |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| D-CAT-01 | `SELECT_REQUESTS` JOIN отдаёт `category_name` для free_form с категорией; `None` без | unit | `cargo test -p trackly-infra requests_sqlite` | ❌ Wave 0 (расширить существующий request-repo тест) |
| D-WS-01 | `WsEvent::RequestStatusChanged.is_visible_to`: employee-автор→true, employee-чужой→false, admin/manager→true | unit | `cargo test -p trackly-app ws_event_visibility` | ❌ Wave 0 (новый тест в dto/printer.rs) |
| D-WS-01 | `transition`/`approve_ad_register`/`reject_ad_register` шлют payload с `requested_by_user_id` | unit/integration | `cargo test -p trackly-app request_service` | ❌ Wave 0 |
| D-PRN-01 | Новый эндпоинт: employee (CreateRequest) → 200 + список; возвращает только {id,name,location}, отсортировано по location | integration | `cargo test -p trackly-app request_printer_options` | ❌ Wave 0 (расширить Phase 5/10 role×endpoint матрицу) |
| D-PRN-01 | employee БЕЗ обхода: эндпоинт НЕ раскрывает прочие поля устройства | integration | (тот же) | ❌ Wave 0 |
| D-CAT-01/WS/PRN | Frontend (рендер имени, тост/нотификация, дропдаун) | manual | — | manual (FE runner вне scope) |

### Sampling Rate
- **Per task commit:** `cargo test -p <crate> <module> -x` (затронутый модуль).
- **Per wave merge:** `cargo test` (полный, один процесс).
- **Phase gate:** полный `cargo test` зелёный + ручная проверка трёх находок в LAN-браузере (после `pnpm --dir ui build`) и в desktop-режиме.

### Wave 0 Gaps
- [ ] `crates/trackly-app/src/dto/printer.rs` — тест `is_visible_to` для `RequestStatusChanged` (3 кейса роли/авторства).
- [ ] `crates/trackly-infra/src/repos/requests_sqlite.rs` — расширить тест: `category_name` для заявки с/без категории.
- [ ] role×endpoint матрица (Phase 5/10) — добавить новый printer-picker эндпоинт (employee 200, проверка минимального DTO).
- [ ] `bindings.ts` регенерация проверяется существующим `tests/export_bindings.rs` (запускается на `cargo test`).

## Security Domain

> `security_enforcement: true`, ASVS level 1.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | Не меняется; WS auth-middleware (сессия) уже есть. |
| V3 Session Management | no | Существующая `tower-sessions`; не трогается. |
| V4 Access Control | **yes** | `is_visible_to` (WS) + `authorize(CreateRequest)` (printer-picker). Объектный доступ: сотрудник видит WS-события ТОЛЬКО своих заявок (BOLA-закрытие, как в `RequestService::get`). |
| V5 Input Validation | yes | Новый эндпоинт — без пользовательского ввода в SQL (нет фильтров «на первое время»); параметризованные запросы. |
| V6 Cryptography | no | Нет нового крипто. |
| V7 Error Handling | yes | Эндпоинт возвращает `AppError`; пустой список ≠ ошибка. |

### Known Threat Patterns for {Rust/axum/Svelte LAN}

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| BOLA: сотрудник получает WS-события чужих заявок | Information Disclosure | `is_visible_to` фильтрует по `identity.user_id == requested_by_user_id` НА СЕРВЕРЕ (не на клиенте). |
| Утечка данных устройства через широкий printer-picker | Information Disclosure | DTO отдаёт строго `{id,name,location}`; никаких SNMP/community/ip полей. |
| Привилегированное чтение принтеров через новый эндпоинт | Elevation of Privilege | Гейт `Action::CreateRequest` (employee имеет), НЕ открывает `ReadPrinters`/`ReadData`, закрытые в Phase 10. Не регрессировать матрицу auth.rs. |
| XSS через имя категории/location в нотификации/тосте | Tampering | Svelte экранирует текст по умолчанию; `new Notification(..., {body})` принимает plain text (не HTML). |

## Sources

### Primary (HIGH confidence)
- Прочитанный исходный код (VERIFIED):
  - `crates/trackly-app/src/dto/request.rs`, `dto/printer.rs` (WsEvent, is_visible_to)
  - `crates/trackly-app/src/services/request_service.rs` (3 точки send RequestStatusChanged)
  - `crates/trackly-app/src/http/ws.rs`, `http/requests.rs`, `tauri_cmds/requests.rs`, `specta_export.rs`
  - `crates/trackly-infra/src/repos/requests_sqlite.rs` (SELECT_REQUESTS, map_row_request), `devices_sqlite.rs` (LEFT JOIN locations)
  - `crates/trackly-core/src/auth.rs` (Identity, authorize матрица — CreateRequest/ReadData/ReadPrinters)
  - `migrations/V024__request_categories.sql` (сид 4 категорий, FK)
  - `ui/src/lib/api/ws.ts`, `features/requests/RequestFormModal.svelte`, `RequestDetail.svelte`, `RequestsPage.svelte`, `features/layout/EmployeeLayout.svelte`, `bindings-phase6.ts`
- CLAUDE.md (проектные конструкции), 10-CONTEXT.md, 11-CONTEXT.md.

### Secondary (MEDIUM confidence)
- [MDN — Notifications API](https://developer.mozilla.org/en-US/docs/Web/API/Notifications_API) — secure-context требование.
- [MDN — Notification.requestPermission()](https://developer.mozilla.org/en-US/docs/Web/API/Notification/requestPermission_static)
- [MDN — Secure contexts](https://developer.mozilla.org/en-US/docs/Web/Security/Defenses/Secure_Contexts) — localhost/127.0.0.1 trustworthy; LAN-IP по HTTP — нет.
- [MDN — Page Visibility API](https://developer.mozilla.org/en-US/docs/Web/API/Page_Visibility_API)

### Tertiary (LOW confidence)
- A1 (Tauri webview = secure context) — проверить вручную на Windows.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — без новых зависимостей; всё на читаном коде.
- Architecture: HIGH — все три потока прослежены по фактическому коду.
- Pitfalls: HIGH (бэкенд/индексы/видимость), MEDIUM (Notification secure-context — поведение зависит от среды запуска).

**Research date:** 2026-06-21
**Valid until:** 2026-07-21 (стабильно — внутренний код + браузерные стандарты).
