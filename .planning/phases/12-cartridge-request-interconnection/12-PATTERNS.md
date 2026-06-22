# Phase 12: Взаимосвязь картриджной заявки - Pattern Map

**Mapped:** 2026-06-22
**Files analyzed:** 13 (создание/правка) + 2 регенерируемых biндинга
**Analogs found:** 13 / 13 (все файлы — правки существующего кода; для каждого есть прямой соседний образец в том же файле или модуле)

Эта фаза не создаёт новых файлов в backend (кроме опционально нового интеграционного
test-файла) — она **точечно расширяет** уже существующие модули. Поэтому "анализ" в
основном — это указание точного места (line range) внутри **того же файла**, где уже
есть соседний код того же рода (другое поле фильтра, другой JOIN, другой transition-кейс),
который нужно скопировать по форме.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `crates/trackly-core/src/domain/cartridges.rs` (CartridgeFilter) | model (domain value type) | CRUD (filter object) | тот же файл: `status_id`/`kind_id`/`model_id` поля (lines 198-209) | exact (расширение того же struct) |
| `crates/trackly-app/src/dto/cartridge.rs` (CartridgeFilter DTO) | model (DTO) | CRUD | тот же файл: `status_id`/`kind_id`/`model_id` + `into_domain()` (lines 324-348) | exact |
| `crates/trackly-infra/src/repos/cartridges_sqlite.rs` (`list()` SQL) | service/repository | CRUD (SQL read) | тот же файл: `list()` WHERE-clause (lines 945-1005), `search()` WHERE-clause (lines 589-664) | exact (тот же `IS NULL OR col = ?N` идиома, расширяемая константным `IN (1,2)`) |
| `crates/trackly-app/src/dto/request.rs` (`RequestDto.printer_location`) | model (DTO) | CRUD (read) | тот же файл: `printer_name: Option<String>` (line 37) + `RequestPrinterOptionDto.location` (lines 91-108) | exact |
| `crates/trackly-infra/src/repos/requests_sqlite.rs` (`SELECT_REQUESTS`) | repository | CRUD (SQL read, JOIN) | `crates/trackly-app/src/services/request_service.rs::printer_options()` SQL (lines 241-250) | exact (готовый `LEFT JOIN locations` приём) |
| `crates/trackly-app/src/services/request_service.rs` (`transition()` notes_json) | service | event-driven (audit write) | тот же файл: существующее построение `notes_json` (lines 469-477) | exact (расширение того же `serde_json::json!({...})`) |
| `crates/trackly-app/src/services/request_service.rs` (`get_history()` parse) | service | transform (JSON→DTO) | тот же файл: текущий `notes` parse (lines 188-207) | exact |
| `crates/trackly-app/src/dto/request.rs` (`RequestHistoryEntryDto` + поля картриджа, если выбран вариант с отдельными полями) | model (DTO) | transform | тот же файл: `RequestHistoryEntryDto` (lines 253-266) | exact |
| `ui/src/features/cartridges/OperationModal.svelte` | component | request-response (form + submit) | тот же файл целиком — расширяется existing install-branch (lines 246-301), `buildPayload()` (124-176), `handleSubmit()` (208-227) | exact (тот же файл, новая ветка внутри) |
| `ui/src/features/requests/RequestDetail.svelte` | component | request-response | тот же файл: `handleInstallSuccess()` (lines 299-323), OperationModal-проброс (lines 581-590) | exact |
| `ui/src/features/cartridges/api.ts` (`list()` signature) | service (API client) | request-response | тот же файл: `cartridges.list`/`cartridges.transition` (lines 22-36) | exact |
| `ui/src/lib/components/<новый CartridgeSelect>.svelte` (если планировщик выберет отдельный компонент) | component | request-response | `ui/src/lib/components/GroupedPrinterSelect.svelte` (весь файл, 149 строк) | role-match (тот же паттерн: плоский DTO[] проп → `<select>`/`<optgroup>`) |
| `crates/trackly-app/tests/cartridges_lifecycle.rs` (новый/расширенный тест на заряд-фильтр) | test | CRUD (integration) | тот же файл: `install_changes_status_to_in_use` + `make_cartridge_service()`/`seed_model()`/`create_stock_cartridge()` helpers (lines 22-86) | exact |
| `crates/trackly-app/tests/phase06_stubs.rs` (`test_req_cart_link` реализация) | test | event-driven (integration) | тот же файл: `test_request_lifecycle()` (lines 241-343) — паттерн создания caller/seed user/transition | exact |
| `ui/src/bindings.ts`, `ui/src/bindings-phase6.ts` | config (generated) | transform | регенерируются `tauri-specta` командой — не редактируются руками | n/a (codegen) |

## Pattern Assignments

