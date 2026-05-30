---
phase: 03
reviewed: 2026-05-30T16:30:00Z
depth: standard
files_reviewed: 28
files_reviewed_list:
  - crates/trackly-app/src/pdf/mod.rs
  - crates/trackly-app/src/pdf/fonts.rs
  - crates/trackly-app/src/pdf/docspec.rs
  - crates/trackly-app/src/pdf/minijinja_env.rs
  - crates/trackly-app/src/pdf/renderer.rs
  - crates/trackly-app/src/services/act_service.rs
  - crates/trackly-app/src/services/organization_service.rs
  - crates/trackly-app/src/services/template_service.rs
  - crates/trackly-app/src/dto/act.rs
  - crates/trackly-app/src/dto/organization.rs
  - crates/trackly-app/src/dto/mod.rs
  - crates/trackly-app/src/http/acts.rs
  - crates/trackly-app/src/http/organization.rs
  - crates/trackly-app/src/http/templates.rs
  - crates/trackly-app/src/tauri_cmds/acts.rs
  - crates/trackly-app/src/tauri_cmds/organization.rs
  - crates/trackly-app/src/tauri_cmds/templates.rs
  - crates/trackly-app/src/context.rs
  - crates/trackly-core/src/domain/acts.rs
  - crates/trackly-core/src/ports/acts.rs
  - crates/trackly-infra/src/repos/acts_sqlite.rs
  - crates/trackly-infra/src/repos/audit_log_sqlite.rs
  - crates/trackly-infra/src/repos/devices_sqlite.rs
  - migrations/V014__acts_indexes_and_status_codes.sql
  - crates/trackly-app/templates/act_handover.minijinja
  - crates/trackly-app/templates/act_acceptance.minijinja
  - ui/src/lib/api/acts.ts
  - ui/src/lib/api/pdf.ts
  - ui/src/lib/api/organization.ts
  - ui/src/lib/api/templates.ts
  - ui/src/lib/components/Modal.svelte
  - ui/src/features/acts/PdfPreviewModal.svelte
  - ui/src/features/acts/DocumentAcceptanceModal.svelte
  - ui/src/features/acts/ReturnModal.svelte
  - ui/src/features/acts/ReturnItemsTable.svelte
  - ui/src/features/devices/DeviceAutocompleteField.svelte
findings:
  critical: 4
  warning: 11
  info: 6
  total: 21
status: issues_found
---

# Phase 3: Code Review Report

**Reviewed:** 2026-05-30
**Depth:** standard
**Status:** issues_found

## Summary

Phase 3 — большой вертикальный срез (Acts CRUD + returns + undo + PDF + UI) поверх Phase 1/2 фундамента. Реализация в целом аккуратна:
- single-writer + `BEGIN IMMEDIATE` discipline соблюдена;
- counter `act_number` инкрементируется атомарно через `UPDATE ... RETURNING`;
- MiniJinja safe-mode настроен корректно (`Strict`, `recursion_limit=64`, `fuel=100_000`, нет loader, 5-секундный wall-clock timeout);
- path-traversal mitigation в `OrganizationService::safe_logo_canonical` правильно использует `canonicalize` + `starts_with(exe_dir.canonicalize())`;
- SQL везде параметризован через `rusqlite::params!`;
- определённость PDF подкреплена pinned metadata + regex safety-net.

Однако обнаружены 4 BLOCKER-уровня дефекта, связанных с целостностью данных:
1. **Возврат не проверяет, что устройство сейчас «в_работе»** — можно вернуть уже-возвращённый device → snapshot для undo указывает на «на_складе», следующий undo неверный.
2. **Дублирование `device_id` в payload возврата не запрещено** — двойной snapshot, undo разваливает invariants.
3. **Количество в возврате не валидируется против handover-quantity** — можно вернуть больше, чем выдавалось.
4. **Логотип организации не рендерится в PDF** — `HeaderBlock.logo_path` принят в DocSpec и сервисом передан, но `renderer::render_docspec` его игнорирует (нет `surface.draw_image`). UX-обещание из ACT-11 («шапка с логотипом») фактически не выполнено.

Поверх — 11 warnings (включая drift archived-флага при прямом update device через DeviceService, и hardcoded UTC+3 в UI без проверки реальной TZ пользователя) и 6 info-уровня замечаний (магические числа, дублирование `build_fts_query` между infra и app, и пр.).

## Critical Issues

### CR-01: PDF логотип не рендерится, хотя HeaderBlock.logo_path передаётся

