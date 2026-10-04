-- V045: схема групп устройств (Phase 41, GRP-01/02/03/05/06/10).
--
-- Только аддитивный DDL: новые таблицы, индексы и триггер.
-- Никакой перестройки существующих таблиц и никакого DROP: перестройка
-- в refinery-транзакции уже стирала данные (урок V042, act_items).
--
-- Засев встроенных типов групп и их свойств по умолчанию (D-31) делает
-- сервис при старте идемпотентно по `group_types.code` — не миграция,
-- чтобы она не зависела от содержимого БД на машине. Маркер однократного
-- засева свойств — колонка `group_types.default_props_seeded`.
--
-- CHECK на `group_types.behavior` и `group_type_properties.data_type`
-- намеренно НЕТ (GRPX-02, урок V042): набор токенов растёт, а менять
-- CHECK в SQLite можно только перестройкой таблицы. Валидация токенов —
-- на стороне Rust.
--
-- Модель значений — одна таблица `group_property_values` (EAV):
-- `value_text` для скаляров, `value_ref` для ссылок (пользователь/устройство),
-- `is_primary` отмечает основное значение многозначного свойства.
-- «Пусто» = отсутствие строки.
--
-- Членство «не более одной группы» обеспечено PRIMARY KEY
-- `group_devices.device_id` (GRP-05). UNIQUE(type_id, seq) — бэкстоп гонки
-- нумерации к `MAX(seq)+1` в единственном writer.

CREATE TABLE group_types (
  id                   INTEGER PRIMARY KEY AUTOINCREMENT,
  code                 TEXT    NOT NULL,
  name                 TEXT    NOT NULL,
  behavior             TEXT    NOT NULL,
  is_builtin           INTEGER NOT NULL DEFAULT 0,
  sort_order           INTEGER NOT NULL DEFAULT 0,
  quick_action_enabled INTEGER NOT NULL DEFAULT 0,
  quick_action_label   TEXT    NULL,
  default_props_seeded INTEGER NOT NULL DEFAULT 0,
  created_at_utc       INTEGER NOT NULL,
  updated_at_utc       INTEGER NOT NULL,
  deleted_at_utc       INTEGER NULL,
  version              INTEGER NOT NULL DEFAULT 1
);

CREATE UNIQUE INDEX idx_group_types_code ON group_types(code);

-- GRP-01: машинный код и поведение типа неизменяемы даже в обход сервиса.
-- Объявлен на UPDATE OF code, behavior — обновление остальных колонок
-- (name, default_props_seeded, ...) его не затрагивает.
CREATE TRIGGER trg_group_types_immutable
BEFORE UPDATE OF code, behavior ON group_types
WHEN NEW.code IS NOT OLD.code OR NEW.behavior IS NOT OLD.behavior
BEGIN
  SELECT RAISE(ABORT, 'group_types.code/behavior are immutable');
END;

CREATE TABLE group_type_properties (
  id              INTEGER PRIMARY KEY AUTOINCREMENT,
  type_id         INTEGER NOT NULL REFERENCES group_types(id) ON DELETE CASCADE,
  name            TEXT    NOT NULL,
  data_type       TEXT    NOT NULL,  -- text | number | ip | mac | users | device_refs (без CHECK)
  sort_order      INTEGER NOT NULL DEFAULT 0,
  is_required     INTEGER NOT NULL DEFAULT 0,
  show_on_map     INTEGER NOT NULL DEFAULT 0,
  archived_at_utc INTEGER NULL,
  created_at_utc  INTEGER NOT NULL,
  updated_at_utc  INTEGER NOT NULL,
  deleted_at_utc  INTEGER NULL,
  version         INTEGER NOT NULL DEFAULT 1
);

-- Уникальность имени среди живых свойств типа. Сравнение без учёта
-- регистра кириллицы делает сервис: lower() в SQLite её не сворачивает.
CREATE UNIQUE INDEX idx_gtp_name_live
  ON group_type_properties(type_id, name) WHERE archived_at_utc IS NULL;

CREATE TABLE groups (
  id              INTEGER PRIMARY KEY AUTOINCREMENT,
  type_id         INTEGER NOT NULL REFERENCES group_types(id) ON DELETE RESTRICT,
  name            TEXT    NOT NULL,  -- не уникальное, свободно редактируемое
  seq             INTEGER NOT NULL,
  place_id        INTEGER NULL REFERENCES places(id) ON DELETE RESTRICT,
  parent_group_id INTEGER NULL REFERENCES groups(id) ON DELETE SET NULL,
  created_at_utc  INTEGER NOT NULL,
  updated_at_utc  INTEGER NOT NULL,
  deleted_at_utc  INTEGER NULL,
  version         INTEGER NOT NULL DEFAULT 1
);

CREATE UNIQUE INDEX idx_groups_type_seq ON groups(type_id, seq);
CREATE INDEX idx_groups_parent ON groups(parent_group_id) WHERE parent_group_id IS NOT NULL;
CREATE INDEX idx_groups_place ON groups(place_id) WHERE place_id IS NOT NULL;

-- Junction: без version / deleted_at_utc. PK по device_id = «не более одной группы».
CREATE TABLE group_devices (
  device_id    INTEGER PRIMARY KEY REFERENCES devices(id) ON DELETE CASCADE,
  group_id     INTEGER NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
  added_at_utc INTEGER NOT NULL
);

CREATE INDEX idx_group_devices_group ON group_devices(group_id);

-- Значения свойств: без version / deleted_at_utc.
CREATE TABLE group_property_values (
  id             INTEGER PRIMARY KEY AUTOINCREMENT,
  group_id       INTEGER NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
  property_id    INTEGER NOT NULL REFERENCES group_type_properties(id) ON DELETE CASCADE,
  position       INTEGER NOT NULL DEFAULT 0,
  value_text     TEXT    NULL,
  value_ref      INTEGER NULL,
  is_primary     INTEGER NOT NULL DEFAULT 0 CHECK (is_primary IN (0, 1)),
  updated_at_utc INTEGER NOT NULL
);

CREATE UNIQUE INDEX uq_gpv_scalar ON group_property_values(group_id, property_id)
  WHERE value_ref IS NULL;
CREATE UNIQUE INDEX uq_gpv_ref ON group_property_values(group_id, property_id, value_ref)
  WHERE value_ref IS NOT NULL;
CREATE UNIQUE INDEX uq_gpv_primary ON group_property_values(group_id, property_id)
  WHERE is_primary = 1;
CREATE INDEX idx_gpv_property ON group_property_values(property_id);
CREATE INDEX idx_gpv_ref ON group_property_values(value_ref, property_id)
  WHERE value_ref IS NOT NULL;

PRAGMA user_version = 45;
