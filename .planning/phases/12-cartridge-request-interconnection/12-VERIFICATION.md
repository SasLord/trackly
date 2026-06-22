---
phase: 12-cartridge-request-interconnection
verified: 2026-06-22T11:38:43Z
status: human_needed
score: 9/9 must-haves verified (programmatically); 1 item requires live human pass
overrides_applied: 0
human_verification:
  - test: "End-to-end happy path: создать заявку «Замена картриджа» → принять → «Установить картридж» → выбрать картридж из списка → завершить → проверить История"
    expected: "Список картриджей в модалке содержит только записи (статус «На складе», заряд Полный/Частичный для картриджей или Новый/Изношенный для барабанов, модель = модель заявки); поля «Кому отдал»/«Расположение» предзаполнены, но редактируемы; после установки заявка переходит в «Выполнена», История показывает строку вида «Установлен C-NNNNNN (Brand Model)»"
    why_human: "Требует визуальной проверки рендера модалки, автокомплитов и истории заявки в живом UI (desktop webview и/или LAN-браузер). 12-03-SUMMARY.md прямо отмечает, что Task 4 (`checkpoint:human-verify`) был авто-одобрен под AUTO_MODE и не прогонялся в реальной браузерной сессии — подтверждён только статическим код-ревью (effectiveCartridge/CartridgeSelect гейты) и automated svelte-check/build. Это соответствует прямому указанию в context_notes текущего запуска верификации."
  - test: "DISC-02 empty-state: открыть «Установить картридж» из заявки на модель, для которой на складе нет подходящих картриджей"
    expected: "Список показывает «Нет подходящих картриджей на складе», форма не блокируется (модалку можно закрыть, заявку можно отклонить или использовать старый cartridge-centric вход)"
    why_human: "Тот же checkpoint — подтверждено только по коду (`CartridgeSelect.svelte` рендерит `<option value=\"\" disabled>Нет подходящих картриджей на складе</option>` при `options.length === 0`), не живой сессией."
  - test: "D-08 regression: открыть карточку картриджа со статусом «На складе» напрямую (меню картриджа → «Установить в принтер»)"
    expected: "Старая форма работает без изменений — БЕЗ нового селектора картриджа (его там нет, картридж уже выбран контекстом меню)"
    why_human: "Подтверждено статически — `CartridgesPage.svelte` передаёт `cartridge={operationModalCartridge}` (не `null`), а `OperationModal` рендерит `CartridgeSelect` только при `cartridge === null` — но визуальное отсутствие регрессии (расположение полей, фокус, скролл) не проверялось интерактивно."
---

# Phase 12: Взаимосвязь картриджной заявки — Verification Report

**Phase Goal:** Сделать установку картриджа из заявки «Замена картриджа» полнофункциональной и взаимосвязанной: выбор физического картриджа из БД (на складе, заряд Полный/Частичный, совместимый с моделью заявки), авто-подстановка Расположения из принтера и «Кому отдал» из заявителя (оба редактируемы), запись установленного картриджа в `completed_cartridge_id` заявки и отражение в истории. Старый cartridge-centric вход сохраняется.

**Verified:** 2026-06-22T11:38:43Z
**Status:** human_needed
**Re-verification:** Нет — первичная верификация.

## Контекст верификации

Фаза состоит из 3 планов (12-01 backend filters, 12-02 service wiring + RBAC, 12-03 frontend selector). Дополнительно прошёл код-ревью (`12-REVIEW.md`): найден 1 блокер (CR-01: `installable_only` исключал фотобарабаны) + 7 предупреждений; блокер и 4 ключевых предупреждения (WR-01..WR-04) исправлены и подтверждены в `12-REVIEW-FIX.md`, остальные (WR-05..WR-07, IN-01..IN-04) осознанно отложены как некритичные.

D-01..D-08 — фазовые decision-ID из `12-CONTEXT.md`, не строки `REQUIREMENTS.md` (намеренно, фаза основана на пользовательских решениях, а не формальных REQ). Их отсутствие в `REQUIREMENTS.md` не считается пробелом.

