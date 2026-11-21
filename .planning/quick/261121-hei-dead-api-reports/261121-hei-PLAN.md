---
phase: quick-261121-hei
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/trackly-app/src/http/reports.rs
  - ui/src/features/reports/ReportsPage.svelte
  - crates/trackly-app/src/http/devices.rs
  - crates/trackly-app/src/http/number_templates.rs
  - crates/trackly-app/src/http/users.rs
  - crates/trackly-app/src/http/settings.rs
  - crates/trackly-app/src/http/auth.rs
  - crates/trackly-app/tests/role_endpoint_matrix.rs
  - crates/trackly-app/tests/http_route_parity.rs
  - crates/trackly-app/tests/devices_http_smoke.rs
  - crates/trackly-infra/src/repos/groups_sqlite.rs
  - crates/trackly-infra/src/repos/devices_sqlite.rs
  - crates/trackly-infra/src/repos/group_types_sqlite.rs
  - crates/trackly-infra/src/repos/places_sqlite.rs
  - crates/trackly-infra/src/repos/repairs_sqlite.rs
  - crates/trackly-app/src/services/device_service.rs
  - crates/trackly-app/src/services/repair_service.rs
  - crates/trackly-app/src/services/group_service.rs
  - crates/trackly-app/src/services/group_act_plan.rs
  - crates/trackly-app/src/services/group_membership.rs
  - crates/trackly-app/tests/acts_groups.rs
  - crates/trackly-app/tests/devices_group_children.rs
  - crates/trackly-app/tests/groups_anchor.rs
  - crates/trackly-app/tests/groups_membership.rs
  - crates/trackly-app/src/server/tls.rs
  - crates/trackly-app/src/main.rs
  - crates/trackly-app/src/tauri_cmds/auth.rs
  - .planning/todos/pending/2026-10-08-persist-self-signed-tls-cert.md
  - .planning/todos/completed/2026-10-08-persist-self-signed-tls-cert.md
autonomous: true
requirements: [INT-04, INT-11, INT-24, INT-12, TODO-2026-10-08-TLS]

must_haves:
  truths:
    - "INT-04: в LAN-браузере вкладки «Отчётов» показывают реальные счётчики — POST /api/v1/reports_get_report_counts отвечает 200 Admin/Manager, 403 Employee, 401 без сессии; ошибка загрузки счётчиков больше не глушится пустым .catch"
    - "INT-11/INT-24: HTTP-маршрутов devices_create, number_template_contexts_set, users_change_password, users_reset_password, server_status, auth_me в роутере нет; живой роутер отвечает на них 404 JSON"
    - "INT-11/INT-24: пять оставшихся команд Tauri без потребителя записаны в CMD_WITHOUT_ROUTE, и новый гейт краснеет, если в ui/src появится вызов любой из них"
    - "INT-12: ни один SQL-запрос в src (trackly-app, trackly-infra) не читает и не пишет groups.deleted_at_utc; удаление группы жёсткое и код больше не делает вид, что мягкое удаление есть; колонка в схеме оставлена и это записано комментарием"
    - "TLS: при пустом cert_path самоподписанный сертификат генерируется один раз, сохраняется в exe_dir (server.crt/server.key/server.sans) и переиспользуется на всех трёх путях старта сервера; при смене набора SAN (host или LAN-IP) генерируется заново"
  artifacts:
    - path: "crates/trackly-app/src/http/reports.rs"
      provides: "маршрут /api/v1/reports_get_report_counts"
      contains: "reports_get_report_counts"
    - path: "crates/trackly-app/tests/http_route_parity.rs"
      provides: "вердикты для 5 мёртвых команд + гейт dead_commands_have_no_ui_consumer"
      contains: "dead_commands_have_no_ui_consumer"
    - path: "crates/trackly-app/src/server/tls.rs"
      provides: "load_or_generate_self_signed(dir, host)"
      contains: "pub fn load_or_generate_self_signed"
  key_links:
    - from: "ui/src/features/reports/ReportsPage.svelte"
      to: "/api/v1/reports_get_report_counts"
      via: "apiCall('reports_get_report_counts') → fetch POST"
      pattern: "reports_get_report_counts"
    - from: "crates/trackly-app/src/http/reports.rs"
      to: "build_reports_get_report_counts"
      via: "handler → общий build_* (authorize ReadData)"
      pattern: "build_reports_get_report_counts"
    - from: "main.rs / http/settings.rs / tauri_cmds/auth.rs"
      to: "tls::load_or_generate_self_signed"
      via: "ветка cert_path.is_empty()"
      pattern: "load_or_generate_self_signed\\(.*exe_dir"
