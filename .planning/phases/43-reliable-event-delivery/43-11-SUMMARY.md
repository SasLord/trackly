---
phase: 43-reliable-event-delivery
plan: 11
subsystem: docs
tags: [adr, protocol, websocket, im, validation]
requires: ["43-08", "43-10"]
provides:
  - "ADR 0003: раздел «Протокол доставки (Фаза 43)» (подразделы 1-11) как спецификация для клиента Link Фазы 44"
  - "43-VALIDATION.md: карта проверок зелёная, nyquist_compliant: true"
affects: [44, 47]
key-files:
  modified:
    - docs/adr/0003-chat-model-fts-delivery.md
    - .planning/phases/43-reliable-event-delivery/43-VALIDATION.md
key-decisions:
  - "Проба живости клиента: текстовый ping при online/visibilitychange, 5 с (браузерный JS не видит протокольных ping/pong)"
  - "Одиночной догрузки по сигналу нет: на любой im_changed клиент вызывает im_sync"
  - "D-06: сообщение, изменённое после выхода участника, не отдаётся; снимок ревизии или надгробие передан Фазе 47"
requirements-completed: [DLV-01, DLV-02, DLV-03, DLV-04]
duration: ~1 ч
completed: 2026-11-27
---

# Phase 43 Plan 11: Протокол доставки и сплошной прогон Summary

Протокол доставки (кадры hello/resync_required/im_changed/pong, тайминги, ресинк, контракты im_send/im_sync, исключённый участник, идемпотентность) зафиксирован в ADR 0003 по фактическому коду; все гейты фазы зелёные.

## Задачи
1. Раздел «Протокол доставки (Фаза 43)» в ADR 0003 — `a538fc8f`
2. Сплошной прогон гейтов и обновление 43-VALIDATION.md — коды ниже

## Сверка констант с кодом
ping 25 с (`PING_INTERVAL`), тишина 60 с (`IDLE_TIMEOUT`), `SEND_TIMEOUT` 5 с, `WS_PROTOCOL_VERSION` 1, `CLIENT_TEXT_MAX` 256 (ws.rs); `SYNC_PAGE_DEFAULT` 50, `SYNC_PAGE_MAX` 200, `SYNC_RELOAD_THRESHOLD` 200, `CLIENT_ID_MAX_LEN` 64 (domain/im.rs); `MAX_CURSORS` 500 (im_service.rs); `RECONNECT_TOAST_DELAY_MS` 5000, `RESYNC_DEBOUNCE_MS` 500, `PROBE_TIMEOUT_MS` 5000, backoff 1000 -> 30000 (ws.ts).

## Гейты (код возврата)
| # | Команда | Код |
|---|---------|-----|
| 1 | `node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256` | 0 |
| 2 | `cargo fmt --all -- --check` | 0 |
| 3 | `cargo clippy --workspace --all-targets -- -D warnings` | 0 |
| 4 | `pnpm --dir ui lint` | 0 |
| 5 | `pnpm --dir ui svelte-check` | 0 |
| 6 | `pnpm --dir link-ui lint` / `pnpm --dir link-ui svelte-check` | 0 / 0 |
| 7 | `pnpm --dir ui build` | 0 |
| 8 | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test --workspace --no-fail-fast -- --test-threads=1` (запускал оркестратор) | 0 (31 бинарь, 3148 passed, 0 failed, 7 ignored) |

## Deviations from Plan
- Правило «сигнал + догрузка» в ADR описано по коду: маршрута одиночной догрузки нет, клиент всегда вызывает `im_sync` (план допускал одиночную догрузку при `rev == cursor+1`).
- Сплошной `cargo test` выполнил оркестратор, а не исполнитель.

Иначе — план выполнен как написан.

## Known Stubs
Нет.

## Threat Flags
Нет.

## Self-Check: PASSED
ADR и VALIDATION существуют; коммит `a538fc8f` присутствует; privacy-гейт 0.
