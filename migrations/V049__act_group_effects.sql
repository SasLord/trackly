-- V049: следы действий акта над группами (Phase 41.2, GAM-02 / GAM-03, D-08 / D-09).
--
-- Только CREATE TABLE и CREATE INDEX; ни одного DROP и ни одной перестройки
-- существующих таблиц. Урок V042: перестройка таблицы с детьми внутри
-- refinery-транзакции стёрла act_items (PRAGMA foreign_keys = OFF там не
-- действует).
--
-- Зачем отдельная таблица, а не audit_log: запись custom:group_member_released
-- пишется и на restore-путях (нельзя отличить исходное членство от снятия
-- блокировки), а группового «до» (место корня, прежний родитель) в audit_log
-- нет вовсе. Откат акта обязан вернуть членство и место группы.
--
--   - act_id БЕЗ внешнего ключа: перестройка acts не должна каскадом
--     стереть следы, а удаление акта не должно становиться заложником
--     сторонних действий с группами (D-09).
--   - group_id БЕЗ внешнего ключа: группы удаляются жёстко, эффект обязан
--     пережить группу; group_label — снимок имени для тоста предупреждения.
--   - kind: 'release' (вынос члена из группы), 'move' (перенос корня группы),
--     'detach' (отцепление группы от родителя). БЕЗ CHECK: валидация только
--     на стороне Rust (ActGroupEffectKind::from_str_lenient), как source /
--     entity_type в V040 — неизвестный токен деградирует при чтении, а не
--     падает при записи.
--   - device_id — для 'release' вынесенный член; from_place_id / to_place_id —
--     для 'move' место корня до и после переноса (to_place_id нужен для
--     сверки «группа не двигалась с тех пор»); from_parent_group_id — для
--     'detach'.
--   - Бэкфилл не делается (как V040 D-02): акты, проведённые до V049,
--     откатываются как раньше.

CREATE TABLE act_group_effects (
    id                   INTEGER PRIMARY KEY AUTOINCREMENT,
    act_id               INTEGER NOT NULL,
    kind                 TEXT    NOT NULL,
    group_id             INTEGER NOT NULL,
    group_label          TEXT    NOT NULL,
    device_id            INTEGER NULL,
    from_place_id        INTEGER NULL,
    to_place_id          INTEGER NULL,
    from_parent_group_id INTEGER NULL,
    created_at_utc       INTEGER NOT NULL
);

CREATE INDEX idx_act_group_effects_act ON act_group_effects(act_id);

PRAGMA user_version = 49;
