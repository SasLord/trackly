---
status: resolved
trigger: "Two pre-existing UI bugs found during Phase 9 human-verify (NOT Phase 9 defects): (1) WS reconnect toast spam in server/browser mode; (2) Reports page constant reload/flicker. User chose to debug both in one session."
created: 2026-06-21
updated: 2026-06-21
---

# Debug Session: WS toast spam + Reports flicker

Two unrelated pre-existing UI bugs, debugged together in one session per user
request. Full prior diagnosis lives in `.planning/phases/09-ad/deferred-items.md`.

## Symptoms

### Bug A — WS reconnect toast spam (server mode, LAN browser)
- **Expected:** On a transient WS disconnect, show at most one «Соединение с
  сервером потеряно. Переподключение…» indication per disconnection episode,
  honoring exponential backoff.
- **Actual:** A `warning` toast appears roughly every second, indefinitely.
- **Errors (server log):** repeated `TLS accept error ... received fatal alert:
  CertificateUnknown` and `tls handshake eof`.
- **Repro:** Run in server mode, open the app in a LAN browser over `wss://`
  with the self-signed cert; observe continuous toasts.
- **Prior code-read diagnosis (not yet verified/fixed):**
  `ui/src/lib/api/ws.ts` — `ws.onclose` calls `showReconnectingToast()` on EVERY
  reconnect attempt (~line 67) with no dedup/throttle. Underlying handshake
  failure: browser does not trust the self-signed TLS cert for `wss://`
  (`CertificateUnknown`) — partly environmental, but the toast-spam UX is a real
  bug regardless.
- **Fix directions:** (a) throttle/dedup the reconnecting toast (once per
  disconnection episode, not per attempt) and honor existing exponential
  backoff; (b) investigate self-signed cert acceptance for WSS on same origin.
- **Scope:** server-mode WS/TLS infra (Phase 5 TLS / Phase 6 WS client).

### Bug B — Reports page constant reload/flicker (admin, desktop app)
- **Expected:** Reports («Отчёты») screen loads once per selection change and
  stays stable.
