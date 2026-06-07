# Phase 4: Картриджи — Research

**Researched:** 2026-06-07
**Domain:** Управление жизненным циклом картриджей и фотобарабанов — CRUD моделей и экземпляров, lifecycle-операции с audit_log, FTS-поиск, switch-bar, баннер низкого остатка. Расширение существующей кодовой базы Phase 1–3.
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**D-Scope-01:** Раздел охватывает картриджи И фотобарабаны. Единый раздел, единый lifecycle, единая схема. Различие — атрибут `kind` модели (Картридж / Фотобарабан).

**D-Model-Fields-01:** `cartridge_models.brand` + `cartridge_models.model` — два TEXT-поля. UNIQUE(brand, model) среди живых строк. Отображение «{brand} {model}». Отдельного поля `name` НЕТ.

**D-Model-Kind-01:** lookup-таблица `cartridge_kinds` (1=Картридж, 2=Фотобарабан) + `cartridge_models.kind_id INTEGER NOT NULL DEFAULT 1` в V016.

**D-Model-Color-01:** `cartridge_models.color TEXT` в V016. Фиксированный набор: Чёрный/Голубой/Пурпурный/Жёлтый/Светло-голубой/Светло-пурпурный. Поле скрыто когда kind=Фотобарабан.

**D-Model-NoCompatType-01:** «Оригинальный/Совместимый» НЕ хранить.

**D-Model-Compat-01:** Junction `cartridge_model_compatibility(printer_brand, printer_model)` уже в V005. Источник автокомплита сейчас — DISTINCT из той же таблицы. Phase 6 расширит сигнатуру.

**D-Nav-01:** Один пункт сайдбара «Картриджи» (route `/cartridges`), два таба внутри: «Картриджи» (по умолчанию) / «Модели».

**D-Detail-01:** master-detail для экземпляров по паттерну `ActsMasterDetail.svelte` + `ActDetail.svelte`. История из `audit_log` в карточке.

**D-Filters-01:** Switch-bar Все/На складе/В работе/На заправке/Списано + фильтр по типу + по модели + FTS-поиск.

**D-Op-Modal-01:** Единая параметризованная модалка операций.

**D-Op-Transitions-01:** Контекстное меню по статусу — строго по CONTEXT.md §D-Op-Transitions-01.

**D-Op-Fields-01:** Поля операций и дефолты заряда строго по CONTEXT.md §D-Op-Fields-01.

**D-Op-Location-01:** «на складе» = `locations.kind = 'warehouse'`. Сейчас оба автокомплита показывают ВСЕ локации (kind массово не проставлен). `cartridges.location` — freeform TEXT, round-trip INSERT OR IGNORE в `locations`.

**D-Code-01:** Авто-код `C-NNNNNN` из `cartridge_seq`. Формат: «C-» + zero-padded 6 цифр.

**D-Code-Override-01:** Custom-код — counter НЕ инкрементируется. При коллизии авто-кода — инкремент в той же tx. Custom-коллизия → `AppError::Conflict`.

**D-History-01:** Операции пишут `audit_log`. Карточка рендерит хронологию. Префикс `custom:` для не-CRUD операций.

**D-LowStock-01:** `app_settings(key TEXT PRIMARY KEY, value TEXT)` в V016. Seed `low_stock_threshold = '2'`.

**D-LowStock-02:** count(status='На складе' AND state='Полный') < threshold → «низкий остаток». Команда `cartridges_low_stock() -> Vec<{model, count, threshold}>`.

**D-LowStock-03:** Баннер в разделе (Phase 4). Дашборд — Phase 7.

**D-Search-01:** `cartridges_fts` уже в V012 (code, location, holder_name). Триггеры синхронизации отсутствуют — добавить в V016. Поиск по модели — через JOIN `cartridge_models`.

### Claude's Discretion

- Сигнатура `cartridges_transition` vs отдельные команды (install/return/to_refill/from_refill/write_off).
- `kind` как lookup-таблица (рекомендуется) vs CHECK-enum.
- Удаление модели при наличии живых экземпляров: запрет с понятной ошибкой (рекомендуется).
- `INSERT OR IGNORE` новых локаций в `locations` при вводе в операциях.
- Конкретные `audit_log` action-коды.
- Структура hexagonal-слоёв и feature-папки — паттерн как acts/devices.
- Точный состав миграции V016.

### Deferred Ideas (OUT OF SCOPE)

