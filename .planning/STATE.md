---
gsd_state_version: 1.0
milestone: v1.1
milestone_name: AD-аутентификация
status: executing
last_updated: "2026-06-22T04:22:13.393Z"
last_activity: 2026-06-22 -- Phase 12 planning complete
progress:
  total_phases: 15
  completed_phases: 14
  total_plans: 79
  completed_plans: 76
  percent: 93
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-05-24)

**Core value:** Учёт устройств и картриджей с актами приёма-передачи и историей перемещений должен работать надёжно и быстро в режиме «одной кнопкой» — без обращения к Excel-таблицам, ручного присвоения номеров актов или потери истории при возврате на склад.
**Current focus:** Milestone complete

## Current Position

Phase: 11
Plan: Not started
Status: Ready to execute
Last activity: 2026-06-22 -- Phase 12 planning complete

### Phase 6 gap-closure decisions (2026-06-15)

- D-GAP-Printer-Add: принтер = устройство type=Принтер + опц. SNMP; завести вручную И через discovery; admit починить (PRN-04 USB).
- D-GAP-Replace-Select: Select принтера в форме замены = устройства type=Принтер (§427), не printers-таблица.
- D-GAP-Employee-Access: полноценный вход сотрудника → AD Phase 8; сейчас только корректный ролевой рендер.
- Критические дефекты: requests_create arg `dto` vs `payload`; requests_status_counts/get_history mismatch; printers_admit заглушка.

## Performance Metrics

**Velocity:**

- Total plans completed: 52
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
| 08 | 2 | - | - |
| 11 | 3 | - | - |

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
| Phase 08 P01 | 2 min | 3 tasks | 4 files |
| Phase 08 P02 | 1 | 3 tasks | 1 files |
| Phase 09 P01 | 8min | 2 tasks | 9 files |
| Phase 09 P02 | 75m | 2 tasks | 14 files |
| Phase 09 P03 | 110m | 2 tasks | 18 files |
| Phase 09-ad P04 | 50min | 2 tasks | 12 files |
| Phase 09-ad P05 | 55min | 2 tasks | 11 files |
| Phase 10 P01 | 12min | 2 tasks | 2 files |
| Phase 10 P02 | 45min | 3 tasks | 12 files |
| Phase 10 P04 | 35min | 3 tasks | 6 files |
| Phase 11 P01 | 50m | 2 tasks | 9 files |
| Phase 11 P02 | 55min | 2 tasks | 11 files |

## Accumulated Context

### Roadmap Evolution

