-- V059: ключи идемпотентности чата заявок и заявок (Phase 43, D-11).
--
-- Только ALTER TABLE ADD COLUMN / CREATE UNIQUE INDEX. `ADD COLUMN ... UNIQUE`
-- в SQLite запрещён, поэтому уникальность — частичными индексами: старые строки
-- (NULL) не конфликтуют, два одинаковых непустых id — конфликт. В ключ чата
-- входит request_id: тот же клиентский id в другой заявке не считается повтором.

ALTER TABLE chat_messages ADD COLUMN client_msg_id TEXT NULL;

CREATE UNIQUE INDEX uq_chat_messages_client
  ON chat_messages (request_id, author_user_id, client_msg_id)
  WHERE client_msg_id IS NOT NULL;

ALTER TABLE requests ADD COLUMN client_request_id TEXT NULL;

CREATE UNIQUE INDEX uq_requests_client
  ON requests (requested_by_user_id, client_request_id)
  WHERE client_request_id IS NOT NULL;

PRAGMA user_version = 59;
