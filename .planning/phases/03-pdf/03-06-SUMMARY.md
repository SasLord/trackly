---
phase: 03-pdf
plan: 06
subsystem: pdf, acts
tags:
  - phase-3
  - gap-closure
  - pdf
  - acts-returns
  - data-integrity
  - act-11
  - act-13
dependency_graph:
  requires:
    - 03-01 (PdfRenderer + DocSpec + krilla 0.7 surface API)
    - 03-02 (ActService::create + validate_return baseline)
    - 03-03 (ActService::do_return per-item loop + audit_log snapshots)
    - 03-04 (DocSpec.HeaderBlock.logo_path + safe_logo_canonical wiring)
  provides:
    - "renderer::render_docspec: реальный logo draw — std::fs::read(path) → \
       krilla Image::from_png / from_jpeg → surface.push_transform + \
       draw_image(size) + pop (ACT-11)"
    - "draw_logo_top_right helper в renderer.rs: верх-правый угол под \
       header-band, max 80×40pt, aspect-ratio preserved, mime по extension \
       (.png|.jpg|.jpeg)"
    - "graceful fallback: если logo_path = None ИЛИ файл не существует ИЛИ \
       Image::from_* возвращает Err — рендер продолжается без логотипа \
       (no panic, no AppError) (ACT-11)"
    - "validate_return: HashSet<i64> dedup для act_item_id И device_id; \
       payload c дублями → AppError::Validation с указанием field=items[N]. \
       (CR-03 / ACT-13)"
    - "do_return per-item: status guard before.status_id != in_work_status_id \
       → AppError::Conflict «Устройство id=N уже не в работе» (CR-02 / ACT-13)"
    - "do_return per-item: quantity-bound — SELECT quantity FROM act_items \
       + SUM(rai.quantity) FROM act_items rai JOIN acts ra ON ra.id=rai.act_id \
       WHERE ra.parent_act_id=?1 AND rai.device_id=?2 AND \
       ra.deleted_at_utc IS NULL; payload.qty + already_returned > \
       handover_qty → AppError::Validation «превышает выданное» (CR-04 / ACT-13)"
    - "in_work_status_id резолвится один раз внутри writer.execute closure \
       перед per-item loop (а не на каждой итерации)"
    - "tests/pdf_logo.rs (новый, 3 теста): act_with_logo_renders_image_in_pdf, \
       render_without_logo_succeeds, render_with_missing_logo_graceful"
    - "tests/fixtures/logo_test.png (новый, 74 байта, 4×4 px PNG)"
    - "tests/acts_returns.rs +4 теста: return_twice_same_device_rejected, \
       return_with_duplicate_device_id_rejected, \
       return_with_duplicate_act_item_id_rejected, \
       return_quantity_exceeds_handover_rejected"
  affects:
    - "PdfRenderer signature не меняется — изменения внутри render_docspec"
    - "ActService::do_return / validate_return — backwards-compatible \
       строже валидируются (отказы там, где раньше принимались)"
    - "Determinism: фикстура act_42.json имеет logo_path=null → no \
       draw_image → SHA256 88df7f9d… НЕ меняется"
tech_stack:
  added: []
  patterns:
    - "krilla Surface::draw_image(image: Image, size: Size) — без Point; \
       позиционирование через push_transform(Transform::from_translate) + \
       pop. Image::from_png(data: Data, interpolate: bool) -> \
       Result<Image, String>; Vec<u8> → krilla::Data через .into()"
    - "graceful logo failure: вложенные match'и (path → fs::read → \
       Image::from_* по extension) — на любой Err просто continue без \
       логотипа, ошибка логируется через tracing::warn"
    - "HashSet dedup паттерн: HashSet::<i64>::new() + .insert() returns \
       false → AppError::Validation; два независимых set'а для act_item_id \
       и device_id"
    - "Quantity-bound SQL: один query на item внутри той же writer-tx, \
       параметризован полностью (?1 = parent_act_id, ?2 = device_id), \
       JOIN через act_items rai → acts ra (return_acts — это acts с \
       parent_act_id IS NOT NULL); фильтр ra.deleted_at_utc IS NULL \
       исключает уже отменённые возвраты"
  removed: []
test_summary:
  added:
    - "pdf_logo: 3 теста (positive draw, None branch, missing-file graceful)"
    - "acts_returns: 4 теста интегрити-инвариантов"
  regression_run:
    - "acts_crud, acts_undo, acts_display_rule, acts_e2e_smoke, acts_search, \
       acts_http_smoke, acts_numbering, pdf_render_act, pdf_text_extract, \
       pdf_determinism — все зелёные"
  determinism: "SHA256 act_42 unchanged: \
    88df7f9d69c5db10a4685f0aa5d390caec90b045067e35cc1caba33efdd15d1f"