**Files:**
- `crates/trackly-app/src/pdf/renderer.rs:88-178` (метод `render_docspec`)
- `crates/trackly-app/src/pdf/docspec.rs:28-39` (`HeaderBlock.logo_path: Option<String>`)
- `crates/trackly-app/src/services/act_service.rs:943, 1043` (передача `safe_logo` в context)
- `crates/trackly-app/templates/act_handover.minijinja:21`
- `crates/trackly-app/src/services/organization_service.rs:110-142` (`safe_logo_canonical` зачем-то делает дорогую валидацию пути)

**Category:** Bug (feature gap, ROADMAP success criterion 4 / ACT-11 nicht erfüllt)

**Issue:** `OrganizationService::safe_logo_canonical` тщательно canonicalize-ит и валидирует `logo_path` для пути traversal (это правильно). `act_service::render_pdf` передаёт результат в шаблонный контекст. Шаблоны рендерят его в `header.logo_path` JSON-полю. `serde_json::from_str::<DocSpec>` десериализует в `HeaderBlock.logo_path: Option<String>`. **Но `PdfRenderer::render_docspec` нигде не использует `spec.header.logo_path`** — оно никогда не читается, и `surface.draw_image(...)` ни разу не вызывается в renderer.rs. PDF выходит без логотипа, даже если файл существует и валидный.

Модуль-док в `pdf/mod.rs:5-12` обещает «emits draw calls against embedded DejaVu Sans Regular/Bold cuts», но не упоминает image rendering. Doc-комментарий `HeaderBlock.logo_path` (`docspec.rs:25-27`) утверждает «`logo_path` is an absolute path the renderer resolves; an `Option<String>` so templates can render an org without a logo». Это ложь — renderer его не resolve-ит.

**Impact:** Документы выходят без логотипа. Phase 3 success criterion 4 («PDF с реальными глифами + шаблоны из БД») технически выполнен, но ACT-11 «Печать/PDF Акта приёма-передачи с шапкой+логотипом» (REQUIREMENTS.md, цитировано в RESEARCH.md) — нет. Конечный пользователь увидит «битый» документ.

**Fix:** Один из двух путей.

A) Реализовать image embedding в `render_section`/header drawing (krilla 0.7 поддерживает `Image::from_png_bytes`/`from_jpeg_bytes`). Поскольку logo_path уже canonicalized, можно сделать `std::fs::read(path)?` → определить mime по extension → `Image::from_*` → `surface.draw_image(point, image, w, h)`. Это полноценный fix.

B) Если фича отложена — отразить это явно: в `act_service::render_pdf` НЕ вычислять `safe_logo_canonical` (он делает file I/O + canonicalize впустую), убрать `logo_path` из `HeaderBlock`, и обновить doc-комментарии + добавить запись в `03-VALIDATION.md` или `03-05-SUMMARY.md` про deferred-фичу.

```rust
// renderer.rs — пример A
if let Some(logo_path_str) = &spec.header.logo_path {
    if let Ok(bytes) = std::fs::read(logo_path_str) {
        // determine mime by extension; krilla::Image::from_png_bytes / from_jpeg_bytes
        // surface.draw_image(Point::from_xy(A4_WIDTH_PT - MARGIN_PT - 100.0, MARGIN_PT), &img, 100.0, 50.0);
    }
}
```

---

### CR-02: Возврат не валидирует, что устройство сейчас в «в_работе»

**File:** `crates/trackly-app/src/services/act_service.rs:362-584` (метод `do_return`)

**Category:** Bug (data integrity, undo chain corruption)

**Issue:** В `do_return` нет проверки `device.status_id == in_work_status_id` перед обновлением. Любое устройство из `payload.items` будет переведено в «на_складе» независимо от текущего статуса. Это создаёт минимум два class-A проблем:

1. **Двойной возврат**: пользователь оформил возврат по акту №42; через час другой оператор открыл UI (с устаревшим cache) и оформляет возврат тех же позиций по тому же акту №42. Backend принимает: создаёт второй return-акт с `sub_number=2`. В audit_log пишутся `before_json={"status_id": <на_складе>}` — но это уже «на складе», и `after_json={"status_id": <на_складе>}`. Identical snapshot. Если потом удалить второй return — undo восстанавливает «на_складе» (фактический no-op). Это OK для устройств, **но**:

