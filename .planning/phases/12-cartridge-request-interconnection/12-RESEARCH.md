# Phase 12: Взаимосвязь картриджной заявки - Research

**Researched:** 2026-06-22
**Domain:** Внутренняя доработка существующего флоу (Rust/Tauri/axum service layer + Svelte 5 UI) — без новых внешних зависимостей
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### Выбор картриджа в потоке установки из заявки
- **D-01:** В `OperationModal` при `op='install'`, открытом из заявки, добавить
  селектор физического картриджа из БД. Список фильтруется: только статус
  «На складе» **И** заряд «устанавливаемый» = `Полный(1)` / `Частичный(2)` (для
  картриджей; фотобарабаны не относятся к этому потоку — заявка «Замена картриджа»).
- **D-02:** Список дополнительно фильтруется по совместимости с моделью из заявки —
  `request.cartridgeModelId` (поле уже есть в `RequestDto`). Показываем экземпляры
  именно запрошенной модели картриджа.
- **D-03:** После выбора картриджа форма установки работает как раньше (Дата / Кто
  выдал / Кому выдал / Расположение), но `cartridge` теперь приходит из селектора, а
  не из пропа. Submit вызывает `cartridges.transition({op:'install', cartridge_id, …})`.

#### «Кому отдал» (given_to_name)
- **D-04:** Поле `Кому выдал` предзаполняется из автора заявки (`request.requesterName`),
  но остаётся **редактируемым** через существующий `PersonAutocomplete`. Заявку мог
  создать один человек, а картридж забрать другой — специалист может поправить.

#### Авто-подстановка «Расположение»
- **D-05:** Поле `Расположение` предзаполняется из расположения принтера заявки
  (location устройства типа «Принтер», `request.printerDeviceId`), остаётся
  **редактируемым** через `LocationAutocomplete`. Если у принтера расположение пустое —
  поле остаётся пустым (обычный ручной ввод).

#### Связь установленного картриджа с заявкой
- **D-06:** При завершении заявки после установки записывать выбранный картридж в
  `completedCartridgeId` заявки. Поток уже завершает заявку через
  `requests.transition({op:'complete', …, linkedCartridgeId})` — сейчас передаётся
  `null`, нужно передавать `id` установленного картриджа.
- **D-07:** Установленный картридж отражается в истории заявки (REQ-07 история) —
  человекочитаемо (код картриджа `C-000001` + модель), чтобы из карточки заявки было
  видно, что именно установили.

#### Сосуществование двух входов установки
- **D-08:** Сохранить **оба** входа установки картриджа: новый request-centric (из
  `RequestDetail`, с выбором картриджа) и старый cartridge-centric (меню картриджа
  «На складе» → «Установить в принтер», `cartridge` уже выбран). Старый вход не
  меняется и служит fallback'ом, когда установка идёт вне заявки.

### Claude's Discretion
- **DISC-01:** Если `request.cartridgeModelId` = `null` (заявка без выбранной модели) —
  fallback: показать все картриджи «На складе» с устанавливаемым зарядом, без фильтра
  совместимости. Реализационная деталь, оставлена планировщику.
- **DISC-02:** Если совместимых картриджей на складе нет — показать понятное
  пустое состояние («Нет подходящих картриджей на складе»); специалист может
  использовать старый cartridge-centric вход (D-08) или отклонить заявку. Без блокировки.
- **DISC-03:** Точная форма селектора (выпадающий список / поиск по коду+модели) и где
  именно он рендерится в `OperationModal` — на усмотрение планировщика, по образцу
  существующих автокомплитов/Select.
- **DISC-04:** `requesterName` иногда может быть логином/AD-учёткой, а не ФИО —
  редактируемое поле (D-04) это покрывает, отдельной логики не требуется.

### Deferred Ideas (OUT OF SCOPE)
- Массовая установка нескольких картриджей по одной заявке — отдельная фаза при
  появлении потребности.
- Создание/отправка на заправку картриджа прямо из потока заявки — вне scope.
- Изменение lifecycle/статусов самих заявок — вне scope.
- Изменение compatibility-матрицы моделей картриджей — вне scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

Фаза 12 не имеет собственных REQ-ID в `.planning/REQUIREMENTS.md` — таблица трассировки
сопоставляет REQ-05 и REQ-07 с Phase 6 (статус Complete), и эти ID уже считаются
выполненными на уровне той фазы. Работа Phase 12 — это закрытие функционального гэпа
поверх уже "выполненных" требований (UI-проводка, которая фактически не работала),
производное целиком из решений CONTEXT.md D-01..D-08. Ниже сопоставление с этими
решениями вместо формальных REQ-ID:

| ID | Description | Research Support |
|----|-------------|------------------|
| D-01/D-02 | Селектор картриджа: статус=На складе, заряд∈{Полный,Частичный}, модель=request.cartridgeModelId | `CartridgeFilter` (domain+DTO+SQL) гэп подтверждён; Pattern 1 даёт конкретный путь реализации (`installable_only` bool → `state_id IN (1,2)`) |
| D-03 | Submit вызывает `cartridges.transition({op:'install', cartridge_id,...})` без изменений контракта | Существующий `buildPayload()`/`CartridgeTransitionPayload::Install` контракт подтверждён как достаточный, см. Code Examples |
| D-04 | «Кому выдал» предзаполняется из `requesterName`, editable | `PersonAutocomplete` уже поддерживает controlled-value с предзаполнением; не требует нового компонента |
| D-05 | «Расположение» предзаполняется из локации принтера заявки, editable | `printer_location` гэп на `RequestDto` подтверждён; готовый JOIN-паттерн из `printer_options()` (Pattern 2) даёт прямой путь реализации |
| D-06 | `completedCartridgeId` записывается реальным id вместо `null` | Backend write-путь (`transition_in_tx` COALESCE) подтверждён как полностью готовый — нужна только фронтовая правка |
| D-07 | Установленный картридж отражается в истории заявки человекочитаемо | `notes_json`/`payload_json` паттерн подтверждён как точка расширения (Pitfall 3, Open Question 2) |
| D-08 | Старый cartridge-centric install не регрессирует | Существующий тест `install_changes_status_to_in_use` (cartridges_lifecycle.rs) подтверждён как regression-guard |

