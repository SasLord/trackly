---
quick_id: 261123-uwq
status: complete
---

# Quick 261123-uwq: R8-01 и R8-04 — перечитывание ленты «История»

Коммит кода: `8547829e` (fix). Только UI, cargo не запускался.

## Сделано
- `PlaceEntityViewModal.svelte`: отдельный `$effect` читает только `placeContentEventsStore.reloadSeq`; сравнение, запись `lastReloadSeq` (не `$state`), сброс `timelineLoadedFor` и вызов `loadTimeline(row)` — внутри `untrack`. Перечитывает только при `pressed`; лента не очищается (без мигания). Объявлен после эффектов сброса/загрузки.
- `PrinterDetail.svelte`, `CartridgeDetail.svelte`: эффект ленты читает `void reloadSeq` (подписка), флаг `cancelled` + cleanup, проверки во всех трёх ветках then/catch/finally; в ветке `null` дополнительно `movementsLoading = false` (иначе спиннер залипал бы после отмены). Подписка поставлена полностью, остатка нет.
- Гейт `ui/scripts/check-history-timeline-refresh.mjs` (инвентарь от обхода `ui/src`, правила а/б, allowlist PROP_TOKEN для GroupPanel с проверкой родителя, EXEMPT, `--selftest`), подключён в `pnpm lint` перед `check-lint-selftest-wiring`. Найдено файлов с лентой: 4.
- Аудит (рабочее дерево, не закоммичено): R8-01 и R8-04 помечены CLOSED (frontmatter + таблица).

## Проверки
- Гейт на исходниках HEAD до фикса (`git archive` в scratchpad, `--src`): КРАСНЫЙ, 5 нарушений (Cartridge, Modal, Printer).
- Мутация 1 (`const seq = placeContentEventsStore.reloadSeq` -> `0` в модалке): первая версия гейта осталась ЗЕЛЁНОЙ (матчила чтение reloadSeq в `let lastReloadSeq = ...` вне эффекта). Гейт усилен: чтение должно быть внутри тела `$effect(...)`; добавлен selftest-случай «reloadSeq вне $effect». После усиления мутация КРАСНАЯ, возврат — ЗЕЛЁНЫЙ.
- Мутация 2 (снят `cancelled`-защита в PrinterDetail, якорь `let cancelled = false;` — 1 вхождение): КРАСНЫЙ; возврат из резервной копии — ЗЕЛЁНЫЙ.
- `node scripts/check-history-timeline-refresh.mjs --selftest`: PASS, 7 случаев; без флага: PASS (4 файла).
- `pnpm --dir ui exec svelte-check`: 0 ERRORS, 75 WARNINGS (в тронутых файлах только прежнее предупреждение `state_referenced_locally` на `row` в PlaceEntityViewModal:104, не от этой правки).
- `pnpm --dir ui lint`: exit 0 (в т.ч. check-lint-selftest-wiring: 71 гейтов подключены).
- `pnpm --dir ui build`: зелёный.
- check-privacy pre-commit: PASS, 0 нарушений.

## Живая проверка — пройдена пользователем 2026-11-23
Живая проверка двумя клиентами (десктоп + LAN-браузер): открыть попап устройства, во втором клиенте переместить устройство или сменить статус — лента должна обновиться без переоткрытия; то же для принтера и расходника; в консоли не должно быть `effect_update_depth_exceeded`. Compile-гейты (svelte-check/lint/build) рантайм-ошибок рун не ловят.

## Отклонения
- [Rule 1] Первая версия правила (а) гейта была вакуумной для мутации — усилена (см. выше).
- [Rule 1] В ветке `null` у Printer/Cartridge добавлен сброс `movementsLoading`.
