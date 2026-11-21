---
phase: quick-261121-heg
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/trackly-core/src/ports/printers.rs
  - crates/trackly-core/src/domain/printers.rs
  - crates/trackly-infra/src/repos/printers_sqlite.rs
  - crates/trackly-app/src/services/printer_service.rs
  - crates/trackly-app/src/dto/printer.rs
  - crates/trackly-app/src/tauri_cmds/printers.rs
  - crates/trackly-app/src/http/printers.rs
  - crates/trackly-app/src/specta_export.rs
  - crates/trackly-app/tests/role_endpoint_matrix.rs
  - crates/trackly-app/tests/devices_printer_network.rs
  - crates/trackly-app/src/services/group_place.rs
  - crates/trackly-app/src/services/attachment_service.rs
  - crates/trackly-app/src/dto/place.rs
  - .planning/STATE.md
  - ui/src/bindings-phase6.ts
  - ui/src/features/printers/PrinterDetail.svelte
  - ui/src/features/settings/StorageSettings.svelte
  - ui/src/features/devices/usbHostPicker.ts
  - ui/src/features/devices/DeviceFormBody.svelte
  - ui/src/lib/api/client.ts
  - ui/scripts/check-html-commands.mjs
  - ui/scripts/check-device-type-labels.mjs
  - ui/scripts/check-contrast.mjs
  - ui/scripts/check-lint-selftest-wiring.mjs
  - ui/package.json
  - .github/workflows/ci-fast.yml
autonomous: true
requirements: [INT-10, INT-20, INT-35, N2, INT-36, NEW-6, INT-19, WR-13, IN-01]

must_haves:
  truths:
    - "N2: карточка принтера показывает историю опросов из printer_readings — команда printers_get_readings есть и на десктопе (Tauri invoke), и по LAN (POST /api/v1/printers_get_readings); новейшие сверху, не более 50 строк"
    - "N2: сотрудник получает 403 на /api/v1/printers_get_readings (authorize ReadData); случай есть в роль-матрице, гейт покрытия маршрутов зелёный без записи в EXEMPT"
    - "INT-36а: в group_place.rs нет #[allow(dead_code)] — GroupMoveCause::act живой (зовётся из group_act_plan.rs), clippy -D warnings зелёный"
    - "INT-36б: IPv6-адрес принтера отклоняется сообщением «IPv6 не поддерживается», а не про порт; адрес с одним двоеточием по-прежнему получает сообщение про порт"
    - "INT-36в: мягко удалённый USB-хост (devices.deleted_at_utc IS NOT NULL) помечен в карточке принтера «(удалён)»"
    - "INT-36г: когда обход поддерева в выборе USB-хоста упирается в SUBTREE_CAP, под полем видно «Показаны первые 50 групп состава — уточните поиск»"
    - "INT-10: кнопка «Открыть папку с базой данных» не рисуется вне Tauri (LAN-браузер её не видит)"
    - "INT-35: apiCall по HTTP бросает ошибку «маршрут не найден», если text/html пришёл на команду вне белого списка HTML-команд; белый список сверен гейтом check-html-commands.mjs с реальными text/html-обработчиками crates/trackly-app/src/http"
    - "INT-20: check-device-type-labels.mjs ловит копию справочника типов с ключом caption/title/text/displayName; selftest содержит мутант caption; на копии реального ui/ мутант краснеет, чистая копия зелёная"
    - "INT-19: check-contrast.mjs проверяет порог 4.6 (D-29); selftest содержит мутант в полосе 4.5–4.6; реальные токены проходят"
    - "WR-13: каждый ui/scripts/check-*.mjs вызывается в pnpm lint, и у каждого с веткой --selftest вызов --selftest тоже есть — закреплено гейтом check-lint-selftest-wiring.mjs; selftest корневых scripts/check-privacy.selftest.mjs и check-write-site-gate-untouched.mjs --selftest идут в ci-fast"
    - "NEW-6: комментарий employee_can_read_device говорит правду (сотрудник не видит вложений устройства в v1.4); решение записано в .planning/STATE.md ### Decisions"
    - "IN-01: PlaceTreeNodeDto удалён (использований нет); протухший комментарий DeviceFormBody.svelte (.snmp-section) описывает текущее поведение"
  artifacts:
    - path: "crates/trackly-app/src/tauri_cmds/printers.rs"
      provides: "build_printers_get_readings + #[tauri::command] printers_get_readings"
      contains: "fn build_printers_get_readings"
    - path: "crates/trackly-app/src/http/printers.rs"
      provides: "маршрут /api/v1/printers_get_readings"
      contains: "/api/v1/printers_get_readings"
    - path: "crates/trackly-infra/src/repos/printers_sqlite.rs"
      provides: "list_readings + usb_host_deleted в выборке принтера"
      contains: "fn list_readings"
    - path: "ui/scripts/check-html-commands.mjs"
      provides: "паритет белого списка HTML-команд клиента с сервером, с --selftest и --src"
    - path: "ui/scripts/check-lint-selftest-wiring.mjs"
      provides: "мета-гейт подключения гейтов и их selftest в pnpm lint, с --selftest и --src"
  key_links:
    - from: "ui/src/features/printers/api.ts"
      to: "/api/v1/printers_get_readings"
      via: "apiCall('printers_get_readings', { id })"
      pattern: "printers_get_readings"
    - from: "crates/trackly-app/src/specta_export.rs"
      to: "tauri_cmds::printers::printers_get_readings"
      via: "регистрация команды (invoke_handler + bindings)"
      pattern: "printers::printers_get_readings"
    - from: "ui/src/lib/api/client.ts"
      to: "HTML_COMMANDS"
      via: "ветка content-type text/html"
      pattern: "HTML_COMMANDS"
    - from: "ui/package.json"
      to: "scripts/check-lint-selftest-wiring.mjs"
      via: "pnpm lint"
      pattern: "check-lint-selftest-wiring"
