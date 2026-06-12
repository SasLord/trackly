-- V017: Разделение фотобарабанов (drums) и картриджей (cartridges) на уровне
-- экземпляров.
--
-- 1. cartridge_states получает kind_id — какому виду расходника применимо
--    состояние. Картриджи (kind 1) используют «состояние заряда»
--    (Полный/Частичный/Пустой); фотобарабаны (kind 2) — «состояние»
--    (Новый/Изношенный/Отработанный).
-- 2. Отдельный счётчик drum_seq для авто-кодов D-NNNNNN (картриджи — C-NNNNNN
--    из cartridge_seq). Префикс выбирается по виду модели экземпляра.

ALTER TABLE cartridge_states ADD COLUMN kind_id INTEGER NOT NULL DEFAULT 1;

-- Существующие состояния (1 Полный, 2 Частичный, 3 Пустой) — для картриджей
-- (kind 1, проставлено DEFAULT). Состояния фотобарабанов (kind 2):
INSERT INTO cartridge_states (id, name, kind_id) VALUES
  (4, 'Новый', 2),
  (5, 'Изношенный', 2),
  (6, 'Отработанный', 2);

-- Выделенный счётчик авто-кодов фотобарабанов (D-NNNNNN), независимый от
-- cartridge_seq.
INSERT INTO counters (name, current_value) VALUES ('drum_seq', 0);

PRAGMA user_version = 17;
