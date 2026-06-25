---
phase: 12-cartridge-request-interconnection
verified: 2026-06-23T05:30:00Z
status: superseded
score: 11/11 must-haves verified (programmatically — BLOCKER и WARNING устранены коммитом 8efeadd); 2 human-verification items still pending from original run
overrides_applied: 0
re_verification:
  previous_status: human_needed
  previous_score: 9/9 (programmatic) + 3 human-verification items
  gaps_closed:
    - "GAP-12-01: общий автокомплит имён (acts + cartridges.holder_name) — D-09/D-10"
    - "GAP-12-02 backend: junction printer_cartridge_models + self-adjusting фильтр совместимости — D-11..D-15a"
    - "GAP-12-02 frontend часть A: чекбокс-редакторы на PrinterDetail/ModelFormModal — D-12"
    - "GAP-12-02 frontend часть B: install picker использует compatible_with_printer_device_id + warning — D-13/D-14"
    - "GAP-12-03 backend: авто-возврат предыдущего картриджа на склад в той же транзакции — D-16..D-19"
    - "GAP-12-03 frontend: редактируемый блок «Предыдущий картридж» в OperationModal — D-16"
  gaps_remaining:
    - "Human-verification тест 2 (DISC-02 empty-state) из исходной верификации — статус остался pending, живой прогон не выполнен"
    - "Human-verification тест 3 (D-08 regression) из исходной верификации — статус остался pending, живой прогон не выполнен"
  regressions:
    - "[RESOLVED 8efeadd] cargo build/test --workspace не компилировались: cartridges_history.rs:70/:112 не имели полей Install { previous_cartridge_state_id, previous_cartridge_location } из 12-09 (4cc9500). Оба сайта дополнены None; cargo build --workspace --tests зелёный, cargo test cartridges_history → 2 passed."
    - "[RESOLVED 8efeadd] svelte-check ERROR в CartridgesPage.svelte:60 — CartridgeFilter literal не содержал compatible_with_printer_device_id (поле из 12-05). Добавлено compatible_with_printer_device_id: null; svelte-check → 0 errors (36 пред-существующих warnings)."
gaps_resolved_inline:
  - "Воркспейс компилируется и весь набор тестов проходит: cargo build --workspace --tests зелёный после 8efeadd; cartridges_history (ранее ломавший компиляцию) → 2 passed."
  - "svelte-check проходит без ошибок (0 errors) после 8efeadd."
human_verification:
  - test: "DISC-02 empty-state: открыть «Установить картридж» из заявки на модель, для которой на складе нет подходящих картриджей (с учётом теперь активного фильтра совместимости из GAP-12-02)"
    expected: "Список показывает «Нет подходящих картриджей на складе»; форма не блокируется (модалку можно закрыть, заявку отклонить, либо использовать старый cartridge-centric вход)"
    why_human: "Перенесено без изменений из исходной верификации (2026-06-22) — статус в 12-HUMAN-UAT.md остался [pending], живой прогон так и не выполнен ни во время gap-closure, ни после. Код-путь не менялся в части самого empty-state рендера (CartridgeSelect.svelte), но взаимодействие с новым D-13/D-14 фильтром не проверено интерактивно."
  - test: "D-08 regression: открыть карточку картриджа со статусом «На складе» напрямую (меню картриджа → «Установить в принтер») — теперь также с учётом нового блока «Предыдущий картридж» (D-16/12-09) и сравнения с GAP-12-02 фильтром"
    expected: "Старая форма работает без изменений: без нового селектора картриджа, без блока «Предыдущий картридж» (картридж уже известен из контекста меню, currentPrinterDeviceId не определён через cartridgeModelId/preFillPrinterId этим путём) — расположение полей, фокус, скролл не регрессировали"
    why_human: "Перенесено без изменений из исходной верификации — статус в 12-HUMAN-UAT.md остался [pending]. Дополнительно теперь требует визуальной проверки, что новый блок «Предыдущий картридж» из 12-09 НЕ появляется в cartridge-centric входе (код проверен статически — эффект гейтирован на cartridge===null && preFillPrinterId!==undefined — но живая сессия не прогонялась)."
---

# Phase 12: Взаимосвязь картриджной заявки — Verification Report (re-verification после gap-closure)