</phase_requirements>

## Summary

Фаза не требует новых библиотек, миграций схемы или новой архитектуры — это **проводка существующих контрактов**, которые уже были спроектированы с запасом на этот кейс, но не были до конца соединены. Все шесть исследовательских вопросов разрешились однозначно при чтении кода:

1. `CartridgeFilter` (domain + DTO + SQL) **не** умеет фильтровать по заряду (`state_id`) — это единственный настоящий backend-гэп фазы. Фильтр по модели (`model_id`) и статусу (`status_id`) уже есть и реюзается как есть.
2. `RequestDto` отдаёт `printer_name`, но **не** `printer_location`. Сходный паттерн (`devices.location_id LEFT JOIN locations`) уже реализован в Phase 11 для `RequestService::printer_options()` — тот же JOIN нужно добавить в `SELECT_REQUESTS` (или в отдельный point-read), скопировав готовый SQL-приём.
3. Запись `completed_cartridge_id` **уже полностью реализована** на бэкенде (`COALESCE(?4, completed_cartridge_id)` в `transition_in_tx`, колонка из миграции V024) — единственная проблема в том, что фронтенд хардкодит `linkedCartridgeId: null`. История заявок (REQ-07) рендерит `payload_json → {"notes": "..."}`; для отображения картриджа нужно расширить тот же JSON-конверт (`notes_json`) дополнительным полем-снапшотом, аналогично тому, как уже хранится `notes`.
4. Минимальная фронтенд-правка: `OperationModal` нужно научить принимать список картриджей и предзаполнение location/given_to_name пропами (вместо одного `preFillPrinterId`-хинта), `RequestDetail` — передавать реальные данные заявки и пробрасывать `id` выбранного картриджа в `linkedCartridgeId`. Готовый паттерн для нового селектора — `GroupedPrinterSelect.svelte` (Phase 11), который как раз принимает плоский список DTO и рендерит `<select>`/`<optgroup>`.
5. **Миграции не требуются.** `completed_cartridge_id` (V024) и `state_id` (V005) — обе колонки существуют. Фильтр по заряду — это только код (Rust WHERE-условие + DTO-поле), не схема.
6. RBAC уже полностью закрывает «Employee не должен ставить картридж»: `Action::MutateCartridges` (cartridge transition) и `Action::TransitionRequests` (complete) оба гейтятся на `Admin|Manager` в `authorize()`. Новый список-для-селектора, если реюзает `cartridges_list` (`Action::ReadData`), тоже автоматически недоступен Employee (Phase 10 закрыл `ReadData` для Employee). Никакой новой авторизационной логики не нужно — только убедиться, что новый/изменённый эндпоинт **не** случайно перегейтится на `Action::CreateRequest` (как у `request_printer_options`, который специально открыт Employee — здесь это будет ошибкой).

**Primary recommendation:** Расширить `CartridgeFilter` полем для набора допустимых `state_id` (заряд), расширить `RequestDto`/SELECT принтерной локацией по образцу `printer_options()`, передать реальный `cartridge_id` в `linkedCartridgeId` при complete, и обогатить `notes_json` снапшотом картриджа (код+модель) на момент завершения заявки — без единой миграции и без новых crates/npm-пакетов.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Фильтрация картриджей (статус+заряд+модель) | API / Backend (`CartridgeService.list` + `CartridgeRepository::list` SQL) | — | Существующий паттерн: вся фильтрация — backend SQL, фронт не плодит свой WHERE (явно зафиксировано в CONTEXT.md «Established Patterns») |
| Резолв локации принтера для заявки | API / Backend (`RequestService` SQL JOIN) | — | Та же причина, что и для `printer_options()` — локация это `devices.location_id → locations.name`, серверный join, не клиентский lookup |
| Авто-подстановка «Кому выдал»/«Расположение» в форме | Browser / Client (`OperationModal.svelte`) | — | Чистая UI-логика предзаполнения redактируемых полей из уже полученных от сервера данных — не нужен отдельный backend-вызов сверх того, что уже несёт `RequestDto` |
| Выбор картриджа → install transition | API / Backend (`CartridgeService.transition`) | Browser/Client (форма) | Бизнес-правило (status/version проверка) уже в сервисе; фронт только собирает payload |
| Связка install → request.complete (`completed_cartridge_id`) | API / Backend (`RequestService.transition`) | Browser/Client (передаёт id) | Write уже в `transition_in_tx`; фронту нужно лишь не отправлять `null` |
| Отображение установленного картриджа в истории заявки | API / Backend (audit `payload_json` снапшот) | Browser/Client (рендер строки) | История — серверный source of truth (audit_log); фронт просто парсит JSON, как уже делает для `notes` |
| RBAC (Employee не ставит картридж) | API / Backend (`authorize()`) | Browser/Client (`isSpecialist` UI-гейт) | Бэкенд — единственный источник истины; фронтовый гейт — UX-улучшение, не security-контроль (уже верно реализовано) |

## Standard Stack

Фаза не вводит новых библиотек. Весь стек — уже зафиксированный в CLAUDE.md и используемый в прошлых фазах: Rust/Tauri 2.x/axum/rusqlite/refinery (backend), Svelte 5 runes + tauri-specta-генерируемые биндинги (frontend). Никаких `cargo add` / `pnpm add` для этой фазы не требуется.

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| rusqlite | 0.39 (уже в Cargo.lock) | SQL-чтение/запись фильтра картриджей и локации принтера | Уже используется во всех repos слоя, не меняется |
| tauri-specta | текущая закреплённая (см. `Cargo.toml`) | Регенерация TS-биндингов при изменении `CartridgeFilter`/`RequestDto` | Единственный канал синхронизации Rust DTO ↔ TS-типов в проекте |
| Svelte 5 (runes) | 5.55+ | UI-правки `OperationModal.svelte`/`RequestDetail.svelte` | Уже стандарт проекта |