### `crates/trackly-core/src/domain/cartridges.rs` — `CartridgeFilter` (+ `installable_only: bool`)

**Analog:** тот же файл, тот же struct (lines 196-209)

**Текущая форма** (lines 196-209):
```rust
/// Filter parameters for cartridge list queries.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CartridgeFilter {
    /// Filter by status (1=На складе, 2=В работе, 3=На заправке, 4=Списано); None = all.
    pub status_id: Option<i64>,
    /// Filter by kind (1=Картридж, 2=Фотобарабан); None = all.
    pub kind_id: Option<i64>,
    /// Filter by model_id; None = all.
    pub model_id: Option<i64>,
    /// Full-text search query (applied by search_acts in repo, not list).
    pub search: Option<String>,
    /// Include soft-deleted rows.
    pub include_deleted: bool,
}
```

**Паттерн для расширения:** добавить новое `pub` поле в стиле существующих
(doc-comment на отдельной строке, имя без сокращений). Использовать
**bool-флаг** `installable_only: bool` (как рекомендовано в RESEARCH.md Pattern 1 /
Open Question 1), а не `Vec<i64>` — набор `{1,2}` фиксирован бизнес-правилом D-01 и
не варьируется, поэтому константный `IN (1,2)` в SQL проще, чем динамический список
плейсхолдеров в rusqlite. `#[derive(Default)]` уже на struct — `false` по умолчанию
не требует ручной инициализации в существующих вызовах конструктора с
`..Default::default()`/`CartridgeFilter { status_id: ..., ..Default::default() }`,
но если где-то есть позиционная конструкция struct-literal без `..Default::default()`,
все её сайты надо найти (`grep -rn "CartridgeFilter {" crates/`) и обновить.

---

### `crates/trackly-app/src/dto/cartridge.rs` — `CartridgeFilter` DTO (+ `installable_only`)

**Analog:** тот же файл (lines 324-348)

**Текущая форма** (lines 324-348):
```rust
/// Filter passed by the UI to `cartridges_list` / `cartridges_search`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
pub struct CartridgeFilter {
    #[specta(type = Option<i32>)]
    pub status_id: Option<i64>,
    #[specta(type = Option<i32>)]
    pub kind_id: Option<i64>,
    #[specta(type = Option<i32>)]
    pub model_id: Option<i64>,
    pub search: Option<String>,
    #[serde(default)]
    pub include_deleted: bool,
}

impl CartridgeFilter {
    /// Convert into the domain filter for repository calls.
    pub fn into_domain(self) -> trackly_core::domain::cartridges::CartridgeFilter {
        trackly_core::domain::cartridges::CartridgeFilter {
            status_id: self.status_id,
            kind_id: self.kind_id,
            model_id: self.model_id,
            search: self.search,
            include_deleted: self.include_deleted,
        }
    }
}
```

**Паттерн:** `bool`-поля в DTO уже несут `#[serde(default)]` (см. `include_deleted`,
line 334) — копировать это же для `installable_only`, чтобы старые фронтовые вызовы
без нового поля не ломались при десериализации (обратная совместимость, важно т.к.
существующий `cartridges_list` экран продолжает слать старый `CartridgeFilter` без
`installable_only`). Не забыть прокинуть поле через `into_domain()` — это типичная
ошибка пропуска при добавлении поля в зеркальную пару domain/DTO struct.

**Note по wire-формату:** DTO здесь **snake_case** (нет `#[serde(rename_all =
"camelCase")]` на этом struct, в отличие от `request.rs`), значит фронт отправляет
`installable_only` как есть (snake_case) — сверить с `CartridgeFilter` TS-типом после
регенерации биндингов (`ui/src/features/cartridges/api.ts` использует `filter:
CartridgeFilter` напрямую из `bindings.ts`).

---

### `crates/trackly-infra/src/repos/cartridges_sqlite.rs` — `list()` SQL (+ `state_id IN (1,2)`)

**Analog:** тот же файл, `list()` (lines 945-1005) и параллельный `search()` (lines 589-664) — оба несут идентичную `(?N IS NULL OR col = ?N)` идиому, которую нужно дополнить новым условием.

