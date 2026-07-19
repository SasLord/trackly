---
quick_id: 260719-ocq
slug: close-bl-01-unify-dropdown-drill-in-rese
title: Закрыть BL-01 — унифицировать сброс drill-in состояния между openPanel() и handleInput()
created: 2026-07-19
mode: quick
---

# Quick Task 260719-ocq: Закрыть BL-01 (round-2 code review, Phase 25)

## Problem

`.planning/phases/25-dropdown/25-REVIEW.md` (round 2, gap closure на плане 25-08) нашёл
critical-дефект BL-01 в `ui/src/lib/components/Dropdown.svelte`.

`open = true` ставится ровно в двух местах: `openPanel()` (строка 287) и `handleInput()`
(строка 276). Коммит `09c3f8c` (план 25-08) добавил полный сброс drill-in состояния
(`expandSeq++; viewMode = 'groups'; activeGroup = null; members = []; showBack = false;`)
только в `openPanel()`, как фикс WR-02 из round-1. `handleInput()` — ДРУГОЙ путь,
добавленный в round 1 для фикса CR-01 (ввод текста обязан переоткрывать панель) —
по-прежнему ставит только `open = true; activeIndex = -1;` и ничего не сбрасывает.

Воспроизведение (combobox-вариант, per-row picker устройства в Актах):

1. Пользователь дриллится в раскрываемую группу → `viewMode = 'members'`,
   `showBack = true`, `members = [...]`.
2. Панель закрывается БЕЗ выбора — клик вне (строка 470), `Escape` (366), или `Tab` на
   раскрываемой группе (415, WR-01-фикс из плана 25-08). Все три пути ставят
   `open = false` и намеренно оставляют drill-in состояние нетронутым.
3. Пользователь начинает печатать новый запрос → `handleInput` → `open = true`.
4. Панель немедленно перерисовывается в member-view ПРЕДЫДУЩЕЙ группы —
   `viewMode === 'members'` всё ещё true, `members` — старый список, кликабельный
   весь 250ms debounce + IPC round-trip. Клик пишет `device_id` из прошлого запроса
   (тот же класс бага, что round-1 CR-02).

Дополнительно: шаг 2 не инкрементит `expandSeq`, поэтому ещё не завершённый
`drillInto` из предыдущей сессии проходит guard `seq !== expandSeq` (строка 201) и
форс-перезаписывает переоткрытую по вводу текста панель.

Round-2 ревью также отметило (WR-01, warning, не блокер): безусловный
`expandSeq++` / `viewMode = 'groups'` в `openPanel()` может НАВСЕГДА отбросить
AUTO-05 auto-flatten, потому что `$effect` (строки 162-190) закреплён на `groups` и
не перезапускается при переоткрытии панели. Единственный продакшн-потребитель
(`ActFormItemsTable`) самоисцеляется, потому что его `fetchGroups` всегда присваивает
свежий массив `groups`, заново триггеря эффект — но это на один IPC round-trip шире,
чем нужно, и не самоисцелится у гипотетического потребителя со статичным/мемоизированным
списком групп.

**Решение по WR-01 (обязательное по инструкции — общий хелпер трогает оба вызова, решение
осознанное): OUT OF SCOPE для этой задачи.** Причины:
- Это отдельная warning-находка round 2 про существующее поведение `openPanel()`, не то,
  что описывает BL-01 (BL-01 — про `handleInput()` НЕ делающий сброс вообще).
- Правильный фикс требует не просто вынести сброс в хелпер, а завести новую reactive-
  зависимость (`autoFlattenTick`) внутри самого AUTO-05 `$effect`, который несёт CR-02
  guard — это отдельное, более рискованное изменение того же эффекта, а не
  консолидация двух вызывающих сторон.
- Единственный продакшн-потребитель не задет (самоисцеляется через `fetchGroups`).
- Задача явно и жёстко скоуплена под BL-01 (blockers-only, по прецеденту round-1
  пользовательского решения).

Фикс BL-01 ниже документирует это решение прямо в коде (docstring хелпера), чтобы
следующий ревьюер не принял отсутствие фикса WR-01 за недосмотр.

## Approach

Вынести общий блок сброса, который сейчас живёт только в `openPanel()`
(строки 298-302), в отдельную функцию `resetDrillState()` и вызвать её из ОБОИХ мест,
ставящих `open = true`: `handleInput()` и `openPanel()`. Порядок операций внутри
хелпера — тот же, что уже в `openPanel()` (increment `expandSeq` ПЕРВЫМ, как в
AUTO-05-эффекте) — семантика не меняется, меняется только то, что теперь оба
входа в открытую панель проходят через один и тот же сброс.

Единственный изменяемый файл: `ui/src/lib/components/Dropdown.svelte`.

## Tasks