Верификация прогнала реальный код, а не доверяла тексту SUMMARY:
- Полный `cargo test --workspace` (с `TRACKLY_AD_MOCK=1`) — **0 failed** по всему воркспейсу.
- Целевые наборы: `cargo test -p trackly-app --test cartridges_lifecycle` (11/11 ok, включая `installable_only_includes_new_drum_excludes_spent_drum` из фикса ревью), `cargo test -p trackly-app --test phase06_stubs` (18/18 ok, включая `test_req_cart_link` без `#[ignore]`, `history_shows_cartridge_snapshot_after_complete`, `history_complete_without_cartridge_keeps_plain_notes`, `request_dto_carries_printer_location`, `request_dto_printer_location_none_when_no_location_or_no_printer`), `cargo test -p trackly-app --test role_endpoint_matrix` (ok, включает Case 31/32).
- `cargo build --workspace` — чисто. `cargo clippy -p trackly-core -p trackly-app -p trackly-infra -- -D warnings` — чисто (lib-таргеты; pre-existing `len_zero` сбои в `--tests` подтверждены изолированными от файлов фазы 12).
- `pnpm --dir ui svelte-check` — 0 ошибок, 36 пред-существующих warnings (ни один не в файлах фазы 12). `pnpm --dir ui build` — успешно. `pnpm --dir ui lint` — 22 пред-существующие ошибки, ни одна не в `CartridgeSelect.svelte`/`OperationModal.svelte`/`RequestDetail.svelte`.
- Прочитан весь модифицированный код (не только grep) — domain/DTO/SQL/service/frontend — построчно сверен с планами и итоговым ревью-фиксом.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Список картриджей фильтруется до записей со статусом «На складе» и устанавливаемым зарядом, kind-aware (Полный/Частичный для картриджей, Новый/Изношенный для барабанов) — D-01 | ✓ VERIFIED | `cartridges_sqlite.rs:966-969,988-991`: `(?5 = 0 OR (c.status_id = 1 AND ((m.kind_id = 1 AND c.state_id IN (1,2)) OR (m.kind_id = 2 AND c.state_id IN (4,5)))))` — пост-ревью kind-aware фикс (CR-01/WR-01) применён в обоих COUNT/SELECT запросах. 11/11 тестов `cartridges_lifecycle.rs` зелёные, включая `installable_only_includes_new_drum_excludes_spent_drum`. |
| 2 | Список дополнительно фильтруется по совместимости с моделью заявки (`request.cartridgeModelId`) — D-02 | ✓ VERIFIED | `OperationModal.svelte:142-150`: `cartridges.list({..., model_id: cartridgeModelId ?? null, ...})`; `cartridges_lifecycle.rs::installable_only_respects_model_filter` зелёный. WR-02 fix добавляет явное предупреждение `noModelScopeWarning`, когда `cartridgeModelId === undefined` (DISC-01 fallback покрыт, не скрыт). |
| 3 | После выбора картриджа форма установки работает как раньше (Дата/Кто выдал/Кому выдал/Расположение), submit вызывает `cartridges.transition({op:'install', cartridge_id, ...})` — D-03 | ✓ VERIFIED | `effectiveCartridge = $derived(cartridge ?? selectedCartridge)` пронизывает `isDrum`/`defaultStateId`/`buildPayload()`/`validate()`/`canSubmit`/`handleSubmit()` — единая логика для обоих входов; `buildPayload()` строит `{op:'install', cartridge_id: effectiveCartridge!.id, ...}` без изменений семантики. |
| 4 | «Кому отдал» предзаполняется из `request.requesterName`, остаётся редактируемым — D-04 | ✓ VERIFIED | `RequestDetail.svelte:606`: `prefillGivenToName={request.requesterName ?? undefined}`; reset-эффект в `OperationModal.svelte:102`: `givenToName = prefillGivenToName ?? ''`; поле рендерится через `PersonAutocomplete` без `disabled`/`readonly`. |
| 5 | «Расположение» предзаполняется из расположения принтера заявки (`printer_location` JOIN), остаётся редактируемым, NULL-safe — D-05 | ✓ VERIFIED | SQL: `requests_sqlite.rs:43,47`: `LEFT JOIN locations dl ON dl.id = d.location_id`, `dl.name AS printer_location` (idx 19, append-only). `RequestRow`/`RequestDto` несут поле; 2 теста (`request_dto_carries_printer_location`, `request_dto_printer_location_none_when_no_location_or_no_printer`) зелёные. Frontend: `RequestDetail.svelte:605` → `prefillLocation={request.printerLocation ?? undefined}` → `OperationModal.svelte:103`: `location = prefillLocation ?? ''`; `LocationAutocomplete` без `disabled`/`readonly`. |
| 6 | После завершения заявки `completed_cartridge_id` записывается равным id установленного картриджа — D-06 | ✓ VERIFIED | SQL `requests_sqlite.rs:158`: `completed_cartridge_id = COALESCE(?4, completed_cartridge_id)` (уже существовало, теперь подтверждено реальным тестом `test_req_cart_link`, не `#[ignore]`-стабом). Frontend: `RequestDetail.svelte:326`: `linkedCartridgeId: cartridgeId` (был `null`). WR-04 fix: версия перечитывается через `requests.get(requestId)` непосредственно перед `complete`, устраняя гонку с устаревшей `version`. |
| 7 | История заявки показывает человекочитаемый код+модель установленного картриджа — D-07 | ✓ VERIFIED | `request_service.rs:484-504`: pre-write `spawn_blocking` чтение `cartridge_repo.get(&conn, cid)`, построение `"Установлен {code} ({brand} {name})"`, фолд в существующий `notes_json["notes"]` ключ (объединение с operator-notes через `"; "`). Тесты `history_shows_cartridge_snapshot_after_complete` и `history_complete_without_cartridge_keeps_plain_notes` зелёные (positive + regression). |
| 8 | Старый cartridge-centric вход («картридж → Установить в принтер») продолжает работать без изменений — D-08 | ✓ VERIFIED | `CartridgesPage.svelte:427-433`: `<OperationModal cartridge={operationModalCartridge} onSuccess={handleOperationSuccess} />` — без новых пропов (`cartridgeModelId`/`prefillLocation`/`prefillGivenToName` не передаются), `handleOperationSuccess` синхронна. `OperationModal` рендерит `CartridgeSelect` только при `op === 'install' && cartridge === null` — путь меню гарантированно не затронут (`cartridge` всегда non-null там). |
| 9 | Employee получает 403 Forbidden при попытке вызвать `cartridges_transition`/`requests_transition` через HTTP (T-12-01 закрытие пробела покрытия) | ✓ VERIFIED | `role_endpoint_matrix.rs` Case 31 (cartridges_transition) и Case 32 (requests_transition на собственной заявке) — оба `assert_eq!(status, StatusCode::FORBIDDEN, ...)`. Полный прогон `role_endpoint_matrix_test` зелёный. |

