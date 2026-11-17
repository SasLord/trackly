---
phase: quick-261117-w2q
plan: 01
status: complete
commits: [d0258fe4, 666b22a5, b22783d0]
---

# Quick 261117-w2q: системный «Администратор» для десктопа

Статус: **complete** — живая проверка (7 пунктов, включая Gap 1 и вход по LAN) пройдена пользователем 2026-11-17: «Всё работает».

## Сделано

- V053: `users.is_system` + частичный UNIQUE-индекс `uq_users_single_system`, последняя строка `PRAGMA user_version = 53;`, без INSERT.
- `AuthService::ensure_system_admin()` (идемпотентно, вызывается в `AppCtx::build`): логин `admin`, иначе `admin-system`, `admin-system-2`...; чужую строку не присваивает; повреждённую системную (удалена/неактивна/не admin) чинит.
- `system_admin_identity()`; `resolve_tauri_identity` (unlocked) и `desktop_identity` (fallback) используют его вместо `trusted_admin()`/None.
- Защита: понизить/деактивировать/удалить системного нельзя (Validation, по-русски); пароль задать можно; по LAN без пароля войти нельзя (Unauthorized, тест).
- Системный исключён из D-Desktop-02 (`desktop_identity`), guard «последний администратор» и `desktop_identity`.
- `unwrap_or(1)` удалён в request_service и repair_service: нет user_id -> Validation «Не удалось определить автора заявки.».
- Повтор D-19 в ремонтном create только при UNIQUE-гонке (`is_unique_race`); FK возвращается с настоящей причиной.

## Тесты

`tests/desktop_system_admin.rs` (в suite `app_auth`): (а) один идемпотентный системный, (б) create ремонта/заявки от системного, (в) логин занят сотрудником -> `admin-system`, V053 на БД с users.id=1, (г) FK не маскируется, (д) защита + пароль + логин, ремонт повреждённой строки, bootstrap.

Мутационная проверка (г): guard временно заменён на `|| true` -> `fk_failure_is_not_masked_as_race` красный; восстановлено (cmp чист).

## Отклонения (Rule 1/3, правка существующих тестов под новую семантику)

- `repair_service.rs`: запрос «первый администратор» для `account_name` исключает системного (иначе подпись акта стала бы «Администратор»).
- Тесты, считавшие админов/пользователей или ждавшие `desktop_identity == None`, обновлены (`auth_bootstrap`, `auth_smoke`, `repair_create`, `request_printer_scope`, `repair_act_number_unique_migration`: версия схемы 53).
- `registry.rs` не менялся: `auth.rs` уже в OUT_OF_SCOPE_WRITERS.

## Принятые побочные эффекты

- Десктопные действия теперь имеют user_id и пишутся в audit_log с актором (раньше без записи).
- Старые десктопные заявки, приписанные users.id=1, не переписаны (неотличимы).
- DTO/UI не менялись, bindings и pnpm не нужны.

## Гейты

app_auth 127, app_repairs 121, app_server 46, app_gates 224, infra_tests 247 — зелёные; полный `cargo test --workspace -- --test-threads=1` — 0 failed; clippy `-D warnings` и fmt чисты. Гейт приватности пройден, только вымышленные имена.

## Ожидает пользователя

Чеклист в CHECKPOINT (см. ответ оркестратору).

## Gap 1 (human-verify): мастер первого запуска по LAN

Живая проверка показала FirstRunWizard при `needs_bootstrap=true`: `count_active_admins` исключал системного. Исправлено: предикат считает системного, `needs_bootstrap` на рабочем приложении всегда false, `bootstrap_first_admin` отказывает (Conflict). Маршрут/Tauri-команда не удалялись; D-Desktop-02 и guard «последний администратор» по-прежнему исключают системного. AD auto-admin (фаза 32) от предиката не зависит. Мутация (вернуть `is_system = 0` в предикат) краснит `system_admin_closes_first_run_wizard`, восстановлено (cmp чист).

**Не сделано сознательно:** отдельное русское сообщение при LAN-входе системного админа без пароля. Модель входа (D-Sec-01/T-09-20) даёт одно общее «Неверный логин или пароль» для всех отказов; особое сообщение раскрыло бы существование учётки `admin`, а UI всё, кроме нескольких кодов, и так сводит к общему тексту. Вход без пароля отказывает (Unauthorized, тест), с заданным паролем проходит.
