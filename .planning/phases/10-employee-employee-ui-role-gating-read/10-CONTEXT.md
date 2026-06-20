# Phase 10: Ограничение роли employee — доступ только к Заявкам + отдельный employee-UI + role-gating read-эндпоинтов - Context

**Gathered:** 2026-06-21
**Status:** Ready for planning
**Source:** /gsd-discuss-phase 10 — 4 области (форма employee-UI, backend read-gating, видимость заявок, поведение при отказе) обсуждены интерактивно.

<domain>
## Phase Boundary

Сделать роль «Сотрудник» (employee) по-настоящему ограниченной после того, как AD-вход (Phase 9)
дал сотрудникам реальный доступ в систему. Три вещи:

1. **Отдельный упрощённый employee-UI** — у сотрудника своя минимальная оболочка (лендинг на
   «Заявки», действие «Новая заявка», профиль/выход), а не урезанный общий интерфейс.
2. **Role-gating read-эндпоинтов на бэкенде** — сейчас чтение устройств/актов/картриджей
   (`list`/`get`/`search`) вообще НЕ вызывает `authorize()`, а матрица `authorize()` отдаёт
   `ReadData`/`ReadRequests` всем ролям. Employee может прочитать всё через API. Закрыть это.
3. **Проверка (CI role × endpoint)** — расширить матрицу Phase 5 (которая покрывала мутации/403)
   на READ-эндпоинты: employee → 403 на чтение чужих разделов, 200 на свои заявки.

**В scope:**
- **Отдельная employee-оболочка (D-UI-01):** минимальный layout для роли employee — без полного
  sidebar; стартовая страница = «Мои заявки»; кнопка «Новая заявка»; выход. Сотрудник физически
  не видит навигацию на чужие разделы.
- **Backend read-gating (D-GATE-01/02):** закрыть от employee чтение devices / acts / cartridges /
  printers / reports / users. Employee сохраняет ТОЛЬКО: `ReadRequests`, `CreateRequest` и доступ к
  **упрощённому** дашборду (D-GATE-03). Механизм: добавить `authorize(caller, &Action::ReadX)` в
  read-методы сервисов (где их сейчас нет) + изменить матрицу `authorize()` так, чтобы
  `ReadData`/`ReadPrinters` больше не были `true` для Employee.
- **Упрощённый employee-дашборд (D-GATE-03):** для employee дашборд показывает ТОЛЬКО данные его
  собственных заявок (счётчики статусов / последние), без org-wide метрик устройств/картриджей/
  принтеров — иначе дашборд = обход read-gating.
- **Заявки только свои (D-REQ-01):** employee в разделе «Заявки» видит только заявки, где он
  `requested_by_user_id`; фильтр в `request_service.list` / status_counts / history для роли Employee.
- **Поведение при отказе (D-DENY-01):** экран «Нет доступа» (403) с кнопкой «К Заявкам» при прямом
  переходе employee на запрещённый роут; обработка 403 от API в `client.ts` рядом с существующим
  401-редиректом.
- **CI-проверка (D-TEST-01):** расширить тест-матрицу role × endpoint на read-эндпоинты.

**НЕ в scope этой фазы (явно deferred):**
- Org-метрики любого вида в employee-дашборде (намеренно request-scoped).
- Тонкая настройка «свои/все заявки» для manager (manager остаётся как сейчас — видит все, кроме
  ad_register).
- Любые изменения прав admin/manager сверх того, что требуется для read-gating.

**Mode:** standard (не MVP).

</domain>

<decisions>
## Implementation Decisions

### Форма employee-UI

#### D-UI-01: отдельная упрощённая employee-оболочка
- Employee получает СВОЙ минимальный layout, а не отфильтрованный общий. Стартовая страница —
  «Мои заявки»; доступные действия — «Новая заявка», профиль/выход. Без полного sidebar и без
  навигации на остальные разделы. Отвечает букве ROADMAP «отдельный employee-UI» и core value
  «одной кнопкой» для нетех-сотрудника.
