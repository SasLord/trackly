---
phase: quick-261121-fdw
plan: 01
status: complete
requirements: [INT-29]
commits:
  - 92f103f0 fix(quick-261121-fdw): гейт роли и сессии на 13 чтениях
  - 8dcb5134 test(quick-261121-fdw): роль-матрица 23 мутаций + 13 чтений, чистый EXEMPT
---

# Quick 261121-fdw: долг роль-матрицы (INT-29)

## Находки безопасности: ответ БЕЗ сессии

Фактическая проба (запрос без cookie, до правок):

| Маршрут | Статус без сессии | Решение |
|---|---|---|
| auth_me | 401 | оставлен, теперь в матрице (любая роль 200 на свою запись) |
| **server_status** | **200** | закрыт: ManageSettings |
| **organization_get** | **200** (реквизиты организации) | закрыт: ReadData |
| settings_get_org / _org_logo / _db_path / _backup_config / _low_stock_basis / _low_stock_threshold / _place_path_defaults | 401 | + ManageSettings |
| templates_list_for_editor | 401 | + ManageSettings |
| **templates_get_active** | **200** (тело шаблона) | закрыт: ManageSettings |
| devices_import_csv_preview | 401 | + MutateDevices |
| **templates_render_preview** | **404 без сессии** (рендер акта 1; при существующем акте отдал бы HTML реального акта) | закрыт: ManageSettings |

Четыре маршрута отвечали без сессии; страница входа их не зовёт (она зовёт только `auth_status`,
`auth_login`, `auth_bootstrap`, `request_ad_restore`). Все исправлены, исключений для страницы входа нет.

## Потребители чтений (ui/src) и роли

| Маршрут | Кто зовёт в UI | Роль | Решение |
|---|---|---|---|
| settings_get_org, settings_get_org_logo, settings_get_place_path_defaults | OrgSettings.svelte | admin (sidebar-config: Настройки только admin) | ManageSettings |
| settings_get_db_path | StorageSettings.svelte | admin | ManageSettings |
| settings_get_backup_config | BackupSettings.svelte | admin | ManageSettings |
| settings_get_low_stock_threshold / _basis | ThresholdSettings.svelte | admin | ManageSettings |
| templates_list_for_editor | TemplateEditor.svelte | admin | ManageSettings |
| devices_import_csv_preview | DeviceImportCsvModal в DevicesPage | Admin/Manager | MutateDevices (как commit) |
| server_status | нет вызовов | - | ManageSettings |
| organization_get | нет (мёртвая обёртка lib/api/organization.ts) | - | ReadData |
| templates_get_active, templates_render_preview | нет (мёртвая обёртка lib/api/templates.ts) | - | ManageSettings |
| auth_me | нет вызовов через apiCall | любая | без Action-гейта, тест на 401 / 200 для 3 ролей |

EmployeeLayout вызывает только `auth_logout`; Employee не затронут. Десктоп (Tauri) идёт через
`resolve_tauri_identity` (системный Администратор) во всех обновлённых обёртках.

## Что сделано

- **Часть A.** 23 мутирующих маршрута в `int29_debt_mutation_cases()` через общий драйвер
  `int29_check_cases` (без сессии 401, Employee 403, Manager по Action, Admin: гейт пропущен и форма принята).
  Роли по коду: MutateCartridges/MutateDevices (Admin|Manager) для картриджей и устройств;
  ManageUsers (Admin) для `requests_approve_ad_register` и `users_*`; ManageSettings (Admin) для настроек,
  шаблонов, бэкапа, сети, `server_toggle`, `desktop_set_lock`. Реальных дыр в мутациях не найдено.
- **Часть B.** `authorize` добавлен в общие `build_*` (сигнатуры получили `caller`), HTTP-handler'ы
  передают session identity, Tauri-обёртки берут `resolve_tauri_identity`. 13 чтений в
  `int29_read_cases()`. `build_backup_run_manual` тоже получил гейт в `build_*` (был только в HTTP-handler).
- `ROUTE_MATRIX_EXEMPT`: удалены 23 строки ДОЛГ, хвостовой комментарий и 14 чтений; осталось только
  read-only/публичное/самообслуживание с конкретной причиной. Проба заменена постоянными assert'ами.

## Успешный путь не проверялся (причина)

- `settings_set_network`: port 0 отбивается валидацией после гейта (смена хоста/порта сервера не выполняется).
- `backup_run_manual`: папка не задана, 400 после гейта (запись вне tempdir исключена).
- `settings_save_org_logo`, `devices_import_csv_commit`: пустой/заведомо неверный payload, без файловых путей.
- `server_toggle` выполняется с `enable:false` (сервера в тестовом ctx нет), запуск не проверялся.
- Цели мутаций: несуществующий id 999999, побочных эффектов вне tempdir нет.

## Доказательство не-вакуумности

- A: убран `authorize` из `build_cartridge_models_delete` -> `int29_debt_mutations_role_matrix_http` красный
  («Employee -> cartridge_models_delete»), откат через `git checkout`.
- B: заглушён `authorize` в `build_settings_get_db_path` -> `int29_ungated_reads_role_matrix_http` красный
  («Employee -> settings_get_db_path»), откат; `git diff` по src после отката совпал с закоммиченным.
- Реальность дыр B подтверждена пробой до правок (таблица выше, 200 без сессии).

## Команды и счётчики (TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1, последовательно, на переднем плане)

- `cargo test -p trackly-app --test app_auth`: 137 passed, 0 failed.
- `cargo test -p trackly-app --lib place_path`: 11 passed (тесты, звавшие изменённый build_*).
- `--test app_devices`: 239 passed; `--test template_edit`: 6 passed; `--test templates_status`: 3 passed;
  `--test app_server`: 48 passed.
- `cargo clippy -p trackly-app --all-targets -- -D warnings`: чисто. `cargo fmt --all --check`: чисто.
- ui не менялся: `pnpm lint`/`svelte-check`/`pnpm build` не запускались (не требуется).
- Полный сплошной прогон workspace: НЕ запускался (UNVERIFIED вне перечисленных целей).

## Побочные наблюдения (не исправлялись, вне плана)

- `users_reset_password` под Admin для несуществующего user id 999999 отвечает 200 OK (тихий no-op).
- `settings_save_org_logo` принимает пустой `logoBytes` с 200 OK.

## Проверить живьём

UNVERIFIED в запущенном приложении. Желательно:
- Десктоп (системный Администратор): Настройки -> все 6 вкладок открываются, данные подгружаются
  (организация/логотип, хранилище, бэкап, порог, шаблоны, путь места).
- Устройства -> импорт CSV (превью) под Admin и под Manager.
- LAN-браузер под Manager: Настройки недоступны; под Employee: заявки и шапка работают как раньше.
- Страница входа до логина: открывается и входит (`auth_status`, `auth_login` не менялись).

## Deviations from Plan

None по логике. Task 2/3 объединены в один fix-коммит и один test-коммит (тестовый файл общий).

## Self-Check: PASSED
