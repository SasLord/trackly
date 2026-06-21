# Phase 11: Заявки/employee UX gap-closure - Context

**Gathered:** 2026-06-21
**Status:** Ready for planning
**Source:** UAT-находки пользователя после Phase 9/10 (live-verify). Спецификации даны пользователем напрямую — обсуждение не требуется, решения зафиксированы ниже.

<domain>
## Phase Boundary

Три находки UAT по разделу «Заявки» и опыту сотрудника, обрабатываются одним gap-closure:
1. Категория заявки показывается числом вместо текста.
2. Ответ администратора не доходит до сотрудника в реальном времени (нужен WS + тост + системная нотификация при неактивной вкладке).
3. Сотрудник не может завести заявку на замену картриджа — список принтеров пуст (регрессия Phase 10: read-gating закрыл `devices_list` от employee).

**НЕ в scope:** изменения ролевой модели/гейтинга сверх необходимого; новый раннер фронтенд-тестов; smtp/email.
</domain>

<decisions>
## Implementation Decisions

### D-CAT-01: категория заявки — текстом, не числом
- Сейчас `requests_categories_list` (tauri_cmds/requests.rs) возвращает только имена (`SELECT name FROM request_categories ORDER BY name`) без id; форма создания заявки шлёт `category_id` числом; `RequestDto.category_id` отдаётся числом; `RequestDetail.svelte` рисует число.
- **Решение:** категория должна отображаться текстовым названием из `request_categories.name`.
  - Список категорий должен возвращать `{ id, name }` (оба транспорта), чтобы форма слала корректный `category_id`.
  - Read-DTO заявки (`RequestDto`) должен прокидывать имя категории: добавить `category_name: Option<String>` через LEFT JOIN `request_categories` в `requests_sqlite.rs` (по образцу уже существующего join `users` для `requester_name`).
  - `RequestDetail.svelte` (и любой список заявок) рендерит `category_name`, а не `category_id`.
- **Discretion планировщику:** точные имена полей/DTO; для free_form-заявок без категории показывать прочерк/пусто. Проверить, что текущее значение «3» — это `category_id`, и что `request_categories` сидируется именами (миграция V24/V024).

### D-WS-01: ответ администратора сотруднику — WebSocket + тост + системная нотификация
- Бэкенд УЖЕ шлёт `WsEvent::RequestStatusChanged` при `RequestService::transition` (через `ctx.ws_broadcast`). Не хватает доставки сотруднику в браузере.
- **Решение:**
  - Сотрудник в браузере подписывается на WS (как админ/десктоп) и при `RequestStatusChanged` по СВОЕЙ заявке показывает тост (например «Ваша заявка принята в работу / отклонена / выполнена»).
  - Если вкладка свёрнута/неактивна (`document.hidden` / Page Visibility API) — показать системную нотификацию (Web Notification API) о результате; запросить разрешение (`Notification.requestPermission`) деликатно (например при первом заходе сотрудника или при первой отправке заявки).
  - Проверить/поправить `WsEvent::is_visible_to(&identity)` (http/ws.rs, dto/printer.rs) так, чтобы автор заявки (employee) получал событие об изменении статуса СВОЕЙ заявки. Не раскрывать сотруднику чужие события.
- **Discretion планировщику:** точные тексты тостов/нотификаций (RU), момент запроса разрешения на нотификации, формат полезной нагрузки `RequestStatusChanged` (содержит ли requested_by_user_id и новый статус — добавить при необходимости для фильтрации на клиенте и для текста).

### D-PRN-01: дропдаун принтеров для заявки сотрудника (регрессия Phase 10 + новый UX)
- `RequestFormModal.svelte` грузит принтеры через `devices.list({type_id:2})`, который Phase 10 закрыл от роли employee (D-GATE-02 → 403) → список пуст. Это блокирует заявку на замену картриджа.
- **Решение:**
  - Завести ОТДЕЛЬНЫЙ эндпоинт списка принтеров для формы заявки, доступный сотруднику (гейт по `Action::CreateRequest`, не `ReadData`). Возвращает минимум: id принтера, наименование, Расположение (location) — без прочих данных устройства. Оба транспорта (Tauri + axum), бинды регенерировать.
  - На первое время — отдавать ВЕСЬ список принтеров с их Расположением (без фильтра по доступу), отсортированный по Расположению.
  - UI: кастомный дропдаун, сгруппированный по Расположению. Для каждой группы — небольшая полоска с сереньким фоном и текстом Расположения (заголовок группы), под ней — принтеры этого Расположения. Сортировка групп по Расположению.
  - `RequestFormModal.svelte` переключается с `devices.list` на новый эндпоинт.
- **Discretion планировщику:** имя эндпоинта/DTO; точная вёрстка кастомного дропдауна в рамках UI-SPEC паттернов (group header = серый фон, см. описание пользователя); поведение при пустом списке принтеров; нужно ли то же для admin/manager формы (можно переиспользовать, но они и так имеют доступ к devices.list).

