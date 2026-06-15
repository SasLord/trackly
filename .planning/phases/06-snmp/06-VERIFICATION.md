---
phase: 06-snmp
verified: 2026-06-15T08:30:00Z
human_uat: 2026-06-15T14:15:00Z
status: verified
score: 6/6 must-haves verified (5 code + 1 deferred to Phase 8; human UAT passed)
re_verification:
  previous_status: gaps_found
  previous_score: 4/6
  gaps_closed:
    - "Gap 1: requests_create arg-key mismatch — исправлен (dto parity в api.ts + http handler)"
    - "Gap 2: requests_counts и requests_get_history — исправлены (переименование + реализация REQ-07)"
    - "Gap 3: printers_admit stub — реализован (device type=Принтер + printers row per IP)"
    - "Gap 4: ручное заведение принтера — PrinterCreateModal.svelte + кнопка в PrintersPage"
    - "Gap 5: select принтера в замене картриджа — переключён на devices type_id=2"
  gaps_remaining: []
  regressions: []
human_verification:
  - test: "Создать заявку «Свободная форма» через веб-интерфейс (cargo tauri dev, TRACKLY_SNMP_MOCK=1)"
    expected: "Форма отправляется, тост «Заявка создана», новая заявка появляется в списке"
    why_human: "arg-key fix был верифицирован частично в UAT — фиксирован NaN-баг, но полный поток заявка→список не прошёл автоматически"
  - test: "Открыть созданную заявку → блок «История»"
    expected: "Показывает строку создания с датой (не NaN), именем пользователя; при reject/complete — причина в поле notes"
    why_human: "NaN-баг был найден и исправлен post-checkpoint (commit 8654f89), нужна финальная ручная проверка"
  - test: "Кнопка «Завести принтер» в разделе Принтеры"
    expected: "Форма открывается, можно ввести Наименование (обязательно) + Расположение + IP/community (опционально); submit создаёт принтер и обновляет список"
    why_human: "PrinterCreateModal — новый компонент 06-08, не прошёл runtime UAT"
  - test: "Discovery → выбрать принтеры → Завести выбранные (TRACKLY_SNMP_MOCK=1)"
    expected: "printers_admit создаёт устройство(а) type=Принтер и записи в printers; список принтеров обновляется"
    why_human: "admit реализован (не заглушка), но не прошёл runtime smoke-test с mock-клиентом"
  - test: "Форма «Замена картриджа» → поле выбора принтера"
    expected: "Dropdown содержит все устройства с type=Принтер (включая те, у кого нет IP/SNMP), не только SNMP-принтеры"
    why_human: "Переключение источника devices.list(type_id=2) требует ручной проверки чтобы убедиться что USB-принтеры действительно видны"
human_uat_result:
  performed: 2026-06-15T14:15:00Z
  outcome: approved
  by: "Alexander P. (пользователь, cargo tauri dev + TRACKLY_SNMP_MOCK=1)"
  details: |
    Все 5 runtime-UAT сценариев подтверждены пользователем в двух checkpoint-итерациях:
    - 06-07 checkpoint: создание заявки «Свободная форма» → список ✓; История с
      корректной датой (после fix NaN, commit 8654f89) ✓; счётчики статусов ✓.
    - 06-08 checkpoint: Discovery admit (PRN-01) ✓; ручная форма «Завести принтер»
      (PRN-04) ✓; select «Замена картриджа» содержит все принтеры-устройства ✓.
    Truth 6 (вход employee через браузер по ролям) — deferred до Phase 8 (AD),
    не gap фазы 6.
---

# Phase 6: Принтеры (SNMP-мониторинг) и Заявки — Re-Verification Report