---

<objective>
Пакет C аудита вехи v1.4 «UI, гейты, мелочи»: закрыть INT-10, INT-20, INT-35, N2, INT-36 а–г, NEW-6, INT-19, WR-13, протухший комментарий DeviceFormBody.svelte и IN-01.

Purpose: убрать мёртвые и лгущие места (вызов несуществующей команды, ложные комментарии, мёртвый DTO), сделать сообщения UI честными и закрепить гейты так, чтобы их ослабление было видно в CI.
Output: работающая история опросов принтера (N2), правки UI и домена, три исправленных гейта и два новых (check-html-commands, check-lint-selftest-wiring).
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@./CLAUDE.md
@.planning/v1.4-MILESTONE-AUDIT.md

Установленные факты (собраны планировщиком, повторно не исследовать):
- Таблица printer_readings существует и пишется мониторингом: SqlitePrinterRepository::upsert_reading_in_tx (printers_sqlite.rs:116-126). Чтение последней строки — get_last_reading (printers_sqlite.rs:512-537; колонки id, printer_id, ts_utc, COALESCE(toner_levels,'{}'), page_count, status). Значит N2 = РЕАЛИЗОВАТЬ команду, а не удалять UI.
- UI уже зовёт printers.getReadings(p.id) → apiCall<PrinterReadingDto[]>('printers_get_readings', { id }) (ui/src/features/printers/api.ts:39); id = printers.id. Тип ведётся вручную в ui/src/bindings-phase6.ts:73: { id, printerId, tsUtc, tonerLevelsJson, pageCount: number|null, status }. Блок «История статусов» — PrinterDetail.svelte:380-401, ошибка сейчас глушится .catch на :92.
- Образец для N2: build_printers_get (tauri_cmds/printers.rs:35-42, authorize ReadData) + тонкая обёртка с #[tauri::command], затем #[specta::specta] (порядок обязателен); handler_get_by_device_id (http/printers.rs:113-126); router() (http/printers.rs:207-225); регистрация в specta_export.rs:140-149 (этот список кормит и invoke_handler, и bindings). Роль-матрица: Case 40 в role_endpoint_matrix.rs:1590-1610 (post_with_cookie + employee_cookie → 403); гейт покрытия http_routes_covered_by_role_matrix (≈:7159) требует, чтобы каждый маршрут встречался в тексте матрицы вне EXEMPT. Матрица входит в suite app_auth (tests/suites/auth.rs).
- Единственный impl PrinterRepository — SqlitePrinterRepository (printers_sqlite.rs:400); трейт — crates/trackly-core/src/ports/printers.rs. PrinterRow строится литералом в двух местах: printers_sqlite.rs:59 (map_row_printer) и tests/devices_printer_network.rs:152 (suite app_devices). PrinterDto строится только через impl From<PrinterRow> (dto/printer.rs:62).
- INT-36а: GroupMoveCause::act вызывается в group_act_plan.rs:1258 и :1432 — #[allow(dead_code)] на group_place.rs:62 протух; удалить атрибут вместе с его комментарием.
- INT-36б: normalize_printer_ip (domain/printers.rs ≈251-273) отклоняет любое ':' сообщением про порт; тест printer_ip_rejects_port_and_ipv6 (≈:374) ждёт «Порт» и для "::1", "2001:db8::1". Подсказка 'IPv4 или IPv6' в GroupPropertiesForm.svelte:292 относится к свойству группы типа ip (другой валидатор) — её НЕ трогать.
- INT-36в: выборка принтера джойнит хост LEFT JOIN devices h ON h.id = p.usb_host_device_id (printers_sqlite.rs:44-53, колонки 15/16 = h.name/h.inventory_number); мягкое удаление устройства = devices.deleted_at_utc. Разметка USB-хоста — PrinterDetail.svelte:462-473.
- INT-36г: SUBTREE_CAP = 50 ограничивает число обходимых ГРУПП в walkGroupSubtree (usbHostPicker.ts:44; отсечка на :106 `if (visited.size >= SUBTREE_CAP) continue;`); единственный вызов — DeviceFormBody.svelte:789. Гейты check-usb-host-picker.mjs и check-printer-usb-host-field.mjs читают эти файлы.
- INT-10: StorageSettings.svelte:29-38 openFolder() зовёт settings_open_db_folder; кнопка на :100; в proceedWithMove уже есть локальная проверка `'__TAURI_INTERNALS__' in window`.
- INT-35: client.ts:60-62 `if (contentType.includes('text/html')) return (await res.text()) as R;`. Легитимные HTML-ответы на сервере: http/acts.rs (handler_render_pdf, handler_render_acceptance_pdf), http/repairs.rs (handler_render_act, handler_render_device_report), http/reports.rs (handler_export_pdf), http/settings_org.rs (handler_templates_validate_preview); http/mod.rs:313 — spa_fallback (не маршрут, исключается). Имена маршрутов — из `.route("/api/v1/<name>", post(handler_x))` в тех же файлах. Серверная защита F17: spa_fallback для /api/* отдаёт JSON NotFound (http/mod.rs:297-302). Форма ошибки клиента: AppError { code, message } (ui/src/lib/api/errors.ts).
- INT-19: check-contrast.mjs:55 COMPOSITE_MIN = 4.5; комментарий :53 заявляет запас ≥ 4.6 (D-29); шапка :21 тоже называет 4.5. Аргументы: --tokens=<копия _tokens.scss>, --selftest [--dump-mutants=<dir>]. Минимум реальных токенов сейчас 4.61.
- INT-20: check-device-type-labels.mjs:116 `const NAME = '(?:name|label)';`; аргументы --src=<копия ui/> и --selftest (selftest(root) на :301, мутанты по реальным исходникам с уникальным якорем).
- WR-13: проверено — сегодня ВСЕ ui/scripts/check-*.mjs с веткой --selftest уже вызываются с --selftest в "lint" (ui/package.json:16); 11 гейтов без selftest (check-tokens, check-focus-outline, check-pagedjs-csp-hash, check-print-isolation, check-placepath-parity, check-path-settings-form, check-report-type-parity, check-print-idempotency, check-operation-modal-validate, check-operation-modal-autofill, check-movements-type-filter). Не закреплено механически и не гоняется в CI: корневые scripts/check-privacy.selftest.mjs и `node scripts/check-write-site-gate-untouched.mjs --selftest` (ci-fast.yml:31 зовёт только check-privacy.mjs). pnpm lint идёт в ci-fast.yml:133-135 и ci-full.yml:144-147.
- NEW-6: attachment_service.rs:88-94 — doc-комментарий «Deny-by-default in 41.3 (D-27): opened by phase 41.6 together with the Devices section rule for Employee» ложен: 41.6 вложения не открывала. Раздел ### Decisions в STATE.md начинается на строке 762, записи вида `- [Phase X]: ...`, последняя перед ### Pending Todos (≈1467).
- IN-01: PlaceTreeNodeDto (dto/place.rs:75-105) нигде не используется (ни в crates, ни в ui/src; bindings.ts генерируется и лежит в .gitignore). Шапка dto/place.rs:3 его упоминает — поправить.
- Протухший комментарий DeviceFormBody.svelte:1628-1629 («IP/SNMP block, create+Принтер only — ported verbatim from the now-removed two-step printer-create modal»); разметка блока .snmp-section около :1455 — условие показа читать там.
- Параллельно спланированы quick-задачи 261121-heh-printer-identity и 261121-hei-dead-api-reports — возможны пересечения по printers.rs/api.ts. Перед каждой задачей `git log -3 --stat`; чужие правки не откатывать, конфликт решать слиянием.

Правила окружения (из памяти проекта):
- cargo test — по одному, СКОУПЛЕННО по suite-бинарникам (`--test app_auth`, `--test app_devices` …) с фильтром по имени; env TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1; ui/dist собран (`pnpm --dir ui build`; его prebuild гоняет `cargo test -p trackly-app --test export_bindings`). Не уводить cargo в фон и не возвращать управление посреди плана. Команды `timeout` на машине нет — не оборачивать в неё.
- Линковка падает с exit 69 → префикс DEVELOPER_DIR=/Library/Developer/CommandLineTools.
- Гейты UI — РАЗНЫЕ команды: `pnpm --dir ui lint` не включает svelte-check — гонять и `pnpm --dir ui check`; clippy и fmt отдельно.
- Мутационная проверка гейта: копия реального ui/ (или файла) в scratchpad, мутация с УНИКАЛЬНЫМ якорем (проверить, что якорь встречается ровно 1 раз), гейт с --src/--tokens на копию: мутант → exit≠0, чистая копия → exit 0.
- Приватность: репозиторий публичный. В тестах, фикстурах, комментариях, SUMMARY — только вымышленные данные («Иванов И.И.», 192.0.2.x, printer.local). Pre-commit-хук check-privacy обязан пройти; не цитировать реквизиты из истории git.
- Коммиты атомарные (по задаче), в конце сообщения строка `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Задача 1: N2 — команда и маршрут printers_get_readings (история опросов принтера)</name>
  <files>crates/trackly-core/src/ports/printers.rs, crates/trackly-infra/src/repos/printers_sqlite.rs, crates/trackly-app/src/services/printer_service.rs, crates/trackly-app/src/dto/printer.rs, crates/trackly-app/src/tauri_cmds/printers.rs, crates/trackly-app/src/http/printers.rs, crates/trackly-app/src/specta_export.rs, crates/trackly-app/tests/role_endpoint_matrix.rs, crates/trackly-app/tests/devices_printer_network.rs</files>
  <behavior>
    - infra: list_readings(conn, printer_id, limit) по 3 засеянным строкам с разными ts_utc возвращает их в порядке ts_utc DESC; при limit=2 — две новейшие; принтер без показаний — пустой Vec; NULL toner_levels приходит как "{}".
    - app (suite app_devices): build_printers_get_readings от Admin для принтера с засеянными показаниями возвращает Vec<PrinterReadingDto> новейшими сверху; serde_json даёт ключи id, printerId, tsUtc, tonerLevelsJson, pageCount, status (ровно форма ui/src/bindings-phase6.ts:73); несуществующий printers.id → AppError::NotFound.
    - роль-матрица (suite app_auth): Employee → POST /api/v1/printers_get_readings { "id": 1 } → 403; http_routes_covered_by_role_matrix зелёный без новой записи в ROUTE_MATRIX_EXEMPT.
  </behavior>
  <action>
Сначала тесты (RED — прогнать и увидеть красный/некомпилирующийся), затем реализация (GREEN), по образцу соседних printers_* (см. факты в context).

1) Порт: в трейт PrinterRepository рядом с get_last_reading добавить list_readings(&self, conn, printer_id: i64, limit: u32) -> Result<Vec<PrinterReadingRow>, AppError> с doc-комментарием «история опросов, новейшие первыми».
2) Infra: реализовать в impl для SqlitePrinterRepository тем же SELECT, что get_last_reading (колонки и COALESCE те же), с ORDER BY ts_utc DESC, id DESC и LIMIT ?2; маппинг строки вынести в одну функцию и использовать её и в get_last_reading. Юнит-тест list_readings — в mod tests этого файла (fresh_conn/seed_device уже есть).
3) Сервис: в PrinterService pub async fn list_readings(&self, id: i64) -> Result<Vec<PrinterReadingDto>, AppError> через readers.acquire() в spawn_blocking (образец PrinterService::get, printer_service.rs:162): сначала repo.get(&conn, id)? (несуществующий принтер → NotFound, как у printers_get), затем repo.list_readings(&conn, id, READINGS_HISTORY_LIMIT). Константа pub const READINGS_HISTORY_LIMIT: u32 = 50 с комментарием: блок «История статусов» — лента последних опросов; ретеншн 90 дней (D-Retention-01) даёт тысячи строк, лента их не листает.
4) DTO: в dto/printer.rs struct PrinterReadingDto (derive Debug, Clone, Serialize, Deserialize, specta::Type; serde rename_all = "camelCase"; поля id, printer_id, ts_utc: i64 с #[specta(type = i32)] как у соседних DTO; toner_levels_json: String; page_count: Option<i64> с #[specta(type = Option<i32>)]; status: String) + impl From<PrinterReadingRow>.
5) Транспорт: build_printers_get_readings(ctx, caller, id: i64) — первой строкой authorize(caller, &Action::ReadData)?, затем ctx.printers.list_readings(id).await. Tauri-обёртка printers_get_readings(state, id: i32) с resolve_tauri_identity. Регистрация в specta_export.rs сразу после printers_get_compatible_aggregates с комментарием «quick 261121-heg (N2): история опросов принтера — UI звал команду с Фазы 6, её не было». HTTP: payload { id: i32 } (образец GetByDeviceIdPayload, тот же serde-стиль), handler_get_readings через session_identity, .route("/api/v1/printers_get_readings", post(handler_get_readings)).
6) Роль-матрица: новый Case по образцу Case 40 (следующий свободный номер; дописать его и в шапочный список //! вверху файла) — Employee → /api/v1/printers_get_readings → FORBIDDEN. В ROUTE_MATRIX_EXEMPT маршрут НЕ добавлять.
7) Интеграционный тест printers_get_readings_* в tests/devices_printer_network.rs (suite app_devices): завести принтер имеющимися в файле хелперами, засеять 3 строки printer_readings через writer (образец INSERT в tests/devices_type_conversion.rs:115), позвать build_printers_get_readings с Admin-Identity, проверить порядок, ключи JSON (serde_json::to_value) и NotFound для чужого id. Адреса — только 192.0.2.x.

UI не трогать: api.ts и PrinterDetail.svelte уже зовут команду и рисуют ленту. Коммит: feat(quick-261121-heg): ...
  </action>
  <verify>
    <automated>cd /Users/madsas/Projects/trackly && export DEVELOPER_DIR=/Library/Developer/CommandLineTools TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 && cargo test -p trackly-infra list_readings && cargo test -p trackly-app --test app_devices printers_get_readings && cargo test -p trackly-app --test app_auth role_endpoint_matrix && cargo test -p trackly-app --test app_auth http_routes_covered_by_role_matrix && grep -c 'printers::printers_get_readings' crates/trackly-app/src/specta_export.rs</automated>
  </verify>
  <done>Все четыре прогона зелёные (у каждого фильтра ≥1 выполненного теста — проверить строку «N passed», 0 passed = провал); маршрут и команда зарегистрированы; Employee → 403; карточка принтера получает реальные показания вместо проглоченной ошибки.</done>
</task>

<task type="auto" tdd="true">
  <name>Задача 2: Rust-мелочи и честный UI принтера — INT-36 а/б/в, NEW-6, IN-01, INT-10, INT-36г, комментарий DeviceFormBody</name>
  <files>crates/trackly-app/src/services/group_place.rs, crates/trackly-core/src/domain/printers.rs, crates/trackly-infra/src/repos/printers_sqlite.rs, crates/trackly-app/src/dto/printer.rs, crates/trackly-app/tests/devices_printer_network.rs, crates/trackly-app/src/services/attachment_service.rs, crates/trackly-app/src/dto/place.rs, .planning/STATE.md, ui/src/bindings-phase6.ts, ui/src/features/printers/PrinterDetail.svelte, ui/src/features/settings/StorageSettings.svelte, ui/src/features/devices/usbHostPicker.ts, ui/src/features/devices/DeviceFormBody.svelte</files>
  <behavior>
    - normalize_printer_ip("::1"), ("2001:db8::1"), ("[2001:db8::1]") → Validation { field: "printer_ip_address", message содержит «IPv6 не поддерживается» и НЕ содержит «Порт» }.
    - normalize_printer_ip("192.0.2.10:161"), ("printer.local:9999") → message по-прежнему содержит «Порт».
    - Выборка принтера, у которого USB-хост мягко удалён (UPDATE devices SET deleted_at_utc = ... для хоста), даёт PrinterRow.usb_host_deleted == true; у живого хоста и без хоста — false.
  </behavior>
  <action>
(а, INT-36а) group_place.rs:62 — удалить строку #[allow(dead_code)] вместе с её комментарием; GroupMoveCause::act живой.

(б, INT-36б) В normalize_printer_ip до проверки порта распознать IPv6: строка, начинающаяся с '[', либо содержащая два и более ':' → Validation с field "printer_ip_address" и сообщением «IPv6 не поддерживается: опрос принтера идёт по IPv4-адресу или имени хоста.». Ровно одно ':' — прежнее сообщение про порт. Обновить doc-комментарий функции (абзац «Отсюда же следует, что IPv6 не поддерживался…» — теперь IPv6 получает собственный отказ). Тест printer_ip_rejects_port_and_ipv6 разделить на printer_ip_rejects_port (только порт → «Порт») и printer_ip_rejects_ipv6_honestly (три IPv6-формы → «IPv6», без «Порт»). Сначала перевернуть тест и увидеть красный.

(в, INT-36в) Бэкенд: в SELECT выборки принтера (printers_sqlite.rs:44-53) добавить колонку `(h.deleted_at_utc IS NOT NULL) AS usb_host_deleted` последней; в PrinterRow (domain/printers.rs) поле usb_host_deleted: bool с doc-комментарием; map_row_printer читает её индексом 17; литерал в tests/devices_printer_network.rs:152 дополнить. PrinterDto (dto/printer.rs) — поле usb_host_deleted: bool (в camelCase-JSON usbHostDeleted), проброс в From<PrinterRow>. Тест: в devices_printer_network.rs — принтер с USB-хостом, хост мягко удалить прямым UPDATE devices через writer, перечитать → usb_host_deleted == true; второй принтер с живым хостом → false. Фронт: в ui/src/bindings-phase6.ts PrinterDto добавить usbHostDeleted: boolean с комментарием; в PrinterDetail.svelte (:462-473) после имени хоста при printer.usbHostDeleted выводить « (удалён)» в приглушённом стиле (существующий класс muted или токен --tr-text-muted, без новых hex-цветов).

(г, INT-36г) usbHostPicker.ts: экспортировать константу SUBTREE_CAP_NOTICE = `Показаны первые ${SUBTREE_CAP} групп состава — уточните поиск.`; в walkGroupSubtree добавить в options необязательный onCapReached?: () => void, вызываемый один раз, когда дочерняя группа пропущена из-за `visited.size >= SUBTREE_CAP` (не при уже посещённой). В DeviceFormBody.svelte у вызова на :789 завести $state-флаг capReached (сбрасывать перед каждым обходом), передать onCapReached, и при capReached показывать SUBTREE_CAP_NOTICE под полем USB-хоста рядом с HOST_FIELD_HINT (тот же стиль подсказки, aria-live="polite"). После правки прогнать check-usb-host-picker.mjs и check-printer-usb-host-field.mjs (обычный режим и --selftest); если их якоря задеты — обновить якорь, не ослабляя проверку.

(Комментарий) DeviceFormBody.svelte:1628-1629 — прочитать условие показа .snmp-section (≈:1455) и переписать комментарий под фактическое поведение (когда блок виден сейчас, откуда он), без ссылки на «create+Принтер only», если это уже неправда.

(INT-10) StorageSettings.svelte: вынести `const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;` на уровень модуля скрипта, локальную копию в proceedWithMove убрать (использовать общую); кнопку «Открыть папку с базой данных» обернуть в {#if isTauri}; в openFolder ранний return при !isTauri. Комментарий: команда settings_open_db_folder есть только у десктопа (INT-10).

(NEW-6) attachment_service.rs:88-94: заменить ложный doc-комментарий на правду — «Осознанное решение v1.4 (quick 261121-heg, NEW-6): сотрудник не видит вложений устройства. Запрет по умолчанию введён в 41.3 (D-27); фаза 41.6 открыла сотруднику раздел устройств, но вложения не открывала. Открытие — отдельное решение с новой веткой роль-матрицы.» В .planning/STATE.md в конец раздела ### Decisions (перед ### Pending Todos) добавить строку `- [Quick 261121-heg]: NEW-6 — сотрудник не видит вложений устройства в v1.4: employee_can_read_device = false осознанно (D-27 41.3 остаётся в силе, 41.6 вложения не открывала); открытие — только отдельным решением.`

(IN-01) dto/place.rs: удалить struct PlaceTreeNodeDto целиком с её doc-комментарием и убрать её из шапки (:3). Перед удалением ещё раз `grep -rn PlaceTreeNodeDto crates ui/src` — допускается только определение.

Коммиты: fix(quick-261121-heg) для б/в/г/INT-10, chore(quick-261121-heg) для а/IN-01/комментариев, docs для STATE.md — либо один fix-коммит задачи, если правки переплетены.
  </action>
  <verify>
    <automated>cd /Users/madsas/Projects/trackly && export DEVELOPER_DIR=/Library/Developer/CommandLineTools TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 && cargo test -p trackly-core printer_ip && cargo test -p trackly-app --test app_devices devices_printer_network && cargo clippy --workspace --all-targets -- -D warnings && test "$(grep -rn 'PlaceTreeNodeDto' crates ui/src | wc -l | tr -d ' ')" = 0 && test "$(grep -v '^\s*//' crates/trackly-app/src/services/group_place.rs | grep -c 'allow(dead_code)')" = 0 && grep -c 'Quick 261121-heg\]: NEW-6' .planning/STATE.md && cd ui && node scripts/check-usb-host-picker.mjs --selftest && node scripts/check-usb-host-picker.mjs && node scripts/check-printer-usb-host-field.mjs --selftest && node scripts/check-printer-usb-host-field.mjs && pnpm check</automated>
  </verify>
  <done>Тесты IPv6/порт и usb_host_deleted зелёные (перевёрнутые тесты были красными до правки); clippy без allow; PlaceTreeNodeDto нет; решение NEW-6 в STATE.md; LAN-клиент не видит «Открыть папку»; удалённый хост помечен; подсказка о лимите поддерева появляется при отсечке; svelte-check зелёный.</done>
