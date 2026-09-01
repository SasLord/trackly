# Phase 40: История перемещений - Research

**Researched:** 2026-09-01
**Domain:** Codebase integration — new append-only history table wired into 6+ existing
Rust write sites (devices/cartridges/acts), a 13th report, and 3 existing Svelte detail
surfaces. No new external dependencies.
**Confidence:** HIGH (this is a codebase-integration phase; every claim below is grounded in
direct reads of the files named, not framework/library research)

## Summary

Phase 40 adds one new table (`place_movements`, migration `V040`) and wires it into every
Rust write site that currently mutates `devices.place_id` or `cartridges.place_id`, so that
each real place→place change produces exactly one movement row in the same transaction as the
mutation. There are **six** such write sites today (three device, three cartridge — enumerated
in `## Architecture Patterns` below), all inside `act_service.rs`, `device_service.rs`, and
`cartridge_service.rs`. `printer_service.rs` mutates no `place_id` at all (a printer's place
lives in its `devices` row, per D-21) so it is not a write site.

The single most important finding of this research, and the one fact that should shape the
plan's task ordering, is: **`device_service.update`, `cartridge_service.update/transition`, and
every `ActService` mutation method hard-code `user_id_opt: Option<i64> = None` today** — none of
them accept a `caller: &Identity` parameter, even though the Tauri/HTTP adapter layer above them
already resolves and authorizes a real `Identity` before calling in
(`crates/trackly-app/src/tauri_cmds/devices.rs:57-66`: `caller` is used for `authorize()` then
silently dropped before `ctx.devices.update(id, version, patch)`). D-09 requires a `user_id` +
ФИО snapshot on every movement row, and D-01 requires the movement insert to share the mutation's
transaction — together these force every one of the six write-site methods to gain a
`caller: &Identity` (or equivalent `user_id: Option<i64>`) parameter, mirroring the pattern
`place_service.rs` already uses (its methods have taken `caller: &Identity` since Phase 39). This
is real signature-surgery across three service files and all their call sites in both
`tauri_cmds/*.rs` and `http/*.rs` — size this as its own wave, not a footnote on the schema task.

Beyond that, everything else in this phase is pattern-cloning: the report is the 13th `list_*`
in `report_service.rs`, the timeline is a third consumer of the same shortened-path snapshot
formula already extracted in Phase 39.2 (`place_path_settings`), and the read-side history
section clones `cartridge_service.get_history`'s service→repo→DTO→UI shape wholesale.

**Primary recommendation:** Build `place_movements` as its own append-only table (mirroring
`audit_log`'s hard-delete, no-`deleted_at_utc` shape) with `act_id`/`place_id`/`path_snapshot`
columns per side, thread `caller: &Identity` into the six existing write-site methods to capture
`user_id` + a `users.full_name` snapshot read inside the same transaction, and reuse
`compute_place_path_short`'s resolution algorithm (promoted out of `act_service.rs` into a
shared location) for rendering both the timeline and the new report's PDF/print output.

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| HST-01 | Каждая смена места устройства или картриджа записывается в историю: откуда, куда, когда, кем и по какой причине | `## Architecture Patterns` enumerates all 6 write sites; `## Common Pitfalls` #1 covers the `caller: &Identity` threading needed for "кем"; migration shape in `## Standard Stack` |
| HST-02 | Пользователь видит таймлайн перемещений в карточке устройства и картриджа | `## Architecture Patterns` (PlaceEntityViewModal/CartridgeDetail/PrinterDetail three-consumer pattern); `## Code Examples` (get_history clone) |
| HST-03 | Акт приёма-передачи автоматически меняет место переданных устройств и фиксирует в истории ссылку на номер акта | `act_service.rs` write sites (create/update/do_return/update_return) all already carry `act_id` in scope; `## Common Pitfalls` #3 (undo/LIFO delete) |
| HST-04 | Пользователь может получить отчёт о перемещениях за период с фильтром по месту и типу устройства | `## Architecture Patterns` (report clone: `report_service.rs`, `ReportFilter`, `columns_for`/`column_labels_for`, `ReportSubNav.svelte`) |
</phase_requirements>

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01 (Отдельная таблица, не выборка из `audit_log`):** история перемещений живёт в
  собственной таблице (рабочее имя `place_movements`) со структурированными колонками:
  тип и id предмета, `from_place_id` / `to_place_id`, снимки путей обеих сторон, `user_id`,
  снимок ФИО, источник, комментарий, `act_id`, момент времени. Запись создаётся **в той же
  транзакции**, что и сама мутация места — через того же единственного писателя. `audit_log`
  остаётся как есть.
- **D-02 (Без ретроспективы):** миграция НЕ бэкфиллит историю из `audit_log`. История
  начинается с момента обновления.
- **D-03 (Отмена акта стирает его записи):** `undo_device_mutations_for_act` уже выполняет
  настоящий undo — место откатывается. Перемещения фактически не было: его записи из истории
  **удаляются**, а не помечаются отменёнными. Никакой компенсирующей записи не создаётся.
- **D-04 (Только смена `place_id`):** смена статуса без смены места в историю НЕ пишется.
- **D-05 (Операции картриджа — только через место):** install/return/refill сами по себе не
  события истории. Но если операция меняет `place_id` картриджа — запись появляется
  автоматически, с осмысленной причиной.
- **D-06 (Только место→место):** запись создаётся, когда обе стороны — реальные узлы дерева.
  NULL → место (первичное заполнение) не пишется; место → NULL (снятие) тоже не пишется.
  Следствие: предмет со снятым местом остаётся в истории на последнем известном месте.
- **D-07 (Закрытый enum источника + свободный комментарий):** `source` — enum из четырёх
  значений: `manual` / `act` / `map` / `workstation`. Все четыре заводятся сразу. `note` —
  свободный текст, необязательный.
- **D-08 (Комментарий необязателен):** поле есть, пустое значение допустимо.
- **D-09 (Двойной снимок автора — `user_id` И ФИО):** запись хранит `user_id` **и** текстовый
  снимок ФИО на момент перемещения (логины `usXXX` переиспользуются — резолв по `user_id`
  задним числом присвоил бы перемещение другому человеку). ⚠ Приватность: фикстуры/тесты —
  только вымышленные имена; `scripts/check-privacy.mjs` обязан остаться зелёным.
- **D-10 (Двойной снимок места — `place_id` И путь):** ровно как акт по D-16 Фазы 39. Обе
  стороны хранят id узла и текстовый снимок полного пути.
- **D-11 (Отображение «кем»):** ФИО, при отсутствии — логин; `user_id IS NULL` → «система».
- **D-12 (Доступ — Admin + Manager):** чтение истории и отчёта. Employee не видит ни того ни
  другого. Гейт на бэкенде, на обоих транспортах. `Action::ReadPlaces` уже даёт ровно
  Admin+Manager.
- **D-13 (Права на мутацию не меняются):** фаза не трогает, кто может менять место устройства
  или картриджа. Только добавляет новое чтение.
- **D-14 (Устройство — расширяем существующую модалку):** у устройств нет детальной панели —
  расширяется `PlaceEntityViewModal.svelte` («Просмотр устройства»): добавляется секция
  истории, модалка открывается **из списка устройств** тоже.
- **D-15 (Объём карточки — минимум):** read-only поля + секция «История перемещений». Ни
  списка актов, ни новых действий.
- **D-16 (Картридж — ДВЕ секции, ничего не теряем):** существующая секция «История
  перемещений» (`CartridgeDetail.svelte:192`) переименовывается в «Журнал операций» и
  остаётся; рядом появляется новая секция «Перемещения» на новом таймлайне.
