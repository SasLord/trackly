-- V058: ядро мессенджера im_* (Phase 43, DLV-02 / DLV-03, D-01).
--
-- Только CREATE TABLE / CREATE INDEX; ни одного DROP и ни одной перестройки
-- существующих таблиц (урок V042: PRAGMA foreign_keys = OFF внутри
-- refinery-транзакции не действует). Без внешних ключей на users: записи
-- переживают удаление учётной записи (как V050 / V054).
--
--   - im_conversations.last_seq: счётчик беседы; новый seq/rev выдаётся одним
--     UPDATE ... RETURNING на соединении единственного писателя (D-04).
--   - im_members.left_at_utc и left_seq ставятся ВМЕСТЕ: left_seq = last_seq беседы
--     на момент выхода — граница видимости исключённого (D-06).
--   - im_messages: колонки правки/удаления/purge заложены для Фазы 47 без логики.
--     Реакции, FTS, im_read_advances, im_mentions создаёт Фаза 47.

CREATE TABLE im_conversations (
  id                  INTEGER PRIMARY KEY AUTOINCREMENT,
  kind                TEXT    NOT NULL CHECK (kind IN ('dialog', 'room')),
  title               TEXT    NULL,
  dialog_key          TEXT    NULL UNIQUE,
  created_by_user_id  INTEGER NOT NULL,
  created_at_utc      INTEGER NOT NULL,
  last_seq            INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE im_members (
  conversation_id   INTEGER NOT NULL,
  user_id           INTEGER NOT NULL,
  role              TEXT    NOT NULL CHECK (role IN ('creator', 'member')),
  joined_at_utc     INTEGER NOT NULL,
  left_at_utc       INTEGER NULL,
  left_seq          INTEGER NULL,
  visible_from_seq  INTEGER NOT NULL DEFAULT 0,
  last_read_seq     INTEGER NOT NULL DEFAULT 0,
  muted             INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (conversation_id, user_id)
);

CREATE INDEX idx_im_members_user ON im_members (user_id, left_at_utc);

CREATE TABLE im_messages (
  id                 INTEGER PRIMARY KEY AUTOINCREMENT,
  conversation_id    INTEGER NOT NULL,
  seq                INTEGER NOT NULL,
  rev                INTEGER NOT NULL,
  kind               TEXT    NOT NULL DEFAULT 'user',
  author_user_id     INTEGER NULL,
  client_msg_id      TEXT    NULL,
  body_json          TEXT    NOT NULL,
  body_text          TEXT    NOT NULL,
  reply_to_id        INTEGER NULL,
  created_at_utc     INTEGER NOT NULL,
  edited_at_utc      INTEGER NULL,
  deleted_at_utc     INTEGER NULL,
  deleted_by_user_id INTEGER NULL,
  purged_at_utc      INTEGER NULL,
  UNIQUE (conversation_id, seq),
  UNIQUE (conversation_id, author_user_id, client_msg_id)
);

CREATE INDEX idx_im_messages_rev ON im_messages (conversation_id, rev);

PRAGMA user_version = 58;
