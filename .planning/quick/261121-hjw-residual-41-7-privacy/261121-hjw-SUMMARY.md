---
phase: quick-261121-hjw
plan: 01
status: complete
subsystem: gates, auth, attachments, privacy, ci
tags: [WR-04, WR-05, WR-07, WR-08, WR-09, privacy-gate, 41.3-security]
requires: [261121-hed, 261121-hef, 261121-heg, 261121-heh, 261121-hei]
provides:
  - Verdict::ExemptWrites (exw) + Kind::ExemptOwnsWriter / ExemptWritesWithoutWrite
  - peer в аудите custom:bootstrap
  - W6 (check-ws-disconnect-toast), INV-11 (check-place-tree-invalidation), G15 (check-attachments-block)
  - межстрочный проход, cp1251/UTF-16, PNG-текст, decomposeLongValue в check-privacy
affects: [entities_changed_gate, auth_bootstrap, ws.ts, check-privacy, release.yml, heic-spike.yml]
tech-stack:
  added: []
  patterns: [мутация копии реестра, мутация дореформенного исходника из BASE, проверка перепроверки внутри писателя]
key-files:
  created:
    - .planning/todos/pending/2026-11-21-wr02-multi-execute-transactional.md
    - scripts/fixtures/privacy/wrapped-marker.md
    - scripts/fixtures/privacy/paragraph-break.md
  modified:
    - crates/trackly-app/tests/entities_support/registry.rs
    - crates/trackly-app/tests/entities_support/scan.rs
    - crates/trackly-app/tests/entities_changed_gate.rs
    - crates/trackly-app/src/services/auth.rs
    - crates/trackly-app/src/http/auth.rs
    - crates/trackly-app/src/tauri_cmds/auth.rs
    - crates/trackly-app/src/services/attachment_admin.rs
    - crates/trackly-app/src/services/cartridge_service.rs
    - crates/trackly-app/src/services/device_service.rs
    - crates/trackly-app/tests/auth_bootstrap.rs
    - crates/trackly-app/tests/desktop_system_admin.rs
    - crates/trackly-app/tests/http_route_parity.rs
    - crates/trackly-app/tests/attachments_integrity.rs
    - crates/trackly-app/tests/backup_service.rs
    - ui/src/lib/api/ws.ts
    - ui/src/features/attachments/AttachmentViewer.svelte
    - ui/scripts/check-ws-disconnect-toast.mjs
    - ui/scripts/check-place-tree-invalidation.mjs
    - ui/scripts/check-attachments-block.mjs
    - scripts/check-privacy.mjs
    - scripts/check-privacy.selftest.mjs
    - scripts/fixtures/privacy/README.md
    - .github/workflows/release.yml
    - .github/workflows/heic-spike.yml
    - .planning/REQUIREMENTS.md
    - .planning/ROADMAP.md
    - .planning/PROJECT.md
    - .planning/phases/41.3-attachments-storage/41.3-SECURITY.md
decisions:
  - "exw — отдельный громкий вердикт для пишущих освобождённых функций; 24 строки реестра переведены без изменения причин"
  - "INV-10 гейта place-tree уже занят (41.12) — правило WR-09 получило номер INV-11"
  - "vcpkg закреплён коммитом master d7b4974a, а не релизным тегом 2026.10.16: в теге нет фичи libheif[libde265]"
  - "iframe PDF без sandbox — принятый риск R-07: Chrome и Edge не рендерят PDF в sandbox-iframe"
  - "STATE.md не правился (запрет оркестратора) — INT-02 в STATE.md остаётся до следующей правки STATE"
metrics:
  completed: 2026-11-21
  tasks: 4
  commits: 10
---

# Quick 261121-hjw: остаток ревью 41.7 и гейт приватности — итог