1. **Вынести `resetDrillState()` и вызвать из `handleInput()` и `openPanel()`** —
   В `ui/src/lib/components/Dropdown.svelte`:
   - Перед `handleInput()` (текущие строки ~262-280) добавить новую функцию
     `resetDrillState()`, тело — ровно блок, который сейчас инлайнится в
     `openPanel()`: `expandSeq++; viewMode = 'groups'; activeGroup = null;
     members = []; showBack = false;` (порядок сохранить, `expandSeq++` первым).
     Docstring хелпера должен: (а) явно ссылаться на BL-01 и объяснять, почему
     сброс нужен из ОБОИХ мест, ставящих `open = true` (переиспользовать
     обоснование из round-2 отчёта — что закрытие панели во время drill-in
     оставляет состояние нетронутым по дизайну, а reopen обязан его смыть);
     (б) явно зафиксировать решение "WR-01 out of scope здесь" с обоснованием
     (самоисцеление продакшн-потребителя через `fetchGroups` + правильный фикс
     требует новой зависимости в AUTO-05 `$effect`, отдельное более рискованное
     изменение).
   - В `handleInput()` добавить вызов `resetDrillState();` сразу после
     `activeIndex = -1;` (перед `onQueryInput?.(query);`) — единственная новая
     строка в теле функции, `open = true` и `activeIndex = -1;` не трогать
     (регрессия CR-01 запрещена).
   - В `openPanel()` заменить пять инлайновых строк сброса (`expandSeq++;` через
     `showBack = false;`, строки ~298-302) на один вызов `resetDrillState();`.
     Существующий развёрнутый комментарий над этим блоком (строки 289-297,
     объясняющий WR-02-логику и порядок операций) — либо удалить как избыточный
     (логика теперь документирована в самом хелпере), либо сократить до ссылки на
     `resetDrillState()`. Не оставлять дублирующий текст в двух местах.
   - НЕ трогать: `drillInto()`, `backToGroups()`, AUTO-05 `$effect` (строки
     162-190), keyboard-слой (`handleKeydown`), любые WR-03/WR-05/WR-09/IN-01/IN-06
     зоны кода (не читать их за пределами уже прочитанного контекста, не менять).
   - files: `ui/src/lib/components/Dropdown.svelte`
   - verify:
     - `pnpm --dir ui svelte-check` — exit 0.
     - `pnpm --dir ui lint` — exit 0.
     - `pnpm --dir ui build` — exit 0 (пересобирает `ui/dist`, нужен для LAN-браузер/
       server-mode проверки, если она понадобится вручную).
     - Ручная проверка (код-ридинг после правки, авто-тестов на Dropdown.svelte
       сейчас нет — Nyquist gap): `grep -n "resetDrillState" ui/src/lib/components/Dropdown.svelte`
       должен показать 1 определение + 2 вызова (в `handleInput` и `openPanel`);
       `open = true` должно по-прежнему стоять первой мутацией в обеих функциях
       (CR-01 regression check); `expandSeq++` не должен встречаться нигде вне
       `resetDrillState()`, AUTO-05 `$effect` и `drillInto()` (не должно появиться
       четвёртого писателя).
   - done: `resetDrillState()` существует, вызывается из `handleInput()` И
     `openPanel()`; повторный инлайн сброса в `openPanel()` удалён; `open = true`
     остаётся первой строкой в обеих функциях; docstring хелпера фиксирует решение
     "WR-01 вне скоупа" с обоснованием; svelte-check/lint/build зелёные.

## must_haves

- truths:
  - Сворачивание панели во время drill-in (click-outside/Escape/Tab на раскрываемой
    группе) без выбора, затем ввод текста в поле — переоткрытая панель показывает
    список ГРУПП по новому запросу, а не устаревший список участников предыдущей
    группы (BL-01 закрыт).
  - In-flight `drillInto`-промис из прошлой сессии не может форс-перезаписать
    панель, переоткрытую через `handleInput()` — `expandSeq` инкрементится и там,
    и в `openPanel()`, одним и тем же путём.
  - CR-01 не регрессирован: `handleInput()` по-прежнему ставит `open = true` первой
    мутацией.
  - CR-02 не регрессирован: guard `seq !== expandSeq` в `drillInto`/AUTO-05-эффекте
    — единственная точка защиты от stale-записи, не тронут.
  - WR-01/WR-02/WR-06 из плана 25-08 не регрессированы (Tab на раскрываемой группе,
    keyboard-слой search input в select-варианте).
- artifacts:
  - `ui/src/lib/components/Dropdown.svelte` — содержит функцию `resetDrillState()`.
- key_links:
  - `handleInput()` → `resetDrillState()` — устраняет BL-01 (новый вызов).
  - `openPanel()` → `resetDrillState()` — тот же хелпер, тот же порядок операций,
    что был инлайн в патче 25-08 (поведение не меняется, только вынесено).

## Constraints

- Единственный изменяемый файл: `ui/src/lib/components/Dropdown.svelte`.
- НЕ регрессировать CR-01 (`open = true` в `handleInput()` остаётся).
- НЕ регрессировать CR-02 (`expandSeq` — generation token, `seq !== expandSeq` guard
  в `drillInto`/AUTO-05-эффекте не трогать).
- НЕ регрессировать WR-01/WR-02/WR-06 плана 25-08 (Tab-фикс на раскрываемой группе,
  keyboard-слой search input).
- НЕ трогать намеренно отложенные находки: WR-03 (focus return), WR-05
  (listbox/`<li>` role nesting), WR-09 (DeviceGroupRow retry-forever), IN-01
  (dead `.hint-warn` CSS), IN-06 (showcase force-open).
- WR-01 (round 2, AUTO-05 auto-flatten discard) — сознательно OUT OF SCOPE, решение
  и обоснование зафиксированы в docstring `resetDrillState()` в коде.
- Никаких новых зависимостей, никаких новых файлов.
