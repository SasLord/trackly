# Phase 34: Единая шапка документов — Pattern Map

**Mapped:** 2026-08-08
**Files analyzed:** 17 (7 new, 10 modified)
**Analogs found:** 17 / 17

> Все паттерны этой фазы уже реализованы в кодовой базе (Фазы 16/17/20/33) — это не
> проектирование нового механизма, а его точное расширение на четвёртый файл (`_header.html`)
> и новое поле `org_settings`. Ниже — verbatim-выдержки из реального кода с путями и номерами
> строк на момент маппинга (2026-08-08); при выполнении фазы номера строк сместятся по мере
> правок, ориентироваться на сигнатуры функций/имена, не на абсолютные номера.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `crates/trackly-app/templates/_header.html` (new) | template/partial | transform (render) | `crates/trackly-app/templates/act_handover.html` (doc-comment + header block) | exact (structural twin, minus surrounding document) |
| `crates/trackly-app/templates/_legacy_defaults/v21/{act_handover,act_acceptance,report}.html` (new) | config/snapshot | file-I/O | `crates/trackly-app/templates/_legacy_defaults/v20/*.html` | exact |
| `crates/trackly-app/tests/html_header_parity.rs` (new) | test | transform (structural gate) | `crates/trackly-app/tests/html_page_parity.rs` | exact |
| `migrations/V036__org_settings_full_name.sql` (new) | migration | file-I/O | `migrations/V035__org_settings_address_line2.sql` | exact |
| `crates/trackly-app/templates/{act_handover,act_acceptance,report}.html` (modified) | template | transform (render) | each other (already byte-identical header block) | exact |
| `crates/trackly-app/src/pdf/html_templates.rs` (modified) | utility/loader | file-I/O | itself — `DEFAULT_HTML_TEMPLATES` / `KNOWN_LEGACY_DEFAULTS` / `materialize_defaults_on_startup` / `upgrade_untouched_defaults_on_startup` | exact (extend existing arrays/branches) |
| `crates/trackly-app/src/pdf/minijinja_env.rs` (modified) | utility | transform (render) | itself — `render_with_timeout` | exact (extend to register 2 templates) |
| `crates/trackly-app/src/services/act_service.rs` (modified, 2 sites) | service | CRUD (context assembly) | itself — `render_pdf` ctx block, mirrored in `render_acceptance_pdf` | exact |
| `crates/trackly-app/src/services/report_service.rs` (modified) | service | CRUD (context assembly) | `act_service.rs` `render_pdf` ctx block | exact |
| `crates/trackly-app/src/services/template_service.rs` (modified) | service | request-response (preview stub) | itself — `demo_context_for_kind` | exact |
| `crates/trackly-app/src/dto/reports.rs` — `OrgSettingsDto`/`OrgPatch` (modified) | model/DTO | CRUD | itself — sibling field `address_line2` | exact |
| `crates/trackly-app/src/services/org_db_service.rs` (modified) | service | CRUD | itself — `save_fields`/`get_for_pdf` | exact |
| `crates/trackly-app/src/tauri_cmds/settings_org.rs` (modified, +1 command) | controller (Tauri) | request-response | itself — `templates_list_for_editor` | exact |
| `crates/trackly-app/src/http/settings_org.rs` (modified, +1 handler) | controller (HTTP) | request-response | itself — `handler_templates_list_for_editor` | exact |
| `ui/src/features/settings/OrgSettings.svelte` (modified) | component | request-response (form) | itself — `addressLine2` field (state + load + save + markup) | exact |
| `ui/src/features/settings/TemplateEditor.svelte` — `VARIABLES_BY_KIND` (modified) | component (data table) | transform | itself | exact |

No file in this phase lacks a close analog — see "No Analog Found" note at the end for the one caveat (D-17 status DTO shape).

---

## Pattern Assignments

### `crates/trackly-app/templates/_header.html` (new partial)

**Analog:** the three existing templates' header block + doc-comment convention (all three are currently byte-identical in this region — `act_handover.html:119-142`, `act_acceptance.html:92-115`, `report.html:123-146`).

**Doc-comment convention to copy verbatim** (`act_handover.html:1-30`):
```jinja
<!DOCTYPE html>
{#- Default HTML template for Акт приёма-передачи (Phase 16, D-01/D-02/D-03).

  Self-contained HTML5 document (inline <style>, no external CSS/CDN, D-11
  data: URI logo) reproducing the Word-sample block order fixed in Phase 15's
  act_handover.minijinja: ...

  Context variables (same shape as act_service::render_pdf's ctx, D-04, plus
  org.logo_data_uri replacing org.logo_path per D-11):
    org.name, org.inn, org.kpp, org.address, org.address_line2, org.logo_data_uri,
    org.phone, org.fax, org.email, org.okpo, org.ogrn
    act.number, act.suffix, act.date, act.date_human, ...

  Autoescape is ON (build_safe_html_env, T-16-01) — plain {{ var }}
  interpolation is HTML-safe by construction. The single exception is
  `org.logo_data_uri | safe` (see inline comment at its use site): ...
-#}
```
`_header.html` is a *partial*, not a full document — it has no `<!DOCTYPE html>`/`<head>`; adapt the doc-comment to describe only the header's own context keys (`org.name`, `org.full_name`, `org.logo_data_uri`, `org.address`, `org.address_line2`, `org.phone`, `org.fax`, `org.email`, `org.okpo`, `org.ogrn`, `org.inn`, `org.kpp`) and note it is `{% include %}`-d by all three parent templates via the registry (not the filesystem — see minijinja_env.rs pattern below).

