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
| _(заполняется после создания PLAN.md — ID задач ещё не существуют)_ | | | | | | | | | ⬜ pending |

**Требование к заполнению:** каждая задача плана обязана сослаться на требование SPEC (1–14) и на строку
из таблицы «Phase Requirements → Test Map» ниже. Карта не остаётся пустой при закрытии фазы —
это зафиксированный повторяющийся дефект прошлых фаз (40.4, 40.5).

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
