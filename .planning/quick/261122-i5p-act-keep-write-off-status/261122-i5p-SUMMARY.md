---
phase: quick-261122-i5p
plan: 01
status: complete
requirements: [R7-01]
---

# Quick 261122-i5p: акт сохраняет статус «На списание»

Акты передают, возвращают и откатывают устройства со статусом «На списание», не меняя статус (R7-01).

## Что сделано
- act_service.rs: `is_act_assignable` допускает «на_списание» (через `DEVICE_STATUS_CODE_TO_WRITE_OFF`), «списано» и открытая ремонтная заявка по-прежнему отклоняют. Хелперы `handover_status_id`, `return_status_id`, `is_returnable_status`; применены в create, update(added), do_return, update_return (added, retained).
- Откат (restore_from_snapshot_in_tx) и group_act_plan менять не пришлось: тест отката зелёный.
- UI: ActFormItemsTable.svelte, новый `fetchToWriteOff`, одиночные устройства «На списание» теперь доступны в пикере (раньше не находились: источник «В работе» фильтрует по статусу). Гейт check-act-group-picker.mjs: ожидаемое число devices.list 2 -> 3.
- Тесты: acts_to_write_off_status.rs (7, подключён в suites/acts.rs).

## Коммиты
- test: тесты + правка act_picker_printers.rs
- fix: act_service.rs
- fix: UI пикер + гейт

## Отклонения
- [Rule 1] act_picker_printers::statuses_outside_groups_other_than_storage_and_working_rejected требовал отказ для «на_списание» (старое правило); убрал «на_списание» из списка отказов.
- TDD: реализация и тесты написаны до первого прогона, RED-фаза отдельно не фиксировалась; тесты охраняют поведение (мутационно не проверялись).

## Гейты
- cargo test -p trackly-app --test app_acts: 311/311 после правки (одна правка теста выше), clippy -D warnings, fmt --check чисто.
- pnpm --dir ui lint, svelte-check (0 ошибок), build зелёные.
- Живая проверка UI в приложении не выполнялась (UNVERIFIED).

## Self-Check: PASSED
