---
phase: quick-261123-uwq
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - ui/src/features/places/PlaceEntityViewModal.svelte
  - ui/src/features/printers/PrinterDetail.svelte
  - ui/src/features/cartridges/CartridgeDetail.svelte
  - ui/scripts/check-history-timeline-refresh.mjs
  - ui/package.json
  - .planning/v1.4-MILESTONE-AUDIT.md
autonomous: true
requirements: [HST-05]
must_haves:
  truths:
    - "Открытая лента «История» в попапе устройства/принтера/расходника перечитывается, когда другой клиент изменил данные (поднялся placeContentEventsStore.reloadSeq), без переоткрытия попапа"
    - "Ответ ленты от прежней записи не подменяет ленту текущей при быстром переключении карточек (PrinterDetail, CartridgeDetail)"
    - "Каждый .svelte-файл, вызывающий history_get_timeline, имеет подписку на перечитывание и защиту от устаревшего ответа; новый файл без них краснит pnpm lint"
  artifacts:
    - path: "ui/scripts/check-history-timeline-refresh.mjs"
      provides: "Гейт: инвентарь от glob ui/src, правила (а) подписка (б) защита от устаревшего ответа, allowlist, --selftest"
  key_links:
    - from: "ui/src/features/places/PlaceEntityViewModal.svelte"
      to: "placeContentEventsStore.reloadSeq"
      via: "$effect + untrack, lastReloadSeq не $state"
      pattern: "reloadSeq"
---

<objective>
Закрыть R8-01 (лента «История» в попапе просмотра не перечитывается у второго клиента: loadTimeline выходит по кэшу timelineLoadedFor) и R8-04 (нет защиты от устаревшего ответа в PrinterDetail/CartridgeDetail), плюс гейт, не дающий новой ленте появиться без подписки (урок «новая лента — старая инвалидация»).

Output: правки трёх .svelte, гейт check-history-timeline-refresh.mjs в pnpm lint, отметки CLOSED в аудите.
Только UI; бэкенд не трогать; cargo не запускать. Приватность: репо публичный, в тексте/фикстурах только вымышленные данные.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@.planning/v1.4-MILESTONE-AUDIT.md
@ui/src/features/places/PlaceEntityViewModal.svelte
@ui/src/features/groups/GroupsPage.svelte
@ui/src/features/groups/GroupPanel.svelte
@ui/src/features/printers/PrinterDetail.svelte
@ui/src/features/cartridges/CartridgeDetail.svelte
@ui/scripts/check-group-gone-silent.mjs
@ui/scripts/_strip.mjs

<interfaces>
Стор: import { placeContentEventsStore } from '$lib/stores/placeContentEvents.svelte'; поле placeContentEventsStore.reloadSeq (число, растёт при событии другого клиента).

Образец (GroupsPage ~:175-189, Фаза 41.7 D-17): let lastReloadSeq = placeContentEventsStore.reloadSeq (НЕ $state); $effect читает ТОЛЬКО reloadSeq, в untrack сравнивает с lastReloadSeq, обновляет его и поднимает токен. Запись токена/lastReloadSeq вне untrack даёт effect_update_depth_exceeded, который svelte-check/eslint/build НЕ ловят.

Образец защиты (GroupPanel ~:131-150): эффект читает void refreshToken; let cancelled = false; ответы проверяют if (!cancelled); cleanup эффекта ставит cancelled = true.

PlaceEntityViewModal: состояние ленты historyEntries/timelineLoading/timelineError/timelineToken/timelineLoadedFor (~:121-125); loadTimeline(r) (~:221) выходит при timelineLoadedFor === key; эффект сброса по смене row (~:245) объявлен РАНЬШЕ эффекта загрузки (~:258, if (pressed) untrack(loadTimeline(r))) — порядок эффектов сохранить.
PrinterDetail ~:177-199 и CartridgeDetail ~:85-104: $effect зовёт apiCall('history_get_timeline') и в then/catch/finally пишет movements/movementsLoadError/movementsLoading без токена.
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Подписка на reloadSeq и защита от устаревшего ответа в трёх компонентах</name>
  <files>ui/src/features/places/PlaceEntityViewModal.svelte, ui/src/features/printers/PrinterDetail.svelte, ui/src/features/cartridges/CartridgeDetail.svelte</files>
  <action>