- Phase 03.1 inserted after Phase 03: Acts quantity model + UAT gap closure (G-1..G-13)
- Phase 03.2 inserted after Phase 03.1: gap-closure deferred UAT items DEF-1/2/3 from Phase 03.1 (URGENT)
- Phase 03.3 inserted after Phase 03.2: Device-list UX round 2 — 4 UAT items after 03.2 (grouping condition column / cell tooltips / status column / location autocomplete) (URGENT)
- Phase 9 added (2026-06-19): AD-аутентификация и заявки на регистрацию пользователей (USR-08..12, REQ-06, SET-10) — вынесено из Phase 8 при SPIDR-split 2026-06-18; traceability в REQUIREMENTS.md синхронизирована
- Phase 10 added (2026-06-21): Ограничение роли employee (Сотрудник) — доступ только к Заявкам + отдельный employee-UI; аудит role-gating read-эндпоинтов на бэкенде
- Phase 12 added (2026-06-22): Взаимосвязь картриджной заявки — сквозная связка заявки на замену картриджа → установка (выбор заправленного картриджа, авто-подстановка расположения принтера, предзаполнение заявителя)

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
- [Phase ?]: 08-01: bundle.active=true — Tauri bundler включён для всех ОС (D-14)
- [Phase ?]: 08-01: bundle.icon расширен до 5 форматов (32x32/128x128/128x128@2x/icns/ico) — Pitfall 3 закрыт
- [Phase ?]: 08-01: bundle.macOS.signingIdentity='-' — ad-hoc подпись без Apple Developer ID (D-04)
- [Phase ?]: MSRV pinned correctly
- [Phase ?]: perl -0pi version injection
- [Phase ?]: GITHUB_EVENT_NAME fallback
- [Phase ?]: portable no-updater discipline
- [Phase 09]: AdClient port + RealAdClient/MockAdClient adapters mirror SnmpClient triad exactly; ldap3 confined to real.rs, hickory-resolver confined to discovery.rs (no OpenSSL pulled in)
- [Phase 09]: AD fallback only on UnknownLogin (never BadPassword) — avoids a second enumeration oracle for known local logins
- [Phase 09]: Added AppError::ServiceUnavailable{service} instead of reusing WriteQueueBusy — distinct infra-fault path for AD-unreachable
- [Phase 09]: on_ad_bind_success scoped to active-user-only this plan; blocked/deleted/unknown branches are typed TODOs for plan 03
- [Phase 09]: approve_ad_register completes the request directly (open->completed) via a manual optimistic-lock UPDATE, not RequestTransitionOp::Complete — that op's state machine requires in_progress as the source state
- [Phase 09]: ad_register reject semantics check the target user's live is_active flag at reject time, not ad_subtype alone, to distinguish pending-discard from auto-accept-then-rejected
- [Phase 09]: AppError::RegistrationPending/AccessBlocked map to HTTP 403, not 401 — AD bind succeeded, identity is known, just not yet admitted
- [Phase 09-ad]: remember=true sets persistent 30-day sliding cookie (Expiry::OnInactivity), set after session.insert() so it survives the flush-before-insert sequence
- [Phase 09-ad]: AdSettingsDto excludes all AD-password fields; connection settings are read-only TOML, only enabled/auto_accept are writable
- [Phase 09-ad]: bindings-phase9.ts placed at ui/src/ (not ui/src/lib/) matching the real bindings-phase6.ts convention; plan frontmatter path was stale
- [Phase 09-ad]: BlockedScreen restore CTA re-invokes auth_login with retained credentials (no dedicated restoration endpoint) — restoration request is created server-side as a side effect of the blocked AD bind path
- [Phase 09-ad]: ad_register reject-confirmation copy is keyed on adSubtype + a UI-fetched AdSettingsDto.auto_accept hint; backend reject_ad_register independently re-derives the correct mutation from user.is_active, so UI copy mismatch cannot cause incorrect deletion
- [Phase 10]: 10-01: Cross-plan RED/GREEN TDD — auth.rs ReadData matrix fix + Case 9 flip land here, intentionally failing (zero authorize(ReadData) call-sites exist yet); Plan 10-02 wires the call sites and turns Case 9 GREEN
- [Phase 10]: Gated all 5 read-domain resource types (devices/acts/cartridges/printers/reports) with authorize(caller, &Action::ReadData) across both HTTP and Tauri transports — Closes the BFLA gap (API5:2023) left after Plan 10-01's permission-matrix fix; Employee role can no longer read data via list/get/search/status-counts/history/low-stock/suggest endpoints
- [Phase 10]: Kept build_printers_refresh on its pre-existing Action::ReadPrinters check, untouched by this plan's ReadData gating — ReadPrinters is a separate, intentionally distinct action from ReadData — conflating them would have been an architectural overreach beyond this plan's scope
- [Phase 10]: Extended role_endpoint_matrix.rs CI test from 10 to 19 cases covering acts_list, cartridges_list, printers_list, reports_list_device_acts, and users_list — Proves the BFLA fix works end-to-end and serves as a regression guard against future endpoint additions in these 5 domains
- [Phase 10]: 10-04: employeeRoutes implemented as a plain route-map switch in App.svelte's if/else-if chain (not svelte-spa-router wrap() guards) — reuses the existing role-gating pattern already used for shell selection
- [Phase 10]: 10-04: AccessDenied.svelte destructures empty Props ({}) instead of binding unused 'location' prop — svelte-check flags unused destructured bindings as an error
- [Phase 11]: category_name appended as LAST column in SELECT_REQUESTS (idx 18) to avoid index-shift; LEFT JOIN request_categories covers get/list/fetch_in_tx via shared mapper
- [Phase 11]: bindings-phase6.ts is hand-maintained (not regenerated by cargo test); updated manually for categoryName + RequestCategoryDto in sync with Rust DTOs
- [Phase 11]: request_printer_options gates on Action::CreateRequest (every role has it), not ReadData/ReadPrinters which Phase 10 closed for Employee — avoids regressing Phase 10's BFLA fix while unblocking the cartridge-replace form.
- [Phase 11]: request_printer_options DTO is strictly {id, name, location} — no SNMP/community/IP/serial fields cross the wire (BOLA/BOPLA closure, API1/API3:2023).

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
| 2026-06-20 | rustls-crypto-provider-panic | Gap-closure fix discovered during 09-05 end-to-end human-verify: server-mode toggle panicked — both `ring` and `aws-lc-rs` providers in dep graph (ldap3 pulls aws-lc-rs; rcgen/tokio-rustls pull ring), rustls 0.23 can't auto-select. Added `ensure_crypto_provider()` (idempotent `Once`, installs `ring`) called first in `tls::build_server_config`/`load_from_pem` + early in `main.rs`. Enabled `ring` feature on `rustls` dep. Resolves the `graceful_shutdown_drain` pre-existing failure flagged in `09-ad/deferred-items.md` (now marked RESOLVED); +regression test `generate_self_signed_does_not_panic`. `cargo build`/`test`/`clippy -D warnings`/`fmt --check` all clean. | complete ✓ |
| 2026-06-20 | ad-test-connection | Gap-closure: "Проверить подключение" button on AD settings was a dead stub (hardcoded `disabled`, no backend). Added `AdClient::test_connection` (port + Real/Mock impls — LDAPS connect + anonymous bind, no end-user creds), `AuthService::test_ad_connection` (ManageSettings-gated, mirrors `settings_set_ad`), HTTP route + Tauri command (both registered, bindings regenerated), and wired the UI button (loading state, success/error toast + inline hint, enabled only when AD is on). +4 backend tests (mock reachable/unreachable, HTTP admin-gating 401/403, mock-mode 200). `cargo build`/test/`clippy -D warnings`/`fmt --check` + `pnpm svelte-check` all clean. | complete ✓ |
| 2026-06-20 | 09-ad-gaps-defects | Reproduce-first fix of 3 defects found during 09-05 human-verify. **Defect 1** (duplicate restore requests): `create_restore_request` unconditionally inserted a new open `ad_register`/`restore` row on every AD bind — a blocked user re-submitting via login form + `BlockedScreen`'s CTA spammed duplicates. Made idempotent (check-then-insert in one writer tx, reuse existing open request). **Defect 2** (reject failed with generic toast): root cause was NOT the service-layer state machine (a service-level test passed unexpectedly) — it was `RequestTransitionPayload`'s `#[serde(tag = "op", rename_all = "camelCase")]` only renaming the tag's variant-name values, not cascading to each variant's field names (documented serde semantics); every real `requestId`-keyed JSON call failed deserialization, and axum's default `Json` rejection returns a plain-text 422 (not a structured AppError), surfacing as the generic fallback toast. Fixed by adding per-variant `rename_all = "camelCase"` to all 3 variants (Accept/Reject/Complete) +3 wire-contract unit tests +1 HTTP-transport repro test. **Lower-priority** (pending-user mis-routed to restore branch): `on_ad_bind_success`'s catch-all routed ANY inactive+non-deleted user (including never-approved pending registrations) into `create_restore_request`, which would create a spurious second `restore`-subtype request. The `users` table has no column distinguishing "never approved" from "approved then blocked" (both are `is_active=0, deleted=false`) — fixed by joining a `has_open_register_request` signal (open `ad_register`/`register` row) into `find_user_any_state` and routing pending re-binds to a new `reuse_or_create_pending_registration` path instead. Tightened the existing test to assert exact behavior instead of accepting either outcome. All 3 fixes committed atomically (`69dd50c`, `7402e60`, `1977fd3`). Final gate: `ad_register`+`requests_ad_register`+`requests_ad_register_http`+`ad_auth` (21 tests) green, full `trackly-app` suite green, `clippy -D warnings` clean, `fmt --check` clean. | complete ✓ |
| 2026-06-20 | 09-ad-gaps-restoration-flow-ux | UX gap-closure: blocked-login no longer auto-creates a restore request on every AD bind (was burying the admin's rejection reason behind a fresh request). `on_ad_bind_success`'s blocked branch is now READ-ONLY — reports the most recent restore request's state via enriched `AppError::AccessBlocked { pending, rejection_reason }` (reads `requests.resolution_notes` for the canonical reject reason). New explicit, idempotent `AuthService::request_ad_restore(login, password)` re-binds to AD and creates/reuses the open restore request — exposed over HTTP (`/api/v1/request_ad_restore`, same rate-limit treatment as `auth_login`) and Tauri, bindings regenerated. Fixed a real bug along the way: `error_axum.rs` was hand-rolling the JSON error body and silently dropping `details` for every `AppError` variant over HTTP — switched to `Json(&self.0)` (the real `Serialize` impl). `BlockedScreen.svelte` reworked to 3 states (none/pending/rejected-with-reason) driven by `LoginPage`-forwarded error details, CTA now calls `request_ad_restore` instead of resubmitting `auth_login`. `docs/AD-SETUP.md` updated. Replaced 3 now-invalid tests in `ad_register.rs` with 8 new ones covering all 3 read states + idempotent create + anti-enumeration (wrong password / active user) + full reject→reason-surfaced→re-request lifecycle (11 tests in that file, all green). Full targeted suite (`ad_auth`+`ad_register`+`requests_ad_register`+`requests_ad_register_http`, 26 tests) green, `export_bindings` no drift, `clippy -D warnings` clean, `fmt --check` clean, `pnpm svelte-check` 0 errors. Pre-existing unrelated `clippy --tests` len_zero issue in `template_service.rs` left as-is (already tracked in `09-ad/deferred-items.md`). | complete ✓ |
| 2026-06-21 | fix-fk-constraint-on-request-accept-assi | UAT bug (Phase 10 live-verify): admin "Принять в работу" failed with `conflict: FOREIGN KEY constraint failed` while "Отклонить" worked. ROOT CAUSE: `RequestDetail.svelte` sent `assignedToUserId: identity.id`; in unlocked-desktop mode that's the sentinel `0` ("Рабочий стол"), which has no `users` row → violated `requests.assigned_to_user_id → users(id)`. Reject sends no assignee. FIX: `RequestService::transition` Accept now resolves the assignee server-side from `caller.user_id` (None for trusted-desktop → COALESCE keeps existing), ignoring the client value (D-REQ-01 override pattern); UI sends `assignedToUserId: null`. +regression test `request_accept_assignee.rs` (trusted-admin accept with forged id 0 → in_progress, assignee NULL). Full suite green (85 bins, AD mock), fmt/svelte-check clean. | complete ✓ |
| 2026-06-20 | 09-ad-gaps-ws-bridge | Cross-transport notification bug found during 09-05 live-verify: admin's desktop Requests page never live-refreshed when a browser/LAN user created or changed a request — required a manual reload. ROOT CAUSE: nothing forwarded `ctx.ws_broadcast` (the `tokio::sync::broadcast` channel browser WS clients subscribe to in `http/ws.rs`) into the Tauri webview; the only `app.emit("trackly-event", ...)` calls lived inside Tauri command handlers themselves (`tauri_cmds/requests.rs`), so HTTP-originated mutations never reached desktop. Affected ALL browser→desktop notifications, not just AD. Fixed by wiring a single global bridge task in `main.rs`'s `tauri::Builder.setup(...)` that subscribes to `ctx.ws_broadcast` and forwards every `WsEvent` via `app.emit("trackly-event", &event)` (same serde payload — `ws.ts`'s existing `event.type` handlers unchanged; Lagged→continue, Closed→exit, mirrors `http/ws.rs`). Confirmed `RequestService::transition`/`approve_ad_register` already pushed the same `WsEvent::RequestStatusChanged` the direct `app.emit` calls were sending — removed those now-redundant direct emits from `requests_transition`/`requests_approve_ad_register` to avoid double-firing on desktop (single source of truth: service layer → ws_broadcast → bridge). +regression test proving `tokio::sync::broadcast` fans an identical event out to every independent subscriber (the property the bridge relies on). Also committed an untracked proven-green HTTP repro (`restore_request_visibility_http.rs`) left over from the investigation, documenting the backend create/visibility/pending chain is correct. `cargo build`/targeted tests (17 passed)/`clippy -D warnings`/`fmt --check` all clean. | complete ✓ |

## Session Continuity

Last session: 2026-06-22T02:44:01.842Z
Stopped at: Phase 12 context gathered
Resume file: 
.planning/phases/12-cartridge-request-interconnection/12-CONTEXT.md
