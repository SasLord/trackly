---
quick_id: 261124-umn
slug: close-stale-artifacts
status: complete
completed: 2026-11-24
commits:
  - 3b99405f  # Задача 1: статусы быстрозадач
  - 560bfefa  # Задача 2: debug-сессии
  - 3bc73576  # Задача 3: UAT- и VERIFICATION-статусы, перенос в v1.5
---

# Quick 261124-umn: исторические «открытые» артефакты перед закрытием v1.4

Только `.planning/`. cargo и pnpm не запускались. Статус complete/resolved/passed ставился
только там, где есть подтверждение: SUMMARY описывает сделанную работу и коммиты есть в
`main`, или в UAT/VERIFICATION записан итог проверки. Без подтверждения — `accepted`, пункты
перечислены ниже. Каждая правка UAT/VERIFICATION сопровождается полем `status_note`:
было → стало, причина и ссылка.

## Числа до/после

| Категория | До (`gsd-sdk query audit-open`) | После, `gsd-sdk` | После, `gsd-tools.cjs audit-open` |
|---|---|---|---|
| debug_sessions | 8 | 1 | 1 |
| quick_tasks | 74 | 73 (ошибка сканера, см. ниже) | 0 |
| uat_gaps | 30 | 3 | 3 |
| verification_gaps | 8 | 0 | 0 |
| todos | 1 | 1 | 1 |
| **Итого** | **121** | **78** | **5** |

Колонка `gsd-tools.cjs` посчитана после записи этого SUMMARY. До него она давала 6: сама
задача 261124-umn числилась `missing`.

### Почему `gsd-sdk` всё ещё видит 73 быстрозадачи

Из-за ошибки сканера, а не из-за незакрытой работы. TS-порт `audit-open` в SDK
(`sdk/src/query/audit-open.ts`, `scanQuickTasks`) читает только файл с именем ровно
`SUMMARY.md`. Файлы `<quick_id>-SUMMARY.md`, которые предписывает `workflows/quick.md`, он не
видит и помечает папку `missing`. CJS-оригинал (`get-shit-done/bin/lib/audit.cjs`) принимает
обе формы. По CJS все 77 быстрозадач закрыты (включая эту), и все их SUMMARY возвращают
`status: complete` через `gsd-sdk query frontmatter.get`. Отсюда и STATE.md-запись при закрытии
v1.3: «быстрозадачи без поля status». Половина пунктов поле на самом деле имела. Обходной путь
в `.planning/` (дублировать каждый SUMMARY как `SUMMARY.md`) отвергнут: он плодит копии.
Нужна правка сканера SDK — это вне рамок задачи (меняем только `.planning/`). Второе
расхождение того же порта: терминальным статусом UAT он считает только `complete`, а CJS ещё
и `resolved`. Его эта задача сняла нормализацией (ниже).

## Задача 1. Быстрозадачи — коммит 3b99405f

- 39 SUMMARY без поля `status` получили `status: complete`. Ещё 2 без frontmatter (`261107-jcg`,
  `261123-uwq`) получили frontmatter. Сюда же входят 2 бывших `unknown` (`260718-x8t`,
  `261009-ul8`). У каждого из 41 есть «Self-Check: PASSED» или явное описание сделанного, коммиты
  есть в `git log --grep <id>`, а в таблице «Quick Tasks Completed» стоит `complete ✓`.
- `260827-wsu` — SUMMARY не было: docs-коммит `e52e16d4` содержал только PLAN и STATE.
  SUMMARY восстановлен по коммитам `0130206c`, `9c9861c6`, `e52e16d4`, `status: complete`.
- Пункты, где human-verify в SUMMARY значился открытым, проверены отдельно:
  - `260827-gim`, `-rzq`, `-ui0`, `-ui3` подтверждены живым прогоном на Windows и десктопе
    (тело коммита `e52e16d4`).
  - `260702-vtf` подтверждён живой проверкой: по её итогам была сделана follow-up-правка
    `fix-y-axis-integer-ticks`.