requirements_closed:
  - "ACT-11 — PDF Акта с шапкой и логотипом организации (CR-01 закрыт)"
  - "ACT-13 — Транзакционная гарантия возврата (CR-02/03/04 закрыты)"
deviations:
  - "krilla 0.7 Surface::draw_image signature отличается от черновика в \
     плане — нет Point-аргумента. Позиционирование через push_transform/pop. \
     Документировано в commit b1298eb."
  - "DeviceRow.status_id — i64 (не Option<i64>), как предполагалось черновиком \
     CR-02. Status guard переписан под i64-сравнение."
  - "Test 9 (status guard) использует handover на 2 устройства, иначе первый \
     возврат архивирует parent и срабатывает archived-check раньше, чем \
     status-guard."
  - "Добавлен pdf_logo_probe.rs временно для калибровки PDF-byte markers; \
     удалён до коммита. Финальная assertion ищет /Subtype /Image ИЛИ /XObject."
commits:
  - "b1298eb — feat(03-06): PDF logo rendering via krilla Image XObject (ACT-11 / CR-01)"
  - "090bc06 — feat(03-06): return-tx integrity invariants — status guard + dedup + qty bound (ACT-13)"
---

# Plan 03-06 Summary — Phase 3 Gap Closure

**Goal:** Закрыть 2 BLOCKER-уровня дефицита из 03-VERIFICATION.md без переписывания
уже отгруженных планов 03-01..03-05.

## What shipped

### Task 1 — Renderer logo draw (commit b1298eb)
`pdf/renderer.rs::render_docspec` теперь читает `spec.header.logo_path` и
вызывает `surface.draw_image` через позиционирующий `push_transform/pop`
паттерн krilla 0.7. Helper `draw_logo_top_right` сохраняет aspect-ratio,
ограничивает 80×40pt в правом верхнем углу под header-band, определяет
mime по расширению (`.png`/`.jpg`/`.jpeg`), и при любой ошибке (отсутствие
файла, битый PNG, неподдерживаемый формат) gracefully продолжает без
логотипа. Тестовая фикстура `logo_test.png` (4×4 px, 74 байта) подтверждает
positive path; missing-file и None-branch тесты подтверждают graceful path.

### Task 2 — Return-tx integrity (commit 090bc06)
Три новых инварианта в `services/act_service.rs`:

1. **Dedup (CR-03)** — `validate_return` строит два `HashSet<i64>` и
   отвергает дублирующиеся `act_item_id` и `device_id` в payload до начала
   транзакции.
2. **Status guard (CR-02)** — внутри per-item loop в writer-tx, после
   `devices_repo.get_in_tx(device_id)`, проверка `before.status_id ==
   in_work_status_id` (i64); при несовпадении — `AppError::Conflict`.
   `in_work_status_id` резолвится один раз перед циклом.
3. **Quantity bound (CR-04)** — один SQL на item: `SELECT quantity FROM
   act_items WHERE id=?1 AND act_id=?2` + `SELECT SUM(rai.quantity) FROM
   act_items rai JOIN acts ra ON ra.id=rai.act_id WHERE ra.parent_act_id=?1
   AND rai.device_id=?2 AND ra.deleted_at_utc IS NULL`. При
   `payload.qty + already_returned > handover_qty` — `AppError::Validation`.

## Verification

- `cargo test -p trackly-app --test pdf_logo` — 3/3
- `cargo test -p trackly-app --test acts_returns` — 12/12 (8 старых + 4 новых)
- `cargo test -p trackly-app --test pdf_determinism` — 2/2, SHA256 без изменений
- Все остальные acts_*/pdf_*/templates_*/organization_* тесты Phase 3 — зелёные
- `cargo clippy --workspace --all-targets -- -D warnings` — чисто
- `cargo fmt --all -- --check` — чисто

## Gaps closed

| Gap | Source | Status |
|---|---|---|
| ACT-11 — PDF logo не рендерится | CR-01 (REVIEW.md) | ✓ CLOSED |
| ACT-13 — status check отсутствует | CR-02 (REVIEW.md) | ✓ CLOSED |
| ACT-13 — нет dedup payload | CR-03 (REVIEW.md) | ✓ CLOSED |
| ACT-13 — нет quantity bound | CR-04 (REVIEW.md) | ✓ CLOSED |

Phase 3 теперь имеет 16/16 requirements satisfied и готова к финальной верификации.
