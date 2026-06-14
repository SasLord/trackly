# Phase 6: Принтеры (SNMP-мониторинг) и Заявки — Research

**Researched:** 2026-06-14
**Domain:** SNMP polling + axum WebSocket push + requests portal (Rust/Tokio/SQLite)
**Confidence:** HIGH (stack decisions locked; OID data MEDIUM for vendor-specific)

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Discovery и опрос SNMP:**
- D-Discovery-01: review перед добавлением — скан показывает список, админ галочками выбирает
- D-Discovery-02: SNMP v2c + community (дефолт 'public'), vendor по sysObjectID + sysDescr → маппинг на OID-профиль
- D-Poll-01: фоновый tokio-task по интервалу + кнопка «Обновить сейчас»
- D-Arch-01: SNMP I/O в отдельном task, snapshots пишутся через single-writer

**Mock SNMP:**
- D-Mock-01: порт `SnmpClient` (trait) в trackly-core; real (snmp2) в trackly-infra + mock (детерминированные фикстуры); runtime-переключение через config/env

**Схема принтера и история:**
- D-Schema-01: отдельная таблица `printers` (FK→devices); community как `Secret<String>`; USB-учёт — признак/FK на host-device
- D-History-01: `printer_readings` — одна строка на опрос (toner_levels JSON, page_count, status)
- D-Retention-01: прореживание + retention в app_settings, фоновый prune
- D-OID-01: data-driven OID-профили в таблице БД, засеянные миграцией (Pantum/Kyocera/HP/Canon + RFC3805 fallback); UI-редактор — deferred Phase 7
- D-Settings-01: настройки discovery/опроса/retention в `app_settings(key, value)`

**Алерты:**
- D-Pantum-01: Pantum hang-эвристика → v2 (PNT); в Phase 6 — только генеричный alert-каркас (offline/error из SNMP-статуса)
- D-Alert-01: in-app алерт только админу, persist, dedup один на принтер

**Портал заявок:**
- D-Req-Form-01: замена картриджа — обязателен только принтер + комментарий; модель картриджа opional
- D-Req-Categories-01: свободная форма — фиксированный набор: «Ремонт техники» / «Расходные материалы» / «Программное обеспечение» / «Прочее»
- D-Notify-01: WebSocket push (браузер → axum WS по session-cookie; десктоп → Tauri-события)
- D-Req-Lifecycle-01: Создана → Принять в работу / Отклонить → Выполнить; enforce в service-слое
- D-Req-CART07-01: кнопка «Установить картридж» → OperationModal pre-filled
- D-PRN07-01: установка картриджа (CART-07) связывается с принтером через FK

### Claude's Discretion

- Concurrency/timeout discovery-скана (конкретные числа в рамках snmp2 API)
- Точный интервал опроса по умолчанию
- Формат toner_levels JSON (ключ → значение/max)
- Стратегия retention/downsample и конкретные значения
- Форма USB-учёта (флаг + FK на host-device vs отдельная связь)
- Хранение категорий свободной формы (lookup-таблица vs CHECK)
- Хранение алертов (таблица vs статус-поле на printers)
- Точная сигнатура WS-протокола и формат payload
- Структура hexagonal-слоёв (паттерн как devices/cartridges/acts)

### Deferred Ideas (OUT OF SCOPE)

- Pantum-специфичная hang-эвристика (prtMarkerLifeCount + SNMP job-table очередь) → v2 (PNT)
- Host-side Windows print spooler как источник сигнала → не v1
- USB-механизм опроса (агент/WMI/RPC) → Phase 8 spike
- Email/Telegram/Webhook уведомления → финальная фаза v2
- AD-заявка на регистрацию (REQ-06, подтип `ad_register`) → Phase 8
- Виджет «Принтеры» (DASH-05) и отчёты → Phase 7
- UI-редактор OID-профилей → Phase 7 при необходимости
- Доп. вендоры принтеров сверх 4 целевых → ADV (v2)
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| PRN-01 | SNMP-discovery по диапазону IP: идентификация vendor/модели, заведение как device type=Принтер | D-Discovery-02, snmp2 v2c API, sysObjectID/sysDescr маппинг |
| PRN-02 | Мониторинг: уровни тонера, статус, страничные счётчики | OID-таблица RFC3805 + vendor-specific, snmp2 get/getbulk |
| PRN-03 | Поддержка Pantum BM5100ADN / Kyocera ECOSYS / HP LaserJet / Canon iR | OID-профили из research §OID Sets |
| PRN-04 | USB-принтеры — учёт подключения к рабочей станции (без SNMP) | D-Schema-01 USB-учёт |
| PRN-05 | История статусов и уровней (printer_readings) | D-History-01, retention stратегия |
| PRN-06 | Alert-каркас (offline/error) — hang-детекция deferred | D-Alert-01, D-Pantum-01 |
| PRN-07 | Связь принтера с моделью картриджа (FK установки) | D-PRN07-01, carry-forward Phase 4 |
| PRN-08 | Mock SNMP-клиент для dev macOS | D-Mock-01, trait `SnmpClient` |
| REQ-01 | CRUD заявок из браузера | axum routes + dual-transport |
| REQ-02 | Два типа заявок: замена картриджа / свободная форма | D-Req-Form-01, D-Req-Categories-01 |
| REQ-03 | Жизненный цикл заявки (lifecycle переходы) | D-Req-Lifecycle-01 |
| REQ-04 | In-app WebSocket push специалисту/админу | D-Notify-01, axum WS + Tauri events |
| REQ-05 | Связь заявки на замену с операцией CART-07 | D-Req-CART07-01, OperationModal pre-fill |
| REQ-07 | История заявок и статусов | audit_log (паттерн Phase 3/4) |
</phase_requirements>

---

## Summary

Phase 6 состоит из двух вертикальных срезов, реализуемых параллельными волнами: (а) SNMP-мониторинг принтеров — discovery, фоновый опрос, OID-профили, alert-каркас; (б) портал заявок — два типа, lifecycle, WebSocket-уведомления.

Весь стек зафиксирован в CONTEXT.md и CLAUDE.md. Ключевые технические пробелы, которые закрывает исследование: (1) API snmp2 для v2c polling и параллельного сканирования подсети; (2) конкретные OID-наборы для четырёх вендоров + RFC3805 fallback с семантикой значений; (3) паттерн WebSocket-аутентификации по session-cookie в axum; (4) Tauri `AppHandle::emit` для десктопного push; (5) стратегия retention/downsample для `printer_readings` в portable SQLite.

