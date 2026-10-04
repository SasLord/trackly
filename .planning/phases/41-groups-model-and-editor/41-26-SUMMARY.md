---
phase: 41-groups-model-and-editor
plan: 26
subsystem: validation
tags: [validation, regression, ci-chain, privacy, live-checks]
requires: [41-16, 41-23, 41-25]
provides:
  - "41-VALIDATION.md закрыт: карта по задачам заполнена фактами, nyquist_compliant/wave_0_complete выставлены, status: executed"
  - "список из 17 живых проверок (human-check планов 19-25) и 2 сквозных сценариев, отданный пользователю ДО /gsd-verify-work"
affects: []
tech-stack:
  added: []
  patterns:
    - "граница последней волны: целевые цели целиком, затем один полный workspace-прогон, затем цепочка CI-гейтов в порядке CI"
key-files:
  created: []
  modified:
    - .planning/phases/41-groups-model-and-editor/41-VALIDATION.md
key-decisions:
  - "nyquist_compliant: true описывает только автоматическую выборку; живые проверки не пройдены и не закрывались исполнителем"
  - "GRP-10 остаётся частичным (traceability Phase 41 + 41.1): запрет удаления устройства-якоря требует groups.anchor_device_id, которого V045 не создаёт"
requirements-completed: [GRP-01, GRP-02, GRP-03, GRP-04, GRP-05, GRP-06, GRP-07, GRP-08, GRD-06]
completed: 2026-10-04
---

# Phase 41 Plan 26: автоматическая граница фазы и карта валидации Summary

Фаза 41 прошла полный автоматический регресс (156 бинарей, 1604 теста, 0 упавших), цепочку CI-гейтов в порядке CI и проверку приватности; 41-VALIDATION.md заполнен фактами, а 17 живых проверок собраны для пользователя и НЕ закрыты.

## Tasks

| Task | Name | Commit |
|------|------|--------|
| 1 | Целевые цели, затем один полный прогон workspace | без правок файлов (только прогоны) |
| 2 | Цепочка UI-гейтов в порядке CI и проверка приватности | без правок файлов (только прогоны) |
| 3 | Заполнение Per-Task Verification Map, сбор живых проверок | 1f82b55b |

Задачи 1 и 2 по плану ничего не меняют в репозитории: результаты зафиксированы в разделе «Результаты прогонов» документа 41-VALIDATION.md.

## Результаты

**Задача 1 — бэкенд.**
- core: 149 passed. infra: `groups_migration` 4, `group_types_repo` 14, `groups_repo` 18, `place_movements_batch_repo` 6, `per_record_invariants` 3, `migration_idempotency` 2.
- app, новые цели: `groups_types_service` 26, `groups_service` 15, `groups_move` 10, `group_movements_journal` 5, `groups_membership` 14, `groups_values_card` 19, `group_write_sites` 23, `group_report_batch` 10, `places_move_groups` 6, `place_movements_group_fields` 5, `role_endpoint_matrix` 12.
- app, регресс write-site'ов целиком: 22 цели (`acts_*`, `place_movements_*`, `places_*`, `devices_crud`, `report_*`, `html_*`, `cartridges_lifecycle`) — все exit 0, 0 failed (полный перечень с числами — в VALIDATION).
- `cargo clippy --workspace --all-targets -- -D warnings` — exit 0. `cargo fmt` на 69 `.rs`-файлах фазы — чисто.
- Полный прогон `cargo test --workspace --no-fail-fast -- --test-threads=1 --skip login_remember_persistent_cookie`: exit 0, 156 бинарей, **1604 passed, 0 failed, 5 ignored**, около 8 минут (запущен в фоне и дождан в рамках этого плана; параллельно cargo не запускался). Совпадает с прогоном оркестратора на c2c31640.
- Следов мутаций нет: `git status` пуст, `grep` по сигнатурам мутаций пуст.

**Задача 2 — UI и приватность (порядок CI).** `export_bindings` 1 passed; `svelte-check` 309 файлов, 0 ERRORS / 68 WARNINGS; `pnpm lint` exit 0 (vocabulary 20 фикстур, 212 файлов; reorder 22 кейса, 3 мутанта; groups-section 6 фикстур; INV-7 0 нарушений); `pnpm build` exit 0; `check-privacy.mjs` PASS.

**Ручной просмотр приватности.** Просмотрен весь `git diff 0b7239b8^..HEAD` (141 файл, около 30 тыс. добавленных строк: планы и сводки фазы, тесты, исходники, миграции, `ui/src`, фикстуры) на ФИО, e-mail, телефоны, IP, MAC, реквизиты, адреса, организационные формы. Найдено только вымышленное («Иванов И.И.», «Петров П.П.», «Сидоров С.С.», «Иванова И.И.», «Иванова А.А.», «Иванкин К.К.»; 10.0.0.x, 192.168.1.10; тестовые MAC). Реальных данных не найдено, удалять нечего. Граница: просмотрен диф фазы, а не вся история.

**Задача 3 — документ.** Все строки карты — green по автоматической команде (в 16 из 59 строк с пометкой «human» дописано «живая проверка открыта»); 0 строк pending; frontmatter `status: executed`, `nyquist_compliant: true`, `wave_0_complete: true`; ссылки Test Map сверены с реальными именами целей (`ls crates/*/tests`); добавлены разделы: результаты прогонов, покрытие требований, заранее существовавшие условия, что проверено машиной и что нет, отклонения от UI-SPEC (принято/требует решения), «Живые проверки для пользователя» (17 пунктов 2+3+3+3+2+2+2 с предусловиями, существующими компонентами UI и обоими транспортами, плюс сценарии А и Б). Предусловий «добавить свойства АРМ вручную» нет (D-31).

## Что требует решения или действия пользователя

- **Живые проверки (17 + 2 сценария) не пройдены.** Рантайм и вид раздела «Группы» в Tauri WKWebView и LAN-браузере машиной не проверены; Playwright-прогон плана 41-23 по правилу проекта не верификация. Список — в конце 41-VALIDATION.md.
- **GRP-10 остаётся `[~]` частичным** (Phase 41 + 41.1): не отмечен выполненным. Вторая половина — запрет удаления устройства-якоря — уходит в фазу 41.1 (нужен `groups.anchor_device_id`, V045 его не создаёт).
- **Копирайт для manager** в `GroupTypePanel` («Код и поведение типа изменить нельзя. Название и набор свойств — можно.») звучит как разрешение, которого нет — решить при верификации (UI-SPEC 9.1); также мелкий визуальный вопрос про подпись «уже в группе «…»» в мета-слоте Dropdown.

## Заранее существовавшие условия (не регрессии фазы 41)

- Тест `login_remember_persistent_cookie` зависает; все прогоны шли с `--skip`. CI-шаг `cargo test` флага `--skip` не содержит — отдельный долг проекта.
- Дрейф `cargo fmt --check` в `crates/trackly-app/src/dto/act.rs` и нескольких тестовых файлах; фаза их не трогала, не «чинился».

## Deviations from Plan

None - plan executed exactly as written. (Полный прогон workspace занял около 8 минут вместо оценочных 80; запускался в фоне с опросом, а не в одном синхронном вызове — из-за 10-минутного предела инструмента; другие cargo-команды в это время не запускались.)

## Known Stubs

None.

## Threat Flags

None.

## Self-Check: PASSED

- 41-VALIDATION.md существует; `grep -c pending` = 0; 17 пунктов `H..-N`; раздел «Живые проверки для пользователя» найден.
- Коммит 1f82b55b существует.