2. **Auto-archive разваливается**: `recompute_parent_archived` пересчитывает по «остатку в работе» (сумма quantities с device.status='в_работе'). Если устройства уже на складе после первого возврата, sum=0, parent.archived=1. Второй return создаст возврат «в архивированный акт». Конфликт по `parent.archived`-check (lines 389-396) не сработает, потому что check был ДО update parent на первом return. Второй return при попытке вернуть в архивный акт получает `AppError::Conflict` (line 390). **Лучшая защита, чем сейчас**, но если первый return был частичным и парент не архивирован, второй return на те же устройства проходит — двойной возврат device.

3. **Cross-act мутации**: если устройство X сейчас выдано по handover-акту №50 (статус «в_работе»), но злонамеренный/ошибочный payload возврата по акту №42 включает X.id — return пройдёт, X переедет в «на_складе», audit пишет «kind: return, act_id: 42», и любой undo возврата 42 восстановит X в состояние «в_работе по акту 50» (что верно). Но акт 50 теперь имеет устройство «на_складе», тогда как `acts.archived=0` — невидимый дрейф состояния.

**Impact:** Тихая порча данных. ACT-13 («транзакционная гарантия, всё или ничего») формально соблюдён в рамках одной транзакции, но cross-tx consistency не защищена.

**Fix:** Добавить проверку статуса И принадлежности устройства актам в `do_return` цикле:

```rust
// внутри for item in &payload.items { ... }
let before = devices_repo.get_in_tx(&tx, item.device_id)?;
if before.status_id != Some(in_work_status_id) {
    return Err(AppError::Conflict {
        reason: format!(
            "Устройство id={} сейчас не «в работе» (status_id={:?}) — возможно, уже возвращено",
            item.device_id, before.status_id,
        ),
    });
}
// Также проверка, что device в данный момент привязан именно к этому handover-акту:
// (по audit_log самой свежей mutation) ИЛИ дополнительная колонка acts.in_work_act_id.
```

В дополнение добавить интеграционный тест: «двойной возврат тех же устройств → второй вызов возвращает Conflict».

---

### CR-03: Дублирование device_id в payload возврата не запрещено

**File:** `crates/trackly-app/src/services/act_service.rs:399-417, 472-546` (validation block + items loop в `do_return`)

**Category:** Bug (data integrity)

**Issue:** `validate_return` (lines 328-360) не проверяет уникальность `device_id`/`act_item_id` в `payload.items`. Каждый item идёт через цикл независимо. При дублирующих device_id:

1. Первая итерация: `before` = snapshot статуса «в_работе»; `update_full_in_tx` → status «на_складе»; audit пишет `before_json` с «в_работе».
2. Вторая итерация для того же device_id: `get_in_tx` снова читает device — но теперь status уже «на_складе» (из шага 1). `before_json` пишется со status «на_складе». Update переводит в «на_складе» (no-op). Audit log имеет ДВА before_json для одного устройства, последний из которых — «на_складе».

При undo `select_device_mutations_for_act` возвращает обе записи в insert-order, и replay восстанавливает (1) «в_работе» → (2) «на_складе». **Последняя выигрывает.** Устройство не возвращается в исходное состояние.

Дополнительно: `INSERT INTO act_items` создаёт две строки с одним device_id для одного return-акта, что заваливает отчётность.

**Fix:** В `validate_return` добавить дедупликацию:

```rust
fn validate_return(payload: &ActReturnDto) -> Result<(), AppError> {
    // ... existing checks ...
    let mut seen_act_items = std::collections::HashSet::new();
    let mut seen_device_ids = std::collections::HashSet::new();
    for (idx, it) in payload.items.iter().enumerate() {
        if !seen_act_items.insert(it.act_item_id) {
            return Err(AppError::Validation {
                field: format!("items[{idx}].act_item_id"),
                message: format!("act_item_id={} продублирован в возврате", it.act_item_id),
            });
        }
        if !seen_device_ids.insert(it.device_id) {
            return Err(AppError::Validation {
                field: format!("items[{idx}].device_id"),
                message: format!("device_id={} продублирован в возврате", it.device_id),
            });
        }
        // ... остальная per-item validation ...
    }
    Ok(())
}
```

---

### CR-04: Возврат не проверяет quantity против исходного handover

**File:** `crates/trackly-app/src/services/act_service.rs:399-417` (валидация act_item_id), `:505-512` (insert_act_item_in_tx с произвольным `item.quantity`)

**Category:** Bug (data integrity)