---

<objective>
Пакет E «мёртвая поверхность API и отчёты»: закрыть INT-04 (счётчики отчётов по LAN),
INT-11/INT-24 (HTTP-маршруты без потребителя), INT-12 (мёртвый groups.deleted_at_utc) и
todo 2026-10-08-persist-self-signed-tls-cert (сертификат пересоздаётся на каждом старте).

Purpose: убрать из кода всё, что делает вид, будто работает (маршрут, которого нет; фильтр
мягкого удаления, которого не бывает; запись сертификата, который никогда не читается), и
починить единственный реально сломанный пользовательский путь (счётчики «Отчётов» в браузере).
Output: 3 атомарных коммита (по задаче) + обновлённый todo.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@./CLAUDE.md
@.planning/todos/pending/2026-10-08-persist-self-signed-tls-cert.md

## Решения по каждому маршруту INT-11/INT-24 (проверено grep'ом по ui/src, bindings.ts исключён)

Как ходит UI: `ui/src/lib/api/client.ts::apiCall(name)` — в десктопе `invoke(name)`, в LAN
`POST /api/v1/<name>`; имя одно на оба транспорта. Поиск вёлся и по snake_case-литералам, и по
camelCase (`authMe`, `serverStatus`, `changePassword`, ...): вызовов НЕТ ни одного.

| Маршрут | Потребитель в ui/src | Решение |
|---|---|---|
| devices_create | нет (UI создаёт через `devices_create_single_with_number_check`, lib/api/devices.ts:54) | удалить маршрут + handler_create в http/devices.rs; команда Tauri остаётся (specta/bindings), вердикт в CMD_WITHOUT_ROUTE |
| number_template_contexts_set | нет (UI вызывает только `number_template_contexts_get`) | удалить маршрут + handler; команда Tauri остаётся, вердикт |
| users_change_password | нет (смены пароля сотрудником в UI нет) | удалить маршрут, handler_change_password и HTTP-хелпер build_users_change_password; Tauri-обёртка build_users_change_password_tauri остаётся, вердикт |
| users_reset_password | нет; админ меняет пароль через `users_update` с полем password (UsersPage.svelte:64-73) | удалить маршрут, handler_reset_password, build_users_reset_password (HTTP-only, команды Tauri нет); сервисный `AuthService::reset_password` НЕ трогать — на нём тесты desktop_system_admin.rs:265,363 (фикс a3dbe430) |
| server_status | нет (NetworkSettings.svelte берёт статус из ответа `server_toggle`) | удалить маршрут + handler_server_status; команда Tauri остаётся, вердикт |
| auth_me | нет (UI определяет сессию через `auth_status`, App.svelte:26) | удалить маршрут + handler_me; команда Tauri остаётся, вердикт |
| printers_create (N1 из аудита) | — | маршрута в src уже нет (grep пуст); ничего не делать, отметить в SUMMARY |

Почему команды Tauri НЕ удаляются: удаление тянет specta_export.rs, регенерацию bindings.ts,
export_bindings.rs и Tauri-кейсы роль-матрицы — за пределами пакета. Чтобы вердикты в
CMD_WITHOUT_ROUTE не ослабили гейт паритета молча, вводится гейт-проверка «у этих команд нет
потребителя в ui/src» (Задача 1).

## Интерфейсы (извлечено из кода)

tauri_cmds/reports.rs:899 —
`pub async fn build_reports_get_report_counts(ctx: &AppCtx, caller: &Identity, domain: String, filter: ReportFilter, period: PeriodDto) -> Result<ReportCountsDto, AppError>` (authorize ReadData внутри).
Образец HTTP-хендлера — `handler_export_pdf` в http/reports.rs (~строка 303): `session_identity(&session)` → `build_*` → `AppErrorResponse`.

