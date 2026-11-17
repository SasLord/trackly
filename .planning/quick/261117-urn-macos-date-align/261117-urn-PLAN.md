---
phase: quick-261117-urn
plan: 01
type: quick
files_modified:
  - ui/src/lib/components/DatePicker.svelte
  - ui/scripts/check-date-picker-layout.mjs
---

# Quick 261117-urn: дата в поле «Дата» прижата влево на macOS (WKWebView)

**Цель:** в десктопном приложении на macOS выбранная дата в `<input type="date">` стоит слева,
как на Windows и в LAN-браузере (DatePicker и PeriodSelector).

## Задача 1 — причина и правка
- UA-стиль WebKit задаёт `input[type=date]` `align-items: center`. Наш `.date-picker` —
  `display:flex; flex-direction:column`, поэтому в колонке это центрирует значение по горизонтали.
- `::-webkit-date-and-time-value { text-align: left }` не действует: macOS-WebKit рисует значение
  через `::-webkit-datetime-edit`.
- Правка: явный `align-items: stretch` в `.date-picker` (гейт уже разрешал stretch/normal, Chromium не страдает).

## Задача 2 — гейт
- `check-date-picker-layout.mjs`: flex-колонка без явного `align-items: stretch|normal` — красная;
  фикстуры selftest обновлены; мутация реального исходника (удалить stretch) — красная.

## Проверка
- Системный WKWebView (Swift-пробник, собранный CSS из `ui/dist`): до — центр, после — слева, вертикаль сохранена.
- `pnpm --dir ui lint`, `svelte-check` (0 errors / 75 warnings).
- Живой взгляд в `cargo tauri dev` — за пользователем.
