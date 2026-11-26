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

| Task ID | Requirement | Behavior | Test Type | Automated Command | File | Status |
|---------|-------------|----------|-----------|-------------------|------|--------|
| 43-01-T1 | DLV-02, DLV-03 | V058/V059: схема, UNIQUE, user_version | infra | `cargo test -p trackly-infra --test infra_tests im_migration` | crates/trackly-infra/tests/im_migration.rs | ⬜ pending |
| 43-01-T2 | DLV-02 | правила domain/im (окно видимости, client id) | unit | `cargo test -p trackly-core --lib domain::im` | crates/trackly-core/src/domain/im.rs | ⬜ pending |
| 43-01-T3 | DLV-02, DLV-03 | репозиторий im_sqlite, окно исключённого | infra | `cargo test -p trackly-infra --test infra_tests im_repo` | crates/trackly-infra/tests/im_repo.rs | ⬜ pending |
| 43-02-T1 | DLV-04 | hello, ping, закрытие молчащего, pong | integration | `cargo test -p trackly-app --test app_server -- ws_heartbeat` | tests/ws_heartbeat.rs | ⬜ pending |
| 43-02-T1 | DLV-04 | остановка сервера закрывает WS (регрессия) | regression | `cargo test -p trackly-app --test app_server -- ws_close_on_server_stop` | tests/ws_close_on_server_stop.rs | ✅ |
| 43-02-T2 | DLV-01 | Lagged общего канала -> resync_required | integration | `cargo test -p trackly-app --test app_server -- ws_resync_required` | tests/ws_resync_required.rs | ⬜ pending |
| 43-02-T3 | DLV-01 | мост десктопа: Lagged -> trackly-resync | unit/source | `cargo test -p trackly-app --test app_chat -- desktop_resync_bridge` | tests/desktop_resync_bridge.rs | ⬜ pending |
| 43-03-T2 | DLV-01, DLV-04 | клиент: ресинк, задержка тоста, online/visibilitychange, проба | JS-исполняющий гейт | `node ui/scripts/check-ws-resync.mjs --selftest && node ui/scripts/check-ws-resync.mjs` | ui/scripts/check-ws-resync.mjs | ⬜ pending |
| 43-03-T3 | DLV-04 | структурный гейт тоста (W1-W7) | gate | `node ui/scripts/check-ws-disconnect-toast.mjs --selftest && node ui/scripts/check-ws-disconnect-toast.mjs` | ui/scripts/check-ws-disconnect-toast.mjs | ⬜ pending |
| 43-04-T1 | DLV-02 | ImHub: маршрутизация, Lagged, сериализация | unit | `cargo test -p trackly-app --lib services::im_hub` | services/im_hub.rs | ⬜ pending |
| 43-04-T2 | DLV-02 | членство, сигналы, исключение (D-05) | integration | `cargo test -p trackly-app --test app_server -- im_membership` | tests/im_membership.rs | ⬜ pending |
| 43-04-T2 | DLV-03 | replay im_send | integration | `cargo test -p trackly-app --test app_server -- im_idempotent_send` | tests/im_idempotent_send.rs | ⬜ pending |
| 43-04-T3 | DLV-01, DLV-02 | im_sync: курсор, reload, read_only, неизменённые видны | integration | `cargo test -p trackly-app --test app_server -- im_sync` | tests/im_sync.rs | ⬜ pending |
| 43-05-T1 | DLV-01 | реестровый гейт ресинка | structural gate | `node ui/scripts/check-ws-resync-registry.mjs --selftest` | ui/scripts/check-ws-resync-registry.mjs | ⬜ pending |
| 43-07-T3 | DLV-01 | каждый onWsEvent имеет ресинк или освобождение | structural gate | `node ui/scripts/check-ws-resync-registry.mjs` | ui/scripts/check-ws-resync-registry.mjs | ⬜ pending |
| 43-07-T1 | DLV-01 | ресинк чата тихий (нет тостов) | JS-исполняющий гейт | `node ui/scripts/check-unread-store-behavior.mjs && node ui/scripts/check-chat-notify-behavior.mjs` | ui/scripts/check-unread-store-behavior.mjs | ⬜ pending |
| 43-06-T1 | DLV-03 | replay chat_send, в т.ч. на закрытой заявке | integration | `cargo test -p trackly-app --test app_chat -- chat_idempotent_send` | tests/chat_idempotent_send.rs | ⬜ pending |
| 43-06-T2 | DLV-03 | replay requests_create | integration | `cargo test -p trackly-app --test app_server -- request_idempotent_create` | tests/request_idempotent_create.rs | ⬜ pending |
| 43-08-T1 | DLV-02, DLV-03 | im_send/im_sync по HTTP: 401/404/403, replay | HTTP | `cargo test -p trackly-app --test app_server -- im_http` | tests/im_http.rs | ⬜ pending |
| 43-08-T1 | gates | паритет http-tauri (ROUTE_WITHOUT_CMD) | gate | `cargo test -p trackly-app --test app_gates -- http_route_parity` | tests/http_route_parity.rs | ⬜ pending |
| 43-08-T2 | DLV-01, DLV-02 | фан-аут IM на реальном WS, исключённый, Lagged IM | integration | `cargo test -p trackly-app --test app_server -- im_ws_fanout` | tests/im_ws_fanout.rs | ⬜ pending |
| 43-08-T3 | gates | suite_wiring и целевые наборы | gate | `cargo test -p trackly-app --test app_gates -- suite_wiring` | tests/suite_wiring.rs | ⬜ pending |
| 43-09-T1 | DLV-03 | sendWithRetry/newClientId | JS-исполняющий гейт | `node ui/scripts/check-send-retry.mjs --selftest && node ui/scripts/check-send-retry.mjs` | ui/scripts/check-send-retry.mjs | ⬜ pending |
| 43-09-T2 | DLV-03 | форма заявки держит один clientRequestId | structural gate | `node ui/scripts/check-request-create-idempotency.mjs --selftest && node ui/scripts/check-request-create-idempotency.mjs` | ui/scripts/check-request-create-idempotency.mjs | ⬜ pending |
| 43-10-T1 | DLV-03 | стор неотправленных: один id на все попытки | JS-исполняющий гейт | `node ui/scripts/check-chat-pending-behavior.mjs --selftest && node ui/scripts/check-chat-pending-behavior.mjs` | ui/scripts/check-chat-pending-behavior.mjs | ⬜ pending |
| 43-10-T2 | DLV-03 | структура UI чата («Не отправлено», «Повторить») | structural gate | `node ui/scripts/check-chat-ui.mjs --selftest && node ui/scripts/check-chat-ui.mjs` | ui/scripts/check-chat-ui.mjs | ⬜ pending |
| 43-11-T2 | all | сплошной прогон: fmt, clippy, cargo test --workspace, lint, svelte-check, build | full | см. план 43-11 | — | ⬜ pending |
| 43-12-T2/T3 | all | живая проверка LAN-браузер и десктоп | manual | чек-лист плана 43-12 | — | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

Имена целей сверены (43-PATTERNS.md): `app_server`, `app_chat`, `app_gates`, `infra_tests`.

---

## Wave 0 Requirements

Тесты создаются в тех же планах, что и код (TDD), отдельной волны 0 нет:

- [ ] crates/trackly-infra/tests/{im_migration,im_repo}.rs (план 43-01)
- [ ] crates/trackly-app/tests/{ws_heartbeat,ws_resync_required,desktop_resync_bridge}.rs (план 43-02)
- [ ] crates/trackly-app/tests/{im_support/mod.rs,im_membership,im_idempotent_send,im_sync}.rs (план 43-04)
- [ ] crates/trackly-app/tests/{chat_idempotent_send,request_idempotent_create}.rs (план 43-06)
- [ ] crates/trackly-app/tests/{im_http,im_ws_fanout}.rs (план 43-08)
- [ ] ui/scripts/{check-ws-resync,check-ws-resync-registry,check-send-retry,check-request-create-idempotency,check-chat-pending-behavior}.mjs с selftest и подключением в `pnpm lint` (планы 43-03, 43-05, 43-07, 43-09, 43-10)
- [ ] Каждый новый tests/*.rs подключён `#[path]` в tests/suites/*.rs (suite_wiring)

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