**Primary recommendation:** снимок (один опрос) → пишется через `WriterHandle::execute`, SNMP I/O — через `tokio::task::spawn` + `tokio::time::timeout` с `AsyncSession::new_v2c`. Discovery — параллельные tokio-задачи с bounded semaphore (рекомендую 64) + timeout 2 с на хост. WS-эндпоинт axum — `Session` extractor перед `.on_upgrade()`, broadcast channel `Arc<tokio::sync::broadcast::Sender<WsEvent>>` в AppCtx.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| SNMP polling I/O | Backend (tokio task) | — | Сетевой I/O не должен блокировать DB и HTTP; отдельный background task |
| Snapshot write (printer_readings) | Backend (single-writer) | — | Все writes через WriterHandle — архитектурный инвариант |
| Discovery scan | Backend (tokio spawn parallel) | — | CPU/network bound; параллельные задачи с semaphore |
| OID-профили | Database (SQLite) | Backend service | Data-driven, засеяны миграцией; читаются при каждом опросе |
| Alert detection | Backend service (PrinterService) | — | Сравнение SNMP статуса с порогом; результат → DB |
| Alert display | Frontend (UI badge/banner) | — | Читается из DB через reader pool |
| WebSocket push | Backend (axum handler + broadcast) | Frontend (WS client) | Push событий через broadcast channel; frontend подписывается |
| Tauri events (desktop) | Backend (AppHandle::emit) | Frontend (listen) | Нативный путь без WS-сервера для десктопа |
| Request lifecycle | Backend service (RequestService) | — | Enforce transitions в service, не в HTTP handler |
| Request form | Frontend (браузер-SPA) | Backend validation | Доступно без Tauri; validation дублируется на сервере |
| Requests portal auth | Backend (session-cookie via tower-sessions) | — | Phase 5 D-Session-01 — auth идёт через уже существующий middleware |

---

## Standard Stack

### Core (только новое для Phase 6)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `snmp2` | `0.4.x` (CLAUDE.md) или `0.5.x` (crates.io latest) | SNMP v1/v2c/v3 client | Единственная зрелая Rust SNMP library с async + v3 + crypto-rust; часть RoboPLC экосистемы. Locked by CLAUDE.md на 0.4.x, но 0.5.x — latest на crates.io. **Плановщику:** pin `0.4` или явно обновить до `0.5` — обе версии совместимы с workspace MSRV 1.92 [ASSUMED: MSRV snmp2 0.4.x ≤ 1.92; требует проверки при добавлении в Cargo.toml] |

**Features для snmp2:** `features = ["crypto-rust", "tokio"]` — чистый Rust без OpenSSL DLL (portable-дисциплина CLAUDE.md).

### Supporting (уже в workspace, расширяется использование)

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `tokio::sync::broadcast` | 1.x (workspace) | Fan-out WS push событий к multiple WS connections | Всегда в Phase 6 для WS-сервера |
| `tokio::sync::Semaphore` | 1.x (workspace) | Ограничить параллелизм SNMP-скана | Discovery task |
| `tokio::time::timeout` | 1.x (workspace) | Timeout для async SNMP-запросов (AsyncSession не имеет встроенного timeout) | Каждый SNMP async вызов |
| `tokio::time::interval` | 1.x (workspace) | Фоновый polling loop | Background poll task |
| `serde_json` | 1.x (workspace) | toner_levels JSON column | Уже в workspace |
| `tower_sessions::Session` | 0.13+ (workspace) | Auth WS-соединения по cookie | WS handler extractor |

**Installation (добавить в trackly-infra/Cargo.toml):**
```toml
snmp2 = { version = "0.4", features = ["crypto-rust", "tokio"] }
```

**Version verification:**
```
cargo search snmp2  →  snmp2 = "0.5.0"  (crates.io, 2026-06-14)
```
[ASSUMED: 0.4.x — последняя minor в 0.4 линии; нужно уточнить точную patch при добавлении]

---

## Package Legitimacy Audit

> slopcheck не установлен на машине. Все пакеты помечены [ASSUMED] ниже. Новых frontend npm-пакетов Phase 6 не вводит (WS через нативный браузерный API).

| Package | Registry | Age | Downloads | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-----------|-------------|-----------|-------------|
| `snmp2` | crates.io | ~2+ лет (RoboPLC проект) | умеренно (специализированный) | github.com/roboplc/snmp2 | [ASSUMED] | Approved — подтверждён `cargo search snmp2`, официальный RoboPLC, уже в CLAUDE.md как рекомендованный |

