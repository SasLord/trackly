---
phase: 03-pdf
verified: 2026-05-30T20:30:00Z
status: human_needed
score: 16/16 requirements satisfied
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 12/16
  gaps_closed:
    - "ACT-11: PDF Акта приёма-передачи рендерит шапку с логотипом организации"
    - "ACT-13: Транзакционная гарантия Возврата защищает от двойного/невалидного возврата (status check, dedup, quantity bounds)"
  gaps_remaining: []
  regressions: []
  gap_closure_plan: ".planning/phases/03-pdf/03-06-PLAN.md"
  gap_closure_summary: ".planning/phases/03-pdf/03-06-SUMMARY.md"
  gap_closure_commits:
    - "b1298eb — feat(03-06): PDF logo rendering via krilla Image XObject (ACT-11 / CR-01)"
    - "090bc06 — feat(03-06): return-tx integrity invariants — status guard + dedup + qty bound (ACT-13)"
gaps: []
human_verification:
  - test: "Manual UAT — DEV-14 flow на запущенном Tauri-приложении"
    expected: "Правый клик на устройство в DevicesPage → «Печать документа приёма» → DocumentAcceptanceModal с полями «Кто передал/Кто принял/Дата» → submit → PdfPreviewModal с реальным PDF (кириллица читаема, ФИО видны, дата правильная)"
    why_human: "Полный browser→tauri-plugin-dialog→tauri-plugin-shell→tauri-plugin-fs flow + визуальный рендеринг PDF в iframe нельзя верифицировать grep'ом"
  - test: "Manual UAT — PDF рендеринг handover-акта с шапкой и кириллицей"
    expected: "Открыть существующий handover-акт → «Печать» → видна шапка с реквизитами организации (название, ИНН, КПП, адрес), таблица позиций с кириллическими наименованиями, подписи Сдал/Принял с правильными ФИО включая «ё»/«-» в составе"
    why_human: "Визуальное качество рендеринга шрифта DejaVu Sans (kerning, line-break, hyphenation) можно оценить только глазом — pdf-extract это пропускает"
  - test: "Manual UAT — визуальная проверка нового логотипа в шапке PDF (ACT-11 closure)"
    expected: "Настроить org с реальным logo.png (или JPG) в exe_dir → render handover-PDF → в правом-верхнем углу шапки виден логотип ≤80×40pt с сохранённым aspect-ratio. Тест с битым/несуществующим logo_path — PDF рендерится без логотипа без ошибки."
    why_human: "Pixel-perfect позиция, размер, прозрачность, читаемость логотипа на фоне шапки — визуальная оценка"
  - test: "Manual UAT — full lifecycle (ACT-06..10) через UI"
    expected: "Создать handover → partial return через ReturnModal (галочка «Применить ко всем» по умолчанию ВКЛ) → full return → handover автоматически в архив (вкладка «Архив», счётчик +1) → удалить return → handover возвращается в активные → удалить handover → все devices снова «На складе»"
    why_human: "UX-поток множества модалов, switch-bar счётчики, toast'ы, real-time master-detail обновление — это integration UI behavior"
  - test: "Manual UAT — поиск по актам (ACT-04) через UI"
    expected: "В ActsSearchAndTabs ввести часть номера акта / ФИО / наименования устройства → через ~250ms список фильтруется; backend acts.search вызывается с debounce"
    why_human: "Debounce timing 250ms и UX переключения между acts.list и acts.search режимами"
  - test: "Manual UAT — ACT-13 cross-tx race на запущенной системе"
    expected: "Открыть два окна с одним handover-актом → в одном оформить полный возврат (success) → во втором (с устаревшим cache) попытаться оформить возврат тех же позиций → backend отвергает с понятным сообщением (HTTP 409 Conflict / AppError::Conflict «уже не в работе»); UI показывает toast/alert, не «тихое сохранение»."
    why_human: "Cross-tab race condition с реальной задержкой между cache-read и tx-write нельзя стабильно воспроизвести в integration test (тест моделирует двойной вызов внутри одного процесса)"
