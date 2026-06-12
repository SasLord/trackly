---
status: partial
phase: 04-cartridges
source: [04-VERIFICATION.md, 04-06-PLAN.md]
started: "2026-06-12T00:00:00Z"
updated: "2026-06-12T00:00:00Z"
---

## Current Test

[awaiting human testing]

## Tests

### 1. Полный end-to-end lifecycle в разделе «Картриджи»
expected: Запустить `cargo tauri dev`, открыть раздел «Картриджи», выполнить сценарий из PLAN 04-06 Task 3 (шаги 1–15): создание модели с матрицей совместимости, создание экземпляра с авто-кодом C-000001, установка в принтер, возврат на склад, switch-bar фильтрация, поиск, баннер низкого остатка. Все шаги без ошибок; Toast-уведомления появляются; статусы и счётчики обновляются реактивно.
result: [pending]

### 2. Скрытие поля «Цвет» при выборе типа «Фотобарабан»
expected: В форме «Новая модель картриджа» переключить тип с «Картридж» на «Фотобарабан» — поле «Цвет» немедленно скрывается; при возврате к «Картридж» поле снова появляется.
result: [pending]

### 3. Focus-open autocomplete в CompatibilityEditor
expected: В форме модели нажать «+ Добавить принтер», кликнуть в поле «Бренд принтера» без ввода — dropdown открывается сразу и показывает ранее введённые бренды (пустой при первом запуске). Аналогично для «Модель принтера» после выбора бренда. (Примечание: бэкенд-баг field-name mismatch исправлен — CR-01.)
result: [pending]

### 4. Human-verify checkpoint из PLAN 04-06 Task 3 (blocking gate)
expected: Выполнить все 15 шагов сценария из `<how-to-verify>` Task 3. Написать "approved", если раздел работает корректно.
result: [pending]

## Summary

total: 4
passed: 0
issues: 3
pending: 4
skipped: 0
blocked: 0

## Gaps

### UAT Round 1 — 2026-06-12 (issues found, fixes applied)

- **GAP-1 (Critical) — `ReaderPool` exhaustion → mutex poison panic.** `cargo tauri dev`
  паниковал при открытии раздела «Картриджи»: `ReaderPool exhausted` под удержанным
  локом → `PoisonError` каскадом → весь reader-пул мёртв процесс-wide. Причина:
  CartridgesPage `loadAll()` (`Promise.all([list, counts, lowStock])` + model_list/
  search) даёт >4 одновременных чтений против пула size=4.
  **Fix:** `acquire()` теперь блокируется на `Condvar` (queue-on-exhaust) + poison-
  устойчивые `lock()` (into_inner); размер 4→8. Регресс-тест добавлен. Commit `c2e5626`.

- **GAP-2 — Модель создаётся («уже создана» при повторе), но не отображается в списках;
  первый сабмит даёт «Данные изменились в другом окне».** Downstream-симптом GAP-1:
  writer-create проходит (отдельный канал), но последующие reader-чтения (refresh
  списка, post-create read) били в мёртвый пул → пустые списки и неверно
  смапленная ошибка. `list_models` SQL корректен (`WHERE deleted_at_utc IS NULL`).
  **Ожидается resolved через GAP-1 — требует повторного ручного теста.**

- **GAP-3 (UX) — switch-bar статусов был зажат внутри узкой колонки списка.**
  **Fix:** `CartridgeFilters` вынесен на уровень страницы (полная ширина, между
  строкой поиска и списком). Commit `158bab5`.

Status: fixes applied, backend tests + clippy + svelte-check + lint зелёные.
Awaiting UAT Round 2 (re-run `cargo tauri dev`).
