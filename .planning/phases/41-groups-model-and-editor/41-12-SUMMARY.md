---
phase: 41-groups-model-and-editor
plan: 12
subsystem: api
tags: [rust, service, groups, property-values, validation, printers, dedup]
requires:
  - phase: 41-02
    provides: "group_values::normalize_scalar, лимиты MAX_REFS_PER_PROPERTY/TEXT_MAX_CHARS"
  - phase: 41-06
    provides: "SqliteGroupRepository: list_values, replace_property_values_in_tx, usb_printers_for_group, ref_devices_for_group"
  - phase: 41-08
    provides: "GroupService, GroupDto, GroupSnapshot"
  - phase: 41-11
    provides: "состав и вложенность групп"
provides:
  - "GroupService::set_values (серверная нормализация, живость ссылок, обязательность, CAS)"
  - "GroupService::user_options (ReadGroups, только id/full_name/login)"
  - "GroupService::card (свойства со значениями + единый список принтеров с дедупликацией)"
  - "DTO: GroupValueInputDto, GroupRefInputDto, GroupSetValuesDto, UserOptionDto, GroupUserDto, GroupPropertyValueDto, GroupPrinterDto, GroupCardDto"
  - "репозиторий: bump_group_version_in_tx, live_user_ids_in_tx, live_device_ids_in_tx, live_printer_device_ids, live_users_by_ids, property_filled_in_tx, list_user_options"
affects: [41-13 транспорты, 41-22 вкладка свойств]
tech-stack:
  added: []
  patterns:
    - "проверка живости ссылок одним запросом на вид ссылки внутри writer-транзакции; в SQL только ?-заглушки"
    - "build_card общий для чтения и ответа set_values: после записи клиент получает уже собранную карточку"
key-files:
  created:
    - crates/trackly-app/tests/groups_values_card.rs
  modified:
    - crates/trackly-infra/src/repos/groups_sqlite.rs
    - crates/trackly-app/src/dto/groups.rs
    - crates/trackly-app/src/services/group_service.rs
key-decisions:
  - "CAS по version группы проверяется ДО валидации значений: устаревшая версия всегда даёт OptimisticLockMismatch, а не ошибку поля"
  - "is_primary для device_refs отклоняется (Validation): основной бывает только у users"
  - "Принтеры по явной ссылке — только устройства со строкой printers; ссылка на не-принтер остаётся в ref_device_ids свойства, но в список принтеров не попадает"
  - "Дедупликация USB и ссылок по device_id: побеждает origin='usb' с has_explicit_link=true (долг плана 41-06 закрыт)"
requirements-completed: [GRP-03, GRP-08, GRP-09]
completed: 2026-10-04
---

# Phase 41 Plan 12: Сервис групп, часть 4 Summary

Значения свойств групп пишутся с серверной нормализацией (ip, mac, число, текст) и проверкой живости пользователей и устройств; карточка группы отдаёт типизированные значения и единый дедуплицированный список подключённых принтеров.

## Tasks

| Task | Name | Commit |
|------|------|--------|
| 1 | groups_set_values: валидация, нормализация, обязательность, CAS | 67d69cc3 |
| 2 | groups_user_options и карточка группы с принтерами и дедупликацией | 67d69cc3 |

Обе задачи правят одни и те же четыре файла и лежат одним коммитом.

## Что сделано

- `set_values`: одна writer-транзакция. Свойство обязано принадлежать типу группы и быть не скрытым, `property_id` не повторяется. Скаляры идут через `group_values::normalize_scalar`, ошибка переписывается в `Validation{field:"values.<id>"}`. Для users/device_refs проверяются лимит 100, дубликаты и не более одного основного. Живость проверяется одним запросом на вид ссылки. Обязательность считается по итоговому состоянию всех живых обязательных свойств, после чего `version` группы растёт на 1. Аудит `set_values` с `property_ids`. Любая ошибка откатывает всё.
- `user_options`: `ReadGroups`, фильтр `to_lowercase().contains` по ФИО и логину в Rust, потолок 50, запрос не длиннее 100 символов, наружу только `id`, `full_name`, `login`.
- `card`: свойства типа по `sort_order` со значениями. Мёртвые устройства и неактивные пользователи скрыты, скрытые свойства не возвращаются (значения в БД остаются). Принтеры: USB-производные плюс явные ссылки на устройства со строкой `printers`, дедупликация по `device_id`. `link_property_id` указывает на первое живое свойство `device_refs`.

## Verification

- `cargo test -p trackly-app --test groups_values_card` — 19 passed
- `groups_membership` 14 passed, `groups_service` 15 passed, `export_bindings` passed, `trackly-infra --test groups_repo` 18 passed
- `cargo clippy -p trackly-app -p trackly-infra --all-targets -- -D warnings` — чисто; `rustfmt --check` по четырём файлам чисто
- grep: `normalize_scalar` в group_service.rs — ровно 1; `LIKE` — 0; `format!("INSERT|UPDATE|SELECT` — 0
- Мутации (якорь уникален, count == 1 проверен, файл восстановлен и сверен diff'ом):
  1. вызов `normalize_scalar` заменён на «как есть» — красные все четыре `values_scalar_*`
  2. убран `seen.insert` во втором цикле принтеров — красный `card_dedup_usb_and_link_collapse_to_one_row`
- Фикстуры не вакуумны: в тесте mac перед каждой проверкой пишется другое значение; в dedup-тесте есть принтер «и USB, и ссылка», «только ссылка» и «ссылка на не-принтер».
- check-privacy в pre-commit — PASS; данные вымышленные.

## Deviations from Plan

### Auto-fixed Issues

None по правилам 1-3.

### Организационное

- Две задачи закоммичены одним коммитом: они меняют те же файлы, а `set_values` в задаче 1 сразу возвращает карточку, поэтому разнести их чисто не получилось.
- `user_options` отклоняет запрос длиннее 100 символов (Validation), а не обрезает его, как `GroupService::search`.

## Known Stubs

None.

## Threat Flags

None. T-41-12-01..06 закрыты: серверная нормализация и параметризованный SQL, живость ссылок в той же транзакции, `user_options` отдаёт только три поля (тест набора ключей JSON), Forbidden для employee (тесты), лимиты 100/2000/32/50, CAS плюс атомарная замена значений.

## Self-Check: PASSED

- groups_values_card.rs, dto/groups.rs, group_service.rs, groups_sqlite.rs — на месте
- Коммит 67d69cc3 — существует