---

# Phase 3: PDF + Acts Verification Report (Re-verified)

**Phase Goal:** Закрыть все 16 требований Phase 3 — ACT-01..14 (handover/return acts с auto-numbering, FTS+LIKE search, PDF render, undo, auto-archive) + DEV-14 (device acceptance intermediate-modal flow) + DEV-15 (acceptance act PDF rendering).

**Verified:** 2026-05-30 (initial) → 2026-05-30 (re-verification, 2nd pass after gap-closure plan 03-06)
**Status:** human_needed (все автоматические проверки PASSED; остаются UAT-сценарии для UX/визуального qa)
**Re-verification:** Yes — initial 2026-05-30 found 2 BLOCKER gaps (ACT-11, ACT-13); plan 03-06 закрыл оба гэпа через commits b1298eb + 090bc06; этот отчёт — 2nd pass

## Summary (TL;DR — что изменилось vs. 1st pass)

**1st pass (2026-05-30, initial):** 12/16 requirements satisfied. ACT-11 (PDF логотип) и ACT-13 (return-tx integrity) — **FAILED**, 4 BLOCKER-уровня дефекта (CR-01..CR-04) из 03-REVIEW.md.

**Plan 03-06 (gap-closure):** 2 commit'а, 4 файла источника + 2 теста + 1 fixture:
- `b1298eb`: `pdf/renderer.rs::draw_logo_top_right` — `std::fs::read` → `krilla::image::Image::from_png/from_jpeg` → `surface.draw_image` (с push_transform позиционированием) + 3 теста (`pdf_logo.rs`).
- `090bc06`: `act_service.rs::validate_return` (HashSet dedup) + `do_return` (status guard + quantity-bound SQL) + 4 теста (`acts_returns.rs`).

**2nd pass result:** Все 4 CR закрыты по коду и тестам; 14 ранее-VERIFIED требований не имеют регрессий; SHA256 фикстуры `act_42` неизменён (`88df7f9d…`). Score **16/16**. Все Phase 3 автоматические тесты зелёные. Остаётся `human_needed` для 6 UAT-сценариев (UX/визуальная оценка), которые не зависят от закрытых гэпов.

## Re-verification Detail — CR-01..CR-04 Closure

| Critical | Source (1st pass) | Closure Evidence (2nd pass) | Status |
|----------|-------------------|------------------------------|--------|
| **CR-01** — PDF логотип не рендерится | `pdf/renderer.rs::render_docspec` не имел `surface.draw_image`; `spec.header.logo_path` игнорировался | Commit `b1298eb`. `renderer.rs:172-173` `if let Some(logo_path_str)…draw_logo_top_right(…)`. Helper `renderer.rs:325` `fn draw_logo_top_right` — реализует `std::fs::read` → `Image::from_png/from_jpeg` (`renderer.rs:343,345`) → `surface.draw_image(image, size)` (`renderer.rs:381`). Test: `pdf_logo::act_with_logo_renders_image_in_pdf` PASSED (3/3 в pdf_logo). | ✓ CLOSED |
| **CR-02** — status guard в do_return | Нет проверки `before.status_id == in_work_status_id` перед `update_full_in_tx` | Commit `090bc06`. `act_service.rs:455` резолвит `in_work_status_id` один раз перед per-item loop; `act_service.rs:516-521` проверяет `if before.status_id != in_work_status_id` → `AppError::Conflict { reason: "Устройство id={} уже не в работе…" }`. Test: `acts_returns::return_twice_same_device_rejected` PASSED. | ✓ CLOSED |
| **CR-03** — HashSet dedup в validate_return | Нет дедупликации device_id/act_item_id в payload | Commit `090bc06`. `act_service.rs:339-340` два независимых `HashSet::<i64>::new()`; `.insert()` возвращает false → `AppError::Validation { field: "items[N].act_item_id"/"items[N].device_id", message: "…продублирован в возврате" }`. Tests: `acts_returns::return_with_duplicate_act_item_id_rejected` + `acts_returns::return_with_duplicate_device_id_rejected` — оба PASSED. | ✓ CLOSED |
| **CR-04** — quantity bound | Нет cross-check `payload.quantity ≤ handover_qty - already_returned` | Commit `090bc06`. `act_service.rs:541` SQL `SELECT COALESCE(SUM(rai.quantity), 0) FROM act_items rai JOIN acts ra ON ra.id = rai.act_id WHERE ra.parent_act_id=?1 AND rai.device_id=?2 AND ra.deleted_at_utc IS NULL`; `act_service.rs:555` сообщение `"Возврат превышает выданное количество для устройства id={}…"`. Test: `acts_returns::return_quantity_exceeds_handover_rejected` PASSED. | ✓ CLOSED |

