---
phase: quick-261123-vn5
plan: 01
status: complete
completed: 2026-11-23
commits: [11dd95a7, 44577371, b0e39a40]
requirements: [DEBT-P2]
---

# Quick 261123-vn5: долг v1.4, пакет П2 «устройства, акты, отчёты — бэкенд»

Миграций нет, `bindings.ts` не менялся (DTO не тронуты). Файлы П3–П5 не трогались, кроме двух
вынужденных правок гейтов (см. «Отклонения»).

## Итог по id

| id | вердикт | коммит / тест |
|---|---|---|
| WR-03 (40.4) | исправлено | 11dd95a7. `restore_from_snapshot_in_tx`: presence-флаг `place_id`, как у `state`/`kit`. Тесты `devices_restore_snapshot` (suite `infra_tests`): A (нет ключа — место прежнее) был КРАСНЫМ (`None` против `Some(1)`), B (`null` обнуляет) и C (`N` ставит) зелёные до и после. |
| WR-05 (40.4) порядок `affected_place_ids` | исправлено | 11dd95a7. Сортировка перед записью в отчёт. Тест `import_commit_affected_place_ids_sorted` (8 мест, строки в обратном порядке) был красным (`[1,3,6,8,7,2,5,4]`). |
| 40.4 WR-02 (doc `restore_from_snapshot_in_tx`) | исправлено | 11dd95a7. Комментарий разнесён: `name`/`type_id` — COALESCE, остальные поля и `place_id` — presence-based CASE WHEN. |
| 40.4 IN-02 (`HashSet` импорт) | исправлено | 11dd95a7. `use std::collections::{HashMap, HashSet}`. |
| 40.4 IN-03 (doc `create_without_broadcast`) | исправлено | 11dd95a7. Переписан без самоссылки. |
| 40.4 IN-01 (пустая охрана `notifyPlaceContentChanged`) | перенесено | Находка в `ui/.../DevicesPage.svelte` — файл П3, не П2. Чисто косметика. |
| WARNING-8 (автор в audit_log) | исправлено | 44577371. `create`, `create_single_with_number_check_with_printer`, `bulk_create`, `delete_soft` устройства и `delete_soft_with_choices` акта пишут `caller.user_id`. Тесты `audit_author_p2` (3 шт.) были красными на HEAD-транспортах, зелёные после. SNMP-приёмка тоже пишет автора (`with_author`). Не покрыт автором `import_csv_commit` (в сервисе нет `caller`, вне плана) — остаётся `NULL`. |
| 41.1-08 (`status_id: 1` в SNMP-приёмке) | исправлено | 44577371. `default_device_status_id` в `tauri_cmds/printers.rs` читает id по коду `DEVICE_STATUS_CODE_IN_STORAGE`. Тест `snmp_admit_uses_status_by_code_and_author` перенумеровывает справочник так, что «на_складе» получает id > 1000. |
| R8-05 хвост П1 (списание из карточки) | исправлено | 44577371. В `DeviceService::update` один `operation_id` создаётся до блока вывода; `group_left` идёт через `ReleaseHistory::ReasonInOperation`, `status_changed` берёт тот же id. Тест `u11_release_shares_operation_id_with_status_change` был красным (два разных id), зелёный. `group_membership.rs` не менялся. |
| 41.12 IN-03 (`TypeLookup` требует id=1) | исправлено | b0e39a40. `from_rows`/`load` больше не требуют; новый `require_default_type` вызывается в обоих путях импорта (preview, commit). Юнит-тесты `type_lookup_*` (первый шаг красный — не компилировался из-за отсутствия `require_default_type`). |
| 41.9 IN-01 (`EntryKind::rank`) | исправлено | b0e39a40. Метод без вызовов (вызовы `.rank()` в кодовой базе относятся к источнику устройств сотрудника) удалён вместе со ссылкой на ранг в doc; порядок задаёт SQL `ORDER BY name, kind, id`. |
| WR-12 (`<title>` акта) | перенесено | Правка активного `act_handover.html` требует нового снимка `_legacy_defaults/vNN` и регистрации в `KNOWN_LEGACY_DEFAULTS` (`pdf/html_templates.rs` — файл П5), иначе установки с материализованным прежним шаблоном перестанут получать апгрейды. Практическая цена мала: `number_display` непуст, когда непуст `number`. Предложение: сделать вместе с П5 одним коммитом (`{% if act.number_display %}` + v-снимок + запись). |
| A-IN-03 (notices теряются в `update`/`update_return`) | перенесено | `update` возвращает `ActSaveOutcome`, `update_return` — `ActDto`; `notices` в них нет (`act_service.rs` ~1554/3107 только логирует). Канал означает поле `notices: Vec<ActNoticeDto>` с `#[serde(default)]` в `ActDto` + потребитель в `ActDetail`/`ReturnModal` + регенерация bindings — цепочка в файлы П4. |
| WR-05 (41.9, `DeviceFilter` применяется частично) | перенесено | По коду и doc-комментариям это осознанная матрица, а не пропуск в одной точке. Применимость: `type_id`, `status_id`, `include_deleted` — везде; `place_id`, `state` — `list`/`list_all` (не `list_grouped`); `name_prefix` — `list`/`list_all`/`export_csv` всегда, `list_grouped` только при `group_by_condition`; `count_hidden_anchor_members` читает только `include_deleted`/`status_id`/`type_id`. Выравнивание меняет контракт эндпоинтов — размер больше S. |
| WARNING-5 (таймзона `'+3 hours'`) | перенесено в v1.5 | Продуктовый выбор пояса (настройка организации против зашитого MSK) и правка `ReportTable.svelte` вне границ; цена M. |
| 39 IN-02 (V058, регистр соседних мест) | перенесено в v1.5 | `lower()`/`NOCASE` SQLite не сворачивают кириллицу; решение — сравнение через Rust `to_lowercase()` в сервисе мест (вне границы), миграция бесполезна и рискует перестройкой `places` с детьми. |

