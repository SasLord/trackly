---
phase: 41
slug: groups-model-and-editor
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-10-04
---

# Phase 41 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Источник: `41-RESEARCH.md` § Validation Architecture. Карта по задачам заполняется
> планировщиком/исполнителем, когда появятся ID задач (см. ниже).

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust: `cargo test` (libtest). UI: `svelte-check`, `eslint`+`prettier`, zero-dependency `node ui/scripts/check-*.mjs` в цепочке `pnpm lint`. **JS-тест-раннера в проекте нет — не вводить.** |
| **Config file** | `/Cargo.toml` (workspace), `ui/package.json` (`lint`), `clippy.toml` |
| **Quick run command** | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test <файл>` · `cargo test -p trackly-infra --test <файл>` · `cargo test -p trackly-core <фильтр>` · `node ui/scripts/check-<gate>.mjs` |
| **Full suite command** | `cargo test --workspace --no-fail-fast -- --test-threads=1` + `pnpm --dir ui svelte-check && pnpm --dir ui lint` + `cargo clippy --workspace --all-targets -- -D warnings` |
| **Estimated runtime** | целевой прогон ~10–60 с; полный workspace ≈80 мин (~1372 теста) |

### Ограничения запуска (обязательны к соблюдению)

1. **Один `cargo test` одновременно.**
2. Полный прогон — **один раз на границе волны, в фоне**; остальное по целям `--test`.
3. Тестам `trackly-app` нужны `TRACKLY_AD_MOCK` / `TRACKLY_SNMP_MOCK` и **настоящий** `ui/dist` (`pnpm --dir ui build`).
4. При запуске пакета `trackly-app` целиком — `-- --skip login_remember_persistent_cookie` (заранее известный зависающий тест).
5. `cargo fmt --check` имеет существующий дрейф — проверять только изменённые файлы.
6. При exit 69 на линковке — префикс `DEVELOPER_DIR=/Library/Developer/CommandLineTools`.
7. Перед `svelte-check` / `lint` сгенерировать биндинги: `cargo test -p trackly-app --test export_bindings`.
8. CI — один последовательный job: первый красный шаг скипает следующие гейты, поэтому прогонять шаги **в порядке CI**.
9. Перед коммитом — `node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256`.

---

## Sampling Rate

- **After every task commit:** целевой `--test` изменённого слоя + `node`-гейт, если трогался UI.
- **After every plan wave:** все цели `groups_*` / `group_*` + `role_endpoint_matrix` + `place_movements_*` + `per_record_invariants` + `acts_*` (затронуты write-site'ы) + `places_*` + `devices_crud` + `svelte-check` + `pnpm lint`.
  **Прогонять цели целиком, а не только добавленные файлы** — «зелено по своим файлам при красном пакете» уже случалось.
- **Before `/gsd-verify-work`:** полный `cargo test --workspace` (один раз, в фоне) + clippy `-D warnings` + `svelte-check` + `pnpm lint` + privacy-гейт.
- **Max feedback latency:** ≤60 с на целевом прогоне.

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 41-01-T1 | 01 | 1 | GRP-01, GRP-05, GRP-06 | T-41-01-01..03 | аддитивный DDL, триггер code/behavior, PK членства | migration | `cargo test -p trackly-infra --test migration_idempotency` | ❌ создаётся задачей | ⬜ pending |
| 41-01-T2 | 01 | 1 | GRP-01, GRP-05, GRP-10 | T-41-01-02 | run_up_to(44)->run без потери act_items; каскады/FK | infra | `cargo test -p trackly-infra --test groups_migration && cargo test -p trackly-infra --test per_record_invariants` | ❌ создаётся задачей | ⬜ pending |
| 41-02-T1 | 02 | 1 | GRP-03 | T-41-02-02 | нормализация ip/mac/число в чистом домене | core unit | `cargo test -p trackly-core group_values` | ❌ создаётся задачей | ⬜ pending |
| 41-02-T2 | 02 | 1 | GRP-09 | T-41-02-01 | Action::ManageGroupTypes admin-only; MutateGroups/ReadGroups admin+manager | core unit | `cargo test -p trackly-core auth && cargo check --workspace` | ❌ создаётся задачей | ⬜ pending |
| 41-03-T1 | 03 | 1 | GRD-06 | T-41-03-01 | нет пользовательских строк «группа» для свёртки | UI/lint | `pnpm --dir ui svelte-check` | ❌ создаётся задачей | ⬜ pending |
| 41-03-T2 | 03 | 1 | GRD-06 | T-41-03-01 | словарный гейт с --selftest | node-гейт | `node ui/scripts/check-group-vocabulary.mjs --selftest && node ui/scripts/check-group-vocabulary.mjs` | ❌ создаётся задачей | ⬜ pending |
| 41-04-T1 | 04 | 2 | GRP-01 | T-41-04-02..03 | триггер -> Conflict; засев ON CONFLICT DO NOTHING | infra | `cargo test -p trackly-infra --test group_types_repo` | ❌ создаётся задачей | ⬜ pending |
| 41-04-T2 | 04 | 2 | GRP-02, GRP-03 | T-41-04-04 | свойства, скрытие, reorder атомарно, нарушители | infra | `cargo test -p trackly-infra --test group_types_repo` | ❌ создаётся задачей | ⬜ pending |
| 41-05-T1 | 05 | 2 | GRP-06 | T-41-05-02 | batch_id/entity_label, сигнатура record_movement_if_applicable не тронута | infra | `cargo test -p trackly-infra --test place_movements_batch_repo && cargo test -p trackly-infra place_movements` | ❌ создаётся задачей | ⬜ pending |
| 41-05-T2 | 05 | 2 | GRP-06 | T-41-05-01 | DTO таймлайна: group_id/group_label; ReadPlaces гейт | app | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test place_movements_group_fields` | ❌ создаётся задачей | ⬜ pending |
| 41-06-T1 | 06 | 3 | GRP-04, GRP-05 | T-41-06-01..03 | seq из колонки, PK членства, CTE цикла, teardown | infra | `cargo test -p trackly-infra --test groups_repo` | ❌ создаётся задачей | ⬜ pending |
| 41-06-T2 | 06 | 3 | GRP-08 | T-41-06-04 | значения, USB-принтеры, счётчики дерева | infra | `cargo test -p trackly-infra --test groups_repo` | ❌ создаётся задачей | ⬜ pending |
| 41-07-T1 | 07 | 3 | GRP-01, GRP-09 | T-41-07-01..03 | засев при старте, code/behavior неизменяемы (невакуумно), права | app service | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test groups_types_service` | ❌ создаётся задачей | ⬜ pending |
| 41-07-T2 | 07 | 3 | GRP-02, GRP-03 | T-41-07-04 | скрытие заполненного, запрет смены data_type, нарушители is_required | app service | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test groups_types_service` | ❌ создаётся задачей | ⬜ pending |
| 41-08-T1 | 08 | 4 | GRP-04, GRP-08 | T-41-08-01 | чтения групп, состав, поиск (кириллица), членство пачкой | app service | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test groups_service` | ❌ создаётся задачей | ⬜ pending |
| 41-08-T2 | 08 | 4 | GRP-04, GRP-10 | T-41-08-02 | нумерация seq, одноимённые группы, delete без смены места | app service | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test groups_service` | ❌ создаётся задачей | ⬜ pending |
| 41-09-T1 | 09 | 4 | GRP-09 | T-41-09-01 | 10 пар команда+роут типов, specta | build | `cargo test -p trackly-app --test export_bindings && cargo check -p trackly-app` | ❌ создаётся задачей | ⬜ pending |
| 41-09-T2 | 09 | 4 | GRP-01, GRP-09 | T-41-09-02..04 | матрица 3 роли x 2 транспорта; неизменяемость по HTTP; полнота маршрутов | HTTP+tauri-path | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test role_endpoint_matrix group_types` | ❌ создаётся задачей | ⬜ pending |
| 41-10-T1 | 10 | 5 | GRP-06 | T-41-10-05 | общий ru_plural, batch-каскад картриджей | app | `cargo test -p trackly-app --lib plural` | ❌ создаётся задачей | ⬜ pending |
| 41-10-T2 | 10 | 5 | GRP-06, GRP-07 | T-41-10-01..03 | перенос атомарен (fault-injection), вложенная read-only | app service | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test groups_move` | ❌ создаётся задачей | ⬜ pending |
| 41-10-T3 | 10 | 5 | GRP-06 | T-41-10-05 | 7 строк с одним batch_id, D-30 только audit_log, картриджи | app | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test group_movements_journal` | ❌ создаётся задачей | ⬜ pending |
| 41-11-T1 | 11 | 6 | GRP-05 | T-41-11-01,04,05 | add/remove состава атомарно, D-21, release-примитив | app service | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test groups_membership` | ❌ создаётся задачей | ⬜ pending |
| 41-11-T2 | 11 | 6 | GRP-05, GRP-06 | T-41-11-02,03,06 | вложенность: цикл, teardown, производное место, инвариант-обход | app service | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test groups_membership` | ❌ создаётся задачей | ⬜ pending |
| 41-12-T1 | 12 | 7 | GRP-03 | T-41-12-01,02,06 | серверная нормализация, живость users/devices, обязательность, CAS | app service | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test groups_values_card values_` | ❌ создаётся задачей | ⬜ pending |
| 41-12-T2 | 12 | 7 | GRP-08, GRP-09 | T-41-12-03 | user_options (id/full_name/login), карточка, дедуп принтеров | app service | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test groups_values_card` | ❌ создаётся задачей | ⬜ pending |
| 41-13-T1 | 13 | 8 | GRP-09 | T-41-13-01 | 15 пар команда+роут групп, specta | build | `cargo test -p trackly-app --test export_bindings && cargo check -p trackly-app` | ❌ создаётся задачей | ⬜ pending |
| 41-13-T2 | 13 | 8 | GRP-09, GRP-05, GRP-03 | T-41-13-01..04,06 | матрица прав групп x 2 транспорта; валидация по HTTP; полнота маршрутов | HTTP+tauri-path | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test role_endpoint_matrix groups_` | ❌ создаётся задачей | ⬜ pending |
| 41-14-T1 | 14 | 8 | GRP-07 | T-41-14-01..03 | guard S1 (реальная смена vs повтор), release S8, S9 | app service | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test group_write_sites` | ❌ создаётся задачей | ⬜ pending |
| 41-14-T2 | 14 | 8 | GRP-07 | T-41-14-01 | devices_update по HTTP и Tauri-пути | HTTP+tauri-path | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test group_write_sites` | ❌ создаётся задачей | ⬜ pending |
| 41-15-T1 | 15 | 8 | GRP-10 | T-41-15-04 | referencing_group_count, русское сообщение блокировки удаления места | app | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test places_delete_blocked` | ❌ создаётся задачей | ⬜ pending |
| 41-15-T2 | 15 | 8 | GRP-07, GRP-06 | T-41-15-01..03 | массовый перенос двигает группу целиком без дублей журнала | app service | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test places_move_groups` | ❌ создаётся задачей | ⬜ pending |
| 41-16-T1 | 16 | 9 | GRP-07 | T-41-16-01,03 | release на 8 write-site'ах актов | app | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test acts_crud` | ❌ создаётся задачей | ⬜ pending |
| 41-16-T2 | 16 | 9 | GRP-07 | T-41-16-01,03,04 | сценарии S3-S7 (фикстура: место члена != место акта) | app service | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test group_write_sites` | ❌ создаётся задачей | ⬜ pending |
| 41-16-T3 | 16 | 9 | GRP-07 | T-41-16-02 | счётный гейт исходников write-site'ов | gate | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test group_write_sites gate` | ❌ создаётся задачей | ⬜ pending |
| 41-17-T1 | 17 | 8 | GRP-06 | T-41-17-03 | ReportRow batch_*, тип «Группа», причина пакета | app | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test group_report_batch` | ❌ создаётся задачей | ⬜ pending |
| 41-17-T2 | 17 | 8 | GRP-06 | T-41-17-02 | печать/CSV — полный состав, шаблон не тронут | app | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test group_report_batch` | ❌ создаётся задачей | ⬜ pending |
| 41-18-T1 | 18 | 9 | GRP-04 | T-41-18-02 | обёртка 25 команд против bindings.ts | UI/types | `pnpm --dir ui svelte-check` | ❌ создаётся задачей | ⬜ pending |
| 41-18-T2 | 18 | 9 | GRP-02 | T-41-18-03 | чистые функции порядка, golden-фикстура + selftest | node-гейт | `node ui/scripts/check-reorder.mjs --selftest && node ui/scripts/check-reorder.mjs` | ❌ создаётся задачей | ⬜ pending |
| 41-19-T1 | 19 | 10 | GRP-04, GRP-01 | T-41-19-04 | дерево типов и групп (ARIA, клавиатура, untrack) | UI/lint + human | `pnpm --dir ui svelte-check && pnpm --dir ui lint` | ❌ создаётся задачей | ⬜ pending |
| 41-19-T2 | 19 | 10 | GRP-01, GRP-04, GRP-10 | T-41-19-02 | модалки типа/группы/удаления без нативного select | UI/lint + human | `pnpm --dir ui svelte-check && pnpm --dir ui lint` | ❌ создаётся задачей | ⬜ pending |
| 41-20-T1 | 20 | 10 | GRP-02, GRP-03 | T-41-20-02 | таблица свойств, скрытие, попап нарушителей, disabled data_type | UI/lint + human | `pnpm --dir ui svelte-check && pnpm --dir ui lint` | ❌ создаётся задачей | ⬜ pending |
| 41-20-T2 | 20 | 10 | GRP-02 | T-41-20-03 | перестановка на pointer-events с откатом | UI/lint + human (оба транспорта) | `pnpm --dir ui svelte-check && pnpm --dir ui lint && node ui/scripts/check-reorder.mjs` | ❌ создаётся задачей | ⬜ pending |
| 41-20-T3 | 20 | 10 | GRP-01, GRP-09 | T-41-20-01 | панель типа, manager только чтение | UI/lint + human | `pnpm --dir ui svelte-check && pnpm --dir ui lint` | ❌ создаётся задачей | ⬜ pending |
| 41-21-T1 | 21 | 10 | GRP-05 | T-41-21-06 | Dropdown getGroupSection аддитивно | UI/lint + human | `pnpm --dir ui svelte-check && pnpm --dir ui lint` | ❌ создаётся задачей | ⬜ pending |
| 41-21-T2 | 21 | 10 | GRP-05, GRP-06 | T-41-21-01,03 | таблица состава, строка-поиск (устройства+группы), вложенные строки | UI/lint + human | `pnpm --dir ui svelte-check && pnpm --dir ui lint` | ❌ создаётся задачей | ⬜ pending |
| 41-21-T3 | 21 | 10 | GRP-06 | T-41-21-03 | мультивыбор + реестр INV-7 (маркеры групп) | gate + human | `node ui/scripts/check-place-tree-invalidation.mjs` | ❌ создаётся задачей | ⬜ pending |
| 41-22-T1 | 22 | 10 | GRP-08 | T-41-22-02 | чипсы пользователей (userOptions, основной) | UI/lint + human | `pnpm --dir ui svelte-check && pnpm --dir ui lint` | ❌ создаётся задачей | ⬜ pending |
| 41-22-T2 | 22 | 10 | GRP-08 | T-41-22-02 | единый список принтеров USB+ссылки | UI/lint + human | `pnpm --dir ui svelte-check && pnpm --dir ui lint` | ❌ создаётся задачей | ⬜ pending |
| 41-22-T3 | 22 | 10 | GRP-03, GRP-08 | T-41-22-01 | форма свойств: серверные ошибки по полю, нормализованные значения | UI/lint + human | `pnpm --dir ui svelte-check && pnpm --dir ui lint` | ❌ создаётся задачей | ⬜ pending |
| 41-23-T1 | 23 | 11 | GRP-06 | T-41-23-03,04 | GroupMoveModal (одно действие из двух точек), панель группы, история на MovementTimeline | UI/lint+gate | `pnpm --dir ui lint && node ui/scripts/check-place-tree-invalidation.mjs` | ❌ создаётся задачей | ⬜ pending |
| 41-23-T2 | 23 | 11 | GRP-04, GRP-09 | T-41-23-01,02 | страница раздела, маршрут, сайдбар, роли | UI build + human | `pnpm --dir ui svelte-check && pnpm --dir ui lint && pnpm --dir ui build` | ❌ создаётся задачей | ⬜ pending |
| 41-23-T3 | 23 | 11 | GRP-04, GRP-09 | T-41-23-01 | гейт сайдбара и маршрутов раздела | node-гейт | `node ui/scripts/check-groups-section.mjs --selftest && node ui/scripts/check-groups-section.mjs` | ❌ создаётся задачей | ⬜ pending |
| 41-24-T1 | 24 | 11 | GRP-07 | T-41-24-01 | PlacePicker disabled для члена группы с местом | UI/lint + human | `pnpm --dir ui svelte-check && pnpm --dir ui lint` | ❌ создаётся задачей | ⬜ pending |
| 41-24-T2 | 24 | 11 | GRP-06 | T-41-24-02 | таймлайн «в составе группы» + allowlist словарного гейта | UI/lint + human | `pnpm --dir ui lint && node ui/scripts/check-group-vocabulary.mjs --selftest` | ❌ создаётся задачей | ⬜ pending |
| 41-25-T1 | 25 | 12 | GRP-06 | T-41-25-01,03 | свёртка пакета в отчёте, печать без правил свёртки | UI/lint + human (печать) | `pnpm --dir ui svelte-check && pnpm --dir ui lint` | ❌ создаётся задачей | ⬜ pending |
| 41-25-T2 | 25 | 12 | GRP-06, GRP-07 | T-41-25-04 | модалка и тост массового переноса с группами | UI/lint + human | `pnpm --dir ui lint && node ui/scripts/check-place-tree-invalidation.mjs` | ❌ создаётся задачей | ⬜ pending |
| 41-26-T1 | 26 | 13 | GRP-01..GRP-10 | T-41-26-02 | полный regress workspace один раз, clippy | full suite | `cargo clippy --workspace --all-targets -- -D warnings` | ❌ создаётся задачей | ⬜ pending |
| 41-26-T2 | 26 | 13 | GRD-06, GRP-04 | T-41-26-01 | UI-гейты в порядке CI, privacy | CI-chain | `pnpm --dir ui lint && node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256` | ❌ создаётся задачей | ⬜ pending |
| 41-26-T3 | 26 | 13 | все | T-41-26-04 | заполнение карты и сбор живых проверок | doc | `node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256` | ❌ создаётся задачей | ⬜ pending |

