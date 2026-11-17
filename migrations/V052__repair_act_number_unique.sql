-- V052: номер акта передачи в ремонт (`repair_visits.act_number`) уникален
-- на уровне БД (долг W-1 фазы 41.4).
--
-- В V051 индекс `idx_repair_visits_act_number` обычный. Дублей до сих пор нет
-- только потому, что номер выдаётся внутри транзакции единственного писателя
-- (`issue_repair_act_number_in_tx` + `ensure_number_free_in_tx`). Теперь это
-- держит и сама БД.
--
-- NULL: колонка `NOT NULL` (V051), частичный индекс по NULL не нужен. Пустой
-- номер (после TRIM) — не номер: в дедупе он не участвует и индекс его
-- исключает, иначе две пустые строки не дали бы создать индекс.
--
-- Ключ сравнения — тот же, что у приложения (`normalize_number_key`:
-- trim + to_lowercase). SQLite `LOWER()` складывает только ASCII, поэтому, как
-- и в V044, после LOWER идёт цепочка REPLACE для кириллицы (А–Я, Ё). Одно и
-- то же выражение — в ключе дедупа и в индексе, расходиться они не могут.
--
-- Старые БД, где дубль уже есть: иначе CREATE UNIQUE INDEX упал бы и
-- приложение не запустилось бы. Самый ранний визит группы (MIN(id)) номер
-- сохраняет, остальные переименовываются в `ДУБЛЬ-NNNNNN (<исходный>)`
-- (формат V044), одна сквозная нумерация по всем группам. На каждое
-- переименование — строка audit_log (user_id = NULL, система) со старым и
-- новым номером: исходный номер напечатан на бумажном акте и восстановим.

CREATE TEMP TABLE _v052_dedup_renames AS
WITH dedup_key AS (
  SELECT id,
         request_id,
         TRIM(act_number) AS original_number,
         REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(
           LOWER(TRIM(act_number)),
           'А', 'а'), 'Б', 'б'), 'В', 'в'), 'Г', 'г'), 'Д', 'д'), 'Е', 'е'), 'Ж', 'ж'), 'З', 'з'),
           'И', 'и'), 'Й', 'й'), 'К', 'к'), 'Л', 'л'), 'М', 'м'), 'Н', 'н'), 'О', 'о'), 'П', 'п'),
           'Р', 'р'), 'С', 'с'), 'Т', 'т'), 'У', 'у'), 'Ф', 'ф'), 'Х', 'х'), 'Ц', 'ц'), 'Ч', 'ч'),
           'Ш', 'ш'), 'Щ', 'щ'), 'Ъ', 'ъ'), 'Ы', 'ы'), 'Ь', 'ь'), 'Э', 'э'), 'Ю', 'ю'), 'Я', 'я'),
           'Ё', 'ё') AS key_val
    FROM repair_visits
   WHERE TRIM(act_number) <> ''
),
group_stats AS (
  SELECT key_val, MIN(id) AS first_id
    FROM dedup_key
   GROUP BY key_val
  HAVING COUNT(*) > 1
)
SELECT dk.id                                          AS id,
       dk.request_id                                  AS request_id,
       dk.original_number                             AS original_number,
       ROW_NUMBER() OVER (ORDER BY gs.first_id, dk.id) AS dup_seq
  FROM dedup_key dk
  JOIN group_stats gs ON gs.key_val = dk.key_val
 WHERE dk.id <> gs.first_id;

UPDATE repair_visits
   SET act_number =
         'ДУБЛЬ-' || printf('%06d', (
           SELECT dup_seq FROM _v052_dedup_renames r WHERE r.id = repair_visits.id
         )) || ' (' || (
           SELECT original_number FROM _v052_dedup_renames r WHERE r.id = repair_visits.id
         ) || ')'
 WHERE id IN (SELECT id FROM _v052_dedup_renames);

INSERT INTO audit_log
  (entity_type, entity_id, action, user_id, before_json, after_json, payload_json, created_at_utc)
SELECT
  'repair_visit',
  r.id,
  'custom:repair_act_number_dedup',
  NULL,
  NULL,
  NULL,
  json_object('request_id', r.request_id,
              'old_number', r.original_number,
              'new_number', v.act_number),
  CAST(strftime('%s', 'now') AS INTEGER)
FROM _v052_dedup_renames r
JOIN repair_visits v ON v.id = r.id;

DROP TABLE _v052_dedup_renames;

DROP INDEX IF EXISTS idx_repair_visits_act_number;

-- Имя индекса прежнее: на него никто не ссылается по имени, а поиск по номеру
-- (если появится) пойдёт по тому же выражению.
CREATE UNIQUE INDEX idx_repair_visits_act_number
  ON repair_visits (
    REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(
      LOWER(TRIM(act_number)),
      'А', 'а'), 'Б', 'б'), 'В', 'в'), 'Г', 'г'), 'Д', 'д'), 'Е', 'е'), 'Ж', 'ж'), 'З', 'з'),
      'И', 'и'), 'Й', 'й'), 'К', 'к'), 'Л', 'л'), 'М', 'м'), 'Н', 'н'), 'О', 'о'), 'П', 'п'),
      'Р', 'р'), 'С', 'с'), 'Т', 'т'), 'У', 'у'), 'Ф', 'ф'), 'Х', 'х'), 'Ц', 'ц'), 'Ч', 'ч'),
      'Ш', 'ш'), 'Щ', 'щ'), 'Ъ', 'ъ'), 'Ы', 'ы'), 'Ь', 'ь'), 'Э', 'э'), 'Ю', 'ю'), 'Я', 'я'),
      'Ё', 'ё')
  )
  WHERE TRIM(act_number) <> '';

PRAGMA user_version = 52;
