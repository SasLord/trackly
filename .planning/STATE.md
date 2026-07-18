---
gsd_state_version: 1.0
milestone: v1.2
milestone_name: Редизайн UI и дизайн-система
status: executing
last_updated: "2026-07-18T06:24:17.716Z"
last_activity: 2026-07-18
progress:
  total_phases: 27
  completed_phases: 26
  total_plans: 167
  completed_plans: 163
  percent: 96
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-07-15 after v1.1.2 milestone)

**Core value:** Учёт устройств и картриджей с актами приёма-передачи и историей перемещений должен работать надёжно и быстро в режиме «одной кнопкой» — без обращения к Excel-таблицам, ручного присвоения номеров актов или потери истории при возврате на склад.
**Current focus:** Phase 24 — base-components

## Current Position

Phase: 24 (base-components) — EXECUTING
Plan: 4 of 7
Status: Ready to execute
Last activity: 2026-07-18

### Phase 6 gap-closure decisions (2026-06-15)

- D-GAP-Printer-Add: принтер = устройство type=Принтер + опц. SNMP; завести вручную И через discovery; admit починить (PRN-04 USB).
- D-GAP-Replace-Select: Select принтера в форме замены = устройства type=Принтер (§427), не printers-таблица.
- D-GAP-Employee-Access: полноценный вход сотрудника → AD Phase 8; сейчас только корректный ролевой рендер.
- Критические дефекты: requests_create arg `dto` vs `payload`; requests_status_counts/get_history mismatch; printers_admit заглушка.

## Performance Metrics

**Velocity:**

- Total plans completed: 122
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
| 12 | 21 | - | - |
| 13 | 8 | - | - |
| 14 | 3 | - | - |
| 16 | 5 | - | - |
| 17 | 7 | - | - |
| 18 | 5 | - | - |
| 22 | 6 | - | - |
| 20 | 6 | - | - |
| 21 | 1 | - | - |
| 23 | 8 | - | - |

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
| Phase 12 P01 | 22min | 2 tasks | 8 files |
| Phase 12 P02 | 35min | 2 tasks | 5 files |
| Phase 12 P03 | 18min | 3 tasks | 3 files |
| Phase 12 P04 | 12min | 1 tasks | 2 files |
| Phase 12 P05 | 25min | 3 tasks | 16 files |
| Phase 12 P06 | 25min | 2 tasks | 5 files |
| Phase 12 P07 | 25min | 3 tasks | 6 files |
| Phase 12 P08 | 15min | 1 tasks | 1 files |
| Phase 12 P09 | 55min | 2 tasks | 5 files |
| Phase 12 P10 | 15min | 2 tasks | 3 files |
| Phase 12 P11 | 14min | 2 tasks | 3 files |
| Phase 12 P13 | 12min | 1 tasks | 2 files |
| Phase 12 P14 | 45m | 3 tasks | 9 files |
| Phase 12 P12 | 12min | 2 tasks | 1 files |
| Phase 12 P15 | 5min | 3 tasks | 2 files |
| Phase 12 P19 | 18min | 2 tasks | 2 files |
| Phase 12 P17 | 12min | 1 tasks | 1 files |
| Phase 12 P16 | 2min | 1 tasks | 1 files |
| Phase 12 P18 | 6min | 1 tasks | 1 files |
| Phase 12 P20 | 35min | 2 tasks | 2 files |
| Phase 12 P21 | 35min | 2 tasks | 9 files |
| Phase 13 P01 | 35min | 3 tasks | 3 files |
| Phase 13 P02 | 30min | 2 tasks | 13 files |
| Phase 13 P03 | 25min | 1 tasks | 6 files |
| Phase 13 P04 | 13min | 2 tasks | 2 files |
| Phase 13 P05 | 20min | 2 tasks | 5 files |
| Phase 13 P06 | 25min | 2 tasks | 4 files |
| Phase 13 P07 | 25min | 2 tasks | 2 files |
| Phase 13 P08 | 15min | 3 tasks | 1 files |
| Phase 14 P01 | 22min | 2 tasks | 10 files |
| Phase 14 P02 | 12min | 2 tasks | 1 files |
| Phase 14 P03 | 30min | 3 tasks | 4 files |
| Phase 15 P01 | 25min | 2 tasks | 3 files |
| Phase 15 P02 | 35min | 3 tasks | 5 files |
| Phase 15 P03 | 50 | 3 tasks | 5 files |
| Phase 15 P04 | 25min | 2 tasks | 2 files |
| Phase 16 P01 | 25min | 3 tasks | 6 files |
| Phase 16 P02 | 30min | 3 tasks | 7 files |
| Phase 16 P03 | 15min | 3 tasks | 3 files |
| Phase 16 P05 | 45min | 3 tasks | 8 files |
| Phase 16 P04 | 20min | 2 tasks | 6 files |
| Phase 17 P01 | 55min | 3 tasks | 6 files |
| Phase 17 P04 | 50min | 3 tasks | 5 files |
| Phase 17 P03 | 25min | 3 tasks | 3 files |
| Phase 17 P05 | 7min | 3 tasks | 3 files |
| Phase 17 P06 | 15 min | 3 tasks | 4 files |
| Phase 17 P07 | 40min | 2 tasks | 1 files |
| Phase 18 P01 | 20min | 2 tasks | 4 files |
| Phase 18 P02 | 12min | - tasks | - files |
| Phase 18 P03 | 9min | 3 tasks | 6 files |
| Phase 18 P04 | 25min | 2 tasks | 1 files |
| Phase 18 P18-05 | 40min | 3 tasks | 1 files |
| Phase 19 P01 | 14min | 3 tasks | 7 files |
| Phase 19 P02 | 11min | 3 tasks | 4 files |
| Phase 19 P03 | 25min | 2 tasks | 2 files |
| Phase 19 P04 | 45min | 3 tasks | 6 files |
| Phase 19 P05 | 20min | 3 tasks | 5 files |
| Phase 19 P06 | 25min | 2 tasks | 2 files |
| Phase 19 P08 | 5min | 2 tasks | 3 files |
| Phase 19 P07 | 20 min | 2 tasks | 2 files |
| Phase 19 P09 | 12min | 2 tasks | 1 files |
| Phase 19 P10 | 8min | 2 tasks | 2 files |
| Phase 22 P01 | 76min | 4 tasks | 11 files |
| Phase 22 P02 | 240min | 2 tasks | 5 files |
| Phase 22 P03 | 25m | 2 tasks | 6 files |
| Phase 22 P04 | 25min | 4 tasks | 3 files |
| Phase 22 P05 | 96min | 2 tasks | 3 files |
| Phase 22 P22-06 | 60 | 2 tasks | 3 files |
| Phase 20 P01 | 25min | 3 tasks | 8 files |
| Phase 20 P02 | 15min | 2 tasks | 2 files |
| Phase 20 P03 | 10min | 2 tasks | 3 files |
| Phase 20 P04 | 8min | 2 tasks | 1 files |
| Phase 21 P01 | 22min | 1 tasks | 2 files |
| Phase 23 P01 | 10min | 2 tasks | 2 files |
| Phase 23 P02 | 20min | 2 tasks | 6 files |
| Phase 23 P03 | 35min | 2 tasks | 115 files |
| Phase 23 P04 | 50min | 2 tasks | 106 files |
| Phase 23 P05 | 20min | 2 tasks | 101 files |
| Phase 23 P06 | 10min | 2 tasks | 7 files |
| Phase 23 P07 | 15min | 2 tasks | 3 files |
| Phase 23 P08 | 15min | 3 tasks | 14 files |
| Phase 24 P01 | 8min | 2 tasks | 5 files |
| Phase 24 P02 | 12min | 2 tasks | 3 files |
| Phase 24 P03 | 3min | 3 tasks | 6 files |

