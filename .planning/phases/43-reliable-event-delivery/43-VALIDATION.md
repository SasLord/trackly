---
phase: 43
slug: reliable-event-delivery
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-11-26
---

# Phase 43 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Источник: `43-RESEARCH.md` → «Validation Architecture».

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (Rust, suite-бинарники `tests/suites/*.rs`) + Node-гейты `ui/scripts/check-*.mjs` |
| **Config file** | `crates/trackly-app/Cargo.toml` (`[[test]]`, `autotests = false`), `ui/package.json` (`lint`) |
| **Quick run command** | `cargo test -p trackly-app --test <suite> <filter>` (скоуп по цели) + затронутый `node ui/scripts/check-*.mjs` |
| **Full suite command** | `ci-full` (один раз на границе последней волны, в фоне; `timeout` на машине нет) |
| **Estimated runtime** | скоуп-цель ~1–3 мин; сплошной прогон — часы |

Отдельные команды (не входят друг в друга): `pnpm --dir ui lint`, `pnpm --dir ui svelte-check`, `cargo clippy -- -D warnings`, `cargo fmt --check`. Одновременно — только один `cargo test`.

---

## Sampling Rate

- **After every task commit:** скоуп-тест затронутой цели + соответствующий `check-*.mjs`
- **After every plan wave:** целевые suite (`server`, infra-suite миграций, `http_route_parity`, `suite_wiring`) + `pnpm --dir ui lint` + `pnpm --dir ui svelte-check` + `cargo clippy -D warnings` + `cargo fmt --check`
- **Before `/gsd-verify-work`:** сплошной `ci-full` зелёный
- **Max feedback latency:** ~180 s (скоуп-цель)

---

## Per-Task Verification Map

Заполняется по планам (ID задач — после планирования). Карта требований → тестов:

| Requirement | Behavior | Test Type | Automated Command | File Exists | Status |
|-------------|----------|-----------|-------------------|-------------|--------|
| DLV-01 | `Lagged` общего канала и канала IM → `resync_required` | integration | `cargo test -p trackly-app --test server ws_resync` | ❌ W0 | ⬜ pending |
| DLV-01 | Мост десктопа: Lagged → `trackly-resync` | unit | `cargo test -p trackly-app --lib desktop_event_bridge` | ❌ W0 | ⬜ pending |
| DLV-01 | `im_sync` по курсору `rev`, `reload:true` по порогу | integration | `cargo test -p trackly-app --test server im_sync` | ❌ W0 | ⬜ pending |
| DLV-01 | Клиентский ресинк на `onopen`-после-обрыва и `resync_required` | JS-исполняющий гейт | `node ui/scripts/check-ws-resync.mjs --selftest && node ui/scripts/check-ws-resync.mjs` | ❌ W0 | ⬜ pending |
| DLV-01 | Реестр: каждый `onWsEvent` имеет ресинк или освобождение | structural gate | `node ui/scripts/check-ws-resync-registry.mjs` | ❌ W0 | ⬜ pending |
| DLV-02 | Не-участник не получает сигнал | integration | `cargo test -p trackly-app --test server im_ws_fanout` | ❌ W0 | ⬜ pending |
| DLV-02 | Исключение: следующий сигнал не приходит; история до выхода видна; правки после — нет | integration | `cargo test -p trackly-app --test server im_membership` | ❌ W0 | ⬜ pending |
| DLV-02 | `im_send`/`im_sync` отвергают не-участника | HTTP | `cargo test -p trackly-app --test server im_http` | ❌ W0 | ⬜ pending |
| DLV-03 | Повтор `im_send`/`chat_send`/`requests_create` с тем же id → одна сущность, сигнал не повторён | integration | `cargo test -p trackly-app --test <suite> idempot` | ❌ W0 | ⬜ pending |
| DLV-03 | UNIQUE-индексы, `user_version` миграций | infra | `cargo test -p trackly-infra --test <suite> im_migration` | ❌ W0 | ⬜ pending |
| DLV-04 | `hello`, ping, закрытие молчащего peer (инжектируемые тайминги) | integration | `cargo test -p trackly-app --test server ws_heartbeat` | ❌ W0 | ⬜ pending |
| DLV-04 | Остановка сервера закрывает WS (регрессия) | regression | `cargo test -p trackly-app --test server ws_close_on_server_stop` | ✅ | ⬜ pending |
| DLV-04 | Тост через 5 с, ≤1 за эпизод, отмена на `onopen`, `online`/`visibilitychange` | JS-исполняющий гейт | `node ui/scripts/check-ws-disconnect-toast.mjs --selftest && node ui/scripts/check-ws-disconnect-toast.mjs` | ✅ расширить | ⬜ pending |
| gates | Паритет http↔tauri (`ROUTE_WITHOUT_CMD`) | gate | `cargo test -p trackly-app --test <suite> http_route_parity` | ✅ править | ⬜ pending |
| gates | `suite_wiring` | gate | `cargo test -p trackly-app --test suite_wiring` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

Имена `--test <suite>` сверяются с `[[test]]` в `Cargo.toml` при планировании.

---

## Wave 0 Requirements

- [ ] `crates/trackly-app/tests/{ws_heartbeat,ws_resync_required,im_ws_fanout,im_membership,im_sync,im_http,idempotent_send}.rs` + подключение в `tests/suites/*.rs`
- [ ] `crates/trackly-infra/tests/im_migration.rs` (V058/V059)
- [ ] `im_support/mod.rs` — хелперы IM-тестов
- [ ] `ui/scripts/check-ws-resync-registry.mjs`, `check-ws-resync.mjs` с selftest, подключение в `pnpm lint`

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Ресинк экранов в десктопе после Lagged/обрыва | DLV-01 | WKWebView/WebView2, синтетический харнесс ≠ проверка | Запустить приложение, вызвать обрыв (остановка сервера/сон), убедиться, что списки и колокольчик догрузились без ручного обновления |
| Тост не мигает при быстром переподключении, появляется при остановке сервера | DLV-04 | Реальный браузер и таймеры | В LAN-браузере: кратко разорвать сеть (<5 с) — тоста нет; остановить сервер — тост есть |
| «Не отправлено» / «Повторить» в чате заявки | DLV-03 | Сетевой сбой в реальном браузере | Отключить сеть, отправить сообщение, увидеть пометку, включить сеть, «Повторить» — одно сообщение без дубля |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 180s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