(R8-01) PlaceEntityViewModal: импортировать placeContentEventsStore. Добавить отдельный $effect по образцу GroupsPage: let lastReloadSeq = placeContentEventsStore.reloadSeq (обычная переменная); эффект читает ТОЛЬКО placeContentEventsStore.reloadSeq, всё остальное (сравнение, запись lastReloadSeq, сброс timelineLoadedFor = null, вызов loadTimeline(row)) — внутри untrack. Перечитывать только если лента уже была запрошена (timelineLoadedFor !== null) И лента сейчас показана (pressed); если не показана — достаточно сбросить timelineLoadedFor = null, чтобы следующее открытие прочитало заново. Историю НЕ очищать перед перечиткой (без мигания); существующий timelineToken уже отбрасывает устаревший ответ. Эффект объявить ПОСЛЕ эффектов сброса по row и загрузки (порядок эффектов важен), не трогать их логику.

(R8-04) PrinterDetail и CartridgeDetail: в существующем $effect с history_get_timeline добавить let cancelled = false, return () => { cancelled = true; } и проверки if (!cancelled) во всех трёх ветках then/catch/finally (как GroupPanel). Раннюю ветку p === null / c === null оставить. Подписать оба на тот же стор: в начале эффекта void placeContentEventsStore.reloadSeq (чтение как зависимость; запись состояния в эффекте остаётся только через промис-колбэки, а movementsLoading/movementsLoadError выставляются синхронно без чтения — эффект их не читает, цикла нет). Если при проверке окажется, что подписка создаёт цикл или тянет за собой правки родителей — оставить ТОЛЬКО cancelled-защиту и записать остаток в SUMMARY.

Никаких комментариев с реальными именами/реквизитами. Не вводить новые $state, читаемые и пишущиеся в одном эффекте.
  </action>
  <verify>
    <automated>pnpm --dir ui exec svelte-check && pnpm --dir ui build</automated>
  </verify>
  <done>svelte-check 0 ошибок, build зелёный; три файла содержат подписку/защиту; запись lastReloadSeq только в untrack.</done>
</task>

<task type="auto">
  <name>Task 2: Гейт check-history-timeline-refresh.mjs + подключение в pnpm lint</name>
  <files>ui/scripts/check-history-timeline-refresh.mjs, ui/package.json</files>
  <action>