### Supporting
Нет новых supporting-библиотек для этой фазы.

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Backend-фильтр заряда в SQL (расширение `CartridgeFilter`) | Клиентский фильтр по `state_id` на уже полученном списке | CONTEXT.md явно запрещает плодить логику фильтрации на фронте («переиспользовать существующий фильтр на стороне backend, не плодить новый SQL») — отклонено |
| Снапшот картриджа в `payload_json` (audit) для истории заявки | Фронт делает доп. `cartridges.get(completedCartridgeId)` при рендере истории | Снапшот консистентен с уже существующим паттерном `notes` в том же JSON-конверте и не плодит N+1 запросов при показе списка истории; доп. read — лишний round-trip и риск показать неверные данные, если картридж позже изменили/удалили |

**Installation:** не требуется — все зависимости уже в `Cargo.lock`/`pnpm-lock.yaml`.

**Version verification:** новых пакетов нет, проверка реестра не требуется.

## Package Legitimacy Audit

**Не применимо.** Фаза не устанавливает внешние пакеты — только модифицирует существующий Rust/TS код в монорепе. Шаги Package Legitimacy Gate пропущены по причине отсутствия новых зависимостей.

## Architecture Patterns

### System Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│  RequestDetail.svelte (статус in_progress, type=cartridge_replace)  │
│                                                                       │
│   [Кнопка «Установить картридж»] ──opens──▶ OperationModal           │
│        │                                         │ op='install'      │
│        │ передаёт: request.cartridgeModelId,     │                  │
│        │   request.printerLocation (НОВОЕ поле), │                  │
│        │   request.requesterName                 │                  │
│        ▼                                         ▼                  │
│   cartridges.list({status=1, stateIds=[1,2],      Селектор картриджа │
│     modelId: request.cartridgeModelId})  ◀────────┘ (новый компонент)│
│        │                                                              │
│        ▼ возвращает отфильтрованный список                          │
│   [выбор конкретного экземпляра C-000042]                            │
│        │                                                              │
│        ▼ given_to_name/location уже предзаполнены, editable          │
│   cartridges.transition({op:'install', cartridge_id, ...})           │
│        │                                                              │
│        ▼ успех → id установленного картриджа известен фронту         │
│   requests.transition({op:'complete', linkedCartridgeId: <id>})      │
│        │                                                              │
│        ▼                                                              │
│   RequestService.transition (Rust)                                   │
│     ├─ request_repo.transition_in_tx(..., linked_cartridge_id, ...)  │
│     │     UPDATE requests SET completed_cartridge_id = COALESCE(...) │
│     ├─ notes_json: {"notes": ..., "cartridgeCode": ..., "cartridgeModel": ...} (РАСШИРЕНИЕ)
│     │     INSERT INTO audit_log (..., payload_json)                  │
│     └─ ws_tx.send(WsEvent::RequestStatusChanged)                     │
│        │                                                              │
│        ▼                                                              │
│   requests.getHistory(id) → парсит payload_json →                    │
│     рендерит "Выполнена; установлен C-000042 (HP CE285A); ..."       │
└─────────────────────────────────────────────────────────────────────┘

Параллельный (неизменный) путь:
┌──────────────────────────────────────────────┐
│ Меню картриджа «На складе» → «Установить»     │
│   OperationModal(op='install', cartridge=X)   │  ← cartridge передан как prop, не null
│   handleSubmit работает как раньше             │
└──────────────────────────────────────────────┘
```

### Recommended Project Structure

Никаких новых директорий — правки укладываются в существующую структуру:
```
crates/trackly-core/src/domain/cartridges.rs   # CartridgeFilter: + state_ids: Vec<i64> (или 2 bool-флага)
crates/trackly-app/src/dto/cartridge.rs        # CartridgeFilter DTO: зеркалит domain-поле
crates/trackly-app/src/dto/request.rs          # RequestDto: + printer_location: Option<String>
                                                # RequestHistoryEntryDto: без изменений (снапшот идёт через notes JSON parse на фронте, либо +cartridge_code/+cartridge_model поля — см. Open Questions)
