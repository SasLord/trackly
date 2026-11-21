---
phase: quick-261121-hei
plan: 01
status: complete
subsystem: http-api, reports, groups, server-tls
tags: [dead-api, route-parity, reports, groups, soft-delete, tls, portable]
requires: []
provides:
  - "POST /api/v1/reports_get_report_counts (гейт ReadData через общий build_*)"
  - "гейт dead_commands_have_no_ui_consumer + CMD_NO_UI_CONSUMER в http_route_parity.rs"
  - "tls::load_or_generate_self_signed(dir, host) + server.crt/server.key/server.sans в exe_dir"
affects: [ReportsPage, http_route_parity, role_endpoint_matrix, groups SQL, server start paths]
tech-stack:
  added: []
  patterns:
    - "вердикт «команда без маршрута» по причине «нет потребителя» закрепляется гейтом-сканом ui/src, а не принимается на веру"
    - "sidecar SAN-набора рядом с сертификатом: переиспользование только при равенстве множеств"
key-files:
  created: []
  modified:
    - crates/trackly-app/src/http/reports.rs
    - crates/trackly-app/src/http/{devices,number_templates,users,settings,auth}.rs
    - ui/src/features/reports/ReportsPage.svelte
    - crates/trackly-app/tests/{http_route_parity,role_endpoint_matrix,devices_http_smoke}.rs
    - crates/trackly-infra/src/repos/{groups,devices,group_types,places,repairs}_sqlite.rs
    - crates/trackly-app/src/services/{device,repair}_service.rs
    - crates/trackly-app/src/services/{group_act_plan,group_membership}.rs (тесты)
    - crates/trackly-core/src/domain/places.rs (комментарий)
    - 11 тестовых файлов trackly-app/trackly-infra (мягкое удаление групп → DELETE)
    - crates/trackly-app/src/server/tls.rs
    - crates/trackly-app/src/{main.rs,http/settings.rs,tauri_cmds/auth.rs}
    - .planning/todos/completed/2026-10-08-persist-self-signed-tls-cert.md (перенесён из pending/)
decisions:
  - "6 мёртвых HTTP-маршрутов удалены, команды Tauri оставлены (specta/bindings); 5 из них — в CMD_WITHOUT_ROUTE с гейтом «нет потребителя в ui/src»"
  - "groups.deleted_at_utc — реликт схемы: все чтения убраны, колонка оставлена без миграции"
  - "TLS: server.crt/server.key/server.sans в exe_dir; перевыпуск при смене множества SAN; срок не проверяется (rcgen 0.14.8 not_after = 4096-01-01); unix 0o600, Windows — ACL каталога"
metrics:
  completed: 2026-11-21
  tasks: 3
  commits: 4
---

# Quick 261121-hei: мёртвая поверхность API, счётчики отчётов, groups.deleted_at_utc, TLS-сертификат

Счётчики вкладок «Отчётов» получили маршрут для LAN (раньше по LAN всегда показывался 0), шесть HTTP-маршрутов без потребителя удалены, а пять оставшихся без маршрута команд Tauri теперь держит гейт, который краснеет при появлении вызова в UI. Из SQL убраны 66 мёртвых фильтров `groups.deleted_at_utc`. Самоподписанный сертификат теперь сохраняется рядом с exe и переиспользуется на всех трёх путях старта сервера.

## Коммиты

| Задача | Коммит | Что |
|---|---|---|
| 1 | d0cf391a | fix: маршрут `reports_get_report_counts`, удалены 6 мёртвых HTTP-маршрутов, гейт «нет потребителя» |
| 2 | 03e66edd | refactor: убраны мёртвые фильтры `groups.deleted_at_utc` (INT-12) |
| 2 (добивка) | ae91d8f6 | test: `request_printer_scope` — удаление группы жёсткое (тест краснел в app_server) |
| 3 | cd908afa | fix: самоподписанный сертификат сохраняется в exe_dir и переиспользуется, todo закрыт |

## Задача 1: INT-04 и INT-11/INT-24

**INT-04.** `handler_get_report_counts` в `http/reports.rs` (payload `domain/filter/period`) вызывает `build_reports_get_report_counts`. Гейт ReadData и исключение ad_register живут в этом общем build_*; своего гейта в HTTP нет. Кейс `("reports_get_report_counts", Staff, …)` добавлен в `int29_read_cases`: без сессии 401, Employee 403, Admin и Manager не получают 401/403. Вердикт удалён из `CMD_WITHOUT_ROUTE`. В `ReportsPage.svelte` пустой `.catch` заменён: `statusCounts = {}`, `console.error` и один тост «Не удалось загрузить счётчики вкладок отчётов» до следующей успешной загрузки. Флаг `countsErrorToasted` — обычный `let`, а не `$state`, как и соседний `countsLoading`. Так он не становится зависимостью эффекта автоперезагрузки (небольшое отклонение от плана).

