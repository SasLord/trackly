---
phase: 41-groups-model-and-editor
plan: 34
subsystem: validation-boundary
tags: [regression, privacy, uat-tracking, live-checks, gap-closure]
requires: ["41-31", "41-32", "41-33"]
provides:
  - "один полный регресс границы волны 41-27…41-33: все семь шагов CI-цепочки с кодом 0"
  - "оба UAT-гэпа в состоянии fixed_pending_reverify (не resolved, не pass)"
  - "единый список живых проверок R1-R5 (пункты 20-24 в 41-HUMAN-UAT.md) на обоих транспортах"
  - "Per-Task Verification Map для 41-27…41-34 (19 строк) и раздел результатов прогонов"
affects: [41-UAT.md, 41-HUMAN-UAT.md, 41-VALIDATION.md]
tech-stack:
  added: []
  patterns: ["статус гэпа fixed_pending_reverify вместо resolved до живой проверки", "пункты живой проверки в формате, который читает uat.cjs (### N. + однострочный expected + result: pending)"]
key-files:
  created: []
  modified:
    - .planning/phases/41-groups-model-and-editor/41-UAT.md
    - .planning/phases/41-groups-model-and-editor/41-HUMAN-UAT.md
    - .planning/phases/41-groups-model-and-editor/41-VALIDATION.md
key-decisions:
  - "Статус гэпа — fixed_pending_reverify: инструментарий GSD (templates/UAT.md, execute-phase.md, verify-work.md, diagnose-issues.md) знает только failed и resolved, значения «исправлено, не перепроверено» в нём нет; resolved/pass/closed запрещены как означающие «проверено»"
  - "Пункты R1-R5 пронумерованы 20-24 в формате uat.cjs, чтобы /gsd-progress и /gsd-audit-uat их видели; свободный раздел плана 41-27 свёрнут в R1/R2"
  - "В 41-HUMAN-UAT.md issues: 0 и fixed_pending_reverify: 1 (а не issues: 1 + ещё одна строка): иначе сумма не сходится с total. Проблема не снята живой проверкой — она учтена отдельной строкой, не в passed"
requirements-completed: [GRP-01, GRP-02, GRP-03, GRP-04, GRP-06, GRP-09]
duration: ~1 ч 40 мин (из них около 80 мин — полный cargo-прогон)
completed: 2026-10-05
---

# Фаза 41 План 34: граница догоняющей волны 41-27…41-33 Summary

Автоматическая граница волны закрыта: все семь шагов CI-цепочки зелёные на полном дереве (158 бинарей, 1625 passed, 0 failed); два UAT-гэпа переведены в «починено, ожидает живой перепроверки», а ни одна живая проверка не пройдена — пять пунктов R1-R5 отданы пользователю.

## Коммиты

- `b01e4f7e` — трекинг UAT, единый список живых проверок, карта валидации (задачи 2 и 3; задача 1 кода не меняла)

## Задача 1: полный регресс, в порядке CI, последовательно

Рабочее дерево перед цепочкой чистое. Базовый коммит волны — `1f82b55b`, HEAD — `e09fd295`. Каждый шаг записан отдельно, один `cargo` за раз.

