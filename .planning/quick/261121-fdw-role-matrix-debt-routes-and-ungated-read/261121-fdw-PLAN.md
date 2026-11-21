---
phase: quick-261121-fdw
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/trackly-app/tests/role_endpoint_matrix.rs
  - crates/trackly-app/src/http/
  - crates/trackly-app/src/
  - ui/src/
autonomous: true
requirements: [INT-29]
must_haves:
  truths:
    - "Каждый из 23 мутирующих маршрутов группы ДОЛГ без сессии даёт 401, под Employee 403, под Manager/Admin ведёт себя по Action из кода"
    - "Для 14 чтений без Action-гейта фактом (тестом) установлено, отвечают ли они без сессии; ни одно не отдаёт 200 без сессии, кроме явно обоснованных (страница входа)"
    - "Служебные чтения (settings_get_db_path, settings_get_backup_config, server_status, templates_list_for_editor, templates_render_preview, devices_import_csv_preview) недоступны Employee"
    - "Гейт стоит в общем build_*, поэтому Tauri и HTTP ведут себя одинаково; десктоп (системный Администратор) не сломан"
    - "http_routes_covered_by_role_matrix зелёный; в ROUTE_MATRIX_EXEMPT не осталось строк группы ДОЛГ и строк без причины-потребителя"
  artifacts:
    - path: "crates/trackly-app/tests/role_endpoint_matrix.rs"
      provides: "кейсы int29-механики для 23 мутирующих и ~14 чтений, чистый EXEMPT"
  key_links:
    - from: "build_* чтений"
      to: "authorize(...)"
      via: "общий build_*, не handler"
      pattern: "authorize\\("
---

<objective>
Разобрать долг роль-матрицы из quick 261121-57x: (A) 23 мутирующих маршрута без роль-теста; (B) 14 чтений без Action-гейта в ROUTE_MATRIX_EXEMPT.

Purpose: закрыть класс дыр, как удалённые read_file_bytes/write_file_bytes (чтение/запись без роли и, возможно, без сессии).
Output: новые кейсы в role_endpoint_matrix.rs, authorize в build_* там, где нужно, очищенный EXEMPT, SUMMARY со списком UI-страниц для живой проверки.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@.planning/quick/261121-57x-close-audit-int-27-int-29-usb-host-gate-/261121-57x-SUMMARY.md
@crates/trackly-app/tests/role_endpoint_matrix.rs

Механика (уже есть в role_endpoint_matrix.rs, ~:6590-6860): enum Int29Roles {Staff, AdminOnly}, int29_cases(), int29_role_matrix_http (без сессии 401 / Employee 403 / Manager по роли / Admin: гейт пропускает и форма payload принята), int29_not_gate_or_shape_error, gt_env() (admin/manager/employee cookies), gt_post, post_with_cookie(app, uri, body, None) для запроса без сессии, post_with_cookie_json, gt_app, gt_seed_device_i29. Инвентарь: ROUTE_MATRIX_EXEMPT (~:6868-6976) между маркерами ROUTE-MATRIX-EXEMPT-BEGIN/END; тест http_routes_covered_by_role_matrix краснит запись EXEMPT, которая уже есть в матрице, поэтому строку нужно удалять в той же правке, где добавляется кейс (имя должно встречаться как "имя" вне EXEMPT).

Приватность: репозиторий публичный; только вымышленные данные («Иванов И.И.», «Петров П.П.», example-домены, 192.0.2.x). Не писать реквизиты организации в payload'ы и SUMMARY.
Запреты: один cargo test одновременно, на переднем плане; env TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1; при exit 69 префикс DEVELOPER_DIR=/Library/Developer/CommandLineTools; команды `timeout` нет.
</context>

<tasks>

<task type="auto">
  <name>Task 1: Часть A — роль-кейсы для 23 мутирующих маршрутов, удалить их из EXEMPT</name>
  <files>crates/trackly-app/tests/role_endpoint_matrix.rs</files>
  <action>
Для каждого маршрута группы ДОЛГ (cartridge_models_create/delete/update, cartridges_delete/update, devices_bulk_create, devices_delete, devices_import_csv_commit, requests_approve_ad_register, users_update, users_delete, users_reset_password, settings_save_org_fields, settings_save_org_logo, settings_remove_org_logo, settings_set_low_stock_threshold, settings_save_backup_config, templates_update_body, templates_reset_to_default, backup_run_manual, server_toggle, settings_set_network, desktop_set_lock) прочитать handler в crates/trackly-app/src/http/*.rs и build_*/сервис, найти authorize(Action::...) и по crates/trackly-core auth.rs определить допущенные роли Action. Payload (имена полей camelCase) взять из handler/DTO. Роли не угадывать.