**INT-11/INT-24.** Перед удалением повторно проверено по `ui/src` (без `bindings.ts`, snake_case и camelCase): у всех шести маршрутов вызовов нет.

| Маршрут | Что сделано |
|---|---|
| devices_create | маршрут, `handler_create` и `CreatePayload` удалены; Tauri-команда остаётся; вердикт в CMD_WITHOUT_ROUTE |
| number_template_contexts_set | маршрут и `handler_contexts_set` удалены; вердикт; Tauri-кейс 69 роль-матрицы держит гейт команды |
| users_change_password | маршрут, handler, HTTP `build_users_change_password` и payload удалены; `build_users_change_password_tauri` остаётся; вердикт |
| users_reset_password | маршрут, handler, `build_users_reset_password` и payload удалены (команды Tauri не было); `AuthService::reset_password` не тронут; вердикт убран из ROUTE_WITHOUT_CMD |
| server_status | маршрут, handler и HTTP `build_server_status` удалены; `build_server_status_tauri` остаётся; вердикт |
| auth_me | маршрут, `handler_me` и HTTP `build_auth_me` удалены; Tauri-команда остаётся; вердикт |
| printers_create (N1) | маршрута в src уже не было, ничего не делалось |

Сопутствующие правки в тестах:
- `int29_debt_mutation_cases`: убран users_reset_password, проверка теперь `22` (пересчитано от HEAD).
- `int29_read_cases`: убран server_status.
- Тест `int29_auth_me_session_probe` удалён.
- HTTP-кейс 68 для contexts_set удалён (Tauri-кейс 69 оставлен), комментарии кейсов 68/69 исправлены.
- Строка users_change_password убрана из `ROUTE_MATRIX_EXEMPT`.
- Канонические кейсы 1–3 и `devices_http_smoke` переведены на `devices_create_single_with_number_check`.
- В `every_dual_transport_build_helper_has_an_http_caller` обёртка `build_*_tauri`, чей core есть в CMD_WITHOUT_ROUTE, освобождается от требования иметь парный HTTP-хелпер.

**Новый гейт `dead_commands_have_no_ui_consumer`.** Константа `CMD_NO_UI_CONSUMER` (5 имён) обязана входить в CMD_WITHOUT_ROUTE. Гейт рекурсивно обходит `ui/src` (`*.ts`, `*.svelte`, без `bindings.ts`, не меньше 50 файлов) и ищет имя в `'…'`, `"…"` или `` `…` ``. Сообщение об ошибке называет файл и строку.

**Мутационная проверка:** в `ui/src/pages/AccessDenied.svelte` временно вставили `void apiCall('auth_me');`. Гейт покраснел (`AccessDenied.svelte:3: auth_me — void apiCall('auth_me');`), файл восстановлен из копии, git его не видит изменённым.

Отсутствие удалённых маршрутов в живом роутере подтверждает `live_router_answers_every_tauri_command_path`: 5 вердиктов сверяются с ответом на заведомо несуществующий путь. Шестой, users_reset_password, — не команда; его отсутствие видно по точному совпадению ROUTE_WITHOUT_CMD.

## Задача 2: INT-12

Инвентарь собран скриптом от репозитория (в scratchpad, не в репо). Он разрешает алиас `X.deleted_at_utc` через `FROM|JOIN <table> X`, а голое `deleted_at_utc` — через ближайший предшествующий FROM/UPDATE/INTO/JOIN. Отобраны вхождения, относящиеся к таблице `groups`: 66 предикатов (номера строк до правки):

- `trackly-app/src/services/device_service.rs`: 493 519 522 529 554 1308
- `trackly-app/src/services/repair_service.rs`: 479 494 2137 2146 2157 2220
- `trackly-infra/src/repos/devices_sqlite.rs`: 40 51 55 58 71 74 118 174 177 190 217 604 634 636 673 (алиас `c`) 678
- `trackly-infra/src/repos/group_types_sqlite.rs`: 582 681
- `trackly-infra/src/repos/groups_sqlite.rs`: 39 42 69 86 168 251 269 343 389 430 445 448 789 792 858 880 897 950 1218 1244 1269 1289 1313
- `trackly-infra/src/repos/places_sqlite.rs`: 236 241 276 287 290 405 477 480
- `trackly-infra/src/repos/repairs_sqlite.rs`: 572 579 619 626 677

