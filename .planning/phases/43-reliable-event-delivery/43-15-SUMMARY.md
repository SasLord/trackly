---
phase: 43-reliable-event-delivery
plan: 15
subsystem: transport
tags: [gap-closure, retry, WR-07, DLV-03]
requires: []
provides:
  - "isNetworkFailure различает сетевой сбой и HTTP-ответ"
  - "apiCall бросает NETWORK_ERROR при отказе fetch и HTTP_<status> для ответов без формы AppError"
affects: [43-21]
key-files:
  modified:
    - packages/transport/src/client.ts
    - packages/transport/src/sendWithRetry.ts
    - ui/scripts/check-send-retry.mjs
key-decisions:
  - "Повторяемые коды вынесены в RETRYABLE_CODES: NETWORK_ERROR, HTTP_502/503/504; плюс TypeError отказа fetch по регэкспу"
requirements-completed: [DLV-03]
completed: 2026-11-27
---

# Phase 43 Plan 15: WR-07 повторы только для сетевых сбоев Summary

Автоповторы получают только отказ fetch и HTTP 502/503/504; 400/413/422 с не-JSON телом, доменные AppError и программные TypeError больше не повторяются, id попыток прежний.

## Что сделано
- **client.ts:** fetch обёрнут в try/catch, отказ даёт `{code:'NETWORK_ERROR', message:'Нет связи с сервером'}`; `!res.ok` с телом не формы AppError даёт `HTTP_<status>` с details `{status}`, тело в message не попадает (T-43-15-02). Хуки 401/403 по res.status сохранены.
- **sendWithRetry.ts:** `isNetworkFailure` = код из `RETRYABLE_CODES` либо `TypeError` с сообщением /fetch|network|load failed/i. Использования `UnknownError` убраны.
- **check-send-retry.mjs:** кейсы S7 (HTTP_422/413), S8 (502/503/504), S9 (программный TypeError против Failed to fetch/Load failed), S10 (NETWORK_ERROR); S3 переведён на сообщение «Failed to fetch N», т.к. прежнее `fail${n}` теперь справедливо не повторяется.

## RED (коммит 6914f3ee, прежний isNetworkFailure)
Красные: S8 (3 кейса), S9, S10. S7 на прежней реализации зелёный: синтетический объект с кодом HTTP_422 уже не UnknownError. Сам дефект WR-07 (ответ без формы AppError -> UnknownError) воспроизводится на уровне apiCall, поэтому ловится связкой client.ts + S7/S9; отдельного исполняющего теста apiCall нет (см. долг).

## Мутационная проверка (selftest exit 0, 8 мутантов)
- «любой HTTP_* сетевой» (якорь `return RETRYABLE_CODES.has(code) || (e instanceof TypeError && ...)`): красный по S7.
- «программный TypeError сетевой» (`e instanceof TypeError && FETCH_FAILURE_RE.test(e.message)` -> `e instanceof TypeError`): красный по S9.
- Прежние 6 мутантов остаются красными.

## Deviations from Plan
**1. [Rule 1] Мутант из плана не годился.** Возврат к `code === 'UnknownError'` не краснит S7 (у HTTP_422 есть код). Заменён на два мутанта выше, которые действительно роняют S7 и S9.

## Verify
`check-send-retry.mjs --selftest`, `check-send-retry.mjs`, `check-transport-hooks.mjs`, `check-transport-tauri.mjs` — exit 0; prettier чист. Полный lint/svelte-check — план 43-21.

## Known Stubs
Нет.

## Долг
Нет исполняющего теста самого apiCall на синтетические коды (HTTP_<status>/NETWORK_ERROR): ветка проверена чтением и косвенно hooks-гейтом.

## Commits
- 6914f3ee: test(43-15): кейсы S7-S10 (RED)
- 9a4b7cde: fix(43-15): автоповторы только для сетевого сбоя и 502/503/504 (WR-07)

## Self-Check: PASSED