### Claude's Discretion
- Нужны ли миграции БД — вероятно нет (всё на уровне DTO/join/эндпоинтов/UI). Подтвердить при планировании.
- Где разместить логику подписки на WS для сотрудника (employee shell vs общий ws-клиент с ролевой фильтрацией).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Backend — заявки/категории
- `crates/trackly-app/src/services/request_service.rs` — `transition` уже шлёт `WsEvent::RequestStatusChanged`; `create`; `list`/`get` (D-REQ-01 employee-scope из Phase 10).
- `crates/trackly-app/src/dto/request.rs` — `RequestDto` (`category_id`, `requester_name`, `printer_name`), `RequestCreateDto`, `RequestTransitionPayload`.
- `crates/trackly-infra/src/repos/requests_sqlite.rs` — read-запросы заявок; уже есть `LEFT JOIN users` для `requester_name` (образец для join `request_categories`).
- `crates/trackly-app/src/tauri_cmds/requests.rs` — `requests_categories_list` (`SELECT name FROM request_categories`); добавить id. + http/requests.rs.
- `migrations/V24__request_categories.sql` (или V024) — схема/сид `request_categories`.

### Backend — WS
- `crates/trackly-app/src/http/ws.rs` — WS-хендлер, `ctx.ws_broadcast`, `WsEvent::is_visible_to(&identity)` фильтрация.
- `crates/trackly-app/src/dto/printer.rs` — `WsEvent` (включая `RequestStatusChanged`, `NewRequest`), `is_visible_to`.
- `main.rs` — bridge `ctx.ws_broadcast` → Tauri `app.emit("trackly-event", ...)` (Phase 9 ws-bridge quick task).

### Backend — принтеры
- `crates/trackly-app/src/tauri_cmds/printers.rs` + `http/printers.rs` — текущие `printers_list/get` (закрыты ReadData/ReadPrinters от employee).
- `crates/trackly-app/src/services/device_service.rs` / `printer_service.rs` — источник списка принтеров (type_id=2). Новый эндпоинт можно положить рядом с request-сервисом (гейт CreateRequest) или как узкий printer-picker.
- `crates/trackly-core/src/auth.rs` — `Action::CreateRequest` (employee имеет), матрица authorize.

### Frontend
- `ui/src/features/requests/RequestDetail.svelte` — рендер категории; тосты.
- `ui/src/features/requests/RequestFormModal.svelte` — `loadPrinters` (сейчас `devices.list({type_id:2})`), `printerDeviceId`.
- `ui/src/lib/ws.ts` (или эквивалент) — WS-клиент, обработчики `event.type` (`RequestStatusChanged`, `NewRequest`).
- `ui/src/lib/api/client.ts` — dual-transport `apiCall`.
- `ui/src/features/layout/EmployeeLayout.svelte` — оболочка сотрудника (Phase 10) — место для подписки/нотификаций.
- `ui/src/bindings.ts` — регенерировать после новых эндпоинтов/DTO.

### Прошлый контекст
- `.planning/phases/10-employee-employee-ui-role-gating-read/10-CONTEXT.md` — D-GATE-01/02 (что закрыто от employee), D-REQ-01 (employee видит свои заявки), D-DENY-01.
- `.planning/STATE.md` Quick Tasks — `09-ad-gaps-ws-bridge` (как работает ws_broadcast→desktop), `fix-fk-constraint-on-request-accept-assi`.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `WsEvent::RequestStatusChanged` + `ctx.ws_broadcast` + `is_visible_to` — инфраструктура WS уже есть; сотруднику нужна подписка + видимость своих событий.
- `LEFT JOIN users` для `requester_name` в `requests_sqlite.rs` — образец для join `request_categories.name`.
- `Action::CreateRequest` — employee уже имеет; новый printer-picker гейтить им.
- StatWidget/паттерны UI из Phase 10; кастомный дропдаун — новый компонент.

### Established Patterns
- «Один DTO, два транспорта»: новый printer-picker и categories{id,name} — в сервис-слой, оба транспорта тонкие.
- Read-gating: employee не имеет ReadData/ReadPrinters (Phase 10) — поэтому нужен отдельный CreateRequest-гейтед эндпоинт для принтеров формы заявки.
- WS: service-layer broadcast → ws_broadcast → (browser WS) / (Tauri bridge). RU-only UI.

### Integration Points
- `request_service.transition` → `RequestStatusChanged` → employee WS подписка → тост/нотификация.
- Новый printer-picker эндпоинт → `RequestFormModal` дропдаун (группировка по Расположению).
- `requests_sqlite` read join → `RequestDto.category_name` → `RequestDetail` рендер.

</code_context>

<specifics>
## Specific Ideas

- Дропдаун принтеров (пользователь дословно): кастомный, отсортирован по Расположению; группы = небольшая полоска с сереньким фоном и текстом Расположения, под ней — принтеры этого Расположения.
- На первое время показывать ВЕСЬ список принтеров (без фильтрации доступа) с Расположением.
- Системная нотификация — только когда браузер сотрудника неактивен/свёрнут; иначе тост.

</specifics>

<deferred>
## Deferred Ideas

- Фильтрация принтеров по доступу/локации пользователя — позже («на первое время весь список»).
- Email/SMTP-уведомления о результате — v2.
- Раннер фронтенд-тестов для WS/нотификаций/дропдауна — вне scope (проверка ручная, как в Phase 10).

</deferred>

---

*Phase: 11-requests-employee-ux-gaps*
*Context gathered: 2026-06-21*