**Issue:** `validate_return` проверяет только `quantity >= 1` (line 336-341). Цикл insert вставляет `item.quantity` без cross-проверки против `act_items.quantity` родительского handover-акта.

Пример: handover-акт №42 выдал device_id=5 в количестве 1 (act_items: quantity=1). Пользовательский payload возврата `items: [{ device_id: 5, quantity: 100 }]`. Service вставит return-act_item с quantity=100 — больше, чем было выдано.

`recompute_parent_archived` (lines 442-449) использует SUM(quantity) JOIN devices WHERE status='в_работе'. Поскольку device_id=5 после return = «на_складе», его quantity не считается → SUM = 0 → archived=1. Здесь автоархив не сломается. Но:

- Отчёты по quantity-выдано/возвращено покажут –99 или ассиметричные суммы.
- Если возврат удаляется (undo), `recompute_parent_archived` пересчитает на основе восстановленного device.status='в_работе'; используется handover act_items quantity (=1), что корректно — но retroactive view «было возвращено: 100» противоречит «было выдано: 1».

**Fix:** Подтянуть исходное `act_items.quantity` родительского handover и проверить `return.quantity <= handover.quantity - already_returned_for_this_item`:

```rust
// в цикле validation:
let handover_qty: i64 = tx.query_row(
    "SELECT quantity FROM act_items WHERE id = ?1 AND act_id = ?2",
    params![it.act_item_id, act_id], |r| r.get(0),
).map_err(map_rusqlite)?;
let already_returned: i64 = tx.query_row(
    "SELECT COALESCE(SUM(rai.quantity), 0) FROM act_items rai \
     JOIN acts ra ON ra.id = rai.act_id \
     WHERE ra.parent_act_id = ?1 AND ra.deleted_at_utc IS NULL \
       AND rai.device_id = ?2",
    params![act_id, it.device_id], |r| r.get(0),
).map_err(map_rusqlite)?;
if it.quantity + already_returned > handover_qty {
    return Err(AppError::Validation {
        field: format!("items[{idx}].quantity"),
        message: format!(
            "Возврат превышает выданное количество: {}+{} > {}",
            already_returned, it.quantity, handover_qty,
        ),
    });
}
```

## Warnings

### WR-01: `archived` флаг handover-акта дрейфует при прямом update device через DeviceService

**Files:**
- `crates/trackly-app/src/services/device_service.rs` (любой update path; не модифицирован Phase 3)
- `crates/trackly-app/src/services/act_service.rs:549, 859` (`recompute_parent_archived` вызывается ТОЛЬКО из `do_return`/`delete_soft`)

**Category:** Bug (data integrity, derived-state drift)

**Issue:** D-Archive-01 declares: archived — это derived state по «остатку в работе». Реализация хранит флаг в `acts.archived` и обновляет ТОЛЬКО в `act_service::do_return` (после возврата) и `act_service::delete_soft` (после undo возврата).

Однако `DeviceService::update` позволяет изменить `status_id` устройства (например, оператор вручную помечает «На ремонте» или «Списано» через UI на странице устройств). Это устройство больше не «в_работе», но `acts.archived` не пересчитывается. Архивный handover может содержать устройства не на складе.

Обратный сценарий: handover архивирован (все вернулось). Оператор вручную меняет device.status на «в_работе». `acts.archived=1` остаётся — но логически архив противоречит наличию активного устройства.

**Fix:** Два варианта:

1. После каждого `DeviceService::update`, который меняет `status_id`, найти все handover-акты, в которых участвует это устройство, и вызвать `recompute_parent_archived`. Дороже, но честный derived state.
2. Сделать `archived` ВЫЧИСЛЯЕМЫМ полем в SELECT (view или generated column), а не хранимым. Дороже на чтение, проще на запись.

Рекомендую (1) с indexed lookup через `idx_act_items_device_id` (уже есть в V014).

```rust
// в DeviceService::update после tx UPDATE devices:
if patch.status_id.is_some() {
    let parent_acts: Vec<i64> = tx.prepare(
        "SELECT DISTINCT a.id FROM acts a \
         JOIN act_items ai ON ai.act_id = a.id \
         WHERE ai.device_id = ?1 AND a.act_type = 'handover' \
           AND a.deleted_at_utc IS NULL"
    )?.query_map(...)?...collect()?;
    for parent_id in parent_acts {
        recompute_parent_archived(&tx, parent_id, now)?;
    }
}
```

---

### WR-02: `compute_suffix_from_display` fragile prefix-strip