**Current (pre-Phase-34) header markup — identical in all three files**, e.g. `act_handover.html:119-142`:
```jinja
  <div class="header">
    <div class="logo">
      {%- if org.logo_data_uri %}
      {#- logo_data_uri is server-constructed exclusively from base64 output
        (RFC 4648 alphabet [A-Za-z0-9+/=], never user-controlled HTML) plus a
        hardcoded mime whitelist (act_service.rs) — `| safe` here does not
        reopen T-16-01's XSS mitigation, it only prevents autoescape from
        HTML-entity-encoding the `/` in "data:image/png;base64,..." into
        `&#x2f;`, which corrupts the URI scheme and breaks the image. -#}
      <img src="{{ org.logo_data_uri | safe }}" alt="Логотип">
      {%- endif %}
    </div>
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
  </div>
```
This is the block that moves into `_header.html`, restructured per CONTEXT D-05/D-07 (1-column `width:80mm` flex block; new `.orgName` div between `.logo` and `.requisites`; reordered/reworded requisites — full inventory + privacy-scrubbed target markup already assembled in `34-RESEARCH.md` §"Эталон вёрстки", item 6, "Эталон" code block — do not re-derive it, copy from there). The `org.logo_data_uri | safe` comment above is the **exact prose template** for the new `org.full_name | safe` comment (D-03's threat-model note), per the "reusable asset" callout in RESEARCH.

**CSS to copy/adapt** — current `.header`/`.logo img`/`.requisites` rules (identical in all three files, e.g. `act_handover.html:47-61`):
```css
  .header {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 12pt;
    align-items: flex-start;
    margin-bottom: 16pt;
  }
  .header .logo img {
    max-height: 60pt;
    max-width: 120pt;
  }
  .header .requisites {
    font-size: 9pt;
    line-height: 1.35;
  }
```
Replace with the flex/`80mm`/`.orgName` ruleset per RESEARCH §"Итоговый чек-лист переноса" (checklist items 1-4, 8) — goes inside `_header.html`'s own `<style>` tag (D-12: markup **and** CSS live in the partial, not split).

---

### `crates/trackly-app/templates/_legacy_defaults/v21/{act_handover,act_acceptance,report}.html` (new)

**Analog:** `crates/trackly-app/templates/_legacy_defaults/v20/` — directory currently contains exactly 3 files, no README/index:
```
_legacy_defaults/v20/act_acceptance.html   4492 bytes
_legacy_defaults/v20/act_handover.html     7649 bytes
_legacy_defaults/v20/report.html           5414 bytes
```
v21 must mirror this shape: same 3 filenames, same directory, sibling `v21/`. **Content = the CURRENT (pre-Phase-34) repo bodies**, captured before the header/font rewrite lands — either `cp crates/trackly-app/templates/{f}.html crates/trackly-app/templates/_legacy_defaults/v21/{f}.html` done *before* editing the canon files, or `git show <pre-phase-commit>:crates/trackly-app/templates/{f}.html > .../v21/{f}.html` after the fact (Pitfall 5 in RESEARCH — verify `diff v21/x.html x.html` is non-empty once Phase 34 edits land).

---

### `crates/trackly-app/src/pdf/html_templates.rs` (modified)

**Analog:** itself — extend three existing constructs, do not invent new ones.

**`DEFAULT_HTML_TEMPLATES`** (lines 30-40) — add `_header.html` as a 4th tuple:
```rust
pub const DEFAULT_HTML_TEMPLATES: &[(&str, &str)] = &[
    (
        "act_handover.html",
        include_str!("../../templates/act_handover.html"),
    ),
    (
        "act_acceptance.html",
        include_str!("../../templates/act_acceptance.html"),
    ),
    ("report.html", include_str!("../../templates/report.html")),
    // NEW: ("_header.html", include_str!("../../templates/_header.html")),
];
```
Doc-comment above `KNOWN_LEGACY_DEFAULTS` (lines 42-59) is the **explicit written instruction** for this exact extension point — quoted here in full because the planner/executor must follow it verbatim:
```rust
/// **Extension point:** whenever a body in `DEFAULT_HTML_TEMPLATES` changes
/// again in a future phase, the PRE-CHANGE body MUST be captured as a new
/// snapshot (a new sibling directory, e.g. `_legacy_defaults/v21/`) and added
/// here as an additional entry in that filename's slice — otherwise installs
/// that predate THAT phase stop being recognized as untouched and silently
/// stop receiving upgrades. Forgetting this only causes a MISSED upgrade (file
/// stays on older-but-valid content), never a wrongful overwrite.
```

**`KNOWN_LEGACY_DEFAULTS`** (lines 60-79) — add `v21` `include_str!` as a 2nd element of each existing 3-element array (do NOT replace `v20`); `_header.html` gets **no** entry here (new file, materialize handles it):
```rust
pub const KNOWN_LEGACY_DEFAULTS: &[(&str, &[&str])] = &[
    (
        "act_handover.html",
        &[
            include_str!("../../templates/_legacy_defaults/v20/act_handover.html"),
            include_str!("../../templates/_legacy_defaults/v21/act_handover.html"), // NEW
        ],
    ),
    // ...same pattern for act_acceptance.html, report.html
];
```

**`upgrade_untouched_defaults_on_startup`** (lines 134-162) — the fall-through branch that D-16 requires a `tracing::warn!` in is the trailing `else` with no arm today:
```rust
        if legacy_bodies.iter().any(|legacy| *legacy == on_disk) {
            std::fs::write(&path, current_default).map_err(|e| AppError::Internal {
                source_chain: format!("write({}) failed: {e}", path.display()),
            })?;
            tracing::info!(
                "Auto-upgraded untouched default HTML template at {}",
                path.display()
            );
        }
        // else: user-customized (matches neither current nor any known legacy
        // default) — leave untouched, fail closed.
