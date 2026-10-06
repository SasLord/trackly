---
phase: 41
slug: groups-model-and-editor
status: executed
nyquist_compliant: true
wave_0_complete: true
created: 2026-10-04
validated: 2026-10-06  # аудит /gsd-validate-phase 41
---

# Phase 41 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Источник: `41-RESEARCH.md` § Validation Architecture. Карта по задачам заполнена
> планировщиком; статусы выставлены планом 41-26 (см. ниже).
> Аудит `/gsd-validate-phase` от 2026-10-06 — раздел «Validation Audit 2026-10-06»
> в конце файла: там же правка WR-01, итог живой приёмки и закрытый пробел G1.

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
| 41-01-T1 | 01 | 1 | GRP-01, GRP-05, GRP-06 | T-41-01-01..03 | аддитивный DDL, триггер code/behavior, PK членства | migration | `cargo test -p trackly-infra --test migration_idempotency` | ✅ создан | ✅ green |
| 41-01-T2 | 01 | 1 | GRP-01, GRP-05, GRP-10 | T-41-01-02 | run_up_to(44)->run без потери act_items; каскады/FK | infra | `cargo test -p trackly-infra --test groups_migration && cargo test -p trackly-infra --test per_record_invariants` | ✅ создан | ✅ green |
| 41-02-T1 | 02 | 1 | GRP-03 | T-41-02-02 | нормализация ip/mac/число в чистом домене | core unit | `cargo test -p trackly-core group_values` | ✅ создан | ✅ green |
| 41-02-T2 | 02 | 1 | GRP-09 | T-41-02-01 | Action::ManageGroupTypes admin-only; MutateGroups/ReadGroups admin+manager | core unit | `cargo test -p trackly-core auth && cargo check --workspace` | ✅ создан | ✅ green |
| 41-03-T1 | 03 | 1 | GRD-06 | T-41-03-01 | нет пользовательских строк «группа» для свёртки | UI/lint | `pnpm --dir ui svelte-check` | ✅ создан | ✅ green |
| 41-03-T2 | 03 | 1 | GRD-06 | T-41-03-01 | словарный гейт с --selftest | node-гейт | `node ui/scripts/check-group-vocabulary.mjs --selftest && node ui/scripts/check-group-vocabulary.mjs` | ✅ создан | ✅ green |
| 41-04-T1 | 04 | 2 | GRP-01 | T-41-04-02..03 | триггер -> Conflict; засев ON CONFLICT DO NOTHING | infra | `cargo test -p trackly-infra --test group_types_repo` | ✅ создан | ✅ green |
| 41-04-T2 | 04 | 2 | GRP-02, GRP-03 | T-41-04-04 | свойства, скрытие, reorder атомарно, нарушители | infra | `cargo test -p trackly-infra --test group_types_repo` | ✅ создан | ✅ green |
| 41-05-T1 | 05 | 2 | GRP-06 | T-41-05-02 | batch_id/entity_label, сигнатура record_movement_if_applicable не тронута | infra | `cargo test -p trackly-infra --test place_movements_batch_repo && cargo test -p trackly-infra place_movements` | ✅ создан | ✅ green |
| 41-05-T2 | 05 | 2 | GRP-06 | T-41-05-01 | DTO таймлайна: group_id/group_label; ReadPlaces гейт | app | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test place_movements_group_fields` | ✅ создан | ✅ green |
| 41-06-T1 | 06 | 3 | GRP-04, GRP-05 | T-41-06-01..03 | seq из колонки, PK членства, CTE цикла, teardown | infra | `cargo test -p trackly-infra --test groups_repo` | ✅ создан | ✅ green |
| 41-06-T2 | 06 | 3 | GRP-08 | T-41-06-04 | значения, USB-принтеры, счётчики дерева | infra | `cargo test -p trackly-infra --test groups_repo` | ✅ создан | ✅ green |
| 41-07-T1 | 07 | 3 | GRP-01, GRP-09 | T-41-07-01..03 | засев при старте, code/behavior неизменяемы (невакуумно), права | app service | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test groups_types_service` | ✅ создан | ✅ green |
| 41-07-T2 | 07 | 3 | GRP-02, GRP-03 | T-41-07-04 | скрытие заполненного, запрет смены data_type, нарушители is_required | app service | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test groups_types_service` | ✅ создан | ✅ green |
| 41-08-T1 | 08 | 4 | GRP-04, GRP-08 | T-41-08-01 | чтения групп, состав, поиск (кириллица), членство пачкой | app service | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test groups_service` | ✅ создан | ✅ green |
| 41-08-T2 | 08 | 4 | GRP-04, GRP-10 | T-41-08-02 | нумерация seq, одноимённые группы, delete без смены места | app service | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test groups_service` | ✅ создан | ✅ green |
| 41-09-T1 | 09 | 4 | GRP-09 | T-41-09-01 | 10 пар команда+роут типов, specta | build | `cargo test -p trackly-app --test export_bindings && cargo check -p trackly-app` | ✅ создан | ✅ green |
| 41-09-T2 | 09 | 4 | GRP-01, GRP-09 | T-41-09-02..04 | матрица 3 роли x 2 транспорта; неизменяемость по HTTP; полнота маршрутов | HTTP+tauri-path | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test role_endpoint_matrix group_types` | ✅ создан | ✅ green |
| 41-10-T1 | 10 | 5 | GRP-06 | T-41-10-05 | общий ru_plural, batch-каскад картриджей | app | `cargo test -p trackly-app --lib plural` | ✅ создан | ✅ green |
| 41-10-T2 | 10 | 5 | GRP-06, GRP-07 | T-41-10-01..03 | перенос атомарен (fault-injection), вложенная read-only | app service | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test groups_move` | ✅ создан | ✅ green |
| 41-10-T3 | 10 | 5 | GRP-06 | T-41-10-05 | 7 строк с одним batch_id, D-30 только audit_log, картриджи | app | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test group_movements_journal` | ✅ создан | ✅ green |
| 41-11-T1 | 11 | 6 | GRP-05 | T-41-11-01,04,05 | add/remove состава атомарно, D-21, release-примитив | app service | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test groups_membership` | ✅ создан | ✅ green |
| 41-11-T2 | 11 | 6 | GRP-05, GRP-06 | T-41-11-02,03,06 | вложенность: цикл, teardown, производное место, инвариант-обход | app service | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test groups_membership` | ✅ создан | ✅ green |
| 41-12-T1 | 12 | 7 | GRP-03 | T-41-12-01,02,06 | серверная нормализация, живость users/devices, обязательность, CAS | app service | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test groups_values_card values_` | ✅ создан | ✅ green |
| 41-12-T2 | 12 | 7 | GRP-08, GRP-09 | T-41-12-03 | user_options (id/full_name/login), карточка, дедуп принтеров | app service | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test groups_values_card` | ✅ создан | ✅ green |
| 41-13-T1 | 13 | 8 | GRP-09 | T-41-13-01 | 15 пар команда+роут групп, specta | build | `cargo test -p trackly-app --test export_bindings && cargo check -p trackly-app` | ✅ создан | ✅ green |
| 41-13-T2 | 13 | 8 | GRP-09, GRP-05, GRP-03 | T-41-13-01..04,06 | матрица прав групп x 2 транспорта; валидация по HTTP; полнота маршрутов | HTTP+tauri-path | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test role_endpoint_matrix groups_` | ✅ создан | ✅ green |
| 41-14-T1 | 14 | 8 | GRP-07 | T-41-14-01..03 | guard S1 (реальная смена vs повтор), release S8, S9 | app service | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test group_write_sites` | ✅ создан | ✅ green |
| 41-14-T2 | 14 | 8 | GRP-07 | T-41-14-01 | devices_update по HTTP и Tauri-пути | HTTP+tauri-path | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test group_write_sites` | ✅ создан | ✅ green |
| 41-15-T1 | 15 | 8 | GRP-10 | T-41-15-04 | referencing_group_count, русское сообщение блокировки удаления места | app | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test places_delete_blocked` | ✅ создан | ✅ green |
| 41-15-T2 | 15 | 8 | GRP-07, GRP-06 | T-41-15-01..03 | массовый перенос двигает группу целиком без дублей журнала | app service | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test places_move_groups` | ✅ создан | ✅ green |
| 41-16-T1 | 16 | 9 | GRP-07 | T-41-16-01,03 | release на 8 write-site'ах актов | app | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test acts_crud` | ✅ создан | ✅ green |
| 41-16-T2 | 16 | 9 | GRP-07 | T-41-16-01,03,04 | сценарии S3-S7 (фикстура: место члена != место акта) | app service | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test group_write_sites` | ✅ создан | ✅ green |
| 41-16-T3 | 16 | 9 | GRP-07 | T-41-16-02 | счётный гейт исходников write-site'ов | gate | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test group_write_sites gate` | ✅ создан | ✅ green |
| 41-17-T1 | 17 | 8 | GRP-06 | T-41-17-03 | ReportRow batch_*, тип «Группа», причина пакета | app | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test group_report_batch` | ✅ создан | ✅ green |
| 41-17-T2 | 17 | 8 | GRP-06 | T-41-17-02 | печать/CSV — полный состав, шаблон не тронут | app | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test group_report_batch` | ✅ создан | ✅ green |
| 41-18-T1 | 18 | 9 | GRP-04 | T-41-18-02 | обёртка 25 команд против bindings.ts | UI/types | `pnpm --dir ui svelte-check` | ✅ создан | ✅ green |
| 41-18-T2 | 18 | 9 | GRP-02 | T-41-18-03 | чистые функции порядка, golden-фикстура + selftest | node-гейт | `node ui/scripts/check-reorder.mjs --selftest && node ui/scripts/check-reorder.mjs` | ✅ создан | ✅ green |
| 41-19-T1 | 19 | 10 | GRP-04, GRP-01 | T-41-19-04 | дерево типов и групп (ARIA, клавиатура, untrack) | UI/lint + human | `pnpm --dir ui svelte-check && pnpm --dir ui lint` | ✅ создан | ✅ green (авто); 👤 живая проверка открыта |
| 41-19-T2 | 19 | 10 | GRP-01, GRP-04, GRP-10 | T-41-19-02 | модалки типа/группы/удаления без нативного select | UI/lint + human | `pnpm --dir ui svelte-check && pnpm --dir ui lint` | ✅ создан | ✅ green (авто); 👤 живая проверка открыта |
| 41-20-T1 | 20 | 10 | GRP-02, GRP-03 | T-41-20-02 | таблица свойств, скрытие, попап нарушителей, disabled data_type | UI/lint + human | `pnpm --dir ui svelte-check && pnpm --dir ui lint` | ✅ создан | ✅ green (авто); 👤 живая проверка открыта |
| 41-20-T2 | 20 | 10 | GRP-02 | T-41-20-03 | перестановка на pointer-events с откатом | UI/lint + human (оба транспорта) | `pnpm --dir ui svelte-check && pnpm --dir ui lint && node ui/scripts/check-reorder.mjs` | ✅ создан | ✅ green (авто); 👤 живая проверка открыта |
| 41-20-T3 | 20 | 10 | GRP-01, GRP-09 | T-41-20-01 | панель типа, manager только чтение | UI/lint + human | `pnpm --dir ui svelte-check && pnpm --dir ui lint` | ✅ создан | ✅ green (авто); 👤 живая проверка открыта |
| 41-21-T1 | 21 | 10 | GRP-05 | T-41-21-06 | Dropdown getGroupSection аддитивно | UI/lint + human | `pnpm --dir ui svelte-check && pnpm --dir ui lint` | ✅ создан | ✅ green (авто); 👤 живая проверка открыта |
| 41-21-T2 | 21 | 10 | GRP-05, GRP-06 | T-41-21-01,03 | таблица состава, строка-поиск (устройства+группы), вложенные строки | UI/lint + human | `pnpm --dir ui svelte-check && pnpm --dir ui lint` | ✅ создан | ✅ green (авто); 👤 живая проверка открыта |
| 41-21-T3 | 21 | 10 | GRP-06 | T-41-21-03 | мультивыбор + реестр INV-7 (маркеры групп) | gate + human | `node ui/scripts/check-place-tree-invalidation.mjs` | ✅ создан | ✅ green (авто); 👤 живая проверка открыта |
| 41-22-T1 | 22 | 10 | GRP-08 | T-41-22-02 | чипсы пользователей (userOptions, основной) | UI/lint + human | `pnpm --dir ui svelte-check && pnpm --dir ui lint` | ✅ создан | ✅ green (авто); 👤 живая проверка открыта |
| 41-22-T2 | 22 | 10 | GRP-08 | T-41-22-02 | единый список принтеров USB+ссылки | UI/lint + human | `pnpm --dir ui svelte-check && pnpm --dir ui lint` | ✅ создан | ✅ green (авто); 👤 живая проверка открыта |
| 41-22-T3 | 22 | 10 | GRP-03, GRP-08 | T-41-22-01 | форма свойств: серверные ошибки по полю, нормализованные значения | UI/lint + human | `pnpm --dir ui svelte-check && pnpm --dir ui lint` | ✅ создан | ✅ green (авто); 👤 живая проверка открыта |
| 41-23-T1 | 23 | 11 | GRP-06 | T-41-23-03,04 | GroupMoveModal (одно действие из двух точек), панель группы, история на MovementTimeline | UI/lint+gate | `pnpm --dir ui lint && node ui/scripts/check-place-tree-invalidation.mjs` | ✅ создан | ✅ green |
| 41-23-T2 | 23 | 11 | GRP-04, GRP-09 | T-41-23-01,02 | страница раздела, маршрут, сайдбар, роли | UI build + human | `pnpm --dir ui svelte-check && pnpm --dir ui lint && pnpm --dir ui build` | ✅ создан | ✅ green (авто); 👤 живая проверка открыта |
| 41-23-T3 | 23 | 11 | GRP-04, GRP-09 | T-41-23-01 | гейт сайдбара и маршрутов раздела | node-гейт | `node ui/scripts/check-groups-section.mjs --selftest && node ui/scripts/check-groups-section.mjs` | ✅ создан | ✅ green |
| 41-24-T1 | 24 | 11 | GRP-07 | T-41-24-01 | PlacePicker disabled для члена группы с местом | UI/lint + human | `pnpm --dir ui svelte-check && pnpm --dir ui lint` | ✅ создан | ✅ green (авто); 👤 живая проверка открыта |
| 41-24-T2 | 24 | 11 | GRP-06 | T-41-24-02 | таймлайн «в составе группы» + allowlist словарного гейта | UI/lint + human | `pnpm --dir ui lint && node ui/scripts/check-group-vocabulary.mjs --selftest` | ✅ создан | ✅ green (авто); 👤 живая проверка открыта |
| 41-25-T1 | 25 | 12 | GRP-06 | T-41-25-01,03 | свёртка пакета в отчёте, печать без правил свёртки | UI/lint + human (печать) | `pnpm --dir ui svelte-check && pnpm --dir ui lint` | ✅ создан | ✅ green (авто); 👤 живая проверка открыта |
| 41-25-T2 | 25 | 12 | GRP-06, GRP-07 | T-41-25-04 | модалка и тост массового переноса с группами | UI/lint + human | `pnpm --dir ui lint && node ui/scripts/check-place-tree-invalidation.mjs` | ✅ создан | ✅ green (авто); 👤 живая проверка открыта |
| 41-26-T1 | 26 | 13 | GRP-01..GRP-10 | T-41-26-02 | полный regress workspace один раз, clippy | full suite | `cargo clippy --workspace --all-targets -- -D warnings` | ✅ (прогон) | ✅ green |
| 41-26-T2 | 26 | 13 | GRD-06, GRP-04 | T-41-26-01 | UI-гейты в порядке CI, privacy | CI-chain | `pnpm --dir ui lint && node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256` | ✅ создан | ✅ green |
| 41-26-T3 | 26 | 13 | все | T-41-26-04 | заполнение карты и сбор живых проверок | doc | `node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256` | ✅ создан | ✅ green |
| 41-27-T1 | 27 | 1 | GRP-03, GRP-04 | T-41-27-03 | гейт «панель меню не в потоке», 10 мутантов selftest | node-гейт | `node ui/scripts/check-action-menu-portal.mjs --selftest && node ui/scripts/check-action-menu-portal.mjs` | ✅ создан | ✅ green |
| 41-27-T2 | 27 | 1 | GRP-03, GRP-04 | T-41-27-02,03 | портал по умолчанию, проп `portal` удалён, триггер виден в деревьях | UI/lint + human | `pnpm --dir ui svelte-check && pnpm --dir ui lint` | ✅ создан | ✅ green (авто); ⬜ ожидает живой перепроверки (R1, R2) |
| 41-27-T3 | 27 | 1 | GRP-03, GRP-04 | T-41-27-03 | сборка и красный сценарий правила F на реальных файлах | UI build + human | `pnpm --dir ui svelte-check && pnpm --dir ui lint && pnpm --dir ui build` | ✅ создан | ✅ green (авто); ⬜ ожидает живой перепроверки (R1, R2) |
| 41-28-T1 | 28 | 1 | GRP-01, GRP-09 | T-41-28-01 | одна нейтральная строка хинта для всех ролей | UI/lint + human | `pnpm --dir ui svelte-check && pnpm --dir ui lint` | ✅ создан | ✅ green (авто); ⬜ ожидает живой перепроверки (R3) |
| 41-28-T2 | 28 | 1 | GRP-01, GRP-09 | T-41-28-02 | UI-SPEC, VALIDATION, deferred-items синхронизированы со старой формулировкой | doc grep | `grep -rn` по прежней формулировке в трёх документах, ожидается код 1 (не найдено) | ✅ создан | ✅ green |
| 41-29-T1 | 29 | 2 | GRP-03 | T-41-29-01,02 | красные `protect_d_*` (скрытие обязательного снимает признак; скрытому нельзя потребовать обязательность; возврат легаси-строки с нарушителями отвергается) | app service | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test groups_types_service protect_d` (красная фаза — в SUMMARY 41-29; сейчас 3 теста зелёные) | ✅ создан | ✅ green |
| 41-29-T2 | 29 | 2 | GRP-03 | T-41-29-01..03 | `archive_property` снимает `is_required` тем же UPDATE; `update_property` отвергает обязательность скрытого; защита при возврате — запасная для легаси-строк | infra + app service | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test groups_types_service && cargo test -p trackly-infra --test group_types_repo` | ✅ создан | ✅ green |
| 41-30-T1 | 30 | 3 | GRP-06 | T-41-30-01,03 | общая фикстура уведомления и `report_trunc_*` (красные до правки) | app + fixture | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test report_movements_truncation` (красная фаза — в SUMMARY 41-30) | ✅ создан | ✅ green |
| 41-30-T2 | 30 | 3 | GRP-06 | T-41-30-01,03,04 | истинный `total` вторым `COUNT(*)`, уведомление в печати и CSV, значок вкладки по `resp.total` | app + human (печать) | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test report_movements_truncation && TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test group_report_batch` | ✅ создан | ✅ green (авто); ⬜ ожидает живой перепроверки (R5) |
| 41-31-T1 | 31 | 4 | GRP-03 | T-41-31-05 | `propertyRemovalCopy`, общая фикстура, гейт с 7 мутантами | node-гейт | `node ui/scripts/check-property-removal.mjs --selftest && node ui/scripts/check-property-removal.mjs` | ✅ создан | ✅ green |
| 41-31-T2 | 31 | 4 | GRP-03 | T-41-31-01..04 | честный копирайт в таблице свойств; сервер соответствует той же фикстуре (код `crates/*/src` не менялся) | UI/lint + parity + human | `pnpm --dir ui svelte-check && pnpm --dir ui lint && TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test groups_property_removal_parity` | ✅ создан | ✅ green (авто); ⬜ ожидает живой перепроверки (R4) |
| 41-31-T3 | 31 | 4 | GRP-03 | T-41-31-01 | H20-1 переписан в VALIDATION, UI-SPEC синхронизирован, решение по контекстным меню записано | doc | `grep -c "Удалить безвозвратно" .planning/phases/41-groups-model-and-editor/41-UI-SPEC.md` (2) | ✅ создан | ✅ green |
| 41-32-T1 | 32 | 5 | GRP-01, GRP-02, GRP-03 | T-41-32-01,02 | красные `atomic_*` с fault-injection триггером на `audit_log` | app service | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test groups_types_service atomic_` (красная фаза — в SUMMARY 41-32; сейчас 10 тестов зелёные) | ✅ создан | ✅ green |
| 41-32-T2 | 32 | 5 | GRP-01, GRP-02, GRP-03 | T-41-32-01,02 | мутатор и запись аудита в одной транзакции; тела мутаторов в `*_on(&self, conn, …)` (отступление от плана: rusqlite 0.38, `&mut tx` не компилируется; критерий `conn.savepoint()` == 1 НЕ выполнен — их 0) | app service + infra | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test groups_types_service && cargo test -p trackly-infra --test group_types_repo` | ✅ создан | ✅ green |
| 41-33-T1 | 33 | 6 | GRP-06 | T-41-33-04 | `movementsTruncationNotice`, гейт с 8 мутантами; фикстура расширена оркестратором кейсом «500 из 800» (e09fd295) против захардкоженного потолка `total <= 1000` | node-гейт | `node ui/scripts/check-report-truncation.mjs --selftest && node ui/scripts/check-report-truncation.mjs` | ✅ создан | ✅ green |
| 41-33-T2 | 33 | 6 | GRP-06 | T-41-33-01..03 | экранный баннер усечения в ReportsPage по `rows.total` (`$derived`, без `$effect`), гейт в `pnpm lint` | UI build + human | `pnpm --dir ui svelte-check && pnpm --dir ui lint && pnpm --dir ui build` | ✅ создан | ✅ green (авто); ⬜ ожидает живой перепроверки (R5) |
| 41-34-T1 | 34 | 7 | GRP-01..GRP-09 | T-41-34-03 | полный регресс одной цепочкой в порядке CI, последовательно | full suite | семь шагов CI-цепочки — см. раздел «Результаты прогонов (план 41-34…)» ниже | ✅ (прогон) | ✅ green |
| 41-34-T2 | 34 | 7 | все | T-41-34-01 | гейт приватности и ручной греп добавленных строк волны | privacy | `node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256` | ✅ создан | ✅ green |
| 41-34-T3 | 34 | 7 | все | T-41-34-02 | два UAT-гэпа — «починено, ожидает перепроверки»; единый список живых проверок (пункты 20-24 в 41-HUMAN-UAT.md) | doc + human | `test "$(grep -c 'status: failed' 41-UAT.md)" = "0"` (в каталоге фазы) | ✅ создан | ✅ green (авто); ⬜ ожидает живой перепроверки (R1-R5 целиком) |
| 41-WR01 | — (после волны, `f027cc0b`) | — | GRP-03 | T-41-29-01,02 | возврат свойства из скрытых снимает `is_required` тем же UPDATE (симметрия со скрытием), а не отказывает — наследная строка «скрыто + обязательное» больше не запирает пользователя | app service | `DEVELOPER_DIR=/Library/Developer/CommandLineTools TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test groups_types_service` (тест `protect_d_unarchive_clears_legacy_required_flag` — заменил `protect_d_unarchive_refuses_legacy_required_with_violators`) | ✅ создан | ✅ green (39/39 на HEAD, перепроверено аудитом 2026-10-06) |
| 41-AUDIT-G1 | — (аудит /gsd-validate-phase) | — | GRP-09 (метод проверки прав/транспортов) | — | команда Tauri без HTTP-маршрута не проходит незамеченной: инвентари читаются от исходников, расхождения сверяются в обе стороны, живой `build_router` отвечает на каждый путь | app gate (разбор исходника + живой роутер) | `DEVELOPER_DIR=/Library/Developer/CommandLineTools TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test http_route_parity` | ✅ создан | ✅ green (5/5, 0.30 с) |

