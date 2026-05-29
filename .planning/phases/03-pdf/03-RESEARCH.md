# Phase 3: Акты приёма-передачи и первая PDF-печать - Research

**Researched:** 2026-05-28
**Domain:** acts CRUD + return lifecycle + PDF rendering (krilla + MiniJinja) + UI master-detail
**Confidence:** HIGH (стек заблокирован Phase 1/2; единственная исследовательская зона — krilla 0.7 detalee — снимается структурным spike'ом plan 01)

## Summary

Phase 3 — это «толстый» вертикальный срез: 16 требований (ACT-01..14 + DEV-14..15), новый PDF-конвейер, новый раздел UI, и поверх уже отлаженного Phase 1+2 фундамента (single-writer, hexagonal layers, FTS5, DTO+specta, dual-transport через `build_*`). Технически новые риски сосредоточены в ОДНОЙ зоне — krilla 0.7 на реальной кириллице с детерминированным CI-хешем; пользователь явно решил снять этот риск **через первый план фазы (структурный spike)**, а не upfront-исследованием. Всё остальное — композиция установленных паттернов.

**Primary recommendation:** план фазы — 5 планов в строгой последовательности.
1. **PDF foundation** — крaте `pdf/` с krilla + embedded DejaVu Sans + MiniJinja safe-mode + DocSpec-IR + CI hash-фикстура на «Привет, мир + Сидоров-Петроградский». **GATE: hash зелёный на linux/macos/windows runners → idti дальше; иначе spike typst-as-lib.**
2. **Acts CRUD + handover** — backend (`core/domain/acts`, `core/ports/acts`, `infra/repos/acts_sqlite`, `app/services/act_service`, тонкие `tauri_cmds/acts` и `http/acts`) + UI master-detail скелет + создание handover + switch-bar.
3. **Returns + archive + undo** — return-модал с bulk+per-row, sub_number counter, auto-archive, undo handover, undo return из `audit_log.before_json`.
4. **Templates + org + PDF endpoints** — runtime-seed `document_templates`, `organization_service` читает `org.json`, `acts_render_pdf` + `acts_render_acceptance_pdf` commands, фронт-модал preview через pdfjs-dist iframe.
5. **DEV-14 acceptance + поиск по актам + полировка** — кнопка «Печать документа приёма» на странице устройств (DEV-14/15), поиск актов через FTS5 (расширение Phase 2 паттерна), счётчики switch-bar, конец-в-конец smoke-тест.

Все 5 планов держат единое инваранство: каждый commit оставляет CI зелёным; каждый план поставляет рабочий вертикальный срез до UI; реальные PDF-байты появляются в plan 01 (фикстура), реальный акт-PDF — в plan 04.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|--------------|----------------|-----------|
| Counter atomic increment (act_number, sub_number) | trackly-infra (writer task + SQL `BEGIN IMMEDIATE`) | — | Single-writer invariant из Phase 1; counter-counter race недопустим. |
| Act lifecycle (create handover, create return, archive, undo) | trackly-app/services/act_service | trackly-infra/repos/acts_sqlite | Service оркестрирует мульти-таблицные транзакции; repo — узкие SQL-операции. |
| Device state mutations (status, location) внутри act-транзакции | trackly-app/services/act_service (вызывает device_repo.update_status_in_tx внутри writer-замыкания) | trackly-infra/repos/devices_sqlite | Должно быть в той же writer-job, не отдельный writer.execute — иначе ACT-13 transactional guarantee нарушается. |
| Audit_log writes (handover create, device update, return undo) | trackly-app/services/act_service внутри writer-job | trackly-infra/repos/audit_log_sqlite (новый thin repo) | Audit пишется в той же транзакции; payload_json содержит act_id для группировки. |
| MiniJinja safe-mode rendering | trackly-app/pdf/minijinja_env | — | App-уровень — единственный потребитель MiniJinja Environment; trackly-core/infra не нужны. |
| DocSpec serde validation | trackly-app/pdf/docspec | — | Typed AST между шаблоном и krilla; нет I/O. |
| krilla PDF rendering | trackly-app/pdf/renderer | — | Engine + embedded font bytes; нет других потребителей. |
| Template storage (CRUD + seed) | trackly-app/services/template_service | trackly-infra/repos/templates_sqlite | На Phase 3 — только seed + read; CRUD UI — Phase 7. |
| Organization data | trackly-app/services/organization_service | trackly-infra (file I/O через Paths) | Читает `org.json` через `Paths::root()`; нет таблицы. |
| Acts list/search/counts UI | ui/src/features/acts (Svelte 5 runes) | ui/src/lib/components (Modal, Input, Button, Toast) | Greenfield feature folder; следует паттерну `features/devices`. |
| PDF preview в webview | ui (pdfjs-dist в blob: iframe) | tauri-plugin-dialog (save), tauri-plugin-shell (open) | Desktop-only в Phase 3; server-mode HTTP endpoint строится но не bind'ится. |

## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-Numbering-01:** Display-rule «в»/«в1»/«в2» — в сервисе/DTO при чтении. В БД `sub_number = 1, 2, 3, ...` всегда (NULL только для handover). Ретроактивное переименование «42в» → «42в1» при появлении 2-го возврата приемлемо; старые распечатанные PDF остаются со снимочным суффиксом.
- **D-Counter-Acts-01:** `UPDATE counters SET current_value = current_value + 1 WHERE name='act_number' RETURNING current_value` под `BEGIN IMMEDIATE` внутри writer task. Override номера: counter НЕ инкрементируется, отдельная audit_log запись `action='custom:act_number_override'` с payload `{requested, next_auto_would_be}`. UNIQUE-конфликт → `AppError::Conflict {field:"number", message:"Акт №X уже существует"}`. `sub_number` для return: `SELECT MAX(sub_number)+1 ...` в той же транзакции.
- **D-Archive-01:** Авто-архив на 100% возврате — derived state, **никакой ручной кнопки «В архив»**. При delete return пересчёт; остаток > 0 → `archived=0`.
- **D-Undo-01:** Универсальный путь через `audit_log.before_json`. Каждая mutation device внутри handover/return пишет audit row с `before_json={...полный row до...}`, `after_json={...полный row после...}`, `payload_json={"act_id": N, "kind":"return"|"handover"}`. Undo: SELECT всех audit-rows по act_id → восстановление каждого device через UPDATE. Soft-delete акта. `act_items.condition_at_time` остаётся в схеме как denormalized snapshot для отчётности, **но в undo не используется**.
- **D-Soft-vs-Hard-Acts-01:** `acts.deleted_at_utc` (soft); `act_items` — junction, hard-DELETE в той же транзакции (FK CASCADE НЕ срабатывает на soft-delete). UNIQUE-индекс `WHERE deleted_at_utc IS NULL` означает «удалённый» номер 42 формально свободен; **сервис явно валидирует переиспользование** (`SELECT 1 FROM acts WHERE number=? LIMIT 1` без фильтра по deleted_at_utc).
- **D-Acts-List-01:** Master-detail (35% / 65%, фиксированный split в Phase 3). Switch-bar: Акты (`act_type='handover' AND archived=0 AND deleted_at_utc IS NULL`), Возвраты (`act_type='return' AND deleted_at_utc IS NULL`), Архив (`act_type='handover' AND archived=1 AND deleted_at_utc IS NULL`). Счётчики — отдельный command `acts_counts() -> {handover_active, returns, archived}`. Поиск общий — `number`, `giver_name`, `receiver_name`, `device.name` (FTS5).
- **D-Acts-Create-01:** Широкий модал (~1000px). Шапка (№ с auto-кнопкой, Дата, Сдал, Принял, Сроком до, Расположение) + table «Позиции» (device autocomplete фильтр `status='На складе'` + количество). Override-номера: badge «override» + tooltip «Будет записано в журнал». Backend — `acts_create(payload) -> ActDto` single writer transaction.
- **D-Acts-Return-01:** Модал возврата с bulk-default + per-row override. Чекбокс «Применить ко всем» **по умолчанию ВКЛ**.
- **D-Print-UX-01:** preview-модал с pdfjs-dist в `<iframe src="blob:...">`. Кнопки «Сохранить как...» (tauri-plugin-dialog save), «Открыть в системном просмотрщике» (tmp-file → tauri-plugin-shell::open), «Печать» (`window.print()` на iframe). pdfjs-dist через npm dep + dynamic import.
- **D-PDF-Engine-01:** krilla 0.7 — БЕЗ upfront-spike, но **первый план = «PDF-инфра + фикстура»** (структурный spike). DejaVu Sans Regular+Bold embedded через `include_bytes!` в `crates/trackly-app/assets/fonts/`. CI hash-test на 3 ОС (linux/macOS/windows); `krilla = "=0.7.0"` pinned. Fixture обязан содержать «Сидоров-Петроградский Иван Александрович (ё) №42». MSRV bump до 1.92 (krilla требование). Если первый план провалится — отдельная mini-фаза typst-as-lib spike.
- **D-PDF-Render-Path-01:** 3-этапный pipeline. (1) MiniJinja safe-mode → JSON-строка; `UndefinedBehavior::Strict`, без loader, 5s `tokio::time::timeout`. (2) `serde_json::from_str::<DocSpec>(rendered)` strict deser; невалид → `AppError::Validation {field:"template", ...}`. (3) DocSpec → krilla → PDF bytes.
- **D-PDF-Templates-Schema-01:** Контракт `act.*` / `org.*` / `return.*` переменных шаблона (точный список ниже в этом документе).
- **D-Templates-Seed-01:** Runtime seed из `include_str!`-файлов на startup. `crates/trackly-app/templates/{act_handover,act_acceptance}.minijinja`. Идемпотентность через `count(*) per kind = 0`. Soft-delete всех → пересоздаём (feature).
- **D-OrgData-01:** `<paths.root>/org.json` через `Paths::root()`. Format: `{name, inn, kpp, address, logo_path}`. Отсутствует → создаём placeholder при первом старте + warning. Logo relative path; krilla читает файл при render; нет файла → рендер без логотипа + warning. Backup БД НЕ захватывает `org.json` (намеренно).
- **D-AppCtx-Extension-03:** AppCtx расширяется 4 полями: `acts: Arc<ActService>`, `organization: Arc<OrganizationService>`, `templates: Arc<TemplateService>`, `pdf: Arc<PdfRenderer>`. `PdfRenderer` держит embedded font bytes (`Arc<[u8]>`) и MiniJinja Environment (тяжёлый — переиспользуем).
- **D-Test-Phase3-01:** Unit (sub_number compute, format_act_number, archive predicate, DocSpec serde round-trip); Integration с real SQLite (create handover; partial return; full return → archived; delete return → archived=0 + devices restored; delete handover); PDF (hash-фикстура + text-extract «Сидоров-Петроградский» и «№42»); Tauri commands (`build_*` helper тесты как в Phase 2).

### Claude's Discretion

- Точная форма DocSpec enum-вариантов и optional полей — уточняется в plan 01 (см. черновик ниже).
- Структура `crates/trackly-app/src/pdf/` (split mod.rs → renderer.rs / docspec.rs / fonts.rs / minijinja_env.rs).
- Точная модель `act_items`: добавлять ли `quantity` или вводить отдельную `act_item_returns` таблицу — решение plan'у после анализа V004 (см. Open Question 1 ниже).
- Имена commands — snake_case + namespace `acts_*` / `templates_*` / `organization_*`.
- pdfjs-dist подключение — npm dep + dynamic import (для tree-shake в server-mode bundle).
- Russian typography (отступы, шрифт-размеры) — монохром по умолчанию (`_tokens.scss` цвета НЕ используются в PDF).
- Дополнительные миграции (V014__acts_indexes_or_seeds.sql) — на усмотрение планировщика.

### Deferred Ideas (OUT OF SCOPE)

- UI редактор шаблонов → Phase 7.
- Полноценная страница Организация/Настройки → Phase 7.
- 3-way merge для обновлённых дефолтных шаблонов → Phase 7.
- Кнопка «Сбросить шаблон к дефолту» → Phase 7.
- Logo binary в БД → Phase 7.
- PDF.js custom worker / встроенный print UI → Phase 3 использует нативный `window.print()`.
- Retention для audit_log → Phase 7.
- Resizable split master-detail → Phase 7.
- Виртуализация списка актов → отдельная perf-фаза.
- Печать списка/отчёта по актам → Phase 7.
- Печать заявок (REQ-04) — переиспользует Phase 3 pipeline → Phase 6.
- Watch-режим для `org.json` → Phase 7.
- Запрет переиспользования удалённых номеров через отдельный индекс → tracked; в Phase 3 — валидация в сервисе.
- Spike krilla vs typst-as-lib — НЕ upfront; только если plan 01 провалится.
- Server-mode HTTP bind — Phase 5.

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| ACT-01 | CRUD актов приёма-передачи | `ActService` + `ActRepository` (новые); pattern: `DeviceService` |
| ACT-02 | Switch-bar Акты/Возвраты/Архив со счётчиками | D-Acts-List-01; `acts_counts` command; pattern: Phase 2 status switch-bar |
| ACT-03 | Поля акта (№ auto+override, Дата, Сдал, Принял, Устройство, Количество, Сроком до, Расположение) | D-Acts-Create-01; counter pattern из V009; UI — Phase 2 modal pattern |
| ACT-04 | Поиск по актам (№, ФИО, наименование) | FTS5 расширение (см. ниже §FTS5 для актов); JOIN на `devices_fts` через act_items |
| ACT-05 | Действия над актом (Просмотр, Редактировать, Возврат, Удалить) | Кнопки в detail-pane; команды `acts_get/update/delete/return` |
| ACT-06 | Удаление handover-акта восстанавливает Состояние/Расположение | D-Undo-01; читаем `audit_log.before_json` по payload_json.act_id |
| ACT-07 | Возврат с суффиксом «в»/«в1»/«в2» | D-Numbering-01; display-rule в сервисе при чтении |
| ACT-08 | При возврате — Состояние+Расположение + bulk «Применить ко всем» | D-Acts-Return-01; модал с bulk-default + per-row override |
| ACT-09 | Архив при 100% возврате | D-Archive-01; derived state |
| ACT-10 | Удаление return-акта восстанавливает Состояние/Расположение к моменту выдачи | D-Undo-01; pересчёт `archived` после восстановления |
| ACT-11 | Печать/PDF Акта приёма-передачи с шапкой+логотипом | D-PDF-Render-Path-01; `acts_render_pdf` command; шаблон `act_handover` |
| ACT-12 | Редактируемый шаблон акта (в БД) | V007 `document_templates` уже есть; D-Templates-Seed-01 для дефолта |
| ACT-13 | Транзакционная гарантия (всё или ничего) | Single writer + одна `conn.transaction()` внутри writer-job |
| ACT-14 | Атомарная генерация номера, override → audit_log | D-Counter-Acts-01; existing V009 `act_number` row |
| DEV-14 | Печать документа приёма товара на склад (PDF, поля Кто передал/принял) | Тот же pipeline; шаблон `act_acceptance`; command `devices_render_acceptance_pdf` |
| DEV-15 | Редактируемый шаблон документа приёма (в БД) | V007 `document_templates.kind='act_acceptance'`; seed на startup |

## Project Constraints (from CLAUDE.md)

- **Стек жёстко зафиксирован:** Rust + Tauri 2.11 + Svelte 5 + SCSS + SQLite (WAL). Никаких альтернатив.
- **Portable:** все пути через `Paths::root()`; **запрещено** `dirs::*_dir()` (clippy disallowed-methods). Это касается `org.json` (рядом с .exe), логотипа, временных PDF для «открыть в системном просмотрщике» — все через `Paths`.
- **WEBVIEW2_USER_DATA_FOLDER** уже выставляется в `main.rs` (Phase 1 FOUND-05) — Phase 3 ничего не делает.
- **Concurrent-доступ:** SQLite WAL, **единственный writer** через `WriterHandle::execute` (Phase 1 invariant). Все мутации актов + counter + audit_log + device updates — внутри одной транзакции одной job.
- **Языковая локализация:** UI и шаблоны документов — **только русский** в v1; никакой инфраструктуры i18n.
- **Документы:** **редактируемые шаблоны в БД** (V007 уже есть); это закрывает ACT-12/DEV-15 структурно — Phase 3 только seed'ит дефолты.
- **Безопасность:** MiniJinja — пользовательский шаблон в БД; safe-mode критичен (no exec, no include, 5s timeout, UndefinedBehavior::Strict).
- **GSD workflow:** Edit/Write только через GSD команду — Phase 3 идёт через `/gsd-execute-phase`.

## Standard Stack

### Core (new in Phase 3)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `krilla` | `=0.7.0` (latest, pinned) | PDF rendering с OpenType subsetting | `[CITED: CLAUDE.md + .planning/research/STACK.md]` — primary engine; `[VERIFIED: crates.io 2026-03-31]` — `newest_version: "0.7.0"`, downloads 590K, repo github.com/LaurenzV/krilla |
| `minijinja` | `^2.20` (default-features=false, features=["json"]) | Шаблонизатор для act_handover/act_acceptance | `[CITED: CLAUDE.md]`; `[VERIFIED: crates.io 2026-05-19]` `newest_version: "2.20.0"` |
| `pdf-extract` | `^0.10` (dev-dep only, для text-extract test) | Извлечение текста из PDF для assertions «Сидоров-Петроградский содержится» | `[VERIFIED: crates.io]` `newest_version: "0.10.0"` |

### Frontend (new in Phase 3)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `pdfjs-dist` | `^4.x` (latest stable) | PDF viewer в `<iframe src="blob:...">` для preview модала | `[CITED: D-Print-UX-01]`; `[ASSUMED: latest 4.x major на момент Phase 3]` — точную версию выбирает плана 04 при добавлении npm dep |

### Already-in-stack (carry-forward, не добавляем)

| Library | Used For |
|---------|----------|
| `rusqlite 0.38` (workspace pin) | act_repo + counter + audit_log SQL |
| `serde 1`, `serde_json 1` | DocSpec serde, payload_json в audit_log |
| `time 0.3` | Форматирование даты «28 мая 2026 г.» (RU locale) |
| `tokio 1` | `tokio::time::timeout(5s)` вокруг MiniJinja render |
| `tracing 0.1` | Логи при отсутствии org.json / logo / template-seed warnings |
| `specta 2.0.0-rc.22` + `tauri-specta 2.0.0-rc.21` | DTO → bindings.ts для всех новых типов |
| `tauri-plugin-dialog 2.7` | Save-as PDF, file pick для logo (если когда-то) |
| `tauri-plugin-shell 2.x` | «Открыть в системном просмотрщике» |
| `svelte-spa-router 5.1` | Раздел /acts |
| `axum 0.8` | `http::acts::router()` строится, mount будет в Phase 5 |

**Installation:**
```bash
# Cargo workspace root
cargo add -p trackly-app krilla@=0.7.0
cargo add -p trackly-app minijinja@^2.20 --no-default-features --features json
cargo add -p trackly-app --dev pdf-extract@^0.10
# Optional dev dep for stable hashing (sha2 уже добавлен в Phase 1 как dev-dep)

# UI
cd ui && pnpm add pdfjs-dist
```

**Version verification:**
- `krilla 0.7.0` — `[VERIFIED: crates.io API 2026-03-31]` newest_version.
- `minijinja 2.20.0` — `[VERIFIED: crates.io API 2026-05-19]` newest_version.
- `pdf-extract 0.10.0` — `[VERIFIED: crates.io API]` newest_version.
- `pdfjs-dist` — `[ASSUMED: 4.x latest]` план 04 проверит `npm view pdfjs-dist version` перед добавлением.

## Package Legitimacy Audit

> slopcheck не установлен в окружении ресёрча (pip не доступен). Маркируем согласно правилу graceful degradation.

| Package | Registry | Age | Downloads | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-----------|-------------|-----------|-------------|
| `krilla` | crates.io | с 2024-09-04 (~1.5 года) | 590K total / 428K recent | github.com/LaurenzV/krilla | NOT_RUN | **Approved** — mainstream PDF crate, `[CITED]` в CLAUDE.md, активно поддерживается. План 01 (PDF-инфра) — структурный spike, фактически проверяет крaте на реальных байтах. |
| `minijinja` | crates.io | давно (>5 лет) | миллионы | github.com/mitsuhiko/minijinja | NOT_RUN | **Approved** — Armin Ronacher's crate, де-факто стандарт. |
| `pdf-extract` | crates.io | давно | стабильно | github.com/jrmuizel/pdf-extract | NOT_RUN | **Approved** dev-dep only — используется только в hash test. |
| `pdfjs-dist` | npm | >10 лет | миллионы/неделю | github.com/mozilla/pdf.js | NOT_RUN | **Approved** — Mozilla официально. |

**Packages removed due to slopcheck [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none (slopcheck не run; все 4 пакета — известные mainstream проекты с явным репо и многолетней историей)

*Если plan-execution на машине пользователя сможет запустить slopcheck — повторите проверку перед `cargo add`. На данный момент все 4 пакета помечены как «approved by manual provenance» (репо, downloads, age проверены вручную через crates.io API и официальные источники).*

## Architecture Patterns

### System Architecture Diagram

```
┌──────────────────────── UI (Svelte 5 + svelte-spa-router) ──────────────────────┐
│  /acts route → ActsPage.svelte (master-detail layout)                            │
│   ├─ ActsList (35%) — switch-bar (Акты/Возвраты/Архив) + search input + cards   │
│   ├─ ActDetail (65%) — header + act_items table + actions (PDF/Edit/Return/Del) │
│   ├─ ActFormModal — wide modal create handover                                   │
│   ├─ ReturnModal — bulk-default + per-row override                              │
│   └─ PdfPreviewModal — pdfjs-dist iframe (blob: URL)                            │
│       └─ Save→ tauri-plugin-dialog | Open→ tauri-plugin-shell | Print→window.print() │
└───────────────────────┬──────────────────────────────────────────────────────────┘
                        │ apiCall<R>() — transport-detected (Tauri invoke OR HTTP)
                        ▼
┌─────────────────── trackly-app::tauri_cmds::acts (thin) ─────────────────────────┐
│  acts_list / acts_get / acts_create / acts_update / acts_delete / acts_return    │
│  acts_search / acts_counts / acts_render_pdf                                     │
│  organization_get / templates_get / templates_render_preview                     │
│  devices_render_acceptance_pdf (DEV-14)                                          │
└────────────────────────┬──────────────────────┬───────────────────────────────────┘
       same body         │                      │
                         ▼                      ▼
┌─────────────────── trackly-app::http::acts (built, not bound) ───────────────────┐
│  POST /api/v1/acts_list, /acts_get, …  same handlers via build_*(&ctx,args)      │
└────────────────────────┬──────────────────────────────────────────────────────────┘
                         │
                         ▼
┌──────────────────── trackly-app::services::act_service ──────────────────────────┐
│  AppCtx { writer, readers, clock, repo, audit_repo, devices: Arc<DeviceService>, │
│          pdf: Arc<PdfRenderer> }                                                 │
│                                                                                  │
│  create(payload) → writer.execute(|conn| {                                       │
│     let tx = conn.transaction(); // BEGIN IMMEDIATE                              │
│     1. SELECT 1 FROM acts WHERE number=? LIMIT 1; → Conflict if exists           │
│     2. UPDATE counters SET cv=cv+1 WHERE name='act_number' RETURNING cv;         │
│        OR use user-override (counter NOT incremented; audit override)            │
│     3. INSERT INTO acts(...)                                                     │
│     4. INSERT INTO act_items(...) x N                                            │
│     5. For each device: UPDATE devices SET status_id='в_работе', location=...    │
│     6. INSERT INTO audit_log per device update + per act create                  │
│     7. COMMIT;                                                                   │
│   })                                                                             │
│                                                                                  │
│  return(act_id, items) → writer.execute(|conn| {                                 │
│     let tx; sub_number = SELECT MAX(sub_number)+1 FROM acts WHERE parent_act_id  │
│     INSERT return-act; INSERT act_items;                                         │
│     For each returned device: UPDATE state+location; INSERT audit_log;           │
│     IF all_returned: UPDATE handover SET archived=1;                             │
│     COMMIT;                                                                      │
│   })                                                                             │
│                                                                                  │
│  delete(act_id) → writer.execute(|conn| {                                        │
│     SELECT audit_log WHERE payload_json LIKE '%"act_id":N%'                      │
│     For each: UPDATE devices SET ...(from before_json);                          │
│     INSERT audit_log action='custom:undo_…';                                     │
│     UPDATE acts SET deleted_at_utc=?, version+=1;                                │
│     DELETE FROM act_items WHERE act_id=?;                                        │
│     IF was_return: recompute parent.archived flag;                               │
│     COMMIT;                                                                      │
│   })                                                                             │
│                                                                                  │
│  render_pdf(act_id) → pdf.render(load_template, build_context, act, org) → Vec<u8>│
└────────────────────────┬──────────────────────────────────────────────────────────┘
                         │
                         ▼
┌──────────────────── trackly-app::pdf (3-stage pipeline) ─────────────────────────┐
│  minijinja_env.render(template_str, ctx) → JSON string  (5s tokio timeout)       │
│         │                                                                        │
│         ▼                                                                        │
│  serde_json::from_str::<DocSpec>(json) → DocSpec  (Validation if malformed)      │
│         │                                                                        │
│         ▼                                                                        │
│  renderer::render(DocSpec, &PdfRenderer) → Vec<u8>  (krilla calls)               │
│         │                                                                        │
│         └─→ embedded DejaVu Sans Regular/Bold (Arc<[u8]>)                        │
└──────────────────────────────────────────────────────────────────────────────────┘
```

### Recommended Project Structure

```
crates/
  trackly-core/
    src/domain/acts.rs           # NEW: ActNew, ActPatch, ActFilter, ReturnItem, ActKind enum
    src/ports/acts.rs            # NEW: ActRepository trait
  trackly-infra/
    src/repos/acts_sqlite.rs     # NEW: SqliteActRepository impl
    src/repos/audit_log_sqlite.rs # NEW (thin): insert + select-by-payload-act-id
  trackly-app/
    src/services/
      act_service.rs             # NEW
      organization_service.rs    # NEW
      template_service.rs        # NEW
    src/dto/
      act.rs                     # NEW: ActDto, ActCreateDto, ActReturnDto, ActDetailDto, ActsCountsDto, ActFilterDto
      organization.rs            # NEW: OrgDto
      doc_spec.rs                # NEW: DocSpec + Section enum (serde::Type for bindings if needed)
    src/tauri_cmds/
      acts.rs                    # NEW
      organization.rs            # NEW
      templates.rs               # NEW (only get + preview in Phase 3)
    src/http/
      acts.rs                    # NEW
      organization.rs            # NEW
    src/pdf/                     # NEW module
      mod.rs                     # re-exports
      renderer.rs                # krilla calls
      docspec.rs                 # DocSpec struct + Section enum
      minijinja_env.rs           # build safe Environment
      fonts.rs                   # include_bytes! for DejaVu Sans Regular/Bold
    assets/fonts/                # NEW
      DejaVuSans.ttf
      DejaVuSans-Bold.ttf
    templates/                   # NEW (compile-time, include_str!)
      act_handover.minijinja
      act_acceptance.minijinja
    tests/
      acts_crud.rs               # NEW
      acts_returns.rs            # NEW
      acts_undo.rs               # NEW
      acts_search.rs             # NEW (FTS join)
      acts_http_smoke.rs         # NEW
      pdf_fixture.rs             # NEW — hash test + text-extract
      templates_seed.rs          # NEW

ui/
  src/
    features/acts/               # NEW feature folder
      ActsPage.svelte
      ActsList.svelte
      ActListRow.svelte
      ActDetail.svelte
      ActItemsTable.svelte
      ActFormModal.svelte
      ReturnModal.svelte
      PdfPreviewModal.svelte
      api.ts
    lib/api/
      acts.ts                    # NEW
      organization.ts            # NEW
      pdf.ts                     # NEW (wraps acts_render_pdf for blob URL handling)
    lib/components/              # potentially reused:
      Modal.svelte (existing)
      Input.svelte (existing)
      Button.svelte (existing)
      Toast.svelte / ToastHost.svelte (existing)
      Select.svelte (existing)
      Spinner.svelte (existing)
      Badge.svelte (existing)

migrations/
  V014__acts_audit_indexes.sql   # NEW (only if планировщик решит — см. Open Q3)
```

### Pattern 1: Atomic act creation with counter + items + audit (single writer-job)

**What:** Все act-mutations (counter increment, INSERT acts, INSERT act_items, UPDATE devices, INSERT audit_log) — **в одной writer-job, в одной транзакции**.

**When to use:** ACT-13 transactional guarantee, ACT-14 thread-safe counter.

**Example:**
```rust
// Source: pattern derived from Phase 1 WriterHandle + V009 counters + V004 acts
pub async fn create(&self, payload: ActCreateDto) -> Result<ActDto, AppError> {
    let now = self.clock.unix_seconds();
    let repo = self.repo.clone();
    let audit_repo = self.audit_repo.clone();
    let device_repo = self.devices.repo.clone();

    self.writer.execute(move |conn| {
        let tx = conn.transaction().map_err(map_rusqlite)?;
        // 1. Resolve number: override OR atomic counter increment
        let number = if let Some(custom) = payload.number_override {
            // Validate uniqueness explicitly (включая удалённые номера — D-Soft-vs-Hard)
            let exists: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM acts WHERE number=?1 LIMIT 1)",
                [custom], |r| r.get(0)
            ).map_err(map_rusqlite)?;
            if exists {
                return Err(AppError::Conflict {
                    field: "number".into(),
                    message: format!("Акт №{custom} уже существует"),
                });
            }
            // Audit the override
            audit_repo.insert(&tx, AuditEntry {
                entity_type: "act", entity_id: 0, // patched after INSERT
                action: "custom:act_number_override",
                payload_json: Some(serde_json::json!({
                    "requested": custom,
                    "next_auto_would_be": peek_counter(&tx, "act_number")?,
                }).to_string()),
                ..
            })?;
            custom
        } else {
            // Atomic increment via RETURNING (rusqlite supports SQLite 3.35+)
            tx.query_row(
                "UPDATE counters SET current_value = current_value + 1
                 WHERE name = 'act_number' RETURNING current_value",
                [], |r| r.get::<_, i64>(0)
            ).map_err(map_rusqlite)?
        };

        // 2. INSERT acts
        let act_id = repo.insert_act(&tx, ActRow {
            number, sub_number: None, parent_act_id: None,
            act_type: "handover", giver_name: payload.giver_name.clone(),
            receiver_name: payload.receiver_name.clone(),
            location_id: payload.location_id, notes: payload.notes.clone(),
            archived: false, created_at_utc: now, updated_at_utc: now,
            deleted_at_utc: None, version: 1,
        })?;

        // 3. INSERT act_items + UPDATE devices + audit each
        for item in &payload.items {
            // Fetch device snapshot BEFORE for audit_log.before_json
            let before = device_repo.fetch_row_for_audit(&tx, item.device_id)?;
            repo.insert_act_item(&tx, ActItemRow {
                act_id, device_id: item.device_id,
                condition_at_time: before.condition.clone(),
                complectation_at_time: before.complectation.clone(),
            })?;
            // Status='В работе' lookup id once
            let status_in_work = lookup_status_id(&tx, "в_работе")?;
            let after = device_repo.update_status_and_location(
                &tx, item.device_id, status_in_work, payload.location_id
            )?;
            audit_repo.insert(&tx, AuditEntry {
                entity_type: "device", entity_id: item.device_id,
                action: "update",
                before_json: Some(serde_json::to_string(&before)?),
                after_json: Some(serde_json::to_string(&after)?),
                payload_json: Some(serde_json::json!({
                    "act_id": act_id, "kind": "handover"
                }).to_string()),
                created_at_utc: now, user_id: None,
            })?;
        }
        // 4. Final audit for act creation
        audit_repo.insert(&tx, AuditEntry {
            entity_type: "act", entity_id: act_id,
            action: "create",
            after_json: Some(serde_json::to_string(&repo.fetch_full(&tx, act_id)?)?),
            payload_json: None,
            ..
        })?;
        tx.commit().map_err(map_rusqlite)?;
        Ok(act_id)
    }).await
    .and_then(|id| self.get(id).await)  // reload via read pool for DTO
}
```

### Pattern 2: Display-rule for «в»/«в1»/«в2»

**What:** В БД sub_number = 1, 2, 3, ...; format в DTO при чтении.

**When to use:** При построении любого ActDto — list, detail, PDF render.

**Example:**
```rust
// Pure function, в act_service или dto/act.rs::ActDto::from_row
fn format_act_number(act: &ActRow, return_count_for_parent: Option<i64>) -> String {
    match act.act_type.as_str() {
        "handover" => act.number.to_string(),
        "return" => {
            let sub = act.sub_number.expect("return must have sub_number");
            let parent_number = act.parent_number.expect("return must have parent_number joined");
            // Если есть ровно 1 возврат у этого parent — «42в»; иначе «42в{sub}»
            if return_count_for_parent == Some(1) {
                format!("{}в", parent_number)
            } else {
                format!("{}в{}", parent_number, sub)
            }
        },
        _ => unreachable!("act_type CHECK constraint"),
    }
    // NOTE: PDF embeds the suffix at render-time (snapshot semantics, D-Numbering-01).
    // The list view computes it at read-time and tolerates retroactive rename.
}
```

**Reading query** — нужен `return_count` per handover; добавляем подзапрос:
```sql
SELECT a.id, a.number, a.sub_number, a.parent_act_id, a.act_type,
       p.number AS parent_number,
       (SELECT COUNT(*) FROM acts r WHERE r.parent_act_id = a.parent_act_id
                                    AND r.deleted_at_utc IS NULL) AS sibling_return_count
  FROM acts a
  LEFT JOIN acts p ON p.id = a.parent_act_id
  WHERE a.id = ?
```

### Pattern 3: Undo via audit_log.before_json

**What:** Delete handover/return акт → SELECT audit_log по `payload_json` → восстановить devices.

**Example:**
```rust
pub async fn delete(&self, act_id: i64) -> Result<(), AppError> {
    let now = self.clock.unix_seconds();
    let repo = self.repo.clone();
    let audit_repo = self.audit_repo.clone();
    let device_repo = self.devices.repo.clone();
    self.writer.execute(move |conn| {
        let tx = conn.transaction().map_err(map_rusqlite)?;
        let act = repo.fetch_full(&tx, act_id)?;
        if act.deleted_at_utc.is_some() {
            return Err(AppError::NotFound {
                entity: "act".into(), id: act_id,
            });
        }
        // Read audit_log rows that mutated devices as part of this act
        // payload_json is TEXT, json_extract supported by rusqlite (SQLite JSON1)
        let mut stmt = tx.prepare("
            SELECT entity_id, before_json
              FROM audit_log
             WHERE entity_type = 'device'
               AND json_extract(payload_json, '$.act_id') = ?1
             ORDER BY created_at_utc ASC, id ASC
        ").map_err(map_rusqlite)?;
        let rows: Vec<(i64, String)> = stmt.query_map([act_id], |r| {
            Ok((r.get(0)?, r.get::<_, String>(1)?))
        })?.collect::<Result<Vec<_>, _>>().map_err(map_rusqlite)?;
        for (device_id, before_json) in rows {
            let before: DeviceRow = serde_json::from_str(&before_json)
                .map_err(|e| AppError::Internal {
                    source_chain: format!("undo: corrupt audit_log.before_json: {e}"),
                })?;
            device_repo.restore_from_snapshot(&tx, device_id, &before)?;
            audit_repo.insert(&tx, AuditEntry {
                entity_type: "device", entity_id: device_id,
                action: "custom:undo",
                payload_json: Some(serde_json::json!({"act_id": act_id}).to_string()),
                created_at_utc: now, ..
            })?;
        }
        // Soft-delete act + hard-delete act_items
        repo.soft_delete_act(&tx, act_id, now)?;
        tx.execute("DELETE FROM act_items WHERE act_id = ?1", [act_id])
            .map_err(map_rusqlite)?;
        // If this was a return — recompute parent.archived
        if let Some(parent_id) = act.parent_act_id {
            recompute_parent_archived(&tx, parent_id, &device_repo)?;
        }
        tx.commit().map_err(map_rusqlite)?;
        Ok(())
    }).await
}
```

**Critical:** `payload_json` запросы через `json_extract` требуют SQLite JSON1 (компилирован by default в rusqlite bundled — OK). Indexed lookup см. ниже §Migration V014.

### Pattern 4: MiniJinja safe-mode bootstrap

```rust
// crates/trackly-app/src/pdf/minijinja_env.rs
use minijinja::{Environment, UndefinedBehavior, AutoEscape};

pub fn build_safe_env() -> Environment<'static> {
    let mut env = Environment::new();
    env.set_undefined_behavior(UndefinedBehavior::Strict);  // any undefined → render error
    env.set_auto_escape_callback(|_| AutoEscape::None);     // we emit JSON, not HTML
    env.set_recursion_limit(64);                            // shallow templates
    env.set_fuel(Some(100_000));                            // bounded instruction count
    // DO NOT call env.set_loader — no filesystem includes
    env
}

pub async fn render_with_timeout(
    env: &Environment<'_>,
    name: &str,
    template_src: &str,
    ctx: serde_json::Value,
) -> Result<String, AppError> {
    let env_owned = env.clone();
    let template_src_owned = template_src.to_owned();
    let name_owned = name.to_owned();
    // MiniJinja render is sync CPU; offload to spawn_blocking + timeout
    let fut = tokio::task::spawn_blocking(move || -> Result<String, AppError> {
        let mut env = env_owned;
        env.add_template_owned(name_owned.clone(), template_src_owned)
            .map_err(|e| AppError::Validation {
                field: "template".into(),
                message: format!("Template parse error: {e}"),
            })?;
        let tmpl = env.get_template(&name_owned)
            .map_err(|e| AppError::Internal {
                source_chain: format!("get_template: {e}"),
            })?;
        tmpl.render(ctx).map_err(|e| AppError::Validation {
            field: "template".into(),
            message: format!("Template render error: {e}"),
        })
    });
    tokio::time::timeout(std::time::Duration::from_secs(5), fut).await
        .map_err(|_| AppError::Validation {
            field: "template".into(),
            message: "Render timeout (5s) — шаблон слишком сложный".into(),
        })?
        .map_err(|e| AppError::Internal {
            source_chain: format!("spawn_blocking joined: {e}"),
        })?
}
```

`[CITED: docs.rs/minijinja/latest/minijinja/struct.Environment.html]` — `set_undefined_behavior`, `set_auto_escape_callback`, `set_recursion_limit`, `set_fuel` все documented; default state — без loader (verified WebFetch выше).

### Pattern 5: krilla rendering with embedded font

```rust
// crates/trackly-app/src/pdf/fonts.rs
use std::sync::Arc;
pub static DEJAVU_SANS_REGULAR: &[u8] = include_bytes!("../../assets/fonts/DejaVuSans.ttf");
pub static DEJAVU_SANS_BOLD: &[u8]    = include_bytes!("../../assets/fonts/DejaVuSans-Bold.ttf");

// crates/trackly-app/src/pdf/renderer.rs
use krilla::{Document, page::PageSettings, text::{Font, TextDirection}, geom::Point};

pub struct PdfRenderer {
    pub font_regular_bytes: Arc<Vec<u8>>,  // wrapped Arc<Vec<u8>> for cheap clone into closures
    pub font_bold_bytes: Arc<Vec<u8>>,
    pub minijinja_env: Arc<minijinja::Environment<'static>>,
}

impl PdfRenderer {
    pub fn new() -> Self {
        Self {
            font_regular_bytes: Arc::new(super::fonts::DEJAVU_SANS_REGULAR.to_vec()),
            font_bold_bytes: Arc::new(super::fonts::DEJAVU_SANS_BOLD.to_vec()),
            minijinja_env: Arc::new(super::minijinja_env::build_safe_env()),
        }
    }

    pub fn render_docspec(&self, spec: &super::docspec::DocSpec) -> Result<Vec<u8>, AppError> {
        // Source: docs.rs/krilla/0.7.0/krilla/ — Document::new, start_page_with(PageSettings::from_wh),
        // page.surface(), Font::new(bytes_into, index), surface.draw_text(Point, Font, size, &str, false, TextDirection::Auto)
        let mut doc = Document::new();
        let font_regular = Font::new(self.font_regular_bytes.as_ref().clone().into(), 0)
            .map_err(|e| AppError::Internal {
                source_chain: format!("krilla Font::new(regular): {e}")
            })?;
        let font_bold = Font::new(self.font_bold_bytes.as_ref().clone().into(), 0)
            .map_err(|e| AppError::Internal {
                source_chain: format!("krilla Font::new(bold): {e}")
            })?;
        // A4 portrait in PDF points = 595.276 x 841.890
        let mut page = doc.start_page_with(PageSettings::from_wh(595.276, 841.890).unwrap());
        let mut surface = page.surface();
        // Walk DocSpec.sections and emit draw_text calls with current y-cursor
        let mut y = 50.0;
        for section in &spec.sections {
            y = render_section(&mut surface, section, &font_regular, &font_bold, y);
        }
        surface.finish();
        page.finish();
        let bytes: Vec<u8> = doc.finish()
            .map_err(|e| AppError::Internal {
                source_chain: format!("krilla doc.finish: {e}")
            })?
            .into();
        Ok(bytes)
    }
}
```

`[CITED: docs.rs/krilla/0.7.0/krilla/]` confirms: `Document::new()`, `start_page_with(PageSettings::from_wh(w,h))`, `page.surface()`, `Font::new(bytes.into(), index)`, `surface.draw_text(Point, &Font, size, &str, false, TextDirection::Auto)`, `doc.finish() -> Vec<u8>`.

### Anti-Patterns to Avoid

- **DO NOT** обходить `WriterHandle::execute` — никаких прямых `repo.create(conn, ...)` где-то ещё. Если что-то делает write — оно через writer.
- **DO NOT** open `Connection::open` где-то кроме `AppCtx::build` (проб-чтение в Phase 1 — единственное исключение).
- **DO NOT** хранить `Vec<u8>` PDF в БД на Phase 3 (что-то типа кэша — не делаем; всегда генерируем на лету; шаблоны меняются).
- **DO NOT** позволять MiniJinja `{% include %}`, `{% extends %}` с файлами — нет loader = нельзя; не пытайтесь «упростить» добавлением loader'а.
- **DO NOT** держать в DocSpec поля типа `raw_pdf_op: Vec<u8>` — DocSpec должен быть полностью typed/serializable enum.
- **DO NOT** делать `format_act_number` в SQL (CASE WHEN) — делаем в Rust, чтобы тест-покрытие и unit-тесты были на Rust-стороне.
- **DO NOT** использовать `Date::now()` или `chrono::Local::now()` — `clippy disallowed-methods` блокирует; используйте инжектированный `Clock`.
- **DO NOT** писать в `org.json` из приложения в Phase 3 (только чтение + placeholder создание при первом запуске).
- **DO NOT** использовать `dirs::data_dir()` для tmp PDF — `Paths::root()` + поддиректория `tmp/`.
- **DO NOT** в `<iframe>` src давать `data:application/pdf;base64,...` (long URLs ломаются) — только `URL.createObjectURL(blob)`.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Atomic per-name counter | Свой mutex + INSERT OR UPDATE | `UPDATE counters SET cv=cv+1 RETURNING cv` под `BEGIN IMMEDIATE` в writer task | SQLite + single writer = бесплатная гарантия; RETURNING избегает round-trip |
| PDF generation с кириллицей | printpdf + ручной font embed | krilla 0.7 с include_bytes! DejaVu Sans | printpdf default Windows-1252 → mojibake; krilla subsetting work-out-of-the-box |
| Cyrillic-safe font (с глифом ё, диакритиками) | Подбор шрифтов вручную | DejaVu Sans Regular+Bold (public-domain-derived) | Покрывает кириллицу + большинство диакритиков; лицензия позволяет embed без атрибуции |
| Шаблонизатор с условиями/итерациями | String formatting | MiniJinja safe-mode | Готовый Jinja-like синтаксис; safe-mode без exec |
| PDF preview в browser/webview | Кастомный canvas renderer | pdfjs-dist в blob: iframe | Mozilla's PDF.js — де-факто стандарт; работает в WebView2/WKWebView/любом браузере |
| File save dialog в Tauri | Свой dialog или `tauri::api::dialog::*` (deprecated v1) | `tauri-plugin-dialog` v2 (уже подключен в Phase 2) | v2 plugin модель; работает на всех 3 ОС |
| «Open in system viewer» | Свой shell-exec | `tauri-plugin-shell::open(path)` | Уже в стеке v2 |
| Date formatting RU «28 мая 2026 г.» | Hardcoded strings | `time` crate с custom format_description | Уже в стеке (Phase 1); chrono запрещён |
| FTS query escaping | Свой escape | `build_fts_query` уже написан в Phase 2 (`crates/trackly-app/src/services/device_service.rs`) | Token quote + asterisk; переиспользуем 1:1 |
| Audit_log payload schema | JSON-строки runtime | `serde_json::json!({...})` с типизированными struct | Type-safety + миграционная читаемость |
| Undo state restore | Diff-patch система | Read full row snapshot (DeviceRow), UPDATE-overwrite | Audit_log уже хранит полный row; не нужен diff |
| pdf-extract для CI test | regex по PDF stream | `pdf-extract` crate | Уже maintenant; работает на DejaVu-embedded PDF |

**Key insight:** Все «трудные» задачи Phase 3 — counter, undo, PDF — уже имеют canonical Rust решения. Hand-rolling в любой из этих зон = bug factory.

## Runtime State Inventory

> Phase 3 — **не** рефактор/rename; пропускаем по правилам секции. Greenfield на старте: новые таблицы (ACT/return data), новые UI routes, новые npm dep (pdfjs-dist), новые fonts (DejaVu Sans Regular/Bold) — все добавляются «с нуля». Существующее runtime state (V004 acts pустая, V007 document_templates пустая, V009 counter act_number=0) не требует миграции данных, только заполнения новыми записями.

## Common Pitfalls

### Pitfall 1: Counter инкремент через два запроса (race condition)
**What goes wrong:** SELECT → счёт → UPDATE без `BEGIN IMMEDIATE` → две job выдают одинаковый номер.
**Why it happens:** Соблазн раздельных запросов «для читаемости».
**How to avoid:** Только `UPDATE ... RETURNING` под `BEGIN IMMEDIATE` (D-Counter-Acts-01). Тест: 50 параллельных create через `tokio::join_all` — все 50 номеров должны быть уникальны.
**Warning signs:** Любой код, который делает SELECT → mutation → UPDATE counter в одном WriterHandle::execute. Должно быть наоборот: counter инкремент в начале транзакции.

### Pitfall 2: `act_items` остаются после soft-delete акта
**What goes wrong:** ACT soft-delete (deleted_at_utc=?) → FK CASCADE не срабатывает (он только на hard DELETE) → `act_items.act_id` ссылается на «мёртвый» акт → отчёты ломаются.
**Why it happens:** Привычка опираться на FK CASCADE.
**How to avoid:** В той же транзакции — `tx.execute("DELETE FROM act_items WHERE act_id=?1", [act_id])`. Это hard-delete на junction (junction не имеет standard4 колонок per D-Schema-03).
**Warning signs:** Sanity test после delete акта — SELECT COUNT(*) FROM act_items WHERE act_id = ?(soft-deleted) → должно быть 0.

### Pitfall 3: Кириллические глифы рендерятся как «?» или пустые квадраты в PDF
**What goes wrong:** krilla без font subsetting попадает на глиф, которого нет в font → отрисовывает .notdef.
**Why it happens:** Шрифт без кириллических глифов (например, дефолтный Helvetica).
**How to avoid:** Embedded DejaVu Sans Regular+Bold — покрывает кириллицу + большинство диакритиков. На plan 01 фикстура «Сидоров-Петроградский Иван Александрович (ё) №42» должна **визуально** рендериться корректно (verify через pdf-extract = строка содержится в text content).
**Warning signs:** SHA256 hash меняется между runs, или extracted text содержит `?` или mojibake.

### Pitfall 4: SHA256 hash PDF не стабилен между OS
**What goes wrong:** krilla PDF включает CreationDate (`/CreationDate (D:2026...)` в /Info dict), Producer string с версией крейта, или font subset prefix (`ABCDEF+DejaVuSans`) — что-то из этого не deterministic.
**Why it happens:** Большинство PDF библиотек инкорпорируют текущее время и/или случайные UUID.
**How to avoid:** **Проверить на plan 01:** сгенерировать тот же PDF 2 раза подряд → assert hash равны. Если нет — копать krilla API на наличие `set_creation_date(None)` / `set_producer(None)` / детерминированный subset hash. Если krilla не позволяет — это становится bin-blocker → spike typst-as-lib (per D-PDF-Engine-01).
**Warning signs:** CI hash test флакает; same input → разный hash на двух consecutive runs.

`[CITED: docs.rs/krilla/0.7.0/krilla/]` API showed `Document::new()` и `doc.finish()` — деталей о creation_date / producer не извлечено из WebFetch. **План 01 ОБЯЗАН** verify это первой задачей и at minimum: (a) проверить krilla `Document` или `SerializeSettings` есть метод подавить timestamp; (b) если нет — обернуть byte-stream постпроцессингом, регексом заменив `/CreationDate` на фиксированный, или принять determinism только на same-machine same-day basis. **Это Open Question 4 ниже.**

### Pitfall 5: Корректный suffix «в»/«в1»/«в2» с retroactive renaming
**What goes wrong:** Первый возврат отображается «42в». Создаём второй — оба должны стать «42в1» и «42в2». Если query закэширована — UI показывает «42в» + «42в2» (рассогласование).
**Why it happens:** Format-rule зависит от `sibling_return_count` — это динамический подсчёт.
**How to avoid:** Не кэшировать форматированный number в БД; всегда вычислять `format_act_number(act, return_count)` при чтении. Уверенность для UI: invalidate `acts list` query после любого `acts_return`.
**Warning signs:** UI показывает разные suffix для двух возвратов одного и того же handover.

### Pitfall 6: MiniJinja UndefinedBehavior::Strict падает на легитимных Optional полях
**What goes wrong:** В шаблоне `{{ act.deadline }}` — но `act.deadline = None`. `UndefinedBehavior::Strict` → render error → пользователь не может напечатать акт без deadline.
**Why it happens:** Strict отвергает любой undefined; None в serde_json становится `null` (defined) но `null|default("—")` или `{% if act.deadline %}` нужны.
**How to avoid:** Дефолтный шаблон должен везде использовать `{% if act.deadline %}...{% endif %}` или `{{ act.deadline | default("—") }}`. Тест: render handover-акт без deadline → success + PDF содержит «—» или пропускает строку.
**Warning signs:** Validation error от render на легитимном создании без optional поля.

### Pitfall 7: org.json в портабельной сборке оказывается в `%APPDATA%`
**What goes wrong:** Код пишет «`dirs::config_dir().join("org.json")`» — `clippy disallowed-methods` молчит если кто-то bypass'нул, и portable invariant нарушается.
**Why it happens:** Привычка из не-portable приложений.
**How to avoid:** **Только** через `Paths::root().join("org.json")`. CI ProcMon тест (FOUND-11) поймает run-time, но добавьте unit-тест для `OrganizationService::file_path` который проверяет, что начинается с `paths.root()`.
**Warning signs:** Self-test после Phase 3 создаёт `org.json` где-то вне `<exe_dir>/`.

### Pitfall 8: pdfjs-dist worker не загружается в Tauri webview
**What goes wrong:** pdfjs-dist по умолчанию ожидает worker как отдельный JS файл; в Tauri custom protocol может не найти.
**Why it happens:** pdfjs-dist использует `import.meta.url` resolution.
**How to avoid:** В Vite сконфигурировать pdfjs worker как URL import: `import workerSrc from "pdfjs-dist/build/pdf.worker.min.js?url"; GlobalWorkerOptions.workerSrc = workerSrc;`. ИЛИ использовать `<iframe>` вместо PDFViewer API — у iframe нет worker requirements, webview сам решает (Tauri WebView2 имеет нативный PDF viewer; Safari/Chromium тоже). **D-Print-UX-01 явно выбирает iframe-путь**, что обходит этот pitfall полностью.
**Warning signs:** В консоли webview: «Setting up fake worker failed» или «Cannot load PDF.js worker».

### Pitfall 9: `payload_json` LIKE-фильтр медленный (full table scan)
**What goes wrong:** `WHERE payload_json LIKE '%"act_id":N%'` без индекса → full scan audit_log (растёт со временем).
**Why it happens:** payload_json — TEXT без extracted-column индекса.
**How to avoid:** Migration V014 добавляет computed-column index или generated column:
```sql
-- generated column extract act_id
ALTER TABLE audit_log
  ADD COLUMN act_id_indexed INTEGER GENERATED ALWAYS AS (
    CASE WHEN payload_json IS NOT NULL
         THEN json_extract(payload_json, '$.act_id')
         ELSE NULL END
  ) VIRTUAL;
CREATE INDEX idx_audit_log_act_id ON audit_log(act_id_indexed) WHERE act_id_indexed IS NOT NULL;
```
ИЛИ просто индекс по `(entity_type, entity_id)` плюс query `WHERE entity_type='device' AND entity_id IN (SELECT device_id FROM act_items WHERE act_id=?)` (избегает payload_json LIKE).
**Warning signs:** Undo time линейно растёт с размером audit_log.

### Pitfall 10: tokio::time::timeout вокруг spawn_blocking не убивает потоковую блокировку
**What goes wrong:** MiniJinja попал в бесконечный цикл; tokio timeout срабатывает на future, но spawn_blocking тред жив и потребляет CPU.
**Why it happens:** spawn_blocking не отменяется по cancel.
**How to avoid:** `env.set_fuel(Some(100_000))` ограничивает MiniJinja по инструкциям — фактическая защита. Timeout 5s — soft guarantee для пользователя, fuel — hard guarantee для процесса.
**Warning signs:** В нагрузочном тесте CPU 100% после render timeout.

## Code Examples

### Atomic counter increment with RETURNING

```rust
// Source: V009 schema + SQLite 3.35+ RETURNING (rusqlite 0.38 bundles SQLite 3.45)
fn next_act_number(tx: &rusqlite::Transaction) -> Result<i64, AppError> {
    tx.query_row(
        "UPDATE counters
            SET current_value = current_value + 1
          WHERE name = 'act_number'
        RETURNING current_value",
        [],
        |r| r.get::<_, i64>(0),
    ).map_err(map_rusqlite)
}
```

### Sub-number for return inside same transaction

```rust
fn next_sub_number_for_parent(tx: &rusqlite::Transaction, parent_act_id: i64) -> Result<i64, AppError> {
    tx.query_row(
        "SELECT COALESCE(MAX(sub_number), 0) + 1
           FROM acts
          WHERE parent_act_id = ?1
            AND deleted_at_utc IS NULL",
        [parent_act_id],
        |r| r.get::<_, i64>(0),
    ).map_err(map_rusqlite)
}
```

### Recompute parent.archived after return create/delete

```rust
fn recompute_parent_archived(
    tx: &rusqlite::Transaction,
    parent_act_id: i64,
    device_repo: &impl DeviceRepository,
) -> Result<(), AppError> {
    // For each device originally in handover, check current status
    let in_work_status: i64 = tx.query_row(
        "SELECT id FROM device_statuses WHERE code='в_работе'", [], |r| r.get(0)
    ).map_err(map_rusqlite)?;
    let remaining: i64 = tx.query_row(
        "SELECT COUNT(*)
           FROM act_items ai
           JOIN devices d ON d.id = ai.device_id
          WHERE ai.act_id = ?1
            AND d.status_id = ?2",
        [parent_act_id, in_work_status],
        |r| r.get(0),
    ).map_err(map_rusqlite)?;
    let archived: i64 = if remaining == 0 { 1 } else { 0 };
    tx.execute(
        "UPDATE acts SET archived = ?1, updated_at_utc = strftime('%s','now'),
                         version = version + 1
          WHERE id = ?2",
        [archived, parent_act_id],
    ).map_err(map_rusqlite)?;
    Ok(())
}
```

### FTS search across acts joining act_items + devices_fts

```sql
-- Phase 2 introduced devices_fts (V013); Phase 3 extends search to acts
-- Option A: union of two searches (acts text vs device text)
WITH act_text_hits AS (
    SELECT a.id
      FROM acts a
     WHERE a.deleted_at_utc IS NULL
       AND (CAST(a.number AS TEXT) LIKE ?1
            OR a.giver_name LIKE ?1
            OR a.receiver_name LIKE ?1)
),
device_text_hits AS (
    SELECT DISTINCT ai.act_id AS id
      FROM act_items ai
      JOIN devices_fts f ON f.rowid = ai.device_id
     WHERE devices_fts MATCH ?2  -- pre-built FTS query
)
SELECT a.* FROM acts a
 WHERE a.id IN (SELECT id FROM act_text_hits UNION SELECT id FROM device_text_hits)
   AND a.deleted_at_utc IS NULL
 ORDER BY a.created_at_utc DESC
 LIMIT ?3 OFFSET ?4
```

Параметр `?1` — `%term%` (LIKE), `?2` — FTS5 query из `build_fts_query` Phase 2.

### MiniJinja template — act_handover.minijinja (skeleton)

```jinja
{
  "title": "Акт приёма-передачи №{{ act.number }}{{ act.suffix }}",
  "header": {
    "org_name": {{ org.name | tojson }},
    "org_inn": {{ org.inn | tojson }},
    "org_address": {{ org.address | tojson }},
    "logo_path": {{ org.logo_path | tojson }}
  },
  "sections": [
    { "Heading": { "level": 1, "text": "Акт приёма-передачи №{{ act.number }}{{ act.suffix }}" } },
    { "Spacer": { "height_pt": 12 } },
    { "KeyValueTable": { "rows": [
        ["Дата", {{ act.date_human | tojson }}],
        ["Сдал", {{ act.giver_name | tojson }}],
        ["Принял", {{ act.receiver_name | tojson }}],
        ["Расположение", {{ act.location_name | default("—") | tojson }}]
        {% if act.deadline %}, ["Сроком до", {{ act.deadline_human | tojson }}]{% endif %}
    ] } },
    { "Spacer": { "height_pt": 16 } },
    { "ItemsTable": {
        "columns": ["№", "Наименование", "Инв. №", "Серийный №", "Модель", "Кол-во"],
        "rows": [
            {% for item in act.items %}
            [ "{{ loop.index }}",
              {{ item.name | tojson }},
              {{ item.inventory_no | default("—") | tojson }},
              {{ item.serial_no | default("—") | tojson }},
              {{ item.model | default("—") | tojson }},
              "{{ item.quantity | default(1) }}"
            ]{% if not loop.last %},{% endif %}
            {% endfor %}
        ]
    } },
    { "Spacer": { "height_pt": 40 } },
    { "Signature": {
        "left_label": "Сдал: ____________ / {{ act.giver_name }} /",
        "right_label": "Принял: ____________ / {{ act.receiver_name }} /",
        "spacer_pt": 30
    } }
  ]
}
```

### DocSpec struct (Rust target of template render)

```rust
// crates/trackly-app/src/pdf/docspec.rs
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DocSpec {
    pub title: String,
    pub header: HeaderBlock,
    pub sections: Vec<Section>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HeaderBlock {
    pub org_name: String,
    pub org_inn: Option<String>,
    pub org_address: Option<String>,
    pub logo_path: Option<String>,  // relative to Paths::root(); renderer resolves
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Section {
    Paragraph { text: String, #[serde(default)] bold: bool },
    Heading { level: u8, text: String },
    KeyValueTable { rows: Vec<[String; 2]> },
    ItemsTable { columns: Vec<String>, rows: Vec<Vec<String>> },
    Signature { left_label: String, right_label: String, #[serde(default = "default_spacer_pt")] spacer_pt: f32 },
    Spacer { height_pt: f32 },
}

fn default_spacer_pt() -> f32 { 24.0 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn round_trip_full_doc() {
        let spec = DocSpec {
            title: "Акт №42".into(),
            header: HeaderBlock {
                org_name: "ООО Ромашка".into(),
                org_inn: Some("7700000000".into()),
                org_address: Some("Москва".into()),
                logo_path: None,
            },
            sections: vec![
                Section::Heading { level: 1, text: "Акт №42".into() },
                Section::KeyValueTable {
                    rows: vec![["Дата".into(), "28 мая 2026 г.".into()]]
                },
            ],
        };
        let json = serde_json::to_string(&spec).expect("ser");
        let back: DocSpec = serde_json::from_str(&json).expect("deser");
        assert_eq!(back, spec);
    }
}
```

### org.json sample (placeholder content)

```json
{
  "name": "Ваша организация",
  "inn": "0000000000",
  "kpp": "000000000",
  "address": "Укажите адрес в settings/org.json",
  "logo_path": "logo.png"
}
```

**Создание** (один раз при первом старте, если файл отсутствует):

```rust
// crates/trackly-app/src/services/organization_service.rs
const PLACEHOLDER: &str = r#"{
  "name": "Ваша организация",
  "inn": "0000000000",
  "kpp": "000000000",
  "address": "Укажите адрес в settings/org.json",
  "logo_path": "logo.png"
}
"#;

impl OrganizationService {
    pub fn read_or_seed(&self) -> Result<OrgDto, AppError> {
        let path = self.paths.root().join("org.json");
        if !path.exists() {
            std::fs::write(&path, PLACEHOLDER).map_err(|e| AppError::Internal {
                source_chain: format!("write placeholder org.json: {e}"),
            })?;
            tracing::warn!(
                path = %path.display(),
                "org.json не найден — создан placeholder; заполните данные организации"
            );
        }
        let bytes = std::fs::read(&path).map_err(|e| AppError::Internal {
                source_chain: format!("read org.json: {e}"),
        })?;
        serde_json::from_slice::<OrgDto>(&bytes).map_err(|e| AppError::Validation {
            field: "org.json".into(),
            message: format!("Некорректный JSON: {e}"),
        })
    }
}
```

### MiniJinja context shape (что шаблон видит)

```rust
// Built by act_service before render:
let ctx = serde_json::json!({
    "org": {
        "name": org.name,
        "inn": org.inn,
        "kpp": org.kpp,
        "address": org.address,
        "logo_path": org.logo_path,  // absolute path joined with Paths::root() OR None
    },
    "act": {
        "number": act.number,        // i64 — 42
        "suffix": act_suffix,        // "" or "в" or "в1" (computed at render time, snapshot)
        "date": act.date_iso,        // "2026-05-28"
        "date_human": format_ru_date(act.date_unix),  // "28 мая 2026 г."
        "giver_name": act.giver_name,
        "receiver_name": act.receiver_name,
        "deadline": act.deadline_iso,           // null or "2026-08-28"
        "deadline_human": deadline_ru,          // null or "28 августа 2026 г."
        "location_name": location.name,         // joined
        "items": items.iter().map(|i| json!({
            "name": i.device_name,
            "inventory_no": i.inventory_no,    // option
            "serial_no": i.serial_no,
            "model": i.model,
            "specs": i.specs,
            "kit": i.complectation,
            "condition": i.condition_at_time,
            "quantity": i.quantity,
        })).collect::<Vec<_>>(),
        "parent": if let Some(p) = parent {
            json!({ "number": p.number, "date_human": format_ru_date(p.date_unix) })
        } else { Value::Null },
    },
    "return": {
        "condition_default": return_bulk.condition,  // null on handover
        "location_default": return_bulk.location_name,
    },
});
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| printpdf + ручной font embed для Cyrillic | krilla 0.7 с subsetting + OpenType первого класса | krilla GA ~2024-09; 0.7.0 — 2026-03-31 | Cyrillic «just works»; меньше ручной геометрии |
| HTML-to-PDF через wkhtmltopdf | Typst или krilla напрямую | wkhtmltopdf abandoned ~2022; krilla — pure Rust | Нет внешних бинарей в portable bundle |
| Tera как Rust Jinja-like | MiniJinja (Armin Ronacher) | MiniJinja 1.0 — 2023; 2.0 — 2024 | Меньше зависимостей, async-friendly, лучшая безопасность по умолчанию |
| sqlx с SQLite для writes | rusqlite + single-writer task | См. CLAUDE.md «What NOT to Use» | Избегаем lock starvation footgun |
| Tauri 1 + plugin chrysalis | Tauri 2 stable plugin model | Tauri 2 GA — Oct 2024 | Capability/ACL + stable plugins |

**Deprecated/outdated:**
- pdfgen, lopdf без font embedding — не подходят для кириллицы.
- `tauri::api::dialog::*` (v1) — заменено `tauri-plugin-dialog` v2.
- MiniJinja без `set_fuel` (до 1.0.21) — fuel API стабилен в 2.x.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | krilla 0.7 даёт детерминированный PDF при отсутствии явных timestamp-генераций | Pitfall 4 + D-PDF-Engine-01 | План 01 (структурный spike) поймает на CI hash test; mitigation — post-process регексом или spike typst-as-lib |
| A2 | krilla `Document::new()` без explicit setter не вставляет CreationDate с текущим временем | Pattern 5 | Тот же spike — поймает первой задачей |
| A3 | DejaVu Sans Regular+Bold (latest 2.37 release) покрывает все ФИО кириллических символов включая «Сидоров-Петроградский» и (ё) | Pitfall 3 | LOW риск — DejaVu широко используется; verify в plan 01 фикстуре |
| A4 | pdfjs-dist v4.x работает в Tauri WebView2 + WKWebView через `<iframe src="blob:">` без worker конфигурации | Pitfall 8 | Mozilla тестирует на всех major engine; iframe-путь обходит worker-issue |
| A5 | rusqlite bundled SQLite >= 3.35 (поддерживает `RETURNING`) | Code Examples Atomic counter | rusqlite 0.38 bundles SQLite 3.45+; verified |
| A6 | `json_extract(payload_json, '$.act_id')` доступен в rusqlite bundled | Pattern 3 Undo | JSON1 — компилируется by default since SQLite 3.38; rusqlite bundled включает |
| A7 | `payload_json LIKE '%"act_id":N%'` достаточно быстр для Phase 3 scale (~хунды актов) | Pitfall 9 | LOW; Phase 7 может добавить generated column index когда retention заработает |
| A8 | MiniJinja `set_fuel(Some(100_000))` достаточен для render «акт на 50 позиций» | Pattern 4 | UNKNOWN — план 01 фикстура должна включать 8+ позиций и `set_fuel` обработку |
| A9 | krilla 0.7 MSRV 1.92 — текущий toolchain 1.88 нужно поднять | Standard Stack | План 01 первая задача — `rust-toolchain.toml` bump + verify CI matrix не сломалась |
| A10 | DocSpec можно полностью построить статически из шаблона — нет фич, где Rust «достраивает» секции после render | Pattern 5 | Если выяснится — план 01 разделит MiniJinja stage на «produce partial» + Rust post-processing |
| A11 | pdfjs-dist npm package 4.x работоспособен на современных webview без полифиллов | Standard Stack | Mozilla maintains; LOW риск |
| A12 | tauri-plugin-dialog `save` API возвращает absolute path при `default_path = paths.root().join("...")` — нужен plan 04 |  Code Examples PDF save | docs.rs/tauri_plugin_dialog: confirmed; LOW риск |

**If this table is empty:** не пусто — Phase 3 имеет 12 assumptions, главный риск (A1+A2 — детерминизм PDF) снимается plan 01 структурным spike'ом per D-PDF-Engine-01.

## Open Questions

1. **Модель количества/возврата для не-уникальных устройств**
   - What we know: V004 `act_items` колонок `quantity` НЕТ; есть только `device_id`. Для не-уникальных устройств одного типа сейчас невозможно записать «в одном акте — 5 штук модели X».
   - What's unclear: Один из вариантов — добавить `act_items.quantity INTEGER NOT NULL DEFAULT 1` (V014); другой — для каждой шт. отдельный device row (Phase 2 bulk-create позволяет до 100 за раз). Второй ближе к Phase 2 паттерну, но взрывает кол-во строк.
   - Recommendation: **Плану принимать решение** — рекомендую вариант с добавлением `quantity INTEGER NOT NULL DEFAULT 1` через V014; для частичного возврата нужна также `quantity_returned` или `act_item_returns(act_item_id, return_act_id, quantity)`. Решение зависит от UX: «вернули 3 из 5» хотим показать на одной row или разбивать. Из CONTEXT D-Acts-Return-01 видно «Количество к возврату (для не-уникальных)» — подтверждает quantity-модель.

2. **Migration V014 нужна ли (и что в неё положить)**
   - What we know: Текущий schema_version = 13 (Phase 2 V013). V004 acts достаточен для handover/return на уникальные устройства; V009 counter готов.
   - What's unclear: Минимально нужно: (a) `act_items.quantity` (см. Q1); (b) индекс по `(parent_act_id, sub_number)` для быстрого MAX query; (c) индекс по `audit_log(entity_type, entity_id, created_at_utc)` для быстрого undo lookup; (d) optionally — generated column для `payload_json.act_id`.
   - Recommendation: **План 02 (Acts CRUD) определит** V014 единой миграцией с (a)+(b)+(c). (d) — defer до Phase 7 (retention уже там).

3. **FTS search для актов — отдельная FTS5 таблица `acts_fts` или JOIN на existing `devices_fts`?**
   - What we know: Phase 2 создал `devices_fts` (FTS5 virtual table) для FTS поиска по устройствам.
   - What's unclear: Можно (a) расширить FTS — создать `acts_fts(number, giver_name, receiver_name)` с triggers, или (b) делать `LIKE '%term%'` на основных полях акта + UNION с `devices_fts MATCH ...` через JOIN act_items. Вариант (a) даёт ranked-поиск + ё/е-инсенситивность (как Phase 2); (b) проще, но slow на больших таблицах.
   - Recommendation: **План 05 (search)** — реализовать (b) для Phase 3 (просто, ≤500 актов в реальной нагрузке — LIKE OK), оставить TODO на (a) если Phase 7 reports потребуют.

4. **krilla detalee детерминизма (CRITICAL — потенциальный план 01 blocker)**
   - What we know: krilla 0.7 имеет stable API per docs.rs. WebFetch не извлёк информацию о creation date / Producer string controls.
   - What's unclear: Embeds ли `Document::finish()` текущую дату в /Info или /Metadata? Стабилен ли font subset prefix (6-char ABCDEF+FontName) между runs?
   - Recommendation: **План 01 первая задача** — голый рендер «Привет, мир» 2 раза → diff/sha256. Если разное — копать krilla `SerializeSettings` / `DocumentSettings` (если есть); если нет API — post-process регексом `/CreationDate (...)` и `/Producer (...)`. Если post-process тоже не помогает (subset prefix не стабилен) → SPIKE typst-as-lib (per D-PDF-Engine-01 fallback).

5. **«Применить ко всем» дефолт ВКЛ — корректно ли при per-row override**
   - What we know: D-Acts-Return-01 + specifics указывают «галочка по умолчанию ВКЛ (90% кейсов — bulk)».
   - What's unclear: Если bulk-default ВКЛ + пользователь правит per-row → должен ли per-row перекрывать или сначала снять галочку?
   - Recommendation: **Plan 03 (Returns)** — bulk-default — fallback; per-row override (если поле заполнено = override existed) **всегда побеждает**. Snapshot semantics: «applied bulk values» вычисляются один раз при submit, не реактивно.

6. **Логотип PNG/JPEG — должен ли rendrer warning'ом сообщать о слишком большом файле?**
   - What we know: krilla поддерживает embed PNG/JPEG; logo path — относительный к Paths::root().
   - What's unclear: Можно ли пользователь подсунуть 50MB логотип и взорвать PDF?
   - Recommendation: **Plan 04 (PDF endpoints)** — софт-limit 5MB для логотипа; warning в логи если больше; рендерить anyway (krilla сжимать всё равно умеет). Hard-cap не нужен в Phase 3.

7. **MSRV bump 1.88 → 1.92 — что ломается?**
   - What we know: Phase 1 зафиксировал MSRV 1.88; krilla 0.7 требует 1.92 (CONTEXT.md D-PDF-Engine-01).
   - What's unclear: Деферd item Phase 1 «Windows 7 32-bit MSRV check» — это окончательно его deferr'ит как «не для v1»?
   - Recommendation: **Plan 01 первая задача** — `rust-toolchain.toml` bump до 1.92 + CI matrix update + `.planning/phases/01-foundation/deferred-items.md` mark Win7 как «v2-deferred». Если в CI matrix runner ≥ 1.92 — нет issues; в production toolchain пользователя — тоже (используем stable).

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain ≥ 1.92 | krilla 0.7 compile | likely ✓ (latest stable >> 1.92) | check `rustup update stable` | — |
| `cargo` (workspace pin from Phase 1) | All Rust builds | ✓ | per `rust-toolchain.toml` | — |
| `pnpm` 10.17 | UI deps (pdfjs-dist) | ✓ | per ui/package.json packageManager | — |
| Node.js ≥ 20 | UI build | ✓ | per ui/package.json engines | — |
| `sqlite` (bundled in rusqlite) | All DB | ✓ | 3.45+ via rusqlite 0.38 bundled | — |
| Tauri 2 CLI | dev / build | ✓ | per ui/package.json | — |
| DejaVu Sans .ttf files | Embed | ✗ (нужно скачать) | DejaVu 2.37 | Download from https://dejavu-fonts.github.io/Download.html → place in `crates/trackly-app/assets/fonts/` |
| `pdf-extract` (dev-dep) | PDF text-extract test | will be added | 0.10.0 | — |

**Missing dependencies with no fallback:**
- **DejaVu Sans .ttf** (Regular + Bold) — план 01 первая задача добавляет файлы; нет fallback (без шрифта нет кириллицы → весь Phase 3 встаёт).

**Missing dependencies with fallback:**
- nothing critical.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust `cargo test` (sync + `tokio::test`) + Svelte: `pnpm svelte-check` (no Vitest yet per Phase 2) |
| Config file | per-crate `Cargo.toml` `[[test]]` entries; `ui/tsconfig.json` for svelte-check |
| Quick run command | `cargo test -p trackly-app --tests acts_crud acts_returns acts_undo` |
| Full suite command | `cargo test --workspace && pnpm -C ui svelte-check` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|--------------|
| ACT-01 | Create handover with valid payload returns ActDto | integration | `cargo test -p trackly-app --test acts_crud create_handover_happy` | ❌ Wave 0 |
| ACT-02 | acts_counts returns correct counts per switch-bar | integration | `cargo test -p trackly-app --test acts_crud counts_match_switch_bar` | ❌ Wave 0 |
| ACT-03 | Auto-number вернул следующий; override → audit_log row | integration | `cargo test -p trackly-app --test acts_crud override_number_audits` | ❌ Wave 0 |
| ACT-04 | Search by number / ФИО / device name finds | integration | `cargo test -p trackly-app --test acts_search` | ❌ Wave 0 |
| ACT-05 | List of action commands available | smoke (frontend manual) | manual / Playwright deferred | — |
| ACT-06 | Delete handover restores devices | integration | `cargo test -p trackly-app --test acts_undo delete_handover_restores` | ❌ Wave 0 |
| ACT-07 | Return creates «42в» / «42в1» suffix | integration | `cargo test -p trackly-app --test acts_returns suffix_rules` | ❌ Wave 0 |
| ACT-08 | Return modal bulk + per-row override applied | unit + integration | `cargo test -p trackly-app --test acts_returns bulk_default_with_overrides` | ❌ Wave 0 |
| ACT-09 | Auto-archive at 100% return | integration | `cargo test -p trackly-app --test acts_returns auto_archive_full_return` | ❌ Wave 0 |
| ACT-10 | Delete return restores | integration | `cargo test -p trackly-app --test acts_undo delete_return_restores` | ❌ Wave 0 |
| ACT-11 | PDF render produces Cyrillic glyphs | integration | `cargo test -p trackly-app --test pdf_fixture cyrillic_glyphs_present` | ❌ Wave 0 |
| ACT-12 | Default templates seeded on first run | integration | `cargo test -p trackly-app --test templates_seed default_seeded` | ❌ Wave 0 |
| ACT-13 | Transactional guarantee (rollback on error) | integration | `cargo test -p trackly-app --test acts_crud rollback_on_device_update_failure` | ❌ Wave 0 |
| ACT-14 | Atomic counter under concurrent create | integration | `cargo test -p trackly-app --test acts_crud concurrent_50_creates_unique_numbers` | ❌ Wave 0 |
| DEV-14 | Acceptance doc render works | integration | `cargo test -p trackly-app --test pdf_fixture acceptance_renders` | ❌ Wave 0 |
| DEV-15 | Acceptance template seeded + render uses BD | integration | `cargo test -p trackly-app --test templates_seed acceptance_seeded_and_used` | ❌ Wave 0 |
| PDF determinism | Same input → same hash 2 consecutive runs | integration | `cargo test -p trackly-app --test pdf_fixture deterministic_hash` | ❌ Wave 0 (PLAN 01 first task) |
| Bindings.ts | Bindings file contains all new DTOs | integration | `cargo test -p trackly-app --test export_bindings` | ✅ EXISTS (Phase 1 — extend assertions) |
| All transports parity | Tauri build_* and axum handler return identical for same input | integration | per-command tests + reuse `tests/health_smoke.rs` pattern | partial — extend `acts_http_smoke.rs` |

### Sampling Rate

- **Per task commit:** `cargo test -p trackly-app --tests acts_<topic>` + `cargo test -p trackly-app --test export_bindings`
- **Per wave merge:** `cargo test --workspace`
- **Phase gate:** `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all -- --check && pnpm -C ui svelte-check && pnpm -C ui lint` зелёные перед `/gsd-verify-work`

### Wave 0 Gaps

- [ ] `crates/trackly-app/tests/pdf_fixture.rs` — covers ACT-11, DEV-14, PDF determinism, Cyrillic glyphs; **plan 01 first task** writes this BEFORE any real act endpoint
- [ ] `crates/trackly-app/tests/acts_crud.rs` — covers ACT-01, ACT-02, ACT-03, ACT-13, ACT-14
- [ ] `crates/trackly-app/tests/acts_returns.rs` — covers ACT-07, ACT-08, ACT-09
- [ ] `crates/trackly-app/tests/acts_undo.rs` — covers ACT-06, ACT-10
- [ ] `crates/trackly-app/tests/acts_search.rs` — covers ACT-04
- [ ] `crates/trackly-app/tests/templates_seed.rs` — covers ACT-12, DEV-15
- [ ] `crates/trackly-app/tests/acts_http_smoke.rs` — covers dual-transport parity (Tauri + axum same body)
- [ ] Extend `crates/trackly-app/tests/export_bindings.rs` — assert presence of `ActDto`, `ActCreateDto`, `ActReturnItem`, `ActsCountsDto`, `OrgDto`, `DocSpec` (если экспортируем)
- [ ] Add `pdf-extract = "0.10"` to `[dev-dependencies]` of trackly-app

*(Test infrastructure из Phase 1+2 уже на месте: `test_writer_and_readers` fixture, minimal_ctx pattern в каждом cmd test, 30-second tokio timeout guard.)*

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|------------------|
| V2 Authentication | no — Phase 5 | — |
| V3 Session Management | no — Phase 5 | — |
| V4 Access Control | no — Phase 5 (audit_log.user_id = NULL в Phase 3) | — |
| V5 Input Validation | yes | serde strict deser; act_service `validate_*`; UI inline validation |
| V6 Cryptography | yes (parties applied) | argon2id уже в Phase 1 (для будущих пользователей); никакого hand-rolling |
| V7 Error Handling and Logging | yes | unified `AppError`; `tracing::warn!` для org.json и template seeds; no PII в hash test |
| V8 Data Protection | yes | `payload_json` в audit_log содержит ФИО — это OK по бизнесу, не лекаем за пределы локальной БД |
| V9 Communications | no — Phase 5 enforce HTTPS | — |
| V14 Configuration | yes | Шаблоны в БД (под контролем admin); `org.json` rel-path (logo path traversal — см. ниже) |

### Known Threat Patterns for {krilla + MiniJinja + Tauri + axum} stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| MiniJinja template injection (user-supplied template body executes filter/macro with side-effect) | Tampering | safe-mode: `UndefinedBehavior::Strict`, no loader, no Python-bridge, `set_fuel`, `tokio::time::timeout` |
| Path traversal через `org.logo_path = "../../../etc/passwd"` | Information Disclosure | Renderer выполняет `paths.root().join(logo_path).canonicalize()` → reject if не под root |
| Path traversal через tmp PDF save | Information Disclosure | Save path выбран user via `tauri-plugin-dialog::save` (user explicit choice); open in viewer → temp в `paths.root().join("tmp/")` валидированный |
| Excel/CSV formula injection при export PDF | n/a (PDF не выполняет формулы) | — (только CSV — Phase 2 уже mitigated) |
| Unbounded payload size в `acts_create(items)` | DoS | `act_service::validate_new`: items.len() ≤ 100 |
| MiniJinja CPU bomb (большой loop) | DoS | `set_fuel(Some(100_000))` hard-cap |
| Race condition counter giving same number twice | Conflict / Data Integrity | Single-writer + `BEGIN IMMEDIATE` + `UPDATE ... RETURNING` |
| Soft-deleted act number reuse | Data Integrity / Audit | Validation в `acts_create`: SELECT WHERE number=? без deleted_at_utc filter; reject reuse |
| User-supplied number override → audit completeness | Repudiation | `audit_log` row с `action='custom:act_number_override'`, payload `{requested, next_auto_would_be}` |

## Sources

### Primary (HIGH confidence)

- `[Context7-equivalent: docs.rs/krilla/0.7.0/krilla/]` — API surface verified via WebFetch
- `[Context7-equivalent: docs.rs/minijinja/latest/minijinja/struct.Environment.html]` — safe-mode setters verified via WebFetch
- `crates.io API` — krilla 0.7.0 (newest, 2026-03-31), minijinja 2.20.0 (2026-05-19), pdf-extract 0.10.0
- `CLAUDE.md` — locked stack: krilla 0.7 default, Typst-as-lib backup; rusqlite 0.38 + refinery 0.9 + single-writer; portable + НЕ-dirs::*
- `.planning/PROJECT.md` — core value «одной кнопкой»
- `.planning/REQUIREMENTS.md §Acts (ACT)` — ACT-01..14 formulations
- `.planning/ROADMAP.md §Phase 3` — 5 success criteria
- `migrations/V004__acts.sql` — schema acts + act_items + UNIQUE(number, COALESCE(sub_number,0)) WHERE not deleted
- `migrations/V007__document_templates.sql` — `kind IN ('act_handover', 'act_acceptance')`, body_minijinja TEXT, partial UNIQUE
- `migrations/V008__audit_log.sql` — before_json/after_json/payload_json
- `migrations/V009__counters.sql` — seeded `act_number=0`, `cartridge_seq=0`
- `.planning/phases/01-foundation/01-04-SUMMARY.md` — `WriterHandle::execute`, `ReaderPool::acquire`, `map_rusqlite`, `AppError 9-variant`, AppCtx Arc-clone composition
- `.planning/phases/01-foundation/01-05-SUMMARY.md` — `build_*` helper pattern, dual-transport через тонкие adapter'ы, HealthDto DTO эталон, specta export pattern, sibling-marker для AppError
- `.planning/phases/02-ui/02-PATTERNS.md` — pattern matrix Phase 2 → Phase 3 (DTO derive, Tauri command+specta order, axum 0.8 `{id}` path syntax, 30-sec timeout guard, minimal_ctx helper)
- `.planning/phases/02-ui/02-04-SUMMARY.md` — FTS5 search pattern (`build_fts_query`), AutocompleteField enum, status switch-bar
- `.planning/phases/02-ui/02-05-SUMMARY.md` — `tauri-plugin-dialog` уже в стеке; FS helpers pattern; UTF-8 BOM CSV (no transfer to PDF)
- `.planning/phases/03-pdf/03-CONTEXT.md` — все D-* decisions для Phase 3 (locked)

### Secondary (MEDIUM confidence)

- DejaVu Fonts — https://dejavu-fonts.github.io/License.html (public-domain-derived; embed без атрибуции); coverage Latin + Cyrillic + extended Latin diacritics
- PDF.js — https://github.com/mozilla/pdf.js (Mozilla; де-факто стандарт; pdfjs-dist npm)
- pdf-extract crate — https://docs.rs/pdf-extract/ (для text-extract test)
- SQLite JSON1 — `json_extract` since 3.38, rusqlite bundled включает по умолчанию

### Tertiary (LOW confidence)

- Точная стабильность krilla `Document::finish()` output между runs — **PLAN 01 verifies, не ASSUMED-blocking**
- pdfjs-dist v4 worker resolution в Tauri WebView2 без extra config — обходим через iframe-путь

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — все 4 ключевых крaте (krilla 0.7, minijinja 2.20, pdf-extract 0.10, pdfjs-dist 4) проверены на crates.io/npm + cross-referenced с CLAUDE.md
- Architecture: HIGH — повторяет Phase 1 (single-writer) + Phase 2 (hexagonal, DTO, dual-transport) паттерны; новые элементы (PDF pipeline, MiniJinja) изолированы в новом модуле `pdf/`
- Pitfalls: MEDIUM-HIGH — 9 pitfalls покрыты из обоих источников (research + CONTEXT); основной риск (krilla determinism) явно адресован планом 01
- Open Questions: 7 — главный (Q4 krilla determinism) blocking для плана 01; остальные плана-level decisions

**Research date:** 2026-05-28
**Valid until:** 2026-06-28 (стабильный стек; krilla/minijinja версии стабильны; пересмотр если krilla > 0.7 выйдет до начала Phase 3 execution)

## RESEARCH COMPLETE

Phase 3 — композиция установленных Phase 1/2 паттернов (single-writer, hexagonal, dual-transport через `build_*`, DTO+specta, FTS5) с **одним новым модулем** `crates/trackly-app/src/pdf/` (krilla 0.7 + MiniJinja safe-mode + DocSpec-IR + embedded DejaVu Sans) и **одним новым UI feature folder** `ui/src/features/acts/` (master-detail + return modal + pdfjs-dist preview). Все 16 требований (ACT-01..14 + DEV-14..15) покрыты с явным маппингом на тесты. Главный технический риск (детерминированность krilla PDF между OS-runners) явно изолирован в **plan 01 структурном spike** — если plan 01 hash-test провалится, разворачивается отдельная mini-фаза typst-as-lib, иначе фаза идёт согласно 5-плана последовательности (PDF foundation → Acts CRUD → Returns+Archive+Undo → Templates+Org+PDF endpoints → DEV-14+Search+Polish). 12 assumptions явно задокументированы; 7 open questions требуют решения плановщика (Q1 act_items.quantity, Q2 V014 migration scope, Q3 acts_fts vs LIKE+UNION, Q4 krilla determinism, Q5 bulk default semantics, Q6 logo size cap, Q7 MSRV bump). Готово к `/gsd-plan-phase` integrated step.