### Grep Gates (2nd pass)

```
$ grep -c "surface.draw_image" crates/trackly-app/src/pdf/renderer.rs        → 2 matches
$ grep -c "std::fs::read"      crates/trackly-app/src/pdf/renderer.rs        → 1 match (the read call; doc-comment refs filtered)
$ grep -E "Image::from_(png|jpeg)" crates/trackly-app/src/pdf/renderer.rs    → 2 matches (PNG + JPEG)
$ grep -c "HashSet"            crates/trackly-app/src/services/act_service.rs → 3 matches (2 inserts + 1 doc-comment)
$ grep -c "уже не в работе"   crates/trackly-app/src/services/act_service.rs → 1 match
$ grep -c "SUM(rai.quantity)"  crates/trackly-app/src/services/act_service.rs → 1 match
$ grep -c "превышает выданное" crates/trackly-app/src/services/act_service.rs → 1 match
```

Все grep-гейты из 03-06-PLAN.md acceptance_criteria выполнены.

### Test Suite Results (2nd pass)

| Suite | Count | Status | Notes |
|-------|-------|--------|-------|
| `pdf_logo` | **3/3** | ✓ NEW PASS | `act_with_logo_renders_image_in_pdf`, `logo_path_none_renders_without_panic`, `logo_path_missing_file_is_graceful` (имена немного отличаются от черновика плана; функционально соответствуют 3 must-have truth'ам ACT-11) |
| `pdf_determinism` | 2/2 | ✓ PASS | SHA256 фикстуры act_42 неизменён: `88df7f9d69c5db10a4685f0aa5d390caec90b045067e35cc1caba33efdd15d1f` (logo_path=null → no draw_image → no byte diff — план предсказал это) |
| `pdf_render_act` | 5/5 | ✓ PASS | новый `render_pdf_with_missing_logo_renders_without_logo` подтверждает graceful path через полный ActService pipeline |
| `pdf_text_extract` | 1/1 | ✓ PASS | без regression |
| `acts_returns` | **12/12** | ✓ PASS | 8 существующих + 4 новых (return_twice_same_device_rejected, return_with_duplicate_act_item_id_rejected, return_with_duplicate_device_id_rejected, return_quantity_exceeds_handover_rejected) |
| `acts_crud` | 8/8 | ✓ PASS | без regression |
| `acts_undo` | 5/5 | ✓ PASS | без regression (delete_handover_with_partial_return_cascades_undo всё ещё зелёный, что важно после CR-02/03) |
| `acts_search` | 6/6 | ✓ PASS | без regression |
| `acts_display_rule` | 4/4 | ✓ PASS | без regression |
| `acts_e2e_smoke` | 3/3 | ✓ PASS | без regression |
| `acts_http_smoke` | 2/2 | ✓ PASS | без regression |
| `acts_numbering` | 1/1 | ✓ PASS | concurrent_50_creates_unique_numbers зелёный — single-writer discipline соблюдена |
| `templates_seed` | 4/4 | ✓ PASS | без regression |
| `organization_io` | 6/6 | ✓ PASS | safe_logo_canonical + path-traversal mitigation работают |
| **TOTAL** | **62/62** | ✓ ALL GREEN | 4 теста добавлены (ACT-13), 3 теста добавлены (ACT-11), 0 регрессий |
| `cargo clippy --workspace --all-targets -- -D warnings` | — | ✓ CLEAN | без новых warnings |

## Goal Achievement (Updated)

### Observable Truths

| # | Truth | 1st pass | 2nd pass | Evidence |
|---|-------|----------|----------|----------|
| 1 | Создание handover-акта через UI с auto-counter / override | ✓ VERIFIED | ✓ VERIFIED | `acts_crud::create_handover_happy`, `acts_numbering::concurrent_50_creates_unique_numbers` |
| 2 | Switch-bar Акты/Возвраты/Архив со счётчиками | ✓ VERIFIED | ✓ VERIFIED | `acts_crud::counts_match_switch_bar` |
| 3 | quantity column персистится в act_items | ✓ VERIFIED | ✓ VERIFIED | `acts_crud::handover_with_quantity_persists` |
| 4 | Поиск по актам (debounce 250ms) | ✓ VERIFIED | ✓ VERIFIED | `acts_search` × 6 |
| 5 | Создание возврата (полный/частичный), bulk + per-row override | ✓ VERIFIED | ✓ VERIFIED | `acts_returns` 8 → 12, добавлены invariant tests |
| 6 | Авто-архив при 100% возврате | ✓ VERIFIED | ✓ VERIFIED | `acts_returns::full_return_archives_handover` |
| 7 | Display-rule «42в»/«42в1»/«42в2» | ✓ VERIFIED | ✓ VERIFIED | `acts_display_rule` × 4 |
| 8 | Undo через audit_log replay | ✓ VERIFIED | ✓ VERIFIED | `acts_undo` × 5 |
| 9 | act_number инкрементируется только для handover | ✓ VERIFIED | ✓ VERIFIED | `return_does_not_increment_act_counter` |
| 10 | PDF render handover-акта с кириллицей | ✓ VERIFIED | ✓ VERIFIED | `render_handover_act_produces_cyrillic_pdf` |
| 11 | PDF шапка с логотипом организации (ACT-11) | **✗ FAILED** | **✓ VERIFIED** | `pdf_logo::act_with_logo_renders_image_in_pdf` PASSED — PDF-байты содержат маркер /Image; commit b1298eb |
| 12 | PDF render document приёма (DEV-15) | ✓ VERIFIED | ✓ VERIFIED | `render_acceptance_pdf_for_device_works`, `acts_e2e_smoke::acceptance_pdf_render_smoke` |
| 13 | Templates seed (handover + acceptance) идемпотентно | ✓ VERIFIED | ✓ VERIFIED | `templates_seed` × 4 |
| 14 | Organization JSON I/O + path-traversal mitigation | ✓ VERIFIED | ✓ VERIFIED | `organization_io::logo_path_traversal_rejected` |
| 15 | DEV-14 UI flow (структурно) | ✓ VERIFIED | ✓ VERIFIED | UI-файлы и связи; manual UAT для UX (см. human_verification) |
| 16 | Транзакционная целостность return: status check + dedup + quantity bounds (ACT-13) | **✗ FAILED** | **✓ VERIFIED** | `acts_returns` + 4 invariant tests PASSED; commit 090bc06 |
| 17 | Determinism PDF byte-exact rendering | ✓ VERIFIED | ✓ VERIFIED | SHA256 неизменён: `88df7f9d69c5db10a4685f0aa5d390caec90b045067e35cc1caba33efdd15d1f` |

**Score:** 17/17 наблюдаемых истин verified (100%) — было 15/17 (88%). 2 FAILED → 2 VERIFIED.

### Requirements Coverage (Updated)

| Requirement | Source Plan | Description | 1st pass | 2nd pass | Evidence |
|-------------|-------------|-------------|----------|----------|----------|
| ACT-01 | 03-02 | Создание акта приёма-передачи | ✓ | ✓ | `create_handover_happy` |
| ACT-02 | 03-02 | Switch-bar Акты/Возвраты/Архив со счётчиками | ✓ | ✓ | `counts_match_switch_bar` |
| ACT-03 | 03-02 | act_items.quantity персистится | ✓ | ✓ | `handover_with_quantity_persists` + V014 |
| ACT-04 | 03-05 | Поиск по актам (FTS+LIKE merge, 250ms) | ✓ | ✓ | `acts_search` × 6 + UI debounce |
| ACT-05 | 03-02 | Override номера + audit | ✓ | ✓ | `override_number_audits_and_increments_only_audit` |
| ACT-06 | 03-03 | Удаление handover восстанавливает devices | ✓ | ✓ | `delete_handover_restores_devices_to_pre_handover` |
| ACT-07 | 03-03 | Display-rule с retroactive promotion | ✓ | ✓ | `acts_display_rule` × 4 |
| ACT-08 | 03-03 | Bulk apply + per-row override | ✓ | ✓ | `bulk_apply_with_per_row_override` |
| ACT-09 | 03-03 | Auto-archive при 100% возврате | ✓ | ✓ | `full_return_archives_handover` |
| ACT-10 | 03-03 | Удаление return восстанавливает devices и unarchive | ✓ | ✓ | `delete_return_restores_to_handover_state_unarchives_parent` |
| **ACT-11** | 03-04 + 03-06 | PDF Акта приёма-передачи с шапкой+логотипом | **✗ BLOCKED** | **✓ SATISFIED** | commit b1298eb; `pdf_logo` × 3; `render_pdf_with_missing_logo_renders_without_logo` |
| ACT-12 | 03-04 | Редактируемый шаблон в БД | ✓ | ✓ | `templates_seed::default_seeded_on_first_startup` |
| **ACT-13** | 03-02, 03-03, 03-06 | Транзакционная гарантия create/return/undo (status/dedup/qty) | **✗ BLOCKED** | **✓ SATISFIED** | commit 090bc06; 4 invariant tests + 8 existing acts_returns tests |
| ACT-14 | 03-02 | 50 параллельных create — уникальные номера | ✓ | ✓ | `concurrent_50_creates_unique_numbers` |
| DEV-14 | 03-05 | UI device acceptance flow | ✓ (структурно) | ✓ (структурно) | UI-файлы; manual UAT для UX |
| DEV-15 | 03-04 | Шаблон документа приёма + рендеринг | ✓ | ✓ | `acceptance_seeded_and_used`, `render_acceptance_pdf_for_device_works` |

**Score:** 16/16 SATISFIED — было 12/16 BLOCKED (4 заблокированы 2 FAILED truths; теперь оба VERIFIED).

### Key Link Verification (Updated for ACT-11)

| From | To | Via | 1st pass | 2nd pass |
|------|----|----|----------|----------|
| `ActService::render_pdf` | `organization_service::safe_logo_canonical` | safe_logo через MiniJinja ctx → DocSpec.header.logo_path | ⚠️ PARTIAL (logo не достигал PDF) | ✓ WIRED — `renderer::draw_logo_top_right` теперь потребляет `spec.header.logo_path` и вызывает `surface.draw_image` |
| `PdfRenderer::render_docspec` | `krilla::image::Image::from_png/from_jpeg` | `std::fs::read(logo_path)` → `Image::from_*` → `surface.draw_image` | ✗ DISCONNECTED | ✓ WIRED — `renderer.rs:343-345` + `:381` |

Остальные key-links (13 шт.) проверены в 1st pass — без регрессий.

### Anti-Patterns Found (Updated)

| Finding | 1st pass | 2nd pass | Status |
|---------|----------|----------|--------|
| CR-01 logo not rendered | 🛑 BLOCKER | ✓ CLOSED (b1298eb) | resolved |
| CR-02 missing status guard | 🛑 BLOCKER | ✓ CLOSED (090bc06) | resolved |
| CR-03 missing HashSet dedup | 🛑 BLOCKER | ✓ CLOSED (090bc06) | resolved |
| CR-04 missing quantity bound | 🛑 BLOCKER | ✓ CLOSED (090bc06) | resolved |
| WR-01..WR-11 | ⚠️ WARNING | ⚠️ STILL OPEN | backlog для Phase 7 / follow-up (плана 03-06 явно ограничен BLOCKER scope) |
| IN-01..IN-06 | ℹ️ INFO | ℹ️ STILL OPEN | quality cleanup, не блокирует goal |

Debt-marker gate: `grep -n "TODO|FIXME|XXX|HACK" crates/trackly-app/src/{services/act_service.rs,pdf/renderer.rs} crates/trackly-app/tests/{pdf_logo.rs,acts_returns.rs}` — без unreferenced markers (gate проходит).

### Behavioral Spot-Checks (2nd pass)

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| ACT-11 logo renders into PDF | `cargo test -p trackly-app --test pdf_logo` | 3 tests passed | ✓ PASS |
| ACT-13 return invariants reject invalid payloads | `cargo test -p trackly-app --test acts_returns` | 12 tests passed (4 new + 8 existing) | ✓ PASS |
| Determinism unchanged after CR-01 | `cargo test -p trackly-app --test pdf_determinism` | 2 tests passed; SHA256 = `88df7f9d…` | ✓ PASS |
| Phase 3 regression suite | `cargo test -p trackly-app --test acts_crud --test acts_undo --test acts_search --test acts_display_rule --test acts_e2e_smoke --test acts_http_smoke --test acts_numbering --test pdf_render_act --test pdf_text_extract --test templates_seed --test organization_io` | 8+5+6+4+3+2+1+5+1+4+6 = 45 tests passed | ✓ PASS |
| Clippy clean | `cargo clippy --workspace --all-targets -- -D warnings` | clean | ✓ PASS |
| Manual UI flows (Tauri dev, visual UAT) | (manual) | not attempted in this verification | ? SKIP — routed to human |

### Human Verification Required

См. блок `human_verification:` в frontmatter — 6 UAT-сценариев (один новый — визуальная проверка нового логотипа в шапке + один новый для ACT-13 cross-tab race). Эти UAT не блокируют closure Phase 3 на автоматическом уровне — они валидируют UX-качество и pixel-perfect рендеринг логотипа.

### Gaps Summary

**Нет blocker-gap'ов.** Оба ранее-FAILED requirement (ACT-11, ACT-13) закрыты двумя commit'ами плана 03-06:

- **ACT-11 (CR-01)** — `pdf/renderer.rs::draw_logo_top_right` (новый helper) читает `spec.header.logo_path`, декодирует PNG/JPEG через krilla `Image::from_png`/`Image::from_jpeg`, и эмитит `surface.draw_image` в правом-верхнем углу под header-band. None-branch и missing-file пути gracefully fallback без panic/error.
- **ACT-13 (CR-02 + CR-03 + CR-04)** — `act_service.rs::validate_return` строит два `HashSet<i64>` для dedup; `do_return` per-item loop резолвит `in_work_status_id` один раз и сравнивает `before.status_id != in_work_status_id` → `AppError::Conflict`; перед `insert_act_item_in_tx` подтягивается `handover_qty + already_returned` через JOIN на `acts` с фильтром `deleted_at_utc IS NULL` → `AppError::Validation` при превышении.

Остаются 11 WARNING-уровня и 6 INFO-уровня находок из 03-REVIEW.md (WR-01..WR-11, IN-01..IN-06) — это правомерный backlog, явно out-of-scope плана 03-06 (которому было поручено только BLOCKER closure). Phase 3 готова к финальному closure после прохождения human-UAT шагов выше.

---

_Verified (initial): 2026-05-30_
_Re-verified (2nd pass): 2026-05-30 — after plan 03-06 (commits b1298eb, 090bc06)_
_Verifier: Claude (gsd-verifier)_
_Depth: standard goal-backward + CR-01..CR-04 closure cross-check_