## Accumulated Context

### Roadmap Evolution

- Phase 03.1 inserted after Phase 03: Acts quantity model + UAT gap closure (G-1..G-13)
- Phase 03.2 inserted after Phase 03.1: gap-closure deferred UAT items DEF-1/2/3 from Phase 03.1 (URGENT)
- Phase 03.3 inserted after Phase 03.2: Device-list UX round 2 — 4 UAT items after 03.2 (grouping condition column / cell tooltips / status column / location autocomplete) (URGENT)
- Phase 9 added (2026-06-19): AD-аутентификация и заявки на регистрацию пользователей (USR-08..12, REQ-06, SET-10) — вынесено из Phase 8 при SPIDR-split 2026-06-18; traceability в REQUIREMENTS.md синхронизирована
- Phase 10 added (2026-06-21): Ограничение роли employee (Сотрудник) — доступ только к Заявкам + отдельный employee-UI; аудит role-gating read-эндпоинтов на бэкенде
- Phase 12 added (2026-06-22): Взаимосвязь картриджной заявки — сквозная связка заявки на замену картриджа → установка (выбор заправленного картриджа, авто-подстановка расположения принтера, предзаполнение заявителя)
- Phase 13 added (2026-06-25): Редизайн совместимости Принтеры↔Картриджи по уникальному наименованию/типу принтера (не per-device junction; сносит промежуточный UI/таблицы из Phase 12) + свёрнутые chip-задачи (kind-aware drum-state дефолт авто-возврата, лимит списка принтеров 500-vs-200). В милстоне v1.1.
- Phase 14 added (2026-07-03): Данные и структура акта — миграции/схема для расширенных реквизитов организации, Комплектации, Технических характеристик, Срока до, мультиустройства и контекста рендера. Milestone v1.1.1 (PDFA-03, PDFA-04, PDFA-06).
- Phase 15 added (2026-07-03): Рендер и соответствие образцу — дефолтный `.minijinja`-шаблон, мультиустройство через `ItemsTable`, двухстрочные подписи, regression-тесты PDF-пайплайна. Milestone v1.1.1 (PDFA-01, PDFA-02, PDFA-05, PDFA-07, PDFA-08).
- Phase 17 added (2026-07-06): Отчёты и Шаблоны через HTML-печать — перевести экспорт Отчётов с krilla `render_docspec` на HTML-печать по паттерну Phase 16 (акты), переделать редактор Шаблонов в Настройках, убрать krilla из активного пути; закрывает отложенные пункты 16-HUMAN-UAT 2a (миграция Отчётов) и 2b (баг `reports_export_pdf` «Ошибка при создании PDF»). Milestone v1.2.
- Phase 18 added (2026-07-09): Автокомплит и дропдауны — все автокомплиты через portal в `body`; выбор устройства в актах: открытие по фокусу, рабочая фильтрация, группировка одинаковых устройств с раскрытием, схлопывание единственной группы. Milestone v1.1.2 (AUTO-01..05).
- Phase 19 added (2026-07-09): Акты — дата и редактирование — дата «Когда отдали» сохраняется как дата акта; кнопка «Редактировать» становится рабочей (требует диагностики первопричины перед фиксом). Milestone v1.1.2 (ACT-01, ACT-02).
- Phase 20 added (2026-07-09): Печать актов и организация — полный org-контекст в шапке device-акта; безопасный SVG-логотип (санитизация/data: URI, без исполняемых скриптов); вторая строка адреса в печатных формах. Milestone v1.1.2 (PRN-01, ORG-01, ORG-02).
- Phase 21 added (2026-07-09): Точечные фиксы — формат автокода картриджа `C-XXXX`, фотобарабана `D-XXXX`. Milestone v1.1.2 (CRT-01).
- Phase 22 added (2026-07-12): Правка возвратов — «Редактировать» на return-акте активна, открывает диалог «Возврат по акту №XXX» с прежними значениями; полная правка возврата с пересборкой эффектов на устройства по дельте. Отменяет D-07 (Phase 19). Milestone v1.1.2 (ACT-03). Вынесено из живого UAT Фазы 19. (Прим.: `gsd-sdk query phase.add` дал сбой на кириллице — номер 20 вместо 22 + пустой slug; фаза добавлена вручную как 22-return-act-edit.)
- Phase 23 added (2026-07-16): Токены и основы дизайн-системы — новый слой `--tr-*` (поверхности/текст/акцент/семантика/нейтрали/тени), миграция space/radius/font-size ПО ЗНАЧЕНИЮ (не по имени — ловушка переименования шкал), фикс 2 undefined-token багов (`--font-size-sm`, `--radius-lg`). Milestone v1.2 (DS-01, DS-02, DS-03, DS-04, QA-01).
- Phase 24 added (2026-07-16): Базовые компоненты — Button/Input-Select-Textarea-Checkbox/Badge/Tabs/Modal переработаны на новой системе. Milestone v1.2 (CMP-01..05).
- Phase 25 added (2026-07-16): Таблицы и Dropdown — строки таблицы + строка-группа (свёртка/счётчик/вложенные устройства), новый компонент Dropdown/комбобокс (плоский + групповой список) — выделены в отдельную фазу как самые сложные компоненты. Milestone v1.2 (CMP-06, CMP-07).
- Phase 26 added (2026-07-16): Окна с готовым макетом — Дашборд и Устройства, единственные 2 окна из ~12 с реальным макетом Claude Design. Milestone v1.2 (WIN-01, WIN-02).
- Phase 27 added (2026-07-16): Окна основного рабочего процесса — Акты, Картриджи, Принтеры; макета нет, раскладка выводится из компонентной системы фаз 24–25. Milestone v1.2 (WIN-03, WIN-04, WIN-05).
- Phase 28 added (2026-07-16): Окна поддержки и администрирования — Заявки, Отчёты, Настройки, Пользователи; макета нет. Milestone v1.2 (WIN-06, WIN-07, WIN-08, WIN-09).
- Phase 29 added (2026-07-16): Вход и интерфейс сотрудника — Логин/Pending/Blocked/FirstRunWizard, EmployeeLayout; отдельные layout-shell от основного приложения, макета нет. Milestone v1.2 (WIN-10, WIN-11).
- Phase 30 added (2026-07-16): Качество — доступность (AA-контраст, focus ring) и визуальный паритет Tauri WebView vs LAN-браузер; финальная сквозная проверка по всем окнам фаз 26–29. Milestone v1.2 (QA-02, QA-03).

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
- [Phase 12]: installable_only implemented as hardcoded SQL state_id IN (1,2), not a parameterized list — D-01/D-02 domain constants, no client-supplied value-set, closes injection surface
- [Phase 12]: printer_location appended LAST (idx 19) in SELECT_REQUESTS after category_name (idx 18) — preserves append-only convention, single shared mapper across get/list/fetch_in_tx
- [Phase 12]: History enrichment folds cartridge code+model into the existing notes_json 'notes' key (no new JSON key) to keep get_history()/RequestHistoryEntryDto unchanged
- [Phase 12]: RBAC test cases numbered 31/32 (not plan's stale suggestion of 25/26) — continued from the file's actual existing max case number
- [Phase 12]: effectiveCartridge derived pattern (cartridge prop ?? selectedCartridge) lets OperationModal serve both cartridge-centric and request-centric install entries off one code path (D-08)
- [Phase 12]: Checkpoint Task 4 (human-verify, gate=blocking) auto-approved under AUTO_MODE; happy path/DISC-02/D-08 regression confirmed via code review + svelte-check/build, not a live interactive session
- [Phase 12]: 12-04: suggest_person() UNIONs acts + cartridges.holder_name (both Giver/Receiver map to holder_name identically); frequency merge via outer GROUP BY SUM(freq) over a UNION ALL CTE
- [Phase 12]: Plan 12-05: CartridgeService gained internal printer_repo: Arc<SqlitePrinterRepository> field (constructed via Arc::new) rather than threading it through CartridgeService::new() — avoids 11 call-site changes
- [Phase 12]: Plan 12-05: printer_cartridge_models compatibility — setter service methods self-gate via inline authorize(), build_* helpers don't double-gate; getter build_* helpers gate directly since getter service methods take no caller param
- [Phase 12]: Plan 12-05: D-13/D-14 narrowing implemented as single SQL predicate (?N IS NULL OR NOT EXISTS(...) OR model_id IN (...)) — one indexed query encodes both narrow-when-configured and pass-through-when-not
- [Phase ?]: 12-06: Auto-return reuses the new install's given_by_name as implicit actor (D-17) — no new actor field added to ReturnToStock
- [Phase ?]: 12-06: current_printer_device_id SET folded into the same optimistic-lock UPDATE as the status transition, rather than a second UPDATE
- [Phase ?]: 12-06: Auto-return previous cartridge via direct UPDATE inside the same tx (not recursing into transition_in_tx) — internal cascade is known-safe by construction
- [Phase 12]: Plan 12-07: bindings.ts already contained PrinterCompatibleModelsDto/CartridgeModelCompatibleDevicesDto from 12-05's cargo test regen; API wrappers built against real modelId/device_ids wrapper DTO contract, not the plan's assumed cartridgeModelId/number[] shape
- [Phase 12]: Plan 12-08: compatibilityUnconfigured state replaces noModelScopeWarning; gated on preFillPrinterId !== undefined, fail-safe default false on getCompatibleModels error (UX hint, not security boundary)
- [Phase 12]: Plan 12-09: Reused existing Select component (value+onchange) for previous-cartridge charge state instead of raw bind:value select — Matches established codebase convention in OperationModal.svelte's own op-state field and avoids Svelte native-select numeric coercion bug documented in CartridgeFilters.svelte
- [Phase 12]: 12-10: SQLite table-rebuild pattern (CREATE _new -> INSERT SELECT explicit columns -> DROP -> RENAME) scoped inside PRAGMA foreign_keys=OFF/ON within one migration file removes the printers connectivity CHECK without touching printer_readings/printer_alerts FK resolution
- [Phase 12]: 12-11: WsEvent per-variant rename_all=camelCase fixes GAP-12-04 — outer tag stays snake_case, fields camelCase, mirrors RequestTransitionPayload pattern
- [Phase 12]: 12-11: OperationModal suppressSuccessToast opt-in prop — RequestDetail passes true to avoid duplicate toast; cartridge-centric entry (D-08) untouched
- [Phase 12]: 12-13: given_by_name_arm built as Giver-scoped Rust string variable (empty for Receiver) instead of unconditional SQL arm — structural guarantee against cross-field leakage
- [Phase ?]: 12-14: cancel() реализован как отдельный сервисный метод/эндпоинт, не вариант RequestTransitionPayload — избегает протаскивания Employee через transition()'s безусловный authorize(TransitionRequests)
- [Phase ?]: 12-14: V031 миграция (CHECK requests.status += 'cancelled') добавлена как Rule 2 auto-fix — без неё cancel() падал с CHECK constraint failed
- [Phase 12]: 12-12: printerContext: $state<PrinterDto | null> populated inside the existing printers.get(preFillPrinterId) $effect (no second API call) — printerContextHint shows deviceName+ipAddress instead of raw #id, rendered first in the install form, before the cartridge-select picker
- [Phase ?]: Plan 12-15: combined Tasks 2+3 into one commit since both modify the same RequestDetail.svelte if/else-if chain; isOwnRequest condition simplified by dropping redundant isAdRegister check (already guaranteed by parent chain)
- [Phase 12]: 12-19: Inverted actor computed server-side from the triggering Install op's given_by_name/given_to_name (no new payload fields) — closes Tampering threat T-12-19-02 by construction
- [Phase 12]: 12-19: Collapsed Install vs ReturnToStock/ToRefill/FromRefill/WriteOff UPDATE branches in transition_in_tx into one — current_printer_device_id is now always written, fixing a latent bug where direct (non-auto) returns left a stale printer link
- [Phase 12]: 12-17: connectWs() refcounted singleton (refCount + activeCleanup module state) replaces single-shot disconnectFn; idempotency keyed on refCount not ws!==null since browser branch nulls ws on every reconnect — fixes GAP-12-10 duplicate WS toasts without touching the 3 call sites
- [Phase 12]: 12-16: renamed locationLabel (stale name — it actually held IP, not location) to ipText; new locationText derived from printer.deviceLocation closes GAP-12-09 (B1) — printer list row now shows device location left, IP/USB/"—" right via margin-left:auto
- [Phase 12]: 12-18: closed GAP-12-11 by broadening OperationModal's printerContext/previousCartridge lookup $effect gate from `cartridge===null && preFillPrinterId!==undefined` to just `preFillPrinterId!==undefined` — cartridge-centric install entry now shows printer name+IP hint and the «Предыдущий картридж» block, same as request-centric; compatibleModels/cartridgeOptions effects intentionally kept on the narrower `cartridge===null` gate (D-08 regression guard preserved)
- [Phase 12]: 12-20: PrinterSelect.svelte adds optional, compatibility-prioritized printer selector to cartridge-centric install (D-20/D-21); falls back to flat list when no compatibility links exist, never blocks
- [Phase 12]: 12-20: effectivePrinterId derived (preFillPrinterId ?? selectedPrinterId) unifies request-centric and cartridge-centric printer context into one lookup/payload path; previousCartridge block (D-22) reused unchanged
- [Phase 12]: 12-21 (Round 5, GAP-12-13): root cause of printerContext staying null — effectivePrinterId is always a device_id, but printers_get resolves WHERE p.id=?1; added parallel printers_get_by_device_id command (same RBAC gate) instead of changing printers_get's contract (used elsewhere keyed by printers.id); OperationModal switched its lookup effect to getByDeviceId
- [Phase 12]: 12-21 (DEC-A/DEC-B): printerContextHint branches on isSelectorVisible (same predicate gating PrinterSelect markup) — omits name when selector already shows it; Расположение auto-fills from printerContext.deviceLocation in the cartridge-centric entry only, never overwriting manual input
- [Phase 13]: 13-01: upsert_compatibility_in_tx stores printer_name as-given (no TRIM at write); normalisation (LOWER+TRIM) applied only at compare time in list()/compatible_model_aggregates (D-02/D-03/D-04)
- [Phase 13]: 13-01: D-05 pass-through scoped strictly to list()'s cartridge-selection filter, NOT applied in compatible_model_aggregates — R4/D-07 require the printer-card aggregate to reflect only real V005 compatibility rows
- [Phase 13]: Pulled forward Plan 13-03's Tauri/HTTP/specta deletion scope into 13-02 (Rule 3 blocking-issue fix) — Removing the printer/cartridge compat service methods broke compilation in 5 transport-adapter files outside 13-02's stated scope; fixing was required to keep trackly-app building, and matches 13-03's own pre-planned deletion instructions exactly
- [Phase 13]: Cartridge model compatibility DTOs switched from Vec<(String,String)> brand/model pairs to Vec<String> printer names — Matches V032 migration's single printer_name column (Plan 13-01); CartridgeModelDto/CreateDto/PatchDto all updated together
- [Phase 13]: 13-03: compatible_aggregates_for_printer placed on CartridgeService (not PrinterService) since the underlying query lives in cartridges_sqlite.rs — Avoids duplicating query logic across domains; printers.rs build_* helper calls through ctx.cartridges
- [Phase 13]: 13-03: no D-07 pass-through on the new aggregate endpoint — A model with zero compatibility rows for a printer is simply absent from the response, not included with zero counts; Admin/Manager with no matches still gets 200 with models: []
- [Phase 13]: 13-04: transition_in_tx — moved resolved_state_id computation to after prev_current.model_kind_id is fetched, since the kind-aware branch depends on it
- [Phase 13]: 13-04: printers_sqlite.rs::list() — removed .min(200) cap entirely rather than raising it, per D-13 uncapped-read decision (no pagination introduced)
- [Phase ?]: 13-05: suggest_compat_printer re-sourced from devices.name (D-06) instead of cartridge_model_compatibility free-text history; dropped legacy field param across service/Tauri/HTTP layers
- [Phase 13]: filteredCompatibility (trim+dedupe) sent in submit payload, per plan action text, not raw compatibility variable — 13-06: plan's <action> for Task 2 explicitly names filteredCompatibility; frontmatter key_links regex was a looser hint
- [Phase 13]: CompatibleModelsEditor.svelte and OperationModal.svelte compat-junction call sites logged to deferred-items.md, not fixed under 13-06 — Both outside 13-06 files_modified; confirmed pre-existing via git-stash diff; CompatibleModelsEditor.svelte explicitly scoped to Plan 13-07 per UI-SPEC
- [Phase ?]: 13-07: compatAggregates/deviceData/installedCartridge each get their own independent $effect keyed on printer, matching the existing readings $effect convention
- [Phase ?]: 13-07: installedCartridge loading-gap renders '…' instead of falling back to the numeric id — no raw id shown in any intermediate state
- [Phase 13]: 13-08: res.models.length === 0 used as direct equivalent of removed modelIds.length === 0 check for compatibilityUnconfigured (no extra heuristic)
- [Phase 13]: 13-08: compatibleDeviceIds D-05 pass-through computed from printerOptions itself (Set of all deviceId) instead of a second network call
- [Phase 13]: 13-08: previousCartridgeStateId kind-aware default (5 drum / 3 cartridge) set when previousCartridge resolves (.then branch), not in the modal-open reset effect
- [Phase ?]: 14-01: org_settings new requisite columns default to empty string (not V026-style placeholder) — missing requisites degrade to blank per D-02
- [Phase ?]: 14-01: HeaderBlock direct-construction sites use ..Default::default() spread for new fields where site doesn't need requisite content
- [Phase ?]: 14-01: new org_settings columns always appended last in SQL SELECT/UPDATE to preserve existing r.get(N) ordinal indexes
- [Phase 14]: 14-02: Task 1 required no code changes to http/settings_org.rs or tauri_cmds/settings_org.rs — both pass OrgPatch through opaquely; bindings.ts already carried the 5 new fields from Plan 01
- [Phase 14]: 14-03: org_db wired via separate with_org_db() builder (not folded into with_pdf_pipeline's 3-arg signature) — avoids breaking existing test call sites; org_db is Option-aware end-to-end
- [Phase 14]: 14-03: render_pdf fallback (org_db=None) reads legacy org.json name/inn/kpp/address, defaults 5 new requisites to empty strings — matches D-02 degrade-to-blank contract
- [Phase 15]: 15-01: Section::Signature sublabels use plain #[serde(default)] + Option<String> idiom (defaulting to None, not the fn-default idiom used for spacer_pt) so absence renders the pre-Phase-15 single-line layout unchanged
- [Phase 15]: 15-01: ttf-parser promoted to direct dependency (0.25.1, exact-pinned, already transitive via krilla->rustybuzz/skrifa) via Task 0 human-verify checkpoint
- [Phase 15]: 15-01: 2-column header grid stays fixed regardless of logo presence (no adaptive single-column fallback); empty requisite lines (phone/fax/email/OKPO+OGRN) skipped entirely rather than shown as blank placeholder
- [Phase ?]: [Phase 15]: 15-02: render_pdf's None org_db branch explicitly returns (dto, None, None) 3-tuple — no behavior change for fixtures without org_db wired
- [Phase ?]: [Phase 15]: 15-02: Section::DeviceCard long_fields renderer does not filter empty values itself — template is sole source of truth for which long fields get emitted (matches existing conditional-injection idiom)
- [Phase ?]: [Phase 15]: 15-02: act.giver_name intentionally no longer displayed in act body per D-09 (moved to bare Выдал signature label; receiver_name now in intro paragraph) — deliberate content change, not a regression
- [Phase ?]: [Phase 15]: 15-03: render_handover_act_produces_cyrillic_pdf assertion updated from stale giver_name-in-body wording to receiver_name (D-09 removed giver_name from body) — planned N=1 regression anchor
- [Phase ?]: [Phase 15]: 15-03: acts_e2e_smoke.rs handover_pdf_render_within_e2e had the same D-09 giver_name-in-body drift as pdf_render_act.rs but was outside the plan's files_modified — fixed as Rule 1 auto-fix (same root cause, single assertion line)
- [Phase ?]: [Phase 15]: 15-03: act_42.sha256 regenerated (88df7f9d -> caaca9c5) via deliberate single-step procedure (run test, copy printed hash, verify act_42.json fixture input untouched) per T-15-09 mitigation — not a blanket auto-accept
- [Phase ?]: [Phase 15]: 15-04: Header renders once on page 1 only (WR-05 gap closure)
- [Phase ?]: [Phase 15]: 15-04: DeviceCard measured via measure_device_card_height (mirrors draw-time wrap_text_to_width arithmetic) — never split across a page boundary; other section variants use a cheap pre-draw bounds check
- [Phase ?]: [Phase 15]: 15-04: act_42.sha256 verified unchanged (not regenerated) — pagination bounds check never fires for the single-device fixture
- [Phase 16]: 16-01: Task 3 (templates + build_safe_html_env) executed before Task 2 (html_templates.rs) to keep every intermediate commit compiling — include_str! in Task 2 depends on the .html files created in Task 3
- [Phase ?]: 16-02: reused pipeline.organization.paths for templates dir resolution instead of adding a new ActService paths field + with_paths builder
- [Phase ?]: 16-02: OrganizationService::read_logo_bytes added — reads legacy org.json logo file bytes+MIME for base64 data: URI embedding in render_acceptance_pdf
- [Phase ?]: 16-02: Rule-3 fix folded Tauri/HTTP adapter type changes (acts.rs, templates.rs, http/acts.rs) into this plan to keep cargo build -p trackly-app green; full delivery UX rework remains Plan 16-03 scope
- [Phase 16]: 16-03: Task 1/2 (String return type, text/html content-type) already complete from Plan 16-02's Rule-3 fix — scope narrowed to deleting acts_open_pdf_in_system + regenerating bindings.ts
- [Phase 16]: 16-03: ui/src/bindings.ts is gitignored, never committed — regenerated via cargo test --test export_bindings, verified in place, no git commit for that file
- [Phase ?]: 16-05: renamed render_with_missing_template_returns_notfound/render_with_broken_template_returns_validation to assert graceful fallback (embedded HTML default), not error
- [Phase ?]: 16-05: Rule 1 bugfix — org.logo_data_uri needed | safe in both HTML templates; autoescape was entity-encoding the / in base64 data: URIs, corrupting the logo in production
- [Phase ?]: 16-04: Save-as-PDF button removed entirely (not repurposed to save raw HTML) — browser print dialog already offers Save-as-PDF (D-09/Req 5)
- [Phase ?]: 16-04: Rule 1 fix in client.ts (outside stated files_modified) — HTTP transport's binary-response branch wrongly converted text/html responses to number[]; added explicit text/html -> res.text() branch, required for D-09 dual-transport correctness
- [Phase ?]: 16-04: templates_render_preview stale application/pdf content-type + Promise<number[]> frontend type left unfixed (dead code, zero UI callers) — logged to deferred-items.md
- [Phase ?]: 17-01: ReportService gained minimal organization: Option<Arc<OrganizationService>> field + with_organization builder (not full pipeline struct) since export_pdf only needs .paths for templates_dir resolution
- [Phase ?]: 17-02: TemplateService organization field + with_organization builder mirrors ActService/ReportService; validate_preview retargeted to HTML render
- [Phase ?]: 17-02: T-17-02-01 mitigated via fixed DEFAULT_HTML_TEMPLATES allowlist check before path join in update_body/reset_to_default
- [Phase ?]: 17-02: tests/template_edit.rs (Rule 3 fix) rewired with_organization + retargeted assertions from DB-backed get_active to file-backed list_all_for_editor
- [Phase ?]: 17-02: test env-var guard mutex switched to tokio::sync::Mutex (from std::sync::Mutex) since guards held across .await (clippy::await_holding_lock)
- [Phase 17]: 17-04: html_report_render.rs negative-artifact assertion avoids literal DocSpec/render_docspec substrings to not trip the Req 6 grep gate on the test file itself
- [Phase 17]: 17-04: fixed a Plan 17-01 unit test in report_service.rs whose negative-match assertion literally contained render_docspec/DocSpec strings, tripping the same Req 6 grep gate
- [Phase 17]: 17-03: PdfPreviewModal mode=report is additive-only extension (no rewrite of print machinery); ReportsPage export+print unified onto one modal-opening trigger; TemplateEditor variables panel is per-kind data-driven (VARIABLES_BY_KIND) replacing static hardcoded block
- [Phase ?]: 17-05: column_labels appended as new 8th arg to export_pdf (not replacing columns) — keeps row_field key-based cell resolution untouched
- [Phase ?]: 17-05: disallowed logo_mime drops the logo entirely (logo_bytes=None) rather than falling back to a default mime
- [Phase 17]: 17-07: full trackly-app test suite confirmed green (77 binaries, 0 failures) via background-monitored canonical CI invocation (mock env + --test-threads=1); closes Req-7's UNCERTAIN status with evidence, not hypothesis
- [Phase 18]: 18-01: list_grouped true-branch group key = (type_id,name,model) (D-05), sort by count DESC (D-04), name_prefix drives FTS5 text filter via build_fts_query (AUTO-03); false-branch untouched
- [Phase ?]: Phase 18 Plan 02: dropdownAnchor.ts wraps .dropdown AND .dropdown-item in :global() (not just .dropdown) — matches DeviceContextMenu.svelte precedent, avoids scoped-CSS pruning risk on portaled nodes; box-shadow --shadow-md -> --shadow-elev-2 (unused token fix)
- [Phase 18]: 18-03: PersonAutocomplete/DeviceAutocompleteField migrated to portal + dropdownAnchor recipe from Plan 18-02; DeviceAutocompleteField passes maxHeight:200 to match its 200px CSS max-height
- [Phase 18]: 18-03: Select/CartridgeSelect/GroupedPrinterSelect/PrinterSelect documented AUTO-01-compliant by construction (native <select>, no custom overlay) after re-reading each source, per T-18-07 mitigation
- [Phase ?]: 18-04: raw <input> replaces Input.svelte for device picker (no ref-forwarding); openByRow[idx] alone gates dropdown visibility, empty-state renders inside
- [Phase ?]: 18-04: activeIndexByRow keyboard-nav highlighting added as Rule 2 completeness fix alongside the plan's ArrowUp/Down/Enter/Tab handler
- [Phase ?]: 18-05: единственная оставшаяся после фильтрации группа всегда разворачивается через drillInto (auto-flatten), единый код-путь с обычным drill-in (AUTO-05/D-09)
- [Phase ?]: 18-05: количество устройства задаётся только в колонке «Количество» таблицы позиций — spinner убран из дропдауна пикера (checkpoint fix)
- [Phase ?]: 18-05: isExpandable требует ids.length>1 — единственный экземпляр не раскрывается, клик сразу выбирает (checkpoint fix)
- [Phase 19]: 19-01: ui/src/bindings.ts is gitignored — Task 1 regeneration verified but produces no committed diff (only Rust ActDto struct change is committed)
- [Phase 19]: 19-01: html_act_render.rs tests assert on act.date_human (RU) not act.date (ISO) — the act_handover.html template only renders date_human; ISO field is unused in markup
- [Phase 19]: 19-02: update_act_header_in_tx SET clause unconditional for 5 original header fields, COALESCE-only for handover_date_utc/number — Plan 19-03 must resolve values before calling
- [Phase 19]: 19-02: complectation_at_time semantics documented on ActUpdateItemDto (retained vs newly-added device); specs (тех.характеристики) intentionally excluded from update DTO
- [Phase 19]: 19-03: update_act_header_in_tx's unconditional SET fields always resolved to Some(..) in ActPatch construction (never left as outer None)
- [Phase 19]: 19-03: custom:update_remove chosen as distinct audit action for edit-driven device removal (vs delete_soft's custom:undo); payload_json still carries act_id for bulk-undo compat
- [Phase 19]: 19-03: requirements-completed left empty (not ACT-02) — requirement spans plans 19-02..19-05, only backend half done here (matches 19-02's precedent)
- [Phase ?]: 19-04: build_acts_update mirrors build_acts_create's single-DTO shape (id/expected_version live inside ActUpdateDto, not split args)
- [Phase ?]: 19-04: RBAC regression landed as Case 42 (grepped actual max 41 first, not a stale plan-suggested number)
- [Phase ?]: 19-04: requirements-completed left empty (not ACT-02) -- transport wiring only; Plan 19-05 closes the user-visible UI loop
- [Phase 19]: Plan 19-05: edit-mode prefill sources directly from initialAct (acts.get(id) result), bypassing live device search since existing act positions are в_работе, not на_складе
- [Phase 19]: Plan 19-05: second, independent ActFormModal instance (mode=edit) added in ActsPage rather than threading shared create/edit state through one modal
- [Phase 19]: Plan 19-05: D-07 edit-button gating deliberately omits !act.archived — archived handover acts remain editable, unlike Возврат
- [Phase 19-06]: recompute_parent_archived call gated on added/removed non-empty, placed after CAS header UPDATE and before final-audit fetch — Closes CR-01 — archived was never recomputed on update()'s device-set mutations; recompute must run after CAS (both bump version) to avoid spurious OptimisticLockMismatch, and gating preserves the header-only version+1 contract
- [Phase ?]: 19-08: WR-02 closed via option (a) — clamp UI quantity to 1 in edit mode (schema-consistent per D-06) rather than extend ActUpdateItemDto with quantity/device_ids
- [Phase ?]: 19-08: edit-mode qty column renders static '1' span, not a disabled input, to avoid a misleading spinner control
- [Phase ?]: 19-08: todayISO() switched to getUTCFullYear/getUTCMonth/getUTCDate, unifying with unixToIso()/isoToUnix() UTC convention (IN-01)
- [Phase 19]: 19-07: WR-01 cascades renamed act number to child return acts in the same tx (option a) instead of excluding act_type='return' from the uniqueness check — preserves do_return's copy-parent-number invariant
- [Phase 19]: 19-07: WR-03 audits retained-item complectacia edits (custom:act_item_complectation_edit) gated on stored != incoming value, so a no-op resubmit writes zero audit rows
- [Phase ?]: 19-09: retained-vs-new row marker for act-edit device cell is complectation_at_time !== undefined (not row.picked) — row.picked is also true for a device freshly chosen during the current edit session; only complectation_at_time (set exclusively by itemsFromInitialAct) distinguishes a retained/prefilled position
- [Phase ?]: 19-09: ActFormBody.svelte left untouched after комплектация UI removal — itemsFromInitialAct prefill and the edit payload's complectation_at_time mapping still round-trip the value unchanged even though the editable input was removed from ActFormItemsTable
- [Phase ?]: 19-10: handleEditSaved assigns selectedAct = act directly (fresh ActDto from acts.update()) for immediate reactive detail refresh, closing D-11 stale-detail bug (selectedActId=act.id alone is a no-op when the act is already selected)
- [Phase ?]: 19-10: Редактировать/Возврат buttons on ActDetail converted from disabled-placeholder to bare omission, gated on act_type==='handover' && !act.archived — closes D-12/D-13; return-act editing stays out of scope
- [Phase 22]: 22-01: D-07 implemented this plan (compute-on-read archived_at_utc, no new column, no migration) per user decision 2026-07-12
- [Phase 22]: 22-01: ActReturnDto new fields (giver_name/receiver_name/handover_date_utc) are Option<T> + serde(default) back-compat; write-site consumption deferred to Plan 22-02
- [Phase ?]: [Phase 22]: 22-02: do_return write-site fix persists payload's own giver/receiver/handover_date_utc (D-05/D-12/Pitfall 1); None falls back to parent-swap/now for back-compat
- [Phase ?]: [Phase 22]: 22-02: update_return() clones Phase 19's update() inverted to ActType::Return (added=newly-returned; removed=un-return restore; retained-with-change=re-apply) in one single-writer tx
- [Phase ?]: [Phase 22]: 22-02: D-11 guard = 3-field snapshot compare (status_id+location_id+state) vs return's own after_json; validate-then-mutate, Conflict aborts whole tx, no force-override (catches reissue AND manual relocation)
- [Phase 22]: 22-03: acts_update_return reuses Action::MutateActs (no new RBAC surface) — same gate as acts_update/acts_return/acts_delete, proven by role_endpoint_matrix Case 43
- [Phase 22]: 22-03: bindings.ts stays generated-only — regenerated via cargo test --test export_bindings, never hand-edited; only Rust command/DTO + export_bindings.rs assertions + acts.ts are committed
- [Phase 22]: 22-04: ReturnModal edit mode defaults applyToAll=false on open — preserves per-row saved condition/location from editTarget.items instead of discarding behind an unset bulk field
- [Phase 22]: 22-04: single ReturnModal instance reused for create+edit via mode/editTarget/parentAct props (not a second modal component)
- [Phase 22]: 22-04: ActUpdateReturnDto unused location_id/location_name/notes/deadline_utc fields sent as null from edit payload — confirmed unread by ActService::update_return
- [Phase 22]: 22-05: CR-01 fix applied at consumption point (location.or(before.location_id) before update_full_in_tx), preserving None='no override' semantics upstream — avoids breaking D-11 change detection which relies on None meaning no location override was requested
- [Phase 22]: 22-05: CR-02 fix tags retained-edit audit rows with custom:return_item_edit and excludes that action from select_latest_device_mutation — generalizes correctly across multiple retained edits before un-return, unlike a status-based filter; select_latest_device_mutation_pair (D-11 drift check) left untouched since it needs the newest row including retained-edits
- [Phase ?]: 22-06: validate_update_return mirrors validate_return (dedup/non-empty/per-item-override) MINUS act_item_id dedup (edit items use act_item_id:0 placeholder) — closes WR-01 raw-HTTP gap
- [Phase ?]: 22-06: update_return step 8a added-loop ports do_return's already_returned+qty<=handover_qty bound (WR-03)
- [Phase ?]: 22-06: parent_act_id .expect() -> AppError::Internal domain error inside single-writer closure (WR-02) — no panic path poisons the write task
- [Phase ?]: 22-06: V034 comment corrected (WR-04) — one-time backfill, NOT safe to re-run manually post-Phase-22; comment edit changes refinery checksum so existing dev DBs must be recreated (tests use fresh temp DBs, unaffected)
- [Phase 20]: 20-01: V035 is next-sequential migration (after V034); address_line2 appended as LAST field/column everywhere (no ordinal shift to existing columns), per D-04/D-10
- [Phase 20]: 20-01: embed_migrations! stale incremental-build cache — touching crates/trackly-infra/src/db/migrations.rs forces rebuild if new migration files aren't picked up by test runs
- [Phase 20]: 20-02: render_acceptance_pdf rewritten to org_db.get_for_pdf() parity with render_pdf; read_logo_bytes/org.json legacy path fully removed (D-11); address_line2 propagated to all 3 render ctx sites (D-07)
- [Phase 20]: OrgSettings.svelte address_line2 wired; bindings.ts regenerated (gitignored, no commit needed)
- [Phase 21]: 21-01: format!("{prefix}-{seq:04}") is minimum-width, not fixed — no migration needed; existing 6-digit codes stay valid distinct strings
- [Phase 21]: 21-01: cartridges_numbering.rs assertion widened to len >= 6 (min 4 digits) per plan spec, forward-compatible with counters > 9999
- [Phase 23]: 23-01: --tr-line-height-mono фиксирован как 1.4 (не задано UI-SPEC для mono-роли) — по аналогии с --tr-text-label при том же размере 13px
- [Phase 23]: 23-01: заголовочный комментарий global.scss переформулирован без буквального @use './tokens' в тексте — иначе греп-критерий D-05 (ровно 1 совпадение) ложно триггерится
- [Phase 23]: 23-02: Rule 3 (closed-world gate) strips comments before matching — {role}-placeholder docs in _tokens.scss otherwise trip a false undefined-token violation
- [Phase 23]: 23-02: scripts/**/*.mjs added to eslint.config.js's existing node-config file-pattern block — new dev scripts weren't covered by any existing glob
- [Phase 23]: 23-02: D-15 closed — all 5 pre-existing eslint errors fixed; 7 pre-existing prettier-formatting-drift files logged in deferred-items.md, out of scope
- [Phase 23]: 23-03: var(--tr-text-inverse) на трёх auth-экранах не переименован в --tr-on-accent — консистентность с необновляемым skip-link паттерном (Layout.svelte/EmployeeLayout.svelte)
- [Phase 23]: 23-03: NetworkSettings/UserListRow success-бейдж мигрирован на --tr-success (color-mix источник) / --tr-success-text (текст) — ближе к установленному -soft/-text triplet паттерну
- [Phase 23]: 23-04: verify-value-map.mjs RADIUS_EXCEPTION_FILES fixed to include ui/ prefix (git diff paths are repo-root-relative) - built in 23-02, false-positived on the exact expected radius-sm allowlist exception
- [Phase 23]: 23-04: --radius-lg QA-01 fix applied to 4 auth screens (LoginPage/BlockedScreen/FirstRunWizard/PendingScreen) as part of Task 1 space/radius sweep
- [Phase ?]: 23-05: ReturnModal.svelte делегирует рендер списка возврата ReturnItemsTable.svelte — deviceLabel декомпозирован в deviceName+inventoryNo для tr-mono seam
- [Phase ?]: 23-05: class="tr-mono" всегда как отдельный вложенный span, не примесь к multi-class атрибуту — гарантирует греп-видимость точного литерала
- [Phase 23]: 23-06: Whole-tree final verification found 0 residual gaps (all 3 check-tokens.mjs rules + verify-value-map.mjs clean on first run) — Confirms plans 23-01..23-05 left no seam gaps between sequential sweep-plans
- [Phase 23]: 23-06: pnpm prettier --write . run per plan instruction, closing last pre-existing prettier-drift file; pnpm lint green for the first time in phase 23 — All 6 diffs manually verified as pure line-wrap/reflow, no logic or value changes
- [Phase 23-07]: check-tokens.mjs Rule 4 intentionally matches rgba/hsl inside var(--tr-x, rgba(...)) fallbacks — closed-world token model makes such fallbacks dead code to remove, not preserve
- [Phase 23-07]: verify-value-map.mjs tokensOnSide() applies an unanchored global regex per split line (no m-flag) instead of one anchored+lazy pattern over the whole hunk text — fixes CR-01
- [Phase 23-07]: tr-danger-ring fixed at alpha 0.2 for both themes (rgb components copied verbatim from tr-danger), canonizing 8 of 9 duplicated invalid-focus-ring sites; Button.svelte 0.3 converges in plan 23-08
- [Phase 23]: 23-08: Modal.svelte overlay dark-mode override удалён после миграции на theme-scoped var(--tr-overlay)
- [Phase 23]: 23-08: Button.svelte danger-ring alpha 0.3->0.2 (var(--tr-danger-ring)) — WR-01-санкционированный visual touch, handoff в фазу 24 (CMP-01)
- [Phase ?]: 24-01: --tr-accent-text values transcribed verbatim from RESEARCH.md (Badges.dc.html/Tabs.dc.html agree), not recomputed
- [Phase ?]: 24-01: theme.svelte.ts applyResolved() uses requestAnimationFrame (not setTimeout) to remove .theme-switching, guaranteeing removal only after new theme paints
- [Phase ?]: Phase 24 Plan 02: added missing --tr-danger-hover/--tr-danger-active tokens to _tokens.scss (Rule 3 blocking fix) — RESEARCH.md claimed VERIFIED-present but they didn't exist
- [Phase ?]: Phase 24 Plan 02: ButtonsSection.svelte written as fully explicit static markup (no #each loops) to match literal-string acceptance greps and keep showcase self-documenting
- [Phase 24]: Checkbox/Radio destructure props with let (not const) — required for bind:checked/bind:group on their own native input; Input/Select/Textarea keep const since they never bind: to themselves
- [Phase 24]: Checkbox/Radio .invalid state reuses Input/Select/Textarea's --tr-danger/--tr-danger-ring pair since Fields.dc.html has no dedicated error-box spec for these two

### Pending Todos

None yet.

### Blockers/Concerns

Spike-зоны, требующие внимания во время планирования соответствующих фаз:

- **Phase 1:** WEBVIEW2_USER_DATA_FOLDER timing, Cyrillic Windows manifest setup, ProcMon-in-CI scaffolding (~½ дня каждый)
- **Phase 3:** krilla vs Typst-as-lib spike на реальном Cyrillic-фикстуре (1–2 дня)
- **Phase 6:** host-side механизм для Pantum hang detection — local agent vs remote WMI/RPC (требует реального BM5100ADN, ~неделя)
- **Phase 8:** валидация LDAP-bind против реального Windows Server 2022 с channel binding enforced (½ дня с реальным DC)

## Deferred Items

Items acknowledged and deferred at v1.1 milestone close on 2026-06-26. The v1.1
milestone audit (`milestones/v1.1-MILESTONE-AUDIT.md`) assessed all of these as
`tech_debt` — no critical blockers. Most are v1.0 (already-shipped) leftovers or
un-automatable human-verify items (no FE test runner by design).

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
| uat_gap | 03.1 — 03.1-DEFERRED-UAT-ITEMS.md (v1.0) | open (0 pending) | 2026-06-26 |
| uat_gap | 03.1 — 03.1-HUMAN-UAT.md (v1.0) | partial (13 scenarios) | 2026-06-26 |
| uat_gap | 03.3 — 03.3-UAT-ITEMS.md (v1.0) | unknown (0 pending) | 2026-06-26 |
| uat_gap | 04 — 04-HUMAN-UAT.md (v1.0) | passed (0 pending) | 2026-06-26 |
| uat_gap | 05 — 05-UAT.md (v1.0) | testing (0 pending) | 2026-06-26 |
| uat_gap | 07 — 07-HUMAN-UAT.md (v1.0) | passed (13 scenarios) | 2026-06-26 |
| uat_gap | 08 — 08-HUMAN-UAT.md (v1.0) | passed (0 pending) | 2026-06-26 |
| uat_gap | 10 — 10-HUMAN-UAT.md (v1.1) | partial (2 scenarios, live-browser only) | 2026-06-26 |
| uat_gap | 11 — 11-HUMAN-UAT.md (v1.1) | partial (7 scenarios, live-browser only) | 2026-06-26 |
| verification_gap | 03 — 03-VERIFICATION.md (v1.0) | human_needed | 2026-06-26 |
| verification_gap | 03.1 — 03.1-VERIFICATION.md (v1.0) | human_needed | 2026-06-26 |
| verification_gap | 03.2 — 03.2-VERIFICATION.md (v1.0) | human_needed | 2026-06-26 |
| verification_gap | 04 — 04-VERIFICATION.md (v1.0) | human_needed | 2026-06-26 |
| verification_gap | 10 — 10-VERIFICATION.md (v1.1) | human_needed (render checks) | 2026-06-26 |
| verification_gap | 11 — 11-VERIFICATION.md (v1.1) | human_needed (render checks) | 2026-06-26 |
| quick_task | 260618-vtm-backup-date-schedule-template-fixes | done (recorded complete ✓ in Quick Tasks table; no separate record file) | 2026-06-26 |
| quick_task | 260621-r8x-fix-fk-constraint-on-request-accept-assi | done (recorded complete ✓ in Quick Tasks table; no separate record file) | 2026-06-26 |

Items acknowledged and deferred at **v1.1.2** milestone close on 2026-07-15. The
v1.1.2 audit assessed all as `tech_debt` (no critical blockers). Major UAT / security /
Nyquist gaps for phases 18–22 were CLOSED before archiving (see
`milestones/v1.1.2-MILESTONE-AUDIT.md` → Close-Time Resolution). What remains:

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
| security | 18 — no SECURITY.md (backend list_grouped + UI; low risk, T-18-07 no-custom-overlay confirmed by code re-read) | deferred | 2026-07-15 |
| code_review | 18 — 5 Info-level findings (IN-01..05, advisory, non-blocking) | deferred | 2026-07-15 |
| security | 20 — 3 defense-in-depth WARNINGs (WR-01/02/03) already disclosed in 20-SECURITY.md, non-blocking | deferred | 2026-07-15 |
| test_coverage | cross-phase — no HTTP role-matrix case for settings_save_org_fields Employee→403 (guard structurally present) | deferred | 2026-07-15 |
| docs | historical "11 vs 12" requirement miscount (12 REQ-IDs actually defined & satisfied) | deferred | 2026-07-15 |

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
| 2026-06-30 | fix-tls-cert-san-for-wildcard-bind-host | UX follow-up to the bind-host fix (4ec2a9b): self-signed cert SAN only held `[host, "localhost"]`, so a wildcard bind host (`0.0.0.0`/`::`/empty) put the useless literal `0.0.0.0` in the SAN — LAN browsers connecting via `https://<LAN-IP>:port` got a hostname-mismatch error on top of the expected self-signed-untrusted warning, worsening the fingerprint-trust UX. `tls::generate_self_signed` now routes its SAN list through a new `collect_subject_alt_names` helper: for wildcard hosts it enumerates the machine's non-loopback IPv4/IPv6 addresses as IP-SANs (rcgen 0.14 auto-classifies IP-parseable strings via `IpAddr::from_str`), adds the OS hostname (validated as a DNS label via `is_valid_dns_name`), and keeps `"localhost"`; non-wildcard hosts retain the original `[host, "localhost"]` behaviour. Call sites unchanged (`main.rs:162`, `http/settings.rs:272`, `tauri_cmds/auth.rs:93` — all pass `&host`). New deps `if-addrs 0.15` + `hostname 0.4` (pure-Rust, libc-only, no OpenSSL/DLL — portable-friendly). +3 unit tests (`is_wildcard_host_classifies_correctly`, `collect_sans_non_wildcard_unchanged`, `collect_sans_wildcard_includes_detected_lan_ip` — asserts ≥1 detected LAN IP in SAN, documents/skips if host has no non-loopback ifaces). tls unit tests + `tls_server_smoke` (incl. `generate_self_signed_does_not_panic`) green, `clippy` clean. | complete ✓ |
| 2026-07-02 | 260702-vtf-y-tooltip | Follow-up to debug session `dashboard-consumption-chart-422` (bc0f00c): the consumption chart rendered but was uninformative (no Y-axis/scale, unreadable magnitudes). Rewrote `ChartWidget.svelte` from a hand-rolled SVG line chart into a dependency-free **grouped bar chart** (viewBox `0 0 500 220`, LEFT_PAD=42): Y-axis with `niceMax` rounding + 5 gridline ticks + numeric labels, grouped vertical bars per model per month with value labels above non-zero bars, and a stylized `$state`-driven `<div>` hover tooltip (`getBoundingClientRect`-based positioning, «Месяц · Модель: N» — chosen over native SVG `<title>` for instant styled UX). Correctly handles the single-month case that previously rendered invisibly. Preserved: Props/ConsumptionPoint interfaces, loading/error/empty states, sr-only a11y table, legend, PeriodToggle; DashboardPage.svelte unchanged. `svelte-check` 0 errors, `pnpm --dir ui build` green (ui/dist rebuilt, gitignored). Commit `4ccc179`. Awaiting user live visual verify. | complete ✓ |
| 2026-07-02 | fix-y-axis-integer-ticks | Follow-up fix (live-verify defect on 260702-vtf): Y-axis skipped labels. Ticks were `round(niceMax*i/4)` over 4 intervals — when `niceMax` wasn't a multiple of 4, fractional tick values rounded to gaps (`niceMax=5` → 0,1,3,4,5, dropping «2» since 2.5 rounds up). Switched `ChartWidget.svelte` to an integer nice-step (`yStep` from 1/2/5/10/… — smallest giving ≤5 intervals; `niceMax = ceil(maxVal/yStep)*yStep`; ticks iterate 0→niceMax by yStep) so labels are always whole and contiguous. `svelte-check` 0 errors, `ui/dist` rebuilt. Commit `9405e62`. | complete ✓ |
| 2026-07-04 | 260704-uw3-template-seed-upgrade | Fix: existing DBs never picked up the Phase 15-02 `act_handover.minijinja` rewrite because `seed_defaults_on_startup` only INSERTed a bundled default when `active_count == 0` — any pre-existing active row short-circuited the seed, permanently freezing the template body at whatever it was when first seeded. Extended `seed_defaults_on_startup` with an auto-upgrade branch: fetches `(is_default, body_minijinja)` for the active row per `kind`, branches 3 ways — no row → INSERT (unchanged), row with `is_default=1` and body differing from bundled → UPDATE in place (mirrors `reset_to_default`'s UPDATE shape, `version+1`), row with `is_default=0` (user-customized via `update_body`) or body already matching → no-op. +3 regression tests (bug-repro upgrade, no-clobber of customized templates, idempotency across repeated calls). `cargo test`/`clippy -D warnings`/`fmt --check` all green. Commits `20fb879`, `1a7a1d7`. | complete ✓ |
| 2026-07-05 | 260704-wxw-act-pdf-word-fidelity-redesign | Rewrote default `act_handover.minijinja` + added `Section::FieldRow` DocSpec/renderer variant so the rendered Акт приёма-передачи matches the Word reference sample's body structure: «метка \| подчёркнутое значение» rows instead of `device_card` boxes, full-length field labels (Инвентарный номер:/Серийный номер:/Модель:/Комплектация:/Технические характеристики:/Состояние:/Сроком до:), no per-device «Устройство №N» heading/counter, devices listed sequentially. `FieldRow` draw-arm in `renderer.rs` uses `krilla::geom::{PathBuilder, Rect}` + `krilla::paint::Fill` + `Surface::set_fill`/`draw_path` for the underline (confirmed `fill_path`/`stroke_path` do not exist in krilla 0.7 by reading vendored source); `measure_field_row_height` mirrors `measure_device_card_height`'s measure-then-place pagination pattern so wrapped values never split across a page boundary. `Section::DeviceCard` and its tests kept unchanged (backward compat). `act_42.sha256` verified unchanged (fixture uses only KeyValueTable/ItemsTable/Signature — untouched by this additive change). Full `trackly-app` suite (75 test binaries)/`clippy -D warnings`/`fmt --check` all green. Commits `6b6148f`, `0aed41a`, `3e73cf6`, `fa13a26`, `dc667e0`, `adbb44b`. | complete ✓ |
| 2026-07-15 | 260715-gt2-act-edit-device-quantity | Edit-акта: разрешено задавать количество >1 у НОВОЙ (не retained) не-serial позиции, когда на складе достаточно устройств той же группы (было — жёстко «1»; нельзя было добавить, напр., 3 клавиатуры за раз). Backend не менялся: `ActUpdateDto.items` — full-replacement set из one-device-per-entry `ActUpdateItemDto`, а `ActService::update`'s `added: Vec<i64>` loop (`act_service.rs:667-754`) уже N-safe (переводит каждое добавленное устройство в `в_работе` + локацию акта). Правки только UI: (1) `ActFormItemsTable.svelte` — убран `mode==='edit'` clause из qty-тернаров в `pickDevice`/`pickGroup` (`hasSerial ? 1 : Math.min(...)`), qty-cell рендерит редактируемый `<input max={qtyMax(row)}>` для свежих non-serial строк, статичную «1» — только для retained (`complectation_at_time !== undefined`) или serial; (2) `ActFormBody.svelte` — edit-branch submit теперь `.flatMap()` разворачивает свежую строку с `quantity>1` в N `ActUpdateItemDto` через `group_ids.slice(0, quantity)` (mirror create-branch), retained-строки по-прежнему по одной записи. Retained/serial позиции неизменны. +regression-тест `add_multiple_positions_transitions_all_devices` (`acts_update.rs`) — доказывает multi-device add (items.len 4, 3 новых → `status_id=2` + `location_id=loc_b` + audit_log). Gates: `cargo test --test acts_update` 14/14, `clippy -D warnings` clean, `svelte-check` 0 errors, `pnpm --dir ui build` ok. Commits `e3ab329`, `ae996bc`, `644278a`, `a5c31bc`. Примечание: pre-existing repo-wide `cargo fmt` drift (12 мест в acts_update.rs + др., присутствует на baseline `efd69b6`, локальный rustfmt 1.8.0/1.92.0) НЕ трогался — отдельная проблема CI-гейта. | complete ✓ |

## Session Continuity

Last session: 2026-07-18T06:24:17.708Z
Stopped at: Completed 24-03-PLAN.md
Resume file: None

## Operator Next Steps

- Start the next milestone with /gsd-new-milestone
