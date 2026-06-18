---
gsd_state_version: 1.0
milestone: v1.0
milestone_name: milestone
status: planning
last_updated: "2026-06-18T17:10:55.574Z"
last_activity: "2026-06-18 - Completed quick task 260618-vtm: backup-date-schedule-template-fixes"
progress:
  total_phases: 11
  completed_phases: 10
  total_plans: 62
  completed_plans: 62
  percent: 91
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-05-24)

**Core value:** Учёт устройств и картриджей с актами приёма-передачи и историей перемещений должен работать надёжно и быстро в режиме «одной кнопкой» — без обращения к Excel-таблицам, ручного присвоения номеров актов или потери истории при возврате на склад.
**Current focus:** Phase 8 — ad вход и релизный пайплайн

## Current Position

Phase: 8
Plan: Not started
Status: Ready to plan
Last activity: 2026-06-18 - Completed quick task 260618-vtm: backup-date-schedule-template-fixes

Progress: [██████████] 100%

### Phase 6 gap-closure decisions (2026-06-15)

- D-GAP-Printer-Add: принтер = устройство type=Принтер + опц. SNMP; завести вручную И через discovery; admit починить (PRN-04 USB).
- D-GAP-Replace-Select: Select принтера в форме замены = устройства type=Принтер (§427), не printers-таблица.
- D-GAP-Employee-Access: полноценный вход сотрудника → AD Phase 8; сейчас только корректный ролевой рендер.
- Критические дефекты: requests_create arg `dto` vs `payload`; requests_status_counts/get_history mismatch; printers_admit заглушка.

## Performance Metrics

**Velocity:**

- Total plans completed: 47
- Average duration: —
- Total execution time: —

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| — | — | — | — |
| 02 | 5 | - | - |
| 03.3 | 2 | - | - |
| 04 | 6 | - | - |
| 5 | 6 | - | - |
| 07 | 14 | - | - |

**Recent Trend:**

- Last 5 plans: —
- Trend: —

*Updated after each plan completion*
| Phase 01 P01 | 25 min | 4 tasks | 35 files |
| Phase 01 P02 | 7 min | 3 tasks | 10 files |
| Phase 01 P03 | 6 min | 3 tasks | 23 files |
| Phase 01 P04 | 25 min | 3 tasks | 24 files |
| Phase 01 P06 | 22 min | - tasks | - files |
| Phase 02-ui P01 | 20 min | 2 tasks | 29 files |
| Phase 02-ui P02-02 | 50m | 4 tasks | 46 files |
| Phase 02-ui P04 | 120 min | 3 tasks | 20 files |
| Phase 02-ui P05 | 240 | 3 tasks | 31 files |
| Phase 03-pdf P01 | 25 | 3 tasks | 19 files |
| Phase 03-pdf P02 | 90 | 3 tasks | 27 files |
| Phase 03 P03 | 60 | 2 tasks | 18 files |
| Phase 03 P04 | 75 | 2 tasks | 34 files |
| Phase 03 P05 | 60 | 2 tasks | 21 files |
| Phase 03.2-deferred-uat-gap-closure P02 | 15 | 2 tasks | 5 files |
| Phase 03.3 P01 | 20 | 3 tasks | 7 files |
| Phase 03.3 P02 | 5min | 3 tasks | 7 files |
| Phase 04 P01 | 6 | 2 tasks | 10 files |
| Phase 04 P02 | 6 min | 2 tasks | 6 files |
| Phase 04-cartridges P03 | 19 | 2 tasks | 23 files |
| Phase 05-auth-server-mode P02 | 95 | 2 tasks | 15 files |
| Phase 05 P03 | 24 | 2 tasks | 11 files |
| Phase 05-auth-server-mode P04 | 180 | 2 tasks | 8 files |
| Phase 05-auth-server-mode P05 | 17 | - tasks | - files |
| Phase 05 P06 | 20 min | 3 tasks | 3 files |
| Phase 06-snmp P01 | 22 | 2 tasks | 21 files |
| Phase 06-snmp P04 | 8 | 3 tasks | 14 files |
| Phase 06 P05 | 7 | 2 tasks | 10 files |
| Phase 06-snmp P06 | 11 | 2 tasks | 2 files |
| Phase 06-snmp P07 | 25 | 3 tasks | 8 files |
| Phase 06-snmp P08 | 7 | 2 tasks | 7 files |
| Phase 07 P01 | 5min | 2 tasks | 12 files |
| Phase 07 P03 | 52 | 2 tasks | 8 files |
| Phase 07 P04 | 4 | 2 tasks | 6 files |
| Phase 07-reports-dashboard-settings P05 | 4 | 2 tasks | 5 files |
| Phase 07 P07 | 120 | 2 tasks | 19 files |
| Phase 07 P10 | 176 | 2 tasks | 4 files |
| Phase 07-reports-dashboard-settings P11 | 8 | 1 tasks | 2 files |
| Phase 07 P13 | 2 | 2 tasks | 4 files |
| Phase 07 P14 | 15 | 2 tasks | 6 files |