**Заполнена планом 41-26 по факту прогонов 2026-10-04** (26 планов, 59 задач): «File Exists» и «Status» выставлены после зелёных прогонов, перечисленных в разделе «Результаты прогонов». Статус «green» означает ТОЛЬКО автоматическую проверку из колонки «Automated Command»; в строках с пометкой «живая проверка открыта» рантайм и вид в приложении не проверены (см. «Живые проверки для пользователя»).
**Дополнена планом 41-34 по факту прогонов 2026-10-05** (задачи 41-27…41-34, 19 строк). Статус «green» — только команда из колонки «Automated Command», реально выполненная зелёной в границе волны. «⬜ ожидает живой перепроверки» — рантайм и вид в приложении не проверены; пункты R1-R5 живых проверок — 41-HUMAN-UAT.md, пункты 20-24. `nyquist_compliant` этой волной не меняется, пока открыты R1-R5.

Файлы тестов в плане именуются по факту: `group_types_repo` (типы, триггер), `groups_repo` (группы, PK, CTE), `groups_membership`, `groups_move`, `groups_values_card`, `group_report_batch`, `places_move_groups` — вместо обобщённых `groups_service move_/card_/values_` из таблицы ниже.

### Phase Requirements → Test Map (из RESEARCH.md)

| Req | Layer | Target | File |
|-----|-------|--------|------|
| 1 — `code`/`behavior` неизменяемы | infra (триггер БД) + app + dual-transport | `group_types_repo` (триггер), `groups_types_service`, `role_endpoint_matrix group_types` | ✅ создан |
| 2 — идемпотентный засев | app service | `groups_types_service seed_` | ✅ создан |
| 3 — свойства 6 типов, порядок, `show_on_map` | app service + DTO round-trip | `groups_types_service properties_` | ✅ создан |
| 4 — защита заполненных свойств (3 отдельных теста) | app service + прямой `SELECT` значений | `groups_types_service protect_` | ✅ создан |
| 5 — валидация значений на сервере | core таблица-тест + оба транспорта | `trackly-core group_values`, `groups_values_card values_`, `role_endpoint_matrix groups_` (HTTP 400) | ✅ создан |
| 6 — раздел и права видимости | UI-гейт + серверный 403 | `check-groups-section.mjs` | ✅ создан |
| 7 — `seq` и нумерация (+ гонка) | app service + `UNIQUE(type_id,seq)` | `groups_service numbering_` (план 08) | ✅ создан |
| 8 — членство, циклы, `teardown` | infra (PK, CTE) + app (сообщения) | `groups_repo`, `groups_membership` | ✅ создан |
| 9 — протаскивание места **атомарно** | infra+app, фикстура сбоя на 4-м устройстве | `groups_move` (move_, move_atomic_), `places_move_groups` | ✅ создан |
| 10 — журнал пакета, отчёт, таймлайн, `ALTER` | app + infra-миграция `run_up_to(44)` | `group_movements_journal`, `place_movements_batch_repo`, `place_movements_group_fields`, `group_report_batch`, `groups_migration` | ✅ создан |
| 11 — запрет индивидуального перемещения | app таблица-драйвер S1–S9 + счётный гейт исходника | `group_write_sites` | ✅ создан |
| 12 — карточка группы, дедупликация принтеров | app service | `groups_values_card` (card_, user_options_) | ✅ создан |
| 13 — матрица 3 роли × (тип, группа) × 2 транспорта | HTTP-сессии + `build_*` + тест полноты маршрутов | `role_endpoint_matrix` Cases 76+ | ✅ создан (расширен) |
| 14 — переименование свёртки | UI-гейт словаря с `--selftest` | `check-group-vocabulary.mjs` | ✅ создан |
| — `Action`-матрица | core unit | `trackly-core auth` | ✅ дописан |
| — `per_record_invariants` новых таблиц | infra | `per_record_invariants` | ✅ дописан |
| — INV-7 реестр новых producer'ов | UI-гейт | `check-place-tree-invalidation.mjs` | ✅ дописан |
| — чистые функции `reorder.ts` | node-гейт-фикстура | `check-reorder.mjs` | ✅ создан |

