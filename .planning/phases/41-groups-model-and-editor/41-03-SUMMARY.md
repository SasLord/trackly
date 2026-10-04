---
phase: 41-groups-model-and-editor
plan: 03
subsystem: ui
tags: [vocabulary, gate, svelte, grd-06]
requires: []
provides:
  - "UI-надписи свёртки одинаковых устройств больше не используют слово «группа»"
  - "ui/scripts/check-group-vocabulary.mjs (+ --selftest) в цепочке pnpm lint"
affects: [41-groups-ui, devices, showcase]
tech-stack:
  added: []
  patterns: ["allowlist пар файл+маркер вместо «ровно 0»", "selftest на той же функции сканирования, что и боевой прогон"]
key-files:
  created: [ui/scripts/check-group-vocabulary.mjs]
  modified:
    - ui/package.json
    - ui/src/features/devices/DeviceFilters.svelte
    - ui/src/features/devices/DeviceList.svelte
    - ui/src/lib/components/TableRow.svelte
    - ui/src/features/showcase/sections/TableSection.svelte
    - ui/src/features/showcase/sections/DropdownSection.svelte
    - crates/trackly-app/src/dto/device.rs
key-decisions:
  - "Гейт ищет кириллические корни после вырезания комментариев: латинские идентификаторы корню не соответствуют, отдельный разбор литералов не нужен"
  - "Комментарии вырезаются regex-ами, а не автоматом по кавычкам: апострофы в русских doc-комментариях залипали бы в литерале"
requirements-completed: [GRD-06]
duration: ~25min
completed: 2026-10-04
---

# Phase 41 Plan 03: Словарь «группа» и гейт GRD-06 Summary

Пять пользовательских строк свёртки одинаковых устройств переименованы («Свернуть одинаковые», «Свёрнуто: N», «Свернуть/Развернуть»), а слово «группа» закреплено за одной сущностью постоянным гейтом с selftest в `pnpm lint`.

## Tasks

| Task | Name | Commit |
| ---- | ---- | ------ |
| 1 | Переименование пяти строк и Rust-doc | 8762bb19 |
| 2 | Гейт check-group-vocabulary.mjs + подключение в lint | e7621d10 |

## Результат

- DeviceFilters «Группировать похожие» -> «Свернуть одинаковые»; DeviceList «Групп: N» -> «Свёрнуто: N»; TableRow aria-label -> «Свернуть/Развернуть»; витрина: «Раскрываемая строка» и «Комбобокс со свёрнутыми наборами (drill-in)».
- Doc-комментарии в `dto/device.rs` переформулированы («набор/свёртка»), идентификаторы не тронуты; `bindings.ts` (в .gitignore) пересоздан через `export_bindings`.
- Гейт: allowlist (features/groups/**, lib/api/groups.ts — целиком; sidebar-config.ts «Группы», DeviceFormBody.svelte «Место задаётся группой», MovementTimeline.svelte «в составе группы» — только по маркеру). Selftest: 9 фикстур (4 негативных, 5 позитивных).
- Мутационная проверка: возврат «Группировать похожие» в DeviceFilters.svelte (подстрока встречалась ровно 1 раз) -> гейт exit 1 с указанием `DeviceFilters.svelte:102`; после отката exit 0, diff файла равен результату Задачи 1.

## Verification

- `cargo test -p trackly-app --test export_bindings` — ok
- `pnpm --dir ui svelte-check` — 0 ошибок (68 прежних предупреждений)
- `pnpm --dir ui lint` — exit 0
- `node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256` — PASS

## Deviations from Plan

Один технический нюанс: критерий «`grep -c check-group-vocabulary ui/package.json` равно 2» буквально даёт 1, так как `grep -c` считает строки, а оба вызова (selftest и боевой) стоят в одной строке lint. Вхождений в строке — 2, как и требовалось.

В остальном план выполнен как написан.

## Known Stubs

None.

## Threat Flags

None.

## Self-Check: PASSED

- ui/scripts/check-group-vocabulary.mjs — найден
- коммиты 8762bb19, e7621d10 — найдены
