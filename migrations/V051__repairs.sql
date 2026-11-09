-- V051: ремонтные заявки (фаза 41.4).
--
-- Что делает миграция:
--   1. Номерные шаблоны (D-28): тип repair_act и контекст repair_act_create.
--      Обе таблицы перестраиваются: ребёнок (number_template_contexts) раньше
--      родителя (number_templates), строки контекстов сохраняются во временной
--      таблице с прежними template_id.
--   2. requests (D-26, D-16, D-19): request_type += repair, status += in_repair,
--      колонки repair_origin_place_id / repair_moved, частичный уникальный
--      индекс «одна открытая ремонтная заявка на устройство».
--   3. request_categories: флаг is_active, категория «Ремонт техники» скрыта,
--      а не удалена.
--   4. Новые таблицы: repair_visits (D-13), request_participants (D-20),
--      repair_visit_scans (D-06).
--
-- PRAGMA foreign_keys = OFF в файле не ставится: в транзакции refinery это
-- no-op; FK на время прогона отключает раннер (db/migrations.rs).

-- ---------------------------------------------------------------------------
-- 1. Номерные шаблоны
-- ---------------------------------------------------------------------------
CREATE TEMP TABLE _ntc_saved AS
  SELECT context, template_id, updated_at_utc FROM number_template_contexts;

DROP TABLE number_template_contexts;

CREATE TABLE number_templates_new (
  id              INTEGER PRIMARY KEY AUTOINCREMENT,
  type            TEXT    NOT NULL CHECK (type IN ('device_inventory', 'act_number', 'cartridge_code', 'drum_code', 'repair_act')),
  mask            TEXT    NOT NULL,
  created_at_utc  INTEGER NOT NULL,
  updated_at_utc  INTEGER NOT NULL,
  version         INTEGER NOT NULL DEFAULT 1,
  UNIQUE(type, mask)
);

INSERT INTO number_templates_new (id, type, mask, created_at_utc, updated_at_utc, version)
SELECT id, type, mask, created_at_utc, updated_at_utc, version FROM number_templates;

DROP TABLE number_templates;
ALTER TABLE number_templates_new RENAME TO number_templates;

CREATE TABLE number_template_contexts (
  context         TEXT    PRIMARY KEY CHECK (context IN ('device_create', 'printer_create', 'act_create', 'cartridge_create', 'drum_create', 'repair_act_create')),
  template_id     INTEGER NULL REFERENCES number_templates(id) ON DELETE SET NULL,
  updated_at_utc  INTEGER NOT NULL
);

INSERT INTO number_template_contexts (context, template_id, updated_at_utc)
SELECT context, template_id, updated_at_utc FROM _ntc_saved;

DROP TABLE _ntc_saved;

INSERT INTO number_templates (type, mask, created_at_utc, updated_at_utc, version)
VALUES ('repair_act', 'РЕМ-[XXXXXX]', strftime('%s', 'now'), strftime('%s', 'now'), 1);

INSERT INTO number_template_contexts (context, template_id, updated_at_utc)
VALUES (
  'repair_act_create',
  (SELECT id FROM number_templates WHERE type = 'repair_act' AND mask = 'РЕМ-[XXXXXX]'),
  strftime('%s', 'now')
);

-- ---------------------------------------------------------------------------
-- 2. requests
-- ---------------------------------------------------------------------------
CREATE TABLE requests_new (
  id                        INTEGER PRIMARY KEY AUTOINCREMENT,
  request_type              TEXT    NOT NULL CHECK (request_type IN ('cartridge_replace', 'free_form', 'ad_register', 'repair')),
  status                    TEXT    NOT NULL CHECK (status IN ('open', 'in_progress', 'in_repair', 'completed', 'rejected', 'cancelled')) DEFAULT 'open',
  requested_by_user_id      INTEGER NOT NULL REFERENCES users(id),
  assigned_to_user_id       INTEGER NULL REFERENCES users(id),
  printer_device_id         INTEGER NULL REFERENCES devices(id),
  cartridge_model_id        INTEGER NULL REFERENCES cartridge_models(id),
  description               TEXT    NULL,
  resolution_notes          TEXT    NULL,
  created_at_utc            INTEGER NOT NULL,
  updated_at_utc            INTEGER NOT NULL,
  deleted_at_utc            INTEGER NULL,
  version                   INTEGER NOT NULL DEFAULT 1,
  category_id               INTEGER NULL REFERENCES request_categories(id),
  completed_cartridge_id    INTEGER NULL REFERENCES cartridges(id),
  ad_subtype                TEXT    NULL,
  repair_origin_place_id    INTEGER NULL REFERENCES places(id),
  repair_moved              INTEGER NOT NULL DEFAULT 0 CHECK (repair_moved IN (0, 1))
);