- **Закрыто с оговоркой:** `260821-w18` (фильтр категорий в отчёте «Заявки»). Работа выполнена,
  коммиты `9940209b`, `2055f60b`, `038b1b00` есть, все задачи плана сделаны. Но в STATE.md
  стоит «needs review», а живая UAT попапа не записана. Косвенный признак: на следующий день
  пользователь заказал правку вида этой кнопки (`68cdf7cb`).
- `status: incomplete` / `abandoned` не понадобились — незавершённых быстрозадач не нашлось.
- Проверка YAML: все 42 изменённых файла прочитаны через `gsd-sdk query frontmatter.get <file>
  status` и вернули `complete`. Выборочно глазами просмотрены 9 файлов: vtm, x8t, jwf, w18,
  ui3, ul8, jcg, uwq, uw3.

## Задача 2. Debug-сессии — коммит 560bfefa

Все 7 открытых сессий относятся к фазе 40. Исправления в `main` подтверждены
(`git merge-base --is-ancestor`). Каждая переведена в `status: resolved` с полями `fixed_by`
и `confirmed_by` и перенесена в `debug/resolved/`.

| Сессия | Было | Исправление | Подтверждение |
|---|---|---|---|
| cartridge-does-not-follow-printer | diagnosed | b370e0d4, 95a81fc9 (40-21), 0dcd8e03 (40-23) | 40-UAT тест 5, 40-VERIFICATION passed |
| deleted-badge-missing-in-live-report-table | diagnosed | 99f067ec (40-25) | 40-UAT тест 13 |
| from-refill-place-looks-filled | awaiting_human_verify | c8e9940b, d67c6d1f | 40-HUMAN-UAT UAT5-02, Windows 2026-09-04 |
| grouped-device-list-place-inversion | diagnosed | 1145eda7, fa006236 (40-26) | 40-UAT тест 13 |
| lan-print-duplicate-first-page | awaiting_human_verify | 08e56c25 (UAT3-02) | 40-HUMAN-UAT раунд 3 |
| return-to-stock-empty-place-field | diagnosed | 449b6fb2 (40-22), 0dcd8e03 (40-23) | 40-UAT тест 16 |
| timeline-act-link-wrong-subsection | diagnosed | 29655512, f4baf26f (40-24) | 40-UAT тест 7 |

- `ws-disconnect-toast-no-show` уже был `resolved` (58cab3ee), но лежал в корне `debug/`.
  Перенесён в `resolved/`.
- Ссылки на перенесённые файлы обновлены в документах фаз 40, 41.7 и в одном todo.
- `knowledge-base.md` — ложное срабатывание сканера. Это накопительная база знаний
  `gsd-debugger`, без frontmatter, а не сессия. Не тронута (так же она была описана при
  закрытии v1.3).

## Задача 3. UAT и VERIFICATION — коммит 3bc73576

**UAT → `complete` (27 файлов):**
- Нормализация `passed` → `complete` (терминальный статус шаблона UAT). Файлы: 04, 07, 08, 16,
  23, 39, 39.1, 40.4, 41.3, 41.10, 41.11, 41.12, 41.13. В фазе 07 построчные отметки раунда 1
  не проставлены, вердикт `passed` и раунды 2–3 — в файле.
- Нормализация `resolved` → `complete`, все пробелы закрыты в самих файлах. Файлы: 17, 34,
  40-HUMAN-UAT, 40-UAT, 41, 41.1, 41.2-HUMAN-UAT, 41.2-UAT, 41.9.
- По фактам:
  - 05-UAT — `testing`, но 11/11 pass.
  - 30-UAT — `diagnosed`, но 5 пробелов закрыты: 30-VERIFICATION passed 2026-07-29.
  - 03.1-DEFERRED-UAT-ITEMS — `open`, но закрыт фазой 03.2: 3377478, 8ef17cb, f6eacbc,
    fccd04b, 237fa82.
  - 03.3-UAT-ITEMS и 40.5-HUMAN-UAT были без frontmatter. Первый закрыт фазой 03.3, во втором
    пользователь подтвердил все пункты 2026-09-30.
