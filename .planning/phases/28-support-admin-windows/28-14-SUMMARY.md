---
phase: 28-support-admin-windows
plan: 14
gap_closure: true
requirements: [WIN-06]
status: complete
---

# 28-14 SUMMARY — GAP-2: Заявки master/detail height + column layout

## Что сделано

Закрыт GAP-2 (28-VERIFICATION.md) плюс серия доработок по живому UAT (3 раунда).

### Исходные задачи (Task 1–2, авто)
- **RequestsPage.svelte** — `.page-content` получил flex-контекст (`display:flex;
  flex-direction:column; min-height:0; overflow-x:auto; overflow-y:hidden`),
  байт-в-байт как проверенный `ActsPage.svelte` (FIX B1). Это корень GAP-2:
  без flex-контейнера-родителя существующий `flex:1 1 auto; min-height:0` у
  `RequestsMasterDetail` был инертен, и скроллилась вся страница вместо
  внутренних панелей. Commit `f9f39ab`.
- **RequestListRow.svelte** — начальный бюджет ширин колонок (Task 2). Позже
  переработан в UAT (см. ниже). Commit `9d6f0ab`.

### UAT round 2–3 (доработки по замечаниям пользователя)
- **Порядок колонок** — «Автор» (с датой) стал первой колонкой:
  Автор | Тип | Описание | Статус. Клавиатурная точка входа и фокус-рамка
  переехали на ячейку «Автор». Commit `06b1b85`.
- **Две пилюли в «Тип»** («Регистрация AD» + «Восстановление доступа»)
  размещаются в две строки. Commit `06b1b85`.
- **Принтер/Картридж пикеры → кастомный Dropdown** — два нативных `<select>`,
  которые верификация пропустила (живут в `$lib/components`):
  `GroupedPrinterSelect` (группировка по локациям, drill-in, без строки поиска)
  и `CartridgeSelect` (плоский список) переписаны на общий `Dropdown`. К
  `Dropdown` добавлен опциональный проп `id` (проброс на поле, чтобы `<label
  for>` сохранил связь). Контракты пропсов не менялись — родительские модалки
  (`RequestFormModal`, `OperationModal`) не тронуты. Commit `4d2099d`.
- **Таблица: разделительные линии + распределение места** — при широком окне
  список терял линии строк и криво распределял ширину. Причина: `.cell-type`
  задавал `display:flex` прямо на `<td>`, что выводит ячейку из модели ширин
  таблицы (в `ActListRow` так не делают). `<td>` возвращён к обычной table-cell,
  вертикальное размещение пилюль ушло во внутренний `.type-badges`; колонки
  распределяются по схеме Актов (Автор/Описание эластичные, Тип по содержимому,
  Статус 110px). Commit `1649bcc`.

## Проверка
- `pnpm --dir ui svelte-check` — 0 ошибок (48 предсуществующих warning'ов, не в
  затронутых файлах).
- `pnpm --dir ui lint` — PASS (eslint + prettier + check-tokens, 0 нарушений).
- `pnpm --dir ui build` — ✓ (bindings перегенерированы через `export_bindings`).
- **Человеческий чекпоинт (обе темы, light/dark)** — одобрено пользователем
  после round 3: высота панелей с нижним отступом, колонка «Автор» видна,
  двойные пилюли в две строки, кастомные пикеры принтера/картриджа, непрерывные
  линии строк и корректное распределение места при растяжении окна.

## Коммиты
- `f9f39ab` fix(28-14): RequestsPage .page-content flex context (GAP-2 root cause)
- `9d6f0ab` fix(28-14): RequestListRow column width budget
- `06b1b85` fix(28-14): Заявки UAT — Автор column first, stacked type badges
- `4d2099d` fix(28-14): Заявки UAT — Принтер/Картридж пикеры на кастомный Dropdown
- `1649bcc` fix(28-14): Заявки UAT — table column distribution + row separators

## Отклонения от плана
Плановые Task 1–2 выполнены как задумано; объём расширен по живому UAT
(колонки/пилюли/пикеры/таблица) — все изменения layout/CSS + миграция двух
native-select на существующий design-system-компонент, без изменения
бизнес-логики.