**Текущий COUNT + SELECT WHERE-clause** (lines 956-983):
```rust
let total: i64 = conn
    .query_row(
        "SELECT COUNT(*) FROM cartridges c \
         LEFT JOIN cartridge_models m ON m.id = c.model_id \
         WHERE (?1 = 1 OR c.deleted_at_utc IS NULL) \
           AND (?2 IS NULL OR c.status_id = ?2) \
           AND (?3 IS NULL OR m.kind_id = ?3) \
           AND (?4 IS NULL OR c.model_id = ?4)",
        params![
            include_deleted as i64,
            filter.status_id,
            filter.kind_id,
            filter.model_id,
        ],
        |r| r.get(0),
    )
    .map_err(map_rusqlite)?;

let mut stmt = conn
    .prepare(&format!(
        "{SELECT_CARTRIDGES} \
         WHERE (?1 = 1 OR c.deleted_at_utc IS NULL) \
           AND (?2 IS NULL OR c.status_id = ?2) \
           AND (?3 IS NULL OR m.kind_id = ?3) \
           AND (?4 IS NULL OR c.model_id = ?4) \
         ORDER BY c.created_at_utc DESC, c.id DESC \
         LIMIT ?5 OFFSET ?6"
    ))
    .map_err(map_rusqlite)?;
```

**Паттерн для нового условия:** добавить пятый параметр-флаг и константный `IN`,
**не** параметризуя сами значения 1/2 (они фиксированы доменным правилом, как и в
`model_kind_in_tx`/`validate_from_status` уже хардкодящих 1/2/3/4 как магические
домейн-константы в этом же crate):
```rust
AND (?5 = 0 OR c.state_id IN (1, 2))
```
с `params![..., filter.installable_only as i64]` добавленным и в COUNT, и в SELECT
веткy, и пятый/шестой позиционные `?N` соответственно сдвинуть (`LIMIT ?6 OFFSET ?7`).
**Важно:** именно `IN (1,2)`, не `state_id = 1 AND state_id = 2` (Pitfall 4 из
RESEARCH.md — невозможное условие на одной строке).

---

### `crates/trackly-app/src/dto/request.rs` — `RequestDto.printer_location` (+ DTO)

**Analog:** тот же файл — `printer_name: Option<String>` (line 37) задаёт точный
шаблон для нового поля того же характера (joined, nullable, NOT specta-i32-wrapped
т.к. это `String`, не FK id).

**Текущий relevant фрагмент** (lines 17-51, ключевые строки 27, 33, 36-37):
```rust
pub struct RequestDto {
    ...
    #[specta(type = Option<i32>)]
    pub printer_device_id: Option<i64>,
    ...
    #[specta(type = Option<i32>)]
    pub completed_cartridge_id: Option<i64>,
    pub description: Option<String>,
    pub resolution_notes: Option<String>,
    pub requester_name: Option<String>,
    pub printer_name: Option<String>,
    ...
}

impl From<RequestRow> for RequestDto {
    fn from(r: RequestRow) -> Self {
        Self {
            ...
            requester_name: r.requester_name,
            printer_name: r.printer_name,
            ...
        }
    }
}
```

**Паттерн:** добавить `pub printer_location: Option<String>` сразу после
`printer_name` (визуальная смежность — оба про принтер), и `printer_location:
r.printer_location` в `From<RequestRow>`. Domain-уровневый `RequestRow` в
`trackly-core` (не прочитан целиком в этой сессии, но из `requests_sqlite.rs`
видно его форму через `map_row_request`, lines 50-72) должен получить такое же
поле — копировать форму `printer_name: row.get(12)?` для нового индекса.

**ВАЖНО (RequestPrinterOptionDto как образец Option<String> от LEFT JOIN):**
```rust
// lines 99-108 — уже использует ровно тот же null-safe паттерн
pub struct RequestPrinterOptionDto {
    #[specta(type = i32)]
    pub id: i64,
    pub name: String,
    /// Joined `locations.name` — `None` when the printer has no location set.
    pub location: Option<String>,
}
```
Этот тип уже доказывает, что `Option<String>` для joined `locations.name` —
established convention в этом домене (Pitfall 5: не паниковать на NULL).

---

### `crates/trackly-infra/src/repos/requests_sqlite.rs` — `SELECT_REQUESTS` (+ `printer_location` JOIN)

**Analog:** `crates/trackly-app/src/services/request_service.rs::printer_options()` SQL (lines 241-250) — уже реализует ровно нужный JOIN на `locations`.

**Источник паттерна** (request_service.rs, lines 241-250):
```rust
let mut stmt = conn
    .prepare(
        "SELECT d.id, d.name, l.name AS location \
         FROM devices d \
         LEFT JOIN locations l ON d.location_id = l.id \
         WHERE d.type_id = (SELECT id FROM device_types WHERE name = 'Принтер') \
           AND d.deleted_at_utc IS NULL \
         ORDER BY l.name IS NULL, l.name, d.name",
    )
    .map_err(map_rusqlite)?;
```

