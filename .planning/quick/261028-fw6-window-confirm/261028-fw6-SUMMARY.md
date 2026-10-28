---
phase: quick-261028-fw6
plan: 01
subsystem: ui/acts
tags: [svelte, modal, confirm, gate, acts]
requires: []
provides:
  - "ActDeleteModal: собственное окно подтверждения удаления акта вместо нативного диалога"
  - "check-act-delete-confirm.mjs: структурный гейт в pnpm lint"
affects: [ui/src/features/acts, ui/scripts, ui/package.json]
tech-stack:
  added: []
  patterns: ["презентационная модалка + родитель владеет deleting-флагом", "гейт со стеком {#if} и мутантами-подменами"]
key-files:
  created:
    - ui/src/features/acts/ActDeleteModal.svelte
    - ui/scripts/check-act-delete-confirm.mjs
  modified:
    - ui/src/features/acts/ActsPage.svelte
    - ui/package.json
    - ui/scripts/check-group-vocabulary.mjs
key-decisions:
  - "handleDelete остался функцией пост-подтверждения (на неё опирается check-act-delete-notices); подтверждение вынесено в requestDelete -> ActDeleteModal -> confirmDelete"
  - "Маркеры allowlist словаря групп переехали вместе с предложением из ActsPage в ActDeleteModal"
requirements-completed: [QUICK-261028-fw6]
duration: ~25 мин
completed: 2026-10-28
---

# Quick 261028-fw6: подтверждение удаления акта в собственном окне

Нативный `window.confirm` в `ActsPage.handleDelete` заменён окном `ActDeleteModal`, которое рисуется в DOM, так что подтверждение и текст последствий видны и в Tauri webview, и в LAN-браузере. Rust, DTO и `showRollbackNotices` не затронуты.

## Коммиты

- `87dfefa8` fix: ActDeleteModal + подключение в ActsPage
- `f4f25a9f` test: гейт check-act-delete-confirm.mjs + запись в цепочку lint
- `f4d2ff5c` fix: перенос маркеров allowlist словаря групп (Rule 3, см. ниже)

## Что сделано

- `ActDeleteModal.svelte`: презентационный (`act`, `deleting`, `onCancel`, `onConfirm`), не импортирует api и не вызывает `acts.delete`. Заголовок и обе ветки тела (акт / акт возврата) скопированы дословно, включая слово «parent» и фразу про возврат устройств в группы. `initialFocus=".modal-footer .btn-secondary"`. Пока `deleting`: обе кнопки `disabled`, «Удалить» в `loading`, `onClose` ничего не делает (Esc, фон и крестик не закрывают).
- `ActsPage.svelte`: `requestDelete` / `cancelDelete` / `confirmDelete`; `ActDetail` получает `onDelete={requestDelete}`; окно смонтировано на верхнем уровне разметки после `PdfPreviewModal`, под `{#if deleteTargetAct}`. `handleDelete` потерял только heading/body/`window.confirm`; вся пост-логика (тосты, `notifyPlaceContentChanged`, `showRollbackNotices`, `selectedActId = null`, `refresh`, `refreshCounts`, тост ошибки) не менялась. Окно закрывается и при успехе, и при ошибке (`finally`).
- Гейт `check-act-delete-confirm.mjs`: правила A (нет нативного `confirm(` в `src/features/acts/`), B (импорт, монтирование под ближайшим `{#if deleteTargetAct}` по стеку, `handleDelete` не в разметке, `confirmDelete` зовёт `handleDelete(`, `onDelete={requestDelete}`), C (каждое из двух предложений про группы ровно по одному разу), D (окно презентационное), E (initialFocus), M (отсутствие файлов или пустой каталог = нарушение). Включён в `lint` сразу после `check-act-delete-notices` (`--selftest` и обычный запуск).

## Проверка утверждения про стили таблицы (предупреждение 8)

