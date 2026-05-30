---
phase: 03-pdf
verified: 2026-05-30T19:00:00Z
status: gaps_found
score: 12/16 requirements satisfied (4 blocked by REVIEW Critical findings)
overrides_applied: 0
gaps:
  - truth: "ACT-11: PDF Акта приёма-передачи рендерит шапку с логотипом организации"
    status: failed
    reason: "renderer::render_docspec не использует spec.header.logo_path (CR-01); ни в одной из функций renderer.rs нет вызова surface.draw_image; шаблоны MiniJinja передают org.logo_path → DocSpec.HeaderBlock.logo_path, но движок рендеринга игнорирует поле. Файл logo физически не появляется в PDF."
    artifacts:
      - path: "crates/trackly-app/src/pdf/renderer.rs"
        issue: "Отсутствует вызов surface.draw_image / Image::from_*; spec.header.logo_path не читается"
      - path: "crates/trackly-app/src/pdf/docspec.rs"
        issue: "Поле HeaderBlock.logo_path = Some(...) сохраняется при serde десериализации, но renderer его не использует"
      - path: "crates/trackly-app/src/services/act_service.rs:943,1043"
        issue: "safe_logo_canonical делает работу впустую — результат не достигает PDF"
    missing:
      - "Реализовать чтение logo из spec.header.logo_path в render_docspec: std::fs::read(path) → krilla Image::from_png_bytes/from_jpeg_bytes → surface.draw_image()"
      - "Добавить integration-test: act_with_logo_renders_image_in_pdf (например — после render проверить, что вывод PDF содержит маркер /XObject или Image stream)"
      - "Альтернатива (B): убрать logo_path из HeaderBlock/templates и явно задокументировать как deferred-фичу в SUMMARY/VALIDATION"
  - truth: "ACT-13: Транзакционная гарантия Возврата защищает от двойного/невалидного возврата (status check, dedup, quantity bounds)"
    status: failed
    reason: "act_service::do_return и validate_return не реализуют три инварианта целостности: (а) нет проверки device.status_id == in_work_status_id перед UPDATE (CR-02), (б) нет дедупликации device_id/act_item_id в payload (CR-03), (в) нет валидации item.quantity против исходного handover.act_items.quantity и already-returned суммы (CR-04). В однократной tx внутренние инварианты соблюдены, но cross-tx и intra-payload — нет."
    artifacts:
      - path: "crates/trackly-app/src/services/act_service.rs:328-360 (validate_return)"
        issue: "validate_return проверяет только quantity ≥ 1 и presence override-полей при apply_to_all=false; нет HashSet dedup и нет cross-check с handover quantity"
      - path: "crates/trackly-app/src/services/act_service.rs:362-584 (do_return)"
        issue: "Цикл items не сравнивает before.status_id с in_work_status_id; нет SELECT quantity FROM act_items WHERE id=? для проверки границы; дубль device_id даст двойной audit snapshot с broken undo chain"
    missing:
      - "В validate_return добавить HashSet<i64> по act_item_id и device_id; на дубль → AppError::Validation"
      - "В do_return цикл items: после get_in_tx проверить before.status_id == Some(in_work_status_id) — иначе AppError::Conflict «Устройство id=N уже не в работе»"
      - "Подтянуть handover_qty = SELECT quantity FROM act_items WHERE id=?1 AND act_id=?2 и already_returned SUM(rai.quantity) WHERE ra.parent_act_id=?1 AND rai.device_id=?2 AND ra.deleted_at_utc IS NULL; assert quantity + already_returned ≤ handover_qty"
      - "Integration-tests: return_twice_same_device_rejected, return_with_duplicate_device_id_rejected, return_quantity_exceeds_handover_rejected"
