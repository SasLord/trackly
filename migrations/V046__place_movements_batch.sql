-- V046: корреляция пакета переноса группы в журнале перемещений (Phase 41, GRP-06).
--
-- Только три ALTER ... ADD COLUMN; перестройка place_movements запрещена
-- (урок V042: перестройка в refinery-транзакции теряет данные).
--
--   - batch_id: UUID v4 пакета переноса группы. Перенос АРМ из шести
--     устройств даёт шесть строк журнала с одним batch_id — отчёт
--     может показать их одним событием.
--   - entity_label: снимок имени группы на момент переноса. Журнал не
--     JOIN-ит группы: группа может быть переименована или удалена.
--   - group_id: id группы БЕЗ внешнего ключа (как entity_label — снимок).
--     Пишется в КАЖДУЮ строку групповой записи, в том числе когда строки
--     самой группы в журнале нет: первое размещение NULL -> место и
--     добавление устройств в группу. Таймлайн не зависит от заголовка пакета.

ALTER TABLE place_movements ADD COLUMN batch_id TEXT NULL;
ALTER TABLE place_movements ADD COLUMN entity_label TEXT NULL;
ALTER TABLE place_movements ADD COLUMN group_id INTEGER NULL;

CREATE INDEX idx_place_movements_batch
  ON place_movements(batch_id) WHERE batch_id IS NOT NULL;
CREATE INDEX idx_place_movements_group
  ON place_movements(group_id) WHERE group_id IS NOT NULL;

PRAGMA user_version = 46;