**Packages removed due to slopcheck [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

*slopcheck недоступен — пакет snmp2 помечен [ASSUMED] выше. Плановщик должен gate install за checkpoint:human-verify. Обоснование доверия: пакет уже рекомендован в CLAUDE.md авторами проекта, подтверждён `cargo search`, официальный репозиторий roboplc/snmp2 верифицирован на GitHub.*

---

## Architecture Patterns

### System Architecture Diagram

```
Discovery trigger (UI → cmd/api)
        │
        ▼
  DiscoveryService::scan(ip_range, community)
        │ spawn N tokio tasks (semaphore 64 concurrent)
        │────────────────────────────────────┐
        │                                    │ per IP:
        │                              tokio::time::timeout(2s)
        │                              AsyncSession::new_v2c
        │                              get([sysObjectID, sysDescr, sysName])
        │                              ────→ [vendor_match → oid_profile_id]
        │                              offline → Ok(None)
        │◄───────────────────────────────────┘
        │
        ▼
  review list → admin picks → PrinterService::create_batch
        │ WriterHandle::execute (insert devices + printers rows)
        ▼

Background Poll Task (tokio::spawn, shutdown: CancellationToken child)
        │
        ├─ tokio::select!
        │   ├─ interval.tick() ──→ poll_all_printers()
        │   ├─ on_demand_rx ───→ poll_single_printer(id)
        │   └─ shutdown.cancelled() ──→ break
        │
        │ per printer:
        │   AsyncSession::new_v2c(ip, Secret::expose(community), ...)
        │   timeout(5s) → getbulk(oid_profile.toner_oids, ...)
        │   timeout(5s) → get([hrPrinterStatus, hrDeviceStatus, page_counter_oid])
        │   ────→ parse → PrinterSnapshot
        │
        │ WriterHandle::execute:
        │   INSERT printer_readings (snapshot)
        │   UPDATE printers SET last_seen_utc, status_cache
        │   IF offline/error → upsert printer_alerts (dedup)
        │
        ▼

axum WS endpoint /api/v1/ws
        │
        ├─ Session extractor (tower-sessions) → authenticate → identity
        ├─ role check (specialist/admin only for alerts)
        │
        ▼
  broadcast::Receiver → forward events → WebSocket::send
        ▲
        │ WsEvent::NewRequest / WsEvent::StatusChange / WsEvent::PrinterAlert
        │
  RequestService::create / transition_status
        │ + PrinterService (alert detection)
        │
        ▼ broadcast::Sender::send(event)  (fan-out to all WS clients)

Desktop path (no server):
  AppHandle::emit("ws-event", payload) → Svelte listen() → toast/refresh
```

### Recommended Project Structure

```
crates/trackly-core/src/
├── ports/
│   ├── printers.rs       # PrinterRepository port trait
│   ├── requests.rs       # RequestRepository port trait
│   └── snmp.rs           # SnmpClient port trait (D-Mock-01)
├── domain/
│   ├── printers.rs       # PrinterRow, PrinterNew, PrinterSnapshot, OidProfile, PrinterAlert
│   └── requests.rs       # RequestRow, RequestNew, RequestTransition

crates/trackly-infra/src/
├── repos/
│   ├── printers_sqlite.rs
│   └── requests_sqlite.rs
└── snmp/
    ├── mod.rs
    ├── real.rs            # snmp2 AsyncSession impl of SnmpClient
    └── mock.rs            # deterministic fixture impl

crates/trackly-app/src/
├── services/
│   ├── printer_service.rs  # poll, discovery, alert detection
│   └── request_service.rs  # lifecycle, CART-07 link
├── dto/
│   ├── printer.rs
│   └── request.rs
├── tauri_cmds/
│   ├── printers.rs
│   └── requests.rs
└── http/
    ├── printers.rs
    ├── requests.rs
    └── ws.rs              # WebSocket handler + broadcast

migrations/
├── V020__printers.sql
├── V021__oid_profiles_seed.sql
├── V022__printer_readings.sql
├── V023__printer_alerts.sql
└── V024__requests_categories.sql  # если lookup-таблица

ui/src/features/
├── printers/
│   ├── PrintersPage.svelte
│   ├── PrintersMasterDetail.svelte
│   ├── PrintersList.svelte
│   ├── PrinterListRow.svelte
│   ├── PrinterDetail.svelte
│   ├── TonerGauge.svelte
│   ├── PrinterAlertBanner.svelte
│   ├── DiscoveryModal.svelte
│   ├── DiscoveryResultsTable.svelte
│   └── api.ts
└── requests/
    ├── RequestsPage.svelte
    ├── RequestsMasterDetail.svelte
    ├── RequestsList.svelte
    ├── RequestListRow.svelte
    ├── RequestDetail.svelte
    ├── RequestFormModal.svelte
    └── api.ts

ui/src/lib/api/
└── ws.ts                  # WS client + Tauri event listener + reconnect
```

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| SNMP protocol encoding/decoding | Кастомный ASN.1 парсер | `snmp2` | BER/DER кодирование, OID парсинг, PDU структуры — сотни edge cases |
| SNMP v3 аутентификация | Собственный SHA/AES | `snmp2` с `crypto-rust` feature | HMAC, key localization, engine discovery — сложный и security-critical |
| Async timeout | `tokio::spawn` с sleep-loop | `tokio::time::timeout(duration, future)` | Единственный правильный способ; AsyncSession не имеет встроенного timeout |
| WS fan-out | `Arc<Mutex<Vec<Sender>>>` вручную | `tokio::sync::broadcast::channel` | Автоматический lagging subscriber handling; clone'аемый Sender |
| Session auth в WS | Парсинг Cookie header вручную | `tower_sessions::Session` как extractor в WS handler | tower-sessions уже в стеке; Session extractor работает до `.on_upgrade()` |
| Tauri события на desktop | HTTP polling из десктопа | `AppHandle::emit(event_name, payload)` | Нативный Tauri механизм; не требует запущенного сервера |
| IP range expansion | Собственный iterator | Простой цикл `start_ip..=end_ip` на u32 | Тривиально в Rust через `u32::from(Ipv4Addr)` и обратно |

**Key insight:** SNMP — сложный binary протокол с множеством вендорных quirks; даже базовый v2c имеет тонкости при OID tree traversal и error handling (noSuchObject vs noSuchInstance). snmp2 обрабатывает всё это.

---

## snmp2 API Reference (VERIFIED)

### Создание v2c сессии
[CITED: docs.rs/snmp2/0.4.9/snmp2]
```rust
use snmp2::AsyncSession;
use std::time::Duration;

// Async (для фонового polling и discovery)
let mut sess = AsyncSession::new_v2c(
    "192.168.1.100:161",  // ip:port
    b"public",            // community (expose из Secret<String>)
    0i32,                 // starting_req_id (0 = random)
).await?;
```

### Получение нескольких OID за один запрос
[CITED: docs.rs/snmp2/latest/snmp2]
```rust
use snmp2::{AsyncSession, Oid};
use tokio::time::timeout;

let oid_status = Oid::from(&[1,3,6,1,2,1,25,3,5,1,1,1]).unwrap();  // hrPrinterStatus.1
let oid_page   = Oid::from(&[1,3,6,1,2,1,43,10,2,1,4,1,1]).unwrap(); // prtMarkerLifeCount

// timeout обязателен — AsyncSession не имеет встроенного timeout
let pdu = timeout(
    Duration::from_secs(5),
    sess.get(&[oid_status.clone(), oid_page.clone()])
).await??;

for (name, value) in pdu.varbinds {
    // value: snmp2::Value enum (Integer, OctetString, ObjectIdentifier, ...)
}
```

### get_many (0.5.x)
[CITED: docs.rs/snmp2/latest/snmp2]
```rust
// В 0.5.x добавлен get_many — атомарный запрос нескольких OID
let pdu = timeout(Duration::from_secs(5), sess.get_many(&[&oid1, &oid2])).await??;
```
[ASSUMED: 0.4.x не имеет get_many — только get(&[oid1, oid2]) или getbulk]

### Parallel discovery scan (паттерн)
[ASSUMED: паттерн на основе tokio документации + snmp2 AsyncSession]
```rust
use tokio::sync::Semaphore;
use std::sync::Arc;
use std::net::Ipv4Addr;

async fn scan_range(
    start: Ipv4Addr,
    end: Ipv4Addr,
    community: &str,
) -> Vec<DiscoveredPrinter> {
    let sem = Arc::new(Semaphore::new(64)); // max 64 параллельных соединений
    let community = community.to_string();
    
    let start_u32 = u32::from(start);
    let end_u32   = u32::from(end);
    
    let mut handles = Vec::new();
    for ip_u32 in start_u32..=end_u32 {
        let ip = Ipv4Addr::from(ip_u32);
        let community = community.clone();
        let sem = sem.clone();
        
        handles.push(tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            probe_printer(ip, &community).await
        }));
    }
    
    let mut results = Vec::new();
    for h in handles {
        if let Ok(Ok(Some(printer))) = h.await {
            results.push(printer);
        }
    }
    results
}

async fn probe_printer(ip: Ipv4Addr, community: &str) -> Result<Option<DiscoveredPrinter>> {
    let addr = format!("{ip}:161");
    let mut sess = match AsyncSession::new_v2c(&addr, community.as_bytes(), 0).await {
        Ok(s) => s,
        Err(_) => return Ok(None), // не может создать сессию
    };
    
    let sys_oids = [
        Oid::from(&[1,3,6,1,2,1,1,2,0]).unwrap(), // sysObjectID
        Oid::from(&[1,3,6,1,2,1,1,1,0]).unwrap(), // sysDescr
        Oid::from(&[1,3,6,1,2,1,1,5,0]).unwrap(), // sysName
    ];
    
    match timeout(Duration::from_secs(2), sess.get(&sys_oids)).await {
        Ok(Ok(pdu)) => Ok(parse_printer_identity(ip, pdu)),
        _ => Ok(None), // timeout или ошибка = offline
    }
}
```

### Обработка значений тонера (RFC 3805 семантика)
[CITED: WebSearch RFC 3805 + SNMP monitoring resources]
```rust
fn parse_toner_percent(level: i64, max_capacity: i64) -> TonerLevel {
    match level {
        -2 => TonerLevel::Unknown,          // RFC 3805: unreported
        -3 => TonerLevel::Unknown,          // RFC 3805: at least one unit (capacity unknown)
        n if n >= 0 && max_capacity > 0 => {
            let pct = (n * 100) / max_capacity;
            TonerLevel::Known(pct as u8)
        }
        _ => TonerLevel::Unknown,
    }
}
```

### Background polling task (паттерн)
[ASSUMED: на основе tokio-util CancellationToken документации + project Phase 5 patterns]
```rust
pub async fn run_poll_task(
    ctx: AppCtx,
    mut on_demand_rx: tokio::sync::mpsc::Receiver<PrinterId>,
    shutdown: tokio_util::sync::CancellationToken,
) {
    // Читаем интервал из app_settings (default 5 мин)
    let interval_secs = ctx.get_poll_interval_secs().await.unwrap_or(300);
    let mut interval = tokio::time::interval(
        std::time::Duration::from_secs(interval_secs)
    );
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    
    loop {
        tokio::select! {
            _ = interval.tick() => {
                poll_all_printers(&ctx).await;
            }
            Some(printer_id) = on_demand_rx.recv() => {
                poll_single_printer(&ctx, printer_id).await;
            }
            _ = shutdown.cancelled() => {
                tracing::info!("printer poll task: shutdown");
                break;
            }
        }
    }
}
```

---

## OID Reference Sets

### RFC 3805 Printer-MIB Fallback (стандартный, для любого принтера)
[CITED: rfc-editor.org/rfc/rfc3805, oidref.com, WebSearch verification]

| OID | Numeric | Тип | Назначение |
|-----|---------|-----|------------|
| `hrPrinterStatus` | `1.3.6.1.2.1.25.3.5.1.1.1` | Integer | Статус: 1=other, 2=unknown, 3=idle, 4=printing, 5=warmup |
| `hrDeviceStatus` | `1.3.6.1.2.1.25.3.2.1.5.1` | Integer | 1=unknown, 2=running, 3=warning, 4=testing, 5=down |
| `hrPrinterDetectedErrorState` | `1.3.6.1.2.1.25.3.5.1.2.1` | OctetString (bits) | bit0=lowPaper, bit2=lowToner, bit3=noToner, bit4=doorOpen, bit5=jammed, bit6=offline |
| `prtMarkerSuppliesLevel` | `1.3.6.1.2.1.43.11.1.1.9.1.N` | Integer32 | Текущий уровень тонера (N=supply index); -2=unknown, -3=reported in units |
| `prtMarkerSuppliesMaxCapacity` | `1.3.6.1.2.1.43.11.1.1.8.1.N` | Integer32 | Максимальная ёмкость |
| `prtMarkerSuppliesType` | `1.3.6.1.2.1.43.11.1.1.5.1.N` | Integer | Тип расходника (3=toner, 4=wasteToner, 7=ink, ...) |
| `prtMarkerSuppliesColorantName` | `1.3.6.1.2.1.43.12.1.1.4.1.N` | OctetString | Цвет ("black", "cyan", etc.) |
| `prtMarkerLifeCount` | `1.3.6.1.2.1.43.10.2.1.4.1.1` | Counter32 | Общее количество страниц (page counter) |
| `prtGeneralPrinterName` | `1.3.6.1.2.1.43.5.1.1.16.1` | OctetString | Имя принтера |
| `sysObjectID` | `1.3.6.1.2.1.1.2.0` | OID | Enterprise OID для идентификации вендора |
| `sysDescr` | `1.3.6.1.2.1.1.1.0` | OctetString | Описание устройства (модель из строки) |
| `sysName` | `1.3.6.1.2.1.1.5.0` | OctetString | Сетевое имя устройства |

**Семантика hrPrinterStatus для alert-каркаса:**
- `idle(3)` + `hrDeviceStatus=running(2)` → статус "В сети" (OK)
- `hrDeviceStatus=warning(3)` → статус "Предупреждение" (alert-worthy)
- `hrDeviceStatus=down(5)` → статус "Не в сети/Ошибка" (alert-worthy)
- Недоступен (timeout/connect fail) → offline

**toner_levels JSON format (Claude's discretion):**
```json
{
  "black": {"level": 45, "max": 100, "pct": 45},
  "cyan":  {"level": -2, "max": -2, "pct": null},
  "drum":  {"level": 78, "max": 100, "pct": 78}
}
```
Null pct = unknown (-2 или -3). Ключи по colorant name из OID.

### Vendor-specific OIDs

#### Pantum (enterprise 1.3.6.1.4.1.40093)
[CITED: github.com/glpi-project/glpi-agent/discussions/590]

| OID | Назначение | Примечание |
|-----|------------|------------|
| `1.3.6.1.4.1.40093.6.3.1` | Оставшийся уровень тонера (%) | Прямой процент, 0-100 |
| `1.3.6.1.4.1.40093.8.1.4` | Уровень барабана (%) | Drum unit |
| `1.3.6.1.4.1.40093.10.3.1.1` | Глобальный счётчик страниц | Page counter |
| `1.3.6.1.4.1.40093.1.1.3.10` | Страниц с текущего тонера | Per-cartridge counter |

**sysObjectID prefix:** `1.3.6.1.4.1.40093` (Pantum enterprise)
**Идентификация:** sysObjectID начинается с `1.3.6.1.4.1.40093` → Pantum профиль

[ASSUMED: OID `1.3.6.1.4.1.40093.6.3.1` проверен на BM5100FDW/M7310DN из community-репорта; BM5100ADN — аналогичная серия, но прямой лабораторной верификации нет. OID профиль требует тестирования на реальном устройстве.]

#### Kyocera ECOSYS (enterprise 1.3.6.1.4.1.1347)
[CITED: WebSearch oidref.com/1.3.6.1.4.1.1347, forum.glpi-project.org]

| OID | Назначение | Примечание |
|-----|------------|------------|
| `1.3.6.1.2.1.43.11.1.1.9.1.1` | Тонер level (RFC3805, используется Kyocera) | Index 1 = black |
| `1.3.6.1.2.1.43.11.1.1.8.1.1` | Тонер max capacity | — |
| `1.3.6.1.2.1.43.10.2.1.4.1.1` | Page counter (prtMarkerLifeCount) | RFC3805, работает на Kyocera |
| `1.3.6.1.4.1.1347.42.*` | Детальные счётчики (трей, цветные) | vendor-specific расширение |

**sysObjectID prefix:** `1.3.6.1.4.1.1347` (Kyocera)
Kyocera ECOSYS в большинстве моделей хорошо поддерживает RFC3805 — вендор-специфичные OID нужны только для детальных счётчиков.

#### HP LaserJet (enterprise 1.3.6.1.4.1.11)
[CITED: ixnfo.com/en/hp-printers-snmp-oid-s-2.html, kb.paessler.com]

| OID | Назначение | Примечание |
|-----|------------|------------|
| `1.3.6.1.2.1.43.11.1.1.9.1.1` | Тонер level (RFC3805) | HP LaserJet поддерживает стандарт |
| `1.3.6.1.2.1.43.11.1.1.8.1.1` | Тонер max capacity | — |
| `1.3.6.1.2.1.43.12.1.1.4.1.1` | Цвет тонера | OctetString ("black", "cyan", etc.) |
| `1.3.6.1.2.1.43.10.2.1.4.1.1` | Page counter | — |
| `1.3.6.1.2.1.43.5.1.1.17.1` | Серийный номер | Используется для dedup при discovery |

**sysObjectID prefix:** `1.3.6.1.4.1.11` (HP)
HP LaserJet — наиболее RFC3805-совместимый вендор; fallback-профиль в большинстве случаев даёт корректные данные.

**Примечание:** `prtMarkerSuppliesLevel = -2` на HP часто означает "near depletion" (отличается от RFC3805 семантики "unreported").

#### Canon iR/imageRUNNER (enterprise 1.3.6.1.4.1.1602)
[CITED: WebSearch hellocomtec.com/snmp-oids-for-canon-ir-adv, community.lansweeper.com]

| OID | Назначение | Примечание |
|-----|------------|------------|
| `1.3.6.1.2.1.43.11.1.1.9.1.1` | Чёрный тонер (RFC3805) | Index .9.1.1 = BK |
| `1.3.6.1.2.1.43.11.1.1.9.1.2` | Cyan (если цветной) | — |
| `1.3.6.1.2.1.43.11.1.1.9.1.3` | Magenta | — |
| `1.3.6.1.2.1.43.11.1.1.9.1.4` | Yellow | — |
| `1.3.6.1.2.1.43.10.2.1.4.1.1` | Page counter | — |

**sysObjectID prefix:** `1.3.6.1.4.1.1602` (Canon)
Canon iR хорошо реализует RFC3805 Printer-MIB для базового мониторинга.

[ASSUMED: Canon iR OID индексы supply — модель-зависимы; порядок CMYK (1-4) типичен, но может отличаться. Требует тестирования на реальном iR устройстве.]

### OID-профиль seed strategy (D-OID-01)

Миграция V021 засевает таблицу `oid_profiles`:
```sql
-- Пример записи профиля
INSERT INTO oid_profiles (name, vendor_prefix, toner_level_oid, toner_max_oid,
    page_counter_oid, status_oid, serial_oid, notes) VALUES
('pantum', '1.3.6.1.4.1.40093',
    '1.3.6.1.4.1.40093.6.3.1',   -- прямой %
    NULL,                          -- у Pantum уровень уже в %
    '1.3.6.1.4.1.40093.10.3.1.1',
    '1.3.6.1.2.1.25.3.5.1.1.1',  -- hrPrinterStatus (стандарт)
    NULL, 'Pantum BM5100ADN и аналоги'),
-- ... Kyocera, HP, Canon, RFC3805 fallback
```

Поле `vendor_prefix` используется для маппинга sysObjectID → профиль при discovery.

---

## WebSocket Pattern (D-Notify-01)

### axum WS handler с session-аутентификацией
[CITED: docs.rs/axum/latest/axum/extract/ws, github.com/tokio-rs/axum examples]
[ASSUMED: паттерн Session extractor + on_upgrade — на основе tower-sessions документации и axum extractor semantics]

```rust
// src/http/ws.rs
use axum::{extract::{State, WebSocketUpgrade}, response::IntoResponse};
use axum::extract::ws::{WebSocket, Message};
use tower_sessions::Session;
use tokio::sync::broadcast;

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    session: Session,        // Session extractor проверяется ДО on_upgrade
    State(ctx): State<AppCtx>,
) -> impl IntoResponse {
    // Аутентификация до upgrade — если нет сессии, вернём 401
    let identity = match session_identity(&session).await {
        Ok(id) => id,
        Err(_) => return axum::http::StatusCode::UNAUTHORIZED.into_response(),
    };
    
    let rx = ctx.ws_broadcast.subscribe();
    
    ws.on_upgrade(move |socket| {
        handle_ws_socket(socket, identity, rx)
    })
}

async fn handle_ws_socket(
    mut socket: WebSocket,
    identity: Identity,
    mut rx: broadcast::Receiver<WsEvent>,
) {
    loop {
        tokio::select! {
            event = rx.recv() => {
                match event {
                    Ok(evt) if evt.is_visible_to(&identity) => {
                        let json = serde_json::to_string(&evt).unwrap();
                        if socket.send(Message::Text(json.into())).await.is_err() {
                            break; // клиент отключился
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("WS client lagged {} events", n);
                    }
                    _ => {}
                }
            }
            msg = socket.recv() => {
                match msg {
                    None | Some(Err(_)) => break, // disconnect
                    Some(Ok(Message::Close(_))) => break,
                    _ => {} // ping/pong обрабатывается axum автоматически
                }
            }
        }
    }
}
```

**WsEvent в AppCtx:** добавить поле `ws_broadcast: Arc<broadcast::Sender<WsEvent>>`.
Capacity broadcast channel: 128 событий — достаточно для пиков при discovery.

### WsEvent payload structure
[ASSUMED: конкретная структура payload — на усмотрение планировщика, соответствует D-Notify-01]
```rust
#[derive(Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsEvent {
    NewRequest { request_id: i64, request_type: String, requester_name: String },
    RequestStatusChanged { request_id: i64, new_status: String },
    PrinterAlert { printer_id: i64, printer_name: String, alert_type: String },
}

impl WsEvent {
    fn is_visible_to(&self, identity: &Identity) -> bool {
        // Сотрудник не получает чужих событий
        match self {
            WsEvent::PrinterAlert { .. } => identity.role == Role::Admin,
            _ => identity.role == Role::Admin || identity.role == Role::Specialist,
        }
    }
}
```

### Tauri events path (десктоп)
[CITED: v2.tauri.app/develop/calling-frontend/]
```rust
// В PrinterService / RequestService после mutation:
app_handle.emit("trackly-event", &ws_event).unwrap();

// Svelte frontend (dual-transport: браузер = WS, десктоп = listen):
// ui/src/lib/api/ws.ts
if (isTauri()) {
  import { listen } from '@tauri-apps/api/event';
  const unlisten = await listen<WsEvent>('trackly-event', (event) => {
    handleWsEvent(event.payload);
  });
  // cleanup: onDestroy → unlisten()
} else {
  // WebSocket path (браузер)
  const ws = new WebSocket(wsUrl);
  // reconnect с exponential backoff
}
```

### Frontend WS reconnect pattern
[ASSUMED: стандартный паттерн; нет специфики для этого проекта]
```typescript
// ui/src/lib/api/ws.ts
function connectWs() {
  const ws = new WebSocket(`wss://${location.host}/api/v1/ws`);
  
  ws.onmessage = (e) => handleWsEvent(JSON.parse(e.data));
  
  ws.onclose = () => {
    // exponential backoff: 1s, 2s, 4s, 8s, max 30s
    showReconnectingToast();
    setTimeout(connectWs, Math.min(delay * 2, 30000));
  };
}
```

---

## Retention / Downsample Strategy (D-Retention-01)

[ASSUMED: конкретная стратегия — на усмотрение планировщика; ниже рекомендация]

### Рекомендуемая стратегия (двухуровневая)

```
printer_readings:
├── "recent" zone:  последние 30 дней — все снимки (один на опрос, напр. каждые 5 мин)
└── "history" zone: старше 30 дней — 1 снимок в день (суточный агрегат)
```

**Prune SQL (фоновая задача, запускается вместе с poll task):**
```sql
-- Удалить строки старше retention_days
DELETE FROM printer_readings
WHERE ts_utc < (strftime('%s','now') - :retention_seconds);

-- Downsample: оставить один снимок в сутки для "истории"
-- (реализуется через дополнительную таблицу printer_readings_daily или
--  через прореживание с группировкой по date(ts_utc, 'unixepoch'))
```

**app_settings ключи:**
```
snmp_poll_interval_secs   = "300"    (5 мин по умолчанию)
snmp_poll_community       = "public"
snmp_discovery_range      = ""       (пусто = не задан)
printer_retention_days    = "90"     (90 дней по умолчанию)
printer_downsample_after_days = "30" (прореживать после 30 дней)
```

### Размер БД оценка
При 5-мин опросе, 10 принтеров, 90 дней хранения:
- "recent" (30 дней): 10 × 12 опросов/час × 24 ч × 30 дней = ~86 400 строк
- "history" (60 дней): 10 × 60 = 600 строк
- Итого: ~87 000 строк × ~100 байт = ~8.7 МБ — приемлемо для portable SQLite

---

## Common Pitfalls

### Pitfall 1: AsyncSession не имеет встроенного timeout
**What goes wrong:** `sess.get(oids).await` зависает навсегда на offline хосте или потерянном пакете.
**Why it happens:** snmp2 `AsyncSession` uses UDP; UDP не имеет connection timeout на уровне сокета.
**How to avoid:** ВСЕГДА оборачивать async SNMP вызов в `tokio::time::timeout(Duration::from_secs(2), ...)`.
**Warning signs:** Discovery зависает на конкретном IP диапазоне.

### Pitfall 2: `SyncSession` блокирует tokio thread
**What goes wrong:** Использование `SyncSession` внутри tokio async runtime вместо `AsyncSession` блокирует thread pool.
**Why it happens:** `SyncSession` — синхронный blocking I/O; в async контексте это deadlock потенциал.
**How to avoid:** Всегда использовать `AsyncSession` в async функциях; `SyncSession` только в `spawn_blocking`.
**Warning signs:** Tokio предупреждения "blocking operation in async context".

### Pitfall 3: SNMP OID индексы supply зависят от модели
**What goes wrong:** OID `1.3.6.1.2.1.43.11.1.1.9.1.1` работает для первого supply, но принтер может иметь supply с разными индексами (.1, .2, .3, .4).
**Why it happens:** Printer-MIB — таблица; index = номер записи, не фиксирован по цвету.
**How to avoid:** При discovery читать всю supply таблицу через `getnext`/`getbulk` начиная с `.43.11.1.1.9` до конца таблицы. В data-driven профиле хранить стартовый OID + тип прохода (indexed vs fixed).
**Warning signs:** Тонер показывает 0 или null для всех цветов, хотя принтер доступен.

### Pitfall 4: Secret<String> community не сериализуется в DTO
**What goes wrong:** Попытка включить community в PrinterDto для фронтенда.
**Why it happens:** `Secret<T>` явно не реализует `Serialize` (архитектурный инвариант Phase 5).
**How to avoid:** В DTO/API community не передаётся; на фронтенде только признак "community настроен" (bool) или маскированное "***". При сохранении — отдельное поле `community_update: Option<String>`.

### Pitfall 5: WS broadcast Lagged
**What goes wrong:** Медленный WS клиент не успевает читать события; `broadcast::Receiver::recv()` возвращает `RecvError::Lagged(n)`.
**Why it happens:** broadcast channel — ring buffer фиксированного размера.
**How to avoid:** Обрабатывать `Lagged` без паники — логировать, продолжить получение следующих событий. Клиент после reconnect делает re-fetch списков.

### Pitfall 6: WebSocket upgrade с Session extractor порядок
**What goes wrong:** Session extractor должен быть ДО вызова `ws.on_upgrade()` — если auth проверяется внутри callback, WebSocket уже установлен.
**Why it happens:** HTTP upgrade — однонаправленный; после него нельзя вернуть 401.
**How to avoid:** Паттерн: `session_identity(&session)` → если Err → `return 401`; иначе `ws.on_upgrade(move |socket| ...)`.

### Pitfall 7: Pantum тонер OID возвращает процент напрямую
**What goes wrong:** Для RFC3805 формула `level * 100 / max`, но Pantum `1.3.6.1.4.1.40093.6.3.1` уже возвращает процент (0-100).
**Why it happens:** Pantum не стандарт; их OID — не prtMarkerSuppliesLevel/MaxCapacity.
**How to avoid:** В OID-профиле Pantum указывать `toner_encoding = "percent"` (прямой процент), для RFC3805 — `toner_encoding = "level_over_max"`.

### Pitfall 8: requests.category — CHECK constraint или lookup
**What goes wrong:** Изменение набора категорий требует миграции если хранить в CHECK; lookup-таблица гибче.
**Why it happens:** Фиксированный набор «Ремонт техники/РМ/ПО/Прочее» — по CONTEXT.md не редактируется в UI Phase 6; но потенциально расширяется в Phase 7.
**How to avoid (рекомендация Claude's discretion):** Lookup-таблица `request_categories(id, name)` засевается миграцией. В `requests` хранить `category_id FK NULL`. Проще чем CHECK для будущего расширения.

---

## Code Examples

### Existing pattern: CartridgeService → PrinterService template
[CITED: trackly codebase `crates/trackly-app/src/services/cartridge_service.rs`]

PrinterService строится по тому же паттерну:
```rust
#[derive(Clone)]
pub struct PrinterService {
    pub writer: Arc<WriterHandle>,
    pub readers: Arc<ReaderPool>,
    clock: Arc<dyn Clock + Send + Sync>,
    printer_repo: Arc<SqlitePrinterRepository>,
    audit_repo: Arc<SqliteAuditLogRepository>,
    // Runtime-configured SNMP client (D-Mock-01)
    snmp_client: Arc<dyn SnmpClient + Send + Sync>,
    // Channel для on-demand refresh
    poll_tx: tokio::sync::mpsc::Sender<PrinterId>,
}
```

### SnmpClient port trait (D-Mock-01)
[ASSUMED: конкретная сигнатура — на усмотрение планировщика; паттерн из DeviceRepository]
```rust
// crates/trackly-core/src/ports/snmp.rs
#[async_trait::async_trait]
pub trait SnmpClient: Send + Sync {
    /// Fetch a set of OIDs from a target. Returns None if target is unreachable/timeout.
    async fn get_oids(
        &self,
        target: &str,
        community: &str,
        oids: &[&str],
        timeout_secs: u64,
    ) -> Result<Option<Vec<OidValue>>, AppError>;
    
    /// Discover: probe sysObjectID + sysDescr + sysName from target.
    async fn probe(
        &self,
        target: &str,
        community: &str,
    ) -> Result<Option<ProbedDevice>, AppError>;
}

// Real impl в trackly-infra (snmp2 AsyncSession)
pub struct RealSnmpClient;

// Mock impl для dev macOS (без реальных принтеров)
pub struct MockSnmpClient {
    pub fixtures: HashMap<String, PrinterFixture>,
}
```

### Runtime switching (D-Mock-01)
[ASSUMED: паттерн на основе config.rs Phase 5]
```rust
// В AppCtx::build или PrinterService::new:
let snmp_client: Arc<dyn SnmpClient + Send + Sync> = 
    if config.snmp.use_mock || std::env::var("TRACKLY_SNMP_MOCK").is_ok() {
        Arc::new(MockSnmpClient::default_fixtures())
    } else {
        Arc::new(RealSnmpClient)
    };
```

### Dual-transport: requests service (D-Req-Lifecycle-01)
[CITED: codebase `src/http/cartridges.rs` паттерн]
```rust
// tauri_cmds/requests.rs
#[tauri::command]
pub async fn requests_transition(
    payload: TransitionPayload,
    ctx: State<'_, AppCtx>,
    app: AppHandle,
) -> Result<RequestDto, AppErrorResponse> {
    let identity = ctx.desktop_identity();
    let result = build_requests_transition(&ctx, &identity, payload).await?;
    // Desktop push через Tauri event
    app.emit("trackly-event", WsEvent::RequestStatusChanged {
        request_id: result.id,
        new_status: result.status.clone(),
    }).ok();
    Ok(result)
}

// http/requests.rs — делегирует тому же build_ helper
pub async fn handler_requests_transition(
    session: Session,
    State(ctx): State<AppCtx>,
    Json(payload): Json<TransitionPayload>,
) -> Result<Json<RequestDto>, AppErrorResponse> {
    let identity = session_identity(&session).await?;
    let result = build_requests_transition(&ctx, &identity, payload).await?;
    // HTTP push через broadcast channel
    ctx.ws_broadcast.send(WsEvent::RequestStatusChanged {
        request_id: result.id,
        new_status: result.status.clone(),
    }).ok();
    Ok(Json(result))
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| SyncSession для всего | AsyncSession + tokio::time::timeout | snmp2 0.3+ | Не блокирует tokio thread pool |
| Hardcoded OID profiles в коде | Data-driven в БД | D-OID-01 (Phase 6 design) | Добавление вендоров без перекомпиляции |
| WebSocket polling interval | tokio::sync::broadcast fan-out | axum 0.7+ pattern | Нулевая задержка push; горизонтальная масштабируемость |
| JWT для LAN auth | tower-sessions cookie | Phase 5 D-Session-01 | Revocable sessions; проще для single-org LAN |
| OpenSSL для SNMP v3 | crypto-rust feature | snmp2 design | Portable build без DLL |

**Deprecated/outdated:**
- `SyncSession` в async runtime: не использовать без `spawn_blocking`
- `tokio::task::block_in_place` для SNMP I/O: правильный путь — `AsyncSession`

---

## Open Questions

1. **snmp2 0.4.x vs 0.5.x**
   - Что мы знаем: CLAUDE.md указывает 0.4.x; crates.io latest = 0.5.0 с дополнительным `get_many` методом
   - Что неясно: есть ли breaking changes в 0.5.x vs 0.4.x API; MSRV 0.4.x
   - Рекомендация: планировщику указать `snmp2 = { version = "0.4", features = [...] }` согласно CLAUDE.md; при необходимости bump до "0.5" после проверки MSRV

2. **Реальные OID для Pantum BM5100ADN**
   - Что мы знаем: `1.3.6.1.4.1.40093.6.3.1` проверен на похожих моделях (BM5100FDW) из community репортов
   - Что неясно: BM5100ADN может иметь другую версию прошивки и другие OID
   - Рекомендация: Plan включает human-verify checkpoint — тестирование на реальном устройстве до финального merge

3. **Структура alert storage (D-Alert-01)**
   - Что мы знаем: "один активный алерт на принтер, dedup, persist до разрешения"
   - Что неясно: отдельная таблица `printer_alerts` vs `status`-поле на `printers`
   - Рекомендация (Claude's discretion): **отдельная таблица** `printer_alerts(printer_id UNIQUE, alert_type, first_seen_utc, last_seen_utc, acknowledged_at_utc)` — легче добавлять поля (тип алерта, история), не раздувает `printers`

4. **USB-учёт форма (D-Schema-01)**
   - Что мы знаем: "признак/FK на host-device"
   - Рекомендация (Claude's discretion): `printers` добавляет `usb_host_device_id INTEGER NULL REFERENCES devices(id)` + CHECK что либо ip_address задан, либо usb_host_device_id — взаимоисключающие

5. **toner_levels индексация для цветных принтеров**
   - Что мы знаем: RFC3805 supply table — indexed (.1.1, .1.2, .1.3, .1.4 для CMYK)
   - Что неясно: нужно ли читать всю таблицу getbulk или достаточно фиксированных индексов
   - Рекомендация: OID-профиль хранит `max_supply_index` (для mono = 1, для CMYK = 4); цикл getbulk через таблицу

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | Компиляция snmp2 | ✓ | 1.92 (workspace) | — |
| tokio 1.x | AsyncSession, timeout, spawn | ✓ | workspace | — |
| Реальные принтеры (SNMP) | PRN-02/03 тестирование | ✗ | — | D-Mock-01 MockSnmpClient |
| macOS сетевой стек UDP:161 | SNMP I/O | ✓ | — | — |
| axum 0.8 | WS handler | ✓ | workspace (Phase 5) | — |
| tower-sessions | WS auth | ✓ | workspace (Phase 5) | — |

**Missing dependencies with no fallback:** нет блокирующих.

**Missing dependencies with fallback:**
- Реальные принтеры → MockSnmpClient (D-Mock-01); разработка и тесты полностью работают без железа

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | `cargo test` (unit/integration); `cargo nextest` (рекомендован) |
| Config file | Workspace `Cargo.toml` (нет отдельного test config) |
| Quick run command | `cargo test -p trackly-app -- printers 2>&1 \| head -50` |
| Full suite command | `cargo test --workspace` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| PRN-01 | Discovery: parse sysObjectID → vendor | unit | `cargo test -p trackly-app test_vendor_identify` | ❌ Wave 0 |
| PRN-02 | Snapshot parsing: level/max → percent | unit | `cargo test -p trackly-app test_toner_percent` | ❌ Wave 0 |
| PRN-03 | OID profile seed: 5 профилей в БД | integration | `cargo test -p trackly-app test_oid_profiles_seeded` | ❌ Wave 0 |
| PRN-06 | Alert detection: hrDeviceStatus=down → alert upsert | unit | `cargo test -p trackly-app test_alert_detection` | ❌ Wave 0 |
| PRN-08 | MockSnmpClient returns fixtures | unit | `cargo test -p trackly-infra test_mock_snmp` | ❌ Wave 0 |
| REQ-01 | RequestService::create persists to DB | integration | `cargo test -p trackly-app test_request_create` | ❌ Wave 0 |
| REQ-03 | Lifecycle: invalid transition → error | unit | `cargo test -p trackly-app test_request_lifecycle` | ❌ Wave 0 |
| REQ-04 | WS broadcast: event sent after request create | unit | `cargo test -p trackly-app test_ws_event_sent` | ❌ Wave 0 |
| REQ-05 | CART-07 link → request status=completed | integration | `cargo test -p trackly-app test_req_cart_link` | ❌ Wave 0 |
| D-Mock-01 | Runtime switch: TRACKLY_SNMP_MOCK → mock | unit | `cargo test -p trackly-app test_snmp_mock_switch` | ❌ Wave 0 |
| D-Retention | prune_old_readings deletes > retention | unit | `cargo test -p trackly-app test_readings_prune` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p trackly-app -- --test-thread=1 2>&1 | tail -20`
- **Per wave merge:** `cargo test --workspace`
- **Phase gate:** Full suite green before `/gsd-verify-work`

### Wave 0 Gaps
- [ ] `crates/trackly-app/src/services/printer_service.rs` — unit tests
- [ ] `crates/trackly-app/src/services/request_service.rs` — unit tests
- [ ] `crates/trackly-infra/src/snmp/mock.rs` — mock fixtures
- [ ] `migrations/V020+__*.sql` — миграции V020-V024
- [ ] `snmp2` в trackly-infra `Cargo.toml`

---

## Security Domain

> `security_enforcement: true` (config.json), `security_asvs_level: 1`.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | yes | tower-sessions cookie (Phase 5 D-Session-01); WS auth по session перед upgrade |
| V3 Session Management | yes | Уже реализовано Phase 5; WS-соединение не создаёт новую сессию |
| V4 Access Control | yes | `authorize(&identity, Action::*)` в сервис-слое; role-filter WS events server-side |
| V5 Input Validation | yes | IP range валидация (Ipv4Addr парсинг); community строка sanitization; request fields |
| V6 Cryptography | yes | `Secret<T>` для community (zeroize-on-drop); SNMP community — не хэш, но не логируется |

### Known Threat Patterns

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| SNMP community string exposure через logs | Information Disclosure | `Secret<String>` community; `Debug=***`; community не передаётся в DTO |
| SSRF через discovery IP range | Tampering | Валидация что IP в RFC1918 диапазоне (опционально); discovery только для admin |
| WS auth bypass (без session) | Elevation of Privilege | Session extractor ДО on_upgrade; 401 при отсутствии identity |
| Request lifecycle bypass через HTTP | Elevation of Privilege | authorize() в service layer; HTTP handler — thin adapter; 403 при неверной роли |
| SNMP injection в community string | Tampering | community — bytes, не SQL; rusqlite params! всегда parameterized |
| Broadcast leakage (сотрудник видит admin events) | Information Disclosure | Server-side role filter в `WsEvent::is_visible_to(&identity)` |

---

## Sources

### Primary (HIGH confidence)
- `codebase: crates/trackly-app/src/context.rs` — AppCtx structure, existing services, CancellationToken pattern
- `codebase: crates/trackly-infra/src/db/writer_worker.rs` — WriterHandle API
- `codebase: crates/trackly-core/src/ports/cartridges.rs` — port trait pattern
- `codebase: crates/trackly-core/src/primitives/secret.rs` — Secret<T>
- `codebase: migrations/V006__requests.sql` — existing requests schema
- `codebase: crates/trackly-app/src/http/mod.rs` — existing HTTP router structure
- [docs.rs/snmp2/0.4.9/snmp2](https://docs.rs/snmp2/0.4.9/snmp2/) — SyncSession/AsyncSession API
- [docs.rs/snmp2/latest](https://docs.rs/snmp2/latest/snmp2/) — latest API (0.5.x)
- [v2.tauri.app/develop/calling-frontend/](https://v2.tauri.app/develop/calling-frontend/) — AppHandle::emit API
- [docs.rs/tokio-util/latest/tokio_util/sync/struct.CancellationToken](https://docs.rs/tokio-util/latest/tokio_util/sync/struct.CancellationToken.html) — CancellationToken pattern

### Secondary (MEDIUM confidence)
- [github.com/glpi-project/glpi-agent/discussions/590](https://github.com/glpi-project/glpi-agent/discussions/590) — Pantum OIDs (проверены на похожих моделях)
- [ixnfo.com/en/hp-printers-snmp-oid-s-2.html](https://ixnfo.com/en/hp-printers-snmp-oid-s-2.html) — HP LaserJet OIDs
- [oidref.com/1.3.6.1.2.1.25.3.5.1.1](https://oidref.com/1.3.6.1.2.1.25.3.5.1.1) — hrPrinterStatus values
- [rfc-editor.org/rfc/rfc3805](https://www.rfc-editor.org/rfc/rfc3805) — RFC 3805 Printer-MIB
- [github.com/tokio-rs/axum WebSocket example](https://github.com/tokio-rs/axum/blob/main/examples/websockets/src/main.rs) — WS handler pattern

### Tertiary (LOW confidence)
- WebSearch results for vendor-specific OIDs (Canon iR, Kyocera ECOSYS) — требуют тестирования на реальных устройствах

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | snmp2 0.4.x MSRV ≤ 1.92 (workspace MSRV) | Standard Stack | Компиляция не пройдёт; нужен upgrade до 0.5.x или снижение MSRV |
| A2 | `get(&[oid1, oid2])` в snmp2 0.4.x принимает slice of Oid (не slice of refs) | snmp2 API Reference | Ошибка компиляции; планировщику проверить сигнатуру по docs.rs 0.4.x |
| A3 | Pantum BM5100ADN OID `1.3.6.1.4.1.40093.6.3.1` работает (проверен на BM5100FDW) | OID Reference Sets | Тонер не читается; fallback на RFC3805 как запасной вариант |
| A4 | Canon iR OID индексы supply 1-4 (CMYK) в ожидаемом порядке | OID Reference Sets | Неверный цвет в UI; requires snmpwalk на реальном устройстве |
| A5 | Session extractor в axum WS handler проверяется ДО on_upgrade callback | WS Pattern | Auth bypass; нужно верифицировать через integration test |
| A6 | `broadcast::Sender::send()` не блокирует (fire-and-forget) | WS Pattern | Паника если channel закрыт; нужен `.ok()` или явная обработка ошибки |
| A7 | snmp2 0.5.x не имеет breaking changes vs 0.4.x (только additive get_many) | Standard Stack | Нужна миграция если breaking; pin 0.4 как safe default |
| A8 | Стратегия retention: 30 дней full / downsample после | Retention Strategy | Неверная оценка размера БД; настраивается через app_settings |

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — snmp2 верифицирован cargo search; остальные зависимости уже в workspace
- Architecture: HIGH — паттерн прямо следует из Phase 5 AppCtx; WS из axum официальных примеров
- OID sets: MEDIUM — RFC3805 HIGH; vendor-specific MEDIUM (community-verified, не lab-tested)
- Pitfalls: HIGH — AsyncSession timeout pitfall подтверждён документацией; остальные из архитектурного анализа

**Research date:** 2026-06-14
**Valid until:** 2026-07-14 (30 дней для стабильного стека; vendor OID могут варьироваться с прошивкой принтера)