**Целевой файл — текущий `SELECT_REQUESTS`** (requests_sqlite.rs, lines 30-47):
```rust
const SELECT_REQUESTS: &str = "
    SELECT r.id, r.request_type, r.status,
           r.requested_by_user_id, r.assigned_to_user_id,
           r.printer_device_id, r.cartridge_model_id,
           r.category_id, r.completed_cartridge_id,
           r.description, r.resolution_notes,
           u.full_name AS requester_name,
           d.name AS printer_name,
           r.created_at_utc, r.updated_at_utc, r.deleted_at_utc, r.version,
           r.ad_subtype,
           rc.name AS category_name
      FROM requests r
      LEFT JOIN users u ON u.id = r.requested_by_user_id
      LEFT JOIN devices d ON d.id = r.printer_device_id
      LEFT JOIN request_categories rc ON rc.id = r.category_id
";

fn map_row_request(row: &rusqlite::Row<'_>) -> rusqlite::Result<RequestRow> {
    Ok(RequestRow {
        id: row.get(0)?,
        ...
        printer_name: row.get(12)?,
        created_at_utc: row.get(13)?,
        updated_at_utc: row.get(14)?,
        deleted_at_utc: row.get(15)?,
        version: row.get(16)?,
        ad_subtype: row.get(17)?,
        category_name: row.get(18)?,
    })
}
```