Ложное срабатывание скрипта: `group_types_sqlite.rs:598`. Это `{SELECT_PROPERTIES}` — таблица `group_type_properties`; оставлено как есть, правка возвращена. Фильтры `d.`/`h.`/`p_*`/`users`/`places`/`group_types` не тронуты. Повторный прогон скрипта по src и tests даёт 0 мест для `groups`, grep-гейт из плана тоже 0.

Висячие конструкции поправлены вручную:
- в `places_sqlite.rs::list_subtree_contents` после удаления строки `WHERE g.deleted_at_utc IS NULL` строка `AND g.parent_group_id IS NULL` превращена в `WHERE …`;
- одиночные `WHERE` в рекурсивных CTE удалены, `)` подклеены к предыдущей строке.

Комментарий о том, что колонка — реликт схемы, добавлен в doc модуля `groups_sqlite.rs` и у `delete_group_in_tx`. Комментарий в `trackly-core/src/domain/places.rs:160` про «мягко удалённую» строку переписан. В `per_record_invariants.rs` у `"groups"` добавлена пометка.

**Тесты мягкого удаления групп.** Все переведены на `DELETE FROM groups`, членство уходит каскадом; ни один assert не ослаблен до тавтологии:

| Тест | Судьба |
|---|---|
| group_act_plan `load_snapshot_skips_soft_deleted_groups_and_devices` | переименован в `…_deleted_groups_and_soft_deleted_devices`; часть про устройства без изменений; группа удаляется DELETE, ожидание (пустой план) прежнее |
| group_membership `restore_deleted_group_is_skipped` и `ask_policy_…` (вторая точка, :791, в плане не было) | DELETE; Skipped сохраняется; комментарий: «удалена» и «не было» теперь неразличимы, это и есть правило |
| tests/acts_groups `update_removing_a_released_member_of_a_deleted_group_still_passes` | DELETE; все проверки прежние |
| tests/devices_group_children `group_children_deleted_or_missing_group_is_empty` | DELETE; параметр `deleted` у `seed_group` убран |
| tests/groups_anchor `group_alive`, tests/groups_membership обход | фильтр снят (группа жива, если строка существует) |
| tests/devices_container_hide (в плане не было) | параметр `deleted` убран; группа c4 удаляется DELETE после заполнения членства; d1 по-прежнему видим |
| trackly-infra act_group_effects_repo (в плане не было) | группа 3 создаётся и удаляется DELETE; ожидаемый счёт `group_devices` 5→4 (каскад), проверка вакуумности теперь «строки id=3 нет» |
| trackly-infra devices_anchor_flags, places_group_members, repairs_repo, devices_container_entries (в плане не было) | DELETE; ожидания прежние |
| tests/request_printer_scope `employee_scope_ignores_deleted_printers_hosts_and_groups` (в плане не было, app_server) | `soft_delete("groups", …)` через `format!("UPDATE {table} …")` grep не видел — тест покраснел в app_server; переведён на `delete_group` (коммит ae91d8f6) |

## Задача 3: TLS

`tls::load_or_generate_self_signed(dir, host)` подключена в `main.rs`, `http/settings.rs` (HTTP `server_toggle`) и `tauri_cmds/auth.rs` (Tauri `server_toggle`). Мёртвый блок `let _ = std::fs::write(...)` в main.rs удалён. Принятые решения:

- **Файлы:** `server.crt`, `server.key`, `server.sans` в `ctx.paths.exe_dir()`. Ключи конфига `cert_path`/`key_path` не трогались.
- **Смена host:** sidecar хранит множество SAN; переиспользование только при равенстве множеств. Это ловит смену host и смену LAN-IP или имени машины при `0.0.0.0`.
- **Срок:** в исходнике rcgen 0.14.8 `CertificateParams::default` даёт `not_after = date_time_ymd(4096, 1, 1)`, поэтому проверки срока нет. Это записано в doc функции.
- **Права ключа:** на unix 0o600, в том числе для уже существующего файла (`set_permissions`). На Windows наследуются ACL каталога, отдельного кода нет.
- **Сбои:** испорченные или неполные файлы приводят к новой генерации. Ошибки записи уходят в `tracing::warn!` с путём, bundle всё равно возвращается. Sidecar удаляется первым и пишется последним. Ключ не логируется.

