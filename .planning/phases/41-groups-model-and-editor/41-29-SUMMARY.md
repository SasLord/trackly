---
phase: 41-groups-model-and-editor
plan: 29
subsystem: groups
tags: [gap-closure, tdd, validation, group-types, W-B01]
requires: ["41-27"]
provides:
  - инвариант «скрытое свойство типа группы никогда не обязательное» на трёх входах сервера
affects: [GroupTypeService, group_types_sqlite, groups_types_service tests]
tech-stack:
  added: []
  patterns: ["инвариант на нескольких входах + защитный пояс для наследных строк"]
key-files:
  modified:
    - crates/trackly-app/src/services/group_type_service.rs
    - crates/trackly-infra/src/repos/group_types_sqlite.rs
    - crates/trackly-app/tests/groups_types_service.rs
key-decisions:
  - "Обязательность снимается при скрытии (archive_property, тот же UPDATE), значения остаются (D-13); след в audit_log: before_json скрытия хранит is_required=true"
  - "update_property отклоняет is_required=true у скрытого (Validation, поле is_required)"
  - "unarchive_property отказывает при наследном «скрыто+обязательное» с нарушителями и остаётся скрытым"
requirements-completed: [GRP-03]
metrics:
  duration: ~25 мин (включая 7 мин clippy)
  completed: 2026-10-05
---

# Phase 41 Plan 29: обход «обязательного» через скрытие свойства (W-B01) Summary

Закрыт GAP-3: цепочка «скрыть -> отметить обязательным -> показать» больше не оставляет живое обязательное свойство, из-за которого любой `set_values` на группе без значения падал. Инвариант «скрытое свойство не обязательное» держится на трёх входах сервера.

## Выполнено

| Задача | Коммит | Что сделано |
|--------|--------|-------------|
| 1 (RED) | 5ed8a679 | Три теста `protect_d_*` в `groups_types_service.rs` |
| 2 (GREEN) | 772bb90c | `archive_property` сбрасывает `is_required = 0` в том же UPDATE; `hidden_property_required_error()`; guard `// W-B01:1` в `update_property`; guard `// W-B01:2` в `unarchive_property`; обновлены doc-комментарии |

Отличие от дословной рекомендации верификации (одна проверка при un-hide): вторая дорога «обязательное -> скрыть -> создать группу -> показать» достижима из интерфейса, и одна проверка при un-hide заперла бы пользователя без выхода. Поэтому флаг снимается при скрытии, а проверка при un-hide осталась поясом для наследных строк (V045 не выпущена, такие строки возможны только в dev-БД).

## RED-прогон (до правки кода)

`TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test groups_types_service protect_d`:

```
test protect_d_unarchive_refuses_legacy_required_with_violators ... FAILED   (expect_err получил Ok: есть нарушители обязательности)
test protect_d_hiding_required_property_clears_flag ... FAILED               (assert: «скрытое свойство не обязательное»)
test protect_d_hide_then_require_is_rejected ... FAILED                      (шаг 2: expect_err получил Ok, is_required: true, archived: true)
test result: FAILED. 0 passed; 3 failed; 0 ignored; 0 measured; 26 filtered out
```

Все три упали по утверждениям, а не по компиляции; вакуумных нет.

## GREEN

- `cargo test -p trackly-app --test groups_types_service`: 29 passed, 0 failed (включая все `protect_*`, `rights_*`, `properties_*`).
- `cargo test -p trackly-infra --test group_types_repo`: 14 passed, 0 failed.
- `cargo clippy -p trackly-app -p trackly-infra --all-targets -- -D warnings`: чисто.
- `node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256`: 0 нарушений (гейт в pre-commit прошёл на обоих коммитах).

## Мутационная проверка

Каждая мутация применена скриптом с assert на ровно одно вхождение якоря, после прогона `git checkout -- crates/`; `git status --porcelain crates/` пуст.

| Мутация | Красный тест |
|---------|--------------|
| m1: `false &&` в условии guard-а `W-B01:1` (update_property) | `protect_d_hide_then_require_is_rejected` |
| m2: условие `current.is_required` -> `false` в guard-е `W-B01:2` (unarchive_property) | `protect_d_unarchive_refuses_legacy_required_with_violators` |
| m3: убрана строка `is_required = 0,` из SQL `archive_property` | `protect_d_hiding_required_property_clears_flag` |

Каждая мутация красит ровно один тест; остальные два остаются зелёными.

## Отклонения от плана

- Маркеры `// W-B01:1` и `// W-B01:2` после rustfmt оказались на строке под условием, а не на одной строке с ним (формат длинных условий). Счётчики маркеров по 1 сохранены; мутации якорились на строку условия непосредственно над маркером.
- Для `// W-B01:1` потребовался только один guard на строку условия; логика существующей проверки нарушителей не менялась.

Иных отклонений нет.

## Известные заглушки

Нет.

## Флаги угроз

Новых сетевых/авторизационных поверхностей нет. `authorize(Action::ManageGroupTypes)` первой строкой методов не менялся; `rights_properties_by_role` зелёный. Угрозы T-41-29-01..03 закрыты (тесты + мутации), T-41-29-04 принята.

## Ожидает ручной проверки

Не требуется: изменения серверные, покрыты автотестами. UI не менялся (в запущенном приложении НЕ проверялось). Полный `cargo test` и `pnpm lint/build` запланированы на границе волны в плане 41-34.

## Self-Check: PASSED

- Файлы: три изменённых файла существуют; коммиты 5ed8a679 и 772bb90c найдены в `git log`.