- **D-17 (Путь в строке — сокращённый + полный в tooltip):** по D-26 Фазы 39 и настройкам
  организации (PLC-07/PLC-08).
- **D-18 (Сокращается ХРАНИМЫЙ снимок, не живой путь):** формула сокращения применяется к
  снимку из записи, а не к текущему пути узла.
- **D-19 (Кликабельны и место, и номер акта):** место → раздел «Места» с фокусом на узле,
  номер акта → карточка акта.
- **D-20 (Порядок — новые сверху, весь список сразу):** `ORDER BY` по времени DESC, без
  пагинации.
- **D-21 (Принтер — одна запись как `device`):** перемещение пишется один раз с типом
  предмета `device`. Отдельной ветки `printer` НЕ заводится. `PrinterDetail.svelte` показывает
  тот же таймлайн, читая по id устройства.
- **D-22 (Своя группа «Перемещения» в `ReportSubNav`):** один отчёт на обе сущности, тип
  предмета разделяется фильтром.
- **D-23 (Колонки):** Дата · Предмет · Тип · Откуда · Куда · Кем · Причина.
- **D-24 (Два отдельных фильтра места — «Откуда» и «Куда»):** оба — subtree-inclusive.
  `ReportFilter` расширяется двумя новыми полями места.
- **D-25 (Мягко удалённые предметы остаются, с пометкой):** отчёт не меняется от списания —
  строка остаётся, рядом пометка «удалено».
- **D-26 (Экспорт — CSV и PDF, как у остальных 12 отчётов):** паритет с существующими
  `export_csv`/`export_pdf`. ФИО в выгрузку попадает.
- **D-27 (Поток не меняется — правка в форме редактирования):** отдельного действия
  «Переместить…» НЕ вводится. Поменял `PlacePicker` → сохранил → запись с источником
  `manual` и пустым комментарием.
- **D-28 (Массовый перенос — через панель содержимого места):** на `PlaceContents.svelte`
  появляется действие «Перенести всё содержимое в…». Пишет по одной записи истории на
  каждый перенесённый предмет, одной транзакцией.
- **D-29 (Без WebSocket):** таймлайн грузится при открытии карточки и после сохранения.
  Отчёт — по кнопке.

### Claude's Discretion

- Имена таблицы и колонок (`place_movements`, `from_place_id`, `source`, `note` — рабочие
  имена, не обязательство).
- Форма хранения `source` — TEXT-токен как у `path_variant` или отдельный справочник;
  ключевое ограничение — неизвестный токен в БД не должен ронять экран целиком (урок IN-01
  Фазы 39.2).
- Как именно D-03 удаляет записи при отмене акта — по `act_id` внутри той же транзакции undo,
  или отдельным шагом; требование — атомарность с undo.
- Точная семантика двух фильтров места (D-24) при заполнении обоих — AND vs OR; рекомендация —
  AND, но подтвердить по формулировке HST-04.
- Где живёт запрос отчёта — новый `list_*` в `report_service.rs` рядом с двенадцатью
  существующими или отдельный модуль.
- Механика «одного владельца» для форматирования строки таймлайна — Rust или фронт.

### Deferred Ideas (OUT OF SCOPE)

- Массовый перенос выбором чекбоксами в списке устройств — отклонён в пользу D-28.
- Employee видит историю своего устройства — отклонено (D-12).
- Отдельное действие «Переместить…» с обязательной причиной — отклонено (D-27/D-08).
- Живое обновление таймлайна по WebSocket — отклонено (D-29). Кандидат Фазы 45.
- Ретенция и чистка журналов — фаза 40 её не вводит.
- Починка числового `place_id` в оставшемся «Журнале операций» картриджа — отдельная
  мелкая задача, вне фазы.
- АРМ, карта, редактор планов, живые статусы — Фазы 41 и 43–45. Здесь только источники
  `map`/`workstation` в enum, ни строки логики.
</user_constraints>

## Project Constraints (from CLAUDE.md)

- Repo is **public** — no real org data, requisites, or real ФИО anywhere (code, fixtures,
  tests, `.planning/` artifacts). D-09 introduces ФИО into a NEW table for the first time in
  this phase's scope — every fixture/test/example MUST use invented names
  («Иванов И.И.», «Петров П.П.»). `scripts/check-privacy.mjs` runs pre-commit and in
  `ci-fast.yml` — it must stay green.
- Stack fixed: Rust + Tauri 2 + Svelte 5 (runes) + SCSS + SQLite (WAL). No new dependencies are
  needed for this phase — confirmed no external package research required.
- SQLite single-writer discipline: all writes go through `WriterHandle::execute` with one
  `rusqlite::Connection`; the movement insert MUST happen inside the same
  `tx: rusqlite::Transaction` as the place mutation it records (see `sqlx` prohibition —
  irrelevant here since the project already uses `rusqlite`, but the "one write, one
  transaction, no partial state" principle is the same one WR-05 (Phase 39.2) fixed for
  path-display settings and this phase must not regress).
- Portable-mode / Windows-target constraints are not implicated by this phase (no new files,
  no new external processes, no new paths).
- GSD workflow enforcement: this research feeds `/gsd-plan-phase`; no direct edits are made
  here.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Movement row creation (device/cartridge place changes) | API / Backend (service layer, same tx as mutation) | Database (`place_movements` table) | D-01 requires atomicity with the mutation; must live in the single-writer transaction, not a follow-up call |
| Movement row deletion on act undo | API / Backend (`act_service::delete_soft` / `undo_device_mutations_for_act`) | Database | D-03 — deletion must be atomic with the undo transaction |
| Actor (user_id + ФИО) resolution | API / Backend (inside the write-site transaction, `SELECT full_name FROM users WHERE id=?`) | — | D-09 — snapshot must be taken at write time, not resolved later at read time (login reuse hazard) |
| Path shortening for display | API / Backend (`compute_place_path_short`-style function, shared) | — | D-18 — must operate on the frozen snapshot, never the live path; existing Rust formula is the single source of truth (Phase 39.2 lesson) |
| Timeline rendering (device/cartridge/printer cards) | Browser / Client (Svelte) | API / Backend (DTO fields pre-formatted) | Frontend renders already-shortened strings; no JS mirror of the shortening formula (avoids repeating the WR-03 placePath.ts drift bug) |
| Report query + CSV/PDF export | API / Backend (`report_service.rs`, 13th `list_*` + `export_csv`/`export_pdf`) | — | Existing 12-report pattern owns this; no new export pipeline |
| Role gate (read history/report) | API / Backend (`authorize(&Identity, &Action::ReadPlaces)`, both transports) | — | D-12 — reuses existing Action, not a new one; UI hiding is cosmetic only |
| Bulk "move all contents" (D-28) | API / Backend (new `PlaceService` method, one tx, one movement row per item) | Browser / Client (`PlaceContents.svelte` new action + confirm dialog) | Mirrors `list_subtree_contents`'s existing caller-aware, place-scoped pattern |

## Standard Stack

### Core

No new external libraries. This phase is 100% additive schema + service-layer wiring within
the existing stack (`rusqlite` 0.39 + `refinery` migrations, `axum`/Tauri dual transport,
Svelte 5 runes). `[CITED: CLAUDE.md Technology Stack]`

### Migration shape

Next free migration number: **`V040`** `[VERIFIED: migrations/ directory listing, last file is
V039__place_path_display.sql]`.