---

## Wave 0 Requirements

- [x] `crates/trackly-core/src/domain/group_values.rs` — таблицы-тесты ip/mac/number/text (Req 5)
- [x] `crates/trackly-infra/tests/groups_migration.rs` — V045/V046, `run_up_to(44)` без потери данных (Req 10)
- [x] `crates/trackly-infra/tests/group_types_repo.rs` — триггер code/behavior, засев, свойства (Req 1–4)
- [x] `crates/trackly-infra/tests/groups_repo.rs` — PK членства, CTE цикла/состава, seq, значения, принтеры (Req 7, 8, 12); атомарность переноса доказана на уровне сервиса в `groups_move` (Req 9)
- [x] `crates/trackly-infra/tests/place_movements_batch_repo.rs` — batch_id, entity_label, group_id (Req 10)
- [x] `crates/trackly-app/tests/groups_types_service.rs` — Req 1–4 + засев
- [x] `crates/trackly-app/tests/groups_service.rs` — Req 7 (нумерация), чтения, CRUD (Req 12 частично)
- [x] `crates/trackly-app/tests/groups_membership.rs` — Req 8 (членство, вложенность, инвариант места)
- [x] `crates/trackly-app/tests/groups_move.rs` — Req 9 (перенос, атомарность)
- [x] `crates/trackly-app/tests/groups_values_card.rs` — Req 5, 12 (значения, карточка, принтеры)
- [x] `crates/trackly-app/tests/places_move_groups.rs`, `group_report_batch.rs`, `place_movements_group_fields.rs` — D-23, отчёт, DTO таймлайна (Req 9, 10)
- [x] `crates/trackly-app/tests/group_write_sites.rs` — Req 11, сценарии S1–S9, счётный гейт `act_service.rs`
- [x] `crates/trackly-app/tests/group_movements_journal.rs` — Req 10 (журнал, отчёт, таймлайн)
- [x] `crates/trackly-app/tests/role_endpoint_matrix.rs` — Cases 76+ (Req 13) + тест полноты маршрутов
- [x] `ui/scripts/check-group-vocabulary.mjs` (+ `--selftest`), `check-groups-section.mjs`, `check-reorder.mjs`; подключить в цепочку `lint` в `ui/package.json`
- [x] Расширить: `auth.rs::tests`, `per_record_invariants.rs` (списки таблиц), `check-place-tree-invalidation.mjs` (реестр INV-7)
- [x] Framework install: **не требуется**

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