Добавить кейсы по механике int29: расширить int29_cases (или завести параллельную таблицу int29b_cases с тем же драйвером) значениями Staff или AdminOnly; при необходимости добавить вариант enum (например ManageUsers = Admin only) сверяя с auth.rs. Проверки: без сессии 401, Employee 403, Manager по роли (Staff: не гейт/не ошибка формы; AdminOnly: 403), Admin: гейт пропущен и форма принята (int29_not_gate_or_shape_error). Для Admin допускается 400/404/409 бизнес-ответ, но не 401/403/415/422.

Побочные эффекты: payload для допущенных ролей строить так, чтобы мутация была безвредной (несуществующий id 999999 для delete/update; для devices_*/cartridge_*_delete по несуществующему id). users_delete / users_reset_password / users_update: завести отдельного вымышленного пользователя (gt_make_user, напр. «Орлов О.О.») и целиться в него, НЕ в admin/manager/employee, от имени которых ходит тест; если Admin-вызов удаляет цель, делать Manager-вызов (AdminOnly -> 403) до Admin-вызова. backup_run_manual: backup-путь/каталог должен быть tempdir окружения (проверить, что тестовый ctx не пишет вне tempdir; иначе проверять только отказ). server_toggle, settings_set_network, desktop_set_lock: если вызов допущенной ролью меняет процессное состояние (запуск/останов сервера, смена порта, блокировка) и безопасно изолировать нельзя, проверять только 401 без сессии и 403 Employee (и Manager там, где AdminOnly), а в комментарии у кейса и в SUMMARY записать «успешный путь не проверялся: причина». devices_import_csv_commit и settings_save_org_logo: передавать минимальный валидный/заведомо пустой payload (не файловые пути на диске сервера).

Удалить из ROUTE_MATRIX_EXEMPT все 23 строки группы ДОЛГ и пустой комментарий-хвост «ДОЛГ: файловый маршрут...» (read_file_bytes уже удалены gsd-fast). Если для маршрута нашлась реальная дыра (гейта в build_* нет, Employee получил не 403) — исправить authorize в общем build_* (не в handler) и отметить в SUMMARY.
  </action>
  <verify>
    <automated>TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test app_auth int29 && TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test app_auth http_routes_covered</automated>
  </verify>
  <done>23 маршрута покрыты кейсами (или только отказами с пометкой), из EXEMPT удалены, оба теста зелёные. Не вакуумность: временно убрать authorize в одном из build_* (напр. cartridge_models_delete) -> int29-тест красный; откатить.</done>
</task>

<task type="auto">
  <name>Task 2: Часть B шаг 1+2 — фактом установить ответ без сессии, расставить гейты в build_*</name>
  <files>crates/trackly-app/tests/role_endpoint_matrix.rs, crates/trackly-app/src/http/, crates/trackly-app/src/</files>
  <action>
Шаг 1 (факт). В role_endpoint_matrix.rs добавить тест int29_ungated_reads_probe: для 14 маршрутов (auth_me, server_status, organization_get, settings_get_org, settings_get_org_logo, settings_get_db_path, settings_get_backup_config, settings_get_low_stock_basis, settings_get_low_stock_threshold, settings_get_place_path_defaults, templates_list_for_editor, templates_get_active, devices_import_csv_preview, templates_render_preview) выполнить запрос БЕЗ сессии (post_with_cookie(..., None)) и записать статус. Метод (POST/GET) и payload взять из роутера. Сначала прогнать и зафиксировать фактическую таблицу в SUMMARY (200 без сессии = дыра).

Шаг 2 (решение по потребителям). Для каждого маршрута grep'ом в ui/src найти вызывающий код (api-обёртки и страницы/лейауты) и роль, под которой он работает: EmployeeLayout (Сотрудник), страница входа/до логина, админские/штатные страницы. Принцип: без сессии 401, кроме реально нужного странице входа (явная причина в EXEMPT: например auth_me как проба сессии на старте и organization_get/logo, если их зовёт экран входа — подтвердить по ui/src, а не предполагать); Employee — только то, что зовёт EmployeeLayout/его страницы (например organization_get/settings_get_org_logo для шапки, templates_* — скорее всего нет); служебное (settings_get_db_path, settings_get_backup_config, server_status, templates_list_for_editor, templates_render_preview, devices_import_csv_preview) — Staff (Admin|Manager) или Admin по смыслу соседней мутации (db_path/backup_config/server_status: ManageSettings; templates_*: соседний гейт templates_update_body; import_csv_preview: гейт соседнего devices_import_csv_commit). Action брать из существующих в crates/trackly-core auth.rs, новых Action не заводить без необходимости.

Где решено добавить гейт: authorize(...) в общем build_* (чтобы Tauri и HTTP одинаково). Учесть: десктоп действует от системного «Администратора» (users.is_system, memory desktop_system_admin), Identity должна доходить до build_* и из Tauri-команд — проверить, что Tauri-обёртки передают identity; если обёртка не передаёт — передать ей системного админа тем же способом, как соседние мутации. Где оставляется открытым — документировать причину и потребителя в EXEMPT.

