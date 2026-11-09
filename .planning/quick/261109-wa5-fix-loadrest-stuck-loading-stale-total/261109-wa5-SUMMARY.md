---
phase: quick-261109-wa5
plan: 01
status: complete
requirements: [GRD-10]
key-files:
  modified:
    - ui/src/features/devices/DevicesPage.svelte
    - ui/scripts/check-device-list-mode.mjs
    - .planning/phases/41.9-container-groups-devices/41.9-VALIDATION.md
completed: 2026-11-09
---

# Quick 261109-wa5: loadRest не зависает в loadingRest, total актуален

Исправлены два дефекта `loadRest()` (аудит 41.9): осиротевший `loadingRest` после устаревания только по `anchorsSeq` и устаревший `total` при пустой последней странице. Гейт `check-device-list-mode` обновлён и закреплён новыми мутантами.

## Что сделано

- `DevicesPage.svelte`: хелпер `releaseStale` (после первой записи `loadingRest: true`, до `try`). Первым оператором сверяет `anchorLoadSeq.get(groupId) !== myGen`, затем снимает флаг у существующей записи, состав не трогает. Оба выхода по охране (после ответа страницы и в catch) теперь `return releaseStale()`. `totalCount = next.total` стоит выше `break`. `current()`, `reloadExpandedAnchors` и `await tick()` не менялись.
- `check-device-list-mode.mjs`: `LOAD_REST_STALE_GUARD_RE` вместо старой охраны; [L1] проверяет хелпер (положение, проверка поколения первым оператором, сброс флага, нет current/expandedAnchors/entries/total/loadedAll); [L2] требует total выше break. Селфтест: константы под новый исходник, мутанты: хелпер без проверки поколения, выход после ответа без сброса, выход в catch без сброса, total после break (перестановка), пустой хелпер. Неиспользуемый `LOAD_REST_GUARD_RE` удалён (eslint).
- `41.9-VALIDATION.md`: пункты 1 и 2 помечены исправленными (261109-wa5), пункт 3 оставлен. Не закоммичен (по указанию).

## Верификация (каждая команда отдельно)

- `node scripts/check-device-list-mode.mjs --selftest`: exit 0 (82 кейса, 35 + 57 структурных мутантов)
- `node scripts/check-device-list-mode.mjs`: exit 0
- `pnpm lint`: exit 0
- `pnpm svelte-check`: 0 ошибок (75 прежних предупреждений)
- `prettier --check` по двум файлам: exit 0
- Ручные мутации реального DevicesPage (файл возвращён, `cmp` совпал, гейт зелёный): без проверки поколения в хелпере даёт [L1]; `return;` после ответа даёт [L1]; `return;` в catch даёт [L1]; total после break/push даёт [L2]. Для массовой замены `return;` exit=1.
- Cargo не запускался (Rust не менялся).

## Отклонения

Удалён `LOAD_REST_GUARD_RE` (стал неиспользуемым, eslint no-unused-vars); `prettier --write` применён к гейту. Остальное по плану.

## Поведение НЕ проверено в рантайме

Гейт проверяет только форму кода. Чеклист для ручной проверки:

- [ ] «Устройства»: раскрыть группу с составом больше 200 (или с «Показать весь состав»), нажать догрузку и, пока идёт, свернуть группу, вызвать обновление списка и раскрыть группу снова: индикатор догрузки не висит вечно.
- [ ] Нажать догрузку ещё раз: вторая догрузка запускается и доводит состав до конца.
- [ ] Группа, сократившаяся на сервере ровно до загруженного числа: после догрузки счётчик в строке группы показывает актуальное число.

Коммит кода и гейта: см. `git log` (fix(ui): loadRest не зависает в loadingRest...).