**Статус на 2026-10-06 (живая приёмка проведена):** все семь поведений выше закрыты пунктами
H19-1…H25-2 и R1-R5 — `41-UAT.md` 21/21 pass, `41-HUMAN-UAT.md` 23 pass / 1 issue из 24
(issue — пункт 24: побочный дефект счётчиков «Отчётов» по LAN, вне фазы 41, отложен решением
пользователя). Прогон на обоих транспортах: Tauri WKWebView + LAN-браузер на macOS.
**Windows WebView2 в артефактах не зафиксирован** — пункты помечены «по возможности», запись о
прогоне на этом движке отсутствует; считать непроверенным.

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

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Per-Task Verification Map заполнена (не остаётся заглушкой)
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 60s
- [x] Живые проверки собраны и отданы пользователю ДО `/gsd-verify-work` (список ниже) — **и пройдены 2026-10-06**: 23 pass / 1 issue из 24
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** автоматическая граница фазы закрыта 2026-10-04 (план 41-26), расширена волной 41-27…41-34
(2026-10-05) и правкой WR-01 (2026-10-06). Живые проверки **пройдены 2026-10-06** (23/24, один issue —
побочный дефект вне фазы). Аудит `/gsd-validate-phase` от 2026-10-06 перепроверил автоматику на HEAD и
закрыл пробел G1 новым гейтом `http_route_parity`. Остаётся непроверенным: Windows WebView2.

---

## Результаты прогонов (план 41-26, 2026-10-04, коммит-основа 1380f9c4)

Все прогоны выполнены последовательно, по одному `cargo` за раз; env `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1`, префикс `DEVELOPER_DIR=/Library/Developer/CommandLineTools`.

### Целевые цели (каждая целиком, код выхода 0)

| Слой | Цель — результат |
|------|------------------|
| core | `trackly-core` — 149 passed, 0 failed |
| infra | `groups_migration` 4 · `group_types_repo` 14 · `groups_repo` 18 · `place_movements_batch_repo` 6 · `per_record_invariants` 3 · `migration_idempotency` 2 — все 0 failed |
| app, новые | `groups_types_service` 26 · `groups_service` 15 · `groups_move` 10 · `group_movements_journal` 5 · `groups_membership` 14 · `groups_values_card` 19 · `group_write_sites` 23 · `group_report_batch` 10 · `places_move_groups` 6 · `place_movements_group_fields` 5 · `role_endpoint_matrix` 12 (с `--skip login_remember_persistent_cookie`) — все 0 failed |
| app, регресс write-site'ов целиком | `acts_crud` 10 · `acts_returns` 20 · `acts_update` 17 · `acts_update_return` 20 · `acts_undo` 6 · `acts_clone_handover` 12 · `acts_changed_place_ids` 5 · `acts_place_snapshot` 4 · `acts_e2e_smoke` 4 · `place_movements_bulk_move` 5 · `place_movements_write_sites_devices` 6 · `place_movements_write_sites_cartridges` 3 · `place_movements_timeline` 6 · `places_contents` 5 · `places_delete_blocked` 11 · `places_service_crud` 4 · `devices_crud` 19 · `report_movements` 18 · `report_csv_export` 2 · `html_report_render` 8 · `html_header_parity` 5 · `cartridges_lifecycle` 36 — все 0 failed |

`role_endpoint_matrix` включает группы: `group_types_role_matrix_{http,tauri_path}`, `groups_role_matrix_{http,tauri_path}`, `group_types_http_immutability_not_vacuous`, `group_types_http_protect_rules`, `groups_http_server_side_validation`, `groups_http_membership_and_nesting_rules`, `groups_manager_can_create_group_not_type`, а также тесты полноты маршрутов `group_types_http_route_completeness` и `groups_http_route_completeness`.

### Полный прогон workspace (один раз)

`cargo test --workspace --no-fail-fast -- --test-threads=1 --skip login_remember_persistent_cookie` — код выхода 0; **156 тестовых бинарей, 1604 passed, 0 failed, 5 ignored**; длительность около 8 минут (инкрементальная сборка, не 80). Совпадает с прогоном оркестратора на c2c31640 (1604) — после него изменялись только документы.

### Цепочка CI-гейтов, в порядке CI

| Шаг | Результат |
|-----|-----------|
| `cargo test -p trackly-app --test export_bindings` | 1 passed |
| `pnpm --dir ui svelte-check` | 309 файлов, **0 ERRORS, 68 WARNINGS** (база восстановлена; ранее случившийся ERROR в `MovementTimelineSection.svelte` — фикстура без `batch_id` после регенерации `bindings.ts` планом 41-05 — исправлен в 17d26df2) |
| `pnpm --dir ui lint` | код 0: eslint, prettier, все `check-*.mjs`; `check-group-vocabulary` — 20 фикстур selftest, 212 файлов; `check-reorder` — selftest ловит 3 мутанта, 22 кейса; `check-groups-section` — 6 фикстур selftest, 0 нарушений; `check-place-tree-invalidation` — 0 нарушений (маркеры `groups.move(`, `groups.addDevices(`, `groups.setParent(` добавлены планом 41-21, гейт расширен, не ослаблен) |
| `pnpm --dir ui build` | код 0 (хук `prebuild` прогнал `export_bindings`); `ui/dist` пересобран |
| `cargo clippy --workspace --all-targets -- -D warnings` | код 0 |
| `node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256` | PASS, 0 нарушений |
| `cargo fmt` на файлах фазы (69 `.rs`, изменённых с 0b7239b8^) | чисто; единственный дифф — в `crates/trackly-app/src/dto/act.rs`, который фаза не меняла (см. «Заранее существовавшие условия») |

### Чистота дерева и мутаций

`git status --short` пуст; `git diff HEAD` и `grep` по `crates`/`migrations` не находят следов мутационных проверок (`ON CONFLICT(code) DO UPDATE`, `DO UPDATE SET name = excluded.name`, `WHEN 0` в хвосте строки).

### Гейт write-site'ов `devices.place_id` (план 41-16)

`crates/trackly-app/tests/group_write_sites.rs`: реестр — `act_service.rs` 8 вызовов записи, `place_service.rs` 1, `group_place.rs` 1, плюс скан `crates/*/src`, падающий на любом незарегистрированном писателе; для `act_service.rs` проверяется парность — у каждого из 8 write-site'ов свой `release` нужного вида в окне после него. Оркестратор независимо подтвердил ровно 8 реальных write-site'ов и 8 вызовов release.