Пакет закрыл оба просроченных todo вехи v1.4 и наблюдения 4, 5, 7, 8 из 41.3-SECURITY.
В реестровом гейте появился громкий вердикт `exw`. Аудит bootstrap теперь хранит IP пира.
Таймер переподключения WS можно отменить. Гейт приватности видит значения, разорванные
переносом строки, тексты в cp1251/UTF-16 и метаданные PNG. Скан целостности перепроверяет
ссылки внутри писателя. vcpkg закреплён коммитом.

**BASE = `7438974565f086605f276cd6c10028e461cb43c0`** (HEAD на старте; по нему шли мутации
ws.ts и check-privacy.mjs).

## Коммиты

| # | Задача | Коммит | Что |
|---|--------|--------|-----|
| 1 | 1 | 8bc33431 | WR-07: `Verdict::ExemptWrites`/`exw`, `ExemptOwnsWriter`, `ExemptWritesWithoutWrite`, свип |
| 2 | 1 | 21355154 | WR-05 peer в аудите, WR-08 /apiary, IN-01..03 |
| 3 | 2 | c97e3493 | WR-04 reconnectTimer + W6/M13-M15, WR-09 INV-11 |
| 4 | 2 | b7f6ee04 | INT-02 → PAG-01, правило INT-NN, todo хвоста WR-02 |
| 5 | 3 | 3ae83bfb | гейт приватности: перенос, кодировки, PNG, класс D, decomposeLongValue |
| 6 | 3 | fcafa311 | оба todo → completed с resolution |
| 7 | 4 | 030e3c3b | набл. 5 (перепроверка в писателе) + набл. 8 (тест симлинков) |
| 8 | 4 | 086649cf | набл. 4: vcpkg закреплён |
| 9 | 4 | bc23aeb1 | набл. 7: принятый риск, маркер, G15 |
| 10 | 4 | 1fb7c7ce | 41.3-SECURITY.md: отметки по наблюдениям 3, 4, 5, 7, 8, R-07 |

## Задача 1 — Rust

- **WR-07.** Новые правила гейта загорелись на **24 строках**. Все они переведены с `ex` на
  `exw`, текст причин не менялся:
  act_service.rs::{delete_soft, do_return, update_return, update_return_group_split_preview};
  cartridge_service.rs::{model_create, model_delete, model_update};
  device_service.rs::{bulk_create, create, create_single_with_number_check, insert_new_and_get};
  group_type_service.rs::{create_property, create_type, delete_property, delete_type,
  reorder_properties, seed_builtin_types_on_startup, unarchive_property, update_property, update_type};
  place_service.rs::set_archived; repair_service.rs::{create, link_scans, merge_into_open}.
  Свип `mutation_exw_to_ex_in_every_writing_exempt_row_turns_gate_red` подменяет каждую
  строку `exw` на `ex` в копии реестра. Все 24 строки дают `ExemptOwnsWriter`, счётчик
  больше нуля. `gate_new_checks_measured_something` проверяет, что `exempt_writes_checked`
  совпадает с числом строк `exw`. Восемь синтетических selftest-ов, которые раньше держали
  `Exempt` на пишущих функциях, переведены на `ExemptWrites`.
- **WR-05.** Сигнатура стала `bootstrap_first_admin(new, peer)`. `payload_json` теперь
  `{"login", "peer"}`: на LAN туда идёт `ConnectInfo.ip()`, у Tauri — `null`. Формат у
  `create_user` прежний. Потребителей payload аудита пользователей нет: проверено грепом
  Rust и UI. Тест утверждает, что для `203.0.113.7` в payload лежит именно этот IP,
  а на пути Tauri — `null`.
- **WR-08.** Проба `/apiary` вынесена из `if ui_bundled`. **IN-01:** `transition` рассылает
  событие до read-back. **IN-02:** `TempDir` держится в кортеже. **IN-03:** добавлены
  комментарии в device_service и cartridge_service.

## Задача 2 — UI-гейты и документы