Column set implied by the locked decisions (D-01/D-07/D-09/D-10), mirroring `audit_log`'s
append-only shape (`migrations/V008__audit_log.sql`: no `deleted_at_utc`, no `version` — hard
delete only) rather than the `standard4` soft-delete convention used elsewhere in this codebase:

```sql
CREATE TABLE place_movements (
  id                  INTEGER PRIMARY KEY AUTOINCREMENT,
  entity_type         TEXT    NOT NULL,   -- 'device' | 'cartridge' (D-21: printer is 'device')
  entity_id           INTEGER NOT NULL,
  from_place_id       INTEGER NOT NULL REFERENCES places(id) ON DELETE RESTRICT,
  from_place_path     TEXT    NOT NULL,   -- D-10 snapshot, via PlaceRepository::full_path
  to_place_id         INTEGER NOT NULL REFERENCES places(id) ON DELETE RESTRICT,
  to_place_path       TEXT    NOT NULL,   -- D-10 snapshot
  source              TEXT    NOT NULL,   -- 'manual' | 'act' | 'map' | 'workstation' (D-07)
  note                TEXT    NULL,       -- D-08: optional
  act_id              INTEGER NULL REFERENCES acts(id) ON DELETE SET NULL,  -- HST-03/D-03
  user_id             INTEGER NULL REFERENCES users(id) ON DELETE SET NULL, -- D-09; NULL = система
  actor_name_snapshot TEXT    NULL,       -- D-09/D-11: ФИО at write time, login-reuse-safe
  created_at_utc      INTEGER NOT NULL
);

CREATE INDEX idx_place_movements_entity
  ON place_movements(entity_type, entity_id, created_at_utc DESC);  -- HST-02 timeline query
CREATE INDEX idx_place_movements_created
  ON place_movements(created_at_utc);                               -- HST-04 period filter
CREATE INDEX idx_place_movements_from_place ON place_movements(from_place_id);  -- HST-04 «Откуда»
CREATE INDEX idx_place_movements_to_place   ON place_movements(to_place_id);    -- HST-04 «Куда»
CREATE INDEX idx_place_movements_act        ON place_movements(act_id) WHERE act_id IS NOT NULL;
  -- D-03: DELETE FROM place_movements WHERE act_id = ? during undo

PRAGMA user_version = 40;
```

Notes on the column choices (all `[ASSUMED]` naming per Claude's Discretion — confirm with
planner/user before locking):

