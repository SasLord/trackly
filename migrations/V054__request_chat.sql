-- V054: переписка по заявкам (Phase 41.5, MSG-02 / MSG-04).
--
-- Только CREATE TABLE / CREATE INDEX / ALTER TABLE ADD COLUMN; ни одного DROP и
-- ни одной перестройки существующих таблиц (урок V042: PRAGMA foreign_keys = OFF
-- внутри refinery-транзакции не действует).
--
--   - chat_messages: собственная таблица, audit_log не трогаем (D-11); слияние
--     сообщений и событий делается на чтении. Нет колонок правки/удаления: сообщения
--     неизменяемы (D-22). moved_from_request_id: пометка «из заявки №N» (D-09).
--     Без внешних ключей (как attachments): запись переживает удаление пользователя.
--   - chat_reads: отметка прочтения на сервере по паре (user_id, request_id) (D-19).
--   - attachments.message_id: файл сообщения; родитель остаётся заявкой (D-16).
--   - app_settings.chat_unread_baseline_audit_id: водяной знак; события аудита,
--     случившиеся до фазы, не считаются непрочитанными (D-17).

CREATE TABLE chat_messages (
  id                     INTEGER PRIMARY KEY AUTOINCREMENT,
  request_id             INTEGER NOT NULL,
  author_user_id         INTEGER NULL,
  body                   TEXT    NOT NULL DEFAULT '',
  created_at_utc         INTEGER NOT NULL,
  moved_from_request_id  INTEGER NULL
);

CREATE INDEX idx_chat_messages_request ON chat_messages (request_id, id);

CREATE TABLE chat_reads (
  user_id          INTEGER NOT NULL,
  request_id       INTEGER NOT NULL,
  last_message_id  INTEGER NOT NULL DEFAULT 0,
  last_audit_id    INTEGER NOT NULL DEFAULT 0,
  updated_at_utc   INTEGER NOT NULL,
  PRIMARY KEY (user_id, request_id)
);

ALTER TABLE attachments ADD COLUMN message_id INTEGER NULL;

CREATE INDEX idx_attachments_message ON attachments (message_id);

INSERT INTO app_settings (key, value, created_at_utc, updated_at_utc)
SELECT 'chat_unread_baseline_audit_id', CAST(COALESCE(MAX(id), 0) AS TEXT), unixepoch(), unixepoch()
FROM audit_log;

PRAGMA user_version = 54;
