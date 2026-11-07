-- V050: вложения (Phase 41.3, ATT-01, D-01 / D-18).
--
-- Только CREATE TABLE и CREATE INDEX; ни одного удаления и ни одной перестройки
-- существующих таблиц. Урок V042: перестройка таблицы-родителя внутри
-- refinery-транзакции каскадно стирает детей (PRAGMA foreign_keys = OFF там не
-- действует).
--
--   - parent_kind + parent_id: полиморфный родитель в ОДНОЙ таблице, БЕЗ внешнего
--     ключа и БЕЗ CHECK. Допустимые значения проверяет Rust (ParentKind), поэтому
--     родитель-сообщение из 41.5 не потребует перестройки таблицы.
--   - uploaded_by_user_id / deleted_by_user_id: пользователь БЕЗ внешнего ключа,
--     запись обязана пережить удаление учётной записи.
--   - content_hash: sha256 в hex (имя файла на диске выводится только из него);
--     один файл может принадлежать нескольким строкам, поэтому refcount считает
--     все строки, включая мягко удалённые.
--   - deleted_at_utc: мягкое удаление с возможностью восстановления.

CREATE TABLE attachments (
  id                   INTEGER PRIMARY KEY AUTOINCREMENT,
  parent_kind          TEXT    NOT NULL,
  parent_id            INTEGER NOT NULL,
  original_name        TEXT    NOT NULL,
  mime                 TEXT    NOT NULL,
  size_bytes           INTEGER NOT NULL,
  content_hash         TEXT    NOT NULL,
  uploaded_by_user_id  INTEGER NULL,
  created_at_utc       INTEGER NOT NULL,
  deleted_at_utc       INTEGER NULL,
  deleted_by_user_id   INTEGER NULL
);

CREATE INDEX idx_attachments_parent ON attachments (parent_kind, parent_id, created_at_utc, id);
CREATE INDEX idx_attachments_hash ON attachments (content_hash);