**Score:** 9/9 истин подтверждены кодом и тестами. 1 дополнительный пункт (полный живой UI-прогон) вынесен в Human Verification — не баллируется как FAIL, см. ниже.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/trackly-core/src/domain/cartridges.rs` | `CartridgeFilter.installable_only: bool` | ✓ VERIFIED | Поле присутствует (line 213), доккомент описывает kind-aware семантику пост-фикса. |
| `crates/trackly-infra/src/repos/cartridges_sqlite.rs` | `list()` фильтрует `state_id IN (1,2)` kind-aware под `installable_only` | ✓ VERIFIED | Обе ветки (COUNT/SELECT) несут идентичный kind-aware предикат с `status_id = 1` включённым (WR-01 fix). |
| `crates/trackly-infra/src/repos/requests_sqlite.rs` | `SELECT_REQUESTS` JOIN locations + `printer_location` колонка idx 19 | ✓ VERIFIED | `LEFT JOIN locations dl`, `dl.name AS printer_location` — последняя колонка, append-only convention соблюдена. |
| `crates/trackly-app/tests/cartridges_lifecycle.rs` | RED→GREEN тест на `installable_only` фильтр | ✓ VERIFIED | 11 тестов, включая 4 из Plan 01 + 1 регрессионный из ревью-фикса (drum). Все зелёные. |
| `crates/trackly-app/src/services/request_service.rs` | `transition()` читает картридж через `cartridge_repo` и обогащает `notes_json` | ✓ VERIFIED | `cartridge_repo: Arc<SqliteCartridgeRepository>` поле + pre-write read + notes enrichment, построчно сверено. |
| `crates/trackly-app/tests/phase06_stubs.rs` | `test_req_cart_link` как `#[tokio::test]`, не `#[ignore]` | ✓ VERIFIED | `grep` подтверждает отсутствие `#[ignore]` рядом с тестом; тест реально запускается и проходит. |
| `crates/trackly-app/tests/role_endpoint_matrix.rs` | Case на employee 403 для `cartridges_transition`/`requests_transition` | ✓ VERIFIED | Case 31/32 присутствуют, нумерация скорректирована относительно плана (документировано как deviation, не баг). |
| `ui/src/lib/components/CartridgeSelect.svelte` | Флэт-список картриджей (без optgroup) | ✓ VERIFIED | 117 строк, NULL-safe рендер `{code} — {brand model} (state)`, DISC-02 empty-state текст присутствует. |
| `ui/src/features/cartridges/OperationModal.svelte` | Селектор + auto-prefill props + `onSuccess(cartridgeId)` | ✓ VERIFIED | Все новые пропы (`cartridgeModelId`, `prefillLocation`, `prefillGivenToName`), `effectiveCartridge`, `CartridgeSelect` импорт и использование подтверждены. |
| `ui/src/features/requests/RequestDetail.svelte` | `handleInstallSuccess(cartridgeId)` передаёт `linkedCartridgeId` | ✓ VERIFIED | Сигнатура изменена, `linkedCartridgeId: cartridgeId` передаётся, WR-04 re-fetch версии перед complete присутствует. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `dto/cartridge.rs::CartridgeFilter` | `domain/cartridges.rs::CartridgeFilter` | `into_domain()` | ✓ WIRED | `installable_only: self.installable_only` явно прокинуто. |
| `requests_sqlite.rs::SELECT_REQUESTS` | `domain/requests.rs::RequestRow` | `map_row_request row.get(19)` | ✓ WIRED | `printer_location: row.get(19)?` — последний параметр маппера, индекс совпадает с позицией колонки в SELECT. |
| `RequestDetail.svelte` | `OperationModal.svelte` | props `cartridgeModelId`/`prefillLocation`/`prefillGivenToName` + `onSuccess` callback | ✓ WIRED | Все три пропа переданы из `request.*`; `onSuccess={handleInstallSuccess}` соответствует новой сигнатуре `(cartridgeId: number) => Promise<void>`. |
| `OperationModal.svelte` | `ui/src/features/cartridges/api.ts` | `cartridges.list({status_id:1, installable_only:true, model_id})` | ✓ WIRED | Вызов присутствует внутри гейтированного `$effect` (`open && op==='install' && cartridge===null`). |
| `RequestDetail.svelte` | `ui/src/features/requests/api.ts` | `requests.transition({op:'complete', linkedCartridgeId: cartridgeId})` | ✓ WIRED | Подтверждено вместе с WR-04 (`requests.get` непосредственно перед transition для свежей версии). |
| `request_service.rs::transition()` | `cartridges_sqlite.rs::SqliteCartridgeRepository::get` | `cartridge_repo.get(&conn, id)` внутри `spawn_blocking` | ✓ WIRED | Чтение происходит ДО входа в writer-транзакцию, результат фолдится в `notes_json` — соответствует плану. |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|---------------------|--------|
| `CartridgeSelect` (через `OperationModal`) | `cartridgeOptions` | `cartridges.list()` → `cartridges_list` Tauri/HTTP команда → `CartridgeService::list()` → SQL `SELECT ... WHERE installable_only-predicate` | Да | ✓ FLOWING — реальный SQL-запрос к БД, не статичный fallback; подтверждено зелёными тестами на реальном SQLite (`test_writer_and_readers`). |
| История заявки (`RequestDetail`) | `entry.notes` | `get_history()` парсит `payload_json` из `audit_log`, куда `transition()` пишет реальный `notes_json` с обогащённой строкой | Да | ✓ FLOWING — обогащение происходит на сервере при реальном чтении картриджа (`cartridge_repo.get`), не client-side заглушка. |
| `completedCartridgeId` (RequestDto) | `dto.completed_cartridge_id` | SQL `UPDATE requests SET completed_cartridge_id = COALESCE(?4, completed_cartridge_id)` | Да | ✓ FLOWING — подтверждено `test_req_cart_link` через реальную `RequestService::transition()` на временной SQLite БД, не мок. |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Backend: kind-aware installable фильтр возвращает только корректные записи | `cargo test -p trackly-app --test cartridges_lifecycle -- --test-threads=1` | 11 passed; 0 failed | ✓ PASS |
| Backend: D-06/D-07 история + связь картридж↔заявка | `cargo test -p trackly-app --test phase06_stubs -- --test-threads=1` | 18 passed; 0 failed | ✓ PASS |
| RBAC: employee 403 на cartridges_transition/requests_transition | `cargo test -p trackly-app --test role_endpoint_matrix -- --test-threads=1` | 1 passed (драйвер всех 32 кейсов); 0 failed | ✓ PASS |
| Полный воркспейс без регрессий | `TRACKLY_AD_MOCK=1 cargo test --workspace` | все `test result: ok`, 0 failed по всем бинарям | ✓ PASS |
| Frontend типы и сборка | `pnpm --dir ui svelte-check && pnpm --dir ui build` | 0 errors (36 pre-existing warnings, не в файлах фазы); сборка успешна | ✓ PASS |
| Frontend lint без новых ошибок в файлах фазы | `pnpm --dir ui lint` | 22 pre-existing errors, 0 в `CartridgeSelect.svelte`/`OperationModal.svelte`/`RequestDetail.svelte` | ✓ PASS |
| Backend build + clippy (lib-таргеты) | `cargo build --workspace`, `cargo clippy -p trackly-core -p trackly-app -p trackly-infra -- -D warnings` | Оба чисто | ✓ PASS |

