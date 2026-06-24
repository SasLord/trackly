---
phase: 12-cartridge-request-interconnection
plan: 17
subsystem: ui
tags: [websocket, svelte, frontend, notifications, singleton, refcount]

# Dependency graph
requires:
  - phase: 12-cartridge-request-interconnection
    provides: "WsEvent::RequestStatusChanged + requested_by_user_id (12-11) feeding EmployeeLayout's per-user toast"
provides:
  - "Refcounted singleton connectWs() in ui/src/lib/api/ws.ts — one real WebSocket/Tauri-listen connection per process regardless of consumer count"
affects: [request-status-notifications, printer-alert-notifications, any-future-ws-consumer]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Refcounted module-singleton connection: refCount + activeCleanup module state, idempotent release() closures, real teardown only at refCount 1->0"

key-files:
  created: []
  modified:
    - ui/src/lib/api/ws.ts

key-decisions:
  - "Idempotency keyed on refCount, not on `ws !== null` — the browser branch nulls `ws` on every reconnect cycle, so a null-check would be unreliable across retries"
  - "Reconnect loop in connectBrowser()'s onclose bails when refCount<=0 as defence-in-depth, even though activeCleanup already nulls onclose before close() — guards against a stray close racing the release"
  - "Removed dead disconnectFn module variable (superseded by activeCleanup) to keep svelte-check's noUnusedLocals-equivalent check green"

patterns-established:
  - "Refcounted singleton for shared client-side transport state when multiple independent onMount call sites need to multiplex one underlying connection without changing their own teardown contract"

requirements-completed: [REQ-04, D-WS-01]

# Metrics
duration: 12min
completed: 2026-06-24
---

# Phase 12 Plan 17: connectWs() refcounted singleton Summary

**connectWs() in ws.ts rewritten as a refcounted module-singleton — one WebSocket (browser) or one `listen('trackly-event')` subscription (Tauri) shared across all concurrent onMount consumers, closing GAP-12-10's duplicate-toast bug without touching any call site.**

## Performance

- **Duration:** 12 min
- **Started:** 2026-06-24T15:45:00Z
- **Completed:** 2026-06-24T15:48:45Z
- **Tasks:** 1 completed
- **Files modified:** 1

## Accomplishments
- `connectWs()` now increments a module-level `refCount`; only the first concurrent caller (0→1) actually opens the transport (Tauri `listen()` or browser `WebSocket`).
- The function returned to every caller is an idempotent `release()` closure (guarded by a local `released` flag) that decrements `refCount` and triggers the real `activeCleanup()` (unlisten/`ws.close()`) only when the **last** consumer releases (1→0).
- Browser reconnect loop (`ws.onclose`) now bails immediately if `refCount <= 0`, so an orphaned connection can never resurrect itself after the last consumer has detached — this is defence-in-depth on top of the existing `ws.onclose = null` nulling that `activeCleanup` performs.
- `disconnectWs()` retains its "force everything to zero" semantics for the logout path: resets `refCount` to 0 and invokes `activeCleanup()` unconditionally.
- Public contract (`onWsEvent`, `connectWs`, `disconnectWs`) is byte-for-byte unchanged; all three existing consumers (`EmployeeLayout.svelte`, `RequestsPage.svelte`, `PrintersPage.svelte`) needed zero edits — confirmed via `git diff --name-only`.

## Task Commits

Each task was committed atomically:

1. **Task 1: Сделать connectWs() refcounted-синглтоном для browser и Tauri веток** - `2e82924` (fix)

**Plan metadata:** (this commit, via final docs commit below)

## Files Created/Modified
- `ui/src/lib/api/ws.ts` - Replaced single-shot `disconnectFn` model with `refCount` + `activeCleanup` refcounted singleton; `connectWs()` gates real connection setup on `refCount === 1`, returns an idempotent release closure; browser reconnect loop checks `refCount <= 0` before re-arming; `disconnectWs()` forces refCount to 0.

## Decisions Made
- Idempotency is keyed on `refCount`, not `ws !== null` — the browser branch nulls `ws` on every reconnect cycle (intentional pre-existing behavior), so a null-check on `ws` would falsely treat a mid-reconnect window as "disconnected" and could double-open a connection.
- Removed the now-dead `disconnectFn` module variable (fully superseded by `activeCleanup`) after `svelte-check` flagged it as an unused-but-assigned binding (`'disconnectFn' is declared but its value is never read`) — see Deviations below.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Removed dead `disconnectFn` variable left over from refactor**
- **Found during:** Task 1 (initial `svelte-check` run after the rewrite)
- **Issue:** The plan's action steps described introducing `activeCleanup` alongside the existing `disconnectFn`, but once `activeCleanup` became the single source of truth for teardown, `disconnectFn` was only ever assigned and never read — `svelte-check` failed with `'disconnectFn' is declared but its value is never read` (1 error).
- **Fix:** Removed the `disconnectFn` module variable and all of its assignments; `activeCleanup` (already tracked per the plan's design) is now the sole teardown reference used by both the `connectWs()` release closure and `disconnectWs()`.
- **Files modified:** `ui/src/lib/api/ws.ts`
- **Verification:** `pnpm --dir ui exec svelte-check` → 0 errors (was 1 error, 36 pre-existing unrelated warnings unchanged)
- **Committed in:** `2e82924` (part of Task 1 commit — found and fixed before the task commit was made, no separate commit needed)

---

**Total deviations:** 1 auto-fixed (1 bug — dead variable causing a type-check failure)
**Impact on plan:** Pure cleanup internal to the rewritten file; no behavior change, no scope creep. Public contract and acceptance criteria unaffected.

## Issues Encountered
None beyond the auto-fixed dead-variable issue above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- GAP-12-10 closed: one backend `RequestStatusChanged`/`printer_alert` event now produces exactly one client-side dispatch regardless of how many pages have WS consumers mounted simultaneously (e.g. admin viewing Заявки + Принтеры at once).
- `pnpm --dir ui exec svelte-check` → 0 errors; `pnpm --dir ui build` succeeds (warnings present are pre-existing and unrelated: CSS unused-selector notices in other components, dynamic/static import duplication notices for `toast.svelte.ts`/`client.ts` — none touch `ws.ts`).
- Behavioral confirmation (live browser, multiple simultaneous WS-consumer pages, one toast per status change) is a manual/live-session step not exercised in this non-interactive execution; code-level guarantee is sound (verified via the Node single-thread reasoning check: synchronous `refCount += 1` before any `await` means concurrent `onMount` calls can never both see `refCount === 1`).
- No blockers for closing out the remaining Round 3 gap-closure plans (GAP-12-09/11/12 — already closed per STATE.md; this was the last of the 4 plans, GAP-12-10).

## Self-Check: PASSED

- FOUND: `/Users/madsas/Projects/trackly/ui/src/lib/api/ws.ts`
- FOUND: commit `2e82924` (`git log --oneline --all | grep 2e82924` → match)

---
*Phase: 12-cartridge-request-interconnection*
*Completed: 2026-06-24*