http_route_parity.rs: `CMD_WITHOUT_ROUTE` (команды без маршрута, сверка точная — лишняя и
исчезнувшая позиция красят гейт), `ROUTE_WITHOUT_CMD`, `every_dual_transport_build_helper_has_an_http_caller`
(строка ~408: освобождает build_<core> при core ∈ CMD_WITHOUT_ROUTE, но проверка
orphan_wrappers для `build_*_tauri` освобождения НЕ знает), `verdict_tables_are_well_formed`
(reason ≥ 40 символов).

role_endpoint_matrix.rs: `int29_debt_mutation_cases()` (assert ровно 23, содержит
users_reset_password ~6957), `int29_read_cases()` (~7027, содержит server_status, роли
`Int29Roles::{AdminOnly, Staff}`), `int29_auth_me_session_probe` (~7063), `ROUTE_MATRIX_EXEMPT`
(содержит users_change_password; инвентарный тест `http_routes_covered_by_role_matrix` краснеет
и на устаревшую запись EXEMPT, ~7238). devices_create — канонические кейсы 1-3 (~673-720),
number_template_contexts_set — HTTP-кейс ~2820 (Tauri-кейс 69 ~2929 через build_* — оставить).

## INT-12: где живёт groups.deleted_at_utc

Колонка никогда не пишется прод-кодом: `delete_group_in_tx` (groups_sqlite.rs:183) делает
`DELETE FROM groups`, V048 ставит `parent_group_id ... ON DELETE SET NULL`. Чтения-фильтры
(`AND g.deleted_at_utc IS NULL`, `FROM groups WHERE ... deleted_at_utc IS NULL`, рекурсивные CTE
~445/785) разбросаны по: trackly-infra/src/repos/{groups,devices,group_types,places,repairs}_sqlite.rs,
trackly-app/src/services/{device,repair,group}_service.rs. Записи — только в тестах:
group_act_plan.rs:2556 (`load_snapshot_skips_soft_deleted_groups_and_devices`),
group_membership.rs:600 (`restore_deleted_group_is_skipped`), tests/acts_groups.rs:1246,
tests/devices_group_children.rs:242; чтения в тест-хелперах: tests/groups_anchor.rs:196,
tests/groups_membership.rs:211. ОСТОРОЖНО: алиас `d.` и фильтры по devices/users/places/cartridges
(`d.deleted_at_utc`, `FROM devices WHERE deleted_at_utc`) — ЖИВЫЕ, их не трогать.

## TLS: текущее состояние

