-- V047: справочник статусов устройств по жизненному циклу (Phase 41.1, GRD-04).
--
-- Только ALTER ... ADD COLUMN / UPDATE / INSERT; перестройка таблиц запрещена
-- (урок V042: перестройка в refinery-транзакции стирает данные, act_items).
--
--   - code — машинный ключ (V014), name — только подпись. Переименование
--     идёт по code, а не по name, и сам code не меняется (D-01, D-02):
--     существующие devices.status_id и все потребители кода остаются целыми.
--   - Новые статусы получают id как MAX+1; на конкретные числа 5 и 6 в коде
--     опираться нельзя — порядок жизненного цикла задаёт sort_order (D-08).

ALTER TABLE device_statuses ADD COLUMN sort_order INTEGER NOT NULL DEFAULT 0;

UPDATE device_statuses SET name = 'Ожидает ремонта' WHERE code = 'на_ремонте';

INSERT INTO device_statuses (name, code) VALUES
  ('В ремонте', 'в_ремонте'),
  ('На списание', 'на_списание');

UPDATE device_statuses SET sort_order = CASE code
  WHEN 'на_складе'   THEN 10
  WHEN 'в_работе'    THEN 20
  WHEN 'на_ремонте'  THEN 30
  WHEN 'в_ремонте'   THEN 40
  WHEN 'на_списание' THEN 50
  WHEN 'списано'     THEN 60
  ELSE sort_order
END;

PRAGMA user_version = 47;