### Probe Execution

Step 7c пропущен: фаза не декларирует и не подразумевает `scripts/*/tests/probe-*.sh` — это backend/frontend feature-фаза, а не migration/tooling-фаза. `grep -R "probe-" .planning/phases/12-*/12-0*-PLAN.md 12-0*-SUMMARY.md` не находит упоминаний probe-скриптов.

### Requirements Coverage

D-01..D-08 — фазовые decision-ID из `12-CONTEXT.md`, явно НЕ являются строками `REQUIREMENTS.md` traceability-таблицы (подтверждено: `grep "D-0[1-8]" .planning/REQUIREMENTS.md` — ноль совпадений, что ОЖИДАЕМО и не является пробелом per инструкции задачи).

| Decision | Источник | Статус | Evidence |
|----------|----------|--------|----------|
| D-01 | 12-CONTEXT.md | ✓ SATISFIED | См. Truth #1. |
| D-02 | 12-CONTEXT.md | ✓ SATISFIED | См. Truth #2. |
| D-03 | 12-CONTEXT.md | ✓ SATISFIED | См. Truth #3. |
| D-04 | 12-CONTEXT.md | ✓ SATISFIED | См. Truth #4. |
| D-05 | 12-CONTEXT.md | ✓ SATISFIED | См. Truth #5. |
| D-06 | 12-CONTEXT.md | ✓ SATISFIED | См. Truth #6. |
| D-07 | 12-CONTEXT.md | ✓ SATISFIED | См. Truth #7. |
| D-08 | 12-CONTEXT.md | ✓ SATISFIED | См. Truth #8. |