- **W6, мутационная проверка.** На дореформенном `ws.ts` из BASE
  (`--file-ws=<scratchpad>/ws-base.ts`) гейт красный, exit 1, все четыре проверки W6.
  На HEAD зелёный. Мутанты M13–M15 в selftest красные, и на реальном ws.ts тоже:
  без `clearTimeout` — exit 1, без гарда `refCount` в колбэке — exit 1.
- **INV-11** (WR-09) красный на трёх мутантах реальных файлов в памяти: вызов
  `reloadSeqNow()` удалён; вызов обёрнут в `untrack`; функция возвращает `.seq`.
  Selftest — 67 кейсов, PASS.
- **INT-NN.** В REQUIREMENTS, ROADMAP и PROJECT требование переименовано в PAG-01,
  в REQUIREMENTS записано правило пространства имён. Требований с префиксом INT в
  REQUIREMENTS не осталось. `gsd-sdk query state.load` отрабатывает.
- **Хвост WR-02.** Создан `.planning/todos/pending/2026-11-21-wr02-multi-execute-transactional.md`.
  Прямой случай один: `printer_service::poll_single` (2 вызова). Транзитивные:
  `request_service::transition`, `auth::on_ad_bind_success`, `auth::force_admin_provisioning`.

## Задача 3 — гейт приватности (мутационные прогоны)

| Прогон | Результат |
|--------|-----------|
| (а) дореформенный гейт из BASE, wrapped-marker.md | exit 0 (фикстура несущая) |
| (а) новый гейт, wrapped-marker.md | exit 1 (оба случая: проза, строка 3; `//`, строка 7) |
| (б) копия без `...extractCrossLineNgrams(content)` в scanText | exit 0 |
| (в) временный git-репозиторий, `--staged`, «Пчёлкин / Артём» через перенос | exit 1 |
| (в) тот же репозиторий, staged-файл в cp1251 | exit 1 |
| HEAD с продовыми хэшами (путь CI) | PASS, 0 нарушений |
| paragraph-break.md / without-marker.md / with-marker.md | 0 / 0 / 1 |

- Расширенный класс D: `git ls-files` по новым расширениям пуст. По 15 закоммиченным PNG
  текстовых чанков нет. Оба cp1251-CSV из тестовых фикстур теперь декодируются: 215
  кириллических символов, U+FFFD нет.
- **На продовых хэшах HEAD новый проход ничего не нашёл.** Обезличивать ничего не пришлось.
- **Ручная разведка** `.planning/{todos,quick,debug,threads,milestones}`: 178 файлов,
  3 кандидата по эвристике «Фамилия И.О.» / «Фамилия Имя Отчество» (известные вымышленные
  исключены). Значения записаны только в
  `/private/tmp/claude-501/-Users-madsas-Projects-trackly/4702bdb3-19d7-4069-aae6-342fcc32e777/scratchpad/recon.txt`,
  вне репозитория. Решить и занести токены через `--add` должен пользователь.

## Задача 4 — 41.3-SECURITY

- **Набл. 5.** Тест `stale_orphan_candidate_with_new_row_is_kept` проходит. Если убрать
  перепроверку (`if false && ...`), он красный: «блоб, на который появилась строка, удалён».
- **Набл. 8.** Тест `attachment_copy_skips_symlinked_shard_and_file` проходит. Если заменить
  `symlink_metadata` на `metadata`, он красный: «симлинк-шард скопирован в приёмник».
- **Набл. 4.** vcpkg закреплён коммитом `d7b4974a4199586fda8201d74f80cacaaa27368d`.
  YAML обоих workflow валиден (проверено ruby YAML; PyYAML и actionlint не установлены).
  **UNVERIFIED на раннере** — проверится ближайшим релизным прогоном.