- `from_place_id`/`to_place_id` are `NOT NULL` — this is the direct consequence of D-06 ("only
  when both sides are real tree nodes"): a row is only ever inserted when both are `Some`. The
  service layer is the enforcement point (skip the insert when either side is `None` or
  unchanged), not a SQL `CHECK`.
- `source` as a bare `TEXT` (no `CHECK` constraint enumerating the four tokens) mirrors the
  precedent set by `places.kind` (V037, validated in Rust via `PlaceKind::from_str`) and
  `places.path_variant_override` (V039, validated via `PathDisplayVariant::from_str`) —
  `[VERIFIED: migrations/V037__places.sql, migrations/V039__place_path_display.sql]`. The IN-01
  lesson from Phase 39.2 (an unknown token in `places.path_variant_override` crashed the whole
  place list via `FromSqlConversionFailure` because ONE of five consumers used `?` instead of
  `.ok()`) means every read site for `source` must degrade softly (`.ok()`/`unwrap_or`), never
  `?`/`.expect()` — this is Common Pitfall below.
- `idx_place_movements_act` exists specifically to make D-03's undo-time delete a plain
  `DELETE FROM place_movements WHERE act_id = ?1` with no JSON parsing — the entire reason D-01
  rejected reusing `audit_log` (whose `act_id` is buried in `payload_json`).

### Package Legitimacy Audit

**Not applicable — this phase installs no new external packages.** No `slopcheck` run was
needed; nothing to gate behind `checkpoint:human-verify`.

## Architecture Patterns

### System Architecture Diagram

```
                         ┌─────────────────────────────────────────────┐
                         │        Six existing write sites               │
                         │  (device/cartridge place_id mutations)        │
                         ├───────────────────────────────────────────────┤
  Tauri invoke ──┐       │ device_service.rs                             │
                 ├──────▶│   update()                       [manual]     │
  axum HTTP  ────┘       │ cartridge_service.rs                          │
   (both resolve         │   update()                       [manual]     │
    caller: Identity     │   transition() Install/Return/                │
    via authorize()      │     ToRefill/FromRefill          [D-05 auto]  │
    BEFORE calling in)   │   transition() nested auto-return             │
                         │     of previous cartridge         [D-05 auto] │
                         │ act_service.rs                                │
                         │   create()          (handover)   [act]        │
                         │   update()           (handover edit) [act]    │
                         │   do_return()        (return create) [act]    │
                         │   update_return()    (return edit) [act]      │
                         └───────────────┬───────────────────────────────┘
                                         │ same rusqlite::Transaction
                                         ▼
                         ┌───────────────────────────────────────────────┐
                         │ if before.place_id != after.place_id          │
                         │    AND both Some (D-06)                        │
                         │  1. resolve actor: caller.user_id +            │
                         │     SELECT full_name FROM users WHERE id=?     │
                         │     (or "система" if user_id IS NULL)          │
                         │  2. snapshot both paths via                    │
                         │     PlaceRepository::full_path(&tx, id)        │
                         │  3. INSERT INTO place_movements (...)          │
                         └───────────────┬───────────────────────────────┘
                                         │
                     ┌───────────────────┼────────────────────────┐
                     ▼                   ▼                        ▼
        ┌────────────────────┐ ┌──────────────────┐  ┌─────────────────────┐
        │ act_service::       │ │ report_service.rs │  │ new get_history-style│
        │ delete_soft /        │ │ list_movements()   │  │ read method (device/ │
        │ undo_device_          │ │ (13th list_*,       │  │ cartridge timeline)  │
        │ mutations_for_act     │ │ ReportFilter +      │  │                       │
        │ → DELETE FROM         │ │ from/to place_id)   │  │                       │
        │ place_movements       │ │ → export_csv/pdf    │  │                       │
        │ WHERE act_id=? (D-03) │ │ (existing pipeline) │  │                       │
        └────────────────────┘ └──────────────────┘  └───────────┬───────────┘
                                                                  ▼
                                                    ┌─────────────────────────────┐
                                                    │ PlaceEntityViewModal.svelte   │
                                                    │ (device+printer, D-14/D-21)   │
                                                    │ CartridgeDetail.svelte (D-16) │
                                                    │ — renders backend-shortened   │
                                                    │   from/to path, no JS formula │
                                                    └─────────────────────────────┘
```

### The six write sites (complete list, HST-01 objective)

All verified by direct read of the current codebase:

1. **`device_service.rs::update`** (line ~258) — manual edits via `PlacePicker` (D-27). Currently
   loads `before` and computes `after` in the same transaction already
   (`crates/trackly-app/src/services/device_service.rs:258-303`) — the before/after diff is
   already available, just needs to feed the new insert instead of being discarded.
2. **`cartridge_service.rs::update`** (line ~180-215) — manual edits. **Gap today:** this method
   does a bare `UPDATE cartridges SET place_id=?1 ...` with **no preceding `SELECT`** — before-
   state must be fetched first (or use SQLite `RETURNING`) before the diff can be computed.
3. **`cartridge_service.rs::transition` → `SqliteCartridgeRepository::transition_in_tx`**
   (`crates/trackly-infra/src/repos/cartridges_sqlite.rs:458-566`) — Install/ReturnToStock/
   ToRefill/FromRefill. `current.place_id` (before) and `new_place_id` (after) are already both
   computed locally inside this function (D-05 auto-movement).
4. **Same function, nested auto-return branch** (lines ~568-680) — when Install finds another
   cartridge already "В работе" in the target printer, it auto-returns that OTHER cartridge to
   stock in the same transaction, with its OWN before (`prev_current.place_id`) / after
   (`resolved_place_id`) pair — this is a **second, easy-to-miss** movement-producing site inside
   the same function.
5. **`act_service.rs::create`** (handover, `update_status_and_place_in_tx` call at line ~455) and
   **`act_service.rs::update`** (handover edit, same repo call at line ~718) — `act_id` is in
   scope at both call sites (HST-03).
6. **`act_service.rs::do_return`** (`update_full_in_tx` at line ~1408) and
   **`act_service.rs::update_return`** (same repo call, TWO call sites at lines ~1942 and ~2011 —
   one per device-processing loop) — return-flow. **Pitfall:** `update_full_in_tx` CAN write
   `place_id = NULL` (documented in-code as `DEF-3`, `act_service.rs:1307-1312`) when no
   `bulk_place_id`/`place_id_override` is supplied — this is the concrete, reachable case of
   D-06's "place → NULL is not recorded."

`printer_service.rs` mutates no `place_id` — confirmed via grep, zero matches
`[VERIFIED: crates/trackly-app/src/services/printer_service.rs — grep "place_id" returns
nothing]`. A printer's place changes exclusively through `device_service::update` on its
underlying `devices` row (D-21).

A **seventh** write site does not exist yet and must be built new: **D-28's "Перенести всё
содержимое в…"** bulk-move action. `PlaceService` (`crates/trackly-app/src/services/
place_service.rs`) already has the caller-aware, place-scoped read method
`list_subtree_contents(caller, root_id, nested)` (line 637) that returns
`Vec<PlaceContentRow>` with `kind: 'device'|'printer'|'cartridge'` and `id` — the bulk-move
method should walk this same result set inside one transaction, calling the SAME before/after
diff + movement-insert logic as write sites #1/#2 per row (treating `kind='printer'` exactly
like `kind='device'`, since both address `devices.place_id`).

### `undo_device_mutations_for_act` and D-03

`crates/trackly-app/src/services/act_service.rs:3073-3110` — `undo_device_mutations_for_act`
restores devices from `audit_log.before_json` snapshots, LIFO (`rows.into_iter().rev()`), called
from `delete_soft` (line ~2422) for handover (cascading through nested returns in reverse order
first, then the handover itself) and for a standalone return act's own undo. **Because `act_id`
is a first-class column on `place_movements` (not buried in JSON), D-03 does not need to walk the
undo's restore loop at all** — a single `tx.execute("DELETE FROM place_movements WHERE
act_id = ?1", [act_id])` per act being undone (each cascaded return act's own id, then the
handover's own id) is sufficient and atomic with the existing undo transaction. This is
simpler than mirroring the LIFO device-restore loop — no compensating record, no synthetic
"came back" row (per D-03's explicit "никакой компенсирующей записи не создаётся").

### The report clone (HST-04)

The report domain is a mechanical 13th addition to an established, rigid pattern:

- **`crates/trackly-app/src/dto/reports.rs`** — `ReportFilter` already carries `date_from_utc`,
  `date_to_utc`, `place_id` (subtree-inclusive, comment at line 30), `type_id`. D-24 needs two
  MORE place fields (`from_place_id`, `to_place_id`) added to this one shared struct — every
  other report domain's `list_*` simply ignores fields it doesn't use, so this is additive, not
  a new struct `[VERIFIED: crates/trackly-app/src/dto/reports.rs:17-61]`.
- **`ReportRow`** is intentionally sparse (`crates/trackly-app/src/dto/reports.rs:90-113`) — one
  wide struct, only relevant fields populated per report type. The movements report needs NEW
  fields this struct doesn't have yet: it currently has exactly one `place_path`/
  `place_path_short` pair (used for "this item's current place"), but D-23's columns need
  **two** place pairs (from/to). Plan to add `from_place_path`/`from_place_path_short`/
  `to_place_path`/`to_place_path_short` (or reuse `place_path` for "to" and add a new
  `secondary_place_path`/`secondary_place_path_short` pair — planner's naming call), plus a
  `reason` field (source label + note or act number, per D-23) and an `actor_name` field.
- **`report_service.rs`** has exactly 12 `list_*` async methods (line 432-914,
  `list_device_acts` .. `list_requests_completed`), all following the identical shape: resolve
  timezone → compute period bounds → `spawn_blocking` → `readers.acquire()` → a `query_*_inner`
  free function → return `ReportResponse`. The 13th (`list_movements` or similar) clones this
  shape exactly.
- **`export_csv`/`export_pdf`** (lines 854-1044) are report-type-agnostic — they take
  `columns: &[&str]` and drive `row_field(row, col, tz, shorten)` (line 1048), a big `match` on
  column-name strings. Adding the movements report means adding new `match` arms to `row_field`
  for the new `ReportRow` fields — no new export function.
- **`crates/trackly-app/src/tauly_cmds`** — correction, **`crates/trackly-app/src/tauri_cmds/
  reports.rs`** owns `columns_for(report_type)` / `column_labels_for(report_type)` /
  `report_display_name(report_type)` (lines 20-97), THREE parallel `match` statements keyed by a
  `report_type: &str` string (`"device_acts"`, `"cartridge_in_stock"`, etc., 12 existing arms).
  Adding `"movements"` (or similar) to all three, index-aligned (there is already a regression
  test enforcing `columns_for`/`column_labels_for` index-alignment,
  `column_labels_for_is_index_aligned_with_columns_for`, line ~609) is required, and this file
  IS the shared Tauri+HTTP layer — both `http/reports.rs` and Tauri commands call its `build_*`
  helpers `[VERIFIED: crates/trackly-app/src/tauri_cmds/reports.rs:1-8, "used by both Tauri
  commands and axum HTTP handlers"]`.
- **`ui/src/features/reports/ReportSubNav.svelte`** — `DOMAINS` array (line 66-70) currently has
  three entries (`devices`/`cartridges`/`requests`), each with its own `ReportConfig[]` array of
  `{key, label, temporal, cmd}`. D-22 wants "Перемещения" as its OWN fourth domain (not nested
  under devices/cartridges) — add a 4th `DOMAINS` entry with a single-report `ReportConfig[]`
  (entity-type split happens via the D-24-style filter, not via separate report keys).

### The timeline read-side clone (HST-02)

`CartridgeService::get_history` → `SqliteCartridgeRepository::get_history` →
`AuditEntryDto` → `CartridgeDetail.svelte`'s `formatHistoryEntry` is the exact shape to clone
for the NEW movement timeline, with two differences the plan must apply, not the old shape:

1. **A new DTO is required, not `AuditEntryDto`.** `AuditEntryDto` has no `user_id` field at all
   `[VERIFIED: crates/trackly-app/src/dto/cartridge.rs — grep "user_id" returns only the
   struct-name line, no field]`, and its `payload_json`-based rendering forces the frontend to
   `JSON.parse` and hand-pick keys (`CartridgeDetail.svelte:88-96`, explicitly commented as "a
   known display-quality tradeoff" printing a raw numeric `place_id`). The new DTO should be a
   flat, fully-typed struct (`from_place_path_short`, `to_place_path_short`, `actor_display`,
   `source`, `note`, `act_number`, `created_at_utc`) with formatting done server-side — this
   avoids reproducing the exact "numeric place_id leaked to the UI" pitfall D-16 explicitly
   calls out as NOT being fixed by this phase (the OLD «Журнал операций» section keeps that
   bug; the NEW one must not repeat it).
2. **`ORDER BY created_at_utc DESC, id DESC`** (`cartridges_sqlite.rs:1120`) is the exact
   sort/no-pagination pattern D-20 wants — clone this SQL shape verbatim for the new repo method.

### One-owner formula reuse (path shortening)

`act_service.rs::compute_place_path_short` (private fn, line ~3007) is the EXACT algorithm the
timeline and report need for rendering `from`/`to` short paths from a frozen snapshot + the
CURRENT effective display variant (D-18: "shortened based on current settings, not settings at
write time" — same resolution-order logic already validated for acts). It is currently
`fn` (private, not `pub`) inside `act_service.rs` and takes `&ReaderPool` directly. **Do not
copy this function a second time** — that is exactly the WR-08 anti-pattern Phase 39.2 just
finished eliminating (`read_path_display_separators` was duplicated 5 times before that phase;
the fix was a single `place_path_settings` module). Promote `compute_place_path_short` to a
shared location (either `place_path_settings` itself, since that module already owns
`DEFAULT_VARIANT`/separator reads, or a new small module) and have BOTH the new movement
write-side snapshot logic (uses `PlaceRepository::full_path`, unrelated) and the movement
READ-side rendering (uses `compute_place_path_short`, directly analogous to act rendering) call
the ONE promoted function.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Path shortening for from/to display | A second Rust function, or (worse) a JS mirror of the formula | Promoted `compute_place_path_short` (currently in `act_service.rs`) | Phase 39.2 (WR-08, WR-03) exists specifically because this formula was duplicated; a JS mirror already drifted once (`placePath.ts` WR-03) and needed a golden-fixture gate (`check-placepath-parity.mjs`) to catch it again |
| Actor "who" resolution | Resolving `user_id → ФИО` lazily at READ time (e.g., a JOIN to `users` in the timeline query) | A snapshot taken at WRITE time, stored as `actor_name_snapshot` | D-09 explicitly rejects read-time resolution: `usXXX` logins are reused across employees, so resolving by `user_id` after the fact attributes history to the WRONG person once a login is recycled |
| Path display at write time | Re-deriving `full_path` by walking `parent_id` manually | `PlaceRepository::full_path(&tx, id)` (already exists, used identically by `act_service.rs` for `place_path_snapshot`, D-16 of Phase 39) | One canonical query against `place_full_paths`, transaction-scoped, already battle-tested |
| Undo/compensation record for D-03 | A new "moved back" movement row when an act is deleted | A plain `DELETE FROM place_movements WHERE act_id = ?` | D-03 explicitly forbids compensating records; deleting by the first-class `act_id` column (not buried JSON) makes this trivial and atomic with the existing undo transaction |
| Bulk-move UI selection mechanism | A new checkbox-selection UI in the device list | The existing `list_subtree_contents` result set on `PlaceContents.svelte`, driving one new "Перенести всё содержимое в…" action | D-28 explicitly rejected checkbox selection; the panel already lists every item in a place's subtree |

**Key insight:** every hand-roll risk in this phase is a *repeat* of a specific historical bug
already paid for and fixed in Phase 39/39.1/39.2 (duplicated formula, lazy actor resolution
equivalent to the login-reuse hazard, numeric-ID-leak-to-UI). The plan should treat "did we just
reintroduce a fixed bug" as a first-class review question per task.

## Common Pitfalls

### Pitfall 1: The `caller: &Identity` plumbing gap (HIGH severity — sizes the whole phase)

**What goes wrong:** A task is written as "add movement recording to `device_service::update`"
without first threading `caller: &Identity` through the method signature, and the implementer
either (a) leaves `user_id` hardcoded `None` (breaking D-09/D-11 silently — every manual move
shows "система" even when a real Manager did it) or (b) has to make an emergency follow-up
signature change mid-plan.

**Why it happens:** The Tauri/HTTP adapter layer (`tauri_cmds/devices.rs`,
`tauri_cmds/cartridges.rs`, `tauri_cmds/acts.rs`, `http/*.rs`) already resolves and authorizes a
real `Identity` before calling into the service layer — it LOOKS like the identity is "already
there." It is there at the adapter, but every one of `device_service::update`,
`cartridge_service::update`, `cartridge_service::transition`, and all four `ActService` mutation
methods currently accept no `Identity`/`user_id` parameter at all and hard-code
`user_id_opt: Option<i64> = None` internally (with an explicit `// Phase 2 — no auth yet` comment
still present in `device_service.rs:165` and `:1031`, dating from before RBAC even existed).

**How to avoid:** Make "thread `caller: &Identity` into all six write-site methods" its own
explicit task/wave, done BEFORE the movement-insert logic is added, mirroring how
`place_service.rs` already takes `caller: &Identity` on every method (Phase 39 precedent). This
also means updating every `build_devices_update`/`build_cartridges_update`/
`build_cartridges_transition`/`build_acts_*` call site in both `tauri_cmds/*.rs` and `http/*.rs`
to actually pass `caller` down instead of dropping it.

**Warning signs:** Any new code path that still has a line reading
`let user_id_opt: Option<i64> = None;` right before the new `place_movements` insert.

### Pitfall 2: Cartridge `update()` has no before-state to diff against

**What goes wrong:** `cartridge_service.rs::update` (line ~180) issues a bare `UPDATE cartridges
SET place_id=?1, notes=?2, ... WHERE id=?4 AND version=?5` with **no preceding `SELECT`** of the
current row — unlike `device_service::update`, which already loads `before` for its
`audit_log.before_json`. Bolting a movement insert onto this method requires fetching the old
`place_id` first (either a `SELECT place_id FROM cartridges WHERE id=?` before the `UPDATE`, or
an SQLite `RETURNING` clause) — this is easy to skip if the task description just says "reuse
the diff pattern from `device_service`" without checking that cartridge's `update` doesn't have
one yet.

**How to avoid:** Explicitly call out in the task that `cartridge_service::update` needs a new
`SELECT` (or `RETURNING`) added, it is not already there.

**Warning signs:** A movement-insert diff that always shows `from_place_id == to_place_id`
because "before" was read from the row AFTER the `UPDATE` already ran.

### Pitfall 3: The nested auto-return inside `transition_in_tx` is a second, easy-to-miss write site

**What goes wrong:** A task only patches the "main" cartridge mutation inside
`SqliteCartridgeRepository::transition_in_tx` and misses the auto-return branch
(`cartridges_sqlite.rs:568-680`) that fires when Install finds ANOTHER cartridge already
installed in the target printer — that second cartridge also gets its `place_id` changed
(back to stock/refill), with its own separate `audit_log.insert` call already present at line
~674, but it is easy to overlook as "part of the same operation" rather than a second entity
needing its own movement row.

**How to avoid:** Treat `transition_in_tx`'s two `audit_repo.insert(...)` call sites (one per
mutated cartridge) as two separate movement-recording call sites, not one.

**Warning signs:** UAT where installing a cartridge into an occupied printer produces a
movement row for the NEW cartridge but not for the auto-returned OLD one.

### Pitfall 4: D-06's "place → NULL" is not hypothetical — it's a documented, reachable code path

**What goes wrong:** A task assumes "place is never cleared to NULL by existing flows, so the
both-Some guard is a formality." `act_service.rs:1307-1312` (comment tag `DEF-3`) documents that
`update_full_in_tx` in the return-flow WILL write `place_id = NULL` whenever the caller doesn't
supply `bulk_place_id`/`place_id_override` — i.e., a normal return-act flow without an explicit
place selection produces exactly the "to NULL" case D-06 says must be silently skipped, not
crash on.

**How to avoid:** Write an explicit test case: return an act with NO place override supplied,
assert zero new `place_movements` rows are created (device's place went to NULL, movement
correctly suppressed).

**Warning signs:** A NOT NULL constraint violation on `to_place_id` during return-act tests, or
(worse) a silently-passing insert with `to_place_id = NULL` that breaks the schema's `NOT NULL`
assumption above.

### Pitfall 5: Undo cascading order and `act_id` scoping (D-03)

**What goes wrong:** `delete_soft`'s handover-cascade path undoes RETURN acts first (LIFO,
`returns.iter().rev()`), soft-deletes each return act, THEN undoes and soft-deletes the handover
itself. If the `place_movements` deletion is bolted onto the WRONG loop iteration (e.g., deleting
by `act_id = handover_id` too early, before the nested returns' own `act_id`-scoped deletes have
run), a return act's movement rows could survive the delete of its parent handover, or vice
versa — because each act in the cascade (each return, then the handover) has its OWN `act_id`,
and each needs its OWN `DELETE FROM place_movements WHERE act_id = ?` at its OWN point in the
existing loop, not one blanket delete at the end.

**How to avoid:** Add the `DELETE FROM place_movements WHERE act_id = ?` call immediately
alongside each existing `acts_repo.soft_delete_in_tx(&tx, ret.id, ...)` / `audit_repo.insert(...,
action: "delete", ...)` pair inside the cascade loop, and again for the handover's own
soft-delete — not as a single extra statement bolted onto the end of `delete_soft`.

**Warning signs:** After deleting a handover act with nested returns, `place_movements` still
has rows for one of the return acts' `act_id`s.

### Pitfall 6: Unknown `source` token must degrade softly (IN-01 recurrence risk)

**What goes wrong:** A read site for `place_movements.source` uses `PathDisplayVariant`-style
strict parsing with `?`/`.expect()` instead of `.ok()`/`unwrap_or` — exactly the Phase 39.2 IN-01
bug (`places_sqlite.rs:59-72` used `?` where four OTHER consumers of the same
`path_variant_override` token used `.ok()`, and the one strict consumer crashed the entire place
list on a single bad row via `FromSqlConversionFailure`). `source` will very likely have MULTIPLE
read sites (timeline row label, report row label, report filter/grouping) — the same "5 copies,
1 forgot to degrade softly" shape that caused IN-01.

**How to avoid:** Write `source` parsing ONCE as a small helper (mirroring how
`PathDisplayVariant::from_str` is the single parse point) that returns a safe fallback label
(e.g., "неизвестно") instead of erroring, and make every read site call that helper, never
`match` the raw string ad hoc.

**Warning signs:** A test that seeds an unrecognized `source` value and asserts the WHOLE
timeline/report screen still renders (not just that one row).

### Pitfall 7: Report `ReportRow`'s existing single place-pair doesn't fit a from/to report

**What goes wrong:** The movements report reuses `ReportRow.place_path`/`place_path_short` for
ONE side (say, "to") and stuffs the other side into a field meant for something else (e.g.,
`device_name` gets overloaded to carry "from"), because `ReportRow` is described as "sparse,
reuse whatever's free" and the two-place requirement (D-24) isn't obviously visible from a
glance at the struct.

**How to avoid:** Add genuinely new, clearly-named fields to `ReportRow` for the "from" side
(the "to" side can reasonably reuse the existing `place_path`/`place_path_short` pair) — this is
additive to a struct 11 other report types already share safely (each report type only reads
the columns it declared in `columns_for`), so there is no compatibility risk in adding fields.

**Warning signs:** A CSV export where the "Откуда" column silently shows the device's model or
another unrelated field.

## Code Examples

### Before/after diff already available at write site #1 (device manual update)

```rust
// Source: crates/trackly-app/src/services/device_service.rs:258-303 (existing code, current before/after diff)
let before = repo.get_in_tx(&tx, id).ok();
// ... 
let after = repo.update_in_tx(&tx, id, version, &domain_patch, now)?;
// D-06/D-27 hook point: compare before.place_id vs after.place_id here, both Some, differ.
```

### The exact shortening-resolution algorithm to promote and reuse

```rust
// Source: crates/trackly-app/src/services/act_service.rs:3007-3038 (compute_place_path_short)
// Resolution order: snapshot present? -> place_id's CURRENT effective_variant (place_effective_variant
// view) -> else org default (read_org_default_variant_token) -> else DEFAULT_VARIANT constant.
// Never .expect()/.unwrap()/`?` on this path — cosmetic field, must not fail act/report/timeline render.
fn compute_place_path_short(
    readers: &ReaderPool,
    place_id: Option<i64>,
    snapshot: Option<String>,
) -> Option<String> { /* ... */ }
```

### The read-side history query pattern to clone (HST-02)

```sql
-- Source: crates/trackly-infra/src/repos/cartridges_sqlite.rs:1110-1121 (get_history)
-- Clone shape for place_movements: same ORDER BY, same WHERE entity_type/entity_id scoping.
SELECT id, entity_type, entity_id, action, user_id,
       before_json, after_json, payload_json, created_at_utc
  FROM audit_log
 WHERE entity_type = 'cartridge'
   AND entity_id = ?1
   AND action NOT IN ('list', 'get')
 ORDER BY created_at_utc DESC, id DESC
```

### D-03's undo scoping — where the DELETE belongs

```rust
// Source: crates/trackly-app/src/services/act_service.rs:2422-2470 (delete_soft, handover branch)
// Each `ret` in the cascade loop, THEN the handover itself, gets its own soft-delete + audit.
// Add: tx.execute("DELETE FROM place_movements WHERE act_id = ?1", [ret.id])? right alongside
// each `acts_repo.soft_delete_in_tx(&tx, ret.id, ...)` call — and again for `id` (handover) below.
for ret in returns.iter().rev() {
    undo_device_mutations_for_act(&tx, &devices_repo, &audit_repo, ret.id, user_id_opt, now)?;
    acts_repo.soft_delete_in_tx(&tx, ret.id, ret.version, now)?;
    // <-- DELETE FROM place_movements WHERE act_id = ret.id here
    audit_repo.insert(&tx, AuditEntry { entity_type: "act", entity_id: ret.id, action: "delete", .. })?;
}
undo_device_mutations_for_act(&tx, &devices_repo, &audit_repo, id, user_id_opt, now)?;
acts_repo.soft_delete_in_tx(&tx, id, version, now)?;
// <-- DELETE FROM place_movements WHERE act_id = id here
```

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Table/column names (`place_movements`, `from_place_id`, `to_place_id`, `source`, `note`, `actor_name_snapshot`) | Standard Stack | Low — explicitly Claude's Discretion per CONTEXT.md; planner/user can rename freely before migration ships |
| A2 | `source` stored as unconstrained `TEXT` (no CHECK, no lookup table) | Standard Stack, Pitfall 6 | Low-medium — mirrors an established, working precedent (`places.kind`/`path_variant_override`), but if the user prefers a lookup table for FK-enforced integrity, migration shape changes |
| A3 | AND semantics for the two place filters (D-24) when both are set | Discretion (carried from CONTEXT.md) | Medium — user explicitly flagged this needs confirming against HST-04's wording before locking; OR semantics would need a different WHERE clause shape |
| A4 | `compute_place_path_short` should be promoted out of `act_service.rs` into a shared module rather than the movements code calling into `act_service` directly | Architecture Patterns | Low — either works functionally; promoting avoids a cross-module `pub(crate)` dependency from report/movement code into act_service, which would read oddly, but is not a correctness risk |
| A5 | The bulk "move all contents" method (D-28) belongs on `PlaceService`, not a new service | Architecture Patterns | Low — `PlaceService` already owns the read-side (`list_subtree_contents`) this action is scoped by; a separate service would duplicate the subtree-walk logic |

**If this table is empty:** N/A — see entries above. All are low-to-medium risk; none blocks
starting the plan, but A3 should be confirmed with the user during `/gsd-discuss-phase`
follow-up or explicitly locked by the planner with a stated rationale.

## Open Questions

1. **AND vs OR semantics for the two place filters when both «Откуда» and «Куда» are set (D-24)**
   - What we know: CONTEXT.md's own discretion note recommends AND ("из А в Б") but says to
     confirm against HST-04's literal wording.
   - What's unclear: HST-04 only says "фильтр по месту" (singular) — the two-filter split is a
     D-24 refinement made during discuss-phase, not present in the original requirement text.
   - Recommendation: Implement AND (matches the CONTEXT.md discussion's own stated intent and
     example — "со склада в Здание Б"); OR would need a fundamentally different SQL shape
     (`from_place_id = X OR to_place_id = X` vs two independent subtree-bounded equalities) and
     nothing in the discussion suggests OR was ever the intent.

2. **Exact `ReportRow` field names for the "from" side of the movements report**
   - What we know: The "to" side can reuse the existing `place_path`/`place_path_short` pair
     (already wired into `row_field`/CSV/PDF); D-23 needs a second, "from" pair that doesn't
     exist on the struct yet.
   - What's unclear: Whether to name it `from_place_path`/`from_place_path_short` (clearest) or
     reuse a more generic pre-existing-but-unused field — none exists, so this is really just a
     naming choice, not an ambiguity about behavior.
   - Recommendation: Add `from_place_path: Option<String>` / `from_place_path_short:
     Option<String>` — clearest name, matches the `from_place_id`/`to_place_id` column naming
     already chosen for the table.

3. **Where does `compute_place_path_short` live after promotion?**
   - What we know: It currently lives in `act_service.rs` as a private fn; `place_path_settings`
     (in `trackly-infra`) already owns the org-default/separator reads it depends on.
   - What's unclear: Whether promoting it INTO `place_path_settings` violates that module's
     stated scope (its own doc-comment frames it narrowly as "settings reads," not "path
     shortening business logic" — `shorten_place_path` itself already lives in `trackly-core`
     per that module's own doc-comment on the hexagonal-boundary rule, FOUND-01).
   - Recommendation: A new small function/module in `trackly-app` (e.g.,
     `services/place_path_display.rs`) that both `act_service.rs`, the new movement read-path,
     and the new report read-path import from, keeping `trackly-core`'s `rusqlite`-free
     boundary intact (the function needs `&ReaderPool`, i.e., I/O, so it cannot live in
     `trackly-core` per the existing `no_io_deps.rs` gate).

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test` (workspace), no `nextest` in CI (confirmed: `ci-fast.yml` runs plain `cargo test --workspace --no-fail-fast -- --test-threads=1`) `[VERIFIED: .github/workflows/ci-fast.yml]` |
| Config file | none — tests are plain `#[test]`/`#[tokio::test]` functions in `crates/*/tests/*.rs` and inline `#[cfg(test)] mod tests` blocks |
| Quick run command | `cargo test -p trackly-app <test_name_substring> -- --test-threads=1` (single package/file scope during dev) |
| Full suite command | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test --workspace --no-fail-fast -- --test-threads=1` (exact CI invocation) |
| Frontend gates | `pnpm svelte-check` + `pnpm lint` (the latter chains eslint, prettier --check, and 7 project-specific `check-*.mjs` scripts including `check-placepath-parity.mjs`, `check-place-path-short.mjs`) `[VERIFIED: ui/package.json "lint" script]` |

**⚠ Known project-wide test hazard (from user memory, not this research session):** running two
`cargo test` invocations concurrently contends on the `target/` build lock and looks like a
multi-minute hang — run one at a time. Also: `cargo test -p trackly-app` (not just
`--workspace`) is needed to reliably hit some pre-existing tests that hang under
`--workspace`-only invocation in specific configurations (`login_remember_persistent_cookie`).

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| HST-01 | Manual device place change (D-27) creates one movement row with source='manual' | integration | `cargo test -p trackly-app place_movements_manual_device -- --test-threads=1` | ❌ Wave 0 |
| HST-01 | Manual device place change with NO actual place change creates zero rows (D-04) | integration | same file | ❌ Wave 0 |
| HST-01 | Cartridge Install into a printer creates one movement row, source derived correctly (D-05) | integration | `cargo test -p trackly-app place_movements_cartridge_transition -- --test-threads=1` | ❌ Wave 0 |
| HST-01 | Cartridge Install auto-return of a previous cartridge creates a SECOND movement row (Pitfall 3) | integration | same file | ❌ Wave 0 |
| HST-01 | place → NULL (return act, no override supplied) creates ZERO rows (D-06, Pitfall 4) | integration | `cargo test -p trackly-app place_movements_null_place_skip -- --test-threads=1` | ❌ Wave 0 |
| HST-01 | NULL → place (first assignment) creates ZERO rows (D-06) | integration | same file | ❌ Wave 0 |
| HST-01 | `user_id IS NULL` movement renders/stores as «система» downstream (D-11) | unit + integration | `cargo test -p trackly-app place_movements_system_actor` | ❌ Wave 0 |
| HST-01 | Unknown `source` token degrades softly, does not crash timeline/report (Pitfall 6, IN-01 recurrence) | integration | `cargo test -p trackly-app place_movements_unknown_source_degrades` | ❌ Wave 0 |
| HST-02 | Device/cartridge/printer timeline query returns rows newest-first, unpaginated (D-20) | unit | `cargo test -p trackly-infra place_movements_history_order` | ❌ Wave 0 |
| HST-02 | Printer timeline reads the SAME rows as its underlying device (D-21, no double-accounting) | integration | `cargo test -p trackly-app place_movements_printer_is_device` | ❌ Wave 0 |
| HST-03 | Handover creates a movement row with `act_id` set (D-01/HST-03 link) | integration | `cargo test -p trackly-app place_movements_act_link` | ❌ Wave 0 |
| HST-03 | Deleting a handover act deletes its movement rows (D-03) | integration | `cargo test -p trackly-app place_movements_act_undo_deletes` | ❌ Wave 0 |
| HST-03 | Deleting a handover with nested returns deletes EACH act's own movement rows correctly scoped (Pitfall 5) | integration | same file | ❌ Wave 0 |
| HST-04 | Report: two place filters (from/to) both set → AND semantics (A3) | integration | `cargo test -p trackly-app report_movements_place_filters` | ❌ Wave 0 |
| HST-04 | Report: subtree-inclusive on both from/to filters (D-24, mirrors D-28 Phase 39) | integration | same file | ❌ Wave 0 |
| HST-04 | Report: soft-deleted item still appears, marked "удалено" (D-25) | integration | `cargo test -p trackly-app report_movements_deleted_item_marker` | ❌ Wave 0 |
| HST-04 | `columns_for`/`column_labels_for` index-alignment holds for the new `"movements"` report type (regression test already exists, must extend) | unit | `cargo test -p trackly-app column_labels_for_is_index_aligned_with_columns_for` | ✅ existing (extend, don't duplicate) |
| HST-01..04 | Role matrix: Manager allowed, Employee 403, on BOTH transports, for every new endpoint (movements read, report, bulk-move) | integration | extend `crates/trackly-app/tests/role_endpoint_matrix.rs` (new Cases, following the Case 45/46/47/48 four-part pattern for `ReadPlaces`) | 🔶 pattern exists, new cases needed |
| Privacy | Fixtures/tests introducing ФИО use only invented names, `check-privacy.mjs` stays green | gate | `node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256` | ✅ existing gate, no new file |

### Sampling Rate

- **Per task commit:** targeted `cargo test -p trackly-app <substring> -- --test-threads=1` for
  the touched write site, plus `pnpm svelte-check` for touched `.svelte` files.
- **Per wave merge:** full workspace `cargo test --workspace --no-fail-fast -- --test-threads=1`
  (with `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1`) + `pnpm lint` + `pnpm build` (rebuild `ui/dist`
  before any test that serves the embedded SPA).
- **Phase gate:** full suite green + `role_endpoint_matrix.rs` new cases green before
  `/gsd-verify-work`.

### Wave 0 Gaps

- [ ] `crates/trackly-infra/tests/place_movements_migration.rs` (or extend
      `migration_idempotency.rs`) — V040 migration idempotency + fresh-DB seed check, mirroring
      the existing pattern for V037-V039.
- [ ] `crates/trackly-app/tests/place_movements_write_sites.rs` — the six write-site integration
      tests listed in the table above (manual device, manual cartridge, transition, nested
      auto-return, null-skip both directions, system actor).
- [ ] `crates/trackly-app/tests/place_movements_act_link.rs` — HST-03 act-link + D-03 undo tests
      (including the nested-cascade Pitfall 5 case).
- [ ] `crates/trackly-app/tests/report_movements.rs` — HST-04 report filter/export tests.
- [ ] Extend `crates/trackly-app/tests/role_endpoint_matrix.rs` with new Cases for
      movements-read endpoints and the new report endpoint (Manager allow, Employee 403, both
      transports — mirror Cases 45-48's four-part shape for `Action::ReadPlaces`).
- [ ] Fixture file for invented ФИО in tests (e.g., "Иванов И.И.", "Петров П.П.") — no new
      infra needed, just discipline; flag explicitly since this is the FIRST time a table other
      than `users`/`acts` stores a ФИО string.
- [ ] Frontend: no new golden-fixture gate needed IF the "one owner" recommendation
      (`compute_place_path_short` reused server-side, no JS mirror) is followed — this is itself
      a thing to verify during review (confirm no new `previewShortenXxx`-style function was
      added to `ui/src/lib/utils/`).

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | Unchanged by this phase — existing session/desktop-lock auth |
| V3 Session Management | no | Unchanged |
| V4 Access Control | yes | `authorize(&caller, &Action::ReadPlaces)` reused verbatim for the new timeline-read and report endpoints (D-12); gate MUST be checked on both Tauri `build_*` helpers and `http/*.rs` handlers, per the IN-02 lesson (a mutation/read that only gets a role check on ONE transport is a real, previously-shipped bug class in this codebase) |
| V5 Input Validation | yes | `source` enum parsed via a single `.ok()`-degrading helper (Pitfall 6), never trusted raw from client input on write (server always sets `source` itself based on which write site fired — clients never supply `source` directly, per D-27's "поменял PlacePicker → сохранил" flow having no new UI field for it) |
| V6 Cryptography | no | Not implicated |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| BOLA on movement-history read (e.g., Employee crafts a request for a specific `entity_id`'s history) | Elevation of Privilege / Information Disclosure | `Action::ReadPlaces` gate is role-based, not ownership-based (Admin/Manager see ALL history, matching D-12's stated scope — no per-item ownership check needed since Employee is denied the whole surface, not filtered) |
| Read-transport asymmetry (gate only on Tauri OR only on HTTP) | Elevation of Privilege | Both `build_*` helper AND the corresponding `http/*.rs` handler must call `authorize()` — mirrors the existing `role_endpoint_matrix.rs` Case 45/48 pattern (Case 45 = HTTP, Case 48 = Tauri, for the SAME action) |
| Client-supplied `source`/`user_id` on write (spoofing the "who"/"why") | Spoofing / Tampering | Server derives `user_id` from the authenticated `caller: &Identity`, never from request body; `source` is derived from which write-site code path fired, never from a client field |

## Sources

### Primary (HIGH confidence — direct codebase reads, this session)

- `crates/trackly-app/src/services/act_service.rs` — write sites (create/update/do_return/
  update_return/delete_soft), `undo_device_mutations_for_act`, `compute_place_path_short`
- `crates/trackly-app/src/services/device_service.rs` — `update` before/after diff pattern
- `crates/trackly-app/src/services/cartridge_service.rs` — `update` (no-diff gap), `transition`
- `crates/trackly-infra/src/repos/cartridges_sqlite.rs` — `transition_in_tx`, nested auto-return,
  `get_history`
- `crates/trackly-infra/src/repos/devices_sqlite.rs` — `update_status_and_place_in_tx`,
  `update_full_in_tx`, `restore_from_snapshot_in_tx`
- `crates/trackly-infra/src/repos/place_path_settings.rs` — single-owner pattern (WR-08 lesson)
- `crates/trackly-infra/src/repos/places_sqlite.rs` — `full_path_impl`
- `crates/trackly-core/src/auth.rs` — `Action`/`Identity`/`authorize` (D-12 reuse target,
  Identity has no ФИО field)
- `crates/trackly-app/src/dto/reports.rs`, `crates/trackly-app/src/services/report_service.rs`,
  `crates/trackly-app/src/tauri_cmds/reports.rs` — 12-report pattern to clone
- `crates/trackly-app/src/services/place_service.rs` — `list_subtree_contents`, caller-aware
  method pattern
- `crates/trackly-app/tests/role_endpoint_matrix.rs` — Cases 45-51, four-part gate-testing
  pattern
- `ui/src/features/cartridges/CartridgeDetail.svelte`, `ui/src/features/places/
  PlaceEntityViewModal.svelte`, `ui/src/features/places/PlaceContents.svelte`,
  `ui/src/features/reports/ReportSubNav.svelte` — UI entry points and clone targets
- `migrations/V008__audit_log.sql`, `V037__places.sql`, `V038__..._migrate_...sql`,
  `V039__place_path_display.sql` — migration shape precedents
- `.github/workflows/ci-fast.yml`, `ui/package.json` — test/lint command inventory

### Secondary (MEDIUM confidence)

None — this phase required no external library/framework research; all findings are direct,
verifiable codebase reads.

### Tertiary (LOW confidence)

None.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new dependencies; schema shape modeled directly on two existing,
  in-repo precedents (`audit_log`, `place_path_display`)
- Architecture: HIGH — all six write sites located and read directly; the `caller: &Identity`
  gap is a verified fact (grep + direct read), not an inference
- Pitfalls: HIGH — every pitfall cites a specific line/file and, where applicable, an already-
  fixed prior incident (IN-01, WR-03, WR-08) from this exact codebase's own review history

**Research date:** 2026-09-01
**Valid until:** No expiry driver — this is an internal-codebase research artifact, not
framework/library research subject to upstream drift. Re-verify write-site line numbers only if
another phase touches `act_service.rs`/`device_service.rs`/`cartridge_service.rs` before Phase 40
executes.
