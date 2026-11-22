-- V057: history_events — единая лента событий истории устройства и группы (фаза 41.10, D-01..D-03, D-25).
--
-- Аддитивная миграция: только CREATE TABLE / CREATE INDEX. Без FOREIGN KEY, без CHECK,
-- без DROP/ALTER: перестройка с DROP внутри транзакции refinery каскадно задела бы
-- детей (FK OFF там не действует), а токены kind/source/status — TEXT без CHECK, чтобы
-- значение, записанное более новым бинарником, не ронявшее чтение (D-23).
--
-- place_movements не расширяется (D-02); audit_log источником ленты не служит и не
-- разбирается (D-01); бэкфилла нет — таблица создаётся пустой (D-03).
-- movement_id — id строки place_movements, записанной той же операцией (D-25), без FK.
-- subject_label / *_label — снимки имён на момент события.

CREATE TABLE history_events (
  id                 INTEGER PRIMARY KEY AUTOINCREMENT,
  operation_id       TEXT    NOT NULL,
  subject_type       TEXT    NOT NULL,
  subject_id         INTEGER NOT NULL,
  subject_label      TEXT    NULL,
  kind               TEXT    NOT NULL,
  from_group_id      INTEGER NULL,
  from_group_label   TEXT    NULL,
  to_group_id        INTEGER NULL,
  to_group_label     TEXT    NULL,
  status_from_code   TEXT    NULL,
  status_from_label  TEXT    NULL,
  status_to_code     TEXT    NULL,
  status_to_label    TEXT    NULL,
  source             TEXT    NOT NULL,
  act_id             INTEGER NULL,
  request_id         INTEGER NULL,
  ref_group_id       INTEGER NULL,
  ref_group_label    TEXT    NULL,
  movement_id        INTEGER NULL,
  user_id            INTEGER NULL,
  actor_name_snapshot TEXT   NULL,
  created_at_utc     INTEGER NOT NULL
);

CREATE INDEX idx_history_events_subject
  ON history_events(subject_type, subject_id, created_at_utc DESC);

CREATE INDEX idx_history_events_to_group
  ON history_events(to_group_id)
  WHERE to_group_id IS NOT NULL;

CREATE INDEX idx_history_events_from_group
  ON history_events(from_group_id)
  WHERE from_group_id IS NOT NULL;

CREATE INDEX idx_history_events_act
  ON history_events(act_id)
  WHERE act_id IS NOT NULL;

CREATE INDEX idx_history_events_movement
  ON history_events(movement_id)
  WHERE movement_id IS NOT NULL;

CREATE INDEX idx_history_events_operation
  ON history_events(operation_id);

PRAGMA user_version = 57;