**Заполнено планировщиком 2026-10-04** (26 планов, 59 задач). Статусы и «File Exists» обновляет план 41-26 по факту прогонов.
Карта не остаётся пустой при закрытии фазы — это зафиксированный повторяющийся дефект прошлых фаз (40.4, 40.5).
Файлы тестов в плане именуются по факту: `group_types_repo` (типы, триггер), `groups_repo` (группы, PK, CTE), `groups_membership`, `groups_move`, `groups_values_card`, `group_report_batch`, `places_move_groups` — вместо обобщённых `groups_service move_/card_/values_` из таблицы ниже.

### Phase Requirements → Test Map (из RESEARCH.md)

| Req | Layer | Target | File |
|-----|-------|--------|------|
| 1 — `code`/`behavior` неизменяемы | infra (триггер БД) + app + dual-transport | `groups_repo`, `groups_types_service` | ❌ Wave 0 |
| 2 — идемпотентный засев | app service | `groups_types_service seed_` | ❌ Wave 0 |
| 3 — свойства 6 типов, порядок, `show_on_map` | app service + DTO round-trip | `groups_types_service properties_` | ❌ Wave 0 |
| 4 — защита заполненных свойств (3 отдельных теста) | app service + прямой `SELECT` значений | `groups_types_service protect_` | ❌ Wave 0 |
| 5 — валидация значений на сервере | core таблица-тест + оба транспорта | `trackly-core group_values`, `groups_service values_` | ❌ Wave 0 |
| 6 — раздел и права видимости | UI-гейт + серверный 403 | `check-groups-section.mjs` | ❌ Wave 0 |
| 7 — `seq` и нумерация (+ гонка) | app service + `UNIQUE(type_id,seq)` | `groups_service numbering_` | ❌ Wave 0 |
| 8 — членство, циклы, `teardown` | infra (PK, CTE) + app (сообщения) | `groups_repo`, `groups_service membership_` | ❌ Wave 0 |
| 9 — протаскивание места **атомарно** | infra+app, фикстура сбоя на 4-м устройстве | `groups_service move_` | ❌ Wave 0 |
| 10 — журнал пакета, отчёт, таймлайн, `ALTER` | app + infra-миграция `run_up_to(44)` | `group_movements_journal`, `groups_migration` | ❌ Wave 0 |
| 11 — запрет индивидуального перемещения | app таблица-драйвер S1–S9 + счётный гейт исходника | `group_write_sites` | ❌ Wave 0 |
| 12 — карточка группы, дедупликация принтеров | app service | `groups_service card_` | ❌ Wave 0 |
| 13 — матрица 3 роли × (тип, группа) × 2 транспорта | HTTP-сессии + `build_*` + тест полноты маршрутов | `role_endpoint_matrix` Cases 76+ | ❌ Wave 0 (расширение) |
| 14 — переименование свёртки | UI-гейт словаря с `--selftest` | `check-group-vocabulary.mjs` | ❌ Wave 0 |
| — `Action`-матрица | core unit | `trackly-core auth` | ✅ дописать |
| — `per_record_invariants` новых таблиц | infra | `per_record_invariants` | ✅ дописать |
| — INV-7 реестр новых producer'ов | UI-гейт | `check-place-tree-invalidation.mjs` | ✅ дописать |
| — чистые функции `reorder.ts` | node-гейт-фикстура | `check-reorder.mjs` | ❌ Wave 0 |