INSERT INTO requests_new (
  id, request_type, status, requested_by_user_id, assigned_to_user_id,
  printer_device_id, cartridge_model_id, description, resolution_notes,
  created_at_utc, updated_at_utc, deleted_at_utc, version, category_id,
  completed_cartridge_id, ad_subtype
)
SELECT
  id, request_type, status, requested_by_user_id, assigned_to_user_id,
  printer_device_id, cartridge_model_id, description, resolution_notes,
  created_at_utc, updated_at_utc, deleted_at_utc, version, category_id,
  completed_cartridge_id, ad_subtype
FROM requests;

DROP TABLE requests;
ALTER TABLE requests_new RENAME TO requests;

-- Индексы V012 были потеряны при перестройке V031; восстанавливаем.
CREATE INDEX IF NOT EXISTS idx_requests_status   ON requests(status);
CREATE INDEX IF NOT EXISTS idx_requests_assigned ON requests(assigned_to_user_id);

-- D-19: одна открытая ремонтная заявка на устройство.
CREATE UNIQUE INDEX uq_requests_open_repair_device
  ON requests(printer_device_id)
  WHERE request_type = 'repair'
    AND printer_device_id IS NOT NULL
    AND deleted_at_utc IS NULL
    AND status IN ('open', 'in_progress', 'in_repair');

-- ---------------------------------------------------------------------------
-- 3. Категория «Ремонт техники» скрыта флагом
-- ---------------------------------------------------------------------------
ALTER TABLE request_categories ADD COLUMN is_active INTEGER NOT NULL DEFAULT 1;
UPDATE request_categories SET is_active = 0 WHERE name = 'Ремонт техники';

-- ---------------------------------------------------------------------------
-- 4. Заходы в сервис, участники, сканы
-- ---------------------------------------------------------------------------
CREATE TABLE repair_visits (
  id                    INTEGER PRIMARY KEY AUTOINCREMENT,
  request_id            INTEGER NOT NULL REFERENCES requests(id),
  device_id             INTEGER NOT NULL REFERENCES devices(id),
  seq                   INTEGER NOT NULL,
  service_place_id      INTEGER NOT NULL REFERENCES places(id),
  from_place_id         INTEGER NULL REFERENCES places(id),
  sent_at_utc           INTEGER NOT NULL,
  returned_at_utc       INTEGER NULL,
  giver_name            TEXT    NOT NULL,
  receiver_name         TEXT    NOT NULL,
  fault_description     TEXT    NOT NULL,
  returned_to_place_id  INTEGER NULL REFERENCES places(id),
  received_by_name      TEXT    NULL,
  works_done            TEXT    NULL,
  cost_kopecks          INTEGER NULL CHECK (cost_kopecks IS NULL OR cost_kopecks >= 0),
  act_number            TEXT    NOT NULL,
  created_by_user_id    INTEGER NULL,
  created_at_utc        INTEGER NOT NULL,
  updated_at_utc        INTEGER NOT NULL,
  version               INTEGER NOT NULL DEFAULT 1,
  UNIQUE (request_id, seq)
);

CREATE UNIQUE INDEX uq_repair_visits_one_open
  ON repair_visits(request_id) WHERE returned_at_utc IS NULL;
CREATE INDEX idx_repair_visits_act_number ON repair_visits(lower(trim(act_number)));
CREATE INDEX idx_repair_visits_device ON repair_visits(device_id);

CREATE TABLE request_participants (
  request_id     INTEGER NOT NULL REFERENCES requests(id),
  user_id        INTEGER NOT NULL REFERENCES users(id),
  joined_at_utc  INTEGER NOT NULL,
  PRIMARY KEY (request_id, user_id)
);
CREATE INDEX idx_request_participants_user ON request_participants(user_id);

CREATE TABLE repair_visit_scans (
  visit_id       INTEGER NOT NULL REFERENCES repair_visits(id),
  attachment_id  INTEGER NOT NULL,
  linked_at_utc  INTEGER NOT NULL,
  PRIMARY KEY (visit_id, attachment_id)
);

PRAGMA user_version = 51;
