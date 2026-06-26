---
phase: 13
slug: per-device-junction-chip-drum-state
status: verified
threats_open: 0
asvs_level: 1
created: 2026-06-26
---

# Phase 13 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| axum HTTP → cartridges_sqlite.rs | free-text `printer_name` приходит от LAN-браузера через JSON-payload, далее в SQL-фильтр и таблицу `cartridge_model_compatibility` | free-text печатное имя принтера (не секрет) |
| migration runner → существующая БД | V032 выполняет DROP TABLE на продуктивных данных — необратимая операция | пользовательские данные совместимости (V005/V029) |
| axum HTTP → printers_get_compatible_aggregates | любой аутентифицированный LAN-клиент может вызвать read-команду; авторизация блокирует Employee до похода в БД | агрегаты совместимых моделей по статусам |
| axum HTTP → printers_sqlite.rs::list() | удаление капа `.min(200)` увеличивает максимальный объём данных за один аутентифицированный запрос | список принтеров (тот же authorize-гейт) |
| axum HTTP → suggest_compat_printer | free-text prefix от LAN-браузера попадает в LIKE-запрос против `devices.name` | префикс имени принтера |
| Браузер (OperationModal/PrinterDetail) → read-команды | клиентское сопоставление `printer_name` над уже авторизованными данными — перенос UI-вычисления на клиент, реальная валидация остаётся на сервере | id устройств/картриджей из авторизованного ответа |

---

## Threat Register

| Threat ID | Category | Component | Disposition | Mitigation | Status |
|-----------|----------|-----------|-------------|------------|--------|
| T-13-01 | Tampering | cartridges_sqlite.rs `list()` / `compatible_model_aggregates` SQL | mitigate | Все user-значения через `rusqlite::params![]`, без конкатенации в текст SQL — `cartridges_sqlite.rs:1188-1196, 1212-1233, 386-398` | closed |
| T-13-02 | Repudiation/Data-loss | V032 `DROP TABLE printer_cartridge_models` | accept | Намеренное необратимое удаление (одобрено 13-SPEC R1); миграция идемпотентна и протестирована | closed |
| T-13-03 | Tampering | V032 миграция на частично-применённых БД | mitigate | rebuild-паттерн (create _new → copy → drop → rename) в одной refinery-транзакции; PRAGMA foreign_keys OFF/ON scoped к файлу — `V032__...sql:40-66` | closed |
| T-13-04 | Information Disclosure | сохранённые audit_log записи удалённых printer_compatibility действий | accept | История не удаляется retroactively; прекращается только новая запись | closed |
| T-13-05 | Tampering | suggest_compat_printer SQL | mitigate | `WHERE name LIKE ?1` через `params![pattern]`, без конкатенации — `cartridge_service.rs:829-842` | closed |
| T-13-06 | Elevation of Privilege | printers_get_compatible_aggregates (новая команда) | mitigate | `authorize(caller, &Action::ReadData)?` — ПЕРВАЯ строка `build_*`, до любого обращения к БД; оба транспорта через один helper; блокирующий RBAC-тест Case 41 (Employee→403) ПРОЙДЕН — `tauri_cmds/printers.rs:207-212`, `http/printers.rs:135-148`, `role_endpoint_matrix.rs:1380-1393` | closed |
| T-13-07 | Tampering | удаление 4 V029-команд без orphan-ссылок | mitigate | Нет живых вызовов удалённых команд в `crates/trackly-app/src/` и `ui/src/`; `cargo build --workspace` (compile-gate) подтверждает отсутствие orphan-ссылок | closed |
| T-13-08 | Denial of Service | printers_sqlite.rs::list() без капа | accept | LAN-only, session-authenticated, ограниченный парк (D-13 — явное решение пользователя); запрос параметризован `LIMIT ?2 OFFSET ?3` — `printers_sqlite.rs:304-315` | closed |
| T-13-09 | Tampering | transition_in_tx kind-aware ветка | mitigate | Ветка читает `model_kind_id` из DB-строки (`fetch_in_tx`), не из payload — `cartridges_sqlite.rs:441, 572` | closed |
| T-13-10 | Tampering | suggest_compat_printer prefix bind | mitigate | `pattern = format!("{}%", prefix)` биндится через `params![pattern]`; `%` склеивается со значением до бинда, не в текст SQL — `cartridge_service.rs:833-842` | closed |
| T-13-11 | Information Disclosure | suggest_compat_printer раскрывает devices.name ролям с ReadData | accept | Имена принтеров не секрет — те же данные уже доступны через printers_list/get | closed |
| T-13-12 | Tampering | compatibility: string[] free-text payload | accept | Свободный текст разрешён дизайном (D-04); серверный матчинг `LOWER(TRIM(...))` параметризован — нет инъекции — `cartridges_sqlite.rs:380, 1188, 1217` | closed |
| T-13-13 | Denial of Service | compatibility-массив без верхнего предела элементов | accept | UI-driven форма, не публичный API большого масштаба | closed |
| T-13-14 | Information Disclosure | DeviceFormModal с полным DeviceDto принтера | accept | Тот же модал/роли (MutateDevices), что и в разделе «Устройства» — нет повышения привилегий | closed |
| T-13-15 | Tampering | onSaved повторно вызывает devices.get без проверки конфликта версии | accept | DeviceFormModal сам обрабатывает version-conflict при сохранении; повторный get — чистое чтение | closed |
| T-13-16 | Tampering | client-side compatibleDeviceIds подделываем через devtools | accept | Подсветка — UX-хинт; реальное ограничение через installable_only + cartridges_list на сервере | closed |
| T-13-17 | Tampering | previousCartridgeStateId kind-aware default подделываем через devtools | accept | Сервер (transition_in_tx) не доверяет фронтовому state_id; UI-список — UX-улучшение — `cartridges_sqlite.rs:441-459, 571-577` | closed |

