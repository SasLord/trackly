---
quick_id: 261105-s6s
status: complete
date: 2026-11-05
closes: INT-26
---

# Quick 261105-s6s — INT-26: SNMP-приёмка принтера больше не пишет строку `printers` дважды

## Что сделано

- `DeviceService::create_with_printer_block(new, Option<PrinterCreateBlockDto>)` —
  `create()` стал обёрткой с `None`. Блок сети уходит в `insert_new_and_get` →
  `sync_printer_row_in_tx`: строка `printers` создаётся сразу с IP и community в
  транзакции устройства. Блок у не-принтера → `Validation`. CSV-импорт — `None`,
  поведение прежнее.
- `build_printers_admit`: шаг 4 (`create_from_device`) удалён; результат читается через
  `get_by_device_id`. Сбой валидации IP откатывает и устройство (сирот нет).
- Новый тест `crates/trackly-app/tests/printers_admit.rs` (a1–a3): все три **красные на
  до-фикс коде** (`UNIQUE constraint failed: printers.device_id`; сирота-устройство при
  невалидном IP), зелёные после.

## Отклонение от формулировки задачи

Задача говорила «записать IP/community поверх строки, созданной `sync_printer_row_in_tx`».
Сделано иначе по форме, то же по смыслу: IP/community передаются В `sync_printer_row_in_tx`
как override (механизм формы «Новый принтер», Phase 40.3), так что строка сразу создаётся
с сетью — одна транзакция вместо двух, без окна «принтер без IP».

## Побочные эффекты

- В `audit_log` больше нет отдельной записи `printer/create` для приёмки — остаётся
  `device/create` (как у формы «Новый принтер»).
- INT-13 (приёмка минует шаблоны номеров) снова достижима — не трогалась.
- Живой SNMP-приёмки на настоящем принтере не было (dev-машина без принтеров); покрыто
  тестом на `MockSnmpClient`.
