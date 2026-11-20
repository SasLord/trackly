---
phase: quick-261121-4nz
plan: 01
status: complete
requirements: [ATT-01, ATT-03, PGR-01, PGR-02, PGR-03]
commits: [ff63dac6, f801cb38, 05a6d9a5]
---

# Quick 261121-4nz: NEW-1 (вложения при переносе БД) и N1 (printers_create)

## Сделано

- **NEW-1.** `BackupService::copy_attachments_to` (ошибка не глотается, нет источника = `Ok(0)`).
  `build_settings_move_db` читает `[paths].attachments_path` из конфиг-файла; если пусто, копирует
  вложения в `<папка new_path>/attachments` ДО переписывания `db_path`. Явный путь не трогается.
  Диалог в `StorageSettings.svelte` сообщает о копировании вложений.
- **N1.** Удалены: tauri-команда и `build_printers_create`, HTTP-маршрут и handler, строка specta,
  `PrinterCreateDto`, `PrinterService::create_from_device`, `printers.create` в `api.ts`, тип в
  `bindings-phase6.ts`, тест n13. Мёртвое поле `audit_repo` в `PrinterService` убрано (стало
  неиспользуемым, clippy). В `41.13-SECURITY.md` у R-05 добавлена пометка о снятии.

## Решения по сохранению

- `PrinterCreateBlockDto` СОХРАНЁН: живой путь создания устройства-принтера
  (device_service, http/devices, tauri_cmds/devices).
- Покрытие валидатора IP на create не потеряно: n11 (блок принтера) использует тот же
  `normalize_printer_ip`.
- `ui/src/bindings.ts` в `.gitignore` (генерируется prebuild при `pnpm build`), перегенерирован.
- Строки реестровых гейтов упоминаний `printers_create` не содержали; гейты не менялись.

## Отклонения

- [Rule 1] clippy в `attachments_move_db.rs` (needless `Ok(?)`) исправлен в коммите 3.
- [Rule 3] Неиспользуемые импорты/поле `audit_repo` после удаления create_from_device.

## Проверки (запущено фактически)

- `cargo test -p trackly-app --test app_attachments move_db_`: 4 passed. Мутация (отключён вызов
  копирования): 2 красных (copies_when_path_empty, copy_failure_keeps_db_path); откат сделан.
- `cargo test -p trackly-app --test app_attachments` (полностью): 75 passed, 0 failed.
- `cargo test -p trackly-app --test app_devices devices_printer_network`: 14 passed.
- `cargo test -p trackly-app --test app_gates` (entities_changed, write-site, http_route_parity):
  244 passed, 0 failed, 1 ignored.
- `cargo test -p trackly-app --test app_auth role_endpoint_matrix`: 28 passed.
- `cargo clippy -p trackly-app --all-targets -- -D warnings`: чисто. `cargo fmt --all --check`: чисто.
- `pnpm --dir ui exec svelte-check --threshold error`: 0 errors (75 warnings, прежние).
- `pnpm --dir ui lint`: exit 0. `pnpm --dir ui build`: успешно.
- `grep -rn printers_create crates ui/src`: 0.
- Pre-commit privacy gate: PASS на всех трёх коммитах.
- НЕ запускались (UNVERIFIED): полный прогон воркспейса, остальные suite-бинарники
  (acts, cartridges, groups, places, repairs, reports, server); живая проверка диалога в приложении.

## Known Stubs

Нет.
