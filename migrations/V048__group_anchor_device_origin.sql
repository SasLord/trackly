-- V048: якорь группы и происхождение детали (Phase 41.1, GRD-02 / GRD-03 / GRP-10).
--
-- Только ALTER ... ADD COLUMN и CREATE INDEX; без перестройки таблиц
-- (урок V042).
--
--   - groups.anchor_device_id — устройство-якорь группы (D-15, D-13).
--     ON DELETE RESTRICT защищает только от физического удаления строки.
--     Устройства удаляются мягко (UPDATE deleted_at_utc), FK этого не видит,
--     поэтому запрет мягкого удаления якоря живёт в сервисе
--     (Pitfall 1 исследования 41.1).
--   - devices.origin_group_id — происхождение детали: группа разбора, из
--     которой деталь вынута (D-16). Это отдельная связь, не членство
--     (group_devices). ON DELETE SET NULL: запрет удаления группы разбора
--     строит сервис по живым деталям, FK не должен давать невидимую
--     причину отказа из-за мягко удалённых строк.
--   - Уникальность якоря — частичным индексом: UNIQUE прямо в ADD COLUMN
--     SQLite не допускает.

ALTER TABLE groups ADD COLUMN anchor_device_id INTEGER NULL DEFAULT NULL REFERENCES devices(id) ON DELETE RESTRICT;
ALTER TABLE devices ADD COLUMN origin_group_id INTEGER NULL DEFAULT NULL REFERENCES groups(id) ON DELETE SET NULL;

CREATE UNIQUE INDEX idx_groups_anchor ON groups(anchor_device_id) WHERE anchor_device_id IS NOT NULL;
CREATE INDEX idx_devices_origin ON devices(origin_group_id) WHERE origin_group_id IS NOT NULL;

PRAGMA user_version = 48;