- **Actual:** The screen flickers / reloads continuously under admin.
- **Errors:** none reported (visual flicker).
- **Repro:** Log in as admin, open «Отчёты».
- **Prior code-read diagnosis (not yet verified/fixed):**
  `ui/src/features/reports/ReportsPage.svelte:320` — a Svelte 5 `$effect` reads
  `activeDomain`/`activeReport`/`period`/`filter` and calls `loadReport()` +
  `loadStatusCounts()`. Classic infinite-loop signature: the load path (or
  `PeriodSelector.svelte:78`'s `$effect` watching `dateFrom`/`dateTo`) likely
  reassigns one of the tracked objects (`period`/`filter`), re-triggering the
  effect.
- **Fix directions:** confirm which reactive dependency gets reassigned during
  load and break the cycle (compare-before-assign, untrack, or split read vs
  write state).
- **Scope:** Phase 7 (reports/dashboard).

## Current Focus

status: investigating (Bug A REOPENED after human-verify) — Bug B confirmed fixed
and verified by user; Bug A's original cert-trust diagnosis DISPROVEN by browser
evidence (handshake now succeeds: HTTP 101 Switching Protocols).

reasoning_checkpoint (Bug A — REVISED):
  hypothesis: "The WSS connection upgrades successfully (101) then the server closes/drops it ~1s later, repeatedly. Because each cycle includes a SUCCESSFUL onopen (which resets the per-episode `reconnecting` dedup flag), the new toast-dedup fix shows a fresh toast every cycle. Real root cause is server-side (or client-side) WS lifecycle: the connection is torn down ~1s after a successful upgrade — candidates: missing/short heartbeat (ping/pong) timeout, idle/read timeout in the axum WS handler, the server task dropping the WS sender right after upgrade, or a per-route subscription teardown (toast seen specifically in Принтеры and Заявки sections)."
  user_browser_evidence:
    - "Status: 101 Switching Protocols — handshake SUCCEEDS (cert IS trusted in this browser; CertificateUnknown theory no longer applies)."
    - "Console error every ~1s: `WebSocket connection to 'wss://127.0.0.1:8443/api/v1/ws' failed: The network connection was lost.`"
    - "Toast «переподключение» appears in sections Принтеры and Заявки."
    - "Origin https://127.0.0.1:8443, same-origin; CSP connect-src includes wss:. Response upgrade headers present and correct."
  next_investigation: "Read the SERVER WS handler (axum extract::ws, likely crates/trackly-app server WS route at /api/v1/ws) — look for: heartbeat/ping interval + close-on-timeout, idle/read timeout, whether the handler returns/drops the socket after the initial send, and any per-message or subscription loop that exits early. Then cross-check client ws.ts (does it send pings? does it close on its own after N ms?). Determine which side initiates the ~1s close."

next_action: RESOLVED. Both bugs fixed and human-verified. Bug B confirmed fixed by user (2026-06-21). Bug A confirmed fixed by user after rebuild + LAN-browser re-verify (2026-06-21): WS upgrades to 101 and stays open, no reconnect-toast loop. Session archived.

reasoning_checkpoint (Bug A — CANDIDATE ROOT CAUSE):
  hypothesis: "The manual hyper accept-loop in crates/trackly-app/src/server/mod.rs drives connections with `hyper::server::conn::http1::Builder::new().serve_connection(io, hyper_service)` WITHOUT `.with_upgrades()`. axum's WebSocketUpgrade returns the 101 response (so the client sees a successful handshake) and spawns a task awaiting `hyper::upgrade::on(req)` inside on_upgrade. But that upgrade future only resolves when the connection is polled via `.with_upgrades()`. Without it, hyper writes the 101, then completes/closes the connection — the upgraded stream is never handed to handle_ws_socket, so the socket is torn down ~1s after the client observes the 101. The client reconnects, repeats, and each successful onopen resets the per-episode toast dedup → a toast every cycle."
  confirming_evidence:
    - "grep: `.with_upgrades(` appears NOWHERE in crates/trackly-app/src. serve_connection is called bare at server/mod.rs:104-105."
    - "Browser sees HTTP 101 Switching Protocols (axum emits the 101 response object), but socket drops ~1s later — exact signature of a 101 sent without the hyper upgrade being driven."
    - "Tauri/desktop path (event listen, no WS) is unaffected — matches: only the browser WS over wss:// breaks."
  falsification_test: "Add tracing at the top of handle_ws_socket. Connect a real WS client over wss://127.0.0.1:8443/api/v1/ws. If handle_ws_socket NEVER logs (socket still drops ~1s later), the on_upgrade future is not resolving → confirms missing .with_upgrades(). If it DOES log and then breaks, the bug is inside the select loop instead."
  fix_rationale: "Adding .with_upgrades() to serve_connection makes hyper poll the connection for protocol upgrades, resolving hyper::upgrade::on(req) and handing the upgraded stream to axum's on_upgrade callback → handle_ws_socket runs and the socket stays open. Addresses the lifecycle root cause, not the toast symptom."
  blind_spots: "oneshot consumes the Router per request — need to confirm .with_upgrades() composes with the service_fn(oneshot) pattern (the upgrade future is tied to the request, driven by the connection, not the service). Also confirm http1 builder is the right place (HTTP/1.1 only; WS requires HTTP/1.1 anyway)."

reasoning_checkpoint (Bug B — RESOLVED, kept for record):

reasoning_checkpoint (Bug B):
  hypothesis: "The shared $effect synchronously calls loadStatusCounts(), whose `if (countsLoading) return;` guard reads the countsLoading $state INSIDE the effect, subscribing the effect to countsLoading. loadStatusCounts then writes countsLoading (true, then false in .finally), and the .finally false-write re-triggers the effect → unbounded async loop."
  confirming_evidence:
    - "Compiled codegen: guard is `if ($.get(countsLoading)) return;` executed synchronously within the user_effect."
    - "Runtime: loop present with loadStatusCounts enabled (~874k calls/sec); EXACTLY 1 call and NO loop when loadStatusCounts is commented out. loadReport (no such guard) never loops."
  falsification_test: "Remove the reactive read of countsLoading from the effect's synchronous path; if loop persists, hypothesis is wrong."
  fix_rationale: "Root cause is a tracked read of internal state (countsLoading) inside an effect. Fix removes that coupling so the effect only depends on activeDomain/activeReport/period/filter. Addresses cause, not symptom."
  blind_spots: "Range-mode period editing (PeriodSelector $effect→onPeriodChange→period reassign) is a separate, legitimate re-trigger; verify it still loads once per change, not repeatedly, after the fix."

next_action: apply Bug B fix (decouple countsLoading from effect), apply Bug A fix (dedup reconnect toast per episode), then `pnpm build` + svelte-check + human-verify.

## Evidence

- timestamp: 2026-06-21
  checked: git history of ReportsPage.svelte $effect
  found: Before commit dcb08c3 the $effect called ONLY loadReport() (sets rows/loading/error — none tracked) and did NOT flicker. dcb08c3 added loadStatusCounts() to the same effect. loadStatusCounts sets statusCounts/countsLoading (neither tracked by the effect).
  implication: The flicker was introduced by adding loadStatusCounts(). loadReport passes period directly + filter via spread {...filter}; loadStatusCounts passes BOTH period and filter as raw $state proxies. Since loadReport already passed period directly pre-commit without looping, "passing a proxy to invoke" alone is not the trigger — need to find what loadStatusCounts writes to a tracked dep.

- timestamp: 2026-06-21
  checked: Compiled $effect codegen (svelte/compiler) for ReportsPage.svelte
  found: effect reads variable signals activeDomain/activeReport/period/filter. loadReport()+loadStatusCounts() are called SYNCHRONOUSLY inside the effect, so every $.get() they perform during that synchronous pass registers as an effect dependency. loadStatusCounts begins with `if ($.get(countsLoading)) return;` — reading countsLoading INSIDE the effect → effect subscribes to countsLoading.
  implication: countsLoading becomes a hidden tracked dependency of the effect even though it is logically internal state.

- timestamp: 2026-06-21
  checked: RUNTIME repro — `cargo tauri dev`, forced initial hash to #/reports (desktop auto-admin), added tracing::warn DBG_REPORT_CALL to build_reports_list_device_acts + build_reports_get_report_counts.
  found: With both loads in the effect → ~874,000 DBG_REPORT_CALL lines in seconds (infinite loop, ~6 calls/ms, alternating list_device_acts + get_report_counts). With loadStatusCounts() commented out → EXACTLY 1 list_device_acts call, 0 get_report_counts, NO loop.
  implication: CONFIRMED — loadStatusCounts() is the sole loop driver. The loop is async-driven: effect subscribes to countsLoading via the guard read; loadStatusCounts sets countsLoading=true then (in .finally) countsLoading=false; the false write re-triggers the effect, which calls loadStatusCounts again, ad infinitum.

- timestamp: 2026-06-21
  checked: CONFIRMED ROOT CAUSE of Bug A — grep + code-read of crates/trackly-app/src/server/mod.rs.
  found: The manual hyper accept-loop drives connections with `hyper::server::conn::http1::Builder::new().serve_connection(io, hyper_service).await` and NEVER calls `.with_upgrades()`. `grep -rn '.with_upgrades('` over crates/trackly-app/src returns ZERO hits. axum's WebSocketUpgrade emits the 101 response (client sees a successful handshake) then awaits `hyper::upgrade::on(req)` inside on_upgrade; that future only resolves when the hyper connection is polled with `.with_upgrades()`. Without it, hyper writes the 101, the connection future completes, and the socket closes — torn down ~1s after the client observes the 101. handle_ws_socket never runs server-side. Tauri/desktop path (event listen, no WS) unaffected → matches "only browser wss:// breaks".
  implication: Root cause is server-side WS upgrade plumbing, NOT the client and NOT the toast logic. The per-episode toast dedup (last cycle) is a correct UX safety net but did not fix the underlying ~1s teardown.

- timestamp: 2026-06-21
  checked: RUNTIME confirmation + regression test — new crates/trackly-app/tests/ws_upgrade_serve_connection.rs. Replicates the EXACT server/mod.rs plumbing (service_fn + oneshot + serve_connection) over plain TCP (TLS orthogonal to upgrades) with a minimal axum WS echo router; a real tokio-tungstenite client attempts a text round-trip within a 3s timeout. Parameterized on with_upgrades on/off.
  found: WITHOUT .with_upgrades() → client never completes a round-trip (socket torn down post-101): test ws_upgrade_fails_without_upgrades PASSES (reproduces Bug A). WITH .with_upgrades() → WS upgrades, stays open, echoes "echo:ping": test ws_upgrade_succeeds_with_upgrades PASSES (confirms fix). Both run green.
  implication: Falsification test satisfied — the on_upgrade future does not resolve without .with_upgrades(). Root cause CONFIRMED with runtime evidence. Fix = add .with_upgrades() to serve_connection in server/mod.rs.

- timestamp: 2026-06-21
  checked: HUMAN-VERIFY of Bug A in real LAN browser (Safari 26.5 over wss://127.0.0.1:8443).
  found: WS handshake SUCCEEDS — "Status: 101 Switching Protocols", correct Upgrade/Sec-WebSocket-Accept headers, same-origin, CSP connect-src allows wss:. But the connection then drops ~1s later, repeatedly. Browser console logs every ~1s: "WebSocket connection to 'wss://127.0.0.1:8443/api/v1/ws' failed: The network connection was lost." Toast appears in sections Принтеры and Заявки.
  implication: Bug A is NOT a cert-trust / CertificateUnknown problem (handshake succeeds). The socket is established then torn down ~1s after upgrade, on a loop. The toast-dedup fix is insufficient because each cycle's successful onopen resets the per-episode `reconnecting` flag, so a toast fires every cycle. Real root cause = WS lifecycle (server or client closing the socket ~1s post-upgrade). Investigate the axum server WS handler at /api/v1/ws (heartbeat/idle timeout, handler dropping the sender, early loop exit) and client ws.ts ping behavior.

## Resolution

root_cause: |
  Bug A (WS drops ~1s after upgrade): CONFIRMED. crates/trackly-app/src/server/mod.rs
  drives connections with `hyper::server::conn::http1::Builder::new()
  .serve_connection(io, hyper_service).await` WITHOUT `.with_upgrades()`. axum's
  WebSocketUpgrade returns the 101 response (so the browser sees a successful
  handshake) and then awaits `hyper::upgrade::on(req)` inside on_upgrade to obtain
  the upgraded stream. That upgrade future only resolves when the hyper connection
  is driven with `.with_upgrades()`. Without it, hyper writes the 101, the
  connection future completes, and the socket is closed — torn down ~1s after the
  client observes the 101. handle_ws_socket never runs server-side; the client
  reconnects, repeats, and each successful onopen resets the per-episode toast
  dedup → a toast every ~1s. Confirmed via runtime test ws_upgrade_serve_connection.rs
  (round-trip fails without .with_upgrades(), succeeds with it). The original
  cert-trust/CertificateUnknown diagnosis was DISPROVEN (handshake succeeds, 101).
  NOTE: the toast-dedup change in ws.ts is an independent, correct UX safety net for
  genuine transient disconnects and is kept, but it was never the root cause.

  Bug B (Reports infinite reload): ui/src/features/reports/ReportsPage.svelte —
  the auto-reload $effect synchronously calls loadStatusCounts(), whose first
  line `if (countsLoading) return;` READS the reactive `countsLoading` $state
  inside the effect, making countsLoading a hidden effect dependency.
  loadStatusCounts then writes countsLoading (true, then false in .finally); the
  async false-write re-triggers the effect, which calls loadStatusCounts again →
  unbounded loop (measured ~874,000 calls/sec). Introduced by commit dcb08c3
  (07-14) which added loadStatusCounts() into the previously-safe effect.

fix: |
  Bug A (REAL FIX): add `.with_upgrades()` to the `serve_connection` call in
  crates/trackly-app/src/server/mod.rs so hyper drives the connection for protocol
  upgrades, resolving `hyper::upgrade::on(req)` and handing the upgraded stream to
  axum's on_upgrade → handle_ws_socket runs and the socket stays open. One-line
  change + explanatory comment. (Prior cycle's ws.ts toast dedup is kept as an
  independent UX improvement, not the fix.)

  Bug B: make `countsLoading` a plain non-reactive `let` (was `$state`) — it is
  never read in the template, only used as an internal overlap guard. Removing
  reactivity means reading/writing it inside the effect no longer creates a
  dependency, breaking the loop. Guard `if (countsLoading) return;` retained
  (now safe). Added explanatory comments at both sites.

verification: |
  Bug B: RUNTIME verified. Re-ran `cargo tauri dev` forced to #/reports
  (desktop auto-admin) with temporary backend tracing. After fix: exactly 1
  list_device_acts + 1 get_report_counts call on open, then stable (was ~874k/s
  before). All temporary instrumentation reverted afterward.
  svelte-check: 0 errors (36 pre-existing warnings, none from these changes).
  pnpm build: succeeded; ui/dist refreshed for LAN-browser verification.
  Bug A: RUNTIME verified via crates/trackly-app/tests/ws_upgrade_serve_connection.rs —
  without .with_upgrades() a real WS round-trip fails (reproduces the bug); with it,
  the WS upgrades and echoes (confirms fix). Full trackly-app suite passes with
  TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 (0 failures; one pre-existing AD test needs
  the mock env, unrelated to this change). cargo build -p trackly-app clean; clippy
  clean on changed files. Needs final human-verify in the real server/LAN-browser
  (TLS environment not reachable from dev macOS) to confirm the ~1s toast loop is
  gone end-to-end.

files_changed:
  - crates/trackly-app/src/server/mod.rs (Bug A REAL FIX: .with_upgrades() on serve_connection)
  - crates/trackly-app/tests/ws_upgrade_serve_connection.rs (Bug A regression test — NEW)
  - crates/trackly-app/Cargo.toml (dev-deps: tokio-tungstenite, futures-util for the WS test)
  - ui/src/lib/api/ws.ts (Bug A: per-episode toast dedup — independent UX safety net, prior cycle)
  - ui/src/features/reports/ReportsPage.svelte (Bug B: non-reactive countsLoading guard — prior cycle, RESOLVED)

## Eliminated

- hypothesis: Tauri invoke serialization of $state proxies mutates them and bumps a tracked version signal.
  evidence: @tauri-apps/api core.js invoke passes args to native IPC; serialization is JSON.stringify-equivalent (read-only) and happens async outside effect tracking. loadReport also passes period directly yet does NOT loop. Disabling loadStatusCounts alone stopped the loop while loadReport stayed.
  timestamp: 2026-06-21

- hypothesis: PeriodSelector.svelte:78 $effect calling onPeriodChange reassigns period and drives the loop.
  evidence: default mode is 'month' → that effect returns early before calling onPeriodChange. Disabling loadStatusCounts (leaving PeriodSelector untouched) stopped the loop entirely.
  timestamp: 2026-06-21

- hypothesis: (Bug A) The WS toast spam is caused by the browser not trusting the self-signed TLS cert (CertificateUnknown), so the wss:// handshake never completes and reconnect attempts spam toasts.
  evidence: Human-verify in real LAN browser shows the handshake SUCCEEDS — "Status: 101 Switching Protocols" with valid Upgrade/Sec-WebSocket-Accept headers. The connection is established, then drops ~1s later ("The network connection was lost"). A failed handshake would never reach 101. Root cause is therefore post-upgrade WS teardown, not cert trust.
  timestamp: 2026-06-21