**Phase Goal:** Сделать установку картриджа из заявки «Замена картриджа» полнофункциональной и взаимосвязанной: выбор физического картриджа из БД (на складе, заряд Полный/Частичный, совместимый с моделью заявки), авто-подстановка Расположения из принтера и «Кому отдал» из заявителя (оба редактируемы), запись установленного картриджа в `completed_cartridge_id` заявки и отражение в истории. Старый cartridge-centric вход сохраняется. ПЛЮС закрытие 3 UAT-gaps (GAP-12-01..03) через планы 12-04..12-09.

**Verified:** 2026-06-23T05:30:00Z
**Status:** human_needed (BLOCKER и WARNING устранены коммитом 8efeadd; остались только 2 живых human-verify пункта)
**Re-verification:** Да — после исполнения 6 gap-closure планов (12-04..12-09) и inline-фикса двух интеграционных регрессов (8efeadd).

## Контекст ре-верификации

Исходная верификация (2026-06-22) дала `human_needed`: все 9 запланированных истин D-01..D-08 (+RBAC) подтверждены кодом, но 3 пункта (живой UI-прогон: happy path, DISC-02 empty-state, D-08 regression) требовали интерактивной человеческой сессии. `12-HUMAN-UAT.md` зафиксировал реальный человеческий прогон: тест 1 (happy path) выполнен и нашёл 3 проблемы (GAP-12-01/02/03), тесты 2 и 3 остались **pending** (не выполнены, ни тогда ни сейчас).

Шесть планов закрытия гэпов (12-04..12-09) исполнены. Эта ре-верификация:
1. Полностью (3-уровневая проверка: существует/содержательно/связано) перепроверила всё, что относится к GAP-12-01/02/03 закрытию — не доверяя тексту SUMMARY.
2. Быстрой регрессионной проверкой подтвердила, что 9 истин D-01..D-08 из исходной верификации не сломаны.
3. **Нашла новую регрессию**, не упомянутую ни в одном SUMMARY: `cargo build/test --workspace` не компилируется.