```
Add an explicit `else { tracing::warn!(...) }` in place of that trailing comment — see RESEARCH §"Warn-ветка (D-16)" for the exact suggested message; must interpolate `path.display()`.

**Test pattern to copy for the new v21-specific test** (`upgrade_replaces_untouched_legacy_default_with_current_bundled_body`, lines 233-256) — currently uses `.first()` (always v20); the new test should explicitly pull index 1 (v21) instead of relying on `.first()`, otherwise the new snapshot is unexercised:
```rust
    #[test]
    fn upgrade_replaces_untouched_legacy_default_with_current_bundled_body() {
        let _guard = ENV_GUARD.lock().unwrap();
        let dir = tempfile::tempdir().expect("tempdir");

        for (filename, _current) in DEFAULT_HTML_TEMPLATES.iter() {
            let legacy = KNOWN_LEGACY_DEFAULTS
                .iter()
                .find(|(name, _)| name == filename)
                .and_then(|(_, bodies)| bodies.first())
                .expect("legacy snapshot registered for filename");
            std::fs::write(dir.path().join(filename), legacy).expect("write legacy body");
        }

        upgrade_untouched_defaults_on_startup(dir.path()).expect("upgrade ok");
        // ...assert_eq! against current
    }
```

---

### `crates/trackly-app/src/pdf/minijinja_env.rs` (modified)

**Analog:** itself — `render_with_timeout` (lines 68-107), currently registers exactly one template per render:
```rust
pub async fn render_with_timeout(
    env: &Environment<'static>,
    name: &str,
    template_src: &str,
    ctx: serde_json::Value,
) -> Result<String, AppError> {
    let env_owned = env.clone();
    let name_owned = name.to_owned();
    let template_src_owned = template_src.to_owned();

    let join = tokio::task::spawn_blocking(move || -> Result<String, AppError> {
        let mut env = env_owned;
        env.add_template_owned(name_owned.clone(), template_src_owned)
            .map_err(|e| AppError::Validation {
                field: "template".into(),
                message: format!("Template parse error: {e}"),
            })?;
        let tmpl = env
            .get_template(&name_owned)
            .map_err(|e| AppError::Validation {
                field: "template".into(),
                message: format!("Template lookup error: {e}"),
            })?;
        tmpl.render(ctx).map_err(|e| AppError::Validation {
            field: "template".into(),
            message: format!("Template render error: {e}"),
        })
    });
    // ...timeout wrapper unchanged
}
```
Extend the signature to also accept the partial's `(name, src)` — both `add_template_owned` calls must happen **before** `get_template`/`render` of the main template (Pitfall 4 in RESEARCH — include resolves at render time but the registry lookup is by name, order between the two `add_template_owned` calls doesn't matter, but both must precede `render`). Concrete signature/call pattern suggested by RESEARCH (`minijinja::HtmlEscape`/registration section) — extend with an `extra_templates: &[(&str, &str)]` parameter or equivalent; all 4 call sites (`act_service.rs` ×2, `report_service.rs` ×1, `template_service.rs`'s `validate_preview` for the editor-preview path) and the 4 existing `#[tokio::test]`s in this file (lines 113-174) must be updated for the new signature.

**Doc-comment fix (C-04)** — currently false, must be corrected as part of this phase (lines 41-52):
```rust
/// ... The only difference is
/// `AutoEscape::Html` instead of `AutoEscape::None`: every template rendered
/// through this environment is HTML output (act_handover.html /
/// act_acceptance.html), so `{{ var }}` interpolation must be HTML-escaped by
/// default — this is the sole mitigation for T-16-01 (Tampering/Injection via
/// device/org field interpolation). No `| safe` filter is used anywhere in
/// the shipped templates.
```
The last sentence is the one to rewrite — it must now enumerate `org.logo_data_uri | safe` (pre-existing) and `org.full_name | safe` (new, D-03) as the only sanctioned exceptions, and state the invariant explicitly ("`| safe` only for values escaped/assembled exclusively server-side from non-user-HTML input").

**Never-loader invariant (unchanged, do not touch):**
```rust
    env.set_undefined_behavior(UndefinedBehavior::Strict);
    env.set_auto_escape_callback(|_| AutoEscape::Html);
    env.set_recursion_limit(64);
    env.set_fuel(Some(100_000));
    // DO NOT set a loader — filesystem includes are not allowed (T-16-02).
```

---

### `crates/trackly-app/src/services/act_service.rs` (modified, 2 sites)