Реестр S1–S10 закрыт: S1/S8/S9 — план 41-14, S2 — 41-15, S3–S7 — 41-16. Вне границ с названной причиной: S10 (создание/массовое/CSV — новое устройство не может быть членом группы), собственный путь группы `group_place.rs`, неиспользуемый нетранзакционный `DeviceRepository::update`, обновления `deleted_at_utc`.

### Приватность

Скрипт-гейт зелёный. Дополнительно, вручную, просмотрен весь `git diff 0b7239b8^..HEAD` (141 файл, около 30 тыс. добавленных строк: `.planning/phases/41-*`, `crates/*/tests`, `crates/*/src`, миграции, `ui/src`, `ui/scripts/fixtures`) регулярными выражениями на ФИО (инициалы, «Фамилия Имя»), e-mail, телефоны, IP, MAC, ИНН/КПП/ОГРН/ОКПО, адреса, организационно-правовые формы. Найдено только вымышленное: ФИО «Иванов И.И.», «Петров П.П.», «Сидоров С.С.», «Иванова И.И.», «Иванова А.А.», «Иванкин К.К.»; адреса 10.0.0.x, 192.168.1.10, 1.2.3.4, 0.0.0.0, 255.255.255.255; MAC `00:1b:44:11:3a:b7`, `aa:bb:cc:dd:ee:ff`, `00:00:00:00:00:01`. E-mail, телефонов, реквизитов, адресов, названий организаций — не найдено. Удалять было нечего. Граница гарантии: просмотрен диф фазы, а не вся история репозитория.

---

## Результаты прогонов (план 41-34, 2026-10-05, граница волны 41-27…41-33, коммит-основа волны 1f82b55b, HEAD e09fd295)

Границу волны закрывает ОДИН полный прогон, строго последовательно, по одному `cargo` за раз; рабочее дерево перед цепочкой чистое. Порядок — порядок `ci-fast.yml`. Каждый шаг записан отдельно.