Три точки делают `tls::generate_self_signed(&host)` при пустом cert_path: main.rs:196-200,
http/settings.rs:~274-279 (кнопка «Запустить сервер»), tauri_cmds/auth.rs:~102-107.
main.rs:202-207 пишет server.crt/server.key в `ctx.paths.exe_dir()` через `let _ =` и никогда
их не читает. tls.rs: `TlsBundle { acceptor, fingerprint_hex, cert_pem, key_pem }`,
`fn collect_subject_alt_names(host) -> Vec<String>` (приватная, ~183),
`generate_self_signed(host)`, `load_from_files(cert, key)`, `load_from_pem`. `tempfile` — обычная
зависимость trackly-app.
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: INT-04 маршрут счётчиков отчётов + INT-11/INT-24 удаление мёртвых HTTP-маршрутов с гейтом «нет потребителя»</name>
  <files>crates/trackly-app/src/http/reports.rs, ui/src/features/reports/ReportsPage.svelte, crates/trackly-app/src/http/devices.rs, crates/trackly-app/src/http/number_templates.rs, crates/trackly-app/src/http/users.rs, crates/trackly-app/src/http/settings.rs, crates/trackly-app/src/http/auth.rs, crates/trackly-app/tests/role_endpoint_matrix.rs, crates/trackly-app/tests/http_route_parity.rs, crates/trackly-app/tests/devices_http_smoke.rs</files>
  <behavior>
    - reports_get_report_counts в int29_read_cases как Staff: без сессии 401, Employee 403, Admin/Manager не 401/403 (валидное тело domain/filter/period)
    - http_route_parity: reports_get_report_counts исчезает из CMD_WITHOUT_ROUTE; 5 мёртвых команд появляются там; users_reset_password исчезает из ROUTE_WITHOUT_CMD; все тесты файла зелёные
    - новый тест dead_commands_have_no_ui_consumer зелёный и краснеет при вставке apiCall('auth_me') в любой ui/src/*.svelte (мутационная проверка, откатить)
    - живой роутер на /api/v1/auth_me и остальные 5 удалённых отвечает 404 (существующий live_router_answers_every_tauri_command_path это уже проверяет через CMD_WITHOUT_ROUTE)
  </behavior>
  <action>
INT-04. В http/reports.rs добавить payload-структуру (domain: String, filter: ReportFilter, period: PeriodDto — тот же serde-атрибут rename_all, что у соседних payload'ов в файле) и `handler_get_report_counts` по образцу handler_export_pdf, вызывающий `build_reports_get_report_counts` из tauri_cmds/reports.rs (никакой своей логики/authorize — гейт ReadData уже в build_*), ответ Json(ReportCountsDto). Зарегистрировать `.route("/api/v1/reports_get_report_counts", post(...))` в router(). В http_route_parity.rs удалить Verdict reports_get_report_counts из CMD_WITHOUT_ROUTE (комментарий в нём сам это требует) и поправить в шапке файла абзац про «счётчики Отчётов» до прошедшего времени (исправлено quick 261121-hei). В role_endpoint_matrix.rs добавить ("reports_get_report_counts", Staff, json!(...)) в int29_read_cases; тело собрать по DTO ReportFilter/PeriodDto (domain "devices"; filter и period — минимально валидные значения из dto/reports.rs). В ReportsPage.svelte (~465) заменить пустой .catch: при ошибке statusCounts = {} (бейджи уходят в прежний фолбэк), console.error с ошибкой и pushToast('error', 'Не удалось загрузить счётчики вкладок отчётов') — не чаще одного раза до следующей успешной загрузки (флаг $state, сбрасывается в .then), чтобы эффект автоперезагрузки не спамил тостами.

INT-11/INT-24 (решения — таблица в context). Удалить .route(...) и соответствующие handler'ы: devices_create (http/devices.rs:189,515 — handler_create только здесь), number_template_contexts_set (http/number_templates.rs ~223,270; убрать неиспользуемый импорт build_number_template_contexts_set), users_change_password и users_reset_password (http/users.rs: handler_change_password, handler_reset_password, build_users_change_password, build_users_reset_password и их payload-структуры; поправить модульный doc строки 3), server_status (http/settings.rs: handler_server_status, строка 7 doc), auth_me (http/auth.rs: handler_me). Удалять только то, что после этого реально не используется — компилятор и clippy -D warnings подскажут; сервис AuthService::reset_password не трогать.

Тесты-последствия: role_endpoint_matrix.rs — убрать users_reset_password из int29_debt_mutation_cases и поменять assert 23→22 (и текст «ровно 23»); убрать server_status из int29_read_cases и поправить doc-комментарий над функцией; удалить тест int29_auth_me_session_probe целиком; удалить HTTP-кейс number_template_contexts_set (~2820, вместе с неиспользуемой после этого переменной payload, если она больше нигде не нужна — Tauri-кейс 69 её использует? проверить); удалить строку users_change_password из ROUTE_MATRIX_EXEMPT; канонические кейсы 1-3 (~673-720) и шапку файла строки 7-9 перевести с devices_create на /api/v1/devices_create_single_with_number_check с телом, как в существующем кейсе ~3110. devices_http_smoke.rs:51-69 — тот же перевод на devices_create_single_with_number_check (смысл теста «без сессии 401» сохраняется). http_route_parity.rs — удалить Verdict users_reset_password из ROUTE_WITHOUT_CMD; добавить в CMD_WITHOUT_ROUTE 5 Verdict (devices_create, number_template_contexts_set, users_change_password, server_status, auth_me) с reason вида «мёртвая поверхность (INT-11/INT-24): потребителя нет ни в одном транспорте — в ui/src вызова нет; HTTP-маршрут удалён quick 261121-hei, команда Tauri оставлена ради specta/bindings; появление потребителя красит dead_commands_have_no_ui_consumer» + конкретика по каждой (чем UI пользуется вместо). В every_dual_transport_build_helper_has_an_http_caller в цикле orphan_wrappers пропускать обёртку, чей core без префикса build_ входит в CMD_WITHOUT_ROUTE (иначе build_users_change_password_tauri станет «сиротой»). Новый #[test] dead_commands_have_no_ui_consumer: константа CMD_NO_UI_CONSUMER с 5 именами; assert, что каждое ⊂ CMD_WITHOUT_ROUTE; рекурсивный обход ui/src (путь от CARGO_MANIFEST_DIR/../../ui/src) по *.ts и *.svelte, кроме bindings.ts; нижняя граница ≥ 50 просканированных файлов; для каждого имени — нет вхождения литерала в одинарных, двойных или обратных кавычках; сообщение об ошибке называет файл и строку. Мутационная проверка: временно вставить apiCall('auth_me') в любой .svelte, увидеть красный, откатить; результат записать в SUMMARY. Поправить в шапке http_route_parity.rs устаревшие счётчики только если они упоминают изменённые позиции.
  </action>
  <verify>
    <automated>cd /Users/madsas/Projects/trackly && TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test app_gates http_route_parity && TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test app_auth role_endpoint_matrix && TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test app_devices devices_http_smoke && pnpm --dir ui lint && pnpm --dir ui check && grep -c "/api/v1/reports_get_report_counts" crates/trackly-app/src/http/reports.rs && test "$(grep -rhoE '"/api/v1/(devices_create|number_template_contexts_set|users_change_password|users_reset_password|server_status|auth_me)"' crates/trackly-app/src/http | wc -l | tr -d ' ')" = "0"</automated>
  </verify>
  <done>Маршрут счётчиков отвечает по LAN с ролевым гейтом ReadData; 6 мёртвых маршрутов удалены, их следы убраны из роль-матрицы/EXEMPT/вердиктов; 5 оставшихся Tauri-команд закреплены гейтом «нет потребителя» с подтверждённой мутацией; пустой .catch в ReportsPage заменён; app_gates/app_auth(фильтр)/app_devices(фильтр), lint, check зелёные. Коммит: fix(quick-261121-hei): маршрут счётчиков отчётов по LAN, удалены 6 мёртвых HTTP-маршрутов (INT-04, INT-11/INT-24).</done>
</task>

<task type="auto">
  <name>Task 2: INT-12 — убрать мёртвые чтения/записи groups.deleted_at_utc</name>
  <files>crates/trackly-infra/src/repos/groups_sqlite.rs, crates/trackly-infra/src/repos/devices_sqlite.rs, crates/trackly-infra/src/repos/group_types_sqlite.rs, crates/trackly-infra/src/repos/places_sqlite.rs, crates/trackly-infra/src/repos/repairs_sqlite.rs, crates/trackly-app/src/services/device_service.rs, crates/trackly-app/src/services/repair_service.rs, crates/trackly-app/src/services/group_service.rs, crates/trackly-app/src/services/group_act_plan.rs, crates/trackly-app/src/services/group_membership.rs, crates/trackly-app/tests/acts_groups.rs, crates/trackly-app/tests/devices_group_children.rs, crates/trackly-app/tests/groups_anchor.rs, crates/trackly-app/tests/groups_membership.rs</files>
  <action>
Per решение пакета (findings INT-12): удаление групп жёсткое, колонку схемы не трогаем, миграцию не пишем. Сначала инвентарь ОТ репозитория (урок «инвентарь от репозитория, не по памяти»): скриптом в scratchpad (не в репо) вывести все SQL-литералы в crates/trackly-app/src и crates/trackly-infra/src, где встречается deleted_at_utc, вместе с таблицей/алиасом, к которому он относится; отобрать относящиеся к таблице groups (алиас, объявленный как `groups g`/`groups gr`/любой иной; голое `FROM groups ... deleted_at_utc`; рекурсивные CTE в groups_sqlite.rs ~445 и ~785; `INSERT INTO groups` со столбцом deleted_at_utc). Список (файл:строка) положить в SUMMARY. Затем по каждому месту удалить предикат, сохранив корректность SQL: `AND g.deleted_at_utc IS NULL` убрать целиком; `WHERE g.deleted_at_utc IS NULL AND X` → `WHERE X`; одиночный `WHERE g.deleted_at_utc IS NULL` → убрать WHERE (проверить, что дальше не идёт AND/ORDER BY с висячей конструкцией); в JOIN-условии `JOIN groups g ON g.id = gd.group_id AND g.deleted_at_utc IS NULL` → только равенство (семантика LEFT JOIN не меняется, т.к. колонка всегда NULL). Если INSERT явно пишет deleted_at_utc = NULL — убрать столбец из списка. Фильтры по devices/users/places/cartridges (d., h., голые devices/users) НЕ трогать — они живые. В groups_sqlite.rs у модуля (или у delete_group_in_tx) добавить комментарий: «groups.deleted_at_utc — реликт схемы: прод-код его не пишет и не читает, удаление группы жёсткое (DELETE, каскады group_devices/значений, parent_group_id SET NULL по V048). Колонка оставлена, чтобы не перестраивать таблицу миграцией (INT-12)». Аналогичную строку-уточнение — у комментария trackly-core/src/domain/places.rs:160, только если он после правки вводит в заблуждение.

Тесты, которые эмулировали мягкое удаление группы (UPDATE groups SET deleted_at_utc): group_act_plan.rs `load_snapshot_skips_soft_deleted_groups_and_devices`, group_membership.rs `restore_deleted_group_is_skipped`, tests/acts_groups.rs `update_removing_a_released_member_of_a_deleted_group_still_passes`, tests/devices_group_children.rs `group_children_deleted_or_missing_group_is_empty`. Для каждого: если смысл переносится на жёсткое удаление — заменить UPDATE на `DELETE FROM groups WHERE id = ?` и при необходимости поправить ожидания с учётом каскада group_devices (перечитать assert'ы, не ослаблять их до тавтологии); если смысл держался только на мягком удалении (а в group_act_plan — часть про devices живая: её сохранить, вырезав только группу) — переименовать/сократить тест и объяснить в SUMMARY. Тест-хелперы groups_anchor.rs:196 (`group_alive`) и groups_membership.rs:211 — убрать фильтр по deleted_at_utc (group_alive = строка существует). Затем grep tests/ и crates/trackly-infra/tests на `groups` + deleted_at_utc — остатков быть не должно.
  </action>
  <verify>
    <automated>cd /Users/madsas/Projects/trackly && test "$(grep -rnE '\bg\.deleted_at_utc\b|FROM groups[^"]*deleted_at_utc|UPDATE groups SET deleted_at_utc' crates/trackly-app/src crates/trackly-infra/src crates/trackly-app/tests crates/trackly-infra/tests | grep -v '^\s*//' | wc -l | tr -d ' ')" = "0" && cargo test -p trackly-infra && TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --lib group && TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test app_groups && TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test app_devices && TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test app_acts && TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test app_places && TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test app_repairs</automated>
  </verify>
  <done>Инвентарь мест в SUMMARY; в src/ и tests/ нет ни одного чтения/записи groups.deleted_at_utc (grep-гейт = 0, алиас-проверка ручная по инвентарю — не только `g.`); комментарий-реликт в groups_sqlite.rs; затронутые тесты переведены на жёсткое удаление или сокращены с обоснованием; trackly-infra, lib-тесты group*, app_groups/app_devices/app_acts/app_places/app_repairs зелёные (по одному cargo за раз). Коммит: refactor(quick-261121-hei): убраны мёртвые фильтры groups.deleted_at_utc — удаление групп жёсткое (INT-12).</done>