**File:** `crates/trackly-app/src/services/act_service.rs:1140-1153`

**Category:** Bug

**Issue:** Helper извлекает суффикс из отформатированного display. Логика:

```rust
if let Some(rest) = display.strip_prefix(&raw_str) {
    rest.to_string()
} else {
    if let Some(idx) = display.find('в') { display[idx..].to_string() } else { String::new() }
}
```

Проблемы:
1. Если `number_raw = 42` и display случайно — «421в» (что невозможно для текущего format_act_number, но компонент изолированный), `strip_prefix("42")` вернёт «1в», вместо ожидаемого «в». Завязка на инвариант format_act_number опасна.
2. Падение на ASCII «B» вместо Cyrillic «в» возможно, если кто-то поменяет format_act_number. Но «в» — это U+0432.
3. Логика проще, если `format_act_number` напрямую возвращал бы tuple `(display, suffix)`.

**Fix:** Заменить compute_suffix on direct call:

```rust
// В act_service::render_pdf:
let suffix = match (act.act_type.as_str(), act.sub_number, /* sibling_count */) {
    ("handover", _, _) => "".to_string(),
    ("return", Some(sub), Some(1)) => "в".to_string(),
    ("return", Some(sub), _) => format!("в{sub}"),
    _ => "".to_string(),
};
```

Это извлекаемое из тех же входов, что `format_act_number` использует, и не парсит уже-отформатированную строку.

---

### WR-03: format_act_number suppress sub_number incorrectly when sub_number != 1

**File:** `crates/trackly-app/src/dto/act.rs:23-42`

**Category:** Bug (edge case)

**Issue:**

```rust
if sibling_return_count == Some(1) {
    format!("{parent}в")
} else {
    format!("{parent}в{sub}")
}
```

Семантика «42в» означает «единственный возврат для парента №42 → не показываем sub». Но представим: создано два возврата (`sub=1, sub=2`), затем `sub=1` soft-удалён. Теперь `sibling_return_count = 1` (count считает только non-deleted), и единственный сохранившийся return имеет `sub_number = 2`. Display = «42в», но **в БД sub_number=2** — UI показывает «42в», PDF снимков (старых) показывает «42в2». Cognitive dissonance + поломка sortability.

**Fix:** Проверять, что `sibling_return_count == Some(1)` И `sub_number == Some(1)`:

```rust
if sibling_return_count == Some(1) && sub_number == Some(1) {
    format!("{parent}в")
} else {
    format!("{parent}в{sub}")
}
```

Если sub_number=2 — это значит, что когда-то был sub=1, и retroactive promotion с «42в» уже не применима — суффикс должен оставаться.

---

### WR-04: SQL injection через build_fts_query невозможна, но `%`/`_` escape реализован стрипом, а не ESCAPE-клаузой

**File:** `crates/trackly-app/src/services/act_service.rs:636-647`

**Category:** Security (defense-in-depth)

**Issue:**

```rust
let cleaned: String = trimmed
    .chars()
    .map(|c| if c == '%' || c == '_' { ' ' } else { c })
    .collect();
let plain_query = format!("%{}%", cleaned.trim());
```

Подход «убрать `%`/`_`» — безопасный, но семантически меняет пользовательский запрос. Поиск по «file_name» становится поиском «file name» с двумя токенами (split в FTS) и LIKE «%file name%». Пользователь не получит ожидаемый результат.

Канонический способ — использовать ESCAPE-клаузу: `WHERE name LIKE ? ESCAPE '\'` + escape `%`, `_`, `\` в pattern.

**Fix:**

```rust
fn like_escape(s: &str) -> String {
    s.chars().flat_map(|c| match c {
        '\\' => vec!['\\', '\\'],
        '%' => vec!['\\', '%'],
        '_' => vec!['\\', '_'],
        other => vec![other],
    }).collect()
}
let plain_query = format!("%{}%", like_escape(trimmed));
// + изменить SQL на LIKE ?1 ESCAPE '\\'
```

В `acts_sqlite::search_acts` SQL → `LIKE ?1 ESCAPE '\'`.

---

### WR-05: `recompute_parent_archived` инкрементирует `version` всегда, даже без изменения archived

**File:** `crates/trackly-infra/src/repos/acts_sqlite.rs:424-459`

**Category:** Bug (optimistic lock UX)

**Issue:** Doc-комментарий гласит «Идемпотентно: если значение не изменилось, version всё равно инкрементируется». Это вызовет `OptimisticLockMismatch` при следующем update handover-акта, если UI закэшировал старый version.