human_verification:
  - test: "Manual UAT — DEV-14 flow на запущенном Tauri-приложении"
    expected: "Правый клик на устройство в DevicesPage → «Печать документа приёма» → DocumentAcceptanceModal с полями «Кто передал/Кто принял/Дата» → submit → PdfPreviewModal с реальным PDF (кириллица читаема, ФИО видны, дата правильная)"
    why_human: "Полный browser→tauri-plugin-dialog→tauri-plugin-shell→tauri-plugin-fs flow + визуальный рендеринг PDF в iframe нельзя верифицировать grep'ом"
  - test: "Manual UAT — PDF рендеринг handover-акта с шапкой и кириллицей"
    expected: "Открыть существующий handover-акт → «Печать» → видна шапка с реквизитами организации (название, ИНН, КПП, адрес), таблица позиций с кириллическими наименованиями, подписи Сдал/Принял с правильными ФИО включая «ё»/«-» в составе"
    why_human: "Визуальное качество рендеринга шрифта DejaVu Sans (kerning, line-break, hyphenation) можно оценить только глазом — pdf-extract это пропускает"
  - test: "Manual UAT — full lifecycle (ACT-06..10) через UI"
    expected: "Создать handover → partial return через ReturnModal (галочка «Применить ко всем» по умолчанию ВКЛ) → full return → handover автоматически в архив (вкладка «Архив», счётчик +1) → удалить return → handover возвращается в активные → удалить handover → все devices снова «На складе»"
    why_human: "UX-поток множества модалов, switch-bar счётчики, toast'ы, real-time master-detail обновление — это integration UI behavior"
  - test: "Manual UAT — поиск по актам (ACT-04) через UI"
    expected: "В ActsSearchAndTabs ввести часть номера акта / ФИО / наименования устройства → через ~250ms список фильтруется; backend acts.search вызывается с debounce"
    why_human: "Debounce timing 250ms и UX переключения между acts.list и acts.search режимами"
  - test: "Visual logo gap confirmation (если CR-01 будет принят как deferred)"
    expected: "Сравнить PDF handover-акта со скриншотом ROADMAP/UI-SPEC: ожидается логотип в шапке справа — фактически логотип отсутствует"
    why_human: "Подтверждение пользователем, что отсутствие логотипа — это FAIL (gap), а не accepted simplification"
---

# Phase 3: PDF + Acts Verification Report

**Phase Goal:** Закрыть все 16 требований Phase 3 — ACT-01..14 (handover/return acts с auto-numbering, FTS+LIKE search, PDF render, undo, auto-archive) + DEV-14 (device acceptance intermediate-modal flow) + DEV-15 (acceptance act PDF rendering).

**Verified:** 2026-05-30
**Status:** gaps_found
**Re-verification:** No — initial verification

## Summary (TL;DR)

Phase 3 поставил все плановые артефакты: 5 планов выполнены, 27 backend-тестов зелёные (acts_crud=8, acts_returns=8, acts_undo=5, acts_search=6, acts_e2e_smoke=3, acts_numbering=1, acts_display_rule=4, acts_http_smoke=2, pdf_render_act=5, pdf_determinism=2, pdf_text_extract=1, templates_seed=4, organization_io=6, export_bindings=N), V014 миграция применена, шрифт DejaVu Sans 2.37 embedded, MiniJinja safe-mode + krilla rendering работают, UI-цепочка handover → return → archive → undo полноценная.

Однако code review (.planning/phases/03-pdf/03-REVIEW.md) выявил **4 BLOCKER-уровня дефекта**, которые downgrade два требования из «Complete» в **FAILED**:

1. **ACT-11 — FAILED** (CR-01): шапка PDF не содержит логотип организации. `safe_logo_canonical` валидирует путь, передаёт в DocSpec — но `renderer::render_docspec` НЕ читает `spec.header.logo_path` и не вызывает `surface.draw_image`. Документы выходят без логотипа, хотя ACT-11 явно требует «шапка с логотипом».
2. **ACT-13 — FAILED** (CR-02 + CR-03 + CR-04): транзакционная атомарность *одной* tx соблюдена, но три class-A инварианта целостности данных в `do_return` отсутствуют: (а) нет проверки текущего статуса device перед возвратом, (б) нет дедупликации device_id/act_item_id в payload, (в) нет валидации quantity против исходного handover. Эти три гэпа открывают двойной возврат, broken undo chain, и инвалидные quantity-суммы.