- **Discretion планировщику:** тонкая ветка над существующим `Layout.svelte` vs полностью отдельный
  shell-компонент — на усмотрение, в рамках UI-SPEC паттернов. Главное требование: сотрудник физически
  не видит и не может навигировать на чужие разделы из своей оболочки.

### Backend read-gating

#### D-GATE-01: закрыть от employee чтение всего, кроме Заявок и (упрощённого) Дашборда
- Employee сохраняет ТОЛЬКО: `ReadRequests`, `CreateRequest`, упрощённый дашборд (D-GATE-03).
- Закрываются для employee: чтение devices, acts, cartridges, printers, reports, users.

#### D-GATE-02: механизм — authorize() в read-сервисах + изменение матрицы
- Источник истины — `authorize(caller, &Action::…)` в сервис-слое (переиспользуем D-RBAC-01 Phase 5).
- Сейчас read-методы (`device_service.get/list/search/list_grouped`, аналогично act/cartridge/printer/
  report) **не вызывают** `authorize()` — добавить явные read-гейты. Это требует протянуть
  `caller: &Identity` в те read-методы, где его сейчас нет в сигнатуре.
- Изменить матрицу `authorize()` в `crates/trackly-core/src/auth.rs`: `ReadData` и `ReadPrinters`
  больше НЕ возвращают `true` для `Role::Employee` (Admin|Manager). `ReadRequests` + `CreateRequest`
  остаются доступны employee.
- **Discretion планировщику:** точные сигнатуры протяжки `caller`, гранулярность Action-ов (использовать
  существующие `ReadData`/`ReadPrinters`/`ReadRequests` vs добавить более узкие) — на усмотрение, лишь бы
  каждый закрытый read-эндпоинт реально проверял роль.

#### D-GATE-03: employee-дашборд — только request-scoped данные сотрудника
- Для роли employee дашборд отдаёт ТОЛЬКО данные его собственных заявок (счётчики по статусам,
  последние заявки), БЕЗ org-wide метрик устройств/картриджей/принтеров.
- Причина: `dashboard_service` сейчас агрегирует org-wide данные устройств/картриджей/принтеров —
  если employee увидит их, это утечка тех самых read-данных, которые гейтим. Дашборд для employee
  не должен быть дырой в read-gating.

### Видимость заявок

#### D-REQ-01: employee видит только свои заявки
- Фильтр по `requested_by_user_id == caller.user_id` для роли Employee в `request_service.list`
  (и соответствующих status_counts / get_history). Реализует существующий комментарий в `auth.rs`
  («сотрудник видит только свои»), который пока не реализован в `list()`.
- Manager/Admin — без этого фильтра; существующее скрытие `ad_register` для не-admin сохраняется.

### Поведение при отказе доступа

#### D-DENY-01: экран «Нет доступа» (403) + обработка 403 в client.ts
- Прямой переход employee на запрещённый роут (по URL) → route-guard показывает страницу-заглушку
  «Нет доступа» с пояснением и кнопкой «К Заявкам».
- 403 от API обрабатывается в `ui/src/lib/api/client.ts` рядом с существующим 401-редиректом на логин
  (сейчас обрабатывается только 401).
- UI-gating (скрытие навигации) остаётся UX-слоем; источник истины безопасности — backend `authorize()`
  (переиспользуем D-RBAC-03 Phase 5: на UI-скрытие не полагаемся для защиты).

### Проверка

#### D-TEST-01: расширить CI-матрицу role × endpoint на read-эндпоинты
- Расширить существующую тест-матрицу Phase 5 (мутации → 403) на READ-эндпоинты: Employee → 403 на
  чтение devices / acts / cartridges / printers / reports / users; Employee → 200 на свои заявки;
  Employee не видит чужие заявки (D-REQ-01); Employee → корректный упрощённый дашборд (D-GATE-03).
  Manager/Admin — read разрешён. Это deliverable фазы (ROADMAP «проверить role-gating read-эндпоинтов»).