Орфанных REQ-ID, отнесённых к Фазе 12 в `REQUIREMENTS.md`, не найдено (`grep "Phase 12" .planning/REQUIREMENTS.md` — ноль совпадений) — это ожидаемо, фаза целиком вне формальной REQ-ID traceability-системы по дизайну.

### Anti-Patterns Found

Сканирование всех файлов, изменённых в Wave 1-3 (10 файлов: domain/DTO/SQL/service/3 frontend) на `TBD|FIXME|XXX|TODO|HACK|PLACEHOLDER|placeholder|coming soon|not yet implemented` — **0 совпадений**. Никаких debt-маркеров, заглушек или пустых реализаций не найдено.

Отдельно проверено: `readonly`/`disabled` отсутствуют на полях «Кому отдал»/«Расположение» (подтверждает реальную редактируемость D-04/D-05, не косметическую видимость).

Известные, осознанно отложенные пункты ревью (не блокеры для цели фазы, задокументированы в `12-REVIEW-FIX.md`):
- WR-05 (`printer_options` LEFT JOIN устойчивость к переименованию seed) — не относится к новой логике фазы 12.
- WR-06 (UTC vs локальное время в истории) — пред-существующая проблема дисплея дат, не специфична для D-06/D-07.
- WR-07 (`get_history` молча проглатывает невалидный JSON) — debuggability-гэп, не функциональный дефект текущего потока.
- IN-01..IN-04 — мелкие UX-нюансы (дублирующийся toast устранён WR-03-фиксом; оставшиеся — `printerContextHint` показывает id, `actionLabel` не покрывает `ad_register_approve`, non-null assertion в `buildPayload`).