Шесть unit-тестов в `tls.rs`: три файла создаются; повтор даёт тот же fingerprint (с контролем, что свежая генерация даёт другой); смена host 127.0.0.1→192.168.1.10 перевыпускает сертификат; мусор в server.crt восстанавливается; права 0o600; несуществующий каталог не мешает получить рабочий bundle. Мутации проверены:
- `try_load_saved` всегда None — 3 теста красные;
- проверка множества SAN отключена — красный `regenerates_when_host_changes`.

Тестовые AppCtx в `tls_server_smoke`, `server_drain_on_stop` и `ws_close_on_server_stop` используют `Paths::resolve()`. Если они проходят через `server_toggle` с пустым cert_path, файлы ляжут рядом с тестовым бинарником в `target/` (gitignored). `git status` после прогонов чистый.

## Тесты по бинарникам (все полные, без фильтра, TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1)

| Бинарник | Результат | Когда |
|---|---|---|
| trackly-app --lib | 404 passed | на финальном HEAD |
| app_gates | 254 passed, 1 ignored | на финальном HEAD |
| app_auth | 137 passed | на финальном HEAD |
| app_devices | 255 passed | на финальном HEAD |
| app_server | 49 passed | с изменениями задачи 3 в рабочем дереве (до коммита cd908afa) |
| app_groups | 147 passed | после задачи 2 |
| app_acts | 293 passed | после задачи 2 |
| app_places | 81 passed | после задачи 2 |
| app_repairs | 138 passed | после задачи 2 |
| app_attachments / app_reports / app_chat / app_cartridges (дополнительно) | 76 / 176 (2 ign) / 41 / 88 passed | на финальном HEAD |
| trackly-infra | 177 + 272 passed | на финальном HEAD |

## Гейты

- `cargo fmt --all -- --check` — чисто.
- `cargo clippy -p trackly-app -p trackly-infra -p trackly-core --all-targets -- -D warnings` — чисто.
- `pnpm --dir ui lint` — exit 0 (мета-гейт: 65 гейтов подключены).
- `pnpm --dir ui svelte-check` — 0 ошибок (75 предупреждений, все в чужих файлах, ReportsPage среди них нет). `pnpm --dir ui check` из verify плана такого скрипта не имеет — запускался `svelte-check`.
- `pnpm --dir ui build` — собран.
- `check-privacy` — PASS перед каждым коммитом.

## Deviations from Plan

1. **[Rule 1 — Bug]** Тестов мягкого удаления групп оказалось больше, чем в плане (4): ещё 7 файлов в trackly-app/trackly-infra и параметризованный `soft_delete("groups", …)` в `request_printer_scope.rs`, который grep не видит. Все переведены на DELETE (коммиты 03e66edd, ae91d8f6).
2. **[Rule 1 — Bug]** В `places_sqlite.rs::list_subtree_contents` механическое удаление строки `WHERE g.deleted_at_utc IS NULL` оставило бы висячий `AND`. Исправлено на `WHERE g.parent_group_id IS NULL`.
3. **[Rule 3]** HTTP-хелперы `build_server_status` и `build_auth_me` удалены вместе с маршрутами (после удаления у них не осталось вызывающих). Для их Tauri-обёрток добавлено освобождение в проверке orphan_wrappers.
4. Флаг тоста в ReportsPage сделан обычным `let`, а не `$state`: он читается только в асинхронных колбэках и не должен попадать в зависимости эффекта.

## Не проверено вживую

- Счётчики «Отчётов» в настоящем LAN-браузере на собранном сервере (проверены маршрут и роль-матрица, UI-путь вживую не открывался).
- Тост отказа счётчиков в работающем приложении.
- Стабильность сертификата между перезапусками сервера на Windows и гашение WS-тоста при возврате сервера. Права ключа на Windows — принятый остаток.
- Поведение удаления групп в UI после снятия фильтров: семантика не должна была измениться, потому что колонка всегда была NULL; проверено только тестами.

## Self-Check: PASSED

- Файлы: `crates/trackly-app/src/server/tls.rs` (pub fn load_or_generate_self_signed), `crates/trackly-app/tests/http_route_parity.rs` (dead_commands_have_no_ui_consumer), `crates/trackly-app/src/http/reports.rs` (/api/v1/reports_get_report_counts), `.planning/todos/completed/2026-10-08-persist-self-signed-tls-cert.md` — на месте.
- Коммиты d0cf391a, 03e66edd, ae91d8f6, cd908afa есть в `git log`.
