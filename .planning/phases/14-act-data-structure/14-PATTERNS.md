# Phase 14: Данные и структура акта - Pattern Map

**Mapped:** 2026-07-03
**Files analyzed:** 8 (1 new, 7 modified)
**Analogs found:** 8 / 8

> Фаза DATA/SCHEMA/CONTEXT-ONLY. Визуальный рендер PDF под образец Word — Phase 15.
> Работа: (1) новая миграция org_settings +5 колонок; (2) сквозной путь этих полей
> repo→service→DTO→HTTP+Tauri→UI; (3) расхардкодить `specs: Null` в контексте акта
> (device.notes); (4) расширить `HeaderBlock` реквизитами.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `migrations/V033__org_settings_requisites.sql` (**NEW**) | migration | schema | `migrations/V026__org_settings.sql` | role-match (ADD COLUMN vs CREATE) |
| `crates/trackly-app/src/dto/reports.rs` | DTO | request-response | self (`OrgPatch`/`OrgSettingsDto` L176-209) | exact |
| `crates/trackly-app/src/services/org_db_service.rs` | service | CRUD | self (`get`/`save_fields` L51-106, `get_for_pdf` L351-383) | exact |
| `crates/trackly-app/src/http/settings_org.rs` | route | request-response | self (`handler_save_org_fields` L176-189) | exact (no change likely) |
| `crates/trackly-app/src/tauri_cmds/settings_org.rs` | command | request-response | self (`settings_save_org_fields` L303-311) | exact (no change likely) |
| `ui/src/features/settings/OrgSettings.svelte` | component | request-response | self (existing INN/KPP fields L211-225) | exact |
| `crates/trackly-app/src/pdf/docspec.rs` | model | transform | self (`HeaderBlock` L34-53) | exact |
| `crates/trackly-app/src/services/act_service.rs` | service | transform | self (`load_items_for_act` L1681-1720, `render_pdf` L1346-1388) | exact |

---

## Pattern Assignments

### `migrations/V033__org_settings_requisites.sql` (NEW — migration, schema)

**Analog:** `migrations/V026__org_settings.sql`

V026 already documents the exact ADD-COLUMN-safe pattern in its header comment
(`NOT NULL DEFAULT '...' on textual fields — allows future ALTER TABLE ADD COLUMN
without risking NULL in historic rows`). V033 is that future migration.

**Column definition pattern to copy** (V026 L17-28 — TEXT NOT NULL DEFAULT ''):
```sql
CREATE TABLE org_settings (
    id              INTEGER  NOT NULL PRIMARY KEY CHECK (id = 1),
    org_name        TEXT     NOT NULL DEFAULT 'Ваша организация',
    inn             TEXT     NOT NULL DEFAULT '0000000000',
    kpp             TEXT     NOT NULL DEFAULT '000000000',
    address         TEXT     NOT NULL DEFAULT 'Адрес не указан',
    ...
    version         INTEGER  NOT NULL DEFAULT 1
);
```

**PRAGMA user_version footer pattern** (V026 L34 — sequential, next free = 33):
```sql
PRAGMA user_version = 26;
```

**What V033 must do** (5 new columns, per CONTEXT D-02, `TEXT NOT NULL DEFAULT ''`):
```sql
-- V033: org_settings extended requisites (PDFA-03).
ALTER TABLE org_settings ADD COLUMN phone TEXT NOT NULL DEFAULT '';
ALTER TABLE org_settings ADD COLUMN fax   TEXT NOT NULL DEFAULT '';
ALTER TABLE org_settings ADD COLUMN email TEXT NOT NULL DEFAULT '';
ALTER TABLE org_settings ADD COLUMN okpo  TEXT NOT NULL DEFAULT '';
ALTER TABLE org_settings ADD COLUMN ogrn  TEXT NOT NULL DEFAULT '';

PRAGMA user_version = 33;
```

**Notes for planner:**
- Next free migration number is **V033** (last is `V032__cartridge_model_compatibility_printer_name.sql`).
- `DEFAULT ''` (empty string) not the placeholder strings V026 used for name/inn — new
  fields degrade to «—»/empty, per CONTEXT `<specifics>` (missing requisites → пусто, not error).
- Refinery auto-applies at startup; sequential `user_version`. `downgrade_protection` test
  in `crates/trackly-app/tests/downgrade_protection.rs` and idempotency test in
  `crates/trackly-infra/tests/migration_idempotency.rs` will exercise the new migration.

---

### `crates/trackly-app/src/dto/reports.rs` (DTO, request-response)

**Analog:** self — `OrgPatch` and `OrgSettingsDto` already define the field set.