Если меняется сигнатура build_* — поправить все вызовы (Tauri-команды, HTTP handler, тесты) и прогнать затронутые suite-бинарники из crates/trackly-app/tests/suites/ (по грепу имён build_*), по одному cargo test за раз.
  </action>
  <verify>
    <automated>cargo build -p trackly-app --tests 2>&1 | tail -5; TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test app_auth int29</automated>
  </verify>
  <done>Таблица «маршрут -> статус без сессии» зафиксирована; для каждого маршрута принято и применено решение (гейт в build_* либо обоснованное открытие); код компилируется, затронутые suite-бинарники зелёные.</done>
</task>

<task type="auto">
  <name>Task 3: Часть B — кейсы матрицы, чистка EXEMPT, полные гейты</name>
  <files>crates/trackly-app/tests/role_endpoint_matrix.rs</files>
  <action>
Для каждого гейтованного чтения добавить кейс по механике int29 (без сессии 401, Employee 403, Manager по Action, Admin пропущен и форма принята); для оставленных открытыми под Employee/без сессии — явные утверждения (Employee 200 там, где его UI зовёт маршрут; без сессии 401 везде, кроме перечисленных страницей входа). Заменить пробу из Task 2 постоянными утверждениями (не оставлять «лог статусов» без assert). Удалить из ROUTE_MATRIX_EXEMPT строки гейтованных маршрутов; оставшиеся (открытые осознанно) получают точную причину: кто потребитель и почему допустимо (например «зовёт EmployeeLayout, шапка»), общая формулировка «любой вошедший пользователь, без Action-гейта» не допускается. Обновить комментарий группы «ДОЛГ» над EXEMPT (группы больше нет).

Если гейт добавлен в UI-видимое чтение: `pnpm --dir ui build` (ui/dist для LAN) и `pnpm --dir ui lint` + svelte-check; в SUMMARY указать, какие страницы проверить живьём, особенно под «Сотрудником» (EmployeeLayout, шапка/логотип, заявки) и страницу входа до логина; пометить как UNVERIFIED, пока не проверено. Финальные гейты: cargo test -p trackly-app --test app_auth целиком, затронутые suite-бинарники, `cargo clippy -p trackly-app --all-targets -- -D warnings`, `cargo fmt --all --check`. Перед коммитом проверить на приватность (grep по изменённым файлам и SUMMARY на реальные ФИО/реквизиты). Коммитить только по просьбе оркестратора.
  </action>
  <verify>
    <automated>TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test app_auth && cargo clippy -p trackly-app --all-targets -- -D warnings && cargo fmt --all --check</automated>
  </verify>
  <done>app_auth зелёный целиком; EXEMPT без группы ДОЛГ, у каждой оставшейся «открытой» записи конкретный потребитель; SUMMARY содержит: таблицу статусов без сессии, решения по 14 чтениям, список незапускаемых успешных путей (server_toggle и др.), страницы UI для живой проверки, что не прогонялось.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| LAN-клиент -> /api/v1/* | недоверенный ввод; сессия и роль проверяются на сервере |
| Tauri invoke -> build_* | десктоп действует от системного администратора |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-fdw-01 | Elevation of Privilege | 23 мутирующих маршрута без роль-теста | mitigate | кейсы int29: 401/403 для Employee/без сессии, Manager по Action |
| T-fdw-02 | Information Disclosure | settings_get_db_path, settings_get_backup_config, server_status | mitigate | authorize(ManageSettings) в build_*, тест отказа Employee |
| T-fdw-03 | Information Disclosure | чтения, отвечающие без сессии | mitigate | проба без сессии -> 401; исключения только по странице входа с причиной |
| T-fdw-04 | Tampering | devices_import_csv_preview / templates_render_preview без гейта | mitigate | гейт Staff/Admin в build_* |
| T-fdw-05 | Denial of Service | тест вызывает server_toggle/set_network | mitigate | успешный путь не вызывается, если не изолируется; только отказы |
</threat_model>

<verification>
app_auth целиком зелёный; http_routes_covered_by_role_matrix зелёный; clippy и fmt чистые; мутационная проверка не вакуумности хотя бы по одному маршруту из A и одному из B (временно снять authorize, тест красный, откат, git diff по src пуст).
</verification>

<success_criteria>
23 маршрута части A покрыты; 14 чтений части B либо гейтованы и покрыты, либо оставлены открытыми с точной причиной; без сессии нет неожиданных 200; десктоп не сломан; SUMMARY перечисляет UI-страницы для живой проверки.
</success_criteria>

<output>
Create `.planning/quick/261121-fdw-role-matrix-debt-routes-and-ungated-read/261121-fdw-SUMMARY.md` when done
</output>