Утверждение плана подтверждено, а не принято на веру. `Modal.svelte` (строки 261-266) сбрасывает на корне `.modal-backdrop`: `white-space: normal; text-align: start; cursor: auto`. В `ActDeleteModal.svelte` нет `nowrap` / `center` (grep exit=1). Окно к тому же смонтировано вне таблицы. Гейт `check-modal-isolation.mjs` реально проверяет эти три объявления (его selftest гоняет 10 мутантов реального `Modal.svelte`).

## Команды, которые реально запускались

Дословные хвосты:

- `pnpm --dir ui lint` -> `lint-exit=0`; хвост: `[check-act-delete-confirm] [selftest] PASS — реальные исходники чисты, мутантов поймано: 15.` / `[check-act-delete-confirm] PASS — 0 нарушений` / `[check-store-response-shape] SELFTEST PASS — 25 кейсов` / `[check-store-response-shape] PASS — 0 нарушений (45 файлов, 2 сторов реестра, 2 кандидатов)`. Это последний гейт цепочки, то есть вся цепочка `&&` прошла до конца, а не оборвалась перед своим или после своего шага. Первый прогон был красным (см. Deviations).
- `pnpm --dir ui svelte-check` -> `svelte-check-exit=0`; хвост: `COMPLETED 319 FILES 0 ERRORS 72 WARNINGS 18 FILES_WITH_PROBLEMS`; предупреждений по `ActDeleteModal` / `ActsPage` нет (grep count 0). Скрипт в `package.json` называется `svelte-check`; команды `check` нет (план называл `pnpm --dir ui check`, она падает с `Command "ui" not found`).
- `pnpm --dir ui build` -> `build-exit=0`; prebuild `cargo test -p trackly-app --test export_bindings`: `test result: ok. 1 passed; 0 failed`; хвост: `✓ built in 3.25s` (предупреждение vite о размере чанков уже существовало).
- `node scripts/check-act-delete-notices.mjs` -> `PASS — 0 нарушений`; `node scripts/check-modal-isolation.mjs` -> `PASS — 0 нарушений` (selftest: `10 мутантов исходника красные`).
- `node scripts/check-group-vocabulary.mjs --selftest` -> `PASS — все 49 фикстур совпали с ожиданием.` (exit 0); без флага -> `OK — проверено файлов: 222, нарушений нет.`
- `node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256 <файлы>` перед каждым из трёх коммитов и перед финальным заходом -> `PASS — 0 нарушений`.

## Мутационная проверка

Selftest гейта (на реальных исходниках через `mutate()` с проверкой ровно одного вхождения якоря), 14 мутантов + пустой каталог = 15, все красные нужным правилом: (1) `window.confirm` в `handleDelete` -> A; (2) голый `confirm(` в `ActDetail` -> A; (3) тег заменён -> B; (4) снят открывающий `{#if deleteTargetAct}` -> B; (5) подмена `onDelete={requestDelete}` на `handleDelete` -> B; (6), (7) стёрты предложения веток -> C; (8) подмена предложения «актом» предложением «возвратом» (общий счёт фразы не меняется) -> C; (9) импорт api в модалке -> D; (10) снят `initialFocus` -> E; (11), (14) отсутствие файла -> M; (12) подмена: `confirmDelete` зовёт `acts.delete(` мимо `handleDelete` -> B; (13) подмена: окно под чужим `{#if pdfModalOpen}` -> B; пустой каталог -> M.

Мутации на копии каталога через `--src` (копия `src/features/acts` во временном каталоге сессии; для каждой проверено `diff -r` с оригиналом и уникальность якоря в скрипте):

| Мутация | Результат |
|---|---|
| чистая копия | exit 0 |
| (a) `window.confirm` в `handleDelete` | exit 1, правило A |
| (b) убрано монтирование окна из разметки | exit 1, правило B |
| (c) убрано предложение ветки «актом» | exit 1, правило C |
| (d) убрано предложение ветки «возвратом» | exit 1, правило C |