## Отклонения от плана

1. **[Rule 3] Дизайн передачи автора.** Первая версия (`*_by` рядом с каждым методом) сломала 12 гейтов `app_gates`: реестр мутаций (`entities_changed_gate`), реестры `history_write_sites_gate` и словарь статусов привязаны к ИМЕНАМ функций. Переделано на `with_author(user_id) -> Self` (копия сервиса с полем `author`, `Arc`-клон дёшев): сигнатуры, имена и тела прежних методов не менялись, 339 вызовов в тестах — нетронуты.
2. **[Rule 3] Правки двух файлов гейтов (граница П5).** `tests/entities_support/registry.rs` — две строки `ro(ACT|DEVICE, "with_author")` (по образцу `with_ws_tx`, это не мутация). `tests/device_status_vocabulary_gate.rs` — снята запись освобождения `status_id: 1` в `printers.rs` и её описание: сам гейт требовал снять её после перевода сайта на код. Других правок гейтов нет.
3. `default_status_id` сначала сделан методом сервиса, затем перенесён в `tauri_cmds/printers.rs` (гейт реестра считает каждый `pub async fn` сервиса мутацией и требует вердикт).

## Проверки (вывод)

- `cargo fmt --check` — чисто; `cargo clippy --workspace --all-targets -- -D warnings` — чисто.
- `cargo test -p trackly-core` — 193 passed.
- `cargo test -p trackly-infra` — 177 passed (suite `infra_tests`, включая 3 новых).
- `cargo test -p trackly-app --lib device_service` — 7 passed.
- `app_devices` 292 passed; `app_gates` 290 passed, 1 ignored; `app_acts` 344 passed; `app_groups` 201 passed; `app_cartridges` 88 passed (там `printers_admit`); `app_places` 91 passed.
- Сплошной прогон не запускался. `pnpm lint`/`svelte-check` не гонялись: UI и DTO не менялись.

## Живая проверка (шаги для человека, не выполнялись)

Нужна сборка десктопа с этими коммитами и `pnpm --dir ui build` для LAN-браузера.
1. Войти пользователем-менеджером, создать устройство, создать пачку из 2, удалить одно; в «Истории»/журнале аудита автор действий — этот пользователь, а не пусто.
2. Удалить акт — в журнале у удаления акта тот же автор.
3. Устройство в группе: смена статуса на «Списано» с выбором «вывести из группы» — в ленте истории одна запись операции (статус и вывод вместе), а не две.
4. Принтеры -> поиск по IP -> принять найденный: создано устройство со статусом «На складе».
5. Откат акта (удаление акта с устройствами): место устройств возвращается к прежнему, не пустеет.
6. Импорт CSV по нескольким местам: после импорта счётчики мест в дереве обновились у всех затронутых мест.

## Self-Check: PASSED

Коммиты 11dd95a7, 44577371, b0e39a40 есть в истории; новые файлы
`crates/trackly-infra/tests/devices_restore_snapshot.rs` и `crates/trackly-app/tests/audit_author_p2.rs` существуют.
Приватность: фикстуры вымышленные, хук `check-privacy` PASS на каждом коммите.