**Analog:** itself — `render_pdf` (handover, ctx block ~2620-2651) is the template; `render_acceptance_pdf` (~2778-2799) mirrors it exactly for the same fields.

**Fallback-DTO construction site (must also gain `full_name`, or the struct literal fails to compile)** — appears at both call sites, e.g. `render_pdf` lines ~2541-2557:
```rust
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
                    address_line2: String::new(),
                    // NEW: full_name: String::new(),
                },
                None,
                None,
            ),
```

**`ctx["org"]` assembly** (handover, lines 2620-2632 — acceptance is identical shape at ~2778-2790):
```rust
        let ctx = serde_json::json!({
            "org": {
                "name": org_dto.org_name,
                "inn": org_dto.inn,
                "kpp": org_dto.kpp,
                "address": org_dto.address,
                "address_line2": org_dto.address_line2,
                "phone": org_dto.phone,
                "fax": org_dto.fax,
                "email": org_dto.email,
                "okpo": org_dto.okpo,
                "ogrn": org_dto.ogrn,
                "logo_data_uri": logo_data_uri,
                // NEW: "full_name": org_full_name_html(&org_dto.full_name),
            },
            "act": { /* unchanged */ },
            "return": { /* unchanged */ },
        });
```
The `"full_name"` value must go through the D-03 escape-then-`<br>` helper (`org_full_name_html` per RESEARCH §"HTML-escape → `<br>` порядок") — NOT the raw `org_dto.full_name` string, since the template will interpolate it via `| safe`.

**Render call** (unchanged shape, both sites — handover at 2653-2659):
```rust
        let rendered = crate::pdf::minijinja_env::render_with_timeout(
            &crate::pdf::minijinja_env::build_safe_html_env(),
            "act_handover_html",
            &template_src,
            ctx,
        )
        .await?;
```
This is the call that needs the new `extra_templates`/`_header.html` parameter once `minijinja_env.rs`'s signature changes.

---

### `crates/trackly-app/src/services/report_service.rs` (modified)

**Analog:** `act_service.rs`'s `render_pdf` ctx block (byte-for-byte same `org` shape).

**Context assembly** (lines 637-655):
```rust
        let ctx = serde_json::json!({
            "org": {
                "name": org.org_name,
                "inn": org.inn,
                "kpp": org.kpp,
                "address": org.address,
                "address_line2": org.address_line2,
                "phone": org.phone,
                "fax": org.fax,
                "email": org.email,
                "okpo": org.okpo,
                "ogrn": org.ogrn,
                "logo_data_uri": logo_data_uri,
                // NEW: "full_name": org_full_name_html(&org.full_name),
            },
            "report_name": report_name,
            "period_label": period_label,
            "columns": column_labels,
            "groups": groups,
        });

        crate::pdf::minijinja_env::render_with_timeout(
            &crate::pdf::minijinja_env::build_safe_html_env(),
            "report_html",
            &template_src,
            ctx,
        )
```

---

### `crates/trackly-app/src/services/template_service.rs` (modified — preview stub)

**Analog:** itself — `demo_context_for_kind` (lines 357-378), the shared `org` block reused by all 3 `match kind` arms.

> 🔒 **ВНИМАНИЕ — находка при pattern-mapping.** «Демо»-контекст в исходнике **не демонстрационный**:
> он содержит настоящие реквизиты организации (телефон/факс с реальным кодом города, ОКПО, ОГРН).
> Это уже закоммичено в историю git (тянется с фазы 15, коммит `6ad0202`) — то есть пере-коммит
> ничего нового не раскрывает, но и распространять дальше нельзя. Значения ниже заменены
> плейсхолдерами намеренно; **исполнитель обязан смотреть текущие значения в самом файле, а не
> здесь**. Фаза 34 всё равно правит эту функцию (добавляет ключ `full_name`) — заменить реальные
> значения на вымышленные тем же касанием дёшево и уместно; окончательная чистка HEAD от утечек
> проходит по PRIV-01 (Фаза 37).