Сценарий: пользователь открыл detail handover (получил version=1). Параллельно (через другой modal/окно) оформил возврат → recompute поднял version до 2. Пользователь нажимает «Удалить акт» → `delete_soft(id, version=1)` → backend читает version=2 → `OptimisticLockMismatch{expected:1, actual:2}` → UX-frustration.

**Fix:** Перед UPDATE сравнить, изменился ли archived:

```rust
let current_archived: i64 = tx.query_row(
    "SELECT archived FROM acts WHERE id = ?1", [parent_act_id], |r| r.get(0),
)?;
let archived = if remaining == 0 { 1 } else { 0 };
if current_archived == archived {
    return Ok(archived == 1);  // no-op, без version bump
}
tx.execute(
    "UPDATE acts SET archived=?1, updated_at_utc=?2, version=version+1 WHERE id=?3",
    params![archived, now_utc, parent_act_id],
)?;
```

---

### WR-06: `organization_get` команда может писать файл при первом вызове из HTTP (Phase 5 attack surface)

**File:** `crates/trackly-app/src/services/organization_service.rs:66-96`

**Category:** Security (side-effect from read-style command)

**Issue:** `read()` named like read но при отсутствии файла вызывает `std::fs::write(&path, ...)` — это write-effect инициируемый read-командой. В Phase 3 это desktop-only, но в Phase 5 axum router строится сейчас (`http::organization::router()`), и `handler_get` уже подключён к `organization_get` → POST `/api/v1/organization_get` будет писать на диск.

Когда server-mode заработает, любой LAN-пользователь триггернёт создание файла на сервере. Не security disaster, но необычное side-effect.

**Fix:** Разделить:

```rust
pub async fn read_or_placeholder(&self) -> Result<OrgData, AppError> { /* read-only; вернуть placeholder без записи если файла нет */ }
pub async fn ensure_file_exists(&self) -> Result<OrgData, AppError> { /* startup-only — вызывать ОДНИН РАЗ в AppCtx::build */ }
```

И в `AppCtx::build` после `seed_defaults_on_startup` добавить `organization.ensure_file_exists().await?;`. Read-команды используют `read_or_placeholder` без побочного эффекта.

---

### WR-07: Hardcoded UTC+3 offset в DocumentAcceptanceModal не валидирует реальную TZ

**File:** `ui/src/features/acts/DocumentAcceptanceModal.svelte:48-60`

**Category:** Bug (timezone assumption)

**Issue:**

```ts
function dateLocalToUtcSeconds(dateStr: string): number {
    const [y, m, d] = dateStr.split('-').map(Number);
    const utcMs = Date.UTC(y, (m ?? 1) - 1, d ?? 1, 0, 0, 0) - 3 * 3600 * 1000;
    return Math.floor(utcMs / 1000);
}
```

Захардкожен MSK offset (-3 часа). Если пользователь:
- запустил приложение на ноутбуке в Калининграде (UTC+2) — дата сместится на день назад;
- временно в командировке в Лондоне (UTC+0) — выбранная дата 28 мая отрендерится как 27 мая.

CONTEXT упоминает «RU-only single-tz», но реальной валидации TZ нет. Также `format_ru_date` в Rust считает unix_seconds как UTC (не MSK):

```rust
let odt = time::OffsetDateTime::from_unix_timestamp(unix_seconds).unwrap();
let day = odt.day();  // UTC day
```

Hardcoded offset на UI пытается компенсировать UTC-чтение на backend, но это связка двух нестабильных предположений.

**Fix:** Один из вариантов:
1. Передавать `date_str: "2026-05-28"` (ISO date string) с UI вместо unix seconds; backend парсит как «полночь UTC того дня» через `time::Date::from_calendar_date(...)`. Никаких offset-вычислений.
2. На backend хранить дату как `INTEGER` юлианский день или `TEXT` YYYY-MM-DD.

Опция (1) — минимальный diff:

```ts
// UI: передаём строку
onSubmit({ deviceId: device.id, giverName, receiverName, dateIso: dateLocal });
```

```rust
// backend: render_acceptance_pdf принимает String "YYYY-MM-DD"
pub async fn render_acceptance_pdf(
    &self, device_id: i64, giver_name: String, receiver_name: String, date_iso: String,
) -> Result<Vec<u8>, AppError> {
    let (y, m, d) = parse_iso_date(&date_iso)?;
    let date_human = format!("{d} {} {y} г.", MONTHS_RU[(m-1) as usize]);
    // ...
}
```

