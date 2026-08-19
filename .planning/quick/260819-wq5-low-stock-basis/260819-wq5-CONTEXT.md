# Quick Task 260819-wq5: Порог низкого остатка — выбор базы подсчёта - Context

**Gathered:** 2026-08-19
**Status:** Ready for planning

<domain>
## Task Boundary

В Настройки → «Порог низкого остатка» добавить выбор (компонент Radio): считать низкий остаток
**по модели картриджа** (текущее поведение) или **по модели принтера** (новое поведение, значение
по умолчанию). Выбор влияет на предупреждающий блок низкого остатка в Дашборде и на странице
Картриджи. Мотивация пользователя: разные модели картриджей разных производителей подходят к одной
модели принтера, и важно знать, хватает ли запаса на замену именно по моделям принтеров.

</domain>

<decisions>
## Implementation Decisions

### Хранение настройки
- Новый ключ в `app_settings`: `low_stock_basis`, значения `'cartridge_model' | 'printer_model'`.
- Миграция схемы НЕ нужна — `app_settings` это key/value, как существующий `low_stock_threshold`.
- Дефолт при отсутствующем или некорректном значении ключа — `'printer_model'`. Это намеренно
  меняет поведение уже существующих БД (пользователь подтвердил, что по умолчанию нужен именно
  подсчёт по моделям принтеров).
- Валидация значения на сервере: неизвестные строки отклоняются с ошибкой, а не пишутся в БД.

### Режим `cartridge_model` (существующее поведение — не менять)
- Группировка по `cartridge_models`, счёт картриджей `status_id = 1 AND state_id = 1 AND
  deleted_at_utc IS NULL`, `HAVING cnt < threshold`.

### Режим `printer_model` (новое)
- Источник списка «моделей принтеров» — **настройки совместимости**, а не парк устройств:
  `DISTINCT LOWER(TRIM(cartridge_model_compatibility.printer_name))`. НЕ `devices.name`.
- Для каждого такого имени считаем суммарное число картриджей
  `status_id = 1 AND state_id = 1 AND deleted_at_utc IS NULL` по ВСЕМ моделям картриджей
  (`cartridge_models.deleted_at_utc IS NULL`), у которых есть строка совместимости с этим именем.
- Порог — тот же `low_stock_threshold`.
- Отображаемое имя принтера — один из вариантов написания внутри группы (например
  `MIN(printer_name)`); группировка и сравнение — по нормализованному `LOWER(TRIM(...))`.

### Pass-through моделей без совместимости
- Модели картриджей БЕЗ строк совместимости (сейчас в `list()` трактуются как «подходят любому
  принтеру», D-05) в режиме `printer_model` **НЕ учитываются ни в одном принтере**.
- Pass-through остаётся строго в фильтре подбора картриджа в `list()` — его поведение не менять.

### Нулевые остатки
- Имя принтера, у которого совместимых картриджей на складе 0, **показывается** в предупреждении
  (0 < порога). Это осознанно: такой принтер — реальный пробел в запасе.

### Claude's Discretion
- Форма представления строки в DTO `LowStockItem` (у строки принтера нет `model_id/brand/model`):
  выбрать минимально ломающий вариант (например, `model_id: Option<i64>` + `label`, либо поле
  `basis`/`kind`), синхронно обновить `ui/src/bindings` и ключ в `{#each}` в `LowStockBanner.svelte`
  (сейчас ключ — `item.model_id`).
- Точные формулировки подписей в UI и в баннере.

</decisions>

<specifics>
## Specific Ideas

Места правок, найденные при разведке:

- `crates/trackly-infra/src/repos/cartridges_sqlite.rs:895` — `low_stock()`; ветвление по basis.
  Сохранить существующий разбор threshold в Rust с guard `> 0` и fallback `2` (комментарий WR-06 —
  `CAST(value AS INTEGER)` в SQLite молча даёт 0 на мусорной строке).
- `crates/trackly-app/src/services/dashboard_service.rs:137-179` — вторая, независимая копия того
  же SQL для виджета дашборда (`low_stock_count` / `low_stock_models`); её тоже нужно ветвить.
  Метки в дашборде — строки, поэтому в режиме принтеров это просто имя принтера.
- `crates/trackly-core/src/domain/cartridges.rs:~259` — `LowStockItem`.
- `crates/trackly-app/src/tauri_cmds/settings_org.rs:182-230` — по образцу
  `build_settings_get/set_low_stock_threshold` добавить get/set для basis.
- `crates/trackly-app/src/http/settings_org.rs` — handler'ы + маршруты
  `/api/v1/settings_get_low_stock_basis`, `/api/v1/settings_set_low_stock_basis`; требования к
  роли — те же, что у threshold.
- `crates/trackly-app/src/specta_export.rs:166` — зарегистрировать новые команды, перегенерировать
  `ui/src/bindings`.
- `ui/src/features/settings/ThresholdSettings.svelte` — Radio-группа над полем порога, из
  `$lib/components/Radio.svelte` (образец использования — `ActiveDirectorySettings.svelte`).
  Сохранение сразу при выборе; подпись поля порога должна соответствовать выбранному режиму.
- `ui/src/features/cartridges/LowStockBanner.svelte` — рендер обеих форм строки.

Тесты (обязательно обновить/дополнить):
- `crates/trackly-app/tests/cartridges_low_stock.rs`
- `crates/trackly-app/tests/dashboard_widgets.rs`
- `crates/trackly-app/tests/role_endpoint_matrix.rs` (новые эндпоинты)
- юнит-тест репозитория рядом с `low_stock_returns_models_below_threshold`
  (`cartridges_sqlite.rs:2018`) — кейс для `printer_model`, включая нулевой остаток и исключение
  модели без совместимости.

Условия запуска тестов: `trackly-app` требует `TRACKLY_AD_MOCK`/`SNMP_MOCK` и собранного
`ui/dist`; одновременно запускать не более одного `cargo test`; известный висяк
`login_remember_persistent_cookie` — пропускать через `--skip`.

ПРИВАТНОСТЬ: в тестах и артефактах — только вымышленные имена/модели, никаких реальных данных
организации (репозиторий публичный).

</specifics>

<canonical_refs>
## Canonical References

- `migrations/V032__cartridge_model_compatibility_printer_name.sql` — текущая модель совместимости
  (`cartridge_model_compatibility.printer_name`, матчинг с `devices.name` по `LOWER(TRIM(...))`).
- `CartridgeRepository::compatible_model_aggregates` (`cartridges_sqlite.rs:353`) — образец
  матчинга совместимости в SQL.

</canonical_refs>