**Current OrgPatch** (L176-182) — add `phone/fax/email/okpo/ogrn: String`:
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct OrgPatch {
    pub org_name: String,
    pub inn: String,
    pub kpp: String,
    pub address: String,
}
```

**Current OrgSettingsDto** (L200-209) — add same 5 fields (read path):
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct OrgSettingsDto {
    pub org_name: String,
    pub inn: String,
    pub kpp: String,
    pub address: String,
    pub has_logo: bool,
}
```

**Notes:** `#[derive(..., Type)]` is `specta::Type` — adding fields auto-propagates to
`ui/src/bindings.ts` on bindings regen. Keep field names snake_case (serde default here).

---

### `crates/trackly-app/src/services/org_db_service.rs` (service, CRUD)

**Analog:** self — 3 SQL sites all list the org columns explicitly and must gain the 5 new ones.

**Read SELECT + row map** (`get`, L54-70):
```rust
conn.query_row(
    "SELECT org_name, inn, kpp, address, \
     (logo_blob IS NOT NULL) as has_logo \
     FROM org_settings WHERE id = 1",
    [],
    |r| {
        Ok(OrgSettingsDto {
            org_name: r.get(0)?,
            inn: r.get(1)?,
            kpp: r.get(2)?,
            address: r.get(3)?,
            has_logo: r.get::<_, bool>(4)?,
        })
    },
)
```

**Write UPDATE + params** (`save_fields`, L86-104):
```rust
self.writer.execute(move |conn| {
    conn.execute(
        "UPDATE org_settings \
         SET org_name=?2, inn=?3, kpp=?4, address=?5, \
             updated_at_utc=?6, version=version+1 \
         WHERE id=1",
        params![1i64, patch.org_name, patch.inn, patch.kpp, patch.address, now],
    )
    .map(|_| ()).map_err(map_rusqlite)
}).await
```

**PDF read tuple** (`get_for_pdf`, L358-375) — **critical for Phase 15 header data**:
```rust
conn.query_row(
    "SELECT org_name, inn, kpp, address, \
     (logo_blob IS NOT NULL) as has_logo, \
     logo_blob, logo_mime \
     FROM org_settings WHERE id = 1",
    [],
    |r| { let dto = OrgSettingsDto { org_name: r.get(0)?, ... }; ... },
)
```

**Notes:** Three call sites to extend consistently (`get` L54, `save_fields` L86,
`get_for_pdf` L358). Column ordinal shifts — append new columns at the end of each SELECT
to avoid reindexing existing `r.get(N)` positions. `authorize(caller, &Action::ManageSettings)?`
guard on all mutations (L84) stays.

---

### `crates/trackly-app/src/http/settings_org.rs` (route, request-response)

**Analog:** self — `handler_save_org_fields` (L176-189). **Likely NO code change** — the
handler passes `OrgPatch` through opaquely; new DTO fields flow automatically.

**Handler pattern (unchanged, for reference)** (L176-189):
```rust
pub async fn handler_save_org_fields(
    State(ctx): State<AppCtx>,
    session: Session,
    Json(p): Json<SaveOrgFieldsPayload>,
) -> Result<Json<()>, AppErrorResponse> {
    let caller = session_identity(&session).await.map_err(AppErrorResponse::from)?;
    authorize(&caller, &Action::ManageSettings).map_err(AppErrorResponse::from)?;
    build_settings_save_org_fields(&ctx, &caller, p.patch).await.map_err(AppErrorResponse::from)?;
    Ok(Json(()))
}
```

**Notes:** Route `/api/v1/settings_save_org_fields` (L338) unchanged. Only touch this file
if a validation rule for new fields is desired at the HTTP boundary (not required).

---

### `crates/trackly-app/src/tauri_cmds/settings_org.rs` (command, request-response)

**Analog:** self — `build_settings_save_org_fields` (L28-34) + `settings_save_org_fields`
Tauri command (L303-311). **Likely NO code change** — passes `OrgPatch` opaquely.

**Tauri command pattern (unchanged, for reference)** (L303-311):
```rust
#[tauri::command]
#[specta::specta]
pub async fn settings_save_org_fields(
    state: tauri::State<'_, AppCtx>,
    patch: OrgPatch,
) -> Result<(), AppError> {
    let caller = resolve_tauri_identity(state.inner()).await?;
    build_settings_save_org_fields(state.inner(), &caller, patch).await
}
```

**Notes:** `#[specta::specta]` + `Type`-derived DTOs feed bindings gen. After DTO change,
**regen bindings**: `cargo test -p trackly-app --test export_bindings`
(ui/package.json `prebuild` step also runs it). Verify `ui/src/bindings.ts` picks up new fields.

---