### Claude's Discretion
- Точная вёрстка/копирайт employee-оболочки и экрана «Нет доступа» — в рамках UI-SPEC паттернов.
- Тонкая ветка над `Layout.svelte` vs отдельный shell-компонент для employee (D-UI-01).
- Точный состав упрощённого employee-дашборда в пределах request-scoped данных юзера (D-GATE-03).
- Гранулярность `Action`-ов и сигнатуры протяжки `caller: &Identity` в read-методы (D-GATE-02).
- Нужна ли миграция БД — скорее всего НЕТ (всё на уровне authorize + сервисных фильтров + UI); подтвердить при планировании.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Roadmap & requirements
- `.planning/ROADMAP.md` §«Phase 10: Ограничение роли employee …» — goal, depends-on Phase 9.
- `.planning/REQUIREMENTS.md` — USR-02 (стр. 116: три роли, «Сотрудник — только создание заявок»),
  USR-06 (enforcement роли нельзя обойти через API), REQ-06 (заявки). USR-02 — основной анкор scope.

### Прошлые решения (переиспользуются, не пересоздавать)
- `.planning/phases/05-auth-server-mode/05-CONTEXT.md` — **D-RBAC-01** (единый `authorize()` в сервис-слое
  = источник истины; CI role × endpoint), **D-RBAC-02** (employee заведён и тестируется), **D-RBAC-03**
  (UI-gating = только UX; sidebar фильтруется по роли; на UI-скрытие не полагаться для защиты).
- `.planning/phases/09-ad/09-CONTEXT.md` — AD-вход дал employee реальный доступ; роль по умолчанию для
  AD-юзеров = employee (D-REG-01/02). Контекст того, почему ограничение employee стало актуальным сейчас.
- `.planning/STATE.md` §«Phase 6 gap-closure» — **D-GAP-Employee-Access**: полноценный вход сотрудника
  отложен до AD; «сейчас только корректный ролевой рендер». Phase 10 закрывает этот долг.

### Существующий код — backend (точки интеграции)
- `crates/trackly-core/src/auth.rs` — `Role` (Admin/Manager/Employee), `Action` enum, `authorize()`
  матрица (стр. ~136). **Здесь меняется матрица** (D-GATE-02): `ReadData`/`ReadPrinters` убрать из
  `true`-для-всех. Комментарий «сотрудник видит только свои» (D-REQ-01) — рядом с `ReadRequests`.
- `crates/trackly-app/src/services/device_service.rs` — `get`/`list`/`search`/`list_grouped`/`list_by_ids`
  **не вызывают authorize()** — добавить read-гейт + протянуть `caller`.
- `crates/trackly-app/src/services/act_service.rs`, `cartridge_service.rs`, `printer_service.rs`,
  `report_service.rs` — аналогично: read-методы без authorize, закрыть от employee.
- `crates/trackly-app/src/services/dashboard_service.rs` — для employee отдавать только request-scoped
  данные (D-GATE-03); сейчас агрегирует org-wide метрики.
- `crates/trackly-app/src/services/request_service.rs` — `list` (стр. ~84, сейчас лишь
  `exclude_ad_register` для не-admin), `get_history`, status_counts: добавить фильтр «только свои» для
  employee (D-REQ-01). `create` уже вызывает `authorize(CreateRequest)`.
- `crates/trackly-app/src/http/*.rs` + `crates/trackly-app/src/tauri_cmds/*.rs` — тонкие транспорт-адаптеры;
  «один DTO, два транспорта». Read-гейт должен жить в сервисе, чтобы покрыть оба.
- CI-матрица role × endpoint из Phase 5 (найти существующий тест мутаций/403 — расширить на read).

### Существующий код — frontend (точки интеграции)
- `ui/src/features/layout/sidebar-config.ts` — `SIDEBAR_ITEMS` + `getVisibleItems(role)`; сейчас к admin
  привязаны ТОЛЬКО `/users` и `/settings`; остальное видно employee. Для employee-оболочки (D-UI-01)
  навигация переосмысливается.