- Управление `locations.kind` через Settings UI → Phase 7 (SET).
- SET-04 UI-редактор порога → Phase 7.
- Дашборд-виджет картриджей + «Динамика расхода» → Phase 7.
- Связь установки с конкретным устройством-принтером (FK) + REQ-05 → Phase 6.
- Автокомплит совместимых принтеров из реальных принтеров БД → Phase 6.
- RBAC/авторизация → Phase 5 (`audit_log.user_id` = NULL в Phase 4).
- Server-mode bind axum-хендлеров → Phase 5/8 (router строится, не bind'ится).
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| CART-01 | CRUD моделей картриджей: Название(=brand+model), Цвет, Примечание, Совместимые принтеры | D-Model-Fields-01, D-Model-Kind-01, D-Model-Color-01; аналог DeviceService CRUD паттерна |
| CART-02 | Совместимые принтеры — массив пар brand+model с автокомплитом ранее введённых | D-Model-Compat-01; junction V005 `cartridge_model_compatibility`; DISTINCT-запрос |
| CART-03 | CRUD экземпляров: Код, Модель, Состояние заряда, Расположение, Примечания | V005 `cartridges` таблица уже существует; сервис создаётся по образцу DeviceService |
| CART-04 | Авто-код C-000001 потокобезопасно; пользовательский override | D-Code-01/D-Code-Override-01; `cartridge_seq` counter уже в V009; `increment_counter_in_tx` образец в acts_sqlite.rs |
| CART-05 | Switch-bar по статусу со счётчиками | D-Filters-01; `cartridge_statuses` lookup уже в V001; паттерн DeviceFilters.svelte |
| CART-06 | Контекстные действия по статусу | D-Op-Transitions-01; паттерн DeviceContextMenu.svelte |
| CART-07 | Установка в принтер: Дата, Кто выдал, Кому выдал, Расположение | D-Op-Fields-01; поля от PersonAutocomplete + LocationAutocomplete + DatePicker |
| CART-08 | Возврат на склад: Состояние заряда (def=Пустой), Расположение, Примечания | D-Op-Fields-01 |
| CART-09 | Передача/возврат с заправки | D-Op-Fields-01; аналог CART-07/08 |
| CART-10 | История перемещений из audit_log в карточке экземпляра | D-History-01; `SqliteAuditLogRepository::insert` паттерн |
| CART-11 | FTS-поиск по коду, модели, расположению | D-Search-01; `cartridges_fts` V012 + триггеры V016; LIKE+FTS UNION CTE паттерн как в acts_sqlite.rs |
| CART-12 | Баннер «низкий остаток» | D-LowStock-01/02/03; `app_settings` таблица V016 |
</phase_requirements>

---

## Summary

Phase 4 — это стандартный вертикальный срез поверх уже отлаженной архитектуры Phase 1–3. Кодовая база предоставляет все необходимые паттерны: счётчик авто-нумерации (`increment_counter_in_tx`), audit_log (`SqliteAuditLogRepository::insert`), master-detail UI (`ActsMasterDetail`), контекстное меню (`DeviceContextMenu`), FTS-поиск через LIKE+FTS UNION CTE, shared-компоненты (`PersonAutocomplete`, `LocationAutocomplete`, `DatePicker`, `Modal`). Основная задача планировщика — структурировать волны в том же порядке, в каком это делалось для Phase 3: DB (V016) → Core domain/ports → Infra repo → App service → Tauri commands → HTTP router (строится, не bind'ится) → UI.

Ключевые сложности по убыванию риска:
1. **FTS-триггеры для `cartridges_fts`** — V012 создал таблицу, но триггеры синхронизации отсутствуют. V016 должен добавить три триггера (ai/ad/au) по образцу V013.
2. **Lifecycle-переходы как единая `cartridges_transition` команда** vs отдельные команды. Выбор влияет на сигнатуру specta bindings и сложность frontend.
3. **Counter-коллизия** — при авто-коде нужен цикл `try_increment → check UNIQUE → retry` внутри единой writer-транзакции.
4. **`app_settings` таблица** — новая, не существует. V016 создаёт и seed'ит `low_stock_threshold`.

**Primary recommendation:** Следуйте 5-волновой структуре Phase 3: V016 (Wave 0) → Core + Infra (Wave 1) → Service + Commands + HTTP (Wave 2) → UI основной (Wave 3) → UI lifecycle + search (Wave 4) → Low stock + FTS smoke (Wave 5). Не вводить новых зависимостей — всё, что нужно, уже в Cargo.toml.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Авто-код C-NNNNNN | API / Backend (writer_worker) | — | Атомарность требует single-writer tx; клиент не участвует |
| CRUD моделей и экземпляров | API / Backend (Service + Repo) | Frontend (UI) | Бизнес-логика (validation, soft-delete, version) в сервисе |
| Lifecycle-переходы (install/return/refill) | API / Backend (Service) | — | Атомарная мутация: статус + аудит в одной writer-tx |
| FTS-поиск | API / Backend (Repo) | — | FTS5 запрос в SQLite; клиент передаёт только строку |
| Switch-bar / фильтры | Frontend (Svelte) | API (counts query) | UI хранит состояние фильтра; счётчики идут отдельным запросом |
| История перемещений | API / Backend (Repo reads audit_log) | Frontend (render) | Запрос в audit_log по entity_id; UI рендерит хронологически |
| Баннер низкого остатка | API / Backend (query) + Frontend (баннер) | — | Логика подсчёта в service; отображение — в CartridgesPage |
| Матрица совместимости | API / Backend (junction CRUD) | Frontend (editor) | Junction CRUD в repo; UI — добавляемый список пар с автокомплитом |
| LocationAutocomplete round-trip | API / Backend (INSERT OR IGNORE locations) | Frontend | `locations` единый справочник; round-trip при операции |

---

## Standard Stack

Фаза 4 не добавляет новых зависимостей. Всё необходимое уже в `Cargo.toml` и `package.json`.

### Core (already in Cargo.toml) [VERIFIED: codebase grep]

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `rusqlite` | `0.38` | SQLite write/read | Единственный writer паттерн, bundled feature |
| `refinery` | `0.9` | Миграции | embed_migrations!, runner в `migrations.rs` |
| `tokio` | `1.x` | Async runtime | WriterHandle::execute + axum |
| `axum` | `0.8.x` | HTTP router (строится, не bind'ится) | Tower ecosystem, Phase 5 bind'ит |
| `serde` / `serde_json` | `1.x` | DTO serialization + audit_log JSON | snake_case JSON на всех транспортах |
| `specta` / `tauri-specta` | `2.x rc.21` | TypeScript binding generation | collect_commands! pattern |
| `thiserror` | `1.x` | AppError domain errors | trackly-core не имеет I/O зависимостей |
| `tracing` | `0.1.x` | Structured logging | Уже используется во всех сервисах |

### Frontend (already in package.json) [VERIFIED: codebase grep]

| Library | Purpose |
|---------|---------|
| Svelte 5 + Vite 6 | UI framework, runes |
| `$lib/components/Modal.svelte` | Диалоги операций |
| `$lib/components/DatePicker.svelte` | Дата в операциях |
| `$lib/components/PersonAutocomplete.svelte` | «Кто выдал» / «Кому выдал» |
| `$lib/components/LocationAutocomplete.svelte` | Расположение |
| `$lib/components/Input, Select, Textarea, Button, Badge` | Форм-элементы |
| `$lib/components/Toast/ToastHost.svelte` | Уведомления об операциях |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Единая `cartridges_transition(op, fields)` | Отдельные команды install/return/refill/writeoff | Единая команда — меньше specta bindings, backend-friendly, но TS-тип `op` нужно дискриминировать по union; отдельные команды — типобезопаснее на TS-стороне. Планировщик выбирает |
| lookup `cartridge_kinds` | CHECK constraint `kind IN (...)` | Lookup — единообразно с `cartridge_statuses`/`cartridge_states` (V001), расширяемо без миграции. Рекомендуется |

**Installation:** Нет новых зависимостей для установки.

---

## Package Legitimacy Audit

Новые внешние пакеты не добавляются. Фаза использует исключительно уже установленные зависимости из Phase 1–3.

**Packages removed due to slopcheck [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

---

## Architecture Patterns

### System Architecture Diagram

```
CartridgesPage.svelte
  ├── CartridgesSearchAndTabs      — поиск + switch-bar статусов + фильтры типа/модели
  ├── LowStockBanner               — GET cartridges_low_stock → баннер
  ├── [tab: Картриджи]
  │   └── CartridgesMasterDetail
  │       ├── [master] CartridgesList
  │       │   └── CartridgeListRow × N  ← context: CartridgeContextMenu (по статусу)
  │       └── [detail] CartridgeDetail
  │           ├── поля экземпляра (код, модель, статус, состояние, расположение)
  │           └── OperationHistory (audit_log WHERE entity='cartridge' AND entity_id=X)
  └── [tab: Модели]
      └── ModelsList
          ├── ModelListRow × N ← context: ModelContextMenu (edit/delete)
          └── ModelFormModal (CRUD + CompatibilityEditor)

UI → apiCall (Tauri invoke | HTTP POST) → CartridgeService
       ↓
CartridgeService (trackly-app/services/)
  ├── reads → ReaderPool.acquire() → SqliteCartridgeRepository.list/get/search/counts/low_stock/suggest_*
  └── writes → WriterHandle.execute(|conn| { tx = conn.transaction(); ... })
       ├── cartridges_create: increment_counter_in_tx("cartridge_seq") → INSERT cartridges → INSERT audit_log
       ├── cartridges_transition: UPDATE cartridges SET status_id=X, state_id=Y ... → INSERT audit_log
       ├── models_create/update: INSERT/UPDATE cartridge_models + DELETE/INSERT cartridge_model_compatibility
       └── app_settings read → low_stock query
```

### Recommended Project Structure

```
crates/
├── trackly-core/src/
│   ├── domain/cartridges.rs        # CartridgeRow, CartridgeModelRow, CartridgeNew, CartridgeTransitionOp
│   └── ports/cartridges.rs         # CartridgeRepository trait (associated Conn type, как acts.rs)
├── trackly-infra/src/repos/
│   └── cartridges_sqlite.rs        # SqliteCartridgeRepository, *_in_tx helpers
└── trackly-app/src/
    ├── dto/cartridge.rs             # CartridgeDto, CartridgeModelDto, CartridgeFilter, TransitionPayload
    ├── services/cartridge_service.rs
    ├── tauri_cmds/cartridges.rs
    └── http/cartridges.rs

migrations/
└── V016__cartridges_kind_color_settings.sql

ui/src/features/cartridges/
├── CartridgesPage.svelte
├── CartridgesSearchAndTabs.svelte
├── CartridgesMasterDetail.svelte
├── CartridgeDetail.svelte
├── CartridgesList.svelte
├── CartridgeListRow.svelte
├── CartridgeContextMenu.svelte      # по образцу DeviceContextMenu.svelte
├── CartridgeFilters.svelte          # switch-bar + тип + модель, по DeviceFilters.svelte
├── CartridgeFormModal.svelte        # CRUD экземпляра
├── OperationModal.svelte            # единая модалка lifecycle-операций
├── LowStockBanner.svelte
├── ModelsList.svelte
├── ModelListRow.svelte
├── ModelFormModal.svelte
├── CompatibilityEditor.svelte       # добавляемый список пар brand+model
└── api.ts
```

### Pattern 1: V016 Migration — Kind Lookup + Color + app_settings + FTS Triggers

**What:** Добавить недостающие колонки в `cartridge_models`, создать `app_settings`, добавить `cartridges_fts` sync-триггеры.
**When to use:** Это первая Wave — без неё нельзя запускать backend.

```sql
-- Source: migrations/V013__devices_fts_triggers.sql (аналог)
-- V016: cartridge_kinds lookup + cartridge_models columns + app_settings + cartridges_fts triggers

CREATE TABLE cartridge_kinds (
  id   INTEGER PRIMARY KEY,
  name TEXT    NOT NULL UNIQUE
);
INSERT INTO cartridge_kinds (id, name) VALUES
  (1, 'Картридж'),
  (2, 'Фотобарабан');

ALTER TABLE cartridge_models ADD COLUMN kind_id INTEGER NOT NULL DEFAULT 1
  REFERENCES cartridge_kinds(id);
ALTER TABLE cartridge_models ADD COLUMN color TEXT NULL;

CREATE TABLE app_settings (
  key             TEXT    NOT NULL PRIMARY KEY,
  value           TEXT    NOT NULL,
  created_at_utc  INTEGER NOT NULL,
  updated_at_utc  INTEGER NOT NULL
);
INSERT INTO app_settings (key, value, created_at_utc, updated_at_utc)
  VALUES ('low_stock_threshold', '2', unixepoch(), unixepoch());

-- FTS sync triggers for cartridges_fts (pattern: V013 devices_fts_ai/ad/au)
CREATE TRIGGER cartridges_fts_ai
AFTER INSERT ON cartridges
WHEN NEW.deleted_at_utc IS NULL
BEGIN
  INSERT INTO cartridges_fts(rowid, code, location, holder_name)
  VALUES (NEW.id, NEW.code, NEW.location, NEW.holder_name);
END;

CREATE TRIGGER cartridges_fts_ad
AFTER DELETE ON cartridges
BEGIN
  INSERT INTO cartridges_fts(cartridges_fts, rowid, code, location, holder_name)
  VALUES ('delete', OLD.id, OLD.code, OLD.location, OLD.holder_name);
END;

CREATE TRIGGER cartridges_fts_au
AFTER UPDATE ON cartridges
BEGIN
  INSERT INTO cartridges_fts(cartridges_fts, rowid, code, location, holder_name)
  VALUES ('delete', OLD.id, OLD.code, OLD.location, OLD.holder_name);
  INSERT INTO cartridges_fts(rowid, code, location, holder_name)
  SELECT NEW.id, NEW.code, NEW.location, NEW.holder_name
  WHERE NEW.deleted_at_utc IS NULL;
END;

PRAGMA user_version = 16;
```

[VERIFIED: codebase] V013 содержит точный паттерн триггеров. V015 — последняя миграция, V016 — следующая.

### Pattern 2: Counter Increment с retry при коллизии (D-Code-01 + D-Code-Override-01)

**What:** Авто-код из `cartridge_seq` + обработка коллизии UNIQUE.
**When to use:** В `CartridgeService::create`, аналог `ActService::create` с `act_number`.

```rust
// Source: crates/trackly-infra/src/repos/acts_sqlite.rs ::increment_counter_in_tx
// Паттерн для cartridges_create внутри writer.execute(|conn| { ... }):

let tx = conn.transaction().map_err(map_rusqlite)?;

// Случай 1: пользовательский code
if let Some(custom_code) = payload.code_override {
    let exists: bool = tx.query_row(
        "SELECT COUNT(*) FROM cartridges WHERE code = ?1",
        params![&custom_code], |r| r.get::<_,i64>(0)
    ).map_err(map_rusqlite)? > 0;
    if exists {
        return Err(AppError::Conflict { reason: format!("Код «{}» уже занят", custom_code) });
    }
    // Пишем custom_code, НЕ инкрементируем counter.
    // audit_log action = "custom:cartridge_code_override"
    code = custom_code;
} else {
    // Случай 2: авто-код с retry при коллизии
    loop {
        let seq = increment_counter_in_tx(&tx, "cartridge_seq")?;
        let candidate = format!("C-{:06}", seq);
        let exists: bool = tx.query_row(
            "SELECT COUNT(*) FROM cartridges WHERE code = ?1",
            params![&candidate], |r| r.get::<_,i64>(0)
        ).map_err(map_rusqlite)? > 0;
        if !exists {
            code = candidate;
            break;
        }
        // При коллизии — инкрементируем ещё раз (счётчик не теряется)
    }
}
```

[VERIFIED: codebase] `increment_counter_in_tx` в `crates/trackly-infra/src/repos/acts_sqlite.rs:370`.

### Pattern 3: Lifecycle Transition под single-writer

**What:** Переход статуса + запись audit_log в одной транзакции.
**When to use:** Любая lifecycle-операция (install, return, to_refill, from_refill, write_off).

```rust
// Source: аналог ActService::do_return (crates/trackly-app/src/services/act_service.rs)
// CartridgeService::transition под WriterHandle::execute:

let cartridge_id = payload.cartridge_id;
let result = self.writer.execute(move |conn| {
    let tx = conn.transaction().map_err(map_rusqlite)?;

    // 1. Fetch current + optimistic lock check
    let before = repo.fetch_full_in_tx(&tx, cartridge_id)?;
    // 2. Validate transition (status must match expected)
    // 3. UPDATE cartridges SET status_id=?, state_id=?, location=?, holder_name=?, updated_at_utc=?, version=version+1
    //    WHERE id=? AND version=? (optimistic lock)
    // 4. INSERT OR IGNORE INTO locations (name, ...) если введена новая локация
    // 5. INSERT INTO audit_log {entity_type='cartridge', action='custom:install'/'custom:return_to_stock'/...,
    //                           before_json, after_json, payload_json={дата, кто выдал, кому выдал, ...}}
    tx.commit().map_err(map_rusqlite)?;
    Ok(after)
}).await?;
```

[VERIFIED: codebase] Паттерн из `ActService::do_return` и `ActService::create`.

### Pattern 4: FTS + LIKE UNION CTE search (CART-11)

**What:** Поиск по картриджам — по коду/расположению/holder через FTS5 + JOIN на модель через LIKE.
**When to use:** `CartridgeRepository::search`.

```sql
-- Source: аналог crates/trackly-infra/src/repos/acts_sqlite.rs ::search
-- cartridges_fts покрывает code, location, holder_name.
-- Для поиска по модели (brand, model) — LIKE JOIN на cartridge_models.

WITH fts_hits AS (
  SELECT f.rowid AS id FROM cartridges_fts f
  WHERE cartridges_fts MATCH ?1          -- FTS5 MATCH expression
),
like_hits AS (
  SELECT c.id FROM cartridges c
  LEFT JOIN cartridge_models m ON c.model_id = m.id
  WHERE c.code LIKE ?2
     OR c.location LIKE ?2
     OR m.brand LIKE ?2
     OR m.model LIKE ?2
)
SELECT ... FROM cartridges c
LEFT JOIN cartridge_models m ON c.model_id = m.id
LEFT JOIN cartridge_statuses cs ON c.status_id = cs.id
LEFT JOIN cartridge_states cst ON c.state_id = cst.id
WHERE c.id IN (SELECT id FROM fts_hits UNION SELECT id FROM like_hits)
  AND c.deleted_at_utc IS NULL
  AND (?3 IS NULL OR c.status_id = ?3)
ORDER BY c.created_at_utc DESC
LIMIT ?4 OFFSET ?5
```

[VERIFIED: codebase] Паттерн LIKE+FTS UNION CTE из `acts_sqlite.rs::search`, Phase 3 Plan 05.

### Pattern 5: audit_log для картриджей (CART-10)

**What:** Каждая lifecycle-операция пишет строку в `audit_log`. Карточка читает хронологически.
**When to use:** При каждой mutation через `cartridges_transition`.

```rust
// Source: crates/trackly-infra/src/repos/audit_log_sqlite.rs
audit_repo.insert(&tx, AuditEntry {
    entity_type: "cartridge",
    entity_id: cartridge_id,
    action: "custom:install",   // или "custom:return_to_stock" / "custom:to_refill" / etc.
    user_id: None,              // Phase 4: всегда NULL (Phase 5 добавит user_id)
    before_json: Some(serde_json::to_string(&before_snapshot)?),
    after_json: Some(serde_json::to_string(&after_snapshot)?),
    payload_json: Some(serde_json::to_string(&json!({
        "op": "install",
        "date_utc": op.date_utc,
        "given_by": op.given_by_name,
        "given_to": op.given_to_name,
        "location": op.location,
    }))?),
    created_at_utc: now,
})?;
```

[VERIFIED: codebase] `AuditEntry` shape из `audit_log_sqlite.rs:27`.

### Pattern 6: AppCtx extension (D-AppCtx-Extension-01)

**What:** Добавить `CartridgeService` в `AppCtx` по паттерну Phase 2/3.
**When to use:** В `context.rs::AppCtx::build`.

```rust
// Source: crates/trackly-app/src/context.rs (Phase 2 pattern)
// Добавить в AppCtx struct:
pub cartridges: Arc<CartridgeService>,

// В AppCtx::build после Step 11 (после devices и acts):
let cartridges = Arc::new(CartridgeService::new(
    writer.clone(),
    readers.clone(),
    clock.clone(),
));
```

[VERIFIED: codebase] `context.rs:59-74` — паттерн полей; `context.rs:149-175` — паттерн build.

### Pattern 7: specta_export extension

**What:** Добавить все `cartridges_*` команды в `collect_commands!`.
**When to use:** После добавления `#[tauri::command] #[specta::specta]` функций.

```rust
// Source: crates/trackly-app/src/specta_export.rs
collect_commands![
    // ... existing ...
    // Phase 4 — Cartridges
    crate::tauri_cmds::cartridges::cartridges_list,
    crate::tauri_cmds::cartridges::cartridges_get,
    crate::tauri_cmds::cartridges::cartridges_create,
    crate::tauri_cmds::cartridges::cartridges_update,
    crate::tauri_cmds::cartridges::cartridges_delete,
    crate::tauri_cmds::cartridges::cartridges_transition,
    crate::tauri_cmds::cartridges::cartridges_search,
    crate::tauri_cmds::cartridges::cartridges_status_counts,
    crate::tauri_cmds::cartridges::cartridges_get_history,
    crate::tauri_cmds::cartridges::cartridges_low_stock,
    crate::tauri_cmds::cartridges::cartridge_models_list,
    crate::tauri_cmds::cartridges::cartridge_models_get,
    crate::tauri_cmds::cartridges::cartridge_models_create,
    crate::tauri_cmds::cartridges::cartridge_models_update,
    crate::tauri_cmds::cartridges::cartridge_models_delete,
    crate::tauri_cmds::cartridges::cartridges_suggest_brand,
    crate::tauri_cmds::cartridges::cartridges_suggest_model,
    crate::tauri_cmds::cartridges::cartridges_suggest_compat_printer,
    crate::tauri_cmds::cartridges::cartridges_suggest_location,
]
```

[VERIFIED: codebase] `specta_export.rs:18-64` — паттерн collect_commands!.

### Anti-Patterns to Avoid

- **Прямые записи без writer_worker:** Любой INSERT/UPDATE/DELETE должен идти через `WriterHandle::execute`. Никогда не писать через reader connection.
- **FTS-таблица без триггеров:** `cartridges_fts` создана в V012 без триггеров (V012 комментарий строка 10: «Triggers that keep FTS5 tables in sync... are NOT created in Phase 1»). Без V016-триггеров поиск будет искать в пустой таблице.
- **Разные lifecycle-операции через разные update-paths:** Все переходы должны через одну точку (service method), которая гарантирует audit_log insert. Никаких «прямых обновлений статуса» в репо без аудита.
- **Хранение `user_id != NULL` в audit_log в Phase 4:** Phase 5 подключает auth; в Phase 4 всегда `user_id = NULL`.
- **NK constraint на code без retry:** `cartridges.code UNIQUE` — при коллизии авто-кода нужен retry в той же tx, иначе транзакция упадёт с rusqlite constraint error.
- **Не вызывать `INSERT OR IGNORE` в locations:** Если не делать round-trip в `locations`, автокомплит расположений в будущих операциях не найдёт введённые значения (единый справочник для устройств и картриджей).
- **`cartridge_models.kind_id` без DEFAULT 1:** Существующие строки в V005 не имеют `kind_id`. `ALTER TABLE ... ADD COLUMN kind_id INTEGER NOT NULL DEFAULT 1` — DEFAULT гарантирует, что существующие строки получат kind=Картридж.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Atomic counter increment | Кастомный лок / SELECT+UPDATE | `increment_counter_in_tx` из `acts_sqlite.rs` | Уже работает, протестирован, races-free под single-writer |
| FTS5 поиск | LIKE по всем полям | `cartridges_fts MATCH` + LIKE UNION CTE | FTS5 обрабатывает кириллицу через unicode61, LIKE only — медленно на больших таблицах |
| Audit history | Кастомная таблица history | `audit_log` (V008) + `SqliteAuditLogRepository::insert` | Единый лог для всех сущностей, уже протестирован |
| Context menu | Кастомный dropdown с overflow fix | `DeviceContextMenu.svelte` паттерн с `use:portal` | Portal решает overflow:hidden clip, mousedown-outside закрытие |
| Focus-open autocomplete | Кастомный input с dropdown | `LocationAutocomplete.svelte` (DEF-1 паттерн) | Открытие на focus без ввода — протестированный UX паттерн |
| Master-detail layout | CSS grid в новом компоненте | `ActsMasterDetail.svelte` (35/65 grid) | Готовый адаптивный layout с min-width constraints |
| Modal backdrop discipline | mouseup/click timing | `Modal.svelte` с mousedown/mouseup pattern (G-1 fix) | Backdrop close bug уже закрыт в Phase 3.1 |

**Key insight:** Phase 1–3 предоставляют полный набор строительных блоков. Phase 4 — сборка из готовых деталей, а не изобретение новых механизмов.

---

## Runtime State Inventory

Фаза 4 — расширение, а не переименование/рефакторинг. Явных rename-операций нет.

| Category | Items Found | Action Required |
|----------|-------------|-----------------|
| Stored data | `cartridges` таблица уже содержит строки из V005 seed (пустая в свежей БД), `cartridge_models` — пустая. `cartridge_model_compatibility` — пустая | V016 `ALTER TABLE cartridge_models ADD COLUMN` — безопасно для существующих строк (DEFAULT 1) |
| Live service config | Нет | — |
| OS-registered state | Нет | — |
| Secrets/env vars | Нет новых | — |
| Build artifacts | `test_db()` fixture ожидает `user_version = 15` (строка 41 test_db.rs). После V016 станет 16 | Обновить assertion в `test_db.rs` с 15 → 16 |

---

## Common Pitfalls

### Pitfall 1: cartridges_fts пустая без триггеров
**What goes wrong:** FTS-поиск возвращает 0 результатов даже при наличии данных.
**Why it happens:** V012 создал `cartridges_fts` как external-content FTS5 таблицу без триггеров синхронизации. Комментарий V012 строка 9: «Triggers... are NOT created in Phase 1 — Phase 4 owns those triggers».
**How to avoid:** V016 ОБЯЗАН создать три триггера (ai/ad/au) по образцу V013 до любого INSERT в `cartridges`.
**Warning signs:** `SELECT COUNT(*) FROM cartridges_fts` = 0 при наличии строк в `cartridges`.

### Pitfall 2: ALTER TABLE + DEFAULT 1 на kind_id
**What goes wrong:** `ALTER TABLE cartridge_models ADD COLUMN kind_id INTEGER NOT NULL` — SQLite не разрешает NOT NULL без DEFAULT на ADD COLUMN при наличии существующих строк.
**Why it happens:** SQLite ограничение: `ALTER TABLE ADD COLUMN NOT NULL` требует `DEFAULT` выражения, иначе constraint-ошибка.
**How to avoid:** `ALTER TABLE cartridge_models ADD COLUMN kind_id INTEGER NOT NULL DEFAULT 1 REFERENCES cartridge_kinds(id)` — DEFAULT 1 обязателен.
**Warning signs:** Migration failure с «Cannot add a NOT NULL column with default value NULL».

### Pitfall 3: cartridges.code UNIQUE коллизия при авто-нумерации
**What goes wrong:** Если пользователь вручную создал `C-000001`, а counter ещё не достиг 1 — следующий авто-код упадёт с rusqlite UNIQUE constraint violation, прерывая транзакцию.
**Why it happens:** counter и UNIQUE — независимые механизмы. Counter не знает об уже занятых кодах.
**How to avoid:** После `increment_counter_in_tx` проверить `SELECT COUNT(*) FROM cartridges WHERE code=?` и при коллизии повторить increment в той же транзакции (цикл).
**Warning signs:** `rusqlite::Error::SqliteFailure` с `SQLITE_CONSTRAINT_UNIQUE` на INSERT cartridges.

### Pitfall 4: specta bindings не регенерированы после добавления команд
**What goes wrong:** Frontend получает TypeScript-ошибку «Unknown tauri command» или тип не совпадает.
**Why it happens:** `collect_commands!` в `specta_export.rs` не включает новые команды, или `export_bindings` test не запущен.
**How to avoid:** Всегда добавлять новые `#[tauri::command] #[specta::specta]` функции в `specta_export.rs::builder()`. Тест `tests/export_bindings.rs` регенерирует `ui/src/bindings.ts` автоматически при `cargo test`.
**Warning signs:** `bindings.ts` не содержит `cartridges_*` функций. `pnpm svelte-check` ошибки на `apiCall`.

### Pitfall 5: locations round-trip не делается — автокомплит расположений не работает
**What goes wrong:** При вводе нового расположения в CartridgeFormModal оно сохраняется в `cartridges.location` TEXT, но не попадает в таблицу `locations`. При следующем обращении `locations_autocomplete` команда не найдёт его.
**Why it happens:** `cartridges.location` — freeform TEXT (не FK). Команда `locations_autocomplete` читает из таблицы `locations`.
**How to avoid:** В writer-транзакции `cartridges_create` и `cartridges_transition` добавлять `INSERT OR IGNORE INTO locations (name, created_at_utc, updated_at_utc, version) VALUES (?, ?, ?, 1)` при непустом location.
**Warning signs:** Расположение картриджа видно в его карточке, но не появляется в dropdown при создании следующего картриджа.

### Pitfall 6: test_db user_version assertion сломается после V016
**What goes wrong:** `crates/trackly-infra/tests/seed_data.rs` и `test_db.rs` проверяют `user_version = 15`. После V016 тест упадёт.
**Why it happens:** Жёстко закодированный номер версии.
**How to avoid:** В Wave 0 (миграция V016) сразу обновить assertion с 15 → 16 в `test_db.rs:41` и любых других файлах с hardcoded version check.
**Warning signs:** `cargo test` падает на `assert_eq!(user_version, 15)`.

### Pitfall 7: OperationModal — выбор расположения по kind
**What goes wrong:** «Установить в принтер» должен показывать «не-склад» локации, «Вернуть на склад» — «склад» локации. Но если kind не проставлен — оба автокомплита показывают всё.
**Why it happens:** `locations.kind` массово не заполнен (управление — Phase 7). D-Op-Location-01 явно разрешает это для Phase 4.
**How to avoid:** Подписи «Рекомендуется склад» / «Рекомендуется не склад» — только текстовые подсказки. Фактический фильтр `WHERE kind = 'warehouse'` включается автоматически после Phase 7. Оба автокомплита используют одну и ту же команду `locations_autocomplete`.
**Warning signs:** Попытка жёсткой фильтрации по kind в Phase 4 сломает UX, пока kind не заполнен.

---

## Code Examples

### Пример 1: CartridgeRow domain struct (по образцу devices.rs)

```rust
// Аналог: crates/trackly-core/src/domain/devices.rs
// Создать: crates/trackly-core/src/domain/cartridges.rs

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CartridgeModelRow {
    pub id: i64,
    pub brand: String,
    pub model: String,
    pub kind_id: i64,
    pub color: Option<String>,
    pub notes: Option<String>,
    pub created_at_utc: i64,
    pub updated_at_utc: i64,
    pub deleted_at_utc: Option<i64>,
    pub version: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CartridgeRow {
    pub id: i64,
    pub code: String,
    pub model_id: i64,
    // Joined:
    pub model_brand: Option<String>,
    pub model_name: Option<String>,
    pub model_kind_id: Option<i64>,
    pub status_id: i64,
    pub status_name: Option<String>,    // joined
    pub state_id: Option<i64>,
    pub state_name: Option<String>,     // joined
    pub location: Option<String>,
    pub holder_name: Option<String>,
    pub notes: Option<String>,
    pub created_at_utc: i64,
    pub updated_at_utc: i64,
    pub deleted_at_utc: Option<i64>,
    pub version: i64,
}
```

[VERIFIED: codebase] Pattern from `crates/trackly-core/src/domain/devices.rs`.

### Пример 2: CartridgeDto (по образцу device.rs DTO)

```rust
// Аналог: crates/trackly-app/src/dto/device.rs
// Создать: crates/trackly-app/src/dto/cartridge.rs
// Паттерн: snake_case JSON, #[specta(type = i32)] на i64 полях

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct CartridgeDto {
    #[specta(type = i32)]   pub id: i64,
    #[specta(type = i32)]   pub version: i64,
    pub code: String,
    #[specta(type = i32)]   pub model_id: i64,
    pub model_brand: Option<String>,
    pub model_name: Option<String>,
    #[specta(type = Option<i32>)] pub model_kind_id: Option<i64>,
    #[specta(type = i32)]   pub status_id: i64,
    pub status_name: Option<String>,
    #[specta(type = Option<i32>)] pub state_id: Option<i64>,
    pub state_name: Option<String>,
    pub location: Option<String>,
    pub holder_name: Option<String>,
    pub notes: Option<String>,
    #[specta(type = i32)]   pub created_at_utc: i64,
    #[specta(type = i32)]   pub updated_at_utc: i64,
}
```

[VERIFIED: codebase] `#[specta(type = i32)]` паттерн из `dto/device.rs:28-60`.

### Пример 3: TransitionPayload для cartridges_transition

```rust
// Один enum TransitionOp, одна команда — Claude's Discretion вариант А:
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(tag = "op")]
pub enum CartridgeTransitionPayload {
    Install {
        cartridge_id: i64,
        version: i64,
        date_utc: i64,
        given_by_name: String,
        given_to_name: String,
        location: String,
    },
    ReturnToStock {
        cartridge_id: i64,
        version: i64,
        state_id: i64,       // default: 3 (Пустой)
        location: String,
        notes: Option<String>,
    },
    ToRefill {
        cartridge_id: i64,
        version: i64,
        date_utc: i64,
        given_by_name: String,
        given_to_name: String,
        location: String,
    },
    FromRefill {
        cartridge_id: i64,
        version: i64,
        state_id: i64,       // default: 1 (Полный)
        location: String,
        notes: Option<String>,
    },
    WriteOff {
        cartridge_id: i64,
        version: i64,
        date_utc: i64,
        notes: Option<String>,
    },
}
```

[ASSUMED] Конкретная сигнатура — Claude's Discretion. Планировщик может выбрать отдельные команды вместо enum.

### Пример 4: LowStockBanner логика

```rust
// CartridgeService::low_stock — использует ReaderPool (reads only)
pub async fn low_stock(&self) -> Result<Vec<LowStockItem>, AppError> {
    let conn = self.readers.acquire()?;
    let threshold: i64 = conn.query_row(
        "SELECT CAST(value AS INTEGER) FROM app_settings WHERE key = 'low_stock_threshold'",
        [], |r| r.get(0)
    ).unwrap_or(2);  // fallback если app_settings не заполнен

    // Для каждой модели: count(status='На складе' AND state='Полный')
    let rows = conn.prepare(
        "SELECT m.id, m.brand, m.model, COUNT(c.id) AS cnt
         FROM cartridge_models m
         LEFT JOIN cartridges c ON c.model_id = m.id
           AND c.status_id = (SELECT id FROM cartridge_statuses WHERE name='На складе')
           AND c.state_id  = (SELECT id FROM cartridge_states   WHERE name='Полный')
           AND c.deleted_at_utc IS NULL
         WHERE m.deleted_at_utc IS NULL
         GROUP BY m.id
         HAVING cnt < ?1
         ORDER BY cnt ASC, m.brand ASC, m.model ASC"
    )?.query_map(params![threshold], |r| Ok(LowStockItem {
        model_id: r.get(0)?,
        brand: r.get(1)?,
        model: r.get(2)?,
        count: r.get(3)?,
        threshold,
    }))...;
    Ok(rows)
}
```

[VERIFIED: codebase] `cartridge_statuses` id=1 для «На складе», `cartridge_states` id=1 для «Полный» — из V001.

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Specta derive на AppError | Manual `impl specta::Type` делегирующий AppErrorRepr | Phase 1 Plan 04 | Нельзя использовать `#[derive(Type)]` на AppError — нужен manual impl |
| `sqlx-sqlite` | `rusqlite` + single-writer task | Phase 1 (locked) | Все мутации через `WriterHandle::execute`, blocking spawn |
| Separate `chrono` | `time` crate для UTC | Phase 1 (locked) | Используем `Clock::unix_seconds()` trait object |
| `app_data_dir()` | `current_exe().parent()` | Phase 1 (locked) | Portable mode — никаких APPDATA путей |

**Deprecated/outdated:**
- `sqlx` для SQLite: исключён по lock-starvation (CLAUDE.md). В Phase 4 не использовать.
- `dirs::*_dir()`: запрещён clippy disallowed-methods (Phase 1 CI gate).

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Сигнатура `CartridgeTransitionPayload` с tagged enum `#[serde(tag = "op")]` | Code Examples §3 | Если планировщик выберет отдельные команды — нужно другой набор specta exports; UI сложность минимальна |
| A2 | `app_settings` таблица полностью новая (не существует в V001-V015) | Pattern 1 | Если уже создана в каком-то migration — DDL упадёт. Проверено: `migrations/` не содержит `app_settings`. [VERIFIED: codebase grep] |

**Все остальные утверждения подтверждены через чтение исходного кода кодовой базы.**

---

## Open Questions

1. **Единая `cartridges_transition` vs отдельные команды**
   - Что мы знаем: D-Op-Modal-01 говорит «бэкенд — одна команда». Claude's Discretion явно отмечает этот выбор.
   - Что неясно: Как лучше для TypeScript — tagged union или отдельные typed функции.
   - Recommendation: Единая `cartridges_transition(payload: CartridgeTransitionPayload)` с tagged enum `#[serde(tag = "op")]` — бэкенд-friendly, одна точка audit_log. TS различает по `op: "Install" | "ReturnToStock" | ...` через discriminated union.

2. **Удаление модели с живыми экземплярами**
   - Что мы знаем: Claude's Discretion рекомендует «запрет с понятной ошибкой».
   - Recommendation: Перед `soft_delete` модели — COUNT живых экземпляров, при >0 вернуть `AppError::Conflict { reason: "Модель используется N экземплярами" }`.

---

## Environment Availability

Инфраструктура уже запущена и проверена в Phase 1–3. Новые внешние зависимости отсутствуют.

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | cargo build | ✓ | 1.88 (rust-toolchain.toml) | — |
| SQLite (bundled) | rusqlite bundled feature | ✓ | компилируется в бинарник | — |
| Node.js / pnpm | frontend build | ✓ | pnpm 10.17.1 | — |
| refinery 0.9 | migrations | ✓ | в Cargo.lock | — |

**Missing dependencies with no fallback:** none.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo nextest` (опционально), `cargo test` (стандарт) |
| Config file | `Cargo.toml` `[dev-dependencies]` |
| Quick run command | `cargo test -p trackly-app --test cartridges_crud -- --nocapture` |
| Full suite command | `cargo test && pnpm svelte-check && pnpm lint` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| CART-04 | Авто-код C-000001 атомарен, нет коллизий | integration | `cargo test --test cartridges_numbering` | Wave 0 |
| CART-04 | Custom override audit_log | integration | `cargo test --test cartridges_crud` | Wave 0 |
| CART-06 | Transition меняет status + пишет audit_log | integration | `cargo test --test cartridges_lifecycle` | Wave 1 |
| CART-11 | FTS поиск по коду/модели | integration | `cargo test --test cartridges_search` | Wave 1 |
| CART-12 | low_stock returns models below threshold | integration | `cargo test --test cartridges_low_stock` | Wave 1 |
| CART-10 | History из audit_log для экземпляра | integration | `cargo test --test cartridges_history` | Wave 1 |
| CART-05 | Counts по статусам корректны | integration | в cartridges_crud | Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p trackly-app --test <новый тест>`
- **Per wave merge:** `cargo test && pnpm svelte-check`
- **Phase gate:** полный `cargo test && pnpm svelte-check && pnpm lint` перед `/gsd-verify-work`

### Wave 0 Gaps

- [ ] `crates/trackly-app/tests/cartridges_crud.rs` — CART-03/04/05 создание/получение/удаление
- [ ] `crates/trackly-app/tests/cartridges_lifecycle.rs` — CART-06/07/08/09 переходы + audit_log
- [ ] `crates/trackly-app/tests/cartridges_numbering.rs` — CART-04 авто-код + коллизия retry
- [ ] `crates/trackly-app/tests/cartridges_search.rs` — CART-11 FTS + LIKE + JOIN модель
- [ ] `crates/trackly-app/tests/cartridges_low_stock.rs` — CART-12 подсчёт ниже порога
- [ ] `crates/trackly-app/tests/cartridges_history.rs` — CART-10 audit_log чтение
- [ ] Обновить `crates/trackly-infra/src/test_support/test_db.rs:41` assertion с `15` → `16`

---

## Security Domain

`security_enforcement: true` (config.json), ASVS Level 1.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | Phase 5 добавит. В Phase 4: `user_id = NULL` в audit_log |
| V3 Session Management | no | Phase 5 |
| V4 Access Control | no | Phase 5. В Phase 4 все команды открыты (desktop trusted mode) |
| V5 Input Validation | **yes** | `AppError::Validation` на все обязательные поля; `brand`/`model` не пустые; `state_id` из разрешённого диапазона |
| V6 Cryptography | no | Нет новых секретов в Phase 4 |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| SQL injection через код/модель | Tampering | Все запросы через `rusqlite::params![]` (parameterized). Никогда не конкатенировать пользовательский ввод в SQL |
| Excel injection в CSV export (если добавится в Phase 7) | Tampering | `csv_safe()` helper уже в `device_service.rs:20-26` — повторить паттерн |
| Path traversal через `cartridges.location` | Tampering | location — только string value, не путь к файлу. INSERT OR IGNORE в locations — только name column |
| UNIQUE constraint race на code | Elevation | Решено: single-writer + `BEGIN IMMEDIATE` + retry loop |

---

## Sources

### Primary (HIGH confidence)

- Codebase (verified): `migrations/V005__cartridges.sql` — существующая схема картриджей
- Codebase (verified): `migrations/V001__init_pragmas_and_lookups.sql` — lookup tables
- Codebase (verified): `migrations/V009__counters.sql` — `cartridge_seq` counter
- Codebase (verified): `migrations/V012__indexes_and_fts.sql` — `cartridges_fts` virtual table + комментарий про отсутствующие триггеры
- Codebase (verified): `migrations/V013__devices_fts_triggers.sql` — образец триггеров для FTS5
- Codebase (verified): `crates/trackly-infra/src/repos/acts_sqlite.rs:370-398` — `increment_counter_in_tx`, `peek_counter`
- Codebase (verified): `crates/trackly-infra/src/repos/audit_log_sqlite.rs` — `AuditEntry` shape + `insert`
- Codebase (verified): `crates/trackly-infra/src/db/writer_worker.rs` — `WriterHandle::execute`
- Codebase (verified): `crates/trackly-app/src/context.rs` — `AppCtx` struct + `build` lifecycle
- Codebase (verified): `crates/trackly-app/src/specta_export.rs` — `collect_commands!` паттерн
- Codebase (verified): `crates/trackly-app/src/services/act_service.rs` — сервис-образец для CartridgeService
- Codebase (verified): `crates/trackly-infra/src/test_support/test_db.rs` — hardcoded version assertion (15 → нужно 16)
- Codebase (verified): `ui/src/features/devices/DeviceContextMenu.svelte` — паттерн контекстного меню
- Codebase (verified): `ui/src/features/devices/DeviceFilters.svelte` — паттерн switch-bar
- Codebase (verified): `ui/src/features/acts/ActsMasterDetail.svelte` — master-detail layout
- Codebase (verified): `ui/src/lib/components/PersonAutocomplete.svelte` — focus-open autocomplete
- Codebase (verified): `ui/src/lib/components/LocationAutocomplete.svelte` — locations autocomplete
- Codebase (verified): `ui/src/features/layout/sidebar-config.ts:15` — placeholder `/cartridges` phase:4
- CLAUDE.md (project): стек, ограничения, паттерны

### Secondary (MEDIUM confidence)

- `.planning/phases/04-cartridges/04-CONTEXT.md` — решения пользователя (locked decisions)
- `.planning/REQUIREMENTS.md` §CART-01..12 — точные формулировки требований
- SQLite FTS5 documentation: `content=` external-content table behaviour — [CITED: https://www.sqlite.org/fts5.html#external_content_tables]

### Tertiary (LOW confidence)

- Нет — все ключевые утверждения подтверждены чтением кодовой базы.

---

## Metadata

**Confidence breakdown:**
- Standard Stack: HIGH — всё взято из существующего Cargo.toml/package.json
- Architecture: HIGH — прямое расширение Phase 1–3 паттернов, подтверждено чтением кода
- Pitfalls: HIGH — выведены из реальных комментариев в миграциях и коде
- Test map: MEDIUM — структура тестов по аналогии с Phase 3; конкретные имена файлов — планировщик

**Research date:** 2026-06-07
**Valid until:** 2026-07-07 (стек стабильный, зависимости не меняются)
