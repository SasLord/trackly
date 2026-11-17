---
phase: quick-261117-urn
plan: 01
status: complete
key-files:
  modified:
    - ui/src/lib/components/DatePicker.svelte
    - ui/scripts/check-date-picker-layout.mjs
completed: 2026-11-17
---

# Quick 261117-urn: дата в поле «Дата» прижата влево на macOS

Причина центрирования в WKWebView — UA-стиль WebKit `align-items: center` у `input[type=date]`.
Вместе с нашим `display:flex; flex-direction:column` он центрирует значение по горизонтали.
Прежняя попытка `::-webkit-date-and-time-value { text-align: left }` ничего не меняла: на macOS
значение рисуется через `::-webkit-datetime-edit`.

## Что сделано
- `DatePicker.svelte`: в `.date-picker` добавлен явный `align-items: stretch`, комментарии исправлены.
  Правило с `::-webkit-date-and-time-value` оставлено: оно безвредно, и гейт его требует.
- `check-date-picker-layout.mjs`: flex-колонка без явного `align-items: stretch|normal` теперь даёт
  нарушение. Selftest расширен, прежний «чистый» вариант без align-items перенесён в плохие.
- PeriodSelector отдельной правки не требует: он только переопределяет размеры `:global(.date-picker)`.

## Проверка
- Системный WKWebView (Swift-пробник `WKWebView.takeSnapshot`): computed `align-items` до правки — `center`,
  после — `stretch`. На снимке до правки дата по центру, после — слева, вертикальное центрирование сохранено.
  Снимок сделан и на собранном `ui/dist` CSS, для формы и для С/По в PeriodSelector.
- Гейт: selftest OK, реальный исходник OK, мутация (удалить stretch) — FAIL, как и ожидалось.
- `pnpm --dir ui lint` — exit 0; `svelte-check` — 0 errors, 75 warnings (базовая линия).
- Внутри запущенного `cargo tauri dev` не смотрел — ожидается взгляд пользователя.
