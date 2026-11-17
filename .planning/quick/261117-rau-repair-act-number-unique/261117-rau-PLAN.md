---
phase: quick-261117-rau
plan: 01
type: quick
files_modified:
  - migrations/V052__repair_act_number_unique.sql
  - crates/trackly-infra/tests/repair_act_number_unique_migration.rs
  - crates/trackly-infra/tests/suites/infra.rs
  - crates/trackly-infra/tests/repairs_migration.rs
  - crates/trackly-infra/tests/repairs_repo.rs
  - crates/trackly-app/tests/report_repairs.rs
---

# Quick 261117-rau: номер акта ремонта уникален на уровне БД (долг W-1 фазы 41.4)

**Цель:** `repair_visits.act_number` защищён UNIQUE-индексом, а не только очередью записи.

## Задача 1 — миграция V052
- `act_number` в V051 `NOT NULL` → частичный индекс по NULL не нужен; пустые (после TRIM) исключены.
- Ключ — как у `normalize_number_key` (trim + регистр). SQLite `LOWER` складывает только ASCII,
  поэтому кириллица — цепочкой REPLACE, как в V044.
- Дубли в старых БД: ранний визит (MIN(id)) сохраняет номер, остальные → `ДУБЛЬ-NNNNNN (<исходный>)`
  (формат V044), строка audit_log на каждое переименование (user_id NULL, old/new номер).
- DROP INDEX + CREATE UNIQUE INDEX с прежним именем; последняя строка `PRAGMA user_version = 52;`.

## Задача 2 — тесты
- Новый `repair_act_number_unique_migration.rs` (подключён в `tests/suites/infra.rs`): дубли
  переименованы и записаны в аудит; чистая БД не тронута, индекс UNIQUE; вставка второго визита
  с тем же номером (в т.ч. другой регистр/пробелы) отвергается БД, другой номер проходит.
- Фикстуры, дававшие всем визитам один номер, получают разные; тесты V051 гонятся `run_up_to(51)`.

## Проверка
`cargo test -p trackly-infra --test infra_tests`, `cargo test -p trackly-app --test app_repairs --test app_gates --test app_reports`.
