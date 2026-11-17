---
phase: quick-261117-rau
plan: 01
status: complete
key-files:
  created:
    - migrations/V052__repair_act_number_unique.sql
    - crates/trackly-infra/build.rs
    - crates/trackly-infra/tests/repair_act_number_unique_migration.rs
  modified:
    - crates/trackly-infra/tests/suites/infra.rs
    - crates/trackly-infra/tests/repairs_migration.rs
    - crates/trackly-infra/tests/repairs_repo.rs
    - crates/trackly-app/tests/report_repairs.rs
completed: 2026-11-17
---

# Quick 261117-rau: номер акта ремонта уникален на уровне БД (W-1 фазы 41.4)

## Что сделано
- **V052**: `idx_repair_visits_act_number` пересоздан как UNIQUE. Ключ — trim + регистр с кириллицей
  (цепочка REPLACE как в V044; SQLite `LOWER` складывает только ASCII), совпадает с
  `normalize_number_key`. `act_number` NOT NULL, поэтому частичный индекс по NULL не нужен;
  пустые номера исключены (`WHERE TRIM(act_number) <> ''`). Последняя строка `PRAGMA user_version = 52;`.
- **Дубли в старых БД**: ранний визит группы (MIN(id)) сохраняет номер, остальные переименовываются в
  `ДУБЛЬ-NNNNNN (<исходный>)` (формат V044), на каждое — строка audit_log
  (`entity_type='repair_visit'`, `action='custom:repair_act_number_dedup'`, user_id NULL,
  payload: request_id, old_number, new_number). Миграция не падает, приложение запускается.
- **Тесты** (`repair_act_number_unique_migration.rs`, подключён в suite infra): переименование дублей
  и аудит; чистая БД не тронута, индекс UNIQUE; второй визит с тем же номером (в т.ч. другой регистр
  и пробелы) отвергается БД с ошибкой UNIQUE, другой номер проходит.
- Фикстуры, дававшие всем визитам один номер (`repairs_repo.rs`, `report_repairs.rs`), получили разные;
  тесты V051 (`repairs_migration.rs`) гонятся `run_up_to(51)`.
- **Попутно — `crates/trackly-infra/build.rs`**: `embed_migrations!` не сообщает cargo о каталоге
  миграций, новый `.sql` не вызывал пересборку. Первый сплошной прогон упал: trackly-infra в нём был
  собран без V052 (`user_version` 51). Теперь `rerun-if-changed=../../migrations`; проверено:
  без изменений пересборки нет, правка миграции её вызывает.

## Проверка
- `infra_tests` 247/0; `app_repairs` 121/0, `app_gates` 224/0 (suite_wiring), `app_reports` 173/0.
- Сплошной `cargo test --workspace` после build.rs: 2602 прошло, 0 упало.
- clippy `-D warnings` (infra, app tests), `cargo fmt --check` — чисто.
