---
phase: quick-261121-heg
plan: 01
status: complete
subsystem: printers, ui, gates
tags: [printers, snmp-history, ui-honesty, gates, ci]
requires: []
provides:
  - "printers_get_readings (Tauri + POST /api/v1/printers_get_readings)"
  - "PrinterDto.usb_host_deleted / usbHostDeleted"
  - "HTML_COMMANDS в ui/src/lib/api/client.ts"
  - "ui/scripts/check-html-commands.mjs"
  - "ui/scripts/check-lint-selftest-wiring.mjs"
affects: [PrinterDetail, DeviceFormBody, StorageSettings, apiCall, pnpm lint, ci-fast]
tech-stack:
  added: []
  patterns:
    - "мета-гейт от каталога: каждый scripts/check-*.mjs и его --selftest обязаны быть в pnpm lint"
    - "белый список text/html-команд клиента сверяется с серверными text/html-обработчиками"
key-files:
  created:
    - ui/scripts/check-html-commands.mjs
    - ui/scripts/check-lint-selftest-wiring.mjs
  modified:
    - crates/trackly-core/src/ports/printers.rs
    - crates/trackly-core/src/domain/printers.rs
    - crates/trackly-infra/src/repos/printers_sqlite.rs
    - crates/trackly-app/src/services/printer_service.rs
    - crates/trackly-app/src/dto/printer.rs
    - crates/trackly-app/src/tauri_cmds/printers.rs
    - crates/trackly-app/src/http/printers.rs
    - crates/trackly-app/src/specta_export.rs
    - crates/trackly-app/src/dto/place.rs
    - crates/trackly-app/src/services/group_place.rs
    - crates/trackly-app/src/services/attachment_service.rs
    - crates/trackly-app/tests/role_endpoint_matrix.rs
    - crates/trackly-app/tests/devices_printer_network.rs
    - crates/trackly-app/tests/http_route_parity.rs
    - ui/src/lib/api/client.ts
    - ui/src/bindings-phase6.ts
    - ui/src/features/printers/PrinterDetail.svelte
    - ui/src/features/settings/StorageSettings.svelte
    - ui/src/features/devices/usbHostPicker.ts
    - ui/src/features/devices/DeviceFormBody.svelte
    - ui/scripts/check-device-type-labels.mjs
    - ui/scripts/check-contrast.mjs
    - ui/scripts/check-usb-host-picker.mjs
    - ui/scripts/fixtures/usb-host-picker/cases.json
    - ui/scripts/check-group-vocabulary.mjs
    - ui/package.json
    - .github/workflows/ci-fast.yml
    - .planning/STATE.md
decisions:
  - "NEW-6: сотрудник не видит вложений устройства в v1.4 (employee_can_read_device = false осознанно)"
  - "N2 реализован, а не удалён: история опросов — лента последних 50 строк (READINGS_HISTORY_LIMIT)"
  - "IPv6 получает собственный отказ «IPv6 не поддерживается», а не сообщение про порт"
metrics:
  completed: 2026-11-21
  tasks: 3
  commits: 7
---

# Quick 261121-heg: UI, гейты, мелочи (пакет C аудита v1.4) — итог

История опросов принтера заработала на обоих транспортах (N2). Карточка принтера и форма
устройства теперь честно показывают IPv6, удалённый USB-хост и лимит поддерева. `apiCall`
больше не выдаёт HTML за данные (INT-35). Три гейта стали держать то, что заявляют
(INT-19, INT-20, плюс USB-пикер), и появились два новых: паритет HTML-команд и мета-гейт
подключения гейтов в `pnpm lint` (WR-13).

## Что сделано по находкам