</task>

<task type="auto">
  <name>Задача 3: Гейты и клиент — INT-35 (белый список HTML-команд + гейт паритета), INT-20, INT-19, WR-13</name>
  <files>ui/src/lib/api/client.ts, ui/scripts/check-html-commands.mjs, ui/scripts/check-device-type-labels.mjs, ui/scripts/check-contrast.mjs, ui/scripts/check-lint-selftest-wiring.mjs, ui/package.json, .github/workflows/ci-fast.yml</files>
  <action>
(INT-35, клиент) В client.ts экспортировать `HTML_COMMANDS: ReadonlySet<string>` — имена команд, чьи HTTP-обработчики законно отвечают text/html (перечислить ОТ репозитория: для каждого text/html-обработчика в crates/trackly-app/src/http/{acts,repairs,reports,settings_org}.rs взять имя из его .route("/api/v1/<name>", …)). В ветке text/html: если name ∈ HTML_COMMANDS — вернуть текст как сейчас; иначе бросить AppError { code: 'NOT_FOUND' — тот же строковый код, что отдаёт сервер для AppError::NotFound (сверить в crates/trackly-core/src/error.rs / error_axum.rs и взять его), message: `Маршрут не найден: ${name}` } — никакого `as R` для HTML вне списка. Комментарий над веткой: SPA-fallback отдаёт index.html с 200, поэтому HTML на JSON-команду = отсутствующий маршрут (INT-35; серверная линия — F17).