---

## Wave 0 Requirements

- [ ] `crates/trackly-core/src/domain/group_values.rs` — таблицы-тесты ip/mac/number/text (Req 5)
- [ ] `crates/trackly-infra/tests/groups_migration.rs` — V045/V046, `run_up_to(44)` без потери данных (Req 10)
- [ ] `crates/trackly-infra/tests/groups_repo.rs` — триггер, PK членства, CTE цикла/состава, атомарность `move_group_in_tx` (Req 1, 8, 9)
- [ ] `crates/trackly-app/tests/groups_types_service.rs` — Req 1–4 + засев
- [ ] `crates/trackly-app/tests/groups_service.rs` — Req 5, 7–9, 12
- [ ] `crates/trackly-app/tests/group_write_sites.rs` — Req 11, сценарии S1–S9, счётный гейт `act_service.rs`
- [ ] `crates/trackly-app/tests/group_movements_journal.rs` — Req 10 (журнал, отчёт, таймлайн)
- [ ] `crates/trackly-app/tests/role_endpoint_matrix.rs` — Cases 76+ (Req 13) + тест полноты маршрутов
- [ ] `ui/scripts/check-group-vocabulary.mjs` (+ `--selftest`), `check-groups-section.mjs`, `check-reorder.mjs`; подключить в цепочку `lint` в `ui/package.json`
- [ ] Расширить: `auth.rs::tests`, `per_record_invariants.rs` (списки таблиц), `check-place-tree-invalidation.mjs` (реестр INV-7)
- [ ] Framework install: **не требуется**

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Перестановка свойств перетаскиванием (жест, ghost, отмена Esc/`pointercancel`, клавиатурный путь, `aria-live`) | 3 / D-10 | Компиляционные гейты не видят жест; механизм зависит от движка webview | В Tauri (WKWebView) и в LAN-браузере после `pnpm --dir ui build`: перетащить свойство, отменить Esc, пройти клавиатурой. По возможности — Windows WebView2 |
| Асимметрия «десктоп vs браузер» по экрану «Группы» (дерево, `Dropdown` с секциями, модалки) | 6, 3, 7 | `svelte-check`/`eslint`/`build` слепы к рантайму рун; синтетический харнесс ≠ проверка | Открыть раздел на обоих транспортах, пройти CRUD типа, свойства, группы, состава |
| Отсутствие `effect_update_depth_exceeded` | 6, 11, 10 | Рантайм-ошибка рун не ловится компиляцией | Экраны «Группы», «Устройства» (форма с заблокированным `PlacePicker`), «Отчёты» (свёртка пакета) — консоль чистая |
| Визуальная печать отчёта «Перемещения» с полным составом пакета, многостраничность, отсутствие шевронов на бумаге | 10 / D-27 | Текст-экстракция не видит перекрытий и переносов страниц | Печать из десктопа и из LAN-браузера, отчёт с переносом группы из 6 устройств |
| Инвалидация счётчиков дерева «Места» после переноса группы | 9 / D-08 | INV-7 ловит структуру регистрации, не рантайм | Перенести группу, проверить счётчики на всех экранах, меняющих место |
| Вёрстка 35/65, фокус-кольцо (inset на первой ячейке), прокрутка региона | 6 | Визуальное | Сравнить с разделом «Места» |
| Тексты тостов и подсказок D-19 / D-24 | 11, 9 | Формулировки в живом приложении | «Место задаётся группой «…»», «Перенесено: группа и N устройств» |