</task>

<task type="auto" tdd="true">
  <name>Task 3: Самоподписанный TLS-сертификат — генерировать один раз, хранить в exe_dir, переиспользовать на всех трёх путях</name>
  <files>crates/trackly-app/src/server/tls.rs, crates/trackly-app/src/main.rs, crates/trackly-app/src/http/settings.rs, crates/trackly-app/src/tauri_cmds/auth.rs, .planning/todos/pending/2026-10-08-persist-self-signed-tls-cert.md</files>
  <behavior>
    - первый вызов load_or_generate_self_signed(tempdir, "127.0.0.1") создаёт server.crt, server.key, server.sans
    - второй вызов с тем же host возвращает тот же fingerprint_hex (сертификат переиспользован)
    - вызов с другим host ("192.168.1.10" после "127.0.0.1") даёт другой fingerprint и перезаписывает server.sans
    - испорченный server.crt (мусор) → без ошибки, новый сертификат, файлы перезаписаны
    - cfg(unix): права server.key = 0o600
    - каталог только для чтения / ошибка записи → функция всё равно возвращает рабочий bundle (ошибка уходит в tracing::warn!, не молча)
  </behavior>
  <action>
Объём S — реализуем по todo (решения, которые todo просит не принимать молча, фиксируются здесь и в doc-комментарии функции). Имена и каталог: server.crt / server.key (уже заданы мёртвым кодом main.rs:202-207) + sidecar server.sans, все в `ctx.paths.exe_dir()` — портативно, без %APPDATA%. Перегенерация при смене host: sidecar хранит набор SAN, полученный `collect_subject_alt_names(host)` (по строке на SAN); переиспользуем, только если сохранённый набор как множество равен текущему — это покрывает и смену host в Настройках, и смену LAN-IP при host=0.0.0.0 (SAN включают реальные адреса). Истечение: проверить в исходнике rcgen (версия из Cargo.lock), что `generate_simple_self_signed` по умолчанию ставит not_after в далёкое будущее (ожидается 4096-01-01); если так — отдельной проверки срока не делать и написать это в doc; если срок ограничен — записать not_after в sidecar второй строкой-ключом и перегенерировать заранее (за 30 дней). Ключи конфига cert_path/key_path не трогаем и дефолтом не доопределяем — ветка пустого cert_path просто перестаёт генерировать каждый раз. Права ключа: на unix файл ключа создавать с mode 0o600 (std::os::unix::fs::OpenOptionsExt под cfg(unix)); на Windows — наследуются ACL портативного каталога, отдельного кода нет (в doc и в todo явно).