- **Набл. 7, замер 2026-11-21.** Стенд: python http.server, `application/pdf` + `nosniff`,
  вымышленный одностраничный PDF. Chrome и Edge показывают PDF без sandbox и заглушку
  блокировки при `sandbox=""`, `allow-scripts` и `allow-same-origin`. Playwright WebKit
  не рендерит PDF в iframe даже без sandbox, так что для WKWebView замер ничего не говорит.
  Исход — **принятый риск R-07**: маркер у iframe плюс гейт G15. G15 красный, если убрать
  маркер или увести его от iframe; контроль «sandbox без маркера» зелёный.

## Тесты и гейты

- cargo test, полные suite-бинарники: app_gates 259/0 (1 ignored), app_attachments 78/0,
  app_server 49/0, `--lib` 404/0, app_auth 137/0, app_cartridges 88/0.
  Переменные `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1`, ui/dist собран.
- `cargo clippy -p trackly-app --all-targets -- -D warnings`: 0. `cargo fmt --all -- --check`: 0.
- `pnpm --dir ui lint`: 0. `pnpm --dir ui svelte-check`: 0 ошибок (75 предупреждений,
  все были до этого пакета).
- `node scripts/check-privacy.selftest.mjs`: PASS. `check-privacy.mjs` по HEAD: PASS.

## Deviations from Plan

1. **[Оркестратор] STATE.md не трогался.** План требовал переименовать INT-02 в STATE.md
   (~стр. 1805), но оркестратор это запретил. Строка остаётся под старым именем.
2. **[Rule 3] INV-10 → INV-11.** Номер INV-10 в check-place-tree-invalidation уже занят
   инвентарём 41.12 (WR-03).
3. **[Rule 1] vcpkg: коммит вместо релизного тега.** Последний релиз 2026.10.16
   (тег → коммит 9eec002d…) несёт libheif 1.23.5 без фичи `libde265`, и
   `libheif[core,libde265]` на нём не ставится. Закреплён коммит master `d7b4974a…`:
   libheif 1.23.7, libde265 1.1.3 — те же версии, что в последнем зелёном релизе 2026-11-20.
   В комментарии записаны тег, коммит тега и выбранный SHA.
4. **[Rule 2] PNG tEXt/zTXt.** Текст декодируется через `decodeBuffer` (строгий UTF-8,
   иначе cp1251), а не как чистый latin1. Кириллица в latin1 непредставима, а редакторы
   кладут туда UTF-8.
5. **[Rule 2] Бинарный буфер (есть NUL) без UTF-16 BOM** декодируется как раньше
   (нестрогий UTF-8), а не через cp1251. Иначе бинарный мусор превращался бы в «кириллицу».
6. В плане команда `pnpm --dir ui check`, такого скрипта нет. Запущен `pnpm --dir ui svelte-check`.

## Human-check (для пользователя, исполнитель НЕ отмечал)

1. **Предпросмотр PDF-вложения** с вымышленным PDF: файл открывается и листается.
   Проверить (а) десктоп macOS (`cargo tauri dev`, WKWebView); (б) десктоп Windows
   (WebView2, сборка с этим коммитом); (в) LAN-браузер Chrome/Edge после
   `pnpm --dir ui build`. Атрибут sandbox не ставился, поэтому ожидается прежнее поведение.
2. **Живая UAT остановки сервера** при создании акта по LAN. Вынесена в новый todo, в этом
   пакете не выполнялась.

## Не проверено

- vcpkg-шаг на Windows-раннере: живой прогон создал бы черновик релиза.
- Отображение PDF в WKWebView: замер Playwright WebKit для этого непригоден.
- Ветка `--add` в TTY (интерактивная, с подтверждением y/N) не запускалась. Разложение
  проверено selftest-ом через экспорт `decomposeLongValue`.
- Сплошной прогон всех suite-бинарников не делался. Прогнаны затронутые: app_gates,
  app_attachments, app_server, `--lib`, app_auth, app_cartridges.

## Self-Check: PASSED

Все 10 коммитов есть в `git log 74389745..HEAD`. Созданные файлы на месте: todo хвоста WR-02,
wrapped-marker.md, paragraph-break.md. Оба todo перенесены в `todos/completed/`.
