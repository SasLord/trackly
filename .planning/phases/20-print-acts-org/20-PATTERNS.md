# Phase 20: Печать актов и организация - Pattern Map

**Mapped:** 2026-07-13
**Files analyzed:** 8 (2 modified backend services, 1 DTO file, 1 migration, 3 templates, 1 Svelte component)
**Analogs found:** 8 / 8 — this phase is entirely "extend existing pattern," no research was needed and no file lacks a strong in-repo analog.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `migrations/V035__org_settings_address_line2.sql` (new, exact number TBD — see Metadata) | migration | batch (schema change) | `migrations/V033__org_settings_requisites.sql` | exact — same table, same `ALTER TABLE ADD COLUMN ... DEFAULT ''` pattern |
| `crates/trackly-app/src/services/act_service.rs::render_acceptance_pdf` (modify) | service (render) | request-response / transform | `act_service.rs::render_pdf` (same file, same struct) | exact — this IS the target-pattern-to-copy-from |
| `crates/trackly-app/src/services/org_db_service.rs` (`get`/`save_fields`/`get_for_pdf`, modify SQL) | service (CRUD) | CRUD | itself (extend existing SQL, same file) | exact — mechanical column addition to existing SELECT/UPDATE lists |
| `crates/trackly-app/src/dto/reports.rs` (`OrgSettingsDto`, `OrgPatch`, modify) | model (DTO) | transform | itself (extend existing structs, same file) | exact — same file already has 5 analogous "extended requisite" fields (phone/fax/email/okpo/ogrn) added in Phase 14 |
| `crates/trackly-app/templates/act_acceptance.html` (`.requisites` block, modify) | template | transform | `crates/trackly-app/templates/act_handover.html` `.requisites` block | exact — verbatim block to copy, extended with `address_line2` |
| `crates/trackly-app/templates/act_handover.html` (add `address_line2` line, modify) | template | transform | itself (existing `.requisites` block, same file) | exact — one line insertion following the field's own established `{% if %}` idiom |
| `crates/trackly-app/templates/report.html` (add `address_line2` line, modify) | template | transform | `act_handover.html` `.requisites` block (report.html's header is already "copied verbatim from act_handover.html" per its own doc-comment) | exact |
| `ui/src/features/settings/OrgSettings.svelte` (add `address_line2` field, modify) | component (form) | CRUD | itself (existing `address` field in same form, same file) | exact — same input pattern, same state/load/save wiring |
| `crates/trackly-app/tests/html_act_render.rs` (add D-09 regression test, modify/extend) | test | transform | `html_handover_contains_required_blocks_and_logo` / `html_is_offline_safe_no_external_links` (same file) | exact — same pipeline fixture (`make_full_pipeline`), same `OrgDbService::save_logo` production-path pattern, same "assert HTML contains X, not Y" style |

**No transport-adapter changes needed:** `crates/trackly-app/src/tauri_cmds/settings_org.rs` and `crates/trackly-app/src/http/settings_org.rs` are pure pass-through (`ctx.org_db.get().await`, `ctx.org_db.save_fields(caller, patch).await`) — they forward whatever fields exist on `OrgSettingsDto`/`OrgPatch` with zero per-field logic, so adding `address_line2` to the DTOs requires **no code change** in these two files, only `bindings.ts` regeneration via the existing `export_bindings` test. `ui/src/features/devices/DevicesPage.svelte` (the "Печать документа приёма" entry point) calls the existing `render_acceptance_pdf`/print flow unchanged — no modification expected there either; it is listed in canonical_refs only as the feature's launch point, not as a file to edit.

---

## Pattern Assignments

### `migrations/V035__org_settings_address_line2.sql` (migration, batch)

**Analog:** `migrations/V033__org_settings_requisites.sql` (exact — same table `org_settings`, same "add optional textual requisite" shape)

**Full analog file** (25 lines, reproduced — this is the template to copy verbatim and adapt):
```sql
-- V033: Organisation extended requisites (PDFA-03).
--
-- Adds phone/fax/email/okpo/ogrn columns to org_settings so the act header
-- can display the full set of requisites required by the Word-fidelity
-- sample (see .planning/PHASE-BRIEF-act-pdf-word-fidelity.md).
--
-- Design decision (14-CONTEXT D-02): DEFAULT '' (empty string), NOT the
-- placeholder strings V026 used for name/inn/kpp — missing requisites on
-- historic rows must degrade to empty/"—" in rendered documents, not to a
-- misleading placeholder value. NOT NULL preserves the ADD-COLUMN-safe
-- pattern documented in V026 (no NULL in historic rows).
--
-- Appended at the end of the column list — existing SELECT/UPDATE ordinal
-- positions in org_db_service.rs are unaffected; new columns are added last
-- in every SQL site touching org_settings.
--
-- PRAGMA user_version = 33 (sequential; downgrade_protection test covers it).

ALTER TABLE org_settings ADD COLUMN phone TEXT NOT NULL DEFAULT '';
ALTER TABLE org_settings ADD COLUMN fax   TEXT NOT NULL DEFAULT '';
ALTER TABLE org_settings ADD COLUMN email TEXT NOT NULL DEFAULT '';
ALTER TABLE org_settings ADD COLUMN okpo  TEXT NOT NULL DEFAULT '';
ALTER TABLE org_settings ADD COLUMN ogrn  TEXT NOT NULL DEFAULT '';

PRAGMA user_version = 33;
```

**Adaptation for this phase (D-04, D-10):** single column, matching the same `DEFAULT ''`-not-placeholder rationale (V033's doc-comment rationale applies verbatim — D-04 explicitly says `TEXT NOT NULL DEFAULT ''`):
```sql
-- V0XX: Organisation address second line (ORG-02).
--
-- Adds address_line2 to org_settings for a free-form second address line
-- (e.g. "офис 305, корпус 2") displayed under the main address in all
-- printed documents (act_handover.html, act_acceptance.html, report.html).
--
-- Design decision (20-CONTEXT D-04): DEFAULT '' (empty string) — same
-- rationale as V033: historic rows degrade to "no second line shown"
-- (D-06's `{% if %}` guard), never a misleading placeholder.
--
-- Appended at the end of the column list — existing SELECT/UPDATE ordinal
-- positions in org_db_service.rs are unaffected.
--
-- PRAGMA user_version = 0XX.

ALTER TABLE org_settings ADD COLUMN address_line2 TEXT NOT NULL DEFAULT '';

PRAGMA user_version = 0XX;
```

**Next migration number:** `V034__return_handover_date_backfill.sql` is the current latest (see Metadata) — the new migration MUST be `V035__...sql` with `PRAGMA user_version = 35`.

---

### `crates/trackly-app/src/services/act_service.rs::render_acceptance_pdf` (service, request-response) — THE central change of this phase

**Analog:** `render_pdf` in the same file/struct (lines 2536-2670) — this is the эталон (gold standard) that `render_acceptance_pdf` (lines 2678-2784) must be brought to parity with.

**Current deficient acceptance implementation** (lines 2678-2784, the exact code to replace):
```rust
pub async fn render_acceptance_pdf(
    &self,
    device_id: i64,
    giver_name: String,
    receiver_name: String,
    date_utc: i64,
) -> Result<String, AppError> {
    let pipeline = self.pdf_pipeline()?;
    let org = pipeline.organization.read().await?;
    // Phase 16 (D-11): legacy org.json logo has no BLOB storage — read the
    // canonicalized local file's bytes (path-traversal-guarded via
    // safe_logo_canonical) and embed as a base64 data: URI.
    let logo_data_uri =
        pipeline
            .organization
            .read_logo_bytes(&org)
            .await?
            .map(|(bytes, mime)| {
                use base64::Engine;
                format!(
                    "data:{mime};base64,{}",
                    base64::engine::general_purpose::STANDARD.encode(bytes)
                )
            });
    // ... template load unchanged (see below) ...
    let ctx = serde_json::json!({
        "org": {
            "name": org.name,
            "inn": org.inn,
            "kpp": org.kpp,
            "address": org.address,
            "logo_data_uri": logo_data_uri,
        },
        "device": device_json,
        "document": { ... },
    });
    // ...
}
```

**Gold-standard `render_pdf` org-context construction to copy** (lines 2545-2578) — replace the acceptance path's `pipeline.organization.read()`/`read_logo_bytes` legacy branch with this exact `org_db.get_for_pdf()` pattern:
```rust
let org_legacy = pipeline.organization.read().await?;
let (org_dto, logo_bytes, logo_mime) = match pipeline.org_db {
    Some(org_db) => {
        let (dto, logo_bytes, logo_mime) = org_db.get_for_pdf().await?;
        (dto, logo_bytes, logo_mime)
    }
    None => (
        crate::dto::reports::OrgSettingsDto {
            org_name: org_legacy.name.clone(),
            inn: org_legacy.inn.clone(),
            kpp: org_legacy.kpp.clone(),
            address: org_legacy.address.clone(),
            has_logo: false,
            phone: String::new(),
            fax: String::new(),
            email: String::new(),
            okpo: String::new(),
            ogrn: String::new(),
            // D-07: add address_line2: String::new() here too
        },
        None,
        None,
    ),
};
// T-16-05 mitigation: `logo_bytes` originates exclusively from
// `OrgDbService::get_for_pdf` (org_settings BLOB, written only via
// authenticated Settings UI) — never from request-supplied bytes.
let logo_data_uri: Option<String> = logo_bytes.map(|bytes| {
    use base64::Engine;
    let mime = logo_mime.as_deref().unwrap_or("image/png");
    format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    )
});
```

**Gold-standard ctx assembly to copy** (lines 2629-2641, `render_pdf`'s `"org"` object — this is the FULL field set `render_acceptance_pdf`'s ctx must be expanded to, per D-03):
```rust
"org": {
    "name": org_dto.org_name,
    "inn": org_dto.inn,
    "kpp": org_dto.kpp,
    "address": org_dto.address,
    "phone": org_dto.phone,
    "fax": org_dto.fax,
    "email": org_dto.email,
    "okpo": org_dto.okpo,
    "ogrn": org_dto.ogrn,
    "logo_data_uri": logo_data_uri,
    // D-07: add "address_line2": org_dto.address_line2,
},
```

**Template-loading boilerplate is UNCHANGED** — `render_acceptance_pdf` already uses the correct file-first/embedded-fallback pattern (lines 2702-2716), identical in shape to `render_pdf`'s (lines 2579-2593). Only the `org`/logo construction changes; the `pipeline.organization.read()` call for legacy org.json (D-11: no fallback kept) and `read_logo_bytes` call are removed entirely.

**Signature note:** `render_pdf` doesn't need `pipeline.organization.read()` for requisites either — `org_legacy` is retained ONLY as the `None`-branch degrade fallback (when `org_db` isn't wired, e.g. in lightweight test fixtures without `.with_org_db(...)`). Follow the same defensive shape in the rewritten `render_acceptance_pdf`.

---

### `crates/trackly-app/src/services/org_db_service.rs` (service, CRUD) — mechanical column addition

**Analog:** itself — the exact same 3-site pattern used when phone/fax/email/okpo/ogrn were added (Phase 14), now repeated for one more column.

**`get()` — SELECT list + struct literal to extend** (lines 51-82):
```rust
pub async fn get(&self) -> Result<OrgSettingsDto, AppError> {
    let readers = self.readers.clone();
    tokio::task::spawn_blocking(move || -> Result<OrgSettingsDto, AppError> {
        let conn = readers.acquire();
        conn.query_row(
            "SELECT org_name, inn, kpp, address, \
             (logo_blob IS NOT NULL) as has_logo, \
             phone, fax, email, okpo, ogrn \
             FROM org_settings WHERE id = 1",
            [],
            |r| {
                Ok(OrgSettingsDto {
                    org_name: r.get(0)?,
                    inn: r.get(1)?,
                    kpp: r.get(2)?,
                    address: r.get(3)?,
                    has_logo: r.get::<_, bool>(4)?,
                    phone: r.get(5)?,
                    fax: r.get(6)?,
                    email: r.get(7)?,
                    okpo: r.get(8)?,
                    ogrn: r.get(9)?,
                })
            },
        )
        .map_err(map_rusqlite)
    })
    // ...
}
```
Extend the SELECT column list with `, address_line2` (append-only, per D-04's migration doc-comment: "new columns are added last") and add `address_line2: r.get(10)?,` to the struct literal.

**`save_fields()` — UPDATE + params list to extend** (lines 85-118):
```rust
pub async fn save_fields(
    &self,
    caller: &Identity,
    patch: crate::dto::reports::OrgPatch,
) -> Result<(), AppError> {
    authorize(caller, &Action::ManageSettings)?;
    let now = self.clock.unix_seconds();
    self.writer
        .execute(move |conn| {
            conn.execute(
                "UPDATE org_settings \
                 SET org_name=?2, inn=?3, kpp=?4, address=?5, \
                     phone=?6, fax=?7, email=?8, okpo=?9, ogrn=?10, \
                     updated_at_utc=?11, version=version+1 \
                 WHERE id=1",
                params![
                    1i64,
                    patch.org_name,
                    patch.inn,
                    patch.kpp,
                    patch.address,
                    patch.phone,
                    patch.fax,
                    patch.email,
                    patch.okpo,
                    patch.ogrn,
                    now
                ],
            )
            .map(|_| ())
            .map_err(map_rusqlite)
        })
        .await
}
```
Add `address_line2=?11` to the SET clause (shift `updated_at_utc`/`version` params accordingly, or append at the end before `now` — either ordering is fine as long as placeholder numbers and the `params![...]` list stay in sync), add `patch.address_line2` to the params list.

**`get_for_pdf()` — SELECT + tuple construction to extend** (lines 363-401, the function `render_pdf`/`render_acceptance_pdf`/`report_service::export_pdf` all consume):
```rust
pub async fn get_for_pdf(
    &self,
) -> Result<(OrgSettingsDto, Option<Vec<u8>>, Option<String>), AppError> {
    type PdfTuple = (OrgSettingsDto, Option<Vec<u8>>, Option<String>);
    let readers = self.readers.clone();
    tokio::task::spawn_blocking(move || -> Result<PdfTuple, AppError> {
        let conn = readers.acquire();
        conn.query_row(
            "SELECT org_name, inn, kpp, address, \
             (logo_blob IS NOT NULL) as has_logo, \
             logo_blob, logo_mime, \
             phone, fax, email, okpo, ogrn \
             FROM org_settings WHERE id = 1",
            [],
            |r| {
                let dto = OrgSettingsDto {
                    org_name: r.get(0)?,
                    inn: r.get(1)?,
                    kpp: r.get(2)?,
                    address: r.get(3)?,
                    has_logo: r.get::<_, bool>(4)?,
                    phone: r.get(7)?,
                    fax: r.get(8)?,
                    email: r.get(9)?,
                    okpo: r.get(10)?,
                    ogrn: r.get(11)?,
                };
                let logo_blob: Option<Vec<u8>> = r.get(5)?;
                let logo_mime: Option<String> = r.get(6)?;
                Ok((dto, logo_blob, logo_mime))
            },
        )
        .map_err(map_rusqlite)
    })
    // ...
}
```
Append `, address_line2` to the SELECT list, add `address_line2: r.get(12)?,` to the `dto` struct literal (mind that `logo_blob`/`logo_mime` sit at ordinals 5/6 — the new column goes after ordinal 11, i.e. becomes ordinal 12).

**Note:** `migrate_from_org_json()` (lines 210-359) does NOT need `address_line2` handling — legacy `org.json`/`OrgData` never had a second address line, and D-11 already retires acceptance's org.json read path entirely; no new migration-from-legacy logic is needed for this field.

---

### `crates/trackly-app/src/dto/reports.rs` (model/DTO, transform)

**Analog:** itself — the file's own doc-comment "Extended requisites (PDFA-03, Phase 14)" pattern for `phone`/`fax`/`email`/`okpo`/`ogrn` is the template for adding `address_line2`.

**`OrgPatch` struct to extend** (lines 176-188):
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct OrgPatch {
    pub org_name: String,
    pub inn: String,
    pub kpp: String,
    pub address: String,
    /// Extended requisites (PDFA-03, Phase 14). Empty string = not filled in.
    pub phone: String,
    pub fax: String,
    pub email: String,
    pub okpo: String,
    pub ogrn: String,
}
```
Add `/// Second address line (ORG-02, Phase 20). Empty string = not filled in.` + `pub address_line2: String,`.

**`OrgSettingsDto` struct to extend** (lines 206-221):
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct OrgSettingsDto {
    pub org_name: String,
    pub inn: String,
    pub kpp: String,
    pub address: String,
    /// True if a logo is stored (logo_blob IS NOT NULL). Frontend shows
    /// "Remove logo" button only when this is true.
    pub has_logo: bool,
    /// Extended requisites (PDFA-03, Phase 14). Empty string = not filled in.
    pub phone: String,
    pub fax: String,
    pub email: String,
    pub okpo: String,
    pub ogrn: String,
}
```
Add the same new field with the same doc-comment convention. Field naming is snake_case (file-level convention documented at the top of `reports.rs`, line 5) — `address_line2` already matches this convention.

After editing, regenerate TypeScript bindings via the existing `export_bindings` test (`crates/trackly-app/tests/export_bindings.rs`) — this is the established mechanism (see canonical_refs Integration Points), not a new pattern to invent.

---

### `crates/trackly-app/templates/act_acceptance.html` (template, transform) — PRN-01 primary target

**Analog:** `act_handover.html` `.requisites` block (lines 131-140) — copy verbatim, per D-01/D-06.

**Current deficient acceptance `.requisites` block** (act_acceptance.html, lines 103-107 — to be replaced):
```html
    <div class="requisites">
      {%- if org.name %}<div>{{ org.name }}</div>{%- endif %}
      {%- if org.inn %}<div>ИНН {{ org.inn }}{% if org.kpp %} / КПП {{ org.kpp }}{% endif %}</div>{%- endif %}
      {%- if org.address %}<div>{{ org.address }}</div>{%- endif %}
    </div>
```

**Target full block to copy from `act_handover.html`** (lines 131-140), with `address_line2` inserted per D-06 immediately after `address`:
```html
    <div class="requisites">
      {%- if org.name %}<div>{{ org.name }}</div>{%- endif %}
      {%- if org.inn %}<div>ИНН {{ org.inn }}{% if org.kpp %} / КПП {{ org.kpp }}{% endif %}</div>{%- endif %}
      {%- if org.address %}<div>{{ org.address }}</div>{%- endif %}
      {%- if org.address_line2 %}<div>{{ org.address_line2 }}</div>{%- endif %}
      {%- if org.phone %}<div>Тел.: {{ org.phone }}</div>{%- endif %}
      {%- if org.fax %}<div>Факс: {{ org.fax }}</div>{%- endif %}
      {%- if org.email %}<div>E-mail: {{ org.email }}</div>{%- endif %}
      {%- if org.okpo %}<div>ОКПО {{ org.okpo }}</div>{%- endif %}
      {%- if org.ogrn %}<div>ОГРН {{ org.ogrn }}</div>{%- endif %}
    </div>
```

**Doc-comment header must also be updated** (act_acceptance.html lines 10-13, currently understating the context shape):
```
  Context (smaller than handover — see act_service::render_acceptance_pdf):
    org.{name,inn,kpp,address,logo_data_uri}
    device.{name,inventory_no,serial_no,model,condition}
    document.{giver_name, receiver_name, date_human}
```
must become (mirroring `act_handover.html`'s doc-comment shape, lines 11-14):
```
  Context (full org header, brought to parity with act_handover.html per
  PRN-01/D-01/D-02/D-03 — Phase 20):
    org.{name,inn,kpp,address,address_line2,logo_data_uri,phone,fax,email,okpo,ogrn}
    device.{name,inventory_no,serial_no,model,condition}
    document.{giver_name, receiver_name, date_human}
```

**Logo `<img>` tag is UNCHANGED** — already `<img src="{{ org.logo_data_uri | safe }}" alt="Логотип">` (line 100), identical to `act_handover.html`'s (line 128) and `report.html`'s (line 132). This is the img-only security invariant (ORG-01/D-08/D-09) — do not touch.

---

### `crates/trackly-app/templates/act_handover.html` (template, transform) — ORG-02 propagation

**Analog:** itself — insert one line into the existing `.requisites` block using the field's own established idiom.

**Current block** (lines 131-140):
```html
    <div class="requisites">
      {%- if org.name %}<div>{{ org.name }}</div>{%- endif %}
      {%- if org.inn %}<div>ИНН {{ org.inn }}{% if org.kpp %} / КПП {{ org.kpp }}{% endif %}</div>{%- endif %}
      {%- if org.address %}<div>{{ org.address }}</div>{%- endif %}
      {%- if org.phone %}<div>Тел.: {{ org.phone }}</div>{%- endif %}
      {%- if org.fax %}<div>Факс: {{ org.fax }}</div>{%- endif %}
      {%- if org.email %}<div>E-mail: {{ org.email }}</div>{%- endif %}
      {%- if org.okpo %}<div>ОКПО {{ org.okpo }}</div>{%- endif %}
      {%- if org.ogrn %}<div>ОГРН {{ org.ogrn }}</div>{%- endif %}
    </div>
```
Per D-06, insert `{%- if org.address_line2 %}<div>{{ org.address_line2 }}</div>{%- endif %}` immediately after the `org.address` line (same as shown in the `act_acceptance.html` section above). Also update the doc-comment context-variable list (lines 11-14) to add `address_line2`.

---

### `crates/trackly-app/templates/report.html` (template, transform) — ORG-02 propagation, third site

**Analog:** `act_handover.html`'s `.requisites` block — `report.html`'s own doc-comment (lines 7-9) explicitly states "The organization header block below is copied verbatim from act_handover.html ... for visual consistency", so this is a documented, intentional 1:1 copy relationship, not just an inferred analog.

**Current block** (lines 135-144, byte-for-byte identical to `act_handover.html`'s pre-change block):
```html
    <div class="requisites">
      {%- if org.name %}<div>{{ org.name }}</div>{%- endif %}
      {%- if org.inn %}<div>ИНН {{ org.inn }}{% if org.kpp %} / КПП {{ org.kpp }}{% endif %}</div>{%- endif %}
      {%- if org.address %}<div>{{ org.address }}</div>{%- endif %}
      {%- if org.phone %}<div>Тел.: {{ org.phone }}</div>{%- endif %}
      {%- if org.fax %}<div>Факс: {{ org.fax }}</div>{%- endif %}
      {%- if org.email %}<div>E-mail: {{ org.email }}</div>{%- endif %}
      {%- if org.okpo %}<div>ОКПО {{ org.okpo }}</div>{%- endif %}
      {%- if org.ogrn %}<div>ОГРН {{ org.ogrn }}</div>{%- endif %}
    </div>
```
Apply the identical one-line `address_line2` insertion (same position, same idiom) as `act_handover.html`. Also update the doc-comment context-variable list (lines 12-13, which already lists `org.name, org.inn, org.kpp, org.address, org.logo_data_uri, org.phone, ...`) to insert `address_line2` after `address`.

**`report_service.rs::export_pdf`'s ctx assembly must also be extended** (lines 637-649, the third and final site building the `"org"` JSON object):
```rust
let ctx = serde_json::json!({
    "org": {
        "name": org.org_name,
        "inn": org.inn,
        "kpp": org.kpp,
        "address": org.address,
        "phone": org.phone,
        "fax": org.fax,
        "email": org.email,
        "okpo": org.okpo,
        "ogrn": org.ogrn,
        "logo_data_uri": logo_data_uri,
    },
    // ...
});
```
Add `"address_line2": org.address_line2,` after `"address": org.address,` — matching the field's position in the templates.

---

### `ui/src/features/settings/OrgSettings.svelte` (component/form, CRUD)

**Analog:** itself — the existing `address` field (form-field--full) in the same form is the direct model; D-05 specifies the new field goes "сразу под полем «Адрес»" using the same `form-field--full` class.

**TS interface to extend** (lines 8-19):
```typescript
interface OrgSettingsDto {
  org_name: string;
  inn: string;
  kpp: string;
  address: string;
  has_logo: boolean;
  phone: string;
  fax: string;
  email: string;
  okpo: string;
  ogrn: string;
}
```
Add `address_line2: string;` after `address`.

**State declarations to extend** (lines 21-30):
```typescript
let orgName = $state('');
let inn = $state('');
let kpp = $state('');
let address = $state('');
let hasLogo = $state(false);
let phone = $state('');
```
Add `let addressLine2 = $state('');` after `address`.

**`loadOrg()` to extend** (lines 39-62, assignment block lines 43-51):
```typescript
async function loadOrg() {
  try {
    const dto = await apiCall<OrgSettingsDto>('settings_get_org', {});
    orgName = dto.org_name;
    inn = dto.inn;
    kpp = dto.kpp;
    address = dto.address;
    hasLogo = dto.has_logo;
    phone = dto.phone;
    // ...
```
Add `addressLine2 = dto.address_line2;` after `address = dto.address;`.

**`saveOrg()` payload to extend** (lines 87-113, patch object lines 91-101):
```typescript
async function saveOrg() {
  saving = true;
  try {
    await apiCall<void>('settings_save_org_fields', {
      patch: {
        org_name: orgName,
        inn,
        kpp,
        address,
        phone,
        fax,
        email,
        okpo,
        ogrn,
      },
    });
```
Add `address_line2: addressLine2,` after `address,`.

**Markup — the new field, copying the existing `address` field's exact shape** (lines 247-256):
```svelte
<div class="form-field form-field--full">
  <label class="form-label" for="org-address">Адрес</label>
  <input
    id="org-address"
    class="form-input"
    type="text"
    bind:value={address}
    placeholder="г. Москва, ул. Примерная, д. 1"
  />
</div>
```
Insert immediately after (per D-05: same `form-field--full`, label text exactly **«Адрес (2-я строка)»**, per Specific Ideas):
```svelte
<div class="form-field form-field--full">
  <label class="form-label" for="org-address-line2">Адрес (2-я строка)</label>
  <input
    id="org-address-line2"
    class="form-input"
    type="text"
    bind:value={addressLine2}
    placeholder="офис 305, корпус 2"
  />
</div>
```
No SCSS changes needed — `.form-field--full` already exists (lines 393-395) and is reused verbatim.

---

### `crates/trackly-app/tests/html_act_render.rs` (test, transform) — D-09 regression test

**Analog:** `html_handover_contains_required_blocks_and_logo` (lines 163-207) for the "save real logo via production path, then render, then assert on HTML content" shape; `html_is_offline_safe_no_external_links` (lines 372-422) for the "assert absence of X, with a sanity check that the assertion isn't vacuous" shape — D-09 needs exactly this pairing (assert NO raw `<script>` in the rendered output, PLUS a sanity check that the logo IS present as `data:` URI, so the negative assertion isn't trivially true because the logo failed to embed at all).

**Fixture setup pattern to copy** (lines 164-181, adapt `LOGO_PNG`/`"image/png"` to a new SVG-with-script fixture and `"image/svg+xml"`):
```rust
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn html_handover_contains_required_blocks_and_logo() {
    let p = make_full_pipeline().await;

    // Save a BLOB logo via the real production path (OrgDbService::save_logo).
    let org_db = Arc::new(OrgDbService::new(
        p.writer.clone(),
        p._readers.clone(),
        Arc::new(SystemClock),
        Arc::new(Paths::resolve_for_exe_dir(p._dir.path().to_path_buf()).expect("paths")),
    ));
    org_db
        .save_logo(
            &Identity::trusted_admin(),
            LOGO_PNG.to_vec(),
            "image/png".to_string(),
        )
        .await
        .expect("save_logo");

    let device_id = seed_device(&p.writer, "HTML-Логотест-Ноутбук").await;
    let act = create_handover(&p.acts, &[device_id], "Выдалов В.В.", "Получилов П.П.").await;

    let html = p.acts.render_pdf(act.id).await.expect("render_pdf");
    // ... assertions ...
}
```

**New fixture needed:** a small SVG file containing an embedded `<script>` tag (per Specific Ideas: "Тест безопасности должен использовать SVG именно с `<script>` внутри"). Follow the `LOGO_PNG` const-fixture pattern (line 24: `const LOGO_PNG: &[u8] = include_bytes!("fixtures/logo_test.png");`) — add a sibling file `crates/trackly-app/tests/fixtures/logo_test_with_script.svg` (or similar name) and a matching `const LOGO_SVG_WITH_SCRIPT: &[u8] = include_bytes!("fixtures/logo_test_with_script.svg");`. Example minimal SVG content:
```xml
<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><script>alert('xss')</script><rect width="10" height="10" fill="red"/></svg>
```

**Assertion pattern to copy** (adapted from `html_is_offline_safe_no_external_links`'s "assert absence + sanity check presence" pairing, lines 372-422):
```rust
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn html_svg_logo_with_script_embeds_img_only_no_inline_script() {
    let p = make_full_pipeline().await;

    let org_db = Arc::new(OrgDbService::new(
        p.writer.clone(),
        p._readers.clone(),
        Arc::new(SystemClock),
        Arc::new(Paths::resolve_for_exe_dir(p._dir.path().to_path_buf()).expect("paths")),
    ));
    org_db
        .save_logo(
            &Identity::trusted_admin(),
            LOGO_SVG_WITH_SCRIPT.to_vec(),
            "image/svg+xml".to_string(),
        )
        .await
        .expect("save_logo");

    let device_id = seed_device(&p.writer, "HTML-SVG-Script-Ноутбук").await;
    let act = create_handover(&p.acts, &[device_id], "Иванов И.И.", "Петров П.П.").await;

    let html = p.acts.render_pdf(act.id).await.expect("render_pdf");

    // Negative assertion (D-09 core claim): the raw <script> tag must never
    // appear inline in the rendered document DOM.
    assert!(
        !html.contains("<script>"),
        "SVG logo's embedded <script> must not be inlined into act HTML \
         (img-only embedding invariant, ORG-01/D-08/D-09)"
    );

    // Sanity check (mirrors html_is_offline_safe_no_external_links's pattern):
    // confirm the logo DID embed as a data: URI — proves the negative
    // assertion above isn't vacuously true because the logo failed silently.
    assert!(
        html.contains("data:image/svg+xml;base64,"),
        "expected SVG logo to be present as a data: URI (proves the \
         <script>-absence check above wasn't vacuous). Head: {:?}",
        html.chars().take(500).collect::<String>()
    );

    // Additional confirmation: the logo is embedded exclusively via <img src=...>,
    // not injected as raw <svg>/inline markup elsewhere in the document.
    assert!(
        html.contains("<img src=\"data:image/svg+xml;base64,"),
        "logo must be embedded via <img src=\"data:...\"> (img-only pattern), \
         not as inline <svg> markup"
    );
}
```

Add this test to `crates/trackly-app/tests/html_act_render.rs` alongside the existing D-14 tests, and add `LOGO_SVG_WITH_SCRIPT` as a sibling const next to `LOGO_PNG` (line 24).

---

## Shared Patterns

### Full-org-header context assembly (the phase's central cross-cutting concern)
**Source:** `crates/trackly-app/src/services/act_service.rs::render_pdf` (lines 2545-2578, 2629-2641) and `crates/trackly-app/src/services/report_service.rs::export_pdf` (lines 559-590, 637-649)
**Apply to:** `act_service.rs::render_acceptance_pdf` (the file being brought to parity)
```rust
// 1. Fetch full DTO + logo bytes from OrgDbService::get_for_pdf() (never
//    legacy pipeline.organization.read() for acceptance, per D-11).
let (org_dto, logo_bytes, logo_mime) = org_db.get_for_pdf().await?;

// 2. Enforce mime allowlist on READ too (report_service.rs's WR-05
//    mitigation, lines 573-582) — act_service.rs's render_pdf does NOT
//    currently re-check mime on read (it trusts get_for_pdf's already-
//    validated-on-write BLOB); Claude's discretion whether to add
//    report_service's stricter double-check to the acceptance path too,
//    but at minimum copy render_pdf's existing (simpler) shape for parity.
let logo_data_uri: Option<String> = logo_bytes.map(|bytes| {
    use base64::Engine;
    let mime = logo_mime.as_deref().unwrap_or("image/png");
    format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes))
});

// 3. Build ctx["org"] with the FULL 10-field set (9 existing + address_line2).
```

### SVG logo img-only embedding (security invariant, ORG-01)
**Source:** `crates/trackly-app/templates/act_handover.html` line 128, `act_acceptance.html` line 100, `report.html` line 132 (all three identical)
**Apply to:** No new template code needed (D-08: already correctly implemented everywhere) — apply only as the REGRESSION TEST TARGET (D-09), see `html_act_render.rs` section above.
```html
<img src="{{ org.logo_data_uri | safe }}" alt="Логотип">
```
The `| safe` filter is scoped exclusively to this one server-constructed base64 `data:` URI — never applied to any user-supplied text field. This is documented identically in all three templates' inline comments (e.g. act_handover.html lines 122-127).

### `address_line2` empty-guard rendering idiom (ORG-02, D-06)
**Source:** the existing `org.phone`/`org.fax`/etc. lines in all three templates (e.g. `act_handover.html` line 135)
**Apply to:** all three templates, inserted right after the `org.address` line
```html
{%- if org.address_line2 %}<div>{{ org.address_line2 }}</div>{%- endif %}
```

### Refinery migration for `org_settings` ADD COLUMN (D-04, D-10)
**Source:** `migrations/V033__org_settings_requisites.sql` (full file reproduced above)
**Apply to:** new `migrations/V035__org_settings_address_line2.sql`
```sql
ALTER TABLE org_settings ADD COLUMN address_line2 TEXT NOT NULL DEFAULT '';
PRAGMA user_version = 35;
```

## No Analog Found

None — every file in scope for this phase has a strong, often verbatim, in-repo analog. This phase is explicitly scoped as "extend existing pattern to a third/fourth site," which is why RESEARCH.md was skipped.

## Metadata

**Analog search scope:** `crates/trackly-app/src/services/`, `crates/trackly-app/src/dto/`, `crates/trackly-app/templates/`, `migrations/`, `crates/trackly-app/tests/`, `ui/src/features/settings/`, `ui/src/features/devices/`
**Files scanned:** act_service.rs (targeted read, lines 2536-2815), report_service.rs (targeted read, lines 540-654), org_db_service.rs (full, 408 lines), dto/reports.rs (full, 307 lines), act_handover.html (full, 235 lines), act_acceptance.html (full, 135 lines), report.html (full, 177 lines), OrgSettings.svelte (full, 483 lines), tauri_cmds/settings_org.rs (targeted, lines 1-40), html_act_render.rs (targeted reads, lines 1-245 and 372-431), migrations/V026, V033 (full)
**Migration numbering confirmed:** latest existing migration is `V034__return_handover_date_backfill.sql`; new migration for this phase MUST be `V035__<name>.sql` with `PRAGMA user_version = 35`.
**Pattern extraction date:** 2026-07-13