### `ui/src/features/settings/OrgSettings.svelte` (component, request-response)

**Analog:** self — existing INN/KPP input fields + `saveOrg()` payload.

**Local state + load pattern** (L8-47) — add `phone/fax/email/okpo/ogrn` `$state('')`:
```typescript
interface OrgSettingsDto { org_name: string; inn: string; kpp: string; address: string; has_logo: boolean; }
let inn = $state(''); let kpp = $state('');
async function loadOrg() {
    const dto = await apiCall<OrgSettingsDto>('settings_get_org', {});
    orgName = dto.org_name; inn = dto.inn; kpp = dto.kpp; address = dto.address; ...
}
```

**Save payload pattern** (L75-82) — extend `patch` with new fields:
```typescript
await apiCall<void>('settings_save_org_fields', {
    patch: { org_name: orgName, inn, kpp, address },
});
```

**Input field markup pattern** (L211-225 — copy for each new requisite):
```svelte
<div class="form-field">
  <label class="form-label" for="org-inn">ИНН</label>
  <input id="org-inn" class="form-input" type="text" bind:value={inn} placeholder="0000000000" />
</div>
```

**Notes:** Svelte 5 runes (`$state`). `.form-grid` is 2-column (L306-311); place новые поля
как обычные `.form-field` (телефон/факс/email/ОКПО/ОГРН — короткие, half-width). Русские
лейблы: «Телефон», «Факс», «E-mail», «ОКПО», «ОГРН». `apiCall` client works in both Tauri
and LAN-browser (dual transport). **Rebuild ui/dist for browser testing**:
`pnpm --dir ui build`.

---

### `crates/trackly-app/src/pdf/docspec.rs` (model, transform)

**Analog:** self — `HeaderBlock` struct (L34-53).

**Current HeaderBlock** (L34-53) — add `org_phone/org_fax/org_email/org_okpo/org_ogrn`:
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct HeaderBlock {
    pub org_name: String,
    pub org_inn: String,
    pub org_kpp: String,
    pub org_address: String,
    pub logo_path: Option<String>,
    #[serde(default)]
    pub logo_bytes: Option<Vec<u8>>,
    #[serde(default)]
    pub logo_mime: Option<String>,
    pub act_label: String,
    pub date_label: String,
}
```

**Backward-compat pattern to reuse** (L31-32, L44/L47): use `#[serde(default)]` on the new
fields so existing templates that omit them still deserialize (`RESEARCH Pitfall 7`).

**Notes:** `HeaderBlock` is populated two ways —
(a) directly in Rust (`report_service.rs` L522-532, `template_service.rs` L318),
(b) deserialized from MiniJinja template JSON output (act path, see below).
Both must be able to omit/supply the new fields; `#[serde(default)]` covers (b).

---

### `crates/trackly-app/src/services/act_service.rs` (service, transform) — THE CORE CHANGE

**Analog:** self. Two edits: (1) carry `device.specs`(=`notes`) into item context;
(2) add requisites to the `org` render context.

#### Edit 1 — specs↔notes (D-01, the single data gap)

**The hardcode to replace** (`render_pdf`, L1346-1361):
```rust
let items_json: Vec<serde_json::Value> = act.items.iter().map(|it| {
    serde_json::json!({
        "name": it.device_name,
        "inventory_no": it.inventory_no,
        "serial_no": it.serial_no,
        "model": it.model,
        "specs": serde_json::Value::Null,      // ← replace with it.specs
        "kit": it.complectation_at_time,
        "condition": it.condition_at_time,
        "quantity": it.quantity,
    })
}).collect();
```

**Where `act.items` (ActItemDto) is built — SELECT to extend** (`load_items_for_act`, L1685-1706):
```rust
let mut stmt = conn.prepare(
    "SELECT ai.id, ai.device_id, ai.quantity, ai.condition_at_time, ai.complectation_at_time, \
            d.name, d.inventory_number, d.serial_number, d.model \
       FROM act_items ai \
       JOIN devices d ON d.id = ai.device_id \
      WHERE ai.act_id = ?1 ORDER BY ai.id ASC",
)?;
// ... query_map -> ActItemDto { ..., device_name: r.get(5)?, inventory_no: r.get(6)?,
//                                     serial_no: r.get(7)?, model: r.get(8)?, ... }
```

**The specs↔notes mapping fact** (`devices_sqlite.rs` L9-11):
```
//! - DTO `specs`  ↔  DB `notes`
//! - DTO `kit`    ↔  DB `complectation`
//! - DTO `state`  ↔  DB `condition`
```