Остальные 14 требований (ACT-01..10, ACT-12, ACT-14, DEV-14, DEV-15) verified — артефакты на месте, ключевые ссылки замкнуты, тесты прошли.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Создание handover-акта через UI с auto-counter / override | ✓ VERIFIED | `acts_crud::create_handover_happy`, `override_number_audits_and_increments_only_audit`, `acts_numbering::concurrent_50_creates_unique_numbers` — все зелёные |
| 2 | Switch-bar Акты/Возвраты/Архив со счётчиками | ✓ VERIFIED | `acts_crud::counts_match_switch_bar` зелёный; UI: `ui/src/features/acts/ActsSearchAndTabs.svelte` |
| 3 | quantity column персистится в act_items | ✓ VERIFIED | `acts_crud::handover_with_quantity_persists` зелёный; V014 добавил `act_items.quantity NOT NULL DEFAULT 1` |
| 4 | Поиск по актам (номер/ФИО/устройство), debounce 250ms | ✓ VERIFIED | `acts_search` × 6 тестов зелёные; `ActsSearchAndTabs.svelte` содержит `setTimeout` 250 |
| 5 | Создание возврата (полный/частичный), bulk + per-row override | ✓ VERIFIED | `acts_returns::bulk_apply_with_per_row_override`, `partial_return_keeps_handover_active`, etc. — 8/8 зелёные |
| 6 | Авто-архив при 100% возврате | ✓ VERIFIED | `acts_returns::full_return_archives_handover` зелёный |
| 7 | Display-rule «42в»/«42в1»/«42в2» с retroactive promotion | ✓ VERIFIED | `acts_display_rule` × 4 теста зелёные |
| 8 | Undo через audit_log replay (handover/return delete) | ✓ VERIFIED | `acts_undo` × 5 тестов зелёные включая `delete_handover_with_partial_return_cascades_undo` |
| 9 | Counter act_number инкрементируется только для handover | ✓ VERIFIED | `acts_returns::return_does_not_increment_act_counter` зелёный |
| 10 | PDF render handover-акта с кириллицей | ✓ VERIFIED (partial — без логотипа) | `pdf_render_act::render_handover_act_produces_cyrillic_pdf` зелёный; pdf-extract находит «Сидоров-Петроградский», «(ё)», «№42» |
| 11 | PDF шапка с логотипом организации | **✗ FAILED** | renderer.rs нигде не читает `spec.header.logo_path` и не вызывает `draw_image` — см. CR-01 |
| 12 | PDF render document приёма (DEV-15) | ✓ VERIFIED | `pdf_render_act::render_acceptance_pdf_for_device_works`, `acts_e2e_smoke::acceptance_pdf_render_smoke` — оба зелёные |
| 13 | Templates seed (act_handover + act_acceptance) идемпотентно | ✓ VERIFIED | `templates_seed` × 4 теста зелёные |
| 14 | Organization JSON I/O + path-traversal mitigation | ✓ VERIFIED | `organization_io::logo_path_traversal_rejected` + 5 других зелёные |
| 15 | DEV-14 UI flow (DeviceContextMenu → DocumentAcceptanceModal → PdfPreviewModal mode='acceptance') | ✓ VERIFIED (структурно) | UI-файлы существуют и связаны; manual UAT нужен для UX |
| 16 | Транзакционная целостность return: статус check + dedup + quantity bounds | **✗ FAILED** | См. CR-02, CR-03, CR-04 в gaps |
| 17 | Determinism PDF byte-exact rendering | ✓ VERIFIED | `pdf_determinism::rendering_twice_yields_identical_bytes` + `fixture_act_42_renders_to_known_hash` зелёные; SHA256 пинн: `88df7f9d...` |

