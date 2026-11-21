-- V056: place_movements.request_id — источник «по ремонтной заявке» (quick 261121-hed, NEW-3).
--
-- Аддитивная миграция: только ADD COLUMN + индекс + один UPDATE, без
-- перестройки таблиц (перестройка с DROP внутри транзакции refinery
-- каскадно задела бы детей — FK OFF там не действует).
--
-- До V056 ремонтный перенос писался как source='manual' с note
-- 'по заявке №<id>' (устройство) и 'вместе с принтером' (каскад картриджей).
-- Теперь сервер пишет source='repair' и request_id = id заявки.
--
-- Перенос прошлого:
--   - строки УСТРОЙСТВА переводятся в source='repair' + request_id, note
--     обнуляется, только если note РОВНО 'по заявке №' || r.id и заявка r
--     существует и имеет request_type='repair'. Точное равенство, не LIKE:
--     ручная пометка пользователя с другим текстом не задевается.
--   - картриджные строки «вместе с принтером» НЕ переносятся: по ним заявку
--     однозначно не восстановить (тот же текст пишет и ручной перенос
--     принтера), они остаются ручными.
--
-- Токен 'repair' схемы не требует: source — TEXT без CHECK (V040).

ALTER TABLE place_movements ADD COLUMN request_id INTEGER NULL REFERENCES requests(id) ON DELETE SET NULL;

CREATE INDEX idx_place_movements_request
  ON place_movements(request_id)
  WHERE request_id IS NOT NULL;

UPDATE place_movements
   SET source = 'repair',
       request_id = (
         SELECT r.id FROM requests r
          WHERE r.request_type = 'repair'
            AND place_movements.note = 'по заявке №' || r.id
       ),
       note = NULL
 WHERE entity_type = 'device'
   AND source = 'manual'
   AND EXISTS (
         SELECT 1 FROM requests r
          WHERE r.request_type = 'repair'
            AND place_movements.note = 'по заявке №' || r.id
       );

PRAGMA user_version = 56;
