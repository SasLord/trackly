# Deferred — quick 261121-hed

## app_groups: 2 красных теста от колонки «Пакет» (261121-hef, bbc463bd)

Найдено при контрольном прогоне `cargo test -p trackly-app --test app_groups` после
задач 261121-hed. К изменениям этого пакета не относится: оба теста сравнивают CSV
отчёта «Перемещения» с набором колонок ДО 261121-hef.

- `group_report_batch::report_batch_csv_full_composition` (group_report_batch.rs:568) —
  ожидает заголовок `Дата;Предмет;Тип;Откуда;Куда;Кем;Причина`, фактически добавлена
  колонка `;Пакет`.
- `group_report_batch::report_batch_act_package_folds_like_group_transfer`
  (group_report_batch.rs:721) — `l.ends_with(&reason)`: строка теперь кончается ячейкой
  «Пакет», а не причиной (0 вместо 7).

Починка — обновить ожидания теста под колонку «Пакет» (зона 261121-hef).

**Закрыто:** quick 261121-heh, коммит `21c0df3a` — ожидания обновлены под колонку «Пакет».