**Change recipe (Claude's Discretion per CONTEXT D-01 — recommended shape):**
1. Add `pub specs: Option<String>` to `ActItemDto` (`crates/trackly-app/src/dto/act.rs`
   L92-109, right after `model`). Derives `Type` → binding auto-updates.
2. In `load_items_for_act`: add `d.notes` to SELECT (last column, index 9) and
   `specs: r.get(9)?` to the `ActItemDto { ... }` construction.
3. In `render_pdf` items_json: `"specs": it.specs,` (was `Value::Null`).
- **Live value, not snapshot** (D-01): reads current `devices.notes`, no `specs_at_time`
  column added to `act_items`. `kit`/`condition` stay as point-in-time snapshots
  (`complectation_at_time`/`condition_at_time`) — do NOT touch those.
- **Backward-compat:** `d.notes` is nullable → `Option<String>` → template degrades to «—».

#### Edit 2 — requisites into org context (D-02)

**Current org context block** (`render_pdf`, L1363-1370):
```rust
"org": {
    "name": org.name,
    "inn": org.inn,
    "kpp": org.kpp,
    "address": org.address,
    "logo_path": safe_logo.map(|p| p.display().to_string()),
},
```

**⚠ ARCHITECTURAL FORK for planner — org data source mismatch:**
The act render path reads `org` via `pipeline.organization.read()` →
`OrganizationService::read()` which loads **`org.json`** (`organization_service.rs`
L66-96, struct `OrgData` L24-32: `name/inn/kpp/address/logo_path` only).
But the **Settings UI writes requisites to the `org_settings` DB table** via `OrgDbService`.
These are two different stores. To surface the new requisites in the act PDF context, the
planner must pick one:
- **Option A:** switch act render to `OrgDbService::get_for_pdf()` (L351-383) which already
  reads the DB table — extend its SELECT with the 5 columns and map into the context.
  Cleaner, single source of truth, but changes the act render data source.
- **Option B:** also add the 5 fields to `OrgData` + org.json + its migrate path. Keeps
  org.json as act source but duplicates the requisites store. Not recommended.
- Recommendation: **Option A** (DB is the source Settings UI already writes to). Note
  `report_service.rs` L522 already builds `HeaderBlock` from `OrgSettingsDto` (DB) — this is
  the precedent for reading requisites from DB into a header.

**Also update `render_acceptance_pdf`** (org block ~L1468, same `"address": org.address` shape)
if that document also needs requisites — CONTEXT scopes semantics to `act_handover` (D-03),
so acceptance may be left as-is; confirm in plan.

---

## Shared Patterns

### SQLite write path (single writer)
**Source:** `org_db_service.rs` L86-104 / `act_service.rs` load helpers
**Apply to:** all mutations
```rust
self.writer.execute(move |conn| {
    conn.execute("UPDATE ... SET ..., version=version+1 WHERE id=1", params![...])
        .map(|_| ()).map_err(map_rusqlite)
}).await
```
Reads use `spawn_blocking` + `readers.acquire()` (org_db_service L52-71). Do not open
raw connections; funnel through `WriterHandle`/`ReaderPool`.

### Authorization guard on settings mutations
**Source:** `org_db_service.rs` L84, `http/settings_org.rs` L184, `tauri_cmds/settings_org.rs` L309
**Apply to:** any new mutation touching org_settings
```rust
authorize(caller, &Action::ManageSettings)?;   // service layer
// HTTP: session_identity(&session).await? then authorize(&caller, &Action::ManageSettings)?
// Tauri: let caller = resolve_tauri_identity(state.inner()).await?;
```

### Specta bindings regen (dual transport contract)
**Source:** `ui/package.json` prebuild, `crates/trackly-app/tests/export_bindings.rs`
**Apply to:** any DTO (`#[derive(Type)]`) or `#[specta::specta]` command change
```
cargo test -p trackly-app --test export_bindings    # regenerates ui/src/bindings.ts
pnpm --dir ui build                                  # rebuild ui/dist for LAN-browser testing
```

### Migration test coverage
**Source:** `crates/trackly-app/tests/downgrade_protection.rs`,
`crates/trackly-infra/tests/migration_idempotency.rs`
**Apply to:** V033 — sequential `user_version`, idempotent re-apply, no downgrade.

---

## No Analog Found

None — every file has a strong self-analog in the existing codebase (org_settings and the
act render pipeline are both mature). No RESEARCH-only fallback needed.

---

## Metadata

**Analog search scope:** `migrations/`, `crates/trackly-app/src/{dto,services,http,tauri_cmds,pdf}/`,
`crates/trackly-infra/src/repos/`, `crates/trackly-core/src/domain/`, `ui/src/features/settings/`
**Files scanned:** ~15
**Pattern extraction date:** 2026-07-03