*Status: open · closed*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-13-01 | T-13-02 | Необратимый DROP TABLE V029 одобрен в 13-SPEC R1; миграция идемпотентна/протестирована | Plan author (13-01) | 2026-06-26 |
| AR-13-02 | T-13-04 | История audit_log не удаляется retroactively — Phase 13 прекращает только новую запись | Plan author (13-02) | 2026-06-26 |
| AR-13-03 | T-13-08 | Uncapped read списка принтеров: LAN-only, session-auth, ограниченный парк (D-13) | User decision D-13 | 2026-06-26 |
| AR-13-04 | T-13-11 | Имена принтеров не секрет — уже доступны через printers_list/get | Plan author (13-05) | 2026-06-26 |
| AR-13-05 | T-13-12 | Free-text совместимость по дизайну (D-04); серверный матчинг параметризован | User decision D-04 | 2026-06-26 |
| AR-13-06 | T-13-13 | UI-driven форма, не публичный API большого масштаба | Plan author (13-06) | 2026-06-26 |
| AR-13-07 | T-13-14 | Тот же DeviceFormModal/роли (MutateDevices), что и раздел «Устройства» | Plan author (13-07) | 2026-06-26 |
| AR-13-08 | T-13-15 | DeviceFormModal обрабатывает version-conflict; повторный get — чистое чтение | Plan author (13-07) | 2026-06-26 |
| AR-13-09 | T-13-16 | Клиентская подсветка — UX-хинт; реальная валидация install на сервере | Plan author (13-08) | 2026-06-26 |
| AR-13-10 | T-13-17 | Сервер не доверяет фронтовому state_id; UI kind-aware список — UX-улучшение | Plan author (13-08) | 2026-06-26 |

*Accepted risks do not resurface in future audit runs.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-06-26 | 17 | 17 | 0 | gsd-security-auditor |

**Audit note:** Register authored at plan time (8 plans, each with `<threat_model>`). Verify-mitigations mode — 7 `mitigate` threats verified present in implementation with file:line evidence; 10 `accept` threats confirmed as documented accepted risk and code-sanity-checked (rationale not contradicted). Code review BLOCKER CR-01 (V032 empty-`printer_name` data corruption) and RBAC/SQL-scoping warnings were fixed before this audit. No `OPEN_THREATS`, no `ESCALATE`. Informational: T-13-01's cited test `params_are_parameterized_not_concatenated` exercises `search()` rather than `list()`/`compatible_model_aggregates`, but the parameterization in those two methods was verified directly by reading the SQL.

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-06-26