Создать гейт по образцу check-group-gone-silent.mjs (импорт stripComments из ./_strip.mjs, флаги --src=<dir> и --selftest, префикс [check-history-timeline-refresh]). Инвентарь строить ОТ репозитория: рекурсивно обойти ui/src, взять каждый .svelte, чей текст без комментариев содержит литерал 'history_get_timeline'; найдено должно быть > 0 (защита от вакуума) и не меньше числа записей allowlist+проверяемых. Для каждого файла правила:
(а) читает placeContentEventsStore.reloadSeq (в тексте без комментариев) ЛИБО файл в явном allowlist PROP_TOKEN: { 'src/features/groups/GroupPanel.svelte': 'принимает проп refreshToken, в GroupsPage связан с reloadSeq (D-17 41.7)' } — для allowlist-файла гейт проверяет `void refreshToken` в тексте файла И что родитель GroupsPage.svelte содержит reloadSeq и поднимает panelRefreshToken; причина обязательна (пустая строка = нарушение);
(б) защита от устаревшего ответа: либо `cancelled` (объявление let cancelled + проверка !cancelled), либо сравнение токена (`token !== ...Token` / `=== ...Token`) в тексте файла.
Для (а) в файлах со стором дополнительно требовать, что запись lastReloadSeq/токена происходит внутри untrack( — искать `untrack(` в том же файле.
Освобождения (EXEMPT) — явный объект «файл → причина», изначально пустой; запись с пустой причиной или несуществующим файлом = нарушение (протухший allowlist).
Выводить нарушения с путём файла, exit 1; при успехе PASS с числом проверенных файлов.
--selftest: на встроенных строковых фикстурах (вымышленные, без реальных данных) проверить: файл с подпиской и cancelled — проходит; без reloadSeq — красный; без защиты — красный; allowlist без причины — красный; пустой инвентарь — красный; stripComments: закомментированный reloadSeq не засчитывается.
В ui/package.json добавить в конец цепочки lint, ПЕРЕД check-lint-selftest-wiring: `node scripts/check-history-timeline-refresh.mjs --selftest && node scripts/check-history-timeline-refresh.mjs` (мета-гейт check-lint-selftest-wiring требует обе формы).
  </action>
  <verify>
    <automated>node ui/scripts/check-history-timeline-refresh.mjs --selftest && node ui/scripts/check-history-timeline-refresh.mjs && pnpm --dir ui lint</automated>
  </verify>
  <done>Гейт зелёный на HEAD после Task 1, selftest зелёный, pnpm lint зелёный (включая check-lint-selftest-wiring).</done>
</task>

<task type="auto">
  <name>Task 3: Мутационная проверка, финальные гейты, отметка в аудите</name>
  <files>.planning/v1.4-MILESTONE-AUDIT.md</files>
  <action>
1) Прогон на HEAD ДО фикса: через git stash (или git worktree/`git show HEAD:<путь>` в копию во временный каталог scratchpad) взять исходники без правок Task 1, запустить гейт с --src=<копия ui/> — ожидается КРАСНЫЙ (PlaceEntityViewModal, PrinterDetail, CartridgeDetail). Вернуть рабочее дерево (git stash pop), убедиться что правки на месте.
2) Мутация: временно удалить из PlaceEntityViewModal эффект подписки (или чтение reloadSeq) — гейт КРАСНЫЙ; вернуть — ЗЕЛЁНЫЙ. Якорь мутации — уникальная строка, проверить что replace затронул ровно одно вхождение. Вторая мутация: убрать cancelled-проверки из PrinterDetail — красный; вернуть — зелёный.
3) Финал, все четыре команды раздельно (не включают друг друга): `pnpm --dir ui exec svelte-check` (0 ошибок), `pnpm --dir ui lint`, `pnpm --dir ui build`, `node ui/scripts/check-history-timeline-refresh.mjs`.
4) Перед коммитом грепнуть изменённые файлы на реальные ФИО/реквизиты (их быть не должно).
5) Коммит кода (русское сообщение, с Co-Authored-By строкой из системы), затем в .planning/v1.4-MILESTONE-AUDIT.md отметить R8-01 (строка таблицы ~:211 и запись frontmatter ~:41) и R8-04 (~:214, ~:62) как CLOSED (quick 261123-uwq, хэши коммитов); остальное в файле не менять. Закоммитить.
6) Создать SUMMARY.md: результаты трёх проверок (HEAD-красный, мутация-красный/зелёный, вторая мутация), решение по подписке PrinterDetail/CartridgeDetail (подписаны или только cancelled + остаток). Явно написать: живая проверка двумя клиентами (десктоп + LAN-браузер: открыть попап устройства, во втором клиенте переместить устройство/сменить статус — лента должна обновиться без переоткрытия; то же для принтера и расходника; убедиться в консоли отсутствия effect_update_depth_exceeded) ОТДАЁТСЯ ПОЛЬЗОВАТЕЛЮ и НЕ выполнена исполнителем; compile-гейты рантайм-ошибок рун не ловят.
  </action>
  <verify>
    <automated>pnpm --dir ui exec svelte-check && pnpm --dir ui lint && pnpm --dir ui build && node ui/scripts/check-history-timeline-refresh.mjs</automated>
  </verify>
  <done>Все четыре команды зелёные; в SUMMARY зафиксированы красный-на-HEAD и мутация; R8-01/R8-04 отмечены CLOSED в аудите; живая проверка помечена как не выполненная.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| WS-событие -> UI | reloadSeq только триггерит перечитывание через существующую команду history_get_timeline; новых входов нет |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-uwq-01 | I | история устройства | accept | права на history_get_timeline проверяются бэкендом; UI лишь повторяет тот же вызов |
| T-uwq-02 | D | цикл эффектов (effect_update_depth_exceeded) | mitigate | запись токенов только в untrack, lastReloadSeq не $state; живая проверка у пользователя |
| T-uwq-03 | I | приватность репо | mitigate | вымышленные фикстуры selftest, греп перед коммитом |
</threat_model>

<verification>
Четыре команды раздельно: svelte-check, lint, build, node гейта. Мутационные результаты в SUMMARY. Живая проверка двумя клиентами — за пользователем.
</verification>

<success_criteria>
- Лента в попапе перечитывается по reloadSeq; PrinterDetail/CartridgeDetail защищены от устаревшего ответа
- Гейт в pnpm lint, красный на HEAD до фикса и при мутации, зелёный после
- R8-01 и R8-04 отмечены CLOSED в аудите, остальное не тронуто
</success_criteria>

<output>
Create `.planning/quick/261123-uwq-r8-01-ws-r8-04/261123-uwq-SUMMARY.md` when done
</output>