| Находка | Что сделано | Где | Коммит |
|---|---|---|---|
| N2 | `list_readings` в порте и infra (`ts_utc DESC, id DESC`, LIMIT, общий `map_row_reading`); `PrinterService::list_readings` + `READINGS_HISTORY_LIMIT = 50`, NotFound на чужой id; `PrinterReadingDto` (camelCase, форма bindings-phase6.ts); `build_printers_get_readings` (ReadData), Tauri-команда, `POST /api/v1/printers_get_readings`; роль-матрица Case 89 (Employee → 403), без EXEMPT | printers_sqlite.rs, printer_service.rs, dto/printer.rs, tauri_cmds/printers.rs, http/printers.rs, specta_export.rs, role_endpoint_matrix.rs | 74d44203 |
| INT-36а | убран протухший `#[allow(dead_code)]` у `GroupMoveCause::act` | services/group_place.rs | 353eb57f |
| INT-36б | `normalize_printer_ip`: `[…]` или ≥2 двоеточий → «IPv6 не поддерживается…»; одно двоеточие — прежнее «Порт…»; тест разделён на `printer_ip_rejects_port` / `printer_ip_rejects_ipv6_honestly` (RED увиден) | domain/printers.rs | 02a566bb |
| INT-36в | колонка `(h.deleted_at_utc IS NOT NULL) AS usb_host_deleted` → `PrinterRow` → `PrinterDto.usbHostDeleted`; в карточке «(удалён)» приглушённым токеном `--tr-text-tertiary` | printers_sqlite.rs, dto/printer.rs, bindings-phase6.ts, PrinterDetail.svelte | 02a566bb |
| INT-36г | `SUBTREE_CAP_NOTICE`, `onCapReached` (один раз, только при отсечке по потолку); флаг `subtreeCapReached` в форме, подсказка под полем с `aria-live="polite"`; гейт check-usb-host-picker закрепляет вызов (фикстура `capReachedCalls` 1/0, 2 новых мутанта, якоря потолка обновлены) | usbHostPicker.ts, DeviceFormBody.svelte, check-usb-host-picker.mjs, cases.json | 02a566bb |
| INT-10 | `isTauri` на уровне модуля, кнопка «Открыть папку с базой данных» в `{#if isTauri}`, ранний return в `openFolder`; вердикты `settings_open_db_folder` / `app_restart` / `settings_move_db` в http_route_parity переписаны под новые строки | StorageSettings.svelte, http_route_parity.rs | 02a566bb |
| комментарий .snmp-section | описывает текущее поведение (тип «Принтер», создание и правка, D-13) | DeviceFormBody.svelte | 02a566bb |
| NEW-6 | правдивый doc-комментарий `employee_can_read_device`; решение в STATE.md `### Decisions` | attachment_service.rs, STATE.md | 353eb57f, 9e64e70e |
| IN-01 | `PlaceTreeNodeDto` удалён, шапка dto/place.rs поправлена | dto/place.rs | 353eb57f |
| INT-35 | `HTML_COMMANDS` (6 команд от сервера); text/html вне списка → `AppError { code: 'NOT_FOUND', message: 'Маршрут не найден: <name>' }`; гейт check-html-commands.mjs ([H0] антивакуум, [H1] обработчик без маршрута, [H2] A≠B в обе стороны) | client.ts, check-html-commands.mjs | 512044ba |
| INT-20 | `NAME = name\|label\|caption\|title\|text\|displayName\|display_name`; selftest-мутант `{ id: 2, caption: 'Принтер' }` в реальном GroupAddDevicesModal; на реальном ui/src ложных срабатываний нет | check-device-type-labels.mjs | 7737fe48 |
| INT-19 | `COMPOSITE_MIN = 4.6`, шапка и комментарий приведены; мутант (8) success-text `#0e6d40` → композит 4.55 | check-contrast.mjs | 7737fe48 |
| WR-13 | мета-гейт check-lint-selftest-wiring.mjs ([W0] антивакуум, [W1] простой прогон, [W2] `--selftest`) — инвентарь от каталога, поэтому учитывает и гейты пакетов hed/heh (check-movement-source-labels, check-printer-identity) и оба новых; подключён последней парой; в ci-fast новый шаг с `check-privacy.selftest.mjs` и `check-write-site-gate-untouched.mjs --selftest` (обе локально зелёные) | check-lint-selftest-wiring.mjs, package.json, ci-fast.yml | 02d9ce21 |

## Мутационная проверка гейтов (копии в scratchpad)

| Гейт | Чистая копия | Мутант копии реального исходника | Прежняя версия (HEAD до правки) на мутанте |
|---|---|---|---|
| check-device-type-labels (INT-20), `--src=<копия ui/>`, якорь `...orderedTypes().map((t) => ({ id: t.id, label: t.name })),` (1 вхождение) → `{ id: 2, caption: 'Принтер' },` | exit 0 | exit 1 ([T1] GroupAddDevicesModal.svelte:59) | exit 0 — пропускала |
| check-contrast (INT-19), `--tokens=<копия>`, `--tr-success-text: #0e6c3f;` (1 вхождение) → `#0e6d40` | exit 0 | exit 1 (light R1 composite success active over --tr-surface-sunken: 4.55 < 4.6) | exit 0 — пропускала |
| check-html-commands (INT-35), `--src=<копия http/ + client.ts>`, удалена строка `'repairs_render_act',` | exit 0 | exit 1 ([H2] … нет: repairs_render_act) | гейта не было |
| check-lint-selftest-wiring (WR-13), `--src=<копия ui/>`, из lint убрано `node scripts/check-contrast.mjs --selftest && ` | exit 0 | exit 1 ([W2] check-contrast.mjs) | гейта не было |
| check-usb-host-picker (INT-36г), встроенный selftest | — | 55 мутантов пойманы, включая «onCapReached не вызывается» и «зовётся на каждый узел» | — |