crates/trackly-infra/src/repos/cartridges_sqlite.rs  # list(): WHERE state_id IN (...)
crates/trackly-infra/src/repos/requests_sqlite.rs    # SELECT_REQUESTS: + LEFT JOIN locations
crates/trackly-app/src/services/request_service.rs   # transition(): notes_json обогащение снапшотом картриджа
ui/src/features/cartridges/OperationModal.svelte      # + cartridge-селектор при открытии из заявки
ui/src/features/requests/RequestDetail.svelte         # передача request.printerLocation/requesterName/cartridgeModelId; linkedCartridgeId: <id>
ui/src/features/cartridges/api.ts                      # list() сигнатура: + stateIds в фильтре
ui/src/features/requests/api.ts                        # RequestHistoryEntry: + cartridgeCode/cartridgeModel (если выбран вариант с доп. полями)
ui/src/lib/components/<NewCartridgeSelect>.svelte      # новый компонент по образцу GroupedPrinterSelect.svelte
ui/src/bindings.ts / bindings-phase6.ts                 # регенерация tauri-specta после правок DTO
```

### Pattern 1: Backend-фильтр по набору значений (multi-value WHERE)
**What:** `CartridgeFilter` сейчас несёт только single-value опциональные поля (`Option<i64>`), которые транслируются в `?n IS NULL OR col = ?n`. Заряд требует **множества** допустимых значений (`{1, 2}`), это другая форма фильтра.
**When to use:** Когда фильтр — это `IN (...)`, а не равенство одному значению.
**Example:**
```rust
// crates/trackly-core/src/domain/cartridges.rs — расширение CartridgeFilter
pub struct CartridgeFilter {
    pub status_id: Option<i64>,
    pub kind_id: Option<i64>,
    pub model_id: Option<i64>,
    pub search: Option<String>,
    pub include_deleted: bool,
    /// Заряд (state_id) — пусто = без фильтра; непусто = WHERE state_id IN (...).
    /// Для D-01 вызывающая сторона передаёт vec![1, 2] (Полный, Частичный).
    pub state_ids: Vec<i64>,
}
```
```rust
// crates/trackly-infra/src/repos/cartridges_sqlite.rs — SQL: вариативный IN
// rusqlite не поддерживает IN с динамическим списком через позиционные ?N напрямую;
// либо построить SQL строку с нужным числом плейсхолдеров, либо (проще и безопаснее)
// захардкодить ровно 2 опциональных слота, так как набор фиксирован доменными
// правилами (Полный=1, Частичный=2) и не меняется извне:
//   AND (?N IS NULL OR c.state_id IN (?N, ?N+1))  -- если allow_partial=false, передать NULL,NULL
// ИЛИ явно — два bool-флага вместо Vec<i64>, что проще для rusqlite positional params:
pub struct CartridgeFilter {
    // ...
    pub charge_full: bool,     // включить state_id=1
    pub charge_partial: bool,  // включить state_id=2
}
// WHERE clause:
// AND ((?N = 0 AND ?N1 = 0) OR
//      (?N = 1 AND c.state_id = 1) OR
//      (?N1 = 1 AND c.state_id = 2) OR
//      (?N = 1 AND ?N1 = 1 AND c.state_id IN (1,2)))
```
**Рекомендация планировщику:** проще и безопаснее всего — добавить **новый bool-параметр на уровне сервиса/DTO** `installable_only: bool` (семантическое имя, не «state_ids»), который при `true` транслируется в SQL `AND c.state_id IN (1, 2)` как константный список (не параметризованный по значениям, раз набор фиксирован бизнес-правилом D-01 и не варьируется). Это устраняет сложность динамического `IN`, не теряя гибкости — обычный список картриджей (без request-контекста) продолжает работать с `installable_only: false`.

### Pattern 2: Реюз JOIN-приёма для резолва локации устройства
**What:** `RequestService::printer_options()` уже содержит готовый SQL `LEFT JOIN locations l ON d.location_id = l.id`.
**When to use:** Для добавления `printer_location` в `RequestDto`.
**Example:**
```sql
-- Source: crates/trackly-app/src/services/request_service.rs:241-249 (существующий код)
SELECT d.id, d.name, l.name AS location
FROM devices d
LEFT JOIN locations l ON d.location_id = l.id
WHERE d.type_id = (SELECT id FROM device_types WHERE name = 'Принтер')
  AND d.deleted_at_utc IS NULL

-- Адаптация для SELECT_REQUESTS (requests_sqlite.rs):
SELECT r.id, ..., d.name AS printer_name, dl.name AS printer_location, ...
FROM requests r
LEFT JOIN devices d ON d.id = r.printer_device_id
LEFT JOIN locations dl ON dl.id = d.location_id
...
```
**Источник:** `crates/trackly-app/src/services/request_service.rs` (метод `printer_options`, строки 224-258 — уже прочитан полностью в этой сессии).

### Pattern 3: Reusable cartridge-selector по образцу GroupedPrinterSelect
**What:** `GroupedPrinterSelect.svelte` (Phase 11) принимает плоский DTO-массив `{id, name, location}` пропом и сам группирует/рендерит `<select>` с `<optgroup>`. Тот же скелет подходит для нового картриджного селектора: `{id, code, modelLabel, stateLabel}` → опции с лейблом `"C-000042 — HP CE285A (Полный)"`.
**When to use:** Для DISC-03 (форма селектора оставлена на усмотрение планировщика) — самый дёшевый и консистентный с UI-языком проекта вариант.
**Example:**
```svelte
<!-- Source: ui/src/lib/components/GroupedPrinterSelect.svelte (паттерн, не копия) -->
<script lang="ts">
  interface Props {
    options: InstallableCartridgeOptionDto[]; // {id, code, modelLabel, stateLabel}
    value: string;
    onchange?: (_value: string) => void;
  }
  // группировка не нужна (нет естественной "локации" для картриджей в этом контексте),
  // флэт-список достаточен — проще, чем GroupedPrinterSelect.