Эти пункты не блокируют достижение цели фазы (выбор картриджа, авто-подстановка, запись связи, история, сохранение старого входа) — корректно классифицированы ревьюером как warning/info, не critical.

### Human Verification Required

### 1. End-to-end happy path установки картриджа из заявки

**Test:** Создать заявку «Замена картриджа» на принтер с известным расположением → принять в работу → «Установить картридж» → выбрать картридж из списка → заполнить «Кто выдал» → подтвердить.
**Expected:** Список картриджей в модалке содержит только подходящие записи (статус «На складе», устанавливаемый заряд, модель = модель заявки); «Кому отдал»/«Расположение» предзаполнены, но редактируемы; после установки заявка переходит в «Выполнена»; История показывает строку с кодом и моделью картриджа.
**Why human:** `12-03-SUMMARY.md` прямо признаёт, что `checkpoint:human-verify` (Task 4 плана 12-03) был авто-одобрен под AUTO_MODE и НЕ прогонялся в живой браузерной/десктоп-сессии — подтверждение только статическим код-ревью + `svelte-check`/`build`. Согласно явному указанию в контексте текущего запуска верификации, этот пункт направляется в human_verification, а не засчитывается как FAILED.

### 2. DISC-02 empty-state при отсутствии подходящих картриджей

**Test:** Открыть «Установить картридж» из заявки на модель, для которой на складе нет совместимых картриджей (или все заряжены «Пустой»/списаны).
**Expected:** Список показывает «Нет подходящих картриджей на складе»; форма не блокируется — модалку можно закрыть, заявку отклонить, или воспользоваться старым cartridge-centric входом.
**Why human:** Тот же неподтверждённый живой checkpoint; код корректен (`CartridgeSelect.svelte:41`), но визуальное поведение (фокус, скролл, читаемость предупреждения) не проверено интерактивно.

### 3. D-08 регрессия — старый cartridge-centric вход

**Test:** Открыть карточку картриджа со статусом «На складе» напрямую (не через заявку) → меню → «Установить в принтер».
**Expected:** Старая форма работает ровно как раньше, без нового селектора картриджа и без полей-предзаполнений из заявки.
**Why human:** Подтверждено статическим анализом (`CartridgesPage.svelte` не передаёт новые пропы; `OperationModal` гейтирует `CartridgeSelect` на `cartridge === null`), но визуальная неизменность UI не проверена в реальном сеансе.

## Gaps Summary

Программная верификация не нашла ни одного провала: все 9 наблюдаемых истин (D-01..D-08 + RBAC-закрытие T-12-01) подтверждены реальным кодом и зелёными тестами, включая критический фикс ревью (kind-aware `installable_only`, иначе фотобарабаны были бы исключены — CR-01) и 4 связанных warning-фикса (WR-01..WR-04). Полный `cargo test --workspace` зелёный без единого сбоя; frontend `svelte-check`/`build`/`lint` чисты относительно файлов фазы.

Единственная причина статуса `human_needed`, а не `passed` — операторский UI-флоу (выбор картриджа, предзаполнение полей, переход в «Выполнена», текст истории, D-08 регрессия, DISC-02 empty-state) был верифицирован кодовым анализом и автоматическими гейтами (`svelte-check`/`build`), но не живой интерактивной сессией браузера/десктоп-приложения — это прямо признано в `12-03-SUMMARY.md` и явно помечено в инструкциях текущего запуска как пункт для `human_verification`, а не `gaps_found`. Закрытие этого пункта не требует кодовых изменений — только ручной прогон шагов из `12-03-PLAN.md` Task 4 `<how-to-verify>`.

---

_Verified: 2026-06-22T11:38:43Z_
_Verifier: Claude (gsd-verifier)_