```rust
fn demo_context_for_kind(kind: &str) -> serde_json::Value {
    let org = serde_json::json!({
        "name": "<...>",         // 🔒 значения в исходнике ЗАМЕНЕНЫ здесь плейсхолдерами
        "inn": "<...>",          //    — см. предупреждение ниже
        "kpp": "<...>",
        "address": "<...>",
        "address_line2": "<...>",
        "logo_data_uri": null,
        "phone": "<...>",
        "fax": "<...>",
        "email": "<...>",
        "okpo": "<...>",
        "ogrn": "<...>"
        // NEW: "full_name": "Общество с ограниченной ответственностью\n«Пример»"
    });
    match kind {
        "act_acceptance" => serde_json::json!({ "org": org, /* ... */ }),
        "report" => serde_json::json!({ "org": org, /* ... */ }),
        // act_handover (default/fallback) also uses `org` — see file for full match
    }
}
```
Since `full_name` is only sourced ONCE (in the shared `org` binding, not per-arm), adding it here is a single-line change that reaches all three preview kinds simultaneously — this is exactly why `UndefinedBehavior::Strict` risk (Pitfall 2 in RESEARCH) is lower for this call site than for the three service.rs sites (which each build `org` independently and must each remember the new key). Use a **fictional multi-line demo value** here (e.g. two/three lines joined by `\n`, matching D-02's multiline intent) — this file is a Rust source string literal, not user input, but per the privacy constraint it must still be fictional (`«Демо Организация»`, never the real org name).

---

### `crates/trackly-app/src/dto/reports.rs` — `OrgSettingsDto` / `OrgPatch` (modified)

**Analog:** itself — `address_line2` (Phase 20/ORG-02) is the field to mirror exactly; it is the most recently added sibling field in both structs.

**`OrgPatch`** (lines 172-190):
```rust
/// Partial update payload for organisation settings (SET-01).
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
    /// Second address line (ORG-02, Phase 20). Empty string = not filled in.
    pub address_line2: String,
    // NEW: /// Full legal name, multiline (DOC-05, Phase 34). Empty = not filled in.
    // NEW: pub full_name: String,
}
```

**`OrgSettingsDto`** (lines 204-225):
```rust
/// Read-only view of org settings returned to the frontend (SET-01/02).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct OrgSettingsDto {
    pub org_name: String,
    pub inn: String,
    pub kpp: String,
    pub address: String,
    pub has_logo: bool,
    pub phone: String,
    pub fax: String,
    pub email: String,
    pub okpo: String,
    pub ogrn: String,
    /// Second address line (ORG-02, Phase 20). Empty string = not filled in.
    pub address_line2: String,
    // NEW: /// Full legal name, multiline (DOC-05, Phase 34). Empty = not filled in.
    // NEW: pub full_name: String,
}
```
Both derive `Serialize, Deserialize, Type` (the `specta::Type` derive that generates the TS binding) — no new derive needed.

---

### `crates/trackly-app/src/services/org_db_service.rs` (modified)

**Analog:** itself — `save_fields` (lines 86-121) and `get_for_pdf` (lines 394-433), both already thread `address_line2` end-to-end; `full_name` is a structural copy.

**`save_fields` UPDATE** (lines 93-121) — append `full_name=?N` to SET and `patch.full_name` to `params!`, renumbering placeholders:
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
                         address_line2=?11, full_name=?12, \
                         updated_at_utc=?13, version=version+1 \
                     WHERE id=1",
                    params![
                        1i64, patch.org_name, patch.inn, patch.kpp, patch.address,
                        patch.phone, patch.fax, patch.email, patch.okpo, patch.ogrn,
                        patch.address_line2, patch.full_name, now
                    ],
                )
                .map(|_| ())
                .map_err(map_rusqlite)
            })
            .await
    }
```
Note the `authorize(caller, &Action::ManageSettings)?` guard at the top — this is the auth pattern every mutation in this service follows (also in `save_logo`/`remove_logo`); the new field rides the existing guard, no new authorization code needed.

**`get_for_pdf` SELECT** (lines 394-433) — column list + struct literal, ordinal-positional (`r.get(N)`):
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
                 phone, fax, email, okpo, ogrn, address_line2, full_name \
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
                        address_line2: r.get(12)?,
                        full_name: r.get(13)?, // NEW — appended, existing ordinals unaffected
                    };
                    let logo_blob: Option<Vec<u8>> = r.get(5)?;
                    let logo_mime: Option<String> = r.get(6)?;
                    Ok((dto, logo_blob, logo_mime))
                },
            )
            .map_err(map_rusqlite)
        })
        .await
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking OrgDbService::get_for_pdf: {e}"),
        })?
    }
```
Appending `full_name` at the END of the SELECT list keeps every existing `r.get(0..12)` ordinal stable — same discipline the `V035__org_settings_address_line2.sql` doc-comment calls out explicitly ("Appended at the end of the column list — existing SELECT/UPDATE ordinal positions in org_db_service.rs are unaffected"). There is also a second, near-identical UPDATE at lines ~347-359 (a different save path in this same file) and a plain `get()` (not `get_for_pdf`) — grep `org_name=?2` / `SELECT org_name` in this file to find all UPDATE/SELECT statements that must gain `full_name` in lockstep; `get_for_pdf` and `save_fields` are the two used by the PDF/HTML render paths and the Settings form respectively, but a full grep is required before considering this touchpoint closed.

---

### Migration — `migrations/V036__org_settings_full_name.sql` (new)

