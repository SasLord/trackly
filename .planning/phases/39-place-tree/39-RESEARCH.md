# Phase 39: Дерево мест - Research

**Researched:** 2026-08-22
**Domain:** Hierarchical tree storage in SQLite (rusqlite/refinery) + Rust dual-transport
service layer (Tauri + axum) + Svelte 5 tree/picker UI, replacing a flat freeform-text
location model across ~20 call sites.
**Confidence:** HIGH (schema/migration/FTS design verified against the actual repo; UI
mechanics deferred to locked `39-UI-SPEC.md`; natural-sort/library claims are LOW→MEDIUM,
flagged below)

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01 (Вложенность — свободная):** жёстких запретов «какой тип в какой» НЕТ. Любой узел
  может лежать в любом. UI при создании подсказывает типичный тип потомка по родителю, но не
  блокирует иное.
- **D-02 (Типы — закрытый enum):** ровно шесть типов — территория / зона / здание / этаж /
  помещение / уличный объект. Расширяемого справочника типов НЕТ.
- **D-03 (Много корней):** `parent_id` NULL допустим для узла ЛЮБОГО типа. Автоматического
  корня-«Организация» нет.
- **D-04 (Уникальность имени в братьях):** `UNIQUE(parent_id, name)` с учётом soft-delete.
- **D-05 (Порядок братьев):** по умолчанию — `level` (если задан), затем натуральное сравнение
  имени («2» раньше «10»). Опциональный ручной `sort_order` побеждает автоматический порядок.
- **D-06 (Привязка к любому узлу):** устройство и картридж привязываются к узлу ЛЮБОГО уровня.
  Ограничения «только листья» нет.
- **D-07 (Место — необязательное поле):** `place_id` NULL допустим. Узла-карантина
  «Не разобрано» НЕТ.
- **D-08 (Склад — не тип узла):** булев признак `is_storage` на обычном узле (обычно тип
  «помещение»).
- **D-09 (Склад внутри кабинета — вложенный узел):** оформляется вложенным узлом с
  `is_storage=true`, а не флагом на самом кабинете.
- **D-10 (Складское место ≠ «не в эксплуатации»):** `is_storage` НЕ определяет статус того, что
  там лежит.
- **D-11 (Что меняет `is_storage` в фазе 39):** (1) форма возврата акта поднимает складские места
  наверх/подставляет по умолчанию; (2) быстрый фильтр «на складе / в эксплуатации»; (3) форма
  ПРЕДЛАГАЕТ статус «на складе» по умолчанию при перемещении на складское место, без
  принудительной смены.
- **D-12 (Собственный `place_id` всегда):** у картриджа своё место, как у устройства, не
  вычисляемое из принтера. При Install подставляется из принтера, дальше живёт самостоятельно.
- **D-13 (Операции картриджа — выбор места вместо текста):** текстовое `location` во всех пяти
  вариантах `CartridgeTransitionOp` заменяется выбором места из дерева.
- **D-14 (Удаление запрещено, пока узел не пуст):** ошибка с точным счётчиком («в месте 12
  устройств и 2 вложенных места») + кнопка «Показать содержимое». Никакого каскада/автоподъёма.
- **D-15 (Архивация отдельно от удаления):** «Архивировать» прячет узел из `PlacePicker`, но
  оставляет в дереве/карточках/истории. Жёсткое удаление — только для пустых узлов без истории.
- **D-16 (Акт хранит снимок пути):** акт хранит `place_id` И текстовый снимок полного пути на
  момент выписки. Печать показывает СНИМОК; навигация в приложении — по `place_id`.
- **D-17 (Один общий PlacePicker):** единственный компонент на всё приложение, заменяет
  `LocationAutocomplete.svelte`. По фокусу — дерево; при наборе текста — плоский список по
  ПОЛНОМУ пути.
- **D-18 (Создание места из PlacePicker — только Admin):** «Создать "214" в "Здание А / 2 этаж"»
  видно только роли Admin.
- **D-19 (Свой пункт в сайдбаре):** отдельный раздел «Места», НЕ вкладка в Настройках, рядом с
  «Картой». `/map` не трогается.
- **D-20 (Доступ):** Admin редактирует дерево; Manager видит дерево и содержимое без кнопок
  редактирования; Employee не видит раздел вообще. Гейт — на бэкенде, на обоих транспортах.
- **D-21 (Перемещение узла — И drag-n-drop, И диалог):** оба пути ведут к одному диалогу
  подтверждения с последствиями («переедет 3 вложенных места и 47 устройств»).
- **D-22 (Две панели в разделе «Места»):** слева дерево, справа содержимое выбранного узла.
- **D-23 (Одна таблица с колонкой «Тип»):** устройство/принтер/картридж в одной таблице,
  фильтр — вкладки. Фаза 41 добавит «АРМ».
- **D-24 (Вложенные — по умолчанию да, с тумблером):** по умолчанию показываются вложенные места,
  тумблер «только здесь».
- **D-25 (Счётчики на узлах дерева):** суммарный счётчик С УЧЁТОМ вложенных.
- **D-26 (В таблицах — сокращённый путь + tooltip):** два последних сегмента, полный путь в
  `title`/карточке.
- **D-27 (В печатных формах — полный путь):** акт печатает ПОЛНЫЙ путь (снимок из D-16).
- **D-28 (Фильтр по месту — включая вложенные):** выбор узла в фильтре захватывает поддерево
  целиком.
- **D-29 (Место — в обоих FTS-индексах):** поиск по «214» находит И устройства, И картриджи по
  месту; переименование/перемещение обновляет ОБА индекса без ручной переиндексации.

### Claude's Discretion

- **Способ хранения дерева в SQLite** — adjacency list + рекурсивный CTE, денормализованная
  колонка пути, closure table или materialized path. Требование — выполнить D-29/PLC-05 на
  масштабе ~4–5 зданий × 2–3 этажа × ≤20 кабинетов.
  → **Research answer:** adjacency list + recursive CTE (see Architecture Patterns §1).
- **Защита от циклов** при перемещении узла — обязательна, реализация на усмотрение.
  → **Research answer:** recursive-CTE ancestor check inside the writer transaction (§1.4).
- **Механика синхронизации FTS** для D-29 — триггеры vs пересчёт в сервисном слое.
  → **Research answer:** do NOT denormalize place text into FTS5 at all; resolve place path live
  via a `place_full_paths` view/JOIN at query time (§1.5). This makes "no reindex needed" true by
  construction rather than by trigger cascade correctness.
- **Форма хранения снимка пути в акте** (D-16) — колонка в `acts` / в `act_items` / в обеих.
  → **Research answer:** `acts.place_path_snapshot` only (§2, rationale below — matches existing
  `act.location_name` being a header-level, not per-item, template field).
- **Названия таблиц и колонок** (`places`, `place_id`, `is_storage`, `level`, `sort_order`) —
  рабочие имена, не обязательство.
  → **Research answer:** keep these names; they match the project's `standard4` naming
  conventions (see §2 for the full proposed schema).

### Deferred Ideas (OUT OF SCOPE)

- Чертежи, планы этажей, SVG-редактор, подложки — Фазы 43–44.
- История перемещений (append-only log of place changes) — Фаза 40. Phase 39 only needs to leave
  `place_id` changes observable at the service layer (a single mutation point) so Phase 40 can
  hook in without refactoring.
- АРМ (рабочее место) — Фаза 41. D-23's "one table + type column" content screen already leaves
  room for a fifth "АРМ" row type.
- Ранжирование принтеров по месту автора — Фаза 42. `requests.printer_location` is touched only
  mechanically (rename to path-resolved value), not reinterpreted.
- MAPX (v2: metric scale, "обход кабинетов", генплан, IP-subnet auto-placement) — explicitly
  deferred by the user 2026-08-22, do not re-propose.