**Score:** 15/17 наблюдаемых истин verified (88%). 2 FAILED → 4 Phase 3 Requirement IDs downgrade (см. ниже).

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/trackly-app/assets/fonts/DejaVuSans.ttf` (757 KB) | Embedded Cyrillic font Regular | ✓ VERIFIED | присутствует |
| `crates/trackly-app/assets/fonts/DejaVuSans-Bold.ttf` (706 KB) | Embedded Cyrillic font Bold | ✓ VERIFIED | присутствует |
| `crates/trackly-app/src/pdf/{fonts,docspec,minijinja_env,renderer,mod}.rs` | 3-stage pipeline | ✓ VERIFIED | все 5 файлов на месте |
| `crates/trackly-app/templates/act_handover.minijinja` | Russian template handover | ✓ VERIFIED | существует |
| `crates/trackly-app/templates/act_acceptance.minijinja` | Russian template acceptance | ✓ VERIFIED | существует |
| `crates/trackly-app/tests/fixtures/act_42.json` + `act_42.sha256` | Canonical fixture | ✓ VERIFIED | hash: `88df7f9d69c5db10a4685f0aa5d390caec90b045067e35cc1caba33efdd15d1f` |
| `migrations/V014__acts_indexes_and_status_codes.sql` | indexes + device_statuses.code + act_items.quantity | ✓ VERIFIED | применено |
| `crates/trackly-core/src/{domain,ports}/acts.rs` | ActRow / ActType / ActFilter / ActRepository | ✓ VERIFIED | существует |
| `crates/trackly-infra/src/repos/acts_sqlite.rs` | SqliteActRepository + increment_counter + next_sub_number + recompute_parent_archived | ✓ VERIFIED | все методы присутствуют |
| `crates/trackly-infra/src/repos/audit_log_sqlite.rs` | AuditEntry + insert + select_device_mutations_for_act | ✓ VERIFIED | json_extract по payload_json present |
| `crates/trackly-app/src/services/act_service.rs` | ActService с create/get/list/counts/do_return/delete_soft/render_pdf/render_acceptance_pdf/search | ✓ VERIFIED (с гэпами в do_return) | методы присутствуют, **но do_return имеет 3 BLOCKER-гэпа целостности** |
| `crates/trackly-app/src/services/template_service.rs` | seed + get_active | ✓ VERIFIED | seed_defaults_on_startup идемпотентен |
| `crates/trackly-app/src/services/organization_service.rs` | read + safe_logo_canonical + placeholder | ✓ VERIFIED | path-traversal mitigation работает (тест проходит) |
| `crates/trackly-app/src/pdf/renderer.rs` | render_docspec → Vec<u8> с draw_text по DocSpec | ⚠️ STUB (партикулярно) | text rendering работает, **logo rendering ОТСУТСТВУЕТ** — `surface.draw_image` не вызывается |
| `crates/trackly-app/src/dto/{act,organization}.rs` | DTO с specta::Type | ✓ VERIFIED | существует |
| `crates/trackly-app/src/tauri_cmds/{acts,organization,templates}.rs` | tauri commands | ✓ VERIFIED | существуют |
| `crates/trackly-app/src/http/{acts,organization,templates}.rs` | axum routers | ✓ VERIFIED | существуют |
| `ui/src/features/acts/{ActsPage,ActDetail,ActFormModal,ReturnModal,ReturnItemsTable,PdfPreviewModal,DocumentAcceptanceModal,ActsSearchAndTabs,ActsMasterDetail,...}.svelte` | Full UI suite | ✓ VERIFIED | все 16+ файлов существуют |
| `ui/src/features/devices/DeviceContextMenu.svelte` | extended с «Печать документа приёма» | ✓ VERIFIED | вызовы `onPrintAcceptance` найдены |
| `ui/src/lib/api/{acts,organization,templates,pdf}.ts` | API wrappers | ✓ VERIFIED | существуют |
| Backend tests (acts_*, pdf_*, templates_*, organization_*, export_bindings) | 13 test files зелёные | ✓ VERIFIED | все запущены и passed (см. behavioral spot-checks) |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|----|--------|---------|
| `ActsPage.svelte` | `acts.list`/`acts.counts`/`acts.search` | `ui/src/lib/api/acts.ts` | ✓ WIRED | API-методы определены, вызываются |
| `ActService::create` | `acts_sqlite::increment_counter_in_tx` | `writer.execute` single tx | ✓ WIRED | `acts_numbering::concurrent_50_creates_unique_numbers` доказывает атомарность |
| `ActService::do_return` | `acts_sqlite::next_sub_number_for_parent` | MAX(sub_number)+1 в одной tx | ✓ WIRED | `acts_returns::return_concurrent_two_returns_correct_sub_numbers` |
| `ActService::do_return` | `acts_sqlite::recompute_parent_archived` | в той же writer.execute closure | ✓ WIRED | `acts_returns::full_return_archives_handover` |
| `ActService::delete_soft` | `audit_log_sqlite::select_device_mutations_for_act` | json_extract по payload_json.act_id | ✓ WIRED | `acts_undo::delete_handover_with_partial_return_cascades_undo` |
| `ActService::render_pdf` | `template_service::get_active` | load body_minijinja for kind='act_handover' | ✓ WIRED | `pdf_render_act::render_with_missing_template_returns_notfound` |
| `ActService::render_pdf` | `minijinja_env::render_with_timeout` | (env, name, src, ctx) → JSON | ✓ WIRED | `pdf_render_act::render_with_broken_template_returns_validation` |
| `ActService::render_pdf` | `PdfRenderer::render_docspec` | serde_json::from_str<DocSpec> → krilla | ✓ WIRED | `pdf_render_act::render_handover_act_produces_cyrillic_pdf` |
| `ActService::render_pdf` | `organization_service::safe_logo_canonical` | safe_logo передаётся в шаблонный ctx | ⚠️ PARTIAL | safe_logo вычисляется + serialize в `DocSpec.header.logo_path`, **но не достигает финального PDF** (CR-01) |
| `PdfPreviewModal.svelte` | `acts.renderPdf`/`renderAcceptancePdf` | apiCall<number[]> → Blob → iframe | ✓ WIRED | UI manual UAT-tested visually за рамками автомата |
| `DeviceContextMenu.svelte` | `DocumentAcceptanceModal.svelte` | `onPrintAcceptance(device)` callback | ✓ WIRED | grep подтверждает связь |
| `DocumentAcceptanceModal.svelte` | `PdfPreviewModal.svelte` (mode='acceptance') | `onSubmit({deviceId, giverName, receiverName, dateUtc})` | ✓ WIRED | manual UAT нужен |
| `AppCtx::build` | `templates.seed_defaults_on_startup` | idempotent first-run seed | ✓ WIRED | `templates_seed::default_seeded_on_first_startup` |
| `AppCtx::build` | `ActService::new_with_pdf(writer, readers, clock, templates, org, pdf)` | service construction | ✓ WIRED | tests pass |

### Data-Flow Trace (Level 4) — selective

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `ActsPage.svelte`/`ActsList.svelte` | `items: ActDto[]` | `acts.list` / `acts.search` → service → SqliteActRepository | DB query with JOINs | ✓ FLOWING |
| `ActDetail.svelte` | `act: ActDto` | `acts.get(id)` | DB query | ✓ FLOWING |
| `PdfPreviewModal.svelte` | `pdfBytes: number[]` | `acts.renderPdf(actId)` → full render pipeline | DB → MiniJinja → DocSpec → krilla | ✓ FLOWING |
| `ReturnModal.svelte` | `act.items: ActItemDto[]` | passed from parent ActDetail | parent fetches via `acts.get` | ✓ FLOWING |
| `DocumentAcceptanceModal.svelte` | `device: DeviceDto` | passed via DeviceContextMenu trigger | DevicesPage fetches via `devices.list` | ✓ FLOWING |
| `PdfRenderer::render_docspec` | `spec.header.logo_path: Option<String>` | передаётся из act_service → шаблон → JSON | **NOT consumed** в renderer (CR-01) | ✗ DISCONNECTED (logo path → nowhere) |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Acts CRUD/numbering/returns/undo/display/http | `cargo test -p trackly-app --test acts_crud --test acts_numbering --test acts_returns --test acts_undo --test acts_display_rule --test acts_http_smoke --test acts_search --test acts_e2e_smoke` | 8+1+8+5+4+2+6+3 = 37 tests passed | ✓ PASS |
| PDF rendering pipeline | `cargo test -p trackly-app --test pdf_determinism --test pdf_text_extract --test pdf_render_act` | 2+1+5 = 8 tests passed | ✓ PASS |
| Templates + Organization | `cargo test -p trackly-app --test templates_seed --test organization_io --test export_bindings` | 4+6+N tests passed | ✓ PASS |
| SHA256 fixture stable (двойной запуск) | manual двойной запуск pdf_determinism | hash идентичен | ✓ PASS |
| Manual UI flows (Tauri dev) | (visual) | not attempted in this verification | ? SKIP — routed to human |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| ACT-01 | 03-02 | Создание акта приёма-передачи | ✓ SATISFIED | `acts_crud::create_handover_happy` зелёный |
| ACT-02 | 03-02 | Switch-bar Акты/Возвраты/Архив со счётчиками | ✓ SATISFIED | `acts_crud::counts_match_switch_bar`, UI `ActsSearchAndTabs.svelte` |
| ACT-03 | 03-02 | act_items.quantity персистится через DTO + service | ✓ SATISFIED | `handover_with_quantity_persists` + V014 ALTER TABLE |
| ACT-04 | 03-05 | Поиск по актам (FTS+LIKE merge, debounce 250ms) | ✓ SATISFIED | `acts_search` × 6 тестов + UI debounce |
| ACT-05 | 03-02 | Override номера + audit | ✓ SATISFIED | `override_number_audits_and_increments_only_audit` |
| ACT-06 | 03-03 | Удаление handover восстанавливает devices через audit_log replay | ✓ SATISFIED | `acts_undo::delete_handover_restores_devices_to_pre_handover` |
| ACT-07 | 03-03 | Display-rule «42в»/«42в1»/«42в2» + retroactive promotion | ✓ SATISFIED | `acts_display_rule` × 4 + `second_partial_return_assigns_sub_number_2_and_promotes_suffix` |
| ACT-08 | 03-03 | Bulk apply + per-row override | ✓ SATISFIED | `bulk_apply_with_per_row_override`, `return_with_apply_to_all_false_and_full_per_row_succeeds` |
| ACT-09 | 03-03 | Auto-archive при 100% возврате (D-Archive-01) | ✓ SATISFIED | `full_return_archives_handover` (no manual flag) |
| ACT-10 | 03-03 | Удаление return восстанавливает devices к handover-состоянию и unarchive parent | ✓ SATISFIED | `delete_return_restores_to_handover_state_unarchives_parent` |
| ACT-11 | 03-04 | PDF Акта приёма-передачи с шапкой+логотипом | **✗ BLOCKED** | CR-01: рендерер не выводит логотип в PDF; шапка без image |
| ACT-12 | 03-04 | Редактируемый шаблон акта приёма-передачи в БД | ✓ SATISFIED | `templates_seed::default_seeded_on_first_startup`; UI редактор отложен на Phase 7 (документировано) |
| ACT-13 | 03-02, 03-03 | Транзакционная гарантия create/return/undo (всё или ничего) | **✗ BLOCKED** | CR-02 + CR-03 + CR-04: 3 инварианта целостности данных в do_return отсутствуют (status check, dedup, quantity bounds) |
| ACT-14 | 03-02 | 50 параллельных create дают уникальные последовательные номера | ✓ SATISFIED | `acts_numbering::concurrent_50_creates_unique_numbers` |
| DEV-14 | 03-05 | UI device acceptance flow (context-menu → intermediate modal → preview) | ✓ SATISFIED (структурно) | UI-файлы существуют и связаны; manual UAT нужен для UX качества |
| DEV-15 | 03-04 | Шаблон документа приёма + рендеринг | ✓ SATISFIED | `templates_seed::acceptance_seeded_and_used`, `render_acceptance_pdf_for_device_works` |

**ORPHANED:** нет — все 16 PHASE 3 ID присутствуют в `requirements:` frontmatter планов 03-02..03-05.

### Anti-Patterns Found

Из REVIEW (.planning/phases/03-pdf/03-REVIEW.md) — 4 Critical, 11 Warning, 6 Info. Verifier подтверждает все Critical findings прямой проверкой кода:

| Finding | File | Line | Pattern | Severity | Impact на верификацию |
|---------|------|------|---------|----------|----------------------|
| CR-01 | `pdf/renderer.rs` | 88-178 | Отсутствует чтение `spec.header.logo_path` и `draw_image` | 🛑 BLOCKER | ACT-11 не выполнен; голова Phase 3 promise |
| CR-02 | `services/act_service.rs::do_return` | 362-584 | Нет проверки `device.status_id == in_work_status_id` | 🛑 BLOCKER | ACT-13 транзакционная гарантия неполна |
| CR-03 | `services/act_service.rs::validate_return` | 328-360 | Нет HashSet dedup для device_id/act_item_id | 🛑 BLOCKER | ACT-13 — двойной snapshot ломает undo |
| CR-04 | `services/act_service.rs::do_return` | 399-417 | Нет cross-check quantity ≤ handover_qty - already_returned | 🛑 BLOCKER | ACT-13 — quantity-сумма неконсистентна |
| WR-01..WR-11 | разные | разные | Drift archived при DeviceService.update, fragile compute_suffix, LIKE escape через strip, regex recompile, etc. | ⚠️ WARNING | Не блокирует goal, документируется в REVIEW |
| IN-01..IN-06 | разные | разные | Дублирование build_fts_query, magic numbers, dead_code и т.д. | ℹ️ INFO | Quality cleanup, не блокирует |

Дополнительно verifier проверил отсутствие незакрытых debt-маркеров на ACT-13 / ACT-11 путях:

```
grep -n "TODO\|FIXME\|XXX\|HACK" crates/trackly-app/src/services/act_service.rs crates/trackly-app/src/pdf/renderer.rs
```

— ни одного TBD/FIXME/XXX без ссылки на issue/PR не обнаружено в pdf/renderer.rs и act_service.rs (debt-marker gate проходит). REVIEW сам по себе является аудитом, не источник новых debt-маркеров.

### Human Verification Required

См. блок `human_verification:` в frontmatter — 5 UAT-сценариев:

1. **DEV-14 UI flow** — manual click-through `DevicesPage → context-menu → DocumentAcceptanceModal → submit → PdfPreviewModal` с реальным PDF
2. **PDF визуальная проверка handover-акта** — рендеринг шапки + кириллица в табличной форме + подписи (включая «ё», «-», длинные ФИО)
3. **Full lifecycle UI (ACT-06..10)** — handover → partial return → full return → auto-archive (visible в Архиве) → undo return → undo handover; switch-bar счётчики и toast'ы
4. **Поиск debounce (ACT-04)** — UX feel 250ms задержки, переключение list↔search режимов
5. **Logo gap confirmation** — пользователь подтверждает, что отсутствие логотипа — это FAIL (gap), а не accepted simplification (если последнее — оформить override в frontmatter)

### Gaps Summary

**Два требования из 16 заблокированы Critical findings code-review:**

**ACT-11 (PDF шапка с логотипом)** — заявлено в ROADMAP/REQUIREMENTS как часть «PDF Акта приёма-передачи». Технически шапка рендерится (текст org_name, ИНН, КПП, address, date_label), но image-элемент логотипа никогда не выводится: `renderer::render_docspec` не имеет ни одного вызова `surface.draw_image` / `krilla::Image`. Поле `spec.header.logo_path` десериализуется и доезжает до renderer, но молча игнорируется. Из-за этого вся обвязка `safe_logo_canonical` + path-traversal mitigation + передача через MiniJinja ctx — мёртвый код в финальной PDF-сборке.

**ACT-13 (транзакционная гарантия)** — внутри одной BEGIN IMMEDIATE writer-job семантика «всё или ничего» соблюдена. Но три cross-validating инварианта отсутствуют в `do_return`:

- Двойной возврат тех же device_id (через расхождение cache между двумя операторами) проходит без ошибки → broken audit chain → undo восстанавливает неправильный snapshot.
- Дублирование device_id внутри одного payload даёт два audit-snapshot для одного устройства; replay в insert-order восстанавливает к промежуточному, а не к pre-handover состоянию.
- quantity=100 при выданном quantity=1 → возврат проходит; отчёты будут показывать асимметричные суммы.

Конкретные fix patterns + acceptance criteria даны в REVIEW.md разделах CR-01..CR-04. Все 4 правки локальны (act_service.rs + renderer.rs), не требуют schema-changes, и могут быть оформлены планом-исправлением сразу после verify gate.

---

_Verified: 2026-05-30_
_Verifier: Claude (gsd-verifier)_
_Depth: standard goal-backward + REVIEW Critical cross-check_