**Phase Goal:** Включить SNMP-мониторинг сетевых принтеров (Pantum/Kyocera/HP/Canon), discovery подсети, детекцию Pantum-зависания (alert-only), плюс портал заявок для сотрудников с двумя типами и жизненным циклом.
**Verified:** 2026-06-15T08:30:00Z (код) · **Human UAT:** 2026-06-15T14:15:00Z (approved)
**Status:** verified
**Re-verification:** Yes — после gap-closure (06-07 + 06-08); все runtime-потоки подтверждены пользователем

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Миграции V020-V025, snmp2, core/infra SNMP-слой | ✓ VERIFIED | Без изменений с начальной верификации. user_version=25, cargo check зелёный. |
| 2 | Сервисы + репозитории + транспорт (Tauri/HTTP/WS) компилируются и протестированы | ✓ VERIFIED | `cargo check --workspace` → Finished 0 errors; `cargo test -p trackly-infra` → 52 passed; `cargo test -p trackly-app` → 1 passed; export_bindings → 1 passed |
| 3 | Discovery находит и **заводит** принтеры | ✓ VERIFIED | `build_printers_admit` полностью реализован (commit a1c147b): duplicate check → SNMP probe → `devices.create(type_id=2)` → `printers.create_from_device`. Более не заглушка. HTTP: `handler_admit` + `POST /api/v1/printers_admit`. Authorize: `MutatePrinters`. |
| 4 | Сотрудник создаёт заявку (свободная форма / замена картриджа) | ✓ VERIFIED (code) | `api.ts`: `create({ dto: payload })` (commit fc9c514); `requests_counts` (не `requests_status_counts`) (commit fc9c514); Tauri: `requests_create(dto: RequestCreateDto)`. Wiring полный. Runtime — human_needed. |
| 5 | UI принтеров отображает мониторинг (toner/alert/картридж) | ✓ VERIFIED (mock) | Без изменений с начальной верификации. TonerGauge/AlertBanner/PrinterDetail рендерятся при TRACKLY_SNMP_MOCK=1. |
| 6 | Портал заявок работает для роли «Сотрудник» | ? PARTIAL | Серверная авторизация корректна; `RequestDetail.svelte:288` lifecycle-кнопки в `{#if isSpecialist}`; history блок подключён через `getHistory(id)` → `requests_get_history`. Вход employee через браузер — отложен до Phase 8 (AD). Runtime — human_needed. |

**Score:** 5/6 truths verified in code (truth 6 остаётся PARTIAL — employee web login отложен в Phase 8, зафиксировано в 06-VERIFICATION.md)

### Gaps Closed (from previous verification)