Встроенные selftest: check-html-commands — контроль зелёный, 5 мутантов красные;
check-lint-selftest-wiring — контроль зелёный, 5 мутантов красные; check-device-type-labels —
40 кейсов; check-contrast — 13 мутантов.

## Прогоны

- cargo (env TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1, DEVELOPER_DIR=CommandLineTools, по одному):
  app_gates 253/0 (1 ignored), app_auth 138/0, app_devices 255/0, app_server 49/0,
  app_cartridges 88/0, `--workspace --lib` (app 398, core 186, infra 177), trackly-infra
  `--tests` 177 + 272. RED увиден: app_devices (нет `build_printers_get_readings`, нет поля
  `usb_host_deleted`), trackly-core `printer_ip_rejects_ipv6_honestly`.
- Гейты: `cargo fmt --all -- --check` 0, `cargo clippy --workspace --all-targets -- -D warnings` 0,
  `pnpm --dir ui lint` 0, `pnpm --dir ui svelte-check` 0 ошибок (75 предупреждений — прежние,
  ни одно не в новых строках), `pnpm --dir ui build` 0 (bindings.ts содержит
  `printersGetReadings`, `usbHostDeleted`), check-privacy 0 нарушений.

## Отклонения от плана

1. **[Rule 1 — тест] Тест N2 учитывает живой опросчик.** Живой `AppCtx` поднимает мониторинг
   с SNMP-моком, и тот при полном прогоне suite дописывал своё показание с текущим временем.
   Тест упал (пройдя в одиночку). Засеянные строки перенесены в будущее (`4_000_000_000`),
   проверяется первая тройка и убывание всей ленты. Коммит 02a566bb.
2. **[Rule 3 — гейт] Якорь check-usb-host-picker.** Мутанты потолка искали строку
   `if (visited.size >= SUBTREE_CAP) continue;`, а она теперь стала блоком. Якоря обновлены
   (`… {`), проверка не ослаблена. Добавлены фикстура `capReachedCalls`, два охранных условия
   и два мутанта на сам сигнал.
3. **[Rule 3 — гейт] check-group-vocabulary.** Текст, который требует план («…групп состава…»),
   краснил GRD-06. В allowlist добавлена узкая запись: файл `usbHostPicker.ts`, маркер
   `'групп состава'`.
4. **Вердикты `app_restart` / `settings_move_db`** в http_route_parity ссылались на строки
   StorageSettings, которые сдвинулись. Номера обновлены вместе с вердиктом INT-10.
5. **Команда `pnpm check`** из `<verify>` плана в ui/ не существует. Гонялся
   `pnpm --dir ui svelte-check`.
6. **PrinterRow-литерал в devices_printer_network.rs:152** — это локальная тестовая структура
   с тем же именем, а не доменная `PrinterRow`. Дополнять её не понадобилось.
7. Тест `usb_host_deleted` лежит в devices_printer_network.rs (suite app_devices) и идёт через
   `AppCtx` / `PrinterService::get_by_device_id`. Отдельного infra-теста нет.

## Known Stubs

Нет.

## Threat Flags

Нет новых поверхностей вне threat_model. Маршрут `/api/v1/printers_get_readings` —
T-heg-01/02/04 (ReadData, Case 89, LIMIT 50, пул ридеров).

## Не проверено вживую (UNVERIFIED)

- Лента «История статусов» в карточке принтера в запущенном приложении: десктоп (WKWebView) и
  LAN-браузер. Проверены только сервис/HTTP-тесты и сборка.
- Пометка «(удалён)» у USB-хоста и подсказка о лимите поддерева (нужен состав > 50 групп) —
  только тесты и svelte-check, в UI глазами не смотрелись.
- Отсутствие кнопки «Открыть папку с базой данных» в LAN-браузере и её наличие в десктопе.
- NOT_FOUND от `apiCall` на HTML вне белого списка: JS-тест-раннера нет, поведение держит
  только гейт паритета и чтение кода.
- Шаг ci-fast с корневыми selftest на GitHub-раннере не запускался: не пушили.

## Self-Check: PASSED

- Файлы: ui/scripts/check-html-commands.mjs, ui/scripts/check-lint-selftest-wiring.mjs — на месте.
- Коммиты: 74d44203, 02a566bb, 353eb57f, 9e64e70e, 512044ba, 7737fe48, 02d9ce21 — в git log.
