---
status: resolved
trigger: "uat-41.2-group-delete-error-toasts: удаление контейнерной группы, из которой ранее актом выведены устройства, показывает два красных тоста-ошибки, хотя группа удаляется"
created: 2026-11-05T15:00:00Z
updated: 2026-11-05T15:40:00Z
---

## Current Focus

hypothesis: CONFIRMED (по коду) — гонка WS-события `entities_changed` (op delete_group) с ответом invoke `groups_delete`. Событие приходит в webview раньше, чем резолвится promise удаления; GroupsPage поднимает panelRefreshToken, пока `selected` ещё = удаляемая группа; смонтированная GroupPanel перечитывает card(21) и GroupContentsTable — composition(21); оба получают NotFound и показывают красные тосты.
test: чтение кода бэкенда (delete_group, broadcast, мост Tauri) и фронта (GroupsPage, GroupPanel, GroupContentsTable, Layout, placeContentEvents)
expecting: n/a (find_root_cause_only)
next_action: вернуть диагноз оркестратору

## Symptoms

expected: Группа удаляется без ошибок. Устройства в удалённую группу не возвращаются. Желательно информировать пользователя (не ошибкой). Позднее удаление акта даёт зелёный + жёлтые тосты «…группа удалена.» — уже работает верно.
actual: Два красных тоста: один с текстом «group 21» (сырой идентификатор / NotFound), второй длиннее (пользователь не запомнил, сказал «на английском»). Группа при этом удалилась.
errors: «group 21» + второе сообщение
reproduction: Тест 4(a) в 41.2-UAT.md: группа с местом, актом вывести устройства, удалить группу в «Группах». Десктоп Windows (Tauri invoke).
started: найдено на UAT 2026-11-05

## Eliminated

- hypothesis: удаление группы частично падает на ссылках эффектов акта (V049, FK на group_id)
  evidence: delete_group — одна транзакция, `DELETE FROM groups WHERE id=?`, commit; при FK-ошибке откатилось бы всё и группа осталась бы. Группа удалилась → запись прошла целиком; delete_group после .await только шлёт broadcast и возвращает Ok.
  timestamp: 2026-11-05T15:20:00Z

- hypothesis: ошибку показывает сам GroupDeleteModal
  evidence: GroupDeleteModal пишет ошибку инлайн (serverErr), не тостом; при успехе зовёт onDeleted().
  timestamp: 2026-11-05T15:20:00Z

## Evidence

- timestamp: 2026-11-05T15:05:00Z
  checked: crates/trackly-core/src/error.rs
  found: NotFound форматируется как "{entity} {id} not found" → "group 21 not found"; фронт показывает e.message как есть.
  implication: тост «group 21» = сырой NotFound для группы 21, т.е. кто-то читал группу ПОСЛЕ удаления.

- timestamp: 2026-11-05T15:10:00Z
  checked: group_service.rs delete_group (стр. ~771-838), entities_broadcast.rs
  found: после коммита вызывается broadcast_entities("delete_group", [], [], [id]) ДО возврата Ok(out). Пустые place_ids при непустых group_ids → событие отправляется (D-17 «перезагрузить всё»).
  implication: событие уходит в канал раньше, чем ответ команды вернётся во фронт.

- timestamp: 2026-11-05T15:12:00Z
  checked: crates/trackly-app/src/main.rs (мост ws_broadcast → app_handle.emit("trackly-event"))
  found: на десктопе отдельная задача сразу эмитит каждое событие в webview; порядок относительно ответа invoke не гарантирован (практически событие приходит первым).
  implication: окно гонки есть именно на десктопе, где воспроизвели.

- timestamp: 2026-11-05T15:15:00Z
  checked: Layout.svelte → notifyEntitiesChanged → placeContentEventsStore.reloadSeq += 1; GroupsPage $effect по reloadSeq → panelRefreshToken += 1
  found: эффект не проверяет, жива ли выбранная группа; `selected` ещё указывает на группу 21 (handleDeleted / handleSelect(null) выполнятся только после резолва groups.delete).
  implication: GroupPanel (key group:21) остаётся смонтированной и получает новый refreshToken.

- timestamp: 2026-11-05T15:18:00Z
  checked: GroupPanel.svelte loadCard (стр. ~78-108), GroupContentsTable.svelte loadComposition (стр. ~87-116)
  found: GroupPanel по refreshToken зовёт groups.card(21) → NotFound → pushToast('error', "group 21 not found"). GroupContentsTable (вкладка «Состав», по умолчанию) по тому же токену зовёт groups.composition(21) → NotFound → pushToast('error', 'Не удалось загрузить группы. Проверьте подключение и повторите.'). История/Свойства ошибки тостом не показывают.
  implication: ровно два красных тоста; второй — длинный (пользователь запомнил его как «английский», по коду он русский; английская часть — только первый тост).

- timestamp: 2026-11-05T15:25:00Z
  checked: связь с актом
  found: ни delete_group, ни путь фронта не читают эффекты акта; гонка срабатывает при удалении ЛЮБОЙ выбранной (открытой в правой панели) группы на десктопе. Акт в сценарии не причинный.
  implication: воспроизводится и на группе без актов, если она выбрана в дереве во время удаления. Не воспроизводится, если удалять невыбранную группу через меню узла.

## Resolution

root_cause: Гонка инвалидации. GroupService::delete_group шлёт entities_changed (group_ids=[id]) до возврата результата; на десктопе мост эмитит его в webview раньше резолва invoke. GroupsPage на reloadSeq поднимает panelRefreshToken, пока selected всё ещё = удалённая группа; GroupPanel перечитывает groups.card(id), GroupContentsTable — groups.composition(id); оба получают NotFound и выводят красные тосты («group 21 not found» и «Не удалось загрузить группы…»). Удаление при этом успешно; акт ни при чём.
fix: (не применялся — режим диагностики)
verification: (не применялась)
files_changed: []


## Закрыто 2026-11-07
Подтверждено живой UAT 41.2 (G1–G12).