- 40-UAT, тест 17 (сотрудник не видит историю): `[pending]` → `skipped`. Перенесён в
  `.planning/v1.5-DISCUSSION-SEEDS.md`, раздел (е).

**VERIFICATION (8):**
- 04 и 16: `human_needed` → `passed`, их HUMAN-UAT пройдены (4/4, 3/3).
- 24: `gaps_found` → `passed`. Три пробела закрыты: 24-08 (`9da4af2c`, `83055dbc`), 24-09
  (`ee6f0f84`), 24-13. Подпись витрины — 24-11; это общее подтверждение пользователя, а не
  построчная проверка. 24-UAT complete.
- 03, 03.1, 03.2 (v1.0) и 10, 11 (v1.1): → `accepted` со ссылкой на закрытие вехи (v1.1
  закрыта 2026-06-26, `milestones/v1.1-MILESTONE-AUDIT.md`, STATE.md «Deferred Items»).
  Сканер `accepted` не считает пробелом. Сами непройденные human-сценарии остаются открытыми
  в соответствующих HUMAN-UAT (ниже).

## Остались открытыми (и почему)

| Пункт | Причина |
|---|---|
| `03.1-HUMAN-UAT.md` — accepted, 13 сценариев | Построчно не отмечены. Ручной UAT раунда 3 был, но результаты по сценариям не записаны. Веха v1.0. Решение вынесено в v1.5-seeds (е) |
| `10-HUMAN-UAT.md` — accepted, 2 сценария | Рендер employee-оболочки и экрана отказа в LAN-браузере не отмечены. Веха v1.1. В v1.5-seeds (е) |
| `11-HUMAN-UAT.md` — accepted, 7 сценариев | Рендер категорий, дропдауны, WS-тост автору, permission, HTTP-деградация — не отмечены. Веха v1.1. В v1.5-seeds (е) |
| todo `2026-11-21-wr02-multi-execute-transactional.md` | Сознательно отложено в v1.5 (решение при учёте П7, quick 261124-u2k) |
| debug `knowledge-base` | Ложное срабатывание: служебный файл, не сессия |
| 73 быстрозадачи в выводе `gsd-sdk` | Ошибка сканера SDK (имя файла SUMMARY), не открытая работа. Нужна правка `scanQuickTasks` в SDK по образцу CJS |

Оговорка без открытого статуса: `260821-w18` закрыт как complete, хотя живая UAT попапа не
записана (см. задачу 1).

## Отклонения от плана

- **[Rule 1 — фактическая причина]** План объяснял «missing» отсутствием поля `status`.
  Фактически у 32 из 72 «missing» поле уже было — SDK-сканер просто не видит файлы
  `<id>-SUMMARY.md`. Поле добавлено там, где его не было. Ошибка сканера не исправлялась
  (вне `.planning/`), а описана выше.
- **[Rule 2 — согласованность со сканером]** `accepted` для UAT (по плану) оставляет файл в
  выдаче обоих сканеров: их терминальные статусы — `complete` (SDK) и `complete`/`resolved`
  (CJS). Подтверждённые `passed`/`resolved` поэтому нормализованы в `complete`. Неподтверждённые
  сознательно оставлены `accepted` и видны в аудите.

## Self-Check: PASSED

- Коммиты 3b99405f, 560bfefa, 3bc73576 есть в `git log`.
- `260827-wsu-SUMMARY.md`, `debug/resolved/*.md` (8 перенесённых) и раздел (е) в
  `v1.5-DISCUSSION-SEEDS.md` на месте.
- Повторный `gsd-sdk query audit-open` выполнен, результаты — в таблице выше.