## Accumulated Context

### Roadmap Evolution

- Phase 03.1 inserted after Phase 03: Acts quantity model + UAT gap closure (G-1..G-13)
- Phase 03.2 inserted after Phase 03.1: gap-closure deferred UAT items DEF-1/2/3 from Phase 03.1 (URGENT)
- Phase 03.3 inserted after Phase 03.2: Device-list UX round 2 — 4 UAT items after 03.2 (grouping condition column / cell tooltips / status column / location autocomplete) (URGENT)

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- **Roadmap:** Standard granularity, 8 phases sequential, MVP mode на всех фазах
- **Stack (locked):** rusqlite 0.39 + refinery 0.8 + split read/write pools + single-writer task; tauri 2.11 + svelte 5 + axum 0.8 + tower-sessions 0.13 + snmp2 0.4 + ldap3 0.12 + argon2 0.5 + rustls 0.23 + rcgen 0.13 + krilla 0.7 (default PDF)
- **«Расходник»:** ОСТАЁТСЯ как тип устройства (бумага, одноразовые флешки и пр.) — НЕ для картриджей; картриджи живут в собственном разделе
- **PDF engine:** krilla 0.7 default, Typst-as-lib — backup по итогам spike в Phase 3
- **Pantum auto-restart:** alert-only в v1 (PRN-06); авто-restart — v2 (PNT)
- [Phase ?]: Plan 01-01: MSRV 1.85 to 1.88 (Tauri 2 dep graph)
- [Phase ?]: Plan 01-01: rusqlite 0.39 to 0.38, refinery 0.8 to 0.9 (rusqlite-bundled feature)
- [Phase ?]: Plan 01-01: Included tauri-plugin-single-instance from Day 1 per RESEARCH Open Question 2
- [Phase ?]: Plan 01-01: ESLint 9 flat config (eslint.config.js); pnpm 10.17.1 pinned via packageManager field
- [Phase ?]: Plan 01-02: Paths::resolve_for_exe_dir is public (test seam)
- [Phase ?]: Plan 01-02: UNC rejection via simple starts_with(r"\\\\") prefix check
- [Phase ?]: Plan 01-02: AppError kept minimal (Internal + Validation); Plan 04 extends
- [Phase ?]: Plan 01-02: webview_env uses #[rustfmt::skip] at fn-level to preserve one-line unsafe contract
- [Phase ?]: Plan 01-03: embed_migrations!(../../migrations) from trackly-infra crate root — refinery 0.9 macro path form
- [Phase ?]: Plan 01-03: MigrationReport { schema_version: u32, applied_count: usize } — Plan 04 AppCtx hardcodes 12 for downgrade check
- [Phase ?]: Plan 01-03: test_db() public (not cfg test) — tempfile-backed, canonical fixture for all downstream integration tests
- [Phase ?]: Plan 01-03: WAL applied via apply_writer_pragmas BEFORE refinery — Pitfall #4 mitigated, idempotency test confirms
- [Phase ?]: Plan 01-03: act_items.condition_at_time TEXT (snapshot, not timestamp) and sessions.expiry_date INTEGER (tower-sessions convention) are allowlisted in timestamp invariant test
- [Phase ?]: Free-fn error mappers (map_rusqlite/refinery/send_timeout/oneshot_recv) instead of impl From — Rust orphan rule blocks impl in trackly-infra
- [Phase ?]: ReaderPool: simple std::sync::Mutex<Vec<Connection>> LIFO, panic on exhaustion accepted for Phase 1 (LAN scale); Phase 2+ can swap to deadpool
- [Phase ?]: Probe-read pattern: SQLITE_OPEN_READ_ONLY | SQLITE_OPEN_URI + explicit drop before writer open — guarantees byte-identical file on downgrade rejection (success criterion #4)
- [Phase ?]: rusqlite promoted to runtime dep of trackly-app for context.rs probe-read step; trackly-core remains rusqlite-free (no_io_deps gate still green)
- [Phase ?]: Plan 06: filter.pmc kept as documentation-only placeholder; CSV-level post-filter in csv_check.rs is the authoritative gate
- [Phase ?]: Plan 06: svelte-check is continue-on-error in ci-full.yml until Phase 2 wires @tauri-apps/api (per deferred-items.md)
- [Phase ?]: Plan 06: Sysinternals ProcMon SHA256 logged but NOT gated (Microsoft does not publish stable checksums; T-06-01 accepted with audit-log mitigation)
- [Phase ?]: Plan 06: cyrillic sandbox doubles as success-criterion-#1 + FOUND-11 fixture; crash gate (T-06-04) prevents silent pass
- [Phase ?]: 02-01: Path B column mapping: domain uses UI names, SQL stays V003
- [Phase ?]: 02-01: DeviceRepository associated type Conn keeps rusqlite out of trackly-core (hexagonal boundary)
- [Phase ?]: 02-01: ImportSessionStore lazy sweep on put() only - no background task
- [Phase ?]: 02-02: DevicesPlaceholder.svelte временный; Plan 03 заменит на features/devices/DevicesPage.svelte
- [Phase ?]: 02-02: initTheme() вызывается в main.ts ДО mount — no-flash guarantee
- [Phase ?]: 02-02: svelte-check теперь blocking gate в ci-fast + ci-full (Phase 1 deferred item закрыт)
- [Phase ?]: Phase 3-01 (PDF spike): krilla 0.7 PASSED — pinned Metadata + xmp_metadata=false + regex post-process yields deterministic byte-stream on macOS aarch64 (sha256 88df7f9d…); Typst-as-lib fallback NOT triggered
- [Phase ?]: Phase 3-01: MSRV 1.88 → 1.92; Win7 32-bit closed in v1
- [Phase ?]: Phase 3-01: minijinja features = json + fuel + serde (required for set_fuel and tmpl.render(serde_json::Value))
- [Phase ?]: Phase 3-01: krilla 0.7 API path — krilla::metadata::{Metadata, DateTime}, krilla::SerializeSettings (interchange/serialize are private modules)
- [Phase ?]: Plan 03-03: DeviceRow остаётся serde-free; canonical snapshot пишется через device_snapshot_json helper
- [Phase ?]: Plan 03-03: ActReturnDto принимает bulk_location_name/location_name_override для UX-friendly resolve
- [Phase ?]: Plan 03-03: cascade-delete handover делает LIFO undo (returns reverse order) в одной writer-tx
- [Phase ?]: Plan 03-04: ActService::with_pdf_pipeline (Optional Arc-deps) вместо breaking-change в new() — backward-compat сохраняет Phase 2/3 test fixtures
- [Phase ?]: Plan 03-04: minijinja +builtins feature; шаблоны default("—", true) для null-handling (срабатывает на explicit JSON null, не только undefined)
- [Phase ?]: Plan 03-04: PDF preview UI = iframe + blob URL (НЕ pdfjs-dist canvas) — Pitfall 8 обход, WebView2/WKWebView сами рендерят PDF нативно
- [Phase ?]: Plan 03-04: DEV-14 UI button «Печать документа приёма» отложена в plan 05 — backend devices_render_acceptance_pdf готов и протестирован
- [Phase ?]: Plan 03-05: ACT-04 поиск через LIKE+FTS5 (UNION CTE), acts_fts отложен до Phase 7
- [Phase ?]: Plan 03-05: DEV-14 UI flow через intermediate-modal → preview-modal mode='acceptance'
- [Phase ?]: Plan 03-05: W-9 MSK encoding на UI; backend UTC форматирование оставлено Phase 7
- [Phase ?]: Phase 3 closed: все 16 требований complete; готова к /gsd-verify-work
- [Phase ?]: 03.2-02
- [Phase 03.3]: ITEM-1 — Вариант A (флаг group_by_condition: bool в DeviceFilter); DevicesPage передаёт false, ActFormItemsTable передаёт true; DEF-2B сохранён
- [Phase 03.3]: ITEM-1 — «разное» для смешанной group (зафиксировано пользователем, UAT-ITEMS §Решения п.1); вычисляется через condition_distinct_count > 1 на клиенте
- [Phase 03.3]: ITEM-2 — нативный title= на всех text-ячейках (не кастомный tooltip-компонент)
- [Phase 03.3]: ITEM-4 — вторая секция в DeviceAutocompleteField через существующую locations_autocomplete Tauri-команду; HTTP route добавляется в http/devices.rs
- [Phase ?]: group_by_condition flag design
- [Phase ?]: 05-02-SUMMARY.md
- [Phase ?]: 05-02: server API design
- [Phase ?]: 05-02: session store design
- [Phase ?]: 05-02: server shutdown
- [Phase ?]: 05-02: auth hashing
- [Phase ?]: authorize() enforced in build_* helpers — единая точка авторизации для HTTP и Tauri
- [Phase ?]: GovernorLayer несовместим с tower oneshot тестами: создавать сессии программно через RusqliteSessionStore::create()
- [Phase ?]: role_endpoint_matrix: macro_rules! new_app! для свежего router на каждый test case (oneshot потребляет router)
- [Phase ?]: bindings-phase6.ts: Phase 6 типы вынесены в отдельный файл (не gitignored bindings.ts) для хранения в git без force-add
- [Phase ?]: specialist role maps to manager in UserRole; isSpecialist = admin || manager in requests portal
- [Phase ?]: 06-08: admit returns Vec<PrinterDto>; two-step probe→device→printer in admit; D-GAP-Replace-Select: devices.list(type_id=2) in RequestFormModal
- [Phase ?]: 07-01: snake_case JSON in Phase 7 DTOs — consistent with existing device.rs, no camelCase rename_all
- [Phase ?]: 07-01: StatusCount in reports.rs distinct from device.rs StatusCount — different semantic shapes (status_name:String+count:i64 vs status_id:i64+count:u64)
- [Phase ?]: 07-01: V026 org_settings single-row invariant enforced via CHECK (id = 1) + seed row at migration time
- [Phase 07]: 07-02: V027 migration for is_default column on document_templates (ALTER TABLE ADD COLUMN NOT NULL DEFAULT 1)
- [Phase 07]: 07-02: OrgDbService coexists with OrganizationService — new write layer, backward compat preserved for act_service PDF pipeline
- [Phase 07]: 07-02: rusqlite::backup::Backup scope block pattern — inner block ensures Backup+reader_guard drop before integrity_check on dest_conn (borrow checker)
- [Phase ?]: 07-04: TemplateEditor full-width card (no max-width: 640px) per UI-SPEC SET-09/D-20 exception — template textarea needs full available width
- [Phase ?]: 07-04: Logo served as img src=data:... not raw SVG injection — scripts blocked in img context (T-07-04-05 mitigated)
- [Phase ?]: DashboardStatusCount: renamed from StatusCount in dto/reports.rs to avoid TypeScript collision with device.rs StatusCount
- [Phase ?]: settings_move_db and app_restart Tauri-only: not exposed in HTTP router (T-07-07-03, D-19)
- [Phase ?]: 07-14: Vec<ReportCountEntry> for ReportCountsDto (no HashMap) — consistent with all existing DTOs in reports.rs; specta derives cleanly; TypeScript gets Array not Record

### Pending Todos

None yet.

### Blockers/Concerns

Spike-зоны, требующие внимания во время планирования соответствующих фаз:

- **Phase 1:** WEBVIEW2_USER_DATA_FOLDER timing, Cyrillic Windows manifest setup, ProcMon-in-CI scaffolding (~½ дня каждый)
- **Phase 3:** krilla vs Typst-as-lib spike на реальном Cyrillic-фикстуре (1–2 дня)
- **Phase 6:** host-side механизм для Pantum hang detection — local agent vs remote WMI/RPC (требует реального BM5100ADN, ~неделя)
- **Phase 8:** валидация LDAP-bind против реального Windows Server 2022 с channel binding enforced (½ дня с реальным DC)

## Deferred Items

Items acknowledged and carried forward from previous milestone close:

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
| *(none)* | | | |

## Quick Tasks Completed

| Date | Slug | Summary | Status |
|------|------|---------|--------|
| 2026-06-14 | http-camelcase-payloads | S-5 parity: `#[serde(rename_all = "camelCase")]` on all axum request payload structs in http/ so browser/HTTP transport accepts the camelCase keys the frontend sends (e.g. `userNew`, `actId`). Fixes latent 422 on multi-word args in server mode. +regression test. | complete ✓ |
| 2026-06-18 | backup-date-schedule-template-fixes | Phase-07 round-3 follow-ups. R3-1: fixed Backups «Последний бэкап: Invalid Date» — `BackupSettings.svelte` read wrong DTO field (`timestamp` instead of `timestamp_utc` unix-seconds) + dropped phantom `last_backup_time`. R3-2: schedule blank after restart — normalized `"disabled"↔""` sentinel at load/save boundary (mirrors GAP-S5 load-on-mount). R3-3/CR-02: `template_service.rs` `update_body`/`reset_to_default` now guard on `rows_affected == 0` → `AppError::NotFound` instead of silent `Ok(())` (+TDD test). R3-4/CR-01 intentionally WONTFIX (RU-only UTC+3 v1). | complete ✓ |

## Session Continuity

Last session: 2026-06-18T17:10:55.567Z
Stopped at: Phase 8 context gathered
Resume file: 
.planning/phases/08-windows-macos-linux/08-CONTEXT.md
