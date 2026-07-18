---
quick_id: 260718-x8t
slug: tabs-segmented-width
title: Segmented-вариант вкладок обжимает подложку по содержимому (витрина)
created: 2026-07-18
mode: quick
---

# Quick Task 260718-x8t: Segmented-вариант вкладок обжимает подложку по содержимому

## Problem

Диагностировано в UAT фазы 24 (`.planning/phases/24-base-components/24-UAT.md`,
тест 6, gap "Segmented-вариант вкладок обжимает подложку по содержимому...").
Segmented-вариант вкладок в витрине компонентов растягивает подложку на всю
ширину контейнера вместо обжатия по содержимому, как в эталоне.

Эталон `.planning/reference/design-system-v2/Tabs.dc.html:64` задаёт сегменти-
рованному контейнеру `display: inline-flex` — подложка обжимает содержимое.
`ui/src/lib/components/Tabs.svelte:132-137` (`.tabs-segmented`) воспроизводит
это верно (`display: inline-flex`) — компонент не трогаем.

Причина — обёртка витрины: `ui/src/features/showcase/sections/TabsSection.svelte:48-51`
задаёт `.variant-block { display: flex; flex-direction: column; gap: ... }`
без `align-items`. Дефолтный `align-items: stretch` растягивает `inline-flex`
потомка (`.tabs-segmented`) по поперечной оси (ширине).

Underline-вариант (`.tabs-underline`, тоже `display: inline-flex`,
`Tabs.svelte:58-59`) уже сегодня не страдает от этого дефекта — сам эталон
для него полноширинный (`Tabs.dc.html:43`, `display: flex`), но ширина у него
и так берётся не из растяжения родителем, а из содержимого/border-bottom.
Фикс `align-items: flex-start` затрагивает оба варианта одинаково (оба —
`inline-flex`), поэтому после фикса нужно явно проверить оба визуально.

## Approach

Один точечный CSS-фикс в файле витрины (не в самом компоненте `Tabs.svelte`):
добавить `align-items: flex-start` в `.variant-block` в
`ui/src/features/showcase/sections/TabsSection.svelte`.

Segmented-подложка (`inline-flex`) перестанет растягиваться по поперечной оси
и обожмётся по содержимому — как в эталоне.

Underline-вариант тоже `inline-flex`, поэтому теоретически мог бы визуально
измениться тем же способом — но его блок не должен визуально "сжаться":
switch-bar по эталону и так не растягивается на всю ширину контейнера в
`Tabs.dc.html` (`display: flex` в разметке относится к строке табов, а не к
самой обёртке-контейнеру уровня `.variant-block`), полноширинность
underline'а обеспечивает `border-bottom` внутри каждого `.tab`, а не
растяжение всего `.tabs-underline` блока родителем. Проверить это визуально
после фикса (см. Task 1, verify) — если underline неожиданно сожмётся и
перестанет выглядеть полноширинным switch-bar'ом, добавить `align-self: stretch`
именно на `.tabs-underline` (не менять `.variant-block` обратно), но только
если фактически потребуется.

## Tasks

1. **CSS-фикс: обжать segmented-подложку по содержимому в витрине** — В
   `ui/src/features/showcase/sections/TabsSection.svelte`, в блоке
   `.variant-block` (строки ~48-51), добавить свойство
   `align-items: flex-start;` (после `flex-direction: column;`, перед или
   после `gap: var(--tr-space-sm);` — порядок свойств не важен). Не менять
   `ui/src/lib/components/Tabs.svelte`. Пересобрать витрину:
   `pnpm --dir ui build` (проект: серверный режим/LAN-браузер отдаёт
   устаревший `ui/dist`, если не пересобрать). Прогнать
   `pnpm --dir ui lint` (валидирует токены через `check-tokens.mjs` — свойство
   `align-items: flex-start` не токенизируемое значение, ошибок быть не
   должно) и `pnpm --dir ui svelte-check`. Открыть витрину
   (`#/showcase`, раздел «Вкладки») в браузере и визуально подтвердить: (а)
   segmented-подложка («Список / Карта / Таблица») теперь обжата по
   содержимому, не тянется на всю ширину секции; (б) underline-вариант
   («Все / Открытые / Закрытые / Архив») остаётся визуально полноширинным
   switch-bar'ом, как было до фикса. Если (б) нарушится — добавить
   `align-self: stretch;` в `.tabs-underline` в `Tabs.svelte` (единственное
   допустимое изменение в компоненте, и только если визуально подтверждён
   регресс).
   - files: `ui/src/features/showcase/sections/TabsSection.svelte`
     (+ `ui/src/lib/components/Tabs.svelte` только если понадобится
     `align-self: stretch` на `.tabs-underline` по факту регресса)
   - verify: `pnpm --dir ui lint` exits 0; `pnpm --dir ui svelte-check` exits 0;
     `pnpm --dir ui build` exits 0; визуальная проверка `#/showcase` в браузере
     (или Tauri dev webview) — segmented обжат, underline остаётся полноширинным.
   - done: `.variant-block` в `TabsSection.svelte` имеет `align-items: flex-start`;
     `Tabs.svelte` не изменён (либо изменён только с добавлением
     `align-self: stretch` на `.tabs-underline`, если это оказалось необходимо);
     lint/svelte-check/build зелёные; `ui/dist` пересобран.

## must_haves

- truths:
  - Segmented-вариант вкладок в витрине компонентов (`#/showcase`) обжимает
    подложку по ширине содержимого, а не растягивается на всю ширину секции —
    как в эталоне `Tabs.dc.html:64`.
  - Underline-вариант вкладок в витрине остаётся визуально полноширинным
    switch-bar'ом (не регрессирует из-за фикса).
  - `Tabs.svelte` не изменён, если фактическое поведение underline не
    потребовало `align-self: stretch`.
- artifacts:
  - `ui/src/features/showcase/sections/TabsSection.svelte` — `.variant-block`
    содержит `align-items: flex-start`.
- key_links:
  - `.variant-block { align-items: flex-start }` → снимает
    `align-items: stretch`-растяжение поперечной оси на `.tabs-segmented`
    (`display: inline-flex`, `Tabs.svelte:132-137`), давая инлайн-контейнеру
    обжаться по содержимому.

## Constraints

- Ровно одна задача, один атомарный коммит.
- Никаких новых зависимостей.
- `Tabs.svelte` не трогать, кроме единственного допустимого случая:
  `align-self: stretch` на `.tabs-underline`, и только если визуальная
  проверка подтвердит, что underline перестал быть полноширинным после
  фикса `.variant-block`.