**Паттерн для расширения:**
1. Добавить `LEFT JOIN locations dl ON dl.id = d.location_id` после строки `LEFT JOIN devices d ON ...` (line 43).
2. Добавить `dl.name AS printer_location` в SELECT-список — **в КОНЕЦ** списка
   столбцов, **не в середину** (комментарий на line 28 явно предупреждает: `category_name`
   уже добавлен "LAST (idx 18) — never insert mid-list, it would shift every subsequent
   row.get(n)"). Новое поле должно встать **после** `category_name` (idx 19), либо явно
   пересчитать ВСЕ последующие индексы в `map_row_request`, если вставлять не в конец.
3. Обновить `map_row_request` симметрично: `printer_location: row.get(19)?,`.
4. NULL-safety уже встроена — `LEFT JOIN` на `printer_device_id IS NULL` корректно
   даёт `NULL` без паники (тот же паттерн, что уже работает для `printer_name`,
   Pitfall 5 из RESEARCH.md).

---

### `crates/trackly-app/src/services/request_service.rs` — `transition()` notes_json (+ cartridge snapshot)

**Analog:** тот же файл, существующая конструкция notes_json (lines 450-477)

**Текущий код** (lines 450-477):
```rust
RequestTransitionPayload::Complete {
    request_id,
    version,
    notes,
    linked_cartridge_id,
} => (
    *request_id,
    *version,
    RequestTransitionOp::Complete {
        notes: notes.clone(),
        linked_cartridge_id: linked_cartridge_id.map(|id| id as i64),
    },
    None,
    linked_cartridge_id.map(|id| id as i64),
),
};

let new_status = op.target_status().to_string();

// Carry the transition notes into the audit payload so the History
// block (REQ-07) can show the reject/complete reason. Create/accept
// have no notes → payload stays NULL.
let notes_json: Option<String> = match &op {
    RequestTransitionOp::Reject { notes } => notes.clone(),
    RequestTransitionOp::Complete { notes, .. } => notes.clone(),
    RequestTransitionOp::Accept => None,
}
.map(|n| serde_json::json!({ "notes": n }).to_string());
```

**Паттерн для D-07 (расширение JSON-снапшота):** при `Complete` с
`linked_cartridge_id.is_some()`, обогатить `notes_json` дополнительными полями
картриджа (код + модель), читая cartridge-репозиторий **до** записи транзакции (т.к.
после install картридж уже существует — этот код в `RequestService::transition`
выполняется ПОСЛЕ install-вызова с фронта, не внутри одной транзакции с ним, см.
архитектурную диаграмму RESEARCH.md). Образец `serde_json::json!({...})` уже
показывает идиому — расширить макрос дополнительными полями:
```rust
serde_json::json!({ "notes": n, "cartridgeCode": code, "cartridgeModel": model_label })
```
**Pitfall 3 (RESEARCH.md):** это осознанный снапшот-на-момент-события, не live-JOIN —
не "исправлять" позже как баг при последующем изменении картриджа (прецедент:
`act_items.condition_at_time`).

---

### `crates/trackly-app/src/services/request_service.rs` — `get_history()` parse (+ cartridge fields)

**Analog:** тот же файл, существующий `notes` JSON-parse (lines 188-207)

**Текущий код** (lines 188-207):
```rust
Ok(rows
    .into_iter()
    .map(|r| RequestHistoryEntryDto {
        id: r.id,
        action: r.action,
        created_at_utc: r.created_at_utc,
        actor_name: r.actor_name,
        // `notes` is carried in payload_json as {"notes": "..."} for
        // reject/complete transitions; absent for create/accept.
        notes: r.payload_json.as_deref().and_then(|p| {
            serde_json::from_str::<serde_json::Value>(p)
                .ok()
                .and_then(|v| {
                    v.get("notes")
                        .and_then(|n| n.as_str())
                        .map(|s| s.to_string())
                })
        }),
    })
    .collect())
```

**Паттерн:** скопировать тот же `.and_then(...).and_then(...)` идиому для нового
поля (например `cartridge_code`/`cartridge_model` либо единая человекочитаемая
строка, в зависимости от того, расширяет ли планировщик `RequestHistoryEntryDto`
новыми полями или просит фронт склеить строку из существующего `notes` JSON через
доп. поля внутри того же `notes` blob). Если выбран вариант "склеить готовую строку
на бэкенде" — проще всего расширить значение, кладущееся в `notes` JSON ключ, новым
явным текстом (`"Установлен C-000042 (HP CE285A)"`), а не добавлять новые DTO-поля —
тогда этот метод **не требует изменений вообще**, меняется только сборка
`notes_json` в `transition()` (см. выше). Планировщик должен явно зафиксировать,
какой вариант выбран (RESEARCH.md Open Question 2).

---

### `ui/src/features/cartridges/OperationModal.svelte` — добавление cartridge-селектора

**Analog:** тот же файл — install-branch уже существует (lines 246-301), нужно
вставить **перед** или **вместо** текущего `printerContextHint` блок выбора
конкретного картриджа.

**Текущий install-branch** (lines 246-301):
```svelte
{#if op === 'install' || op === 'to_refill'}
  {#if printerContextHint}
    <p class="field-hint">{printerContextHint}</p>
  {/if}
  <!-- Дата -->
  <div class="field">
    <label class="label" for="op-date">Дата</label>
    <DatePicker bind:value={dateIso} id="op-date" required />
  </div>
  <!-- Кто выдал -->
  ...
  <!-- Кому выдал -->
  <div class="field">
    <label class="label" for="op-given-to">Кому выдал</label>
    <PersonAutocomplete
      field="receiver"
      bind:value={givenToName}
      placeholder="ФИО получившего"
      id="op-given-to"
      invalid={!!givenToError}
    />
    ...
  </div>
  <!-- Расположение -->
  <div class="field">
    <label class="label" for="op-location">Расположение</label>
    <LocationAutocomplete
      value={location}
      placeholder="Расположение"
      id="op-location"
      invalid={!!locationError}
      onChange={(v) => (location = v)}
    />
    ...
  </div>
{:else if ...}
```

**Текущий props/state контракт** (lines 20-44):
```typescript
interface Props {
  open: boolean;
  op: Op;
  cartridge: CartridgeDto | null;
  /** Pre-fill the «Принтер» context when op='install' is opened from a request (REQ-05). */
  preFillPrinterId?: number;
  onClose: () => void;
  onSuccess: () => void;
}

const { open, op, cartridge, preFillPrinterId, onClose, onSuccess }: Props = $props();

let dateIso = $state('');
let givenByName = $state('');
let givenToName = $state('');
let location = $state('');
let stateId = $state(3);
let notes = $state('');
let submitting = $state(false);
```

**Текущий reset-эффект** (lines 57-74):
```typescript
$effect(() => {
  void op;
  if (open) {
    const now = new Date();
    ...
    givenByName = '';
    givenToName = '';
    location = '';
    notes = '';
    stateId = defaultStateId;
    locationError = '';
    givenByError = '';
    givenToError = '';
  }
});
```

**Паттерн для расширения:**
1. **Props:** заменить/дополнить `preFillPrinterId?: number` новыми пропами —
   `cartridgeModelId?: number` (для фильтра списка), `prefillLocation?: string`,
   `prefillGivenToName?: string` (по образцу D-04/D-05 — заявка несёт эти данные).
   `cartridge` prop остаётся (D-08: cartridge-centric вход продолжает передавать его
   напрямую) — но при request-centric входе он приходит как `null`, и модалка должна
   **сама** управлять внутренним `$state<CartridgeDto | null>` после выбора в новом
   селекторе (Pitfall 1: убедиться, что `cartridge` реактивно обновляется через
   `$state`, не залипает на пропе).
2. **Reset-эффект:** добавить `givenToName = prefillGivenToName ?? '';` и
   `location = prefillLocation ?? '';` вместо безусловного `''` — то же место (lines
   65-67), та же идиома, только с fallback на проп.
3. **Список картриджей:** загрузить через `$effect` при `open && op === 'install' &&
   !cartridge` (т.е. только когда модалка реально открыта в request-centric режиме
   без заранее заданного картриджа) — вызов `cartridges.list({ status_id: 1,
   installable_only: true, model_id: cartridgeModelId ?? null }, { offset: 0, limit:
   200 })` (тот же `api.ts` клиент, см. ниже).
4. **Рендер селектора:** вставить новый блок **внутри** существующего `{#if op ===
   'install' || op === 'to_refill'}` (но реально показывать только при `op ===
   'install' && !cartridge prop передан напрямую`, т.е. различать оба входа D-08)
   **до** блока «Дата», по образцу `GroupedPrinterSelect.svelte` скелета (см. ниже)
   — простой `<select>` без группировки достаточен (DISC-03), т.к. для картриджей
   нет естественной "локации"-группы как у принтеров.
5. **canSubmit/handleSubmit:** не менять форму (lines 208-233) — они уже корректно
   используют внутренний `cartridge` (через `$derived`/`!!cartridge`), просто
   убедиться, что это теперь internal `$state`, не constant prop.

---

### `ui/src/lib/components/GroupedPrinterSelect.svelte` — образец для нового CartridgeSelect

**Analog:** весь файл (149 строк) — скелет `<select>` + caret-иконка + SCSS уже
готовы, копировать стиль 1:1, упростив до флэт-списка (без `<optgroup>`).

**Props-контракт для копирования** (lines 13-29):
```typescript
interface Props {
  options: RequestPrinterOptionDto[];
  value: string;
  disabled?: boolean;
  invalid?: boolean;
  id?: string;
  onchange?: (_value: string) => void;
}

const {
  options,
  value = $bindable(''),
  disabled = false,
  invalid = false,
  id,
  onchange,
}: Props = $props();
```

**Рендер-скелет для адаптации** (lines 51-86, упрощённый без optgroup):
```svelte
<div class="select-wrapper">
  <select {id} {disabled} class="select" class:invalid {value} onchange={(e) => {
    const v = (e.currentTarget as HTMLSelectElement).value;
    onchange?.(v);
  }}>
    <option value="">Выберите картридж</option>
    {#if options.length === 0}
      <option value="" disabled>Нет подходящих картриджей на складе</option>
    {:else}
      {#each options as o (o.id)}
        <option value={String(o.id)}>{o.code} — {o.modelLabel} ({o.stateLabel})</option>
      {/each}
    {/if}
  </select>
  <!-- caret icon — copy lines 76-85 verbatim -->
</div>
```
**SCSS** (lines 88-149) — копировать целиком без изменений (design tokens
`--color-bg`, `--color-border`, `--radius-sm` и т.п. уже используются проектно-широко).

---

### `ui/src/features/requests/RequestDetail.svelte` — проброс данных заявки + linkedCartridgeId

**Analog:** тот же файл, `handleInstallSuccess()` (lines 299-323) и OperationModal JSX (lines 581-590)

**Текущий (дефектный) код** (lines 299-323):
```typescript
async function handleInstallSuccess() {
  if (!request) return;
  operationModalOpen = false;
  try {
    await requests.transition({
      op: 'complete',
      requestId: request.id,
      version: request.version,
      notes: null,
      linkedCartridgeId: null,
    });
    pushToast('success', 'Заявка выполнена');
    onTransition();
  } catch (e: unknown) {
    const msg = ...;
    pushToast('error', msg);
    onTransition();
  }
}
```

**Текущий проброс пропов** (lines 581-590):
```svelte
{#if request !== null}
  <OperationModal
    open={operationModalOpen}
    op="install"
    cartridge={null}
    preFillPrinterId={request.printerDeviceId ?? undefined}
    onClose={() => (operationModalOpen = false)}
    onSuccess={handleInstallSuccess}
  />
{/if}
```

**Паттерн для D-06:**
1. `handleInstallSuccess` должен принять параметр (id установленного картриджа),
   например `handleInstallSuccess(cartridgeId: number)`, и передать его в
   `linkedCartridgeId: cartridgeId` вместо `null` — единственная необходимая правка
   к существующей форме вызова (структура try/catch/pushToast/onTransition не меняется).
2. `OperationModal`'s `onSuccess` callback prop (interface `Props.onSuccess: () =>
   void` в OperationModal.svelte line 27) должен **сменить сигнатуру** на `(_cartridgeId:
   number) => void`, чтобы `RequestDetail` мог получить id — это связанная правка на
   обеих сторонах контракта (OperationModal.svelte Props + handleSubmit success-путь,
   lines 213-217: `onSuccess()` → `onSuccess(updatedCartridge.id)`).
3. Проброс новых пропов в JSX (вместо/вместе с `preFillPrinterId`):
   `cartridgeModelId={request.cartridgeModelId ?? undefined}`,
   `prefillLocation={request.printerLocation ?? undefined}` (новое поле из DTO,
   см. backend раздел выше), `prefillGivenToName={request.requesterName ?? undefined}`.

---

### `ui/src/features/cartridges/api.ts` — `list()` принимает `installable_only`

**Analog:** тот же файл (lines 22-36) — сигнатура не меняется (типы приходят из
регенерированного `CartridgeFilter` в `bindings.ts`), только вызывающий код в
OperationModal передаёт новое поле объекта.
```typescript
export const cartridges = {
  list: (filter: CartridgeFilter, pagination: Pagination) =>
    apiCall<CartridgeListResponse>('cartridges_list', { filter, pagination }),
  ...
  transition: (payload: CartridgeTransitionPayload) =>
    apiCall<CartridgeDto>('cartridges_transition', { payload }),
  ...
};
```
Никаких правок в `api.ts` не требуется — после регенерации `bindings.ts` TS-тип
`CartridgeFilter` уже несёт новое поле, вызывающий код просто передаёт его в
объекте литерале.

---

### `crates/trackly-app/tests/cartridges_lifecycle.rs` — новый тест на installable-фильтр

**Analog:** тот же файл, `make_cartridge_service()`/`seed_model()`/
`create_stock_cartridge()` helpers + `install_changes_status_to_in_use` (lines 1-86)

**Helper-паттерн для копирования** (lines 22-56):
```rust
fn make_cartridge_service() -> (CartridgeService, tempfile::TempDir) {
    let (writer, readers, dir) = test_writer_and_readers();
    let clock = Arc::new(SystemClock);
    let svc = CartridgeService::new(writer, readers, clock);
    (svc, dir)
}

async fn seed_model(svc: &CartridgeService) -> i64 {
    svc.model_create(CartridgeModelCreateDto {
        brand: "HP".into(),
        model: "CE285A".into(),
        kind_id: 1,
        color: Some("Чёрный".into()),
        notes: None,
        compatibility: vec![],
    })
    .await
    .expect("seed model")
    .id
}

async fn create_stock_cartridge(
    svc: &CartridgeService,
    model_id: i64,
) -> trackly_app::dto::cartridge::CartridgeDto {
    svc.create(CartridgeCreateDto {
        model_id,
        code_override: None,
        state_id: Some(1), // Полный
        location: Some("Склад".into()),
        notes: None,
    })
    .await
    .expect("create cartridge")
}
```
**Test-skeleton-паттерн** (lines 58-86, `#[tokio::test(flavor = "multi_thread",
worker_threads = 4)]` + `tokio::time::timeout(Duration::from_secs(30), async { ...
}).await.expect(...)`) — обязательно копировать этот timeout-wrapper (проектная
конвенция против зависших тестов, видна во всех тестах файла).

**Новый тест должен:** создать 2-3 картриджа с разными `state_id` (1, 2, 3) +
разными `model_id`, вызвать `svc.list(CartridgeFilter { status_id: Some(1),
installable_only: true, model_id: Some(target_model), ..Default::default() },
Pagination::default())`, проверить что результат содержит только state∈{1,2} И
этой модели. Покрыть DISC-01 (model_id: None → без фильтра модели) и DISC-02
(пустой результат → `Ok((vec![], 0))`, не ошибка) отдельными `#[tokio::test]` fn.