| # | Шаг | Код | Результат |
|---|-----|-----|-----------|
| 1 | `node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256` | 0 | PASS, 0 нарушений |
| 2 | `pnpm --dir ui build` | 0 | собран; `git status` по `ui/src` и `bindings.ts` пуст (контракт DTO не менялся) |
| 3 | `rustfmt --check` по 7 `.rs`-файлам, изменённым волной | 0 | чисто |
| 4 | `cargo clippy --workspace --all-targets -- -D warnings` | 0 | чисто (по кэшу: исходники с последнего clippy не менялись) |
| 5 | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test --workspace --no-fail-fast -- --test-threads=1 --skip login_remember_persistent_cookie` | 0 | **158 бинарей, 1625 passed, 0 failed, 5 ignored, 4779 с (около 80 мин)** |
| 6 | `pnpm --dir ui svelte-check` | 0 | 311 файлов, 0 ERRORS, 68 WARNINGS (база прежняя) |
| 7 | `pnpm --dir ui lint` | 0 | все гейты, включая `check-action-menu-portal` (selftest 10 фикстур), `check-property-removal` (7 мутантов, 6 кейсов), `check-report-truncation` (8 мутантов, 8 кейсов), `check-groups-section`, `check-reorder`, `check-group-vocabulary` |

Замечания по прогону:

- **Время.** Полный `cargo test --workspace` занял около 80 минут, а не ~8, на которые рассчитывал план (в 41-26 было около 8 минут на тёплом `target/`). Прогон запускался в фоне, вывод — сырым в файл, без конвейера через `grep`. В этот раз сборка тестов шла заново.
- **Шаг 3 — fmt только по файлам волны.** Полный `cargo fmt --all -- --check` не запускался: в `crates/trackly-app/src/dto/act.rs` и ряде тестов давний дрейф, не относящийся к волне. Сказано явно: это не полный fmt-гейт.
- **Шаг 4.** Clippy отработал за секунду по кэшу — после последнего зелёного clippy исходники не менялись; заново не пересобирался.
- **Предсуществующий пропуск:** `login_remember_persistent_cookie` (подвисание) — `--skip`, как и в 41-26. Волной не вводится.
- В полном прогоне зелёные цели волны: `groups_types_service` 39 (из них `atomic_*` 10, `protect_d_*` 3), `report_movements_truncation` 7, `groups_property_removal_parity` 1, `group_report_batch` 10, `group_types_repo` 14. Против 41-26: +2 бинаря, +21 тест.
- `git status` после цепочки: изменены только файлы трекинга этого плана.

## Задача 2: приватность

- Гейт `check-privacy.mjs` — код 0 (до коммита, после `git add`).
- Ручной греп добавленных волной строк (`git diff 1f82b55b..HEAD -U0`: около 3400 строк кода и тестов и около 3400 строк `.planning/`): ФИО (инициалы и «Фамилия Имя Отчество»), e-mail, телефоны, IP, MAC, ИНН/КПП/ОГРН/ОКПО, организационно-правовые формы, адреса. В коде и тестах совпадений нет; в `.planning/` — только вымышленные значения. Совпадений вне допустимого вымышленного класса — 0. Один набор инициалов («Кузнецов К.К.») не входит в узкий список из плана, но это вымышленное имя, давно используемое в тестах предыдущих фаз.
- Файлы волны из списка плана (три Rust-теста, обе `cases.json`, три `.mjs`-гейта, `propertyRemoval.ts`, `truncationNotice.ts`) проверены тем же способом: реальных имён нет.
- SUMMARY планов 41-27…41-33 и этот план прочитаны на цитаты живой БД, логов и вывода запросов: не найдено (упоминания «копия dev-БД» — описание процедуры).
- Найдено 0 случаев, удалять было нечего. **Граница гарантии:** проверен диф волны, не вся история репозитория. Скрипт-гейт ФИО в обычном тексте не ловит — поэтому ручной греп.

## Задача 3: трекинг UAT и живые проверки

- **Значение статуса.** Поиск по `~/.claude/get-shit-done` (`templates/UAT.md`, `workflows/execute-phase.md`, `verify-work.md`, `diagnose-issues.md`) показал, что инструментарий знает только `failed` и `resolved`; значения «исправлено, не перепроверено» нет. Выбрано `fixed_pending_reverify` (как предписывает план). `resolved`, `pass`, `closed` не использованы. Побочный эффект, полезный здесь: шаг `execute-phase.md` «failed -> resolved для гэпов закрывающей фазы» теперь не найдёт `status: failed` и ничего не закроет сам.
- **41-UAT.md:** оба гэпа — `fixed_pending_reverify`, `fixed_by` (`41-27`; `41-31` с побочными `41-29`, `41-28`), `reverify` со ссылкой на 41-HUMAN-UAT.md. Поля `reason`, `root_cause`, `artifacts`, `missing` не тронуты (в диффе только добавленные строки и смена `status`). Тест 5 остаётся `result: issue` с припиской; в Summary — `fixed_pending_reverify: 1` как подмножество `issues: 1`.
- **41-HUMAN-UAT.md:** пункт 3 (H20-1) — `result: fixed_pending_reverify` с `note`; заголовок пункта 3 заменён ссылкой «переписан в 41-31…»; в пункте 5 (H20-3) просьба решить вопрос копирайта заменена на «Копирайт решён в 41-28»; результат `pass` пункта 5 не менялся; оба гэпа — `fixed_pending_reverify` + `fixed_by`.
- **Единый список живых проверок** — разделом «Повторная проверка после волны 41-27…41-33» (пункты 20-24, R1-R5), все пять `result: pending`, ни одного `pass`. Пункты названы по существующим компонентам: R1 — ActionMenu (все 13 вызовов поштучно по файлам и строкам: GroupTypePropertiesTable, GroupTreeNode ×2, PlaceTreeNode, GroupContentsTable ×2, GroupPrintersList, DevicesPage, DeviceFormModal, OrgSettings, NumberTemplateField, ButtonsSection ×2; число вызовов сверено грепом по `ui/src`), R2 — поведение ActionMenu (фокус, Esc, Tab, z-порядок), R3 — GroupTypePanel, R4 — GroupTypePropertiesTable (H20-1 в трёх частях), R5 — ReportsPage (баннер, печать из десктопа и LAN, CSV). Преамбула требует оба транспорта (`cargo tauri dev` с `main`, LAN-браузер после `pnpm --dir ui build`), консоль на `effect_update_depth_exceeded`, dev-БД или копию. Отдельно названо, что W-B02 и W-B01 закрыты автотестами, а не живой проверкой. Свободный раздел «Открытые проверки плана 41-27» удалён (его семь пунктов свёрнуты в R1/R2).
- **Счётчики `## Summary` приведены к фактам.** Было: `total: 19, passed: 18, issues: 1, pending: 0` при висящем `result: [pending]` и семи пунктах плана 41-27 в свободном разделе, которых никто не считал. Стало: `total: 24, passed: 18, issues: 0, fixed_pending_reverify: 1, pending: 5` (18 + 1 + 5 = 24). `pending` — ровно пять строк `result: pending`.
- **41-VALIDATION.md:** в карту добавлены 19 строк (41-27-T1 … 41-34-T3), колонка «Threat Ref» сверена с реестрами планов. Статус green — только у строк, чьи команды реально зелёные в шаге 5-7 или в узких проверках (grep-проверки 41-28-T2 и 41-31-T3 выполнены отдельно); у семи строк с живой частью — «✅ green (авто); ⬜ ожидает живой перепроверки» (в файле фраза «ожидает живой перепроверки» встречается 8 раз: 7 строк карты и пояснение, не менее 4). Добавлен раздел «Результаты прогонов (план 41-34…)» и указатель в разделе живых проверок. `nyquist_compliant` не менялся.
- **41-VERIFICATION.md** и планы 41-01…41-33 не затронуты.