В tls.rs добавить `pub fn load_or_generate_self_signed(dir: &Path, host: &str) -> anyhow::Result<TlsBundle>`: если server.crt, server.key, server.sans существуют, набор SAN совпадает и `load_from_files` успешен — вернуть его (tracing::info! «TLS: переиспользован сохранённый самоподписанный сертификат»); иначе `generate_self_signed(host)`, записать три файла (ключ — с ограниченными правами; запись sidecar последней, чтобы частичная запись не давала ложного «совпадения»), ошибки записи — tracing::warn! с путём и ошибкой, НЕ `let _ =`, и всё равно вернуть bundle (сервер должен стартовать). Ключ не логировать. Тесты из behavior — в #[cfg(test)] mod tests в tls.rs на tempfile::tempdir (никаких реальных адресов/имён организации; только 127.0.0.1/192.168.1.10/localhost, как в существующих тестах).

Подключить на всех трёх путях: main.rs — ветку cert_path.is_empty() заменить на load_or_generate_self_signed(ctx.paths.exe_dir(), &host) и удалить мёртвый блок «Save generated cert/key to exe_dir for reuse» (202-207); http/settings.rs (~274) и tauri_cmds/auth.rs (~104) — то же, map_err в AppError::Internal с source_chain "load_or_generate_self_signed: {e}". Убедиться, что тестовые AppCtx в app_server берут exe_dir из temp (Paths::resolve_for_exe_dir) — если какой-то тест-хелпер использует Paths::resolve(), файлы лягут в target/, это допустимо (gitignored), но отметить в SUMMARY. В todo-файле дописать раздел «Решено quick 261121-hei» с принятыми решениями и перенести файл через git mv в .planning/todos/completed/ (соглашение каталога: pending/ и completed/).
  </action>
  <verify>
    <automated>cd /Users/madsas/Projects/trackly && TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --lib server::tls && TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test app_server && test "$(grep -rn 'generate_self_signed(&host)' crates/trackly-app/src/main.rs crates/trackly-app/src/http/settings.rs crates/trackly-app/src/tauri_cmds/auth.rs | wc -l | tr -d ' ')" = "0" && test "$(grep -c 'let _ = std::fs::write' crates/trackly-app/src/main.rs)" = "0"</automated>
  </verify>
  <done>Сертификат генерируется один раз и переиспользуется на старте, кнопке «Запустить сервер» и Tauri-команде; смена SAN перегенерирует; ошибки записи видны в логе; unit-тесты tls (6 сценариев) и app_server зелёные; todo закрыт с записанными решениями. Коммит: fix(quick-261121-hei): самоподписанный сертификат сохраняется в exe_dir и переиспользуется (todo 2026-10-08).</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| LAN-браузер → axum /api/v1/* | недоверенный ввод и сессия |
| файловая система exe_dir → TLS-загрузчик | файлы server.crt/server.key может подменить локальный пользователь с правами на каталог |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-hei-01 | Elevation of Privilege | новый маршрут reports_get_report_counts | mitigate | гейт только через build_reports_get_report_counts → authorize(ReadData); кейс в int29_read_cases (401/403/OK) + инвентарь http_routes_covered_by_role_matrix |
| T-hei-02 | Information Disclosure | счётчики отчётов для ad_register | mitigate | excludes_ad_register берётся из caller в build_* — HTTP не дублирует логику |
| T-hei-03 | Tampering | удаление маршрутов | mitigate | живой роутер-тест подтверждает 404 на удалённые пути; гейт dead_commands_have_no_ui_consumer не даёт молча вернуть потребителя без маршрута |
| T-hei-04 | Information Disclosure | server.key в exe_dir | mitigate | unix 0o600; Windows — ACL портативного каталога (принятый остаток, записан в todo); ключ не логируется |
| T-hei-05 | Spoofing | подмена server.crt/key в exe_dir | accept | кто может писать в каталог exe, тот может подменить и сам бинарник; путь cert_path для собственного сертификата не меняется |
| T-hei-06 | Tampering | SQL-правки INT-12 | mitigate | только удаление всегда-истинных предикатов по groups; полные скоупленные сюиты groups/devices/acts/places/repairs + trackly-infra |
</threat_model>

<verification>
- Гейты после всех задач (по одному cargo за раз): `cargo fmt --all -- --check` (сверять только свои файлы — дрейф мог быть до задачи), `cargo clippy -p trackly-app -p trackly-infra --all-targets -- -D warnings`, `pnpm --dir ui lint`, `pnpm --dir ui check` — это отдельные команды, lint не включает check.
- При exit 69 на линковке — префикс `DEVELOPER_DIR=/Library/Developer/CommandLineTools`. Команды `timeout` на машине нет — не оборачивать.
- `ui/dist` должен быть собран (`pnpm --dir ui build`) для тестов trackly-app; после правки ReportsPage пересобрать.
- Перед КАЖДЫМ коммитом: `node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256`; в тестах и фикстурах только вымышленные данные, в SUMMARY — никаких реальных реквизитов/ФИО/адресов.
- Коммиты атомарные (по задаче), сообщения заканчиваются строкой `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
</verification>

<success_criteria>
- В LAN-браузере вкладки «Отчётов» получают счётчики с сервера; отказ виден тостом, а не молчит.
- В роутере нет ни одного маршрута без потребителя из списка INT-11/INT-24; гейт паритета не ослаблен — освобождения закреплены проверкой «нет потребителя».
- Код больше не изображает мягкое удаление групп.
- Перезапуск сервера при пустом cert_path отдаёт браузерам тот же сертификат (fingerprint стабилен), пока не изменились host/SAN.
</success_criteria>

<output>
Create `.planning/quick/261121-hei-dead-api-reports/261121-hei-SUMMARY.md` when done: решения по 6 маршрутам (+ printers_create уже отсутствовал), результат мутационной проверки нового гейта, инвентарь мест INT-12, судьба 4 тестов мягкого удаления, принятые TLS-решения (имена, SAN-sidecar, срок rcgen, права ключа), результаты гейтов.
</output>