---

### `crates/trackly-app/tests/phase06_stubs.rs` — реализация `test_req_cart_link`

**Analog:** тот же файл, `test_request_lifecycle()` (lines 241-343) — даёт полный
шаблон seed user → `RequestService::new(...)` → `create()` → `transition()` →
`assert_eq!`.

**Текущий ignored stub** (lines 575-578):
```rust
/// REQ-05: Complete{linked_cartridge_id} записывает completed_cartridge_id
#[test]
#[ignore]
fn test_req_cart_link() {}
```

**Паттерн для реализации** — скопировать структуру `test_request_lifecycle` (seed
user через `writer.execute`, `Identity { user_id: Some(1), role: Role::Admin }`,
`RequestService::new(writer.clone(), readers, clock.clone(), ws_tx)`), затем:
1. Сделать `svc.create(...)` с `request_type: "cartridge_replace"`.
2. `svc.transition(Accept {...})` → `in_progress`.
3. (Опционально, если тест должен покрыть И install, И complete) создать
   `CartridgeService` параллельно на том же `writer`/`readers`, выполнить install-
   transition, получить `cartridge.id`.
4. `svc.transition(Complete { request_id, version, notes: None,
   linked_cartridge_id: Some(cartridge_id as i32) }, &caller)`.
5. `assert_eq!(completed.completed_cartridge_id, Some(cartridge_id))`.
6. Изменить `#[test]` + `#[ignore]` на `#[tokio::test(flavor = "multi_thread",
   worker_threads = 2)]` (метод async, `#[test]`+`#[ignore]` сейчас — placeholder
   синтаксис, не реальный async test attribute; нужно заменить полностью, не просто
   снять `#[ignore]`).