Реально выполненные команды (а не переиспользованные результаты из SUMMARY):
- `cargo test -p trackly-app --test cartridges_lifecycle -- --test-threads=1` → 18 passed; 0 failed.
- `cargo test -p trackly-app --test acts_suggest -- --test-threads=1` → 10 passed; 0 failed.
- `cargo test -p trackly-app --test cartridges_crud -- --test-threads=1` → 9 passed; 0 failed (включая 3 новых `printer_compatib_*` теста).
- `cargo test -p trackly-app --test role_endpoint_matrix -- --test-threads=1` → ok, включая новые Case 33/34/35.
- `TRACKLY_AD_MOCK=1 cargo test --workspace` → **не компилируется** (E0063 в `cartridges_history.rs`).
- `cargo build --workspace --tests` → тот же сбой компиляции, подтверждён отдельно.
- `cargo clippy -p trackly-core -p trackly-app -p trackly-infra -- -D warnings` (lib-only, как в исходной верификации) → чисто.
- `cargo clippy --workspace --tests -- -D warnings` → падает на пред-существующий `len_zero` в `template_service.rs` (не относится к фазе 12) И на тот же компиляционный сбой `cartridges_history.rs`.
- `pnpm --dir ui exec svelte-check` → **1 ERROR** (не warning): `CartridgesPage.svelte:60` — `CartridgeFilter` literal не содержит `compatible_with_printer_device_id`. 243 файла проверено, 36 warnings (пред-существующие, не в файлах фазы).
- `pnpm --dir ui build` → собирается успешно (Vite/esbuild не делает строгую проверку типов — ошибка svelte-check не блокирует сборку, но остаётся реальным регрессом качества).
- Точечный serde-тест (написан и запущен мной, не из SUMMARY, затем откатан `git checkout`): подтверждено, что `Option<T>`-поле без `#[serde(default)]` всё равно десериализуется в `None` при отсутствии ключа JSON — то есть `CartridgesPage.svelte:60`'s проблема НЕ ломает runtime-десериализацию через HTTP/Tauri invoke, это чисто TypeScript compile-time несоответствие (specta генерирует required-поле, так как `#[serde(default)]` физически отсутствует на Rust-стороне).

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 (D-01..D-08, regression check) | 9 истин исходной фазы (выбор картриджа, авто-подстановка, completed_cartridge_id, история, старый вход, RBAC) не сломаны gap-closure работой | ✓ VERIFIED | `cargo test -p trackly-app --test cartridges_lifecycle/cartridges_crud/role_endpoint_matrix` зелёные (см. список выше); код `OperationModal.svelte`/`RequestDetail.svelte`/`request_service.rs` не показывает признаков отмены исходных правок (grep подтверждает все исходные паттерны на месте). |
| 2 (D-09/D-10, GAP-12-01) | Автокомплит «Кому выдал»/«Кто выдал» учитывает имена из `cartridges.holder_name`, не только `acts` | ✓ VERIFIED | `act_service.rs:1040-1140` — `suggest_person()` SQL расширен `UNION ALL` с веткой `cartridges.holder_name`, внешний `GROUP BY name, SUM(freq)` для дедупа/частоты. Коммит `7b4e966` подтверждён в `git log`. 10/10 тестов `acts_suggest.rs` зелёные, включая новые `suggest_person_dedupes_name_present_in_acts_and_cartridges`, `suggest_person_excludes_soft_deleted_cartridges`. |
| 3 (D-11..D-15a, GAP-12-02 backend) | Junction-таблица `printer_cartridge_models` + self-adjusting SQL-предикат сужают список картриджей по совместимости с принтером заявки | ✓ VERIFIED | `migrations/V029__printer_cartridge_models.sql` существует с ожидаемой схемой (FK+уникальный индекс); применяется чисто (подтверждено логом миграций в `role_endpoint_matrix` прогоне). `cartridges_sqlite.rs:1093-1094,1119-1120` несёт предикат `(?6 IS NULL OR NOT EXISTS (...) OR c.model_id IN (...))` в обоих COUNT/SELECT. 3/3 теста `printer_compatib_*` в `cartridges_crud.rs` зелёные. `printers_sqlite.rs` — все 4 метода (`set_compatible_models_in_tx`, `set_compatible_devices_in_tx`, `get_compatible_model_ids`, `get_compatible_device_ids`) присутствуют. RBAC: Case 33/34/35 в `role_endpoint_matrix.rs` подтверждают 403 для Employee на все 3 мутирующих/защищённых эндпоинта. |
| 4 (D-12, GAP-12-02 frontend часть A) | Связь принтер↔модель картриджа редактируема с обеих сторон (карточка принтера и карточка модели) | ✓ VERIFIED | `CompatibleModelsEditor.svelte` (новый, вшит в `PrinterDetail.svelte`) и `CompatibleDevicesEditor.svelte` (новый, вшит в `ModelFormModal.svelte`) подтверждены существующими и осмысленными (чек-лист с load/save через реальные API). Оба пишут в одну и ту же junction-таблицу — подтверждено трассировкой к одним и тем же backend-методам. |
| 5 (D-13/D-14, GAP-12-02 frontend часть B) | Install picker в `OperationModal.svelte` реально фильтрует по совместимости принтер↔модель, с warning-текстом при отсутствии связей, и всегда исключает фотобарабаны | ✓ VERIFIED | `compatible_with_printer_device_id: preFillPrinterId ?? null` подключён к вызову `cartridges.list()`; `kind_id: 1` хардкодом (не `null`); warning «Совместимость не задана — проверьте вручную» подтверждён в коде. Коммит `9ebc38e`. |
| 6 (D-16..D-19, GAP-12-03 backend) | При установке нового картриджа в принтер, где уже стоит картридж «В работе», прежний автоматически возвращается на склад в той же транзакции, с опциональным override состояния/расположения | ✓ VERIFIED | `cartridges_sqlite.rs::transition_in_tx` — полный блок auto-return прочитан построчно: ищет предыдущий картридж по `current_printer_device_id`+`status_id=2`, UPDATE с `unwrap_or(3)`/`unwrap_or("")` фоллбэками, отдельная audit_log запись. 18/18 тестов `cartridges_lifecycle.rs` зелёные, включая `install_auto_returns_previous_cartridge_in_same_printer`, `install_auto_return_uses_previous_cartridge_overrides_when_present`, `install_auto_return_falls_back_to_defaults_when_overrides_absent`, `auto_return_writes_return_to_stock_audit_entry`. |
| 7 (D-16, GAP-12-03 frontend) | `OperationModal.svelte` показывает редактируемый блок «Предыдущий картридж» (состояние заряда + расположение) при установке в принтер, где уже есть картридж «В работе» | ✓ VERIFIED | Состояния `previousCartridge`/`previousCartridgeStateId`/`previousCartridgeLocation` (строки 88-90), `{#if previousCartridge}` шаблонный блок (438-463) с `Select`+`LocationAutocomplete`, оба поля прокинуты в `buildPayload()` (216, 288-291) в `previous_cartridge_state_id`/`previous_cartridge_location`. |
| 8 (вытекающая истина) | Воркспейс компилируется и весь тестовый набор проходит без регрессий после всех 6 планов закрытия гэпов | ✗ **FAILED** | `cargo build --workspace --tests` / `cargo test --workspace` падают с E0063 в `crates/trackly-app/tests/cartridges_history.rs:70,112` — 2 сайта конструирования `CartridgeTransitionPayload::Install` не получили новые поля `previous_cartridge_state_id`/`previous_cartridge_location` из Plan 12-09 (коммит `4cc9500`). Это прямая регрессия, введённая работой над GAP-12-03, не пред-существующая проблема (12-06 ранее аккуратно обновил ровно эти же 2 сайта под `printer_device_id` — `git show b5e5e26` подтверждает). 12-09-SUMMARY.md заявляет «Updated all 10 existing Install {..} construction sites» только применительно к `cartridges_lifecycle.rs`; `cartridges_history.rs` не упомянут в файлах, изменённых коммитом `4cc9500` (`git show 4cc9500 --stat` подтверждает отсутствие). |
| 9 (вытекающая истина — качество фронтенда) | `pnpm --dir ui exec svelte-check` проходит без ошибок (ERROR), как было заявлено к завершению gap-closure | ✗ **FAILED** (warning-уровень, см. классификацию ниже) | Реальный прогон даёт **1 ERROR**: `CartridgesPage.svelte:60` — `CartridgeFilter` literal не содержит `compatible_with_printer_device_id` (поле появилось в 12-05, не имеет `#[serde(default)]` на Rust-стороне DTO, поэтому specta генерирует его как required в TS). Дважды зафиксировано в `deferred-items.md` (планами 12-07 и 12-08) как «принадлежит любому будущему плану, который коснётся `CartridgesPage.svelte`» — но ни один из планов 12-04..12-09 не коснулся этого файла, поэтому проблема осталась открытой несмотря на двукратное самостоятельное обнаружение исполнителем. Подтверждено эмпирически (написан и прогнан точечный serde-тест, затем откатан), что это НЕ ломает runtime-десериализацию (Option<T> без #[serde(default)] всё равно становится None при отсутствии ключа) — то есть `CartridgesPage.svelte`'s страница картриджей продолжает функционировать в браузере, но качество typecheck-гейта регрессировало. |

**Score:** 7/9 проверяемых истин (включая всю объединённую закрытую работу GAP-12-01/02/03) полностью подтверждены. 2 истины провалены: #8 (компиляция воркспейса — BLOCKER) и #9 (typecheck-чистота — WARNING, не функциональный сбой). Из исходных 9 истин D-01..D-08+RBAC ни одна не регрессировала по функциональности.

### Required Artifacts (gap-closure, 12-04..12-09)

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/trackly-app/src/services/act_service.rs` | `suggest_person()` UNION ALL с `cartridges.holder_name` | ✓ VERIFIED | Подтверждено построчным чтением, SQL точно соответствует плану. |
| `crates/trackly-app/tests/acts_suggest.rs` | Новые тесты дедупа/исключения soft-deleted cartridges | ✓ VERIFIED | 10/10 тестов зелёные. |
| `migrations/V029__printer_cartridge_models.sql` | Junction-таблица с FK + уникальный индекс | ✓ VERIFIED | Схема точно соответствует плану; применяется чисто в живых тестовых БД. |
| `crates/trackly-infra/src/repos/printers_sqlite.rs` | 4 новых метода (set/get compatible models/devices) | ✓ VERIFIED | Все 4 присутствуют по grep, используются в командах. |
| `crates/trackly-infra/src/repos/cartridges_sqlite.rs` | D-13/D-14 self-adjusting предикат в `list()`; D-16..D-19 auto-return блок в `transition_in_tx` | ✓ VERIFIED | Оба блока прочитаны полностью, логика корректна и протестирована. |
| `crates/trackly-app/src/dto/cartridge.rs` | `CartridgeFilter.compatible_with_printer_device_id`; `Install.printer_device_id/previous_cartridge_state_id/previous_cartridge_location` | ✓ VERIFIED (с оговоркой) | Поля присутствуют и форвардятся корректно. Оговорка: `compatible_with_printer_device_id` не имеет `#[serde(default)]`, что создаёт TS-required несоответствие (см. Truth #9). |
| `ui/src/features/printers/CompatibleModelsEditor.svelte` | Чек-лист моделей картриджей на карточке принтера | ✓ VERIFIED | Существует, вшит в `PrinterDetail.svelte`, load/save через реальные API. |
| `ui/src/features/cartridges/CompatibleDevicesEditor.svelte` | Чек-лист принтеров на карточке модели картриджа | ✓ VERIFIED | Существует, вшит в `ModelFormModal.svelte` (только edit-режим), load/save через реальные API. |
| `ui/src/features/cartridges/OperationModal.svelte` | D-13/D-14 install-picker фильтр + D-16 блок «Предыдущий картридж» | ✓ VERIFIED | Оба механизма подтверждены кодом, никаких заглушек. |
| `crates/trackly-app/tests/cartridges_history.rs` | Совместим с новой формой `CartridgeTransitionPayload::Install` после 12-09 | ✗ **MISSING UPDATE** | 2 сайта конструирования `Install {..}` (строки 70, 112) не обновлены — компиляция всего воркспейса падает. |
| `ui/src/features/cartridges/CartridgesPage.svelte` | Совместим с расширенным `CartridgeFilter` после 12-05 | ✗ **MISSING UPDATE** | Строка 60 не содержит `compatible_with_printer_device_id` — 1 ERROR в svelte-check; дважды задокументировано как deferred, никогда не закрыто ни одним из 6 планов. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `act_service.rs::suggest_person` | `cartridges.holder_name` (SQL) | `UNION ALL` ветка | ✓ WIRED | Подтверждено SQL-текстом и зелёными тестами. |
| `OperationModal.svelte` (install picker) | `cartridges_list` (compatible_with_printer_device_id) | `cartridges.list({compatible_with_printer_device_id: preFillPrinterId ?? null, ...})` | ✓ WIRED | Подтверждено grep+построчным чтением. |
| `CompatibleModelsEditor.svelte` / `CompatibleDevicesEditor.svelte` | `printer_cartridge_models` (junction) | 4 dual-transport команды → `SqlitePrinterRepository` методы | ✓ WIRED | Обе стороны редактирования пишут в ту же таблицу — подтверждено трассировкой реализации (не только сигнатур). |
| `cartridges_sqlite.rs::transition_in_tx` (Install) | auto-return UPDATE предыдущего картриджа | Тот же `tx` (одна транзакция) | ✓ WIRED | Подтверждено: поиск предыдущего, UPDATE, audit_log — всё внутри одной транзакции `transition_in_tx`. |
| `OperationModal.svelte` (previous-cartridge block) | `buildPayload()` install-ветка | `previous_cartridge_state_id`/`previous_cartridge_location` поля | ✓ WIRED | Подтверждено: значения из `$state` пробрасываются в payload только когда `previousCartridge !== null`, иначе `null` (не ломает старый вход). |
| `crates/trackly-app/tests/cartridges_history.rs` | `CartridgeTransitionPayload::Install` (текущая форма после 12-09) | Структурное конструирование `Install {..}` | ✗ **NOT WIRED** | Компилятор отклоняет файл — связь разорвана на уровне типов, не просто предупреждение. |
| `ui/src/features/cartridges/CartridgesPage.svelte` | `CartridgeFilter` (текущая форма после 12-05) | Структурное конструирование `activeFilter` объекта | ⚠️ **PARTIAL** | TypeScript отклоняет это как ERROR при svelte-check, но Vite/esbuild (используемый в `pnpm build`) не применяет строгую проверку типов — объект всё равно компилируется в JS и runtime-десериализация на Rust-стороне толерантна к отсутствующему `Option<T>`-полю (подтверждено эмпирически). Функционально страница работает, но typecheck-гейт качества нарушен. |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|---------------------|--------|
| `OperationModal.svelte` (install picker) | `cartridgeOptions` (теперь сужен по совместимости) | `cartridges.list({compatible_with_printer_device_id})` → реальный SQL self-adjusting предикат | Да | ✓ FLOWING — подтверждено 3 зелёными `printer_compatib_*` тестами на реальной SQLite. |
| `act_service.rs::suggest_person` | Автокомплит имён | UNION ALL `acts` + `cartridges.holder_name`, реальный SQL | Да | ✓ FLOWING — подтверждено зелёными тестами с реальными seed-данными в обеих таблицах. |
| Auto-return предыдущего картриджа | `previous.status_id/location/holder_name` | Реальный `SELECT ... WHERE current_printer_device_id=?1 AND status_id=2` внутри транзакции | Да | ✓ FLOWING — подтверждено 3 целевыми тестами (`install_auto_returns_previous_cartridge_in_same_printer` и override/fallback вариантами) на временной SQLite БД. |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| GAP-12-01 backend: автокомплит объединяет acts+cartridges | `cargo test -p trackly-app --test acts_suggest -- --test-threads=1` | 10 passed; 0 failed | ✓ PASS |
| GAP-12-02 backend: совместимость принтер↔модель | `cargo test -p trackly-app --test cartridges_crud -- --test-threads=1` | 9 passed; 0 failed (3 printer_compatib_*) | ✓ PASS |
| GAP-12-03 backend: авто-возврат предыдущего картриджа | `cargo test -p trackly-app --test cartridges_lifecycle -- --test-threads=1` | 18 passed; 0 failed | ✓ PASS |
| RBAC: новые 403-кейсы для compatibility-команд | `cargo test -p trackly-app --test role_endpoint_matrix -- --test-threads=1` | 1 passed (драйвер всех кейсов, включая 33/34/35); 0 failed | ✓ PASS |
| Полный воркспейс компилируется и тестируется без регрессий | `TRACKLY_AD_MOCK=1 cargo test --workspace` | **Компиляция падает**: E0063 в `cartridges_history.rs:70,112` | ✗ **FAIL** |
| Backend build (с тестовыми бинарями) | `cargo build --workspace --tests` | **Падает** — тот же E0063 | ✗ **FAIL** |
| Backend lib-only build+clippy (как в исходной верификации) | `cargo clippy -p trackly-core -p trackly-app -p trackly-infra -- -D warnings` | Чисто | ✓ PASS |
| Frontend типы | `pnpm --dir ui exec svelte-check` | **1 ERROR** (`CartridgesPage.svelte:60`), 36 pre-existing warnings | ✗ **FAIL** (warning-уровень severity, не функциональный) |
| Frontend сборка | `pnpm --dir ui build` | Успешно (Vite/esbuild не делает строгую проверку типов) | ✓ PASS |
| Точечная проверка runtime-десериализации `Option<T>` без `#[serde(default)]` | Написан/прогнан/откатан scratch-тест в `dto/cartridge.rs` | `Ok(CartridgeFilter {..., compatible_with_printer_device_id: None})` — десериализация успешна при отсутствии ключа | ✓ PASS (подтверждает, что #9 не функциональный сбой) |

### Probe Execution

Step 7c пропущен: фаза не декларирует и не подразумевает `scripts/*/tests/probe-*.sh` (backend/frontend feature-фаза, не migration/tooling). Подтверждено повторно — никаких новых упоминаний probe-скриптов в планах 12-04..12-09.

### Requirements Coverage

D-09..D-19 — decision-ID из `12-CONTEXT.md`/`12-HUMAN-UAT.md` (gap-closure решения), как и исходные D-01..D-08, намеренно НЕ являются строками `.planning/REQUIREMENTS.md`. Это ожидаемо и не является пробелом (фаза идёт от пользовательских UAT-решений, не от формальных REQ).

| Decision | Источник | Статус | Evidence |
|----------|----------|--------|----------|
| D-09, D-10 | 12-04-PLAN.md / 12-HUMAN-UAT.md GAP-12-01 | ✓ SATISFIED | См. Truth #2. |
| D-11..D-15a | 12-05-PLAN.md / GAP-12-02 (backend) | ✓ SATISFIED | См. Truth #3. |
| D-12 | 12-07-PLAN.md / GAP-12-02 (frontend A) | ✓ SATISFIED | См. Truth #4. |
| D-13, D-14 | 12-08-PLAN.md / GAP-12-02 (frontend B) | ✓ SATISFIED | См. Truth #5. |
| D-16..D-19 | 12-06-PLAN.md / GAP-12-03 (backend) | ✓ SATISFIED | См. Truth #6. |
| D-16 | 12-09-PLAN.md / GAP-12-03 (frontend) | ✓ SATISFIED (с оговоркой) | См. Truth #7. Оговорка: тот же план оставил компиляционную регрессию в `cartridges_history.rs` — функциональность D-16 сама по себе работает, но фаза в целом не проходит `cargo test --workspace`. |

Орфанных REQ-ID, отнесённых к Фазе 12 в `REQUIREMENTS.md`, не найдено — ожидаемо, фаза целиком вне формальной REQ-ID traceability-системы по дизайну (подтверждено повторно).

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `crates/trackly-app/tests/cartridges_history.rs` | 70, 112 | Неполное обновление структурного варианта enum после расширения полей в зависимом плане (12-09) | 🛑 BLOCKER | Ломает компиляцию `cargo build/test --workspace` — невозможно запустить полный тестовый набор воркспейса, что является прямым требованием проектных гейтов качества (CI gate на каждый push, `cargo nextest`). |
| `ui/src/features/cartridges/CartridgesPage.svelte` | 60 | Дважды самостоятельно обнаруженный, дважды задокументированный, но никогда не закрытый typecheck-регресс (`compatible_with_printer_device_id` отсутствует в literal) | ⚠️ WARNING | Не ломает runtime (Option<T> без #[serde(default)] десериализуется в None), но нарушает заявленный фазой/проектом гейт «`svelte-check` — CI gate» (см. CLAUDE.md Development Tools: `svelte-check` — «CI gate»). `pnpm build` всё же проходит, так что страница картриджей продолжает работать в браузере — деградация ограничена качеством типов, не функциональностью. |

Debt-маркеры (`TBD`/`FIXME`/`XXX`) — поиск по всем файлам, изменённым в Wave 12-04..12-09 (act_service.rs, migrations/V029, printers_sqlite.rs, cartridges_sqlite.rs, dto/cartridge.rs, domain/cartridges.rs, 4 .svelte файла) — **0 совпадений**. Нет незакрытых debt-маркеров.

`TODO`/`HACK`/`PLACEHOLDER`/`placeholder`/`coming soon`/`not yet implemented` — тоже 0 совпадений в этих файлах.

`deferred-items.md` корректно используется как механизм отложенных пунктов (а не скрытых) — но в случае `CartridgesPage.svelte:60` процесс отложения не сработал: пункт был дважды зафиксирован как «принадлежит следующему плану, который коснётся этого файла», но ни один из оставшихся планов фазы (12-08, 12-09) не коснулся файла, и фаза подошла к завершению без закрытия.

### Human Verification Required

### 1. DISC-02 empty-state (перенесено из исходной верификации, статус не изменился)

**Test:** Открыть «Установить картридж» из заявки на модель, для которой на складе нет подходящих картриджей (с учётом активного теперь фильтра совместимости из GAP-12-02).
**Expected:** Список показывает «Нет подходящих картриджей на складе»; форма не блокируется (модалку можно закрыть, заявку отклонить, либо использовать старый вход).
**Why human:** `12-HUMAN-UAT.md` тест 2 остался в статусе `[pending]` — не выполнен ни во время, ни после gap-closure работы. Код-путь самого empty-state не менялся, но взаимодействие с новым D-13/D-14 фильтром не проверено интерактивно.

### 2. D-08 regression + новый блок «Предыдущий картридж» (перенесено и расширено)

**Test:** Открыть карточку картриджа со статусом «На складе» напрямую (меню картриджа → «Установить в принтер»).
**Expected:** Старая форма работает без изменений: без селектора картриджа, БЕЗ нового блока «Предыдущий картридж» (добавленного в 12-09) — расположение полей, фокус, скролл не регрессировали.
**Why human:** `12-HUMAN-UAT.md` тест 3 остался `[pending]`. Дополнительно теперь нужно визуально подтвердить, что новый D-16/12-09 блок корректно скрыт в этом входе (статически проверено — эффект гейтирован на `cartridge===null && preFillPrinterId!==undefined` — но живая сессия не прогонялась ни разу за всю фазу, включая после 6 планов изменений).

## Gaps Summary

Шесть планов закрытия гэпов (12-04..12-09) **в целом успешно реализовали свою заявленную функциональность** — все 3 UAT-гэпа (GAP-12-01 автокомплит, GAP-12-02 совместимость принтер↔модель, GAP-12-03 авто-возврат картриджа) подтверждены реальным, протестированным, связанным кодом, не заглушками. Это не пересмотр выводов по самой функциональности.

Однако ре-верификация нашла **1 BLOCKER**, не упомянутый ни в одном из 6 SUMMARY-файлов и не учтённый при объявлении фазы «готовой к закрытию»: gap-closure работа сама внесла **компиляционную регрессию** — `cargo build/test --workspace` не проходит из-за двух пропущенных сайтов обновления структурного enum-варианта в `crates/trackly-app/tests/cartridges_history.rs`. Это прямо противоречит проектному правилу CI gate («проверки кода на push») и базовому принципу «фаза готова, если весь воркспейс компилируется и тестируется». План 12-06 ранее правильно обновил эти же 2 сайта под аналогичное расширение поля (`printer_device_id`) — план 12-09 пропустил их при аналогичном расширении (`previous_cartridge_state_id`/`previous_cartridge_location`), и его собственный SUMMARY ложно заявляет полное покрытие («Updated all 10 existing Install {..} construction sites»), говоря на самом деле только о `cartridges_lifecycle.rs`.

Дополнительно найден **1 WARNING**: `ui/src/features/cartridges/CartridgesPage.svelte:60` несёт реальную `svelte-check` ERROR (не warning) — `compatible_with_printer_device_id` отсутствует в литерале `CartridgeFilter`. Эмпирически подтверждено (точечный serde-тест), что это НЕ ломает runtime — страница продолжает работать в браузере, `pnpm build` проходит. Но это нарушение заявленного CI-гейта `svelte-check`, дважды самостоятельно обнаруженное исполнителями (планы 12-07, 12-08) и дважды корректно задокументированное в `deferred-items.md` как «принадлежит следующему плану, который коснётся этого файла» — но фаза завершилась без того, чтобы какой-либо план фактически коснулся файла и закрыл пункт.

Наконец, **2 человеческих пункта из исходной верификации остаются непройденными** (`12-HUMAN-UAT.md` тесты 2 и 3, статус `[pending]`) — они не требуют новых кодовых изменений, но фаза не может быть объявлена `passed` без их живого прогона, тем более что код, который они проверяют (cartridge-centric вход, empty-state), теперь взаимодействует с двумя новыми механизмами (D-13/D-14 фильтр, D-16 блок «Предыдущий картридж»), что повышает риск необнаруженной визуальной регрессии.

**Рекомендация:** перед закрытием фазы 12 необходим короткий gap-closure план (или ручная правка) для:
1. Добавления `previous_cartridge_state_id: None, previous_cartridge_location: None` в оба сайта `cartridges_history.rs` (5 минут работы, тривиальный фикс) — BLOCKER, обязательно перед закрытием.
2. Добавления `compatible_with_printer_device_id: null` в `CartridgesPage.svelte:60`'s `activeFilter` (тривиальный фикс) — WARNING, желательно перед закрытием, чтобы не нарушать заявленный CI-гейт `svelte-check`.
3. Живого прогона тестов 2 и 3 из `12-HUMAN-UAT.md` (после фиксов выше).

---

_Verified: 2026-06-23T05:00:00Z_
_Verifier: Claude (gsd-verifier)_
