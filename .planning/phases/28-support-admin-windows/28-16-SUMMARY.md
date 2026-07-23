---
phase: 28-support-admin-windows
plan: 16
gap_closure: true
requirements: [WIN-07]
status: complete
---

# 28-16 SUMMARY — GAP-3: Отчёты table framing + height

## Что сделано

Закрыт GAP-3 (28-VERIFICATION.md) плюс доработки по живому UAT (round 2).

### Исходные задачи (Task 1–2, авто)
- **ReportsPage.svelte** — `.reports-content` получил `min-height:0` и
  `overflow-x:auto; overflow-y:hidden` вместо both-axis `overflow:auto`,
  по образцу `ActsPage.svelte` (FIX B1). Это даёт `ReportTable`'s
  `.report-table-wrap { flex:1; min-height:200px }` ограниченную высоту, и
  таблица заполняет остаток вместо скролла всего блока. Commit `35a88dd`.
- **ReportTable.svelte** — убран `framed={false}`, таблица рендерится с дефолтным
  `framed={true}` у `Table` (рамка + радиус 8px + `box-shadow: var(--tr-elev-1)`),
  как у всех standalone-таблиц (Пользователи). Commit `67644cc`.

### UAT round 2 (доработки по замечаниям пользователя)
- **Нижний отступ** — таблица больше не прижата к нижней кромке окна:
  `.reports-content` получил `padding-bottom: var(--tr-space-xl)` (24px, как в
  Актах). Commit `5f93cb8`.
- **Выравнивание controls-row** — селектор Месяц/Год/Диапазон, комбобоксы и
  кнопки экспорта центрированы по одной линии (все 28px); у `.period-selector`
  убран вертикальный «ореол» (`align-items:center`, без vertical padding).
  Commit `5f93cb8`.
- **Dropdown без строки поиска** — у комбобоксов Месяц/Год убрана строка поиска
  сверху. Реализовано через новый проп `searchable` (default `true`) у общего
  `Dropdown` — переиспользуемо в разных вариантах; sticky-offset drill-header
  теперь завязан на реальное наличие строки поиска. Commit `5f93cb8`.

## Проверка
- `pnpm --dir ui svelte-check` — 0 ошибок.
- `pnpm --dir ui lint` — PASS (0 нарушений).
- `pnpm --dir ui build` — ✓.
- Возвраты грузятся без ошибки (совместно с 28-15 GAP-4 fix), диапазон С/По
  читается раздельно (28-13) — подтверждено в совместном чекпоинте.
- **Человеческий чекпоинт (обе темы, light/dark)** — одобрено пользователем:
  таблица в рамке-«карточке», заполняет высоту с нижним отступом, шапка залипает,
  controls-row ровный.

## Коммиты
- `35a88dd` fix(28-16): ReportsPage .reports-content flex context (GAP-3 root cause)
- `67644cc` fix(28-16): ReportTable framed card styling (GAP-3)
- `5f93cb8` fix(28-16): Отчёты UAT — searchless month/year Dropdown, control alignment, bottom padding

## Отклонения от плана
Плановые Task 1–2 выполнены как задумано; объём расширен по живому UAT
(отступ/выравнивание/строка поиска) — layout/CSS + добавление опционального
пропа к существующему `Dropdown`, без изменения данных/логики.