---

## Shared Patterns

### Optimistic locking / version-based concurrency
**Source:** `crates/trackly-infra/src/repos/requests_sqlite.rs::transition_in_tx` (lines 115-184), `crates/trackly-infra/src/repos/cartridges_sqlite.rs` transition (lines 337-444)
**Apply to:** оба write-пути (`cartridges.transition`, `requests.transition`) — уже не требуют изменений, фаза только проводит существующий контракт. Любой новый код **не должен** обходить `version`-проверку.
```rust
if current.version != version {
    return Err(AppError::OptimisticLockMismatch { entity: "request", id: request_id, expected: version, actual: current.version });
}
```

### NULL-safe LEFT JOIN → Option<String>
**Source:** `crates/trackly-app/src/dto/request.rs::RequestPrinterOptionDto` (lines 99-108), уже используемый для `printer_name`
**Apply to:** новое поле `printer_location` — тот же `Option<String>`, никаких `unwrap()`/`expect()`.

### Audit payload_json snapshot (история)
**Source:** `crates/trackly-app/src/services/request_service.rs` lines 469-477 (notes_json construction) + lines 188-207 (parse)
**Apply to:** расширение для D-07 — снапшот картриджа в том же JSON blob, не отдельная таблица/JOIN.

### RBAC gate reuse — НЕ создавать новый Action
**Source:** `crates/trackly-core/src/auth.rs` `Action::ReadData`/`MutateCartridges`/`TransitionRequests` (уже используются в существующих `authorize()` вызовах cartridges/request сервисов)
**Apply to:** новый/расширенный список картриджей-для-установки — **обязательно** `Action::ReadData` (НЕ `Action::CreateRequest`, который специально открыт Employee для `printer_options()` — копирование этого гейта на картриджный эндпоинт было бы BFLA-регрессией, явно отмеченной как security risk в RESEARCH.md Anti-Patterns).