- **Data migration of existing `locations` values is explicitly NOT required** — confirmed twice
  in CONTEXT.md: the app is not in production yet, all acts in the DB are test data, placement
  values are zeroed out. `REQUIREMENTS.md` Out of Scope table states this explicitly. Migrations
  below are therefore schema-only (DROP + CREATE), not data-preserving ETL.

</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| PLC-01 | Дерево мест произвольной вложенности, переименование/перемещение без потери привязок | §1 (adjacency-list model — rename/move are single-row `UPDATE`s, FK `place_id` on devices/cartridges never changes during a place rename/move, so bindings survive by construction) |
| PLC-02 | Этаж — числовой `level`, включая 0 и отрицательные, сортировка по уровню не по имени | §2 (`places.level INTEGER NULL`, no `CHECK >= 0`), §5 (sibling-sort algorithm, implemented in Rust not SQL) |
| PLC-03 | Выбор места из дерева с поиском по полному пути, замена свободного текста | §1.2 (full-path resolution), §6 (existing blast-radius inventory), §7 (PlacePicker backend contract) |
| PLC-04 | Удаление `locations` и свободнотекстового «Размещение» из всего приложения | §3 (migration sequence), §6 (full call-site inventory: Rust + Svelte, file:line) |
| PLC-05 | Переименование/перемещение мгновенно отражается в поиске и списках без ручной переиндексации | §1.5 (live-JOIN search design — no cache to go stale) |
| PLC-06 | Открыв место, видеть всё размещённое в нём и во вложенных (устройства, АРМ, принтеры, картриджи) одним списком | §1.3 (subtree contents/counts query, reused for D-14/D-21/D-25/D-28) |

</phase_requirements>

---

## Summary

Trackly's place model is currently a single flat `locations` table (`name UNIQUE`) referenced
three inconsistent ways: `devices.location_id` (FK), `acts.location_id` /
`acts.bulk_location_id` / `ActReturnItem.location_id_override` (FK, three separate fields at
three call sites), and `cartridges.location` (freeform `TEXT`, explicitly commented in the
schema as "locations table is for devices"). Phase 39 replaces all four with one polymorphic
`places` table and a single `place_id` FK used consistently everywhere.

Given the confirmed real scale (~4–5 buildings × 2–3 floors × ≤20 rooms, 100–150 devices per
floor — a few hundred place rows, a few thousand device/cartridge rows), the correct engineering
choice is the **simplest correct model, not the most scalable one**: a plain adjacency list
(`places.parent_id`) with recursive CTEs computed on read. Closure tables and materialized-path
columns exist to solve query-latency problems that do not exist at this scale, and both add
non-trivial trigger/maintenance code that this phase does not need. The one place where this
matters most — PLC-05's "no manual reindex" requirement — is best satisfied not by making FTS5
cascade correctly on rename (a genuinely hard problem, see Common Pitfalls §1) but by **never
caching place text in the first place**: resolve full paths live via a `place_full_paths` SQL
view at query time. There is nothing to go stale because nothing is denormalized.

The blast radius is large but mechanical: three Rust crates (`trackly-core` domain types,
`trackly-infra` repo/SQL, `trackly-app` services/DTOs/Tauri commands/HTTP handlers) each touch
5 domain modules (devices, cartridges, acts, printers, requests) plus a brand-new `places` module
following the exact same five-file pattern (`domain/ports/repos/service/dto+transport`) already
established by every other entity in this codebase. On the frontend, one component
(`LocationAutocomplete.svelte`) is deleted and replaced by a new `PlacePicker.svelte` (fully
specified in `39-UI-SPEC.md` — this research does not re-derive any UI decision), consumed by
~9 existing feature files.