**Analog:** `migrations/V035__org_settings_address_line2.sql` (verbatim, in full):
```sql
-- V035: Organisation address second line (ORG-02).
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
-- PRAGMA user_version = 35.

ALTER TABLE org_settings ADD COLUMN address_line2 TEXT NOT NULL DEFAULT '';

PRAGMA user_version = 35;
```
V036 (next free number; last migration is V035 / `user_version = 35`) is a structural copy: `ALTER TABLE org_settings ADD COLUMN full_name TEXT NOT NULL DEFAULT '';`, `PRAGMA user_version = 36;`, doc-comment explaining D-01/D-02/D-04 rationale (`DEFAULT ''` → independent `{% if %}` guard degrades cleanly, matching this file's own precedent).

---

### `crates/trackly-app/src/tauri_cmds/settings_org.rs` + `crates/trackly-app/src/http/settings_org.rs` (new D-17 status command)

**Analog:** the `templates_list_for_editor` / `handler_templates_list_for_editor` pair — closest existing **read-only**, no-`ManageSettings`-check endpoint that returns per-file template info.

**`build_*` helper + Tauri wrapper** (`tauri_cmds/settings_org.rs:258-262` and `422-428`):
```rust
// build_* helper
pub async fn build_templates_list_for_editor(
    ctx: &AppCtx,
) -> Result<Vec<TemplateEditorItem>, AppError> {
    ctx.templates.list_all_for_editor().await
}

// Tauri command wrapper
#[tauri::command]
#[specta::specta]
pub async fn templates_list_for_editor(
    state: tauri::State<'_, AppCtx>,
) -> Result<Vec<TemplateEditorItem>, AppError> {
    build_templates_list_for_editor(state.inner()).await
}
```
The new command (`build_templates_status` / `templates_status`) follows this exact 2-tier shape: a `pub async fn build_*(ctx: &AppCtx) -> Result<T, AppError>` (business logic, transport-agnostic) plus a thin `#[tauri::command] #[specta::specta]` wrapper that just calls it. **No `resolve_tauri_identity` call** — `templates_list_for_editor` doesn't authenticate/authorize either (read-only, same class of information).

**HTTP handler + router registration** (`http/settings_org.rs:160-172`, `320-372`):
```rust
pub async fn handler_templates_list_for_editor(
    State(ctx): State<AppCtx>,
    session: Session,
) -> Result<Json<Vec<TemplateEditorItem>>, AppErrorResponse> {
    let _identity = session_identity(&session)
        .await
        .map_err(AppErrorResponse::from)?;
    Ok(Json(
        build_templates_list_for_editor(&ctx)
            .await
            .map_err(AppErrorResponse::from)?,
    ))
}
```
```rust
pub fn router() -> Router<AppCtx> {
    Router::new()
        // Read endpoints
        .route("/api/v1/settings_get_org", post(handler_get_org))
        // ...
        .route(
            "/api/v1/templates_list_for_editor",
            post(handler_templates_list_for_editor),
        )
        // ...
}
```

> **Correction to RESEARCH.md's suggestion:** RESEARCH proposes `axum::routing::get` for the new
> status endpoint because it's read-only. **The actual codebase convention is uniformly `post`
> for every route in this router — including every read endpoint** (`settings_get_org`,
> `settings_get_org_logo`, `templates_list_for_editor`, all POST). There is no `get(...)` call
> anywhere in `http/settings_org.rs`'s `router()`. Follow the real convention: register the new
> `/api/v1/templates_status` route with `post(handler_templates_status)`, not `get`, for
> consistency with every sibling route in this file — deviating to GET would be a one-off
> inconsistency this codebase does not otherwise have.

**Mutation-pattern contrast** (for reference — the new command does NOT need this, since it's read-only, but shown so the planner doesn't accidentally add it): mutation handlers add `authorize(&caller, &Action::ManageSettings)` after `session_identity`, e.g. `handler_save_org_fields` (`http/settings_org.rs:178-191`) and the matching Tauri side `settings_save_org_fields` (`tauri_cmds/settings_org.rs:305-313`) which calls `resolve_tauri_identity` first.

**DTO note:** `TemplateEditorItem` (`dto/reports.rs:277-286`) is NOT the right return type to reuse — it's tied to the frozen DB-backed `document_templates` table (`kind`, `body`, `is_default`), a different source of truth than the file-based `templates/` mechanism this phase touches. A new DTO (e.g. `TemplateStatusDto { filename: String, status: TemplateFileStatus, templates_dir: String }`) is required — see "No Analog Found" below.

---

### `ui/src/features/settings/OrgSettings.svelte` (modified — new multiline field)

**Analog:** itself — the `addressLine2` field is the most recently added sibling and the exact 4-touchpoint shape (state / load / save / markup) to replicate for the new field.

**1. `$state` declaration** (line 27):
```ts
  let addressLine2 = $state('');
  // NEW: let fullName = $state('');
```

**2. DTO interface** (lines 9-21) — must also gain the new key or TS typing drifts from the backend:
```ts
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
    address_line2: string;
    // NEW: full_name: string;
  }
```

**3. Load** (`loadOrg`, lines 52-76):
```ts
  async function loadOrg() {
    try {
      const dto = await apiCall<OrgSettingsDto>('settings_get_org', {});
      orgName = dto.org_name;
      // ...
      addressLine2 = dto.address_line2;
      // NEW: fullName = dto.full_name;
      // ...
    } catch (e: unknown) { /* ...toast pattern... */ }
  }
```

**4. Save payload** (`saveOrg`, lines 104-131):
```ts
  async function saveOrg() {
    saving = true;
    try {
      await apiCall<void>('settings_save_org_fields', {
        patch: {
          org_name: orgName,
          inn,
          kpp,
          address,
          address_line2: addressLine2,
          // NEW: full_name: fullName,
          phone,
          fax,
          email,
          okpo,
          ogrn,
        },
      });
      pushToast('success', 'Настройки организации сохранены');
    } catch (e: unknown) { /* ... */ } finally { saving = false; }
  }
```

**5. Markup — `<Input>` pattern to break from** (address-line-2 field, lines 260-268, `<Input>` used for single-line text):
```svelte
    <div class="form-field form-field--full">
      <label class="form-label" for="org-address-line2">Адрес (2-я строка)</label>
      <Input
        id="org-address-line2"
        type="text"
        bind:value={addressLine2}
        placeholder="офис 305, корпус 2"
      />
    </div>
```
**For the new multiline field, use `Textarea.svelte` instead of `Input.svelte`** — a shared component already exists at `ui/src/lib/components/Textarea.svelte` (props: `value` (`$bindable`), `placeholder`, `disabled`, `invalid`, `id`, `rows` (default 3), `oninput`). It is already used elsewhere in the design system, e.g. `ui/src/features/cartridges/CartridgeFormBody.svelte:324-332`:
```svelte
  <div class="field">
    <label class="label" for="cart-notes">Примечания</label>
    <Textarea
      value={notes}
      placeholder="Необязательно"
      id="cart-notes"
      oninput={(v) => (notes = v)}
    />
  </div>
```
Combining both conventions, the new field in `OrgSettings.svelte` should look like:
```svelte
  <script lang="ts">
    // add: import Textarea from '$lib/components/Textarea.svelte';
  </script>

    <div class="form-field form-field--full">
      <label class="form-label" for="org-full-name">Полное юридическое наименование</label>
      <Textarea
        id="org-full-name"
        value={fullName}
        rows={3}
        placeholder={'Общество с ограниченной ответственностью\n«Название»'}
        oninput={(v) => (fullName = v)}
      />
    </div>
```
Uses `.form-field.form-field--full` (grid-column: 1 / -1, same class already used by `org-name`/`org-address`/`org-address-line2`) and `.form-label`, both already defined in this file's `<style lang="scss">` block (lines 373-387) — no new CSS needed, only the new `<div class="form-field form-field--full">` wrapper + `Textarea` swap-in for `Input`.

---

### `ui/src/features/settings/TemplateEditor.svelte` — `VARIABLES_BY_KIND` (modified)

**Analog:** itself — the existing `org.name`/`org.address` entries in all three kind arrays (lines 33-90ish).

```ts
  const VARIABLES_BY_KIND: Record<string, VariableEntry[]> = {
    act_handover: [
      { code: 'org.name', desc: 'название организации' },
      // NEW: { code: 'org.full_name', desc: 'полное юридическое наименование (многострочное)' },
      { code: 'org.inn', desc: 'ИНН' },
      // ...unchanged...
    ],
    act_acceptance: [
      { code: 'org.name', desc: 'название организации' },
      // NEW: { code: 'org.full_name', desc: 'полное юридическое наименование (многострочное)' },
      // ...unchanged...
    ],
    report: [
      { code: 'org.name', desc: 'название организации' },
      // NEW: { code: 'org.full_name', desc: 'полное юридическое наименование (многострочное)' },
      // ...unchanged...
    ],
  };
```
`org.full_name` must be added to **all three** kind arrays (not just `act_handover`), because `_header.html` is `{% include %}`-d by all three parent templates and D-14 makes it a 4th first-class file — every kind's editor-preview variable list must reflect that the header (and therefore this key) is shared.

---

## Shared Patterns

### Transport-wrapper pair (Tauri + HTTP), applies to the D-17 status command
**Source:** `crates/trackly-app/src/tauri_cmds/settings_org.rs` + `crates/trackly-app/src/http/settings_org.rs` (see full excerpt above under that file's section).
**Apply to:** the new `templates_status` command/`handler_templates_status` handler pair.
Shape: one `pub async fn build_X(ctx: &AppCtx, [caller_identity: &Identity,] ...) -> Result<T, AppError>` in `tauri_cmds/*.rs` holding all business logic; a `#[tauri::command] #[specta::specta]` wrapper in the same file that resolves identity (`resolve_tauri_identity`) only if the operation mutates or is access-controlled; a `handler_X` in `http/*.rs` that resolves `session_identity(&session)` (always, even for reads) then optionally calls `authorize(&caller, &Action::ManageSettings)` before delegating to the same `build_X`; and a `.route("/api/v1/x", post(handler_x))` entry in that file's `router()`. **All routes in this router are POST**, including reads — do not introduce a GET route for consistency (see correction note above).

### `UndefinedBehavior::Strict` — every context key must reach 4 render paths
**Source:** `crates/trackly-app/src/pdf/minijinja_env.rs:33` (`env.set_undefined_behavior(UndefinedBehavior::Strict)`), consumed by all context-assembly sites.
**Apply to:** `act_service.rs` (×2 sites incl. fallback-DTO literal), `report_service.rs`, `template_service.rs::demo_context_for_kind`. Any new template key referenced in `_header.html` (`org.full_name`) MUST appear in the `ctx["org"]` object at all 4 sites or the corresponding render path panics with "undefined variable" at runtime, not compile time. Use this as a literal checklist when the new field is wired.

### `| safe` + Rust-side pre-escape, for values that must not be autoescaped
**Source:** `crates/trackly-app/templates/act_handover.html:121-129` (`org.logo_data_uri | safe`, with its adjacent threat-model comment) — the exact prose pattern D-03 requires be replicated for `org.full_name | safe`.
**Apply to:** `_header.html`'s new `.orgName` block, plus the new Rust helper (suggested name `org_full_name_html`) that does `HtmlEscape` (from `minijinja::HtmlEscape`, already available via the existing `minijinja ^2.20` dependency — no new crate) THEN `.replace('\n', "<br />")`, in that order — reversing the order is a stored-XSS vector via the LAN preview (see RESEARCH §"HTML-escape → `<br>` порядок" for the full threat writeup and code sample). Also requires the `minijinja_env.rs:51-52` doc-comment fix noted above (C-04).

### File-loader materialize/upgrade mechanism (no new code needed, only new data)
**Source:** `crates/trackly-app/src/pdf/html_templates.rs` in full — `DEFAULT_HTML_TEMPLATES`, `KNOWN_LEGACY_DEFAULTS`, `materialize_defaults_on_startup`, `upgrade_untouched_defaults_on_startup`, `load_template`.
**Apply to:** `_header.html` slots into `DEFAULT_HTML_TEMPLATES` as a 4th tuple; it does NOT get a `KNOWN_LEGACY_DEFAULTS` entry (new file — nothing to "upgrade from"); `materialize_defaults_on_startup`'s missing-file branch creates it fresh on any install that doesn't have it yet, satisfying delivery to existing installs without any new code path.

### Org DTO column-addition ripple (10-point checklist)
**Source:** `migrations/V035__org_settings_address_line2.sql` end-to-end trail — this exact same trail, now for `full_name`.
**Apply to (in order):** 1) migration `ALTER TABLE ... ADD COLUMN full_name TEXT NOT NULL DEFAULT ''`; 2) `OrgSettingsDto.full_name`; 3) `OrgPatch.full_name`; 4) `OrgDbService::save_fields` UPDATE SET-list + params (and any sibling UPDATE — grep the file); 5) `OrgDbService::get_for_pdf` SELECT list + struct literal, appended at the end to keep existing ordinals stable; 6) `act_service.rs::render_pdf` ctx["org"] (+ its fallback-DTO literal); 7) `act_service.rs::render_acceptance_pdf` ctx["org"]; 8) `report_service.rs::export_pdf` ctx["org"]; 9) `template_service.rs::demo_context_for_kind`'s shared `org` block; 10) `OrgSettings.svelte` state/interface/load/save/markup; 11) `TemplateEditor.svelte::VARIABLES_BY_KIND` (all 3 kinds); 12) doc-comments in all 3 `templates/*.html` (+ new `_header.html`) files' context-variable lists.

