---
phase: 06-snmp
verified: 2026-06-15T00:30:00Z
status: gaps_found
score: 4/6 must-haves verified (code+automated); human UAT failed
source: human verification (06-06 checkpoint)
---

# Phase 6: Принтеры (SNMP-мониторинг) и Заявки — Verification Report

**Phase Goal:** Включить SNMP-мониторинг сетевых принтеров (Pantum/Kyocera/HP/Canon), discovery подсети, детекцию Pantum-зависания (alert-only), плюс портал заявок для сотрудников с двумя типами и жизненным циклом.

**Verified:** 2026-06-15
**Status:** gaps_found
**Source:** Ручная верификация на checkpoint плана 06-06 (cargo tauri dev). Автоматические проверки (cargo check/test workspace, svelte-check, export_bindings) — зелёные; но runtime/UX/wiring дефекты ими не ловятся.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Миграции V020-V025, snmp2, core/infra SNMP-слой | ✓ VERIFIED | Применяются на dev DB (user_version=25), mock/real клиенты |
| 2 | Сервисы + репозитории + транспорт (Tauri/HTTP/WS) компилируются и протестированы | ✓ VERIFIED | cargo test workspace зелёный; WS 401 gate + mock switch |
| 3 | Discovery находит и **заводит** принтеры | ✗ FAILED | `printers_admit` — заглушка: `let results = Vec::new()`, `device_id: 0`; всегда возвращает пусто |
| 4 | Сотрудник создаёт заявку (свободная форма / замена картриджа) | ✗ FAILED | `requests_create` ждёт arg `dto`, фронт шлёт `payload` → invoke падает |
| 5 | UI принтеров отображает мониторинг (toner/alert/картридж) | ✓ VERIFIED (mock) | TonerGauge/AlertBanner/PrinterDetail рендерятся (требует TRACKLY_SNMP_MOCK=1) |
| 6 | Портал заявок работает для роли «Сотрудник» | ? PARTIAL | Sidebar отдаёт /requests всем ролям; но create сломан, employee-вход откладывается до AD (Phase 8) |

**Score:** 4/6 (truths 3 и 4 — провалены; 6 — частично)

### Key Link Verification (wiring gaps)

| From | To | Via | Status | Details |
|------|----|----|--------|---------|
| RequestFormModal | requests_create | `requests.create({payload})` | ✗ NOT WIRED | команда ждёт `dto`, не `payload` (api.ts + http handler) |
| RequestDetail | история заявки | `requests_get_history` | ✗ NOT WIRED | такой команды нет в бэкенде вообще |
| RequestsSearchAndTabs | счётчики | `requests_status_counts` | ✗ NOT WIRED | бэкенд-команда называется `requests_counts` |
| RequestFormModal (замена) | список принтеров | `printers.list` | ⚠ WRONG SOURCE | спека §427: должно быть devices type=Принтер, не SNMP-таблица |
| DiscoveryModal admit | создание принтера | `printers_admit` | ✗ STUB | возвращает пустой Vec, device_id=0 |

## Design Decisions (зафиксированы пользователем 2026-06-15)

- **D-GAP-Printer-Add:** Принтер = устройство (type=Принтер) + опциональная SNMP-строка. Завести можно **вручную** (форма «Завести принтер» из устройства type=Принтер, IP/SNMP опционально) **и** через discovery. Discovery/admit починить. Покрывает USB-принтеры (PRN-04).
- **D-GAP-Replace-Select:** Select принтера в форме «замена картриджа» берёт **устройства type=Принтер** (все, включая USB/без SNMP), не printers-таблицу. (спека §427)
- **D-GAP-Employee-Access:** Полноценный вход сотрудника откладывается до AD (Phase 8). Сейчас — только гарантировать корректный ролевой рендер (сотрудник видит «Создать заявку» + свои заявки read-only). Тестовый employee-логин не строим.

## Gaps Summary

### Critical Gaps (Block Goal)

1. **Создание заявки сломано (arg key mismatch)**
   - Missing: `requests_create` принимает `dto`, фронт `requests/api.ts` и http handler оперируют `payload`
   - Impact: ни свободная форма, ни замена картриджа не создаются → core value фазы (портал заявок) не работает
   - Fix: согласовать имя аргумента (`dto`) на всех транспортах + регресс-тест

2. **Рассинхрон имён команд заявок**
   - Missing: `requests_status_counts` (бэкенд `requests_counts`), `requests_get_history` (команды нет)
   - Impact: счётчики статусов и история заявки не работают
   - Fix: переименовать вызовы в api.ts + реализовать `requests_get_history` (или убрать историю из scope, но REQ-07 требует историю)

3. **Discovery admit — заглушка**
   - Missing: `build_printers_admit` возвращает `Vec::new()`, `device_id: 0`
   - Impact: discovery не заводит принтеры → PRN-01 не выполняется end-to-end
   - Fix: реализовать admit (создать device type=Принтер + printers-строку) per D-GAP-Printer-Add

4. **Нет ручного заведения принтера**
   - Missing: UI/команда «Завести принтер» из устройства type=Принтер (SNMP опц.)
   - Impact: USB/не-SNMP принтеры (PRN-04) завести нельзя
   - Fix: форма + wire `printers_create` (create_from_device) per D-GAP-Printer-Add

5. **Select принтеров в форме замены — неверный источник**
   - Missing: грузит `printers.list` вместо devices type=Принтер
   - Impact: USB-принтеры/устройства без SNMP не выбрать; противоречит §427
   - Fix: грузить устройства type=Принтер (devices_list/search с фильтром type)

### Non-Critical Gaps (Can Defer / Polish)

1. **a11y: `<nav>` с role="tablist"** в RequestsSearchAndTabs / PrintersSearchAndTabs — svelte warning; заменить элемент или роль.
2. **Ролевой рендер портала** — проверить, что сотрудник видит только create + свои заявки read-only (enforcement на сервисе уже есть; UI-проверка).

## Out of Scope / Not a bug
- TLS `CertificateUnknown` в логах — self-signed cert, браузер требует принять (ожидаемо, не дефект фазы).
- Пустой список принтеров при запуске без `TRACKLY_SNMP_MOCK=1` — реальный SNMP-режим, в dev-сети принтеров нет. Для проверки запускать `TRACKLY_SNMP_MOCK=1 cargo tauri dev`.

## Recommended Fix Plans

Запустить `/gsd-plan-phase 6 --gaps` — gap-планировщик прочитает этот отчёт и решения, создаст план(ы) с `gap_closure: true`:
- **Gap-план A (заявки):** arg `dto` parity + переименование команд + `requests_get_history` (REQ-07) + ролевой рендер + a11y.
- **Gap-план B (принтеры):** реализовать admit + ручное «Завести принтер» (create_from_device) + Select из devices type=Принтер.

Затем `/gsd-execute-phase 6 --gaps-only`.

## Note on tracking inconsistency
ROADMAP/STATE ошибочно отметили Phase 6 как Complete: executor плана 06-06 закоммитил SUMMARY + обновил ROADMAP **до** возврата human-verify checkpoint, который фактически НЕ пройден. Статус скорректирован на gap-closure.