(INT-35, гейт) Новый ui/scripts/check-html-commands.mjs в стиле соседних гейтов (заголовочный комментарий с назначением и usage; --src=<копия корня репо> для мутаций, по умолчанию корень репозитория = ui/..; --selftest). Правило: множество A = для каждого вхождения `CONTENT_TYPE, "text/html` в crates/trackly-app/src/http/*.rs (кроме тела spa_fallback в mod.rs; строки-комментарии игнорировать) найти объемлющий `async fn <handler>` и в том же файле `.route("/api/v1/<name>", post(<handler>))` → name; обработчик без маршрута — ошибка. Множество B = литералы из `HTML_COMMANDS` в ui/src/lib/api/client.ts. Красный, если A ≠ B (печатать разницу в обе стороны) или если A пусто (защита от вакуумного прохода). Selftest во временной копии (fs.mkdtempSync в os.tmpdir) по РЕАЛЬНЫМ исходникам: мутант 1 — убрать одно имя из HTML_COMMANDS → красный; мутант 2 — лишнее имя в HTML_COMMANDS → красный; мутант 3 — добавить в копию http/printers.rs text/html-обработчик с маршрутом → красный; контроль — немутированная копия зелёная. Якорь каждого мутанта проверять на уникальность (ровно 1 вхождение), иначе selftest падает.

(INT-20) check-device-type-labels.mjs:116 — NAME = '(?:name|label|caption|title|text|displayName|display_name)', комментарий: любой ключ-подпись, копия справочника с caption проходила зелёной (INT-20). В selftest добавить мутант: в реальный исходник (с уникальным якорем, как у существующих мутантов) вставить карту вида { id: 2, caption: 'Принтер' } → ожидается красный T-правила. Прогнать гейт по реальному ui/src — ложных срабатываний быть не должно; если есть — разобрать каждое (настоящая копия справочника чинится в коде, ложное — сужением правила с комментарием, не пустым EXEMPT).

(INT-19) check-contrast.mjs: COMPOSITE_MIN = 4.6; комментарий :53 и шапку :21 привести к «порог 4.6 = запас D-29 над 4.5 (R1/D-15), чтобы округление канала не уводило под границу». В selftest добавить мутант в полосе [4.5, 4.6): сдвинуть один существующий токен так, чтобы его композит попал между 4.5 и 4.6 (подобрать значение прогоном с --tokens на копии; в имени мутанта указать полученное отношение) → ожидается красный; существующие мутанты должны остаться зелёными (пойманными). Реальные токены (минимум 4.61) проходят.

(WR-13) Новый ui/scripts/check-lint-selftest-wiring.mjs (--src=<копия ui/>, --selftest): читает scripts.lint из package.json; для каждого scripts/check-*.mjs: (1) должен встречаться в lint как `node scripts/<file>`; (2) если в исходнике гейта есть литерал '--selftest' вне комментариев — в lint обязан быть `node scripts/<file> --selftest`. Красный с перечнем нарушений; пустой список гейтов — красный. Selftest по копии реального ui/: мутант 1 — из lint удалить один `<file> --selftest` → красный; мутант 2 — добавить в копию scripts/check-zz-new.mjs с веткой --selftest, не подключённый в lint → красный; контроль — чистая копия зелёная. Подключить в "lint" в package.json: `node scripts/check-html-commands.mjs --selftest && node scripts/check-html-commands.mjs`, `node scripts/check-lint-selftest-wiring.mjs --selftest && node scripts/check-lint-selftest-wiring.mjs` (последней парой в цепочке). Корневые selftest: в .github/workflows/ci-fast.yml сразу после шага check-privacy (:31) добавить шаг с `node scripts/check-privacy.selftest.mjs` и `node scripts/check-write-site-gate-untouched.mjs --selftest` — СНАЧАЛА прогнать обе команды локально из корня; если какая-то красная сама по себе, не подключать её, а описать в SUMMARY как найденный дефект.

(Мутационная проверка каждого изменённого/нового гейта — обязательна, вывод приложить в SUMMARY) В scratchpad: `cp -R` ui/ (без node_modules) и, для check-html-commands, нужные части корня (crates/trackly-app/src/http + ui/src/lib/api) в копию; на копии: гейт зелёный → внести мутант реального исходника (INT-20: caption-карта в реальный .svelte; INT-19: токен в полосе 4.5–4.6 в копии _tokens.scss через --tokens; INT-35: удалить имя из HTML_COMMANDS в копии client.ts; WR-13: убрать `--selftest` одного гейта в копии package.json) → гейт exit≠0 → вернуть → exit 0. Дополнительно прогнать ПРЕЖНЮЮ версию check-device-type-labels.mjs и check-contrast.mjs (git show HEAD:…) на тех же мутантах — она должна быть зелёной (доказательство, что правка несущая).

Коммиты: fix(quick-261121-heg) для client.ts + check-html-commands, test/chore для гейтов; prettier --write на новых .mjs до коммита.
  </action>
  <verify>
    <automated>cd /Users/madsas/Projects/trackly/ui && node scripts/check-html-commands.mjs --selftest && node scripts/check-html-commands.mjs && node scripts/check-device-type-labels.mjs --selftest && node scripts/check-device-type-labels.mjs && node scripts/check-contrast.mjs --selftest && node scripts/check-contrast.mjs && node scripts/check-lint-selftest-wiring.mjs --selftest && node scripts/check-lint-selftest-wiring.mjs && grep -c "COMPOSITE_MIN = 4.6" scripts/check-contrast.mjs && grep -c "HTML_COMMANDS" src/lib/api/client.ts && cd .. && node scripts/check-privacy.selftest.mjs && node scripts/check-write-site-gate-untouched.mjs --selftest && pnpm --dir ui lint && pnpm --dir ui check</automated>
  </verify>
  <done>Все гейты зелёные на реальном репо и в --selftest; каждый изменённый/новый гейт краснеет на мутанте копии реального исходника и зеленеет на чистой копии, прежние версии INT-19/INT-20-гейтов на тех же мутантах зелёные (вывод в SUMMARY); apiCall бросает NOT_FOUND на HTML вне белого списка; pnpm lint и pnpm check зелёные; ci-fast зовёт корневые selftest.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| LAN-браузер → axum /api/v1/printers_get_readings | неаутентифицированный или низкоправный клиент читает историю опросов |
| HTTP-ответ → apiCall (client.ts) | тело ответа сервера интерпретируется как данные приложения |
| CI → гейты ui/scripts | ослабление гейта молча снимает защиту |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-heg-01 | Information Disclosure | build_printers_get_readings | mitigate | authorize(caller, &Action::ReadData) первой строкой; Case Employee → 403 в role_endpoint_matrix.rs; маршрут не в EXEMPT, покрытие держит http_routes_covered_by_role_matrix |
| T-heg-02 | Elevation of Privilege | printers_get_readings (Tauri) | mitigate | resolve_tauri_identity + тот же build_* (один гейт на оба транспорта) |
| T-heg-03 | Tampering | apiCall ветка text/html | mitigate | HTML вне HTML_COMMANDS → ошибка NOT_FOUND вместо `as R`; паритет белого списка с сервером держит check-html-commands.mjs |
| T-heg-04 | Denial of Service | list_readings | mitigate | LIMIT READINGS_HISTORY_LIMIT = 50, чтение из пула ридеров (не писатель) |
| T-heg-05 | Information Disclosure | вложения устройства для Employee | accept | employee_can_read_device = false осознанно (NEW-6), решение в STATE.md |
| T-heg-06 | Repudiation | ослабление гейтов | mitigate | check-lint-selftest-wiring.mjs: каждый check-*.mjs и его --selftest обязаны быть в pnpm lint; корневые selftest в ci-fast |
</threat_model>

<verification>
- Задачи 1–3: их `<automated>` зелёные.
- Финально: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` (DEVELOPER_DIR при exit 69), `pnpm --dir ui build` (обновляет ui/dist и bindings.ts), `pnpm --dir ui lint`, `pnpm --dir ui check`.
- Приватность: `node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256` зелёный; в диффе нет реальных имён, адресов, реквизитов.
</verification>

<success_criteria>
- N2: история опросов приходит в карточку принтера по обоим транспортам; Employee получает 403.
- INT-36 а–г, INT-10, NEW-6, IN-01, комментарий DeviceFormBody: закрыты кодом, тестами или правкой текста, как описано в must_haves.truths.
- INT-35, INT-20, INT-19, WR-13: гейты/клиент исправлены, каждый подтверждён мутационной проверкой на копии реального исходника.
- Все гейты (lint, check, clippy -D warnings, fmt) зелёные; атомарные коммиты с Co-Authored-By.
</success_criteria>

<output>
Создать `.planning/quick/261121-heg-ui-gates-cleanup/261121-heg-SUMMARY.md`: по каждой находке — что сделано, файл:строка, коммит; вывод мутационных проверок гейтов (мутант → exit, чистая копия → exit, прежняя версия → exit); отклонения от плана. Только обезличенные данные.
</output>