---

## No Analog Found

| File/Item | Role | Data Flow | Reason |
|---|---|---|---|
| `TemplateStatusDto` (new struct, likely in `dto/reports.rs` near `TemplateEditorItem`) | model/DTO | — | No existing DTO models "on-disk file vs. bundled-default vs. legacy-snapshot" comparison result for the file-based `templates/` mechanism — `TemplateEditorItem` is DB-table-shaped (`kind`/`body`/`is_default`) and belongs to the frozen `document_templates` path, not reusable. Design freely per RESEARCH's suggested shape (`filename`, `status` enum "current"/"customized", `templates_dir`); no closer analog exists in the codebase to constrain the exact field names. |
| Structural HTML extraction for the render-gate half of `html_header_parity.rs` (extracting `<div class="header">...</div>` specifically, as opposed to `@page {...}`) | test helper | transform | `html_page_parity.rs`'s `extract_page_block` regex (`@page\s*\{[^}]*\}`) works because `@page` has no nested braces; `<div class="header">` DOES have nested `<div>`s (`.logo`, `.orgName`, `.requisites`), so the direct regex analog is unsafe as-is. RESEARCH flags this explicitly (Code Examples section) and recommends marker comments (`<!-- HEADER-START -->`/`<!-- HEADER-END -->`) inside `_header.html` instead of a naive regex — a planner decision, not a copyable pattern. |