**Primary recommendation:** adjacency list (`places.parent_id`, recursive CTE for tree/path
reads) + a `place_full_paths` view for live path resolution, no FTS5 caching of place text, one
new `places_*` Rust module mirroring the existing `devices_*` five-file pattern, and a single
reusable "subtree stats" query (descendant place IDs + content counts) shared by D-14/D-21/D-25/
D-28/PLC-06.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Place tree CRUD (create/rename/move/archive/delete) | API/Backend (`trackly-app` service, single-writer) | Database (SQLite constraints: `UNIQUE(parent_id,name)`, `FK RESTRICT`) | Single-writer pattern already mandated by CLAUDE.md; DB constraints are defense-in-depth, not the primary enforcement point (need precise error counts, which SQL constraints can't produce) |
| Cycle prevention on move | API/Backend (service layer, recursive-CTE ancestor check inside the writer's transaction) | — | Must run inside the same transaction as the move to avoid TOCTOU races; single-writer task makes this trivial (no concurrent writers to race against) |
| Full path resolution (breadcrumbs, PlacePicker labels, print snapshot) | Database (SQL view via recursive CTE) | API/Backend (thin wrapper) | Recursive CTE is the correct SQLite-native tool; wrapping it in a view keeps the SQL in one place instead of duplicated across 6+ call sites |
| Place-based search (PLC-05 instant reflect) | API/Backend (service layer composes FTS MATCH + live JOIN against `place_full_paths`) | Database (FTS5 for intrinsic device/cartridge fields only) | Live JOIN structurally guarantees "no reindex" — see Summary; FTS5 stays scoped to fields that don't change via cascading edits |
| PlacePicker tree/search UI | Browser/Client (Svelte 5 component) | — | Pure presentation; UI-SPEC already locks every visual/interaction decision |
| Role gate (Admin edits, Manager/Admin read, Employee blocked) | API/Backend (`authorize()` in `trackly-core::auth`, checked on both Tauri and axum handlers) | Browser/Client (sidebar item hidden, buttons hidden — UX only, not security) | Established pattern in this codebase (`Action` enum + `authorize()`); UI-level hiding is explicitly documented as non-authoritative in CLAUDE.md/backlog 999.1 |
| Content-of-place screen (PLC-06) | API/Backend (subtree query joining devices+cartridges+printers by `place_id IN (descendants)`) | Database (recursive CTE) | Printers have no `place_id` of their own — they resolve through `devices.place_id` (printers extend devices 1:1, confirmed in V020) |

---

## Standard Stack

No new external dependencies. This phase is pure SQL schema + Rust service/repo code + a
hand-written Svelte component; UI-SPEC §16 confirms `ui/package.json` is not touched. All library
versions below are already pinned in the workspace `Cargo.toml` — verified by reading the file
directly (`[VERIFIED: workspace Cargo.toml]`), not assumed from training data.

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `rusqlite` | `0.38` (`bundled`, `serde_json`, `backup` features) `[VERIFIED: workspace Cargo.toml]` | SQLite driver | Already the project's fixed DB layer (CLAUDE.md); bundled feature ships a recent SQLite (≥3.44) — recursive CTEs (available since SQLite 3.8.3, 2014) and FTS5 (since 3.9.0) are comfortably supported `[CITED: sqlite.org/lang_with.html, sqlite.org/fts5.html]` |
| `refinery` | `0.9` (`rusqlite-bundled` feature) `[VERIFIED: workspace Cargo.toml]` | Embedded SQL migrations | Already fixed; next migration number is `V037` (last committed: `V036__org_settings_full_name.sql`) `[VERIFIED: migrations/ directory listing]` |

### Supporting

No new supporting libraries needed. Natural-sort comparison (D-05: "2" before "10") and cycle
detection are both implemented as plain Rust/SQL, not via a crate — see Don't Hand-Roll §
"natural sort" for the reasoning on why this specific problem is safe to hand-roll here.

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Adjacency list + recursive CTE | Closure table (`place_closure(ancestor_id, descendant_id, depth)`) | Faster descendant/ancestor queries at large scale (no recursive CTE per query), but requires maintaining edge-rebuild logic on every move (delete stale ancestor-pairs, insert new ones — O(depth × subtree size) per move) and an extra table to keep in sync. Not worth it at ~300 place rows; revisit only if a future scale (multi-tenant, thousands of buildings) is confirmed. |
| Adjacency list + recursive CTE | Materialized path column (`places.path_ids TEXT`, e.g. `/1/5/23/`) | Enables `WHERE path_ids LIKE 'X/%'` for O(1)-ish descendant lookups without a CTE, but a move requires rewriting `path_ids` for the moved node AND every descendant (a recursive `UPDATE`), which is more invasive than the recursive-CTE approach and duplicates data that the adjacency list already encodes. Consider only if profiling later shows the recursive CTE is a bottleneck (unlikely at this scale). |
| Live-JOIN place-text search | FTS5 trigger cascade (place rename → cascade-update `devices_fts`/`cartridges_fts` rows for the whole affected subtree) | Would give true FTS5 tokenized ranking for place text (matching today's `cartridges_fts.location` behavior), but the trigger must reconstruct the *old* full path of every affected descendant device/cartridge to issue FTS5's required `'delete'` command with matching old values — the `places` row has already been overwritten by the time an `AFTER UPDATE` trigger fires, making this proveably correct only with extra snapshot bookkeeping. Live JOIN sidesteps the whole problem by never caching place text. |
| Hand-rolled natural sort | `natord` crate | `natord` (last published ~2015, effectively unmaintained) does the same alternating-digit-run comparison this phase needs in ~15 lines. Given the narrow scope (Latin/Arabic digit runs only, no locale collation), hand-rolling avoids adding an unmaintained crate for a problem small enough to unit-test directly. |

**Installation:** none — no `Cargo.toml`/`package.json` changes required by this phase.

---

## Package Legitimacy Audit

Not applicable — this phase installs zero new packages (confirmed: `ui/package.json` untouched
per UI-SPEC §16 "Registry Safety"; workspace `Cargo.toml` unchanged, all needed crates already
present and pinned). The Package Legitimacy Gate protocol is skipped by design, not by omission.

**Packages removed due to slopcheck [SLOP] verdict:** none (no packages evaluated — none proposed)
**Packages flagged as suspicious [SUS]:** none

---

## Architecture Patterns

### System Architecture Diagram

```
┌─────────────────────────────┐        ┌──────────────────────────────┐
│  Tauri webview (desktop)    │        │  LAN browser (axum-served)    │
│  PlacesMasterDetail.svelte  │        │  same ui/dist bundle          │
│  PlacePicker.svelte         │        │                                │
└──────────────┬───────────────┘        └───────────────┬────────────────┘
               │ tauri invoke                              │ POST /api/v1/places_*
               ▼                                            ▼
        ┌──────────────────────────────────────────────────────────┐
        │        trackly-app: thin adapters (tauri_cmds::places /   │
        │        http::places) — identical DTOs, same PlaceService  │
        └───────────────────────────┬─────────────────────────────┘
                                     ▼
        ┌──────────────────────────────────────────────────────────┐
        │  PlaceService (trackly-app::services::place_service)      │
        │  - authorize(&identity, &Action::{Read,Mutate}Places)     │
        │  - cycle check (recursive CTE) before move                │
        │  - subtree stats (counts) before delete/move confirm      │
        │  - single-writer: all mutations go through the writer     │
        │    task's mpsc queue (existing pattern, unchanged)         │
        └───────────────────────────┬─────────────────────────────┘
                                     ▼
        ┌──────────────────────────────────────────────────────────┐
        │  PlaceRepository port (trackly-core::ports::places)        │
        │  SqlitePlaceRepository (trackly-infra::repos::places_sqlite)│
        │  - places table (adjacency list)                          │
        │  - place_full_paths view (recursive CTE)                  │
        │  - devices.place_id / cartridges.place_id FK               │
        └───────────────────────────┬─────────────────────────────┘
                                     ▼
                              SQLite (WAL, single writer conn +
                              reader pool — unchanged pattern)

Search path (PLC-05):
  query term ──▶ FTS5 MATCH on devices_fts/cartridges_fts (intrinsic fields:
                 name/inventory_number/serial_number/model/code/holder_name)
             ──▶ UNION live JOIN: devices/cartridges ⋈ place_full_paths
                 WHERE full_path LIKE '%term%' (case-folded in Rust, not SQL —
                 see Common Pitfalls §"Cyrillic LIKE")
             ──▶ merged, deduped in PlaceService/relevant entity service
```

### Recommended Project Structure

Mirrors the existing five-file-per-entity pattern exactly (verified against `devices`/`acts`/
`cartridges`/`printers`):

```
crates/trackly-core/src/
├── domain/places.rs          # PlaceRow, PlaceNew, PlacePatch, PlaceKind enum, PlaceFilter
├── ports/places.rs           # PlaceRepository trait (create/get/list_children/list_tree/
│                              #   rename/move/archive/unarchive/delete_hard/subtree_stats)
├── auth.rs                   # + Action::ReadPlaces, Action::MutatePlaces (see Common Pitfalls)

crates/trackly-infra/src/repos/
└── places_sqlite.rs          # SqlitePlaceRepository — recursive CTE queries live here

crates/trackly-app/src/
├── dto/place.rs               # PlaceDto, PlaceTreeNodeDto (with subtree count), PlacePathDto
├── services/place_service.rs  # PlaceService — authorize + cycle-check + subtree-stats + writer
├── tauri_cmds/places.rs       # #[tauri::command] thin adapters
├── http/places.rs             # axum thin adapters, /api/v1/places_*
└── specta_export.rs           # + register every places_* command (existing checklist habit)

migrations/
├── V037__places.sql           # places table + indexes + place_full_paths view
├── V038__places_migrate_devices_acts_cartridges.sql   # place_id columns, drop locations
```

### Pattern 1: `places` table — adjacency list

```sql
-- V037: places tree (adjacency list). No data migration from `locations`
-- is required (confirmed: app not in production, REQUIREMENTS.md Out of Scope).

CREATE TABLE places (
  id              INTEGER PRIMARY KEY AUTOINCREMENT,
  parent_id       INTEGER NULL REFERENCES places(id) ON DELETE RESTRICT,
  kind            TEXT    NOT NULL
                          CHECK (kind IN ('territory','zone','building','floor','room','outdoor')),
  name            TEXT    NOT NULL,
  level           INTEGER NULL,               -- floors only; NULL for other kinds (PLC-02: 0 and negatives OK)
  is_storage      INTEGER NOT NULL DEFAULT 0,  -- 0/1 boolean (D-08)
  sort_order      INTEGER NULL,                -- manual override (D-05); NULL = automatic
  archived_at_utc INTEGER NULL,                -- D-15: archived (hidden from PlacePicker), distinct from soft-delete
  notes           TEXT    NULL,
  created_at_utc  INTEGER NOT NULL,
  updated_at_utc  INTEGER NOT NULL,
  deleted_at_utc  INTEGER NULL,                -- hard-delete only allowed when empty (D-14), still soft-delete column per standard4
  version         INTEGER NOT NULL DEFAULT 1
);

-- D-04: unique name among live siblings. COALESCE(parent_id, 0) mirrors the
-- existing idx_acts_number_sub_unique pattern for nullable-column uniqueness.
CREATE UNIQUE INDEX idx_places_parent_name_unique
  ON places(COALESCE(parent_id, 0), name)
  WHERE deleted_at_utc IS NULL;

CREATE INDEX idx_places_parent ON places(parent_id) WHERE deleted_at_utc IS NULL;

-- Full path per node, from root to leaf, ' / '-joined (PLC-03 canonical format).
-- Recomputed on every query — cheap at ~300 rows, and crucially NEVER stale
-- (no cache to invalidate — this is what makes D-29/PLC-05 "instant, no
-- reindex" true by construction, not by trigger correctness).
CREATE VIEW place_full_paths AS
WITH RECURSIVE path_cte(id, path, parent_id) AS (
  SELECT id, name, parent_id FROM places WHERE deleted_at_utc IS NULL
  UNION ALL
  SELECT pc.id, p.name || ' / ' || pc.path, p.parent_id
  FROM path_cte pc
  JOIN places p ON p.id = pc.parent_id
  WHERE p.deleted_at_utc IS NULL
)
SELECT id AS place_id, path AS full_path
FROM path_cte
WHERE parent_id IS NULL;

PRAGMA user_version = 37;
```

```sql
-- V038: point devices/cartridges/acts at places, drop `locations`.
-- Schema-only migration — no data preserved (confirmed decision, see
-- User Constraints > Deferred Ideas).

ALTER TABLE devices  ADD COLUMN place_id INTEGER NULL REFERENCES places(id) ON DELETE RESTRICT;
ALTER TABLE cartridges ADD COLUMN place_id INTEGER NULL REFERENCES places(id) ON DELETE RESTRICT;
ALTER TABLE acts ADD COLUMN place_id INTEGER NULL REFERENCES places(id) ON DELETE RESTRICT;
ALTER TABLE acts ADD COLUMN bulk_place_id INTEGER NULL REFERENCES places(id) ON DELETE RESTRICT;
ALTER TABLE acts ADD COLUMN place_path_snapshot TEXT NULL; -- D-16
ALTER TABLE act_items ADD COLUMN place_id_override INTEGER NULL REFERENCES places(id) ON DELETE RESTRICT;

-- Old columns dropped. SQLite ALTER TABLE DROP COLUMN is supported since
-- 3.35.0 (2021) — comfortably inside rusqlite 0.38's bundled SQLite version.
ALTER TABLE devices DROP COLUMN location_id;
ALTER TABLE cartridges DROP COLUMN location;
ALTER TABLE acts DROP COLUMN location_id;
-- NOTE: acts.bulk_location_id / ActReturnItem.location_id_override are NOT
-- schema columns today (verified: grep found zero migration hits) — they are
-- Rust-only domain struct fields resolved against `locations` at write time.
-- Only the Rust struct fields need renaming; no DROP COLUMN needed for them.

DROP INDEX idx_devices_location;
DROP INDEX idx_devices_autocomplete_name_location;
CREATE INDEX idx_devices_place ON devices(place_id) WHERE deleted_at_utc IS NULL AND place_id IS NOT NULL;
CREATE INDEX idx_cartridges_place ON cartridges(place_id) WHERE deleted_at_utc IS NULL AND place_id IS NOT NULL;

DROP TABLE locations;  -- PLC-04

-- devices_fts / cartridges_fts: no schema change needed here. Per Common
-- Pitfalls, place text is NOT added as an FTS5 column (live JOIN instead,
-- see Pattern 3). cartridges_fts DOES lose its old `location` FTS column
-- content (cartridges.location column is gone) — rebuild via
-- INSERT INTO cartridges_fts(cartridges_fts) VALUES('rebuild') after dropping
-- the column reference in the trigger bodies (V016 triggers must be redefined
-- without the `location` column — DROP TRIGGER + CREATE TRIGGER, refinery
-- migrations can't ALTER a trigger).

PRAGMA user_version = 38;
```

### Pattern 2: subtree contents + counts (PLC-06, reused by D-14/D-21/D-25/D-28)

One reusable query answers: "all descendant place IDs of X (including X itself)". Every other
subtree operation is built on top of it.

```sql
-- Descendant place IDs (including self) of :root_id.
WITH RECURSIVE subtree(id) AS (
  SELECT id FROM places WHERE id = :root_id AND deleted_at_utc IS NULL
  UNION ALL
  SELECT p.id FROM places p
  JOIN subtree s ON p.parent_id = s.id
  WHERE p.deleted_at_utc IS NULL
)
SELECT id FROM subtree;
```

```sql
-- PLC-06 content screen ("Все", nested-by-default per D-24):
SELECT 'device' AS kind, d.id, d.name, d.inventory_number, d.serial_number,
       pfp.full_path, ds.name AS status_name
FROM devices d
JOIN place_full_paths pfp ON pfp.place_id = d.place_id
LEFT JOIN device_statuses ds ON ds.id = d.status_id
WHERE d.deleted_at_utc IS NULL
  AND d.place_id IN (SELECT id FROM subtree)   -- CTE above, or just = :root_id when "Только здесь"
UNION ALL
SELECT 'cartridge', c.id, m.brand || ' ' || m.model, NULL, c.code,
       pfp.full_path, cs.name
FROM cartridges c
JOIN place_full_paths pfp ON pfp.place_id = c.place_id
JOIN cartridge_models m ON m.id = c.model_id
LEFT JOIN cartridge_statuses cs ON cs.id = c.status_id
WHERE c.deleted_at_utc IS NULL AND c.place_id IN (SELECT id FROM subtree)
-- printers UNIONed via devices with type_id = Принтер (printers has no
-- place_id of its own — it extends devices 1:1, confirmed V020__printers.sql)
```

```sql
-- Subtree stats for D-14 error / D-21 consequences preview / D-25 tree counters:
-- (child_places, devices, cartridges) counts under :root_id, INCLUSIVE of root.
SELECT
  (SELECT COUNT(*) FROM places WHERE parent_id = :root_id AND deleted_at_utc IS NULL) AS direct_children,
  (SELECT COUNT(*) FROM places WHERE id IN (subtree) AND id != :root_id AND deleted_at_utc IS NULL) AS nested_places,
  (SELECT COUNT(*) FROM devices WHERE place_id IN (subtree) AND deleted_at_utc IS NULL) AS device_count,
  (SELECT COUNT(*) FROM cartridges WHERE place_id IN (subtree) AND deleted_at_utc IS NULL) AS cartridge_count;
```

D-25's per-node tree counter ("2 этаж · 47") is this same subtree-stats query run once per
visible tree node — at ~300 nodes and a handful of devices this is trivial; do not
pre-aggregate/cache it (same "never denormalize what you don't need to" principle as §1.5).

### Pattern 3: cycle prevention on move (Claude's Discretion)

```sql
-- Reject if :new_parent_id is :moving_id itself, or a descendant of :moving_id.
-- Run inside the writer's transaction immediately before the UPDATE.
WITH RECURSIVE ancestors(id) AS (
  SELECT parent_id FROM places WHERE id = :new_parent_id
  UNION ALL
  SELECT p.parent_id FROM places p JOIN ancestors a ON p.id = a.id WHERE p.parent_id IS NOT NULL
)
SELECT 1 WHERE :new_parent_id = :moving_id
UNION ALL
SELECT 1 FROM ancestors WHERE id = :moving_id;
-- Any row returned → reject with "Нельзя переместить место внутрь самого
-- себя или своего вложенного места." (copy locked in 39-UI-SPEC.md §14.3)
```

Because this codebase uses a single dedicated writer task (mpsc queue → one `rusqlite::Connection`
— CLAUDE.md's SQLite WAL + single-writer pattern), there is no concurrent-writer race window
between the cycle check and the actual `UPDATE places SET parent_id = ...`; both happen inside the
same job on the same connection. No `SELECT ... FOR UPDATE`-style locking is needed (SQLite
doesn't have it anyway).

### Pattern 4: natural sibling sort (D-05)

Do this in Rust after fetching children, not in SQL. SQLite has no built-in natural-sort
collation; hand-rolling a `CASE`-heavy `ORDER BY` expression for "compare numeric runs as
integers" is far more error-prone than a ~20-line Rust comparator, and the fetch size per
parent is capped at ≤20 (confirmed real scale) — sorting in memory is free.

```rust
// Sketch — sort_order wins if set, else level (floors), else natural-name compare.
fn sibling_cmp(a: &PlaceRow, b: &PlaceRow) -> std::cmp::Ordering {
    if let (Some(sa), Some(sb)) = (a.sort_order, b.sort_order) {
        return sa.cmp(&sb);
    }
    if let (Some(la), Some(lb)) = (a.level, b.level) {
        return la.cmp(&lb);
    }
    natural_name_cmp(&a.name, &b.name)
}

/// Splits into alternating digit/non-digit runs, compares digit runs as u64.
/// Narrow scope (ASCII digits only — place names like "214", "Каб. 3А") means
/// this does not need locale-aware collation; a single pure function is
/// simpler and more testable than pulling in `natord` (see Alternatives Considered).
fn natural_name_cmp(a: &str, b: &str) -> std::cmp::Ordering { /* ... */ }
```

### Anti-Patterns to Avoid

- **Denormalizing place path text into `devices`/`cartridges` rows or FTS5 tables.** This is the
  single biggest trap in this phase — see Common Pitfalls §1. Any cached copy of "current full
  path" needs a cascade-update mechanism on every rename/move, and that mechanism is genuinely
  hard to get right with SQLite triggers (the trigger fires on `places` but needs the *old*
  resolved path of every descendant device, which requires reconstructing pre-image ancestor
  chains). Resolve live instead.
- **Reusing the `MutateDevices`/`ReadData` `Action::Admin|Manager` bucket for places.** D-20
  explicitly makes place *mutation* Admin-only (Manager reads but cannot edit) — this is a
  *different* permission bucket than every other entity in this codebase (devices/acts/
  cartridges/printers are all `Admin|Manager` for mutation). Copy-pasting the existing pattern
  will silently give Manager edit rights that D-20 forbids. See Common Pitfalls §"Action enum".
- **Enforcing "only leaf nodes can hold a place assignment" anywhere** — D-06 explicitly forbids
  this; a device can point at a building-level node with no room.
- **Auto-creating a place row when a free-text name doesn't match** — this is exactly what the
  current `resolve_location_id_in_tx` (`crates/trackly-infra/src/repos/devices_sqlite.rs:145`)
  does today (`INSERT OR IGNORE INTO locations ... ` on ANY typed string), and it is the pattern
  being explicitly replaced by D-18 (place creation from `PlacePicker` is an explicit,
  Admin-gated action with a confirmation label, never an implicit side effect of typing a
  string). Every call site touching `resolve_location_id_in_tx` needs its calling convention
  changed from "pass a name, get an id, auto-create if missing" to "pass a validated `place_id`,
  reject if it doesn't resolve or is archived."

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Recursive tree traversal (ancestors, descendants, full path) | Loop-based Rust code walking `parent_id` row by row with N queries | SQLite `WITH RECURSIVE` CTE (single query) | N+1 query pattern for a tree of unknown depth is exactly the kind of bug that "works in dev with 3 test rows, degrades in prod" — SQLite's recursive CTE support is mature and does this in one round-trip |
| FTS5 sync-on-write | Custom string-matching / manual index table | Existing external-content FTS5 + `AFTER INSERT/UPDATE/DELETE` triggers (already the pattern for `devices_fts`/`cartridges_fts`, `V013`/`V016`) — keep using it for the fields that don't change via cascading edits (name/inv/serial/model/code/holder) | The project already solved this correctly; place text specifically should NOT go through this mechanism (see Anti-Patterns) but everything else should stay exactly as-is |
| Optimistic-lock CAS on place mutation | Ad-hoc version checks | Reuse the exact `expected_version` + `WHERE version = ?` pattern already used by `ActPatch`/`DevicePatch` | Established, tested pattern in this codebase; no reason to invent a new one for places |

**Key insight:** the two things worth NOT hand-rolling here (recursive traversal, FTS sync) are
both things SQLite/the existing codebase already does well. The two things worth explicitly
NOT reaching for a library (natural sort, cycle detection) are narrow enough in scope that a
hand-rolled function is simpler, more auditable, and avoids a new dependency in a privacy/
portable-sensitive public repo.

---

## Common Pitfalls

### Pitfall 1: FTS5 cascade-on-rename is a correctness trap — avoid it structurally

**What goes wrong:** A tempting design is "add a `place_path` column to `devices_fts`/
`cartridges_fts`, sync it via triggers like every other FTS column." But FTS5's external-content
mode requires the exact *old* column values to issue a `'delete'` command
(`INSERT INTO fts(fts, rowid, col...) VALUES ('delete', old_rowid, old_col_values...)`). When a
**place** is renamed or moved, the FTS rows that need updating live on **devices/cartridges**,
not on the `places` row the trigger fires on — and by the time an `AFTER UPDATE ON places`
trigger runs, the `places` table already reflects the NEW state, so recomputing what the *old*
path text was for every affected descendant device requires reconstructing a stale ancestor
chain that no longer exists in the table.

**Why it happens:** External-content FTS5's `'delete'` command isn't "delete this rowid" — it's
"remove exactly these previously-indexed token values," which SQLite can't infer on its own.
This is a documented FTS5 requirement, not a Trackly-specific bug.

**How to avoid:** Don't cache place text in FTS5 at all (§1.5, Pattern 1). Resolve place text via
a live `JOIN place_full_paths` at search time instead. This makes D-29/PLC-05's "no manual
reindex ever" trivially true, because there's no index to go stale.

**Warning signs:** If a plan/task proposes "add place_path column to devices_fts + trigger on
places table to cascade the update," that's this trap — flag it in review.

### Pitfall 2: SQLite's `LIKE` does not case-fold Cyrillic

**What goes wrong:** PLC-03 and the `PlacePicker` search mode (`39-UI-SPEC.md` §10.3) both need
substring search over Cyrillic place names ("здание" should match "Здание А"). SQLite's built-in
`LIKE` operator is documented to perform case-insensitive matching **only for ASCII characters**
by default — Cyrillic (and any non-ASCII script) is compared case-sensitively unless the ICU
extension is loaded `[CITED: sqlite.org/lang_expr.html#the_like_glob_regexp_and_match_operators]`.
The bundled `rusqlite` build does not include ICU. This is a genuine, easy-to-miss gap: a plan
that does `WHERE full_path LIKE '%' || :query || '%'` in SQL will silently fail to match
differently-cased Cyrillic queries.

**Why it happens:** SQLite's LIKE case-folding table is hardcoded to ASCII for size/performance
reasons; this is long-standing, documented SQLite behavior, not a version regression.

**How to avoid:** At the confirmed scale (~300 place rows), fetch the small candidate set (all
live, non-archived places' full paths — a single `SELECT * FROM place_full_paths` costs nothing
at this size) into Rust and filter with `.to_lowercase()` substring matching, which IS
Unicode-aware in Rust's standard library. Do the substring match in the service layer, not SQL.
Note that the existing FTS5 tables use the `unicode61 remove_diacritics 2` tokenizer, which DOES
correctly case-fold Cyrillic for `MATCH` queries — so this pitfall applies specifically to plain
`LIKE`/`GLOB` on non-FTS columns (i.e., the `place_full_paths` view), not to the existing
`devices_fts`/`cartridges_fts` intrinsic-field search.

**Warning signs:** A manual test searching "здание" (lowercase) fails to find "Здание А"
(capitalized) in `PlacePicker`'s search mode.

### Pitfall 3: this codebase's "Admin|Manager can mutate" pattern does NOT apply to places

**What goes wrong:** Every existing `Action` variant that gates entity mutation in
`crates/trackly-core/src/auth.rs` (`MutateDevices`, `MutateActs`, `MutateCartridges`,
`MutatePrinters`) allows **both** Admin and Manager. D-20 for places is different: **Manager can
read but not mutate** — place editing is Admin-only, joining the `ManageUsers`/`ManageSettings`
bucket instead. Copy-pasting the familiar `Admin | Manager` match arm for a new
`Action::MutatePlaces` variant will silently violate D-20.

**Why it happens:** Reflexive pattern-matching against the nearest similar code (`MutateDevices`)
without re-reading the phase's specific access decision.

**How to avoid:** add exactly two new `Action` variants:
```rust
/// Просмотр дерева мест и содержимого узла. Admin | Manager (D-20).
ReadPlaces,
/// Создание/переименование/перемещение/архивация/удаление места. Admin ONLY (D-20).
MutatePlaces,
```
`MutatePlaces` goes in the `Action::ManageUsers | Action::ManageSettings => Admin-only` arm, NOT
the `Action::MutateDevices | ... => Admin|Manager` arm. `ReadPlaces` goes in the same bucket as
`ReadData`/`ReadPrinters` (Admin|Manager).

**Warning signs:** A Manager-role test can successfully rename/move/archive/delete a place — this
should return `AppError::Forbidden`.

### Pitfall 4: `resolve_location_id_in_tx`'s implicit auto-create is a silent trap for every migrated call site

**What goes wrong:** Today, `DeviceRepository::resolve_location_id_in_tx` (used by
`act_service.rs` at 6+ call sites: create, update, return bulk/per-item, and by
`DeviceService`/`DeviceAutocompleteField.svelte`'s `location_name`-driven flows) silently
`INSERT OR IGNORE`s a brand-new `locations` row for **any** typed string, with no role check.
If a plan simply renames this helper to operate on `places` without changing its calling
contract, it will resurrect exactly the unrestricted auto-create behavior that D-18 explicitly
scopes to Admin-only, explicit-confirmation UI action.

**Why it happens:** the helper is deeply embedded (6+ call sites across `act_service.rs`) and the
"just rename the column" refactor path is tempting because it minimizes the diff.

**How to avoid:** every one of these call sites must switch from "pass a name string, get-or-
create an id" to "pass a validated `place_id: Option<i64>` that the caller already resolved
through `PlacePicker` (which itself calls the Admin-gated create endpoint explicitly, per D-18)."
The service layer should reject an act/device/cartridge write that references a `place_id` which
doesn't exist or is soft-deleted — never silently create one.

**Warning signs:** grep for `resolve_location_id_in_tx` in the diff — every call site must be
converted, not one preserved "for compatibility."

### Pitfall 5: `DB-backed templates upgrade trap` applies to `act.location_name` → `act.place_path`

**What goes wrong:** `act_handover.minijinja`'s contract (`act.location_name`,
`return.location_default`, per the file header comment) will be renamed to something like
`act.place_path`. This project's act templates live **in the DB**, not just as the bundled
`.minijinja` file (documented project trap, confirmed in canonical refs and prior project
memory: "DB-backed templates upgrade trap" / `260704-uw3` fix) — editing the bundled file alone
does NOT propagate to existing DBs unless the seed's auto-upgrade-untouched-defaults logic (in
`template_service.rs`, added by that prior fix) picks up the changed variable name too.

**Why it happens:** the auto-upgrade logic only fires for templates whose stored body still
matches the *previous* bundled default; user-customized templates are correctly left alone — but
if the template CONTRACT (available variable names) changes and a customized template still
references the removed `act.location_name`, rendering it will now silently produce an empty
field (minijinja's `default("—", true)` filter swallows undefined variables) instead of erroring.

**How to avoid:** keep `act.location_name` as a defined-but-deprecated alias in the render
context for at least one release (mapped to the same resolved path string as `act.place_path`),
OR treat this as a breaking template-contract change and explicitly document it, since this repo
has zero real customized templates to protect (confirmed: repo is pre-production, no real org
data). Given the "data migration not required" decision already established for this phase, the
simpler and consistent choice is: **rename the contract outright** (`act.place_path`), update the
bundled `.minijinja`, and rely on the existing seed auto-upgrade path — but explicitly test that
an *unmodified* seeded template on a pre-Phase-39 test DB re-renders correctly post-migration
(this is exactly what the `260704-uw3` regression tests already cover the pattern for; add an
equivalent test for this rename).

---

## Code Examples

### Full-path resolution for a single node (breadcrumbs, print snapshot)

```sql
-- Source: derived from place_full_paths view (Pattern 1) — single-row lookup.
SELECT full_path FROM place_full_paths WHERE place_id = ?1;
```

### PlaceRepository port sketch (mirrors `DeviceRepository`, verified pattern)

```rust
// Source: pattern verified against crates/trackly-core/src/ports/devices.rs
pub trait PlaceRepository {
    type Conn;

    fn create(&self, conn: &mut Self::Conn, new: &PlaceNew, now_utc: i64) -> Result<i64, AppError>;
    fn get(&self, conn: &Self::Conn, id: i64) -> Result<PlaceRow, AppError>;
    /// Direct children only, in DB order (caller applies natural sort — Pattern 4).
    fn list_children(&self, conn: &Self::Conn, parent_id: Option<i64>) -> Result<Vec<PlaceRow>, AppError>;
    /// Whole tree, flattened, for initial PlacePicker/tree-view hydration.
    fn list_all(&self, conn: &Self::Conn, include_archived: bool) -> Result<Vec<PlaceRow>, AppError>;
    fn rename(&self, conn: &mut Self::Conn, id: i64, name: &str, version: i64, now_utc: i64) -> Result<PlaceRow, AppError>;
    /// Runs the Pattern 3 cycle check internally before the UPDATE.
    fn move_node(&self, conn: &mut Self::Conn, id: i64, new_parent_id: Option<i64>, version: i64, now_utc: i64) -> Result<PlaceRow, AppError>;
    fn archive(&self, conn: &mut Self::Conn, id: i64, version: i64, now_utc: i64) -> Result<(), AppError>;
    fn unarchive(&self, conn: &mut Self::Conn, id: i64, version: i64, now_utc: i64) -> Result<(), AppError>;
    /// AppError::Conflict with counts if not empty (D-14) — service layer formats the message.
    fn delete_hard(&self, conn: &mut Self::Conn, id: i64, version: i64) -> Result<(), AppError>;
    /// Pattern 2 — shared by D-14/D-21/D-25/D-28/PLC-06.
    fn subtree_stats(&self, conn: &Self::Conn, root_id: i64) -> Result<SubtreeStats, AppError>;
    fn full_path(&self, conn: &Self::Conn, id: i64) -> Result<String, AppError>;
}
```

### `Action` enum additions (see Common Pitfalls §3)

```rust
// Source: crates/trackly-core/src/auth.rs — new variants + updated match arms.
pub enum Action {
    // ... existing variants unchanged ...
    /// Просмотр дерева мест и содержимого узла (PLC-06). Admin | Manager.
    ReadPlaces,
    /// Создание/переименование/перемещение/архивация/удаление места. Admin ONLY (D-20).
    MutatePlaces,
}

pub fn authorize(identity: &Identity, action: &Action) -> Result<(), AppError> {
    let allowed = match action {
        Action::ManageUsers | Action::ManageSettings | Action::MutatePlaces => {
            matches!(identity.role, Role::Admin)
        }
        Action::MutateDevices
        | Action::MutateActs
        | Action::MutateCartridges
        | Action::MutatePrinters
        | Action::TransitionRequests
        | Action::ReadPrinters
        | Action::ReadData
        | Action::ReadPlaces
        | Action::DeleteRequests => {
            matches!(identity.role, Role::Admin | Role::Manager)
        }
        Action::CreateRequest | Action::ReadRequests | Action::CancelOwnRequest => true,
    };
    // ...
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|---------------|--------|
| `locations` flat table, `name UNIQUE`, freeform `kind` text | `places` tree, closed 6-value `kind` enum, `parent_id` adjacency | This phase (V037/V038) | Every read/write path touching location must switch from string/id-lookup to `place_id` FK + tree-aware queries |
| `devices.location_id` FK / `cartridges.location` freeform TEXT — two different storage strategies for the "same" concept | Single `place_id` FK on both | This phase | `cartridges_fts`'s `location` FTS column disappears entirely (superseded by live JOIN, Pattern 1) |
| Implicit get-or-create-by-name (`resolve_location_id_in_tx`) on every act/device write | Explicit, Admin-gated place creation via `PlacePicker` (D-18); all other writers must reference an existing `place_id` | This phase | See Common Pitfalls §4 — every call site of the old helper needs its calling contract changed, not just its target table |

**Deprecated/outdated:**
- `LocationAutocomplete.svelte` — replaced entirely by `PlacePicker.svelte` (D-17, UI-SPEC §6.3).
- `AutocompleteField::Location` / `is_location()` in `crates/trackly-core/src/domain/devices.rs`
  — the whole per-field-autocomplete mechanism for location becomes unnecessary once `PlacePicker`
  talks directly to the places tree/search endpoints instead of `devices_autocomplete`.
- `locations_autocomplete` Tauri command + `/api/v1/locations_autocomplete` HTTP route — both
  removed, replaced by new `places_*` endpoints.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | SQLite's `LIKE`/`GLOB` case-folding is ASCII-only by default (no ICU loaded in this build) | Common Pitfalls §2 | If wrong (e.g., bundled SQLite somehow includes case-folding for Cyrillic), the Rust-side lowercase-filter workaround is merely redundant, not harmful — low risk either way. `[CITED: SQLite official docs describe ASCII-only default LIKE case-folding; not independently re-verified against this exact bundled build's compile flags this session]` |
| A2 | `natord`-class crates are effectively unmaintained / narrow-scope enough to hand-roll safely | Standard Stack > Alternatives Considered, Don't Hand-Roll | If the planner disagrees and scope grows (e.g., future locale-aware sorting needs), a maintained crate could be substituted later without schema impact — `sort_order`/`level`/`name` columns are agnostic to how the comparator is implemented |
| A3 | SQLite `ALTER TABLE ... DROP COLUMN` is supported by the bundled SQLite version (added in SQLite 3.35.0, 2021) | Architecture Patterns > V038 migration | If the bundled SQLite in `rusqlite 0.38` is somehow older than 3.35 (very unlikely — verify with `SELECT sqlite_version()` in a throwaway test before writing the migration), the migration would need a table-rebuild (`CREATE TABLE new; INSERT; DROP old; ALTER RENAME`) instead of `DROP COLUMN` |

**If this table is empty:** N/A — see rows above. All other claims in this research are either
`[VERIFIED]` against the repo directly (schema, code, config files read in this session) or
`[CITED]` against SQLite's own documented, long-standing behavior (recursive CTE support, FTS5
external-content delete semantics), which is stable/foundational SQL-engine behavior rather than
a fast-moving library API.

---

## Open Questions

1. **Should `place_id_override` on `act_items` (return-time per-item place override) also carry
   its own snapshot text, or does the act-level `place_path_snapshot` suffice for D-16?**
   - What we know: today `act.location_name` is a single header-level template variable (verified
     in `act_handover.minijinja`'s documented contract); per-item place isn't rendered anywhere.
   - What's unclear: whether a future print-layout change (outside this phase's scope, Фазы
     34–36 own template typography) will need per-item place display for split returns to
     different storage locations.
   - Recommendation: ship act-level `place_path_snapshot` only (matches current template
     contract, matches current `condition_at_time`/`complectation_at_time` being the ONLY
     per-item snapshot fields already in `act_items`); this doesn't block adding a per-item
     snapshot column later since it's purely additive.

2. **Does `is_storage` need a partial index for the D-11.2 "быстрый фильтр «на складе»"
   report/filter query?**
   - What we know: `is_storage` sits on `places`, and the filter needs "devices whose place (or
     any ancestor place?) is storage" — D-08/D-09 examples show storage can be a leaf node
     (shelf inside a room) OR a whole room; D-11.2's wording doesn't specify whether storage
     status should be inherited from an ancestor.
   - What's unclear: whether "склад" filter means "device's own place_id has `is_storage=1`" or
     "device's place OR any ancestor has `is_storage=1`" (e.g., a device sitting directly in a
     `is_storage` building without a room). Given D-06 (bindings at any level are valid) this
     ambiguity is real.
   - Recommendation: planner should get an explicit answer during plan-checking or default to
     "own place_id only" (simpler, matches the literal reading of D-08's examples) and document
     the choice; an ancestor-inheriting version can be added later as `WHERE place_id IN (SELECT
     id FROM subtree_of_any_storage_ancestor)` without a schema change.

## Environment Availability

Skipped — this phase has no external tool/service/runtime dependencies beyond the already-fixed
project stack (SQLite via bundled `rusqlite`, already verified present via `Cargo.toml`). No new
CLI tools, databases, or network services are introduced.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test` (Rust integration tests, workspace-standard) + `svelte-check`/`eslint`/`pnpm build` (frontend static gates) `[VERIFIED: existing test suite layout]` |
| Config file | none dedicated — standard `Cargo.toml` `[[test]]` auto-discovery in `crates/trackly-app/tests/` and `crates/trackly-infra/tests/` |
| Quick run command | `cargo test -p trackly-infra --test migration_idempotency` (schema-only smoke, <5s) |
| Full suite command | `cargo test -p trackly-app -- --skip login_remember_persistent_cookie --test-threads=1` (see Known Test Constraints below) |

### Known Test Constraints (repo-specific — verified this session)

- **`cargo test` must NEVER be run concurrently** — two invocations contend on `target/`'s lock
  and manifest as a multi-minute apparent hang (documented project pitfall).
- **`login_remember_persistent_cookie` (in `crates/trackly-app/tests/auth_remember_cookie.rs:71`)
  is a pre-existing hanging test**, unrelated to this phase. Any full-package run MUST pass
  `-- --skip login_remember_persistent_cookie`, and this applies to `-p trackly-app` runs
  specifically (not just `--workspace`).
- **CI-mocked env required for `trackly-app` tests touching AD/SNMP:**
  `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1` (per `devices_csv_import.rs:21` and prior project
  memory) — irrelevant to places tests directly, but any full-suite run of `trackly-app` needs
  these set.
- **`ui/dist` must be rebuilt** (`pnpm --dir ui build`) after any frontend change before LAN-
  browser verification — `cargo tauri dev` only HMRs the desktop webview, not the axum-served
  bundle (documented project pitfall, relevant since this phase adds a new route `/places`).

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| PLC-01 | Build tree, rename, move without losing device FK bindings | integration (`trackly-core`/`trackly-infra`) | `cargo test -p trackly-infra --test places_crud -x` | ❌ Wave 0 |
| PLC-01 | Cycle rejection on move | unit (`trackly-infra` repo test or `trackly-app` service test) | `cargo test -p trackly-app --test places_move_cycle` | ❌ Wave 0 |
| PLC-02 | Floor `level` accepts 0 and negatives, siblings sort by level not name | unit (Rust comparator test, Pattern 4) | `cargo test -p trackly-core --lib natural_sort` | ❌ Wave 0 |
| PLC-03 | Full-path search returns correct matches (incl. Cyrillic case-insensitivity, Pitfall 2) | integration | `cargo test -p trackly-app --test places_search` | ❌ Wave 0 |
| PLC-04 | `locations` table and all `location_id`/`location` columns fully removed | integration (schema assertion) | `cargo test -p trackly-infra --test migration_idempotency` (extend existing test to assert absence) | ✅ (extend existing) |
| PLC-05 | Rename/move instantly reflected in search/lists, no manual step | integration | `cargo test -p trackly-app --test places_search_live_reflect` | ❌ Wave 0 |
| PLC-06 | Content-of-place screen returns nested items by default, toggle limits to direct only | integration | `cargo test -p trackly-app --test places_contents` | ❌ Wave 0 |
| D-14 | Delete blocked with exact counts when non-empty | integration | `cargo test -p trackly-app --test places_delete_blocked` | ❌ Wave 0 |
| D-16 | Act stores `place_id` + frozen path snapshot; rename doesn't retroactively change a printed act | integration (extends existing `acts_*` test files) | `cargo test -p trackly-app --test acts_place_snapshot` | ❌ Wave 0 |
| D-20 | Manager blocked from `MutatePlaces`, Employee blocked from `ReadPlaces` on both transports | integration (mirrors existing `role_endpoint_matrix.rs`) | `cargo test -p trackly-app --test role_endpoint_matrix` (extend existing) | ✅ (extend existing) |
| UI (all) | `PlaceTree`/`PlacePicker` keyboard + ARIA contract (§8.5/§10.5 of UI-SPEC) | manual-only (a11y interaction, per project's documented "synthetic harness ≠ real verification" constraint) | N/A — manual UAT in running app (Tauri + LAN browser) | N/A |

### Sampling Rate

- **Per task commit:** targeted `cargo test -p trackly-app --test <new_test_file>` for the file
  just touched (fast, <30s).
- **Per wave merge:** `cargo test -p trackly-infra --test migration_idempotency` +
  `cargo test -p trackly-app -- --skip login_remember_persistent_cookie --test-threads=1` (full
  package, respecting the known hang skip).
- **Phase gate:** full suite green (with the documented skip) + `cargo clippy --all-targets -- -D
  warnings` + `svelte-check` 0 errors + `pnpm --dir ui build` succeeds, before `/gsd-verify-work`.

### Wave 0 Gaps

- [ ] `crates/trackly-infra/tests/places_crud.rs` — covers PLC-01 (create/rename/move/archive/
      delete, uniqueness constraint, FK survival)
- [ ] `crates/trackly-app/tests/places_move_cycle.rs` — covers cycle-rejection (Claude's Discretion item)
- [ ] `crates/trackly-core/src/domain/places.rs` unit tests — covers PLC-02 natural-sort comparator
- [ ] `crates/trackly-app/tests/places_search.rs` — covers PLC-03 (incl. a Cyrillic case-fold
      regression test per Pitfall 2 — this is the single highest-value new test in this phase,
      since it's the one most likely to silently pass in English/ASCII testing and fail in the
      RU-only production UI)
- [ ] `crates/trackly-app/tests/places_contents.rs` — covers PLC-06 nested-vs-direct toggle
- [ ] `crates/trackly-app/tests/places_delete_blocked.rs` — covers D-14 exact-count error
- [ ] `crates/trackly-app/tests/acts_place_snapshot.rs` — covers D-16 (extend existing
      `acts_search.rs`/`acts_update.rs` patterns rather than a wholly new file, if simpler)
- [ ] Extend existing `crates/trackly-app/tests/role_endpoint_matrix.rs` — covers D-20's
      non-standard Admin-only-mutate/Admin-Manager-read split (Pitfall 3)
- [ ] Extend existing `crates/trackly-infra/tests/migration_idempotency.rs` — assert `locations`
      table and old columns are gone post-migration (PLC-04)

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-------------------|
| V2 Authentication | no | Unchanged by this phase — session/auth flow not touched |
| V3 Session Management | no | Unchanged |
| V4 Access Control | **yes** | `authorize(&identity, &Action::{Read,Mutate}Places)` checked in every Tauri command AND every axum handler (dual-transport gate, established pattern) — see Pitfall 3 for the non-standard Admin-only-mutate bucket |
| V5 Input Validation | **yes** | `place.name` length/non-empty validation (mirrors existing `AppError::Validation` pattern); `kind` restricted to the closed 6-value enum via Rust enum (not raw string) at the domain layer, with a DB `CHECK` as defense-in-depth; `level`/`sort_order` bounds (reasonable i32 range, not literally unbounded) |
| V6 Cryptography | no | Not applicable — no secrets/crypto introduced by this phase |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|----------------------|
| SQL injection via place search term | Tampering | `rusqlite` parameterized queries throughout (already the established pattern in every repo file read this session — no string-concatenated SQL for user input; the `LIKE` pattern-escaping helper already exists in `device_service.rs::locations_autocomplete` and should be reused/generalized for place search) |
| Privilege escalation: Manager calling `places_delete`/`places_move` directly via HTTP, bypassing UI button-hiding | Elevation of Privilege | Server-side `authorize()` gate on both transports (Pitfall 3) — UI hiding (`ActionMenu` not rendered for Manager, per UI-SPEC §8.3) is explicitly documented in this project as UX-only, not a security boundary |
| Orphaned FK / cascading data loss on hard-delete | Tampering / DoS (data integrity) | `ON DELETE RESTRICT` on `devices.place_id`/`cartridges.place_id`/`acts.place_id` FKs as defense-in-depth alongside the service-layer precise-count check (D-14) — belt-and-suspenders, matches existing `acts.parent_act_id ON DELETE RESTRICT` precedent |
| Information disclosure: Employee probing `/api/v1/places_*` endpoints directly (bypassing sidebar hiding) | Information Disclosure | Same `authorize()` gate — Employee has neither `ReadPlaces` nor `MutatePlaces`, request rejected with `AppError::Forbidden` regardless of transport |

---

## Sources

### Primary (HIGH confidence — read directly this session)

- `migrations/V001__init_pragmas_and_lookups.sql` through `V036__org_settings_full_name.sql` —
  full migration history, exact current schema of `locations`/`devices`/`acts`/`cartridges`/
  `printers` and existing FTS5 trigger patterns (`V012`, `V013`, `V016`)
- `crates/trackly-core/src/domain/{devices,cartridges,acts,printers,requests}.rs` — exact struct
  shapes touching location today
- `crates/trackly-core/src/auth.rs` — `Action` enum + `authorize()` permission matrix
- `crates/trackly-core/src/ports/devices.rs` — `DeviceRepository` trait shape (pattern to mirror)
- `crates/trackly-infra/src/repos/devices_sqlite.rs` — `resolve_location_id_in_tx` (the
  get-or-create pattern being replaced, Pitfall 4)
- `crates/trackly-app/src/services/{act_service,device_service,report_service}.rs`,
  `crates/trackly-app/src/http/devices.rs`, `crates/trackly-app/src/specta_export.rs` — full
  call-site inventory across both transports
- `ui/src/lib/components/LocationAutocomplete.svelte`,
  `ui/src/features/devices/DeviceAutocompleteField.svelte`,
  `ui/src/features/reports/{ReportsPage,ReportFilters}.svelte`,
  `ui/src/features/requests/RequestsMasterDetail.svelte`, `ui/src/routes.ts`,
  `ui/src/features/layout/sidebar-config.ts` — full UI inventory + layout pattern to mirror
- `crates/trackly-app/templates/act_handover.minijinja` — current print-template contract
- `crates/trackly-infra/tests/migration_idempotency.rs`,
  `crates/trackly-app/tests/{devices_location_roundtrip,role_endpoint_matrix,
  devices_csv_import,auth_remember_cookie}.rs` — existing test patterns and known constraints
- `Cargo.toml` (workspace) — exact `rusqlite 0.38` / `refinery 0.9` versions
- `.planning/phases/39-place-tree/39-CONTEXT.md`, `39-UI-SPEC.md`, `.planning/REQUIREMENTS.md`,
  `.planning/ROADMAP.md` — locked decisions and requirement scope

### Secondary (MEDIUM confidence)

- SQLite `WITH RECURSIVE` support since 3.8.3 (2014) and FTS5 since 3.9.0 — long-standing,
  foundational SQLite engine features, consistent with training knowledge and not contradicted
  by any version-specific concern found in this repo's pinned `rusqlite 0.38`/bundled SQLite.

### Tertiary (LOW confidence — flagged in Assumptions Log)

- WAI-ARIA Authoring Practices treeview + roving-tabindex pattern — confirmed current via
  WebSearch this session (2026 references found), but the specific keyboard contract for this
  phase is already fully locked in `39-UI-SPEC.md` §8.5/§10.5, so this is background
  confirmation only, not a new decision.

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new dependencies, all versions verified directly against `Cargo.toml`
- Architecture (tree model, migrations, FTS design): HIGH — schema/pitfalls verified against the
  actual current migration files and repo code, not assumed
- Blast-radius inventory (§6/call sites): HIGH — every file:line cited was grepped/read directly
  this session
- Pitfalls: HIGH for Pitfalls 1/3/4/5 (verified against repo code and documented SQLite/FTS5
  semantics); MEDIUM for Pitfall 2 (SQLite LIKE ASCII-only case-folding is well-documented SQLite
  behavior but not independently re-verified against this exact bundled build's compile flags —
  flagged as A1 in Assumptions Log)

**Research date:** 2026-08-22
**Valid until:** 30 days (schema/migration design is stable once locked; re-verify if Phase 40/41
discussion surfaces new place-model requirements before Phase 39 plans are written)