| Gap | Fix | Commit | Status |
|-----|-----|--------|--------|
| requests_create arg-key mismatch (`dto` vs `payload`) | `api.ts` исправлен на `{ dto: payload }` | fc9c514 | CLOSED |
| `requests_status_counts` → `requests_counts` | api.ts переименован | fc9c514 | CLOSED |
| `requests_get_history` — команды не было | Реализован end-to-end (repo → service → tauri cmd → http handler → bindings.ts) | 734e257 | CLOSED |
| History NaN-date bug (`AuditEntryDto` snake_case) | Новый `RequestHistoryEntryDto` (camelCase `createdAtUtc` + `actorName` + `notes`), LEFT JOIN users | 8654f89 | CLOSED |
| `printers_admit` — заглушка `Vec::new()` | Полная реализация двух-шагового create | a1c147b | CLOSED |
| Нет ручного заведения принтера (PRN-04) | `PrinterCreateModal.svelte` + кнопка «Завести принтер» в `PrintersPage.svelte` | f3ae33b | CLOSED |
| Select принтера в форме замены — неверный источник | `devices.list({ type_id: 2 })` вместо `printers.list()` | f3ae33b | CLOSED |
| a11y: `<nav role="tablist">` конфликт | Заменён на `<div role="tablist" aria-label="...">` в обоих SearchAndTabs | e73c629 | CLOSED |

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/trackly-app/src/tauri_cmds/printers.rs` | `build_printers_admit` — полная реализация | ✓ VERIFIED | 80+ строк реализации, authorize, duplicate check, probe, device+printer create |
| `crates/trackly-app/src/tauri_cmds/requests.rs` | `requests_create(dto: ...)`, `requests_get_history(id: ...)` | ✓ VERIFIED | Arg `dto` на строках 43, 126; `requests_get_history` на строках 65, 172 |
| `crates/trackly-app/src/http/requests.rs` | `handler_get_history` + route `requests_get_history` | ✓ VERIFIED | Строки 162, 193-194 |
| `crates/trackly-app/src/http/printers.rs` | `handler_admit` + `POST /api/v1/printers_admit` | ✓ VERIFIED | Строки 68, 166, 191 |
| `ui/src/features/printers/PrinterCreateModal.svelte` | Форма ручного заведения принтера | ✓ VERIFIED | Существует; двух-шаговый submit (devices.create → printers.create); IP опционален |
| `ui/src/features/requests/api.ts` | `dto` arg key, `requests_counts`, `requests_get_history` | ✓ VERIFIED | Строка 23: `{ dto: payload }`; строка 30: `requests_counts`; строка 32: `requests_get_history` |
| `ui/src/features/requests/RequestFormModal.svelte` | Принтеры из `devices.list(type_id=2)` | ✓ VERIFIED | Строки 70-77: `devices.list({ type_id: 2, ... })`; `availablePrinters: DeviceDto[]` |
| `ui/src/bindings.ts` | `RequestHistoryEntryDto` с camelCase полями | ✓ VERIFIED | Строки 1278-1291: `id`, `action`, `createdAtUtc`, `actorName`, `notes` |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|----|--------|---------|
| `RequestFormModal` | `requests_create` | `requests.create({ dto: payload })` | ✓ WIRED | api.ts строка 23 |
| `RequestDetail` | история заявки | `requests.getHistory(id)` → `requests_get_history` | ✓ WIRED | RequestDetail строки 33-92; backend строки 65-69 |
| `RequestsSearchAndTabs` | счётчики | `requests.statusCounts()` → `requests_counts` | ✓ WIRED | api.ts строка 30 |
| `RequestFormModal (замена)` | список принтеров | `devices.list({ type_id: 2 })` | ✓ WIRED | RequestFormModal строки 73-77 |
| `DiscoveryModal admit` | создание принтера | `printers.admit()` → `build_printers_admit` → device+printer create | ✓ WIRED | Полная цепочка; http handler + tauri cmd |
| `PrintersPage` | `PrinterCreateModal` | кнопка «Завести принтер» | ✓ WIRED | PrintersPage строки 126, 182-186 |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `RequestDetail.svelte` | `historyEntries` | `requests.getHistory(id)` → SQL `audit_log JOIN users` | Да — `test_request_get_history_*` зелёные | ✓ FLOWING |
| `RequestFormModal.svelte` | `availablePrinters` | `devices.list({ type_id: 2 })` → `device_types` filter | Да — devices infra протестирована в Phase 2 | ✓ FLOWING |
| `PrintersPage.svelte` | список принтеров после admit | `printers_admit` → `Vec<PrinterDto>` + `onSuccess → refresh()` | Да — impl читает из DB (не mock-static) | ✓ FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| cargo check workspace | `cargo check --workspace` | Finished 0 errors | ✓ PASS |
| trackly-infra tests (incl. request history) | `cargo test -p trackly-infra` | 52 passed, 0 failed | ✓ PASS |
| request history specific tests | `cargo test -p trackly-infra -- test_request` | 5 passed (create, lifecycle, wrong_transition, get_history×2) | ✓ PASS |
| trackly-app tests | `cargo test -p trackly-app` | 1 passed, 0 failed | ✓ PASS |
| export_bindings | `cargo test -p trackly-app --test export_bindings` | 1 passed | ✓ PASS |

### Requirements Coverage

| Requirement | Description | Status | Evidence |
|-------------|-------------|--------|---------|
| PRN-01 | Discovery + admit принтеров | ✓ SATISFIED | `build_printers_admit` полностью реализован; HTTP + Tauri |
| PRN-04 | Ручное заведение принтера (USB/без SNMP) | ✓ SATISFIED | `PrinterCreateModal.svelte`; IP опционален |
| REQ-01 | Создание заявки (свободная форма) | ✓ SATISFIED (code) | arg-key `dto` исправлен; runtime UAT pending |
| REQ-02 | Создание заявки (замена картриджа) с принтером | ✓ SATISFIED (code) | devices.list(type_id=2) в select; runtime UAT pending |
| REQ-07 | История заявок и их статусов | ✓ SATISFIED | `requests_get_history` end-to-end; `RequestHistoryEntryDto` camelCase; notes в payload; LEFT JOIN users |
| D-GAP-Printer-Add | Принтер = device(type_id=2) + SNMP опц. | ✓ SATISFIED | Реализован admit + manual form |
| D-GAP-Replace-Select | Select из devices type=Принтер | ✓ SATISFIED | `RequestFormModal` строки 73-77 |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| (none found in gap-closure files) | — | — | — | — |

Проверены все 8 файлов gap-closure: нет TBD/FIXME/XXX/placeholder/`return null`/`Vec::new()` (заглушка убрана).

### Human Verification Required

Автоматические проверки (code, wiring, data-flow, tests) — зелёные. Требуется финальный runtime UAT в `cargo tauri dev` (TRACKLY_SNMP_MOCK=1):

#### 1. Создание заявки — полный поток

**Test:** TRACKLY_SNMP_MOCK=1 cargo tauri dev → раздел «Заявки» → «Создать заявку» → «Свободная форма» → заполнить поле → отправить.
**Expected:** Тост «Заявка создана», заявка появляется в списке со статусом «Создана».
**Why human:** arg-key fix верифицирован в коде, но полный POST-to-display поток требует runtime проверки.

#### 2. История заявки (NaN-fix)

**Test:** Открыть созданную заявку → раздел «История».
**Expected:** Блок показывает строку с реальной датой (не `NaN.NaN.NaN NaN:NaN`), именем пользователя.
**Why human:** commit 8654f89 исправил NaN-баг, нужна финальная runtime проверка поста.

#### 3. Жизненный цикл заявки и notes в истории

**Test:** Специалист (admin/manager) принимает заявку → отклоняет с причиной.
**Expected:** История показывает строку reject с причиной в поле notes.
**Why human:** `payload_json` → `notes` маппинг проверен тестом, но UI-рендер notes требует ручной проверки.

#### 4. Ручное заведение принтера (PRN-04)

**Test:** Раздел «Принтеры» → кнопка «Завести принтер» → ввести Наименование → Submit.
**Expected:** Принтер создаётся без IP (USB-режим), появляется в списке.
**Why human:** PrinterCreateModal — новый компонент 06-08, не прошёл runtime UAT.

#### 5. Discovery admit (PRN-01)

**Test:** TRACKLY_SNMP_MOCK=1 → DiscoveryModal → запустить discovery → выбрать mock-принтер → «Завести выбранные».
**Expected:** Toast с количеством заведённых принтеров > 0; они появляются в списке принтеров.
**Why human:** `printers_admit` реализован (не заглушка), но runtime admit+refresh поток не верифицирован.

#### 6. Select принтера в замене картриджа (D-GAP-Replace-Select)

**Test:** «Создать заявку» → «Замена картриджа» → поле «Принтер».
**Expected:** Dropdown содержит все устройства type=Принтер (включая тех, кто без IP), а не только SNMP-принтеры.
**Why human:** Переключение источника верифицировано кодом; runtime проверка что фильтр type_id=2 возвращает ожидаемый набор.

### Gaps Summary

**Все 5 критических gaps из предыдущей верификации закрыты.**

Оставшийся PARTIAL truth 6 (портал для роли «Сотрудник») не является gap — вход employee через браузер явно отложен до Phase 8 (AD), зафиксировано в Design Decisions 06-VERIFICATION.md.

Единственное, что блокирует `status: passed` — 6 пунктов runtime UAT, перечисленных выше. Все автоматически верифицируемые критерии прошли.

---

_Verified: 2026-06-15T08:30:00Z_
_Verifier: Claude (gsd-verifier) — re-verification after gap-closure 06-07 + 06-08_