**Процедурное требование:** `human-verify`-чекпоинты исполнители закрывают без запуска приложения
(`auto_advance` авто-одобряет). Чекпоинты **собирать и отдавать пользователю ДО верификации**, иначе они
станут пробелами. Пункт чеклиста обязан называть существующий компонент UI.

---

## Защита от ранее выявленных отказов проверки

- **Вакуумный тест:** HTTP-тест неизменяемости шлёт **отличающийся** `code` и проверяет БД; переименование —
  в имя ≠ сидового; после скрытия свойства — прямой `SELECT COUNT(*) FROM group_property_values`; для Req 11 —
  фикстура различает «повтор текущего места» и «реальную смену». Ожидаемую строку вычислять из прочитанного обратно, не хардкодить.
- **Неуникальные якоря мутаций:** якорь обязан быть специфичным для функции (`ON CONFLICT(code) DO NOTHING`,
  `if new_place != before_place_id`, `tx.commit()` внутри `move_group_in_tx`, конкретный вызов `release_device_in_tx(`).
  В скрипте мутации — `assert s.count(old) == 1` и `git diff` после патча. Если мутация не покраснила —
  сперва доказать, что она применилась.
- **Фикстура, не различающая до/после:** для D-22 (S3–S7) устройство-член должно иметь место, отличное от места акта.

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Per-Task Verification Map заполнена (не остаётся заглушкой)
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 60s
- [ ] Живые проверки собраны и отданы пользователю ДО `/gsd-verify-work`
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