Восстановление: мутации применялись только к копиям, реальные файлы не трогались. Доказательство: `git diff --exit-code -- src scripts package.json` -> exit 0 после всех мутаций. Контрольные суммы реальных файлов (сняты после мутаций, равны содержимому HEAD):

- `baad82350d129576af910d74d3280874cd92f64e1507b614580216b22040e6fa` ActDeleteModal.svelte
- `3d69198e443ba2393c3f3bf0ef7ee9fa9d9c7b2d509bab2dd82228c3d7fce3e3` ActsPage.svelte
- `8e3804c915105991c238f749a162a07bcd69377991e61125e02cb0b86a09cfa9` ActDetail.svelte
- `65d6e19eb18d90d31a11edff388129c1f5856f4888df7c228a9bd070d09990c6` check-act-delete-confirm.mjs

Честная оговорка: первая попытка сравнения «до/после» через `shasum $FILES` в zsh не разбила переменную на слова, обе контрольные суммы оказались пустыми, а строка «IDENTICAL» была вакуумной. Она не считается доказательством; доказательство выше (`git diff --exit-code` + суммы, снятые заново).

## Deviations from Plan

**1. [Rule 3 - Blocking] check-group-vocabulary краснел на перенесённых строках**
- **Found during:** Task 3, первый `pnpm --dir ui lint` (exit 1, цепочка оборвалась на этом гейте; ActDeleteModal.svelte:30-31).
- **Issue:** слово «группа» в `ActDeleteModal.svelte` не покрыто allowlist словаря групп (разрешён был только `ActsPage.svelte`).
- **Fix:** маркеры `из состава групп` и `вернутся в свои группы` убраны из записи `ActsPage` (она сужена) и вынесены в новую запись `ActDeleteModal`; обновлены фикстуры selftest: позитив для модалки, узкий негатив для модалки, негатив «предложение вернулось в ActsPage», правка существующего негатива `ActsPage` (раньше опирался на снятый маркер).
- **Files modified:** `ui/scripts/check-group-vocabulary.mjs`
- **Commit:** `f4d2ff5c`

**2. [Уточнение плана] команда svelte-check**: в плане `pnpm --dir ui check`; фактически скрипт называется `svelte-check`. Запущен он.

**3. Расширенный selftest**: добавлены сверх плана мутанты (12), (13), (14) (подмены для порядка/принадлежности, не только удаление).

## Known Stubs

Нет.

## Threat Flags

Нет новой поверхности: серверная часть, права и `acts.delete(id, version)` не менялись.

## UNVERIFIED (не доказано прогоном)

Компиляционные и структурные гейты не доказывают поведения в живом приложении. Статус UNVERIFIED до живой проверки в десктопном webview (ожидается на Windows пользователем):

- окно действительно открывается по «Удалить» в карточке акта и в Tauri webview, и в LAN-браузере (до этого нативный диалог молча пропускался);
- начальный фокус на «Отмена» (`initialFocus` селектор `.modal-footer .btn-secondary` совпадает с классом кнопки `secondary` по образцу `GroupReleaseConfirmPopup`, но фактическое попадание фокуса не проверялось);
- Esc и «Отмена» закрывают окно без удаления;
- во время запроса окно не закрывается Esc / фоном / крестиком, кнопки заблокированы, «Удалить» показывает loading;
- текст визуально переносится и выровнен по левому краю (подтверждено только чтением CSS и `check-modal-isolation`, не рендером);
- серверная часть не проверялась и не менялась.

## Self-Check: PASSED

- ActDeleteModal.svelte, check-act-delete-confirm.mjs: существуют (`git log` показывает коммиты `87dfefa8`, `f4f25a9f`, `f4d2ff5c`).
- Нативного `confirm(` в `ui/src/features/acts/` нет (правило A гейта на реальных файлах зелёное).