</script>
<select {value} onchange={...}>
  <option value="">Выберите картридж</option>
  {#if options.length === 0}
    <option value="" disabled>Нет подходящих картриджей на складе</option>
  {/if}
  {#each options as o (o.id)}
    <option value={String(o.id)}>{o.code} — {o.modelLabel} ({o.stateLabel})</option>
  {/each}
</select>
```

### Anti-Patterns to Avoid
- **Клиентская фильтрация по заряду на полном списке картриджей:** прямо противоречит зафиксированному в CONTEXT.md паттерну («не плодить новый SQL» подразумевает обратное — расширять существующий backend SQL, а не обходить его клиентским фильтром).
- **Гейтить новый эндпоинт списка картриджей-для-установки на `Action::CreateRequest`:** это «открытый Employee»-гейт, специально созданный для формы создания заявки (D-PRN-01). Установка картриджа — операция специалиста; реюз `cartridges_list` под `Action::ReadData` (уже закрыт для Employee, Phase 10) — корректный выбор.
- **Добавление нового `Action`-варианта в RBAC enum:** не нужно — все нужные действия (`ReadData`, `MutateCartridges`, `TransitionRequests`) уже существуют и уже гейтят ровно то, что нужно.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Группировка/рендер select со списком опций | Свой `<select>` с ручной разметкой `<option>` с нуля | Скопировать скелет `GroupedPrinterSelect.svelte` (стили, caret-иконка, invalid/disabled state уже готовы) | Уже есть готовый, протестированный в проде компонент с нужной SCSS-структурой |
| Debounced автокомплит по коду/модели картриджа (если планировщик выберет поиск, а не статичный select) | Свой debounce/keyboard-nav | `PersonAutocomplete.svelte` как шаблон (200ms debounce, ArrowUp/Down/Enter/Escape, `onmousedown preventDefault`) | DISC-03 явно отсылает «по образцу существующих автокомплитов» |
| JSON-снапшот деталей операции в истории | Отдельная таблица "request_cartridge_snapshots" | Расширение существующего `payload_json` на audit_log (как уже сделано для `notes`) | Меньше схемы, консистентно с уже работающим паттерном; не требует миграции |

**Key insight:** Весь домен уже содержит готовые операционные кирпичи (фильтры, JOIN-приёмы, UI-компоненты) для соседних, почти идентичных задач (печать принтеров для заявки, выдача картриджа). Задача фазы — связать их, а не изобретать новые механизмы.

## Common Pitfalls

### Pitfall 1: Передача `null`/неправильного `cartridge` в `OperationModal` остаётся незамеченной из-за раннего return
**What goes wrong:** `handleSubmit()` молча делает `if (!cartridge || submitting) return;` — без UI-фидбэка если выбор не сделан, кнопка Submit просто не реагирует.
**Why it happens:** Старый код был написан под cartridge-centric вход, где `cartridge` всегда не-null с момента открытия модалки; новый request-centric вход начинает с `cartridge=null` до выбора в селекторе.
**How to avoid:** `canSubmit = $derived(!submitting && !!cartridge)` уже корректно блокирует кнопку — но нужно убедиться, что после добавления селектора `cartridge` реактивно обновляется (`$state`) при выборе из нового компонента, а не остаётся залипшим в `null` через стейл-замыкание.
**Warning signs:** Кнопка Submit остаётся disabled после явного выбора картриджа в новом селекторе — означает, что внутренний `cartridge`-state модалки не синхронизирован с выбором.

### Pitfall 2: Десериализация `RequestTransitionPayload::Complete` — camelCase ловушка (уже зафиксирована тестами)
**What goes wrong:** Enum-уровневый `#[serde(tag = "op", rename_all = "camelCase")]` переименовывает только значение тега, не поля каждого варианта — это исторический баг (09-AD-GAPS Defect 2).
**Why it happens:** `serde` rename_all на enum применяется к variant-именам, а не к их внутренним полям; для полей нужен **свой** `#[serde(rename_all = "camelCase")]` на каждом варианте.
**How to avoid:** Это уже исправлено и закреплено тестом `complete_deserializes_camel_case_wire_format` в `dto/request.rs` (wire_contract_tests модуль) — НЕ трогать структуру атрибутов варианта `Complete`, только добавлять данные внутрь существующего `linked_cartridge_id`.
**Warning signs:** Если планировщик решит добавить новое поле в `Complete`-вариант (не нужно для этой фазы — `linked_cartridge_id` уже есть) — обязательно прогнать существующий `wire_contract_tests` модуль, не расширять его вручную без перепроверки camelCase roundtrip.

### Pitfall 3: Снапшот картриджа в истории вместо live-джойна — десинхронизация при последующем редактировании картриджа
**What goes wrong:** Если выбрать вариант «снапшот в payload_json при complete», а потом картридж переименуют/перепривяжут к другой модели — история заявки покажет старые code/model, которые могут не совпадать с текущим состоянием картриджа.
**Why it happens:** Снапшот по определению не следует за последующими изменениями сущности.
**How to avoid:** Это сознательный и приемлемый трейдофф (этот же паттерн уже применяется в проекте — `act_items.condition_at_time` снапшот, согласно STATE.md). История должна показывать состояние **на момент события**, а не текущее — так и must быть. Документировать в комментарии к коду, чтобы не «исправили» это позже как «баг».
**Warning signs:** Жалоба «история показывает неправильную модель картриджа» — нужно сначала проверить, не изменилась ли сама модель/код картриджа после complete, прежде чем считать это багом.

### Pitfall 4: Фильтр по заряду должен работать «ИЛИ», не «И» — Полный ИЛИ Частичный, не оба одновременно у одного экземпляра
**What goes wrong:** Легко перепутать семантику и написать `state_id = 1 AND state_id = 2` (невозможное условие) вместо `state_id IN (1, 2)`.
**Why it happens:** D-01 формулировка «Полный(1) И Частичный(2)» на русском читается двусмысленно (в значении «оба значения допустимы», т.е. множество), но синтаксически наводит на AND.
**How to avoid:** SQL-условие — `state_id IN (1, 2)`, эквивалентно логическому ИЛИ на уровне предиката одной строки. Уже явно переформулировано в CONTEXT.md как «ИЛИ Полный(1) ИЛИ Частичный(2)» — следовать этой формулировке, не первоначальной.
**Warning signs:** Тест на пустой список картриджей при наличии и Полных, и Частичных экземпляров на складе — верный признак перепутанного AND/IN.

### Pitfall 5: `printer_location` = NULL у заявок типа `free_form`/`ad_register` или у `cartridge_replace` без принтера
**What goes wrong:** Не все заявки несут `printer_device_id` — NULL-джойн должен корректно давать `printer_location: None`, а не паниковать/ошибаться.
**Why it happens:** `LEFT JOIN devices d ON d.id = r.printer_device_id` — если `printer_device_id IS NULL`, джойн естественно не матчит ни одной строки `devices`, что rusqlite корректно мапит в `NULL` для `d.name`/`dl.name` — это уже паттерн, отработанный в текущем `SELECT_REQUESTS` для `printer_name`.
**How to avoid:** Просто следовать тому же `Option<String>`-маппингу, что уже используется для `printer_name` — никакой дополнительной защиты не требуется, NULL-safety уже встроена в существующий LEFT JOIN.
**Warning signs:** Любой `unwrap()`/`expect()` на `printer_location` в новом коде — сигнал ошибки; поле обязано остаться `Option<String>`.

## Code Examples

### Существующий install-payload (фронт) — образец для нового селектора
```typescript
// Source: ui/src/features/cartridges/OperationModal.svelte (buildPayload(), уже прочитан)
function buildPayload(): CartridgeTransitionPayload | null {
  if (op === 'install' && cartridge) {
    return {
      op: 'install',
      cartridge_id: cartridge.id,
      version: cartridge.version,
      date_utc: isoToUnix(dateIso),
      given_by_name: givenByName.trim(),
      given_to_name: givenToName.trim(),
      location: location.trim(),
    };
  }
  // ...
}
```

### Текущий (дефектный) проброс из RequestDetail — точка правки D-06
```typescript
// Source: ui/src/features/requests/RequestDetail.svelte (handleInstallSuccess(), уже прочитан)
async function handleInstallSuccess() {
  if (!request) return;
  operationModalOpen = false;
  try {
    await requests.transition({
      op: 'complete',
      requestId: request.id,
      version: request.version,
      notes: null,
      linkedCartridgeId: null, // ← должен стать id установленного картриджа
    });
    pushToast('success', 'Заявка выполнена');
    onTransition();
  } catch (e: unknown) { /* ... */ }
}
```

### Существующий wire-контракт для `linkedCartridgeId` (бэкенд, уже готов)
```rust
// Source: crates/trackly-app/src/dto/request.rs (RequestTransitionPayload::Complete)
#[serde(rename_all = "camelCase")]
Complete {
    #[specta(type = i32)]
    request_id: i64,
    #[specta(type = i32)]
    version: i64,
    notes: Option<String>,
    /// Links a cartridge installation (REQ-05 / D-Req-CART07-01).
    linked_cartridge_id: Option<i32>,
},
```

### Существующий write-путь для `completed_cartridge_id` (бэкенд, уже готов)
```sql
-- Source: crates/trackly-infra/src/repos/requests_sqlite.rs (transition_in_tx, уже прочитан)
UPDATE requests
   SET status = ?1, resolution_notes = COALESCE(?2, resolution_notes),
       assigned_to_user_id = COALESCE(?3, assigned_to_user_id),
       completed_cartridge_id = COALESCE(?4, completed_cartridge_id),
       updated_at_utc = ?5, version = version + 1
 WHERE id = ?6 AND version = ?7 AND deleted_at_utc IS NULL
```

### Существующий placeholder-тест, который нужно реализовать (Wave 0 gap)
```rust
// Source: crates/trackly-app/tests/phase06_stubs.rs:575-578 (уже прочитан, никогда не реализован)
/// REQ-05: Complete{linked_cartridge_id} записывает completed_cartridge_id
#[test]
#[ignore]
fn test_req_cart_link() {}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|---------------|--------|
| `OperationModal` открывается из заявки с `cartridge={null}` (no-op) | `OperationModal` получает выбранный картридж из нового селектора внутри модалки | Эта фаза | Делает request-centric install реально работающим |
| `linkedCartridgeId: null` всегда | Реальный id выбранного картриджа | Эта фаза | Замыкает REQ-05 связь заявка↔картридж |
| История заявки показывает только `notes`/`actorName` | + человекочитаемая информация об установленном картридже (код+модель) | Эта фаза | Закрывает REQ-07 для этого конкретного кейса |

**Deprecated/outdated:** ничего не устаревает — старый cartridge-centric вход (D-08) остаётся неизменным и продолжает работать как раньше.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Лучший способ реализовать D-01 (фильтр по заряду) — bool-флаг `installable_only`/константный `IN (1,2)`, а не общий `Vec<i64>` параметр | Architecture Patterns, Pattern 1 | Низкий — оба подхода реализуют один и тот же бизнес-результат; если планировщик выберет `Vec<i64>`, придётся решить проблему динамического числа `?N`-плейсхолдеров в rusqlite (решаемо, но больше кода) |
| A2 | Снапшот картриджа в `payload_json` (audit) — предпочтительный способ показать картридж в истории заявки (D-07), а не отдельный SQL JOIN в `get_history()` или доп. фронтовый `cartridges.get()` | Don't Hand-Roll, Pitfall 3 | Средний — если планировщик предпочтёт live-JOIN (показывать текущие code/model картриджа, а не снапшот на момент complete), потребуется другая реализация (JOIN в `get_history()` SQL вместо JSON-расширения); seams для теста будут отличаться |
| A3 | Новый эндпоинт списка «картриджей для установки из заявки» можно реализовать как **расширение** существующего `cartridges_list`/`CartridgeFilter`, а не отдельный новый Tauri-command | Architecture Patterns, Pattern 1; Don't Hand-Roll | Низкий — оба пути технически работают и одинаково RBAC-безопасны (оба гейтятся `Action::ReadData`); расширение существующего эндпоинта экономит код, но если планировщик предпочтёт отдельный узкий DTO (по аналогии с `RequestPrinterOptionDto`/BOLA-минимизацией), потребуется новый command + DTO |

## Open Questions

1. **Как именно технически реализовать множественный заряд-фильтр в rusqlite (`Vec<i64>` IN vs. два bool-флага)?**
   - What we know: набор `{1, 2}` фиксирован бизнес-правилом D-01 и не варьируется извне (UI всегда просит «оба» или «не фильтровать вовсе» — DISC-01 описывает только кейс «без фильтра по модели», не «без фильтра по заряду»; заряд-фильтр всегда активен при открытии из заявки).
   - What's unclear: нужен ли когда-либо частичный набор (только Полный ИЛИ только Частичный) где-то ещё в продукте — если нет, константный `IN (1,2)` через единственный bool достаточен и проще всего в rusqlite.
   - Recommendation: планировщику — выбрать single bool `installable_only: bool` (раскрывается в `state_id IN (1,2)` константно), если нет другого требования на гибкую комбинацию зарядов где-то ещё в проекте.

2. **D-07 (история заявки): снапшот в JSON vs. живой JOIN vs. фронтовый доп.-запрос?**
   - What we know: текущий паттерн (`notes` в `payload_json`) — это снапшот-на-момент-события; есть прецедент денормализованных снапшотов в проекте (`act_items.condition_at_time`).
   - What's unclear: нет явного решения в CONTEXT.md, какой из трёх вариантов выбрать — только требование «человекочитаемо (код+модель)».
   - Recommendation: снапшот в `payload_json` (Assumption A2) — наименее затратный, консистентный с существующим кодом, не создаёт N+1 запросов при показе списка истории. Планировщик должен явно зафиксировать это решение в плане (не оставлять имплементацию неопределённой), так как от выбора зависит, какие файлы трогать (`request_service.rs::transition()` vs. `requests_sqlite.rs::get_history()` vs. фронтовый `RequestDetail.svelte`).

3. **Нужен ли отдельный узкий DTO для списка «картриджей для установки», аналогично `RequestPrinterOptionDto` (BOLA-минимизация), или достаточно существующего полного `CartridgeDto`?**
   - What we know: `RequestPrinterOptionDto` был специально создан в Phase 11 как минимальный {id, name, location} DTO, чтобы не светить SNMP/community/IP/serial поля Employee-доступному эндпоинту.
   - What's unclear: установка картриджа — admin/manager-only операция (`MutateCartridges`/`ReadData` оба закрыты для Employee), так что мотивация для узкого DTO (защита от Employee) здесь не применяется тем же образом, как для `request_printer_options`. Полный `CartridgeDto` для Admin/Manager — не over-exposure.
   - Recommendation: реюзать существующий `CartridgeDto` + существующий `cartridges_list` эндпоинт (расширенный фильтром заряда) — отдельный узкий DTO избыточен здесь, в отличие от Phase 11 кейса.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| cargo/rustc | Backend сборка и тесты | ✓ | 1.92.0 | — |
| pnpm | Frontend сборка | ✓ | 10.17.1 | — |
| node | Vite dev server | ✓ | v22.18.0 | — |

Все зависимости уже присутствуют в dev-окружении; фаза не добавляет новых внешних требований (нет SNMP/AD/сетевых зависимостей в этой работе — чисто SQL + UI).

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | `cargo test` (tokio integration tests, `crates/trackly-app/tests/*.rs`) + `pnpm` (`svelte-check`, без e2e UI-фреймворка в проекте) |
| Config file | `Cargo.toml` (workspace), нет отдельного test-config файла |
| Quick run command | `cargo test --test phase06_stubs test_req_cart_link -- --ignored` (после реализации — снять `#[ignore]`) |
| Full suite command | `cargo test` (⚠️ ОДИН процесс за раз — project memory: "No concurrent cargo test", контеншн на `target/`-lock) |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| D-01/D-02 | `CartridgeService.list` с заряд-фильтром возвращает только status=1 AND state_id∈{1,2} AND (model_id совпадает либо все, если null) | integration | `cargo test --test cartridges_lifecycle installable_filter` | ❌ Wave 0 (новый тест-файл или новая fn в `cartridges_lifecycle.rs`) |
| D-06 | `RequestService.transition(Complete{linked_cartridge_id: Some(id)})` → `requests.completed_cartridge_id == id` | integration | `cargo test --test phase06_stubs test_req_cart_link -- --ignored` (снять ignore) | ✅ существует как `#[ignore]`-плейсхолдер, требует реализации |
| D-07 | История заявки (`get_history`) после Complete с картриджем содержит человекочитаемый код+модель | integration | `cargo test --test phase06_stubs test_req_cart_link` (расширить тот же тест либо новый `test_req_cart_history_shows_cartridge`) | ❌ Wave 0 |
| D-05 | `RequestDto.printer_location` корректно резолвится (NULL-safe для заявок без принтера/без локации устройства) | integration | `cargo test --test request_printer_options` (расширить существующий файл по аналогии) или новый тест в `phase06_stubs.rs` | ❌ Wave 0 (новый assert в существующем файле) |
| D-08 (regression) | Старый cartridge-centric install (`cartridge` передан напрямую) продолжает работать без изменений | integration | `cargo test --test cartridges_lifecycle install_changes_status_to_in_use` | ✅ существует, должен остаться зелёным без изменений |
| RBAC (Employee не ставит картридж) | `cartridges_transition`/`requests_transition` остаются недоступны Employee | integration | `cargo test --test role_endpoint_matrix` (существующий файл — проверить, покрывает ли он `cartridges_transition`/`requests_transition`; если нет, добавить строку в матрицу) | ⚠️ файл существует (1110 строк), нужно подтвердить точечное покрытие этих двух команд при планировании — не прочитан полностью в этой сессии |

### Sampling Rate
- **Per task commit:** `cargo test --test phase06_stubs` (целевой файл с `test_req_cart_link`) + `cargo test --test cartridges_lifecycle` (regression на D-08)
- **Per wave merge:** `cargo test` (полный набор, один процесс — см. project memory о конкурентном запуске)
- **Phase gate:** Full suite green перед `/gsd-verify-work`; дополнительно `pnpm --dir ui build` перед ручной/UAT-проверкой в браузере (project memory: dev browser testing требует свежей `ui/dist` сборки — `cargo tauri dev` не HMR-ит LAN-режим)

### Wave 0 Gaps
- [ ] Реализовать тело `test_req_cart_link` (`crates/trackly-app/tests/phase06_stubs.rs:575-578`) — снять `#[ignore]`, покрыть D-06
- [ ] Новый/расширенный тест на заряд-фильтр в `cartridges_lifecycle.rs` (или новый файл `cartridges_filter_charge.rs`) — покрыть D-01/D-02, включая DISC-01 (null model_id → без фильтра модели) и DISC-02 (пустой результат → корректная пустая выборка, не ошибка)
- [ ] Новый assert на `printer_location` NULL-safety в `RequestDto` — расширить `request_printer_options.rs` или `phase06_stubs.rs::test_request_lifecycle`
- [ ] Подтвердить (при планировании) покрытие `role_endpoint_matrix.rs` для `cartridges_transition`/`requests_transition` под Employee-ролью — если отсутствует, добавить строку в матрицу

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | нет (не меняется в этой фазе) | — |
| V3 Session Management | нет | — |
| V4 Access Control | да | Существующий `authorize(caller, &Action::MutateCartridges | TransitionRequests | ReadData)` — переиспользуется без изменений |
| V5 Input Validation | да | Существующая server-side `validate_from_status()` (cartridge status/version проверка) + `version`-based optimistic locking на обеих сущностях (cartridge и request) |
| V6 Cryptography | нет | — |

### Known Threat Patterns for этого стека

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| BOLA — Employee получает доступ к списку картриджей-для-установки через реюз `cartridges_list` | Elevation of Privilege | Уже закрыт: `cartridges_list` гейтится `Action::ReadData`, закрытым для Employee с Phase 10 — не менять этот гейт при добавлении заряд-фильтра |
| BFLA — новый/расширенный эндпоинт случайно гейтится на `Action::CreateRequest` (открытый для Employee) вместо `Action::ReadData`/`MutateCartridges` | Elevation of Privilege | Явно проверить при имплементации — это единственный реальный security-риск этой фазы (по аналогии с тем, как `request_printer_options` намеренно открыт Employee; новый картридж-эндпоинт должен использовать ДРУГОЙ, закрытый гейт) |
| TOCTOU между выбором картриджа в UI и фактическим install-transition (картридж может быть установлен кем-то другим между открытием селектора и сабмитом) | Tampering | Уже покрыто существующим optimistic-locking (`version` mismatch → `OptimisticLockMismatch` error) — install-операция атомарно проверяет `current_status_id`/`version` в транзакции, ничего нового добавлять не нужно |

## Sources

### Primary (HIGH confidence — прочитан исходный код в этой сессии)
- `crates/trackly-core/src/domain/cartridges.rs` — `CartridgeFilter`, `CartridgeTransitionOp::Install`, `validate_from_status()`
- `crates/trackly-core/src/ports/cartridges.rs` — `CartridgeRepository::list`
- `crates/trackly-app/src/services/cartridge_service.rs` — `CartridgeService.list/transition`
- `crates/trackly-app/src/dto/cartridge.rs` — `CartridgeDto`, `CartridgeFilter` DTO, `CartridgeTransitionPayload`
- `crates/trackly-infra/src/repos/cartridges_sqlite.rs` — `list()` SQL (WHERE-clause без state_id)
- `crates/trackly-app/src/dto/request.rs` — `RequestDto`, `RequestTransitionPayload::Complete`, `RequestHistoryEntryDto`, wire_contract_tests
- `crates/trackly-app/src/services/request_service.rs` — `transition()`, `printer_options()`, `get_history()`, `notes_json` construction
- `crates/trackly-infra/src/repos/requests_sqlite.rs` — `SELECT_REQUESTS`, `transition_in_tx()`, `get_history()` SQL
- `crates/trackly-core/src/auth.rs` — `Action` enum, `authorize()` матрица
- `crates/trackly-app/src/tauri_cmds/requests.rs` — `build_request_printer_options`, тонкие Tauri wrappers
- `ui/src/features/cartridges/OperationModal.svelte` — install-флоу, `buildPayload`, `canSubmit`
- `ui/src/features/requests/RequestDetail.svelte` — `handleInstallSuccess`, role-гейт `isSpecialist`
- `ui/src/features/cartridges/api.ts`, `ui/src/features/requests/api.ts` — фронт-клиенты
- `ui/src/lib/components/GroupedPrinterSelect.svelte`, `PersonAutocomplete.svelte`, `LocationAutocomplete.svelte`, `Select.svelte` — переиспользуемые UI-паттерны
- `migrations/V006__requests.sql`, `V024__request_categories.sql`, `V025__cartridge_printer_link.sql` — подтверждение существующей схемы (никаких изменений не требуется)
- `crates/trackly-app/tests/phase06_stubs.rs`, `cartridges_lifecycle.rs` — существующее тестовое покрытие и Wave 0 gap (`test_req_cart_link`)
- `.planning/phases/12-cartridge-request-interconnection/12-CONTEXT.md` — авторитетные пользовательские решения D-01..D-08, DISC-01..04
- `.planning/REQUIREMENTS.md`, `.planning/STATE.md` — подтверждение отсутствия новых REQ-ID для Phase 12, прецедент `request_printer_options` минимального DTO

### Secondary (MEDIUM confidence)
Нет — все находки подтверждены прямым чтением кода проекта в этой сессии.

### Tertiary (LOW confidence)
Нет внешних WebSearch-источников использовано — фаза целиком внутренняя, не требует исследования внешних библиотек.

## Metadata

**Confidence breakdown:**
- Standard Stack: HIGH — нет новых зависимостей, весь стек зафиксирован в CLAUDE.md и неизменен
- Architecture: HIGH — все архитектурные решения выводятся прямым чтением существующего кода (готовые паттерны `printer_options`, `GroupedPrinterSelect`, `notes_json`)
- Pitfalls: HIGH — все pitfalls обоснованы либо существующими code-comments/тестами (camelCase wire-bug, optimistic locking), либо прямым анализом diff-поверхности

**Research date:** 2026-06-22
**Valid until:** Бессрочно в рамках текущей версии стека (внутренняя доработка, не зависит от внешних релизов библиотек) — переисследовать только если CONTEXT.md решения D-01..D-08 изменятся.