---

### WR-08: PdfPreviewModal делает второй network call (re-render PDF) для save/open вместо использования закэшированных bytes

**File:** `ui/src/features/acts/PdfPreviewModal.svelte:118-204` (effect + handleSave + handleOpen)

**Category:** Bug (UX, race)

**Issue:** `$effect` создаёт `blobUrl` из bytes, но не сохраняет bytes в state (комментарий lines 131-134: «We can pull them back from the blob via blob.arrayBuffer() in the handlers — but it's simpler to re-call backend if user saves»).

Когда пользователь нажимает Save или Open, handler делает `pdfBytes ?? (await renderCall())` — второй PDF generation. Race-условие: если PDF rendering недетерминирован (хотя бы в reality, между сессиями), saved bytes != preview bytes. CR-PDF-Determinism теоретически гарантирует, но safety-net regex заменяет /Producer и /CreationDate — если их не оказалось, output меняется при разных запусках krilla версий.

Также: 2× работа backend (один раз для preview, один раз для save) — впустую.

**Fix:** Сохранить bytes в state при первом рендере:

```ts
$effect(() => {
    // ...
    (async () => {
        try {
            const bytes = await renderCall();
            if (cancelled) return;
            pdfBytes = bytes;  // CACHE
            const blob = new Blob([new Uint8Array(bytes)], { type: 'application/pdf' });
            createdUrl = URL.createObjectURL(blob);
            blobUrl = createdUrl;
        } catch (e) { ... }
    })();
});

async function handleSave() {
    if (!pdfBytes) return;  // single source of truth
    // ... use pdfBytes directly ...
}
```

---

### WR-09: `act_service::do_return` not checking device.location_id when bulk-fallback used

**File:** `crates/trackly-app/src/services/act_service.rs:480-502`

**Category:** Bug (validation gap)

**Issue:**

```rust
let effective_location: Option<i64> = per_row_loc_id.or({
    if payload.apply_to_all { resolved_bulk_location_id } else { None }
});
```

`update_full_in_tx` принимает `location_id: Option<i64>` и записывает `?2` напрямую. Если `effective_location == None` (т.е. user не задал ни bulk, ни per-row), `location_id` в `devices` устанавливается в NULL — устройство теряет привязку к локации.

Validation в `validate_return` (lines 343-356) требует per-row location ТОЛЬКО при `apply_to_all = false`. Но при `apply_to_all = true` пользователь может оставить и bulk пустым, и per-row пустыми — все check pass → backend записывает `location_id = NULL`.

**Fix:**

```rust
fn validate_return(payload: &ActReturnDto) -> Result<(), AppError> {
    // ...
    if payload.apply_to_all && payload.bulk_location_id.is_none() && payload.bulk_location_name.as_deref().map_or(true, str::is_empty) {
        // bulk не задан — каждая checked-row обязана иметь свой location
        for (idx, it) in payload.items.iter().enumerate() {
            if it.location_id_override.is_none() && it.location_name_override.as_deref().map_or(true, str::is_empty) {
                return Err(AppError::Validation {
                    field: format!("items[{idx}].location"),
                    message: "Расположение возврата обязательно (нет bulk-значения)".into(),
                });
            }
        }
    }
    Ok(())
}
```

---

### WR-10: Регулярки в `normalize_pdf_for_determinism` компилируются на каждый рендер

**File:** `crates/trackly-app/src/pdf/renderer.rs:310-329`

**Category:** Quality (out-of-scope perf, but minor correctness drift)

**Issue:** Doc-комментарий явно объясняет, что regex компилируются per-call для избежания «multi-threading surface». Но `regex::bytes::Regex::new` сама по себе thread-safe (returns `Send + Sync`), `OnceLock<Regex>` тривиально работает. Многократная компиляция — лишний CPU и неоправданный обоснование.

Также: regex `/CreationDate \(D:[^)]*\)` не обрабатывает PDF литералы с escaped `\)`. PDF spec позволяет `\(` и `\)` в string literals — `/CreationDate (D:202601\)01...)` теоретически валиден, но `[^)]*` остановится на `\)`. Krilla не должен такое генерировать, но safety net хрупкий.

**Fix:**

```rust
use std::sync::OnceLock;
fn re_creation() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"/CreationDate ?\([^)]*\)").unwrap())
}
// usage: re_creation().replace_all(...)
```

---

### WR-11: `safe_logo_canonical` использует `tracing::warn` для пути traversal

**File:** `crates/trackly-app/src/services/organization_service.rs:118-122, 130-135`

**Category:** Security (logging consistency)

**Issue:** Когда файл логотипа не найден — `tracing::warn`. Когда обнаружен path-traversal attempt — `Err(AppError::Validation { ... })`, без логирования. Path-traversal — это suspicious activity (admin отредактировал org.json со зловредным logo_path); должно быть `tracing::warn!` (или `error!`) с контекстом, чтобы CI/devops могли отследить попытки.

**Fix:**

```rust
if !canonical.starts_with(&exe_canonical) {
    tracing::warn!(
        attempted_path = %canonical.display(),
        exe_dir = %exe_canonical.display(),
        "Path traversal attempt в org.logo_path — отклонено"
    );
    return Err(AppError::Validation {
        field: "org.logo_path".to_string(),
        message: format!("Путь к логотипу вне рабочей папки: {}", canonical.display()),
    });
}
```

## Info

### IN-01: Дублирование `build_fts_query` между infra и app

**Files:**
- `crates/trackly-infra/src/repos/devices_sqlite.rs:75-83`
- `crates/trackly-app/src/services/act_service.rs:1286-1294`

**Category:** Quality (DRY)

Тело идентичное. Комментарий в act_service объясняет «кросс-crate publish не оправдан в Phase 3». Тем не менее, изменение в одном без другого создаёт inconsistency. Минимальный fix: переместить в публичный helper в `trackly-infra::repos::fts::build_query` или (более правильно) в новый `trackly-core::primitives::fts` (без зависимости на rusqlite).

### IN-02: Magic numbers в renderer (font sizes, margins)

**File:** `crates/trackly-app/src/pdf/renderer.rs:46-59, 135, 144, 153, 162, 207-211`

Константы `HEADING_SIZE_PT = 14.0`, `BODY_SIZE_PT = 10.0`, MARGIN_PT — это OK на уровне модуля, но в `render_section` встречаются inline magic: `y += HEADING_SIZE_PT + 4.0` (4.0 — что?), `y += BODY_SIZE_PT + 16.0` (16.0 — спейсер?). Стоит вынести `LINE_GAP_PT`, `BLOCK_GAP_PT` константы.

### IN-03: ActService.PdfPipelineRefs использует `&Arc<T>`

**File:** `crates/trackly-app/src/services/act_service.rs:1086-1090`

```rust
struct PdfPipelineRefs<'a> {
    templates: &'a Arc<TemplateService>,
    organization: &'a Arc<OrganizationService>,
    pdf: &'a Arc<PdfRenderer>,
}
```

`&'a Arc<T>` — нестандартный антипаттерн (clippy lint `needless_borrow_for_generic_args`). Лучше:

```rust
struct PdfPipelineRefs<'a> {
    templates: &'a TemplateService,
    organization: &'a OrganizationService,
    pdf: &'a PdfRenderer,
}
```

И `.as_ref()` при build.

### IN-04: `_act_item_row_ref` — мертвая функция с allow(dead_code)

**File:** `crates/trackly-app/src/services/act_service.rs:1267-1268`

```rust
#[allow(dead_code)]
fn _act_item_row_ref(_r: &ActItemRow) {}
```

Suppression dead_code для re-export hint. Сегодня лишний — `ActItemRow` нигде не используется напрямую через service. Удалить или сделать настоящий re-export через `pub use`.

### IN-05: `acts.ts` Tauri command имена в snake_case — но `actId` camelCase передаётся

**File:** `ui/src/lib/api/acts.ts:30-31, 40-41`

```ts
doReturn: (actId: number, payload: ActReturnDto) =>
    apiCall<ActDto>('acts_return', { actId, payload }),
```

Tauri-specta автоматически конвертирует camelCase JS args в snake_case Rust args (это документировано в S-5). Работает. Минор: было бы яснее в comment-блоке сверху уточнить, что `actId` → backend `act_id`.

### IN-06: `act_acceptance.minijinja` не использует `org.logo_path`

**File:** `crates/trackly-app/templates/act_acceptance.minijinja:17`

Шаблон передаёт `logo_path` в header, но как и CR-01, renderer его не рендерит. Если CR-01 будет исправлен через путь B (deferred), удалить и из шаблона тоже.

---

_Reviewed: 2026-05-30_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