## Metadata

**Analog search scope:** `crates/trackly-app/templates/`, `crates/trackly-app/src/pdf/`, `crates/trackly-app/src/services/`, `crates/trackly-app/src/dto/`, `crates/trackly-app/src/tauri_cmds/`, `crates/trackly-app/src/http/`, `crates/trackly-app/tests/`, `migrations/`, `ui/src/features/settings/`, `ui/src/lib/components/`.
**Files read directly:** `act_handover.html`, `act_acceptance.html`, `report.html` (templates + doc-comments + header block + CSS), `_legacy_defaults/v20/` (dir listing), `html_templates.rs` (full), `minijinja_env.rs` (full), `act_service.rs` (both render sites), `report_service.rs` (export_pdf ctx), `template_service.rs` (demo_context_for_kind), `dto/reports.rs` (OrgPatch, OrgSettingsDto, TemplateEditorItem), `org_db_service.rs` (save_fields, get_for_pdf), `V035__org_settings_address_line2.sql`, `tauri_cmds/settings_org.rs` (full command list + build_templates_* + templates_list_for_editor), `http/settings_org.rs` (full: handlers + router), `OrgSettings.svelte` (full), `TemplateEditor.svelte` (VARIABLES_BY_KIND), `Textarea.svelte`, `CartridgeFormBody.svelte` (Textarea usage example), `tests/html_page_parity.rs` (full).
**Privacy check performed:** no real organization name/INN/KPP/address/phone/e-mail copied into this file; all header-markup excerpts are the current *repository* canon (already scrubbed) or the RESEARCH.md's own privacy-scrubbed "Эталон" reconstruction — `target/debug/templates/act_handover.html` (the real, un-scrubbed source) was NOT read directly by this pattern-mapping pass; its structure was taken second-hand from `34-RESEARCH.md`'s already-scrubbed diff inventory.
**Pattern extraction date:** 2026-08-08