| # | Шаг | Код | Результат |
|---|-----|-----|-----------|
| 1 | `node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256` | 0 | PASS, 0 нарушений |
| 2 | `pnpm --dir ui build` | 0 | собран; `git status` по `ui/src` пуст, `bindings.ts` не менялся |
| 3 | `rustfmt --check` по 7 `.rs`-файлам, изменённым волной | 0 | чисто. Полный `cargo fmt --all -- --check` НЕ запускался: в `crates/trackly-app/src/dto/act.rs` и ряде тестов — заранее существующий дрейф, не волны |
| 4 | `cargo clippy --workspace --all-targets -- -D warnings` | 0 | чисто (кэш: исходники после последнего clippy не менялись) |
| 5 | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test --workspace --no-fail-fast -- --test-threads=1 --skip login_remember_persistent_cookie` | 0 | **158 тестовых бинарей, 1625 passed, 0 failed, 5 ignored**, 4779 с (около 80 минут, не 8) |
| 6 | `pnpm --dir ui svelte-check` | 0 | 311 файлов, 0 ERRORS, 68 WARNINGS (база прежняя) |
| 7 | `pnpm --dir ui lint` | 0 | eslint, prettier и все `check-*.mjs`; среди них `check-action-menu-portal` (selftest 10 фикстур + прогон, 0 нарушений), `check-property-removal` (7 мутантов, 6 кейсов), `check-report-truncation` (8 мутантов, 8 кейсов), `check-groups-section`, `check-reorder` (22 кейса), `check-group-vocabulary` (214 файлов) |

Относительно 41-26 (156 бинарей, 1604 passed): +2 бинаря (`groups_property_removal_parity`, `report_movements_truncation`), +21 тест. По целям волны в полном прогоне: `groups_types_service` 39 passed (в том числе `atomic_*` — 10, `protect_d_*` — 3), `report_movements_truncation` 7, `groups_property_removal_parity` 1, `group_report_batch` 10, `group_types_repo` 14 — все 0 failed.

Предсуществующий пропуск: `login_remember_persistent_cookie` (подвисание) — `--skip`, как и в 41-26; не вводится волной.

### Отступления плана, зафиксированные в границе

- 41-29: вместо одной защиты при возврате — `archive_property` снимает `is_required` тем же UPDATE, что и скрытие; `update_property` отвергает `is_required=true` у скрытого; защита при возврате осталась запасной для легаси-строк (иначе пользователь оказывался в тупике).
- 41-31: `crates/*/src` не менялся вообще (сервер вёл себя верно) — копирайт плюс паритет-тест на общей фикстуре.
- 41-32: rusqlite 0.38 (не 0.39, как в CLAUDE.md), у `Transaction` нет `DerefMut`; тела мутаторов вынесены в `*_on(&self, conn, …)`. Критерий плана `grep -c "conn.savepoint()" == 1` НЕ выполнен (0) — отступление записано, не затушёвано.
- Оркестратор (e09fd295): в фикстуру `report-truncation/cases.json` добавлен 8-й кейс «500 из 800» — мутант с захардкоженным `total <= 1000` проходил все 7 прежних кейсов. Кейс убивает мутанта, selftest гейта зелёный, `report_movements_truncation` 7/7 на обновлённой фикстуре.
- 41-30: тест `report_trunc_under_limit_total_equals_rows` — охранный, зелёный и до, и после правки; исполнитель заявил это открыто.

### Приватность (план 41-34)

Скрипт-гейт зелёный. Вручную просмотрены добавленные волной строки (`git diff 1f82b55b..HEAD -U0`: около 3400 строк кода и тестов, около 3400 строк `.planning/`) регулярными выражениями на ФИО (инициалы, «Фамилия Имя Отчество»), e-mail, телефоны, IP, MAC, ИНН/КПП/ОГРН/ОКПО, организационно-правовые формы, адреса. В коде и тестах совпадений нет. В `.planning/` — только вымышленное («Иванов И.И.», «Петров П.П.», «Сидоров С.С.», «Иванова И.И.», «Иванова А.А.», «Иванкин К.К.», «Кузнецов К.К.»; адреса 192.168.1.10, 192.168.1.100, 1.2.3.4; MAC из документационных примеров). Реквизитов, e-mail, телефонов, организаций, адресов — 0. Файлы волны из списка плана (`groups_types_service.rs`, `report_movements_truncation.rs`, `groups_property_removal_parity.rs`, обе `cases.json`, три `.mjs`-гейта, `propertyRemoval.ts`, `truncationNotice.ts`) проверены тем же способом: реальных имён нет. SUMMARY планов 41-27…41-33 прочитаны на цитаты живой БД, логов и вывода запросов — не найдено (упоминания «копия dev-БД» — описание процедуры, не вывод). Найдено 0 случаев, удалять было нечего. Граница гарантии: проверен диф волны, не вся история репозитория.

---

## Покрытие требований автоматикой

| Требование | Чем покрыто (машинно) | Остаётся человеку |
|------------|----------------------|-------------------|
| GRP-01 неизменяемые `code`/`behavior` | триггер БД (`group_types_repo`), сервис, HTTP-тест с отличающимся `code` (`group_types_http_immutability_not_vacuous`) | панель типа, права manager (H20-3) |
| GRP-02 свойства типа, порядок | `groups_types_service`, `check-reorder` (22 кейса, 3 мутанта) | жест перетаскивания (H20-2, сценарий А) |
| GRP-03 скрытие, типы данных, нормализация | `groups_types_service protect_*`, `protect_d_*` (W-B01, план 41-29), `groups_property_removal_parity` и `check-property-removal` (ветвление копирайта по `filled_group_count`, план 41-31), `group_values` (core), `groups_values_card values_*`, HTTP 400 | формы и попап нарушителей (H20-1, H22-3) |
| GRP-04 раздел «Группы», нумерация | `groups_service numbering_*`, `check-groups-section` | страница на обоих транспортах (H19-1, H23-1) |
| GRP-05 членство, вложенность | `groups_repo`, `groups_membership` (PK, CTE цикла, teardown) | состав, мультивыбор (H21-1…3) |
| GRP-06 место группы, журнал пакета | `groups_move` (атомарность с fault-injection), `group_movements_journal`, `group_report_batch`, `places_move_groups` | модалка, тост, отчёт, печать (H23-1, H24-2, H25-1, H25-2, сценарий Б) |
| GRP-07 запрет индивидуального перемещения | `group_write_sites` (S1–S9 + счётный гейт) | заблокированный PlacePicker (H24-1) |
| GRP-08 карточка группы | `groups_values_card`, `groups_repo` | чипсы пользователей, принтеры (H22-1, H22-2) |
| GRP-09 права на обоих транспортах | `role_endpoint_matrix` (3 роли x 2 транспорта + полнота маршрутов), `Action`-матрица в core | видимость по ролям (H20-3, H23-2) |
| GRP-10 удаление группы | освобождение устройств, неизменный `devices.place_id`, вложенные группы становятся корнями, число освобождаемых для подтверждения — `groups_service`, `places_delete_blocked` | текст модалки удаления (H19-2) |
| GRD-06 переименование «Свернуть…» | `check-group-vocabulary` (20 фикстур) | — |

**GRP-10 — ЧАСТИЧНО, сознательно.** В `REQUIREMENTS.md` отмечен `[~]`, трассировка «Phase 41 + 41.1». Фаза 41 даёт освобождение устройств при удалении группы, неизменное `devices.place_id`, превращение вложенных групп в корневые и число освобождаемых для подтверждения. Вторая половина — «удаление устройства-якоря запрещено, пока существует группа» — требует `groups.anchor_device_id`, которого миграция V045 не создаёт; она переносится в фазу 41.1. GRP-10 НЕ считать выполненным.

Остальные галочки `[x]` в `REQUIREMENTS.md` означают «автоматическая граница пройдена», а не «принято в живом приложении».

---

## Заранее существовавшие условия (НЕ регрессии фазы 41)

- **Зависающий тест** `login_remember_persistent_cookie` в `trackly-app` — зависает и при запуске пакета, и при `--workspace`. Все прогоны выше идут с `-- --skip login_remember_persistent_cookie`. Фазой 41 не вводился и не менялся; CI-шаг `cargo test` в `ci-fast.yml` флага `--skip` не содержит — это отдельный долг проекта.
- **Дрейф `cargo fmt --check`** в `crates/trackly-app/src/dto/act.rs` и нескольких тестовых файлах. Фаза 41 их не трогала (`git diff 0b7239b8^..HEAD` по `act.rs` пуст); файлы фазы форматированы корректно. Не «чинился» — вне границ плана.
- **Нужен настоящий `ui/dist`** (`pnpm --dir ui build`) для тестов `trackly-app`; при линковке с кодом 69 — `DEVELOPER_DIR=/Library/Developer/CommandLineTools`.
- **Хук `prebuild`** у `pnpm run build` запускает `cargo test -p trackly-app --test export_bindings` — это cargo-шаг (план 41-21 об этом сообщил).

---

## Что проверено машиной, а что НЕТ

**Машиной проверено:** схема и миграции (V045/V046, `run_up_to(44)` без потери `act_items`); триггеры; домен значений свойств; атомарность переноса группы; журнал пакета, отчёт, CSV и печатный состав на уровне данных; закрытый реестр write-site'ов `devices.place_id`; права на обоих транспортах; типы `bindings.ts` против обёртки из 25 команд; словарный гейт, реестр инвалидации INV-7, наличие раздела в сайдбаре и маршрутах, чистые функции перестановки против golden-фикстуры; clippy, svelte-check, eslint/prettier, сборка SPA; отсутствие известных токенов приватности.

**Машиной НЕ проверено — нужен человек в настоящем приложении (Tauri WKWebView и LAN-браузер):** рантайм и вид всего раздела «Группы». Компиляционные гейты не видят ошибок рун (в том числе `effect_update_depth_exceeded`), жестов, вёрстки, скролл-регионов, фокус-колец, многостраничной печати и асимметрии «десктоп vs браузер». Планы 41-19 … 41-25 записали каждый свой список UNVERIFIED и оставили `human-check` открытыми. План 41-23 прогнал смонтированную страницу в Playwright WebKit против поддельного бэкенда (около 12 сценариев, без ошибок консоли и без `effect_update_depth_exceeded`) — полезный сигнал, но по правилу проекта это НЕ верификация: вёрстка и ответы настоящего сервера остаются непроверенными. Исполнитель этого плана живые проверки не запускал и не закрывал.

---

## Отклонения от UI-SPEC и планов, зафиксированные в сводках

| Отклонение | Источник | Статус |
|------------|----------|--------|
| Шеврон пакета лежит в ячейке «Предмет» обычной `TableRow`, а не в `TableRow group` с colspan (UI-SPEC §14.1) | 41-25 | принято планировщиком (решение плана) |
| Hit-test строк перетаскивания через `button.drag-handle[data-prop-id]` -> `closest('tr')`, обработчики на div-обёртке вокруг `Table` | 41-20 | принято планировщиком (прямо предусмотрено планом) — проверить жестом (H20-2) |
| «Место» в модалке мультивыбора — `PlacePicker`, а не Dropdown | 41-21 | принято планировщиком (так предписано планом) |
| Пометка «уже в группе «…»» выводится в мета-слот Dropdown, а не отдельным caption-стилем | 41-21 | требует решения пользователя при просмотре (H21-3) |
| Выбранный узел группы хранится страницей (`trackly:groups:selected`), а не деревом | 41-19 | принято планировщиком (план 23 поручает странице) |
| Тексты удаления для 0 и 1 устройства («Устройств в составе нет.», «освободится») | 41-19 | принято: копирайт, сверить в H19-2 |
| Тост после «Скрыть»: «Свойство скрыто» или «Свойство удалено» по исходу сервера | 41-20 | принято (исправление ошибки плана); с 41-31 копирайт модалки и пункта меню различает исходы заранее |
| Проп `showInitialPlacementNote` в `MovementTimeline` (вне `files_modified` плана 23) | 41-23 | принято (по умолчанию `true`, прочие потребители не меняются) |
| Эффект «статус На складе» отключён для заблокированного члена группы | 41-24 | принято (Rule 2) |
| Текст в `GroupTypePanel` второе предложение хинта (про название и набор свойств) для manager звучало как разрешение, которого нет | deferred-items, 41-23 | закрыто планом 41-28: пользователь решил — нейтральная формулировка «Код и поведение типа изменить нельзя.» для всех ролей |
| Пустой блок «Перемещений ещё не было…» в `MovementTimeline` остаётся устройство-ориентированным (для группы свой блок в `GroupPanel`) | deferred-items, 41-23 | принято планировщиком |
| Строчное «группировать» в `PlaceContents.svelte` проходит словарный гейт (ширина маркера `'групп'`; «Группировать» и «сгруппировать» ловятся) | 41-24/41-25 | известный задокументированный пробел гейта |

---

## Живые проверки для пользователя (до /gsd-verify-work)

> **Повторная проверка после волны 41-27…41-33** (план 41-34): пункты R1-R5 собраны отдельным списком в `41-HUMAN-UAT.md` (пункты 20-24), все `pending`. Пункт H20-1 ниже переписан планом 41-31, копирайт H20-3 решён планом 41-28; живая перепроверка обоих — за пользователем.

Это список к РУЧНОМУ прохождению. Исполнитель плана 41-26 ни одну из них не запускал и не отмечал пройденной. Запускать на ОБОИХ транспортах: десктоп — `cargo tauri dev` с ветки/дерева, где лежит изменение (исправления в стороннем worktree в работающее приложение не попадают, пока не слиты в `main`); LAN-браузер — после `pnpm --dir ui build`. По возможности — Windows WebView2. В каждом пункте дополнительно смотреть консоль: не должно быть `effect_update_depth_exceeded`.

**Предусловия, общие для списка.** Свойства типа «АРМ» — «Пользователи», «Подключённые принтеры», «Хост», «IP», «MAC» — засеваются по умолчанию (D-31); добавлять их руками НЕ нужно. «Хост» засеян необязательным. Встроенные типы: «АРМ», «Системный блок», «Разбор». Нужна dev-БД с тестовыми данными (не боевая) и минимум два пользователя помимо текущего.

### План 41-19 — дерево и модалки (2 пункта)

- [ ] **H19-1 · GroupTree.** Раздел «Группы» (после плана 23): дерево показывает три встроенных типа, раскрывается стрелками, у групп видны счётчики и бейдж «Без места»; ActionMenu строки подписан «Действия: …». Ожидание: ↑ ↓ Home End → ← работают; поиск «Поиск группы» раскрывает ветки; одинаково в Tauri и в браузере.
- [ ] **H19-2 · GroupTypeFormModal, GroupFormModal, GroupDeleteModal.** Под admin: «Создать тип» -> поля «Название» и «Поведение»; создать тип «Тест»; «Переименовать» в меню узла — поля «Поведение» нет; «Создать группу» (PageHeader) — без названия имя «{тип} #N»; удалить группу через меню узла — показано число освобождаемых устройств; выпадающие списки — компонент Dropdown, не нативный select. Тексты сверить с UI-SPEC §9.4, §17.4, §17.5. Отдельно: путь переименования типа шлёт тело без ключей `code`/`behavior` (приведение типа в `GroupTypeFormModal`; на рантайме не запускался).

### План 41-20 — свойства типа (3 пункта)

- [ ] **H20-1 · GroupTypePropertiesTable, PropertyRequiredViolatorsPopup.** Под admin, тип «АРМ» (переписано планом 41-31 после UAT (тест 5, GAP-2): прежнее ожидание считало, что скрывается любое свойство). (а) Добавить НОВОЕ свойство «Резервный адрес» (тип «IP-адрес»; имена засеянных свойств заняты) без значений: меню строки показывает красный пункт «Удалить свойство», модалка «Удалить свойство» с текстом про безвозвратное удаление, кнопка «Удалить безвозвратно», тост «Свойство удалено»; под «Показать скрытые» оно НЕ появляется — это ожидаемое поведение D-13 (пустое не архивируется), а не неработающий чекбокс. (б) Свойство с заполненными значениями (создать группу «АРМ», заполнить «Хост»): пункт «Скрыть», подтверждение с «можно будет вернуть», тост «Свойство скрыто»; при «Показать скрытые» — строка с бейджем «Скрыто» и пунктом «Вернуть»; вернуть. (в) Обязательное заполненное свойство: подтверждение скрытия содержит «Признак «Обязательное» при скрытии снимается.», после возврата чекбокс «Обязательное» снят. Прежние части: включить «Обязательное» при существующей группе без значения — PropertyRequiredViolatorsPopup со списком группы и кнопкой «Открыть», чекбокс остаётся снятым; у свойства с заполненными значениями Dropdown «Тип данных» disabled с подписью «заполнено в N группах».
- [ ] **H20-2 · Перестановка свойств.** В панели типа «АРМ» (минимум 3 свойства) перетащить свойство за ручку «⠿»; затем начать перетаскивание и нажать Esc; затем «Выше»/«Ниже» из меню; перезагрузить страницу. Ожидание: призрак и индикатор вставки; порядок сохраняется после перезагрузки; Esc и `pointercancel` возвращают прежний порядок без запроса; клик по чекбоксу «На карте» не запускает драг; озвучивается «перемещено на позицию N из M». (HTML5 DnD в WKWebView не работает — урок фазы 39.)
- [ ] **H20-3 · Панель типа по ролям.** Выбрать узел «АРМ» под admin и отдельной сессией под manager. Ожидание: admin видит «Показать скрытые» и строку добавления; manager — только чтение (Код/Поведение/Встроенный/Быстрое действие видны, контролов правки нет); нет двойного скролла. Строка над таблицей одинакова у admin и у manager: «Код и поведение типа изменить нельзя.» (решено в 41-28).

### План 41-21 — состав группы (3 пункта)

- [ ] **H21-1 · DropdownSection и существующие потребители Dropdown.** «Витрина компонентов» (admin) -> DropdownSection, демонстрация «Комбобокс с заголовками секций»: видны заголовки секций, ↑↓ перескакивают их. Затем форма акта (ActFormItemsTable) и пикер принтера выглядят и работают как раньше.
- [ ] **H21-2 · GroupContentsTable.** Группа -> вкладка «Состав»: в строке-поиске часть названия устройства — секции «Устройства» и «Группы»; выбрать устройство; выбрать другую группу (вложение); раскрыть шевроном вложенную строку; в меню ⋯ — «Вывести из состава». Ожидание: добавление без перезагрузки, поле очищается и держит фокус; «Вывести из состава» без подтверждения; занятое устройство -> Toast-ошибка с именем группы; счётчики дерева «Места» обновляются.
- [ ] **H21-3 · GroupAddDevicesModal.** «Добавить несколько…» -> отметить 3 устройства, одно уже в другой группе. Ожидание: занятое серое с подсказкой «Уже в группе «…»» и не отмечается; «Выбрано: 2», «Добавить (2)»; после добавления счётчики в дереве групп и в «Места» обновились.

### План 41-22 — карточка группы (3 пункта)

- [ ] **H22-1 · SC7 «из коробки», GroupUsersField.** На ЧИСТОЙ dev-БД (создана заново; админ свойства типа «АРМ» не добавлял и не менял) под manager создать группу типа «АРМ» и открыть карточку, вкладка «Свойства»: поля «Пользователи» (GroupUsersField) и «Подключённые принтеры» (GroupPrintersList) видны СРАЗУ. Добавить двух пользователей через «Добавить пользователя», второго отметить звёздочкой. Ожидание: поиск находит по ФИО и логину (в том числе кириллицей другого регистра) даже под manager; звёздочка переносится (основной один); «×» убирает чипс; звёздочка и «×» достижимы Tab и нажимаются Enter/Space.
- [ ] **H22-2 · GroupPrintersList, USB-связь.** USB-связь в интерфейсе НЕ создаётся (поле `usb_host_device_id` только отображается в карточке принтера). Подготовка на ЛОКАЛЬНОЙ dev-БД SQL-командой: `UPDATE printers SET usb_host_device_id = <id устройства-члена группы> WHERE device_id = <id принтера>` (id из `SELECT id, name FROM devices`). Затем карточка группы «АРМ» -> GroupPrintersList; затем добавить тот же принтер ссылкой. Ожидание: USB-принтер появляется сам с Badge «USB» без меню ⋯; после добавления ссылкой строка остаётся ОДНА, Badge «USB», но появляется ⋯ с «Убрать ссылку»; сетевой принтер только ссылкой — Badge «Ссылка». Если данные подготовить нельзя — пометить «не проверено: USB-связь не создаётся в UI», а не «пройдено».
- [ ] **H22-3 · GroupPropertiesForm.** Предусловия по шагам: (1) если у типа «АРМ» уже есть группы — заполнить «Хост» в карточке каждой (включение «Обязательное» сервер отклоняет, пока есть группы с пустым «Хост»; на чистой БД без групп шаг пропускается); (2) под admin в панели типа «АРМ» включить «Обязательное» у засеянного «Хост»; (3) создать НОВУЮ группу «АРМ» — «Хост» пуст. В её карточке: в «IP» ввести `2001:DB8:0:0:0:0:0:1`, в «MAC» — `AA-BB-CC-DD-EE-FF`, «Сохранить»; затем в «IP» `01.2.3.4` и сохранить; для обязательного «Хост» оставить пусто и сохранить. Ожидание: после первого сохранения `2001:db8::1` и `aa:bb:cc:dd:ee:ff` (пришли с сервера); невалидный IP — ошибка под полем (FormField), не Toast; пустое обязательное — «Заполните обязательное свойство «…».» под нужным полем; «Сохранить» неактивна без изменений; «Отмена» возвращает значения.

### План 41-23 — сборка раздела (2 пункта)

- [ ] **H23-1 · GroupsPage, GroupPanel, GroupMoveModal.** Под admin: «Группы» в сайдбаре между «Устройства» и «Акты»; создать группу «Создать группу» (PageHeader); выбрать в дереве GroupTree -> GroupPanel с вкладками «Состав»/«Свойства»/«История»; «Перенести…» из меню узла и затем кнопкой в панели. Ожидание: обе точки входа открывают один GroupMoveModal; тост «Перенесено: группа и N устройств»; дерево и счётчики обновились; вёрстка 35/65 как в «Места»; единственный скролл-регион внутри панели; одинаково в Tauri и браузере.
- [ ] **H23-2 · Роли и AccessDenied.** Под manager (отдельная сессия): раздел виден, можно создать группу и добавлять в состав, но GroupTypePanel без контролов правки и нет кнопки «Создать тип». Под employee — пункта нет в сайдбаре, по `#/groups` вручную — «Доступ запрещён».

### План 41-24 — место и история (2 пункта)

- [ ] **H24-1 · DeviceFormBody / PlacePicker.** «Устройства» -> форма правки устройства-члена группы «АРМ #3» с местом; затем члена группы без места; затем вне групп. Ожидание: у члена с местом PlacePicker неактивен, под ним «Место задаётся группой «АРМ #3»» со ссылкой, клик ведёт в «Группы» и выделяет группу; у члена без места и вне групп место редактируется как обычно; сохранение прочих полей члена группы проходит без ошибки.
- [ ] **H24-2 · MovementTimeline.** В «Группы» впервые «Перенести…» группу «АРМ #3» (создана БЕЗ места, несколько устройств) на место, затем ещё раз; в «Места» открыть попап устройства (PlaceEntityViewModal, вкладка истории — MovementTimeline); затем вкладка «История» самой группы (GroupPanel). Ожидание: строка таймлайна устройства — «в составе группы «АРМ #3»», имя — ссылка в «Группы»; в «Истории» группы — «перенос группы»; прежние строки («вручную», «актом №…») как раньше.

### План 41-25 — отчёт и массовый перенос (2 пункта)

- [ ] **H25-1 · ReportTable, печать.** Создать группу БЕЗ места, добавить устройства, впервые перенести на место (D-21: строки группы в журнале нет), затем перенести ещё раз; «Отчёты» -> «Перемещения» за период; раскрыть и свернуть строку пакета шевроном; применить фильтр «тип устройства»; напечатать отчёт из десктопа и из LAN-браузера. Ожидание: пакет — ОДНА свёрнутая строка «АРМ #3 (N устройств)» с датой/откуда/куда/причиной «перенос группы»; шеврон раскрывает строки устройств; при фильтре и для пакета первого размещения показывается тот же заголовок пакета; на БУМАГЕ — ВСЕ строки пакета независимо от раскрытия на экране, шевронов нет, многостраничность не ломается (расхождение «экран vs бумага» сознательное, D-27).
- [ ] **H25-2 · PlaceContents.** Место «Склад А» с группой «АРМ #3» (место «Склад А») и парой одиночных устройств: «Места» -> «Склад А» -> «Перенести всё содержимое в…». Ожидание: в модалке «Перенести содержимое в другое место?» после прежнего текста — «Вместе с содержимым переедут 1 группа и весь их состав.»; после подтверждения тост «Перенесено: N устройств и 1 группа»; группа и все её устройства в новом месте (раздел «Группы»), одиночные тоже; для места без групп модалка и тост прежние; счётчики дерева «Мест» обновились.

Итого 17 пунктов: 2 + 3 + 3 + 3 + 2 + 2 + 2.

### Два сквозных сценария

- [ ] **S-А · Перетаскивание свойства.** GroupTypePropertiesTable: перетащить свойство, отменить Esc/`pointercancel`, пройти тем же порядком клавиатурой («Выше»/«Ниже») — на Tauri и в LAN-браузере, по возможности на WebView2 (покрывается H20-2; здесь — как единый проход на всех трёх движках).
- [ ] **S-Б · Перенос группы до печати.** GroupMoveModal -> тост «Перенесено: группа и N устройств» -> счётчики «Мест» -> отчёт «Перемещения» (ReportTable, раскрытие пакета) -> печать полного состава (покрывается H23-1, H25-1; здесь — одним сквозным проходом).

---

## Validation Audit 2026-10-06

Аудит `/gsd-validate-phase 41` (состояние A: `41-VALIDATION.md` существовал, карта заполнена).

| Metric | Count |
|--------|-------|
| Gaps found | 6 |
| Resolved | 4 (S1-S3 — записи, G1 — новый гейт) |
| Escalated | 2 (G2, G3 — в Manual-Only/долг, зелёными быть не могут) |

### Что перепроверено на HEAD своими руками (а не по отчётам)

| Проверка | Результат |
|----------|-----------|
| Наличие 19 названных тестовых файлов (6 `trackly-infra`, 13 `trackly-app`) | все существуют |
| 7 гейтов фазы (`check-group-vocabulary`, `check-reorder`, `check-groups-section`, `check-action-menu-portal`, `check-property-removal`, `check-report-truncation`, `check-place-tree-invalidation`), `--selftest` + прогон | exit 0 каждый |
| `cargo test -p trackly-app --test groups_types_service` | 39 passed, 0 failed (в т.ч. `protect_d_unarchive_clears_legacy_required_flag`) |
| Код на HEAD против коммита полного регресса `81f15b57` | `git diff --name-only` — только `.planning/**`, impl не менялся ⇒ результат «157 целей, 1624 пройдено, 0 падений» действителен для HEAD |

### Закрытые пробелы записи

- **S1** Sign-Off и «Что проверено машиной» утверждали «живые проверки НЕ пройдены» — исправлено, приёмка 2026-10-06 зафиксирована.
- **S2** Правка WR-01 (`f027cc0b`, после волны) в карте отсутствовала: добавлена строка `41-WR01`, зафиксированы замена теста и послеправочный полный регресс (157/1624, собран из трёх частей — фоновая команда резалась по лимиту времени, см. `41-REVIEW-gap-wave.md`).
- **S3** Manual-Only перечисляла поведения как ожидающие человека — добавлен статус живой приёмки и честная оговорка про Windows WebView2.
- Побочно: в `41-HUMAN-UAT.md` счётчик `passed` был 22 при 23 фактических `result: pass` — исправлен на 23.

### G1 — закрыт автоматикой

**Пробел:** сквозного гейта «команда Tauri ↔ HTTP-маршрут» в проекте не было. Существующие тесты полноты (`role_endpoint_matrix.rs`: `group_types_http_route_completeness`, `groups_http_route_completeness`) скоупнуты на `src/http/group_types.rs` и `src/http/groups.rs` — пропуск в другом модуле они увидеть не могут. Из-за этого `/api/v1/reports_get_report_counts` отсутствовал с фазы 28 и вскрылся только живой приёмкой R5.

**Чем закрыт:** `crates/trackly-app/tests/http_route_parity.rs` — 5 тестов, `cargo test -p trackly-app --test http_route_parity` → **5 passed, 0 failed, 0.30 с**.

| Тест | Что ловит |
|------|-----------|
| `every_tauri_command_is_reachable_over_http` | команда Tauri без `/api/v1/<name>` |
| `every_http_api_route_maps_to_a_tauri_command` | маршрут без одноимённой команды |
| `every_dual_transport_build_helper_has_an_http_caller` | `build_*` без вызывающей стороны в `src/http/*.rs` |
| `live_router_answers_every_tauri_command_path` | путь объявлен, но роутер не смерджен в `build_router()` |
| `verdict_tables_are_well_formed` | освобождение без вердикта, дубли, переехавший источник истины |

**Инвентарь — от исходников, не от списка имён:** команды — `collect_commands![…]` в `src/specta_export.rs` (184; это и есть число из записи UAT, а не 181 `pub async fn build_*` — соответствие не один-к-одному: 16 команд через обёртку `build_<name>_tauri`, 3 без хелпера вовсе); маршруты — вызовы `.route("/api/v1/<name>", …)` по `src/http/*.rs` (185 вызовов, 183 уникальных пути; `auth_login`/`auth_status` регистрируются дважды).

**Множество «команда без маршрута» фиксируется точно, с вердиктом на каждую позицию:** `app_restart`, `settings_move_db` (освобождены: десктопные, закрыты `isTauri` в `StorageSettings.svelte:41-47`), `settings_open_db_folder` (освобождён по ДРУГОЙ причине — см. находку 1 ниже), `reports_get_report_counts` (**известный дефект, не освобождение**; при добавлении маршрута строку удалить, гейт специально покраснеет). Обратное направление зафиксировано тоже: `auth_ad_sso`, `ws`, `users_reset_password`.

**Защита от вакуумности:** нижние границы инвентарей (≥150 команд, ≥150 маршрутов, ≥15 файлов в каталоге) — поломка регулярки даёт красный, а не зелёный; нераспознанная строка в `collect_commands!` валит тест; расхождения сверяются точно в обе стороны (и новая позиция, и мёртвое освобождение красят); живой тест сравнивает ответ не с «200 text/html», а с контрольным ответом на заведомо несуществующий путь, поэтому переживёт починку `spa_fallback`.

**Мутационная проверка.** Исполнитель прогнал 5 мутантов (удаление реального маршрута; фиктивный `build_*`; фиктивная команда в `collect_commands!`; лишняя строка в `CMD_WITHOUT_ROUTE`) — все покраснели, якоря проверены на уникальность, все откачены. Оркестратор **перепроверил сам, независимо**: удалил строку `.route("/api/v1/devices_list", post(handler_list))` из `crates/trackly-app/src/http/devices.rs:415` (`grep -c` = 1, `git diff` — `1 deletion`) → гейт **упал**, exit 101, 2 теста из 5, сообщения: «команды Tauri без HTTP-маршрута …: devices_list» и «живой `build_router()` отвечает … status 200, content-type "text/html; charset=utf-8"» — то есть тест воспроизвёл механизм дефекта R5 дословно. Файл восстановлен побайтово (`diff -q` чисто).

### G2, G3 — escalated, автоматикой не закрываются

| # | Пробел | Почему не закрыт |
|---|--------|------------------|
| G2 | `spa_fallback` (`src/http/mod.rs:252`) отдаёт `index.html`/200 на любой несуществующий `/api/`; теста на 404 JSON нет | тест был бы красным: дефект существует, починка отложена решением пользователя 2026-10-06 вне границ фазы 41. Косвенно отказ теперь заметен — `live_router_answers_every_tauri_command_path` ловит сам симптом |
| G3 | IN-03: `format!("{summary}. {notice}")` (`tauri_cmds/reports.rs:402-411`) даёт «..», если путь места или имя типа кончается точкой | покрытие требует вынести чистую функцию, то есть правку impl — аудитору запрещена. Достижимо только пользовательскими данными с точкой на конце; косметика |

### Параллельная правка гейта Req 11 (не этим аудитом)

Пока шёл аудит, в `main` пришла отдельная сессия `/gsd-quick` `261006-svt` (коммиты `ef87ff3c`, `ccb04509`): счётный гейт write-site'ов `devices.place_id` в `crates/trackly-app/tests/group_write_sites.rs` расширен с трёх методов до полного реестра из 8 методов `SqliteDeviceRepository`, у каждого закодирован вердикт (ReleaseRequired / BirthExempt / NoProductionCallers), добавлены мутационные самопроверки и гейт парности `update_in_tx`. Прогон: **30 passed, 0 failed**; продакшн-код не менялся. Строка Req 11 в карте выше («`group_write_sites` S1–S9 + счётный гейт») тем самым усилена — читать вместе с `.planning/quick/261006-svt-write-site-devices-place-id-group-write-/261006-svt-SUMMARY.md`. Там же зафиксировано, что первая версия эвристики «пропал write» была вакуумной (`place_id` встречался в сигнатуре и `params!`) и исправлена на разбор только SQL-литерала.

### Находки аудита вне границ фазы (требуют решения пользователя)

1. **`settings_open_db_folder` не закрыт `isTauri`.** Допущение из записи UAT («три прочих десктопные по смыслу и закрыты проверкой `isTauri`») проверку не прошло: `openFolder()` (`ui/src/features/settings/StorageSettings.svelte:29-39`) гарда не имеет, кнопка «Открыть папку с базой данных» рисуется и в браузере (`:103-105`). `isTauri` (`:41-47`) стоит только в `proceedWithMove()` и закрывает `settings_move_db` + `app_restart`. Освобождение в гейте оставлено с честным вердиктом: каталог открывается на машине сервера, для LAN-клиента операция бессмысленна, а отказ **виден** тостом «Не удалось открыть папку» (в отличие от пустого `.catch(() => {})` у счётчиков). Гард/скрытие кнопки — доработка UI.
2. **`users_reset_password` — HTTP-only сирота.** `handler_reset_password` и `build_users_reset_password` есть (`src/http/users.rs`), но ни команды Tauri, ни вызывающей стороны в `ui/src` нет: админский сброс пароля недостижим из интерфейса на обоих транспортах. Решить: доделать UI или удалить маршрут.