### Wire-contract camelCase pitfall (НЕ ТРОГАТЬ при добавлении данных в Complete)
**Source:** `crates/trackly-app/src/dto/request.rs` lines 191-220 (`RequestTransitionPayload` enum + per-variant `#[serde(rename_all = "camelCase")]`), закреплено тестом `complete_deserializes_camel_case_wire_format` (wire_contract_tests модуль, упомянут в RESEARCH.md, не прочитан целиком в этой сессии)
**Apply to:** `linked_cartridge_id` уже существует в `Complete` варианте — фронт уже обязан слать его как `linkedCartridgeId` (camelCase). Никаких новых serde-атрибутов добавлять не нужно для этой фазы — только передавать реальное значение вместо `null`.

## No Analog Found

Все файлы фазы — правки существующего кода с прямым аналогом в том же файле/модуле.
Единственная зона с неполной уверенностью:

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `crates/trackly-app/tests/role_endpoint_matrix.rs` (возможное расширение для `cartridges_transition`/`requests_transition`) | test | request-response (RBAC matrix) | Подтверждено прямым grep: файл покрывает `cartridges_list` (Cases 13/14, lines 329-639) под Employee/Manager, но **не содержит** ни `cartridges_transition`, ни `requests_transition` ни в одном кейсе — RBAC для этих двух команд **не покрыт** существующей матрицей. Планировщик должен явно решить: добавить новые Case-записи в этот файл (копируя структуру Case 13/14) либо явно отложить как известный gap. Нет прямого "образца" внутри файла для этих двух конкретных команд — нужно скопировать структуру существующих read-кейсов и адаптировать под mutating-эндпоинты (нужен валидный payload + существующая сущность, не просто пустой POST). |

## Metadata

**Analog search scope:** `crates/trackly-core/src/domain/`, `crates/trackly-app/src/dto/`,
`crates/trackly-app/src/services/`, `crates/trackly-app/src/tauri_cmds/`,
`crates/trackly-app/src/http/`, `crates/trackly-infra/src/repos/`,
`crates/trackly-app/tests/`, `ui/src/features/cartridges/`, `ui/src/features/requests/`,
`ui/src/lib/components/`

**Files scanned:** `domain/cartridges.rs`, `dto/cartridge.rs`, `dto/request.rs`,
`services/request_service.rs`, `repos/cartridges_sqlite.rs`, `repos/requests_sqlite.rs`,
`http/cartridges.rs`, `tauri_cmds/cartridges.rs` (grep only), `tauri_cmds/requests.rs`
(grep only), `OperationModal.svelte`, `RequestDetail.svelte`, `GroupedPrinterSelect.svelte`,
`PersonAutocomplete.svelte` (grep only), `LocationAutocomplete.svelte` (grep only),
`api.ts` (both), `cartridges_lifecycle.rs`, `phase06_stubs.rs`, `role_endpoint_matrix.rs` (grep only)

**Pattern extraction date:** 2026-06-22