- `ui/src/features/layout/Layout.svelte` — текущая единая оболочка (sidebar). Решить: ветка для employee
  vs отдельный shell (D-UI-01).
- `ui/src/App.svelte` — корневой shell; читает `status.user.role`; рендерит `<Layout>`. Тут добавляется
  route-guard / выбор оболочки по роли.
- `ui/src/routes.ts` — карта роутов (`/requests` = RequestsPage, и т.д.); без per-route role-guard сейчас.
- `ui/src/lib/api/client.ts` — `apiCall()` dual-transport; обрабатывает только 401-редирект. Добавить
  обработку 403 (D-DENY-01).
- `ui/src/lib/stores/auth.svelte` — `UserRole`, текущий пользователь/роль.
- `ui/src/pages/RequestsPage.svelte`, `ui/src/pages/Dashboard.svelte` — экраны, которые employee видит
  (упрощённо).

### Стек / практики
- `CLAUDE.md` — «единый authorize() для обоих транспортов»; «Session middleware gates /api/* except login»;
  роли admin/manager/employee; «один DTO, два транспорта».

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `authorize()` + `Action` enum (trackly-core): каркас RBAC готов — Phase 10 правит матрицу и
  добавляет вызовы, новой инфраструктуры не нужно.
- `Identity { user_id, role }` (trackly-core): уже прокидывается в мутирующие сервисы — паттерн протяжки
  `caller` существует, расширяется на read-методы.
- `request_service.list` уже принимает `caller: &Identity` и ветвится по роли (`exclude_ad_register`) —
  «только свои» (D-REQ-01) добавляется тем же приёмом.
- `getVisibleItems(role)` + `roles?` на `SidebarItem` — механизм роле-фильтрации sidebar уже есть.
- Phase 5 CI role × endpoint матрица — эталон, расширяется на read.

### Established Patterns
- Источник истины безопасности — `authorize()` в сервис-слое (D-RBAC-01); транспорты тонкие. Read-гейт
  кладём в сервис, не в handler.
- UI-gating = только UX (D-RBAC-03); защита всегда на backend.
- Single-writer / reader-pool: read-методы идут через reader-pool в `spawn_blocking`; добавление
  authorize-проверки — до обращения к пулу.
- Роли TEXT: admin/manager/employee; по-русски в UI (Администратор/Специалист/Сотрудник).

### Integration Points
- `crates/trackly-core/src/auth.rs::authorize()` — изменение матрицы (ReadData/ReadPrinters ≠ Employee).
- Read-методы device/act/cartridge/printer/report сервисов — добавить `authorize` + `caller`.
- `dashboard_service` — ветка employee → request-scoped данные.
- `request_service` — фильтр «только свои» для employee.
- `App.svelte` / route-guard + `client.ts` 403-обработка — поведение при отказе.
- Frontend employee-shell — отдельная оболочка для роли employee.

</code_context>

<specifics>
## Specific Ideas

- ROADMAP буквально просит «отдельный employee-UI» — пользователь подтвердил отдельную упрощённую
  оболочку (не просто отфильтрованный sidebar).
- Дашборд для employee оставлен, но строго request-scoped — пользователь выбрал «всё кроме Заявок +
  Дашборд», и дашборд должен показывать данные собственных заявок сотрудника, не org-метрики.
- Критическая находка для планировщика: read-эндпоинты сейчас вообще не вызывают `authorize()` — это
  не «ужесточение», а закрытие реальной дыры (employee может читать всё через API сегодня).

</specifics>

<deferred>
## Deferred Ideas

- Org-метрики/виджеты в employee-дашборде — намеренно не делаем (request-scoped only).
- Гибкая настройка «manager видит свои/все заявки» — вне scope; manager как сейчас.
- Более узкие/гранулярные `Action`-ы для каждого ресурса (если решат не использовать существующие
  `ReadData`/`ReadPrinters`) — на усмотрение планировщика, не обязательно.

None из обсуждения не ушло за пределы scope фазы.

</deferred>

---

*Phase: 10-employee-employee-ui-role-gating-read*
*Context gathered: 2026-06-21*