## Deviations from Plan

### Auto-fixed Issues

Нет багов кода — план кода не менял.

### Отклонения от текста плана

**1. [Формат] Пункты R1-R5 пронумерованы 20-24 и оформлены как `### N.` с однострочным `expected` и подробностями ниже**
- **Причина:** парсер `uat.cjs` распознаёт только блоки `### <число>. …` с `expected:` сразу под заголовком; заголовки «R1.» и многострочный `expected` остались бы невидимыми для `/gsd-progress` и `/gsd-audit-uat` (в точности та дыра, о которой предупреждает урок auto_advance).
- **Следствие:** вместо «пяти пунктов R1-R5» — пять пунктов 20-24 с метками R1-R5 в заголовках; содержание плана сохранено.

**2. [Счётчики] В 41-HUMAN-UAT.md `issues: 0`, а не «не уменьшать»**
- **Причина:** план велел оставить `issues` без изменений и добавить `fixed_pending_reverify: 1`; но пункт 3 в этом файле по тому же плану получает `result: fixed_pending_reverify` (не `issue`), и при `issues: 1` одно и то же считалось бы дважды, сумма не сходилась бы с `total`. Исходная формулировка про «не снимать проблему» сохранена смыслом: проблема не отнесена к `passed` и вынесена отдельной строкой с комментарием. В 41-UAT.md, где тест 5 остаётся `result: issue`, `issues: 1` сохранён.

**3. [Полнота fmt] fmt проверен только по файлам волны** — как и предписано планом, но это не полный fmt-гейт CI (в нём давний дрейф).

### Отклонения волны, зафиксированные в границе (не этого плана)

- 41-29: `archive_property` снимает `is_required` тем же UPDATE; `update_property` отвергает обязательность скрытого; защита при возврате — запасная для легаси-строк.
- 41-31: `crates/*/src` не менялся; паритет-тест на общей фикстуре.
- 41-32: rusqlite 0.38 (в CLAUDE.md указано 0.39), у `Transaction` нет `DerefMut` — тела мутаторов вынесены в `*_on(&self, conn, …)`. Критерий плана `grep -c "conn.savepoint()" == 1` НЕ выполнен (0). Отмечено в карте валидации, не затушёвано.
- Оркестратор (`e09fd295`): 8-й кейс «500 из 800» в `report-truncation/cases.json` против мутанта `total <= 1000`; selftest гейта зелёный, `report_movements_truncation` 7/7.
- 41-30: тест `report_trunc_under_limit_total_equals_rows` — охранный (зелёный и до, и после правки), заявлено открыто.

## Authentication Gates

Нет.

## Known Stubs

Нет.

## Threat Flags

Нет новой поверхности: план менял только документы трекинга.

## Что осталось за человеком

Живая перепроверка R1-R5 (пункты 20-24 в `41-HUMAN-UAT.md`) на десктопе (`cargo tauri dev` с `main`) и в LAN-браузере (после `pnpm --dir ui build`). Только после неё `/gsd-verify-work` может перевести оба гэпа из `fixed_pending_reverify` в `resolved`. Ни одна живая проверка этим планом не проведена и не отмечена пройденной.

## Self-Check: PASSED

- Файлы найдены: `41-UAT.md`, `41-HUMAN-UAT.md`, `41-VALIDATION.md`, `41-34-SUMMARY.md`.
- Коммит найден: `b01e4f7e`.
- `grep -c "status: failed"` по `41-UAT.md` и `41-HUMAN-UAT.md` — 0; `result: pending` в `41-HUMAN-UAT.md` — 5; `result: pass` в разделе R1-R5 — 0.
