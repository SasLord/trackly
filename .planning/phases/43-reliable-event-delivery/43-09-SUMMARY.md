---
phase: 43-reliable-event-delivery
plan: 09
subsystem: ui-transport
tags: [idempotency, retry, requests, gates]
requires:
  - phase: 43-06
    provides: "серверная идемпотентность requests_create по client_request_id"
  - phase: 43-07
    provides: "реестровый гейт ресинка в pnpm lint"
provides:
  - "packages/transport/src/sendWithRetry.ts: sendWithRetry, newClientId, isNetworkFailure, RETRY_DELAYS_MS"
  - "check-send-retry.mjs (исполняющий, 6 мутантов) и check-request-create-idempotency.mjs (структурный, 5 мутантов) в pnpm lint"
affects: [43-10, 43-12]
key-files:
  created:
    - packages/transport/src/sendWithRetry.ts
    - ui/scripts/check-send-retry.mjs
    - ui/scripts/check-request-create-idempotency.mjs
  modified:
    - ui/src/features/requests/RequestFormModal.svelte
    - ui/src/bindings-phase6.ts
    - ui/package.json
key-decisions:
  - "D-12: повтор только при значении без формы AppError (сетевой сбой); любой AppError с code пробрасывается сразу"
  - "D-12: clientRequestId создаётся при открытии формы (в блоке сброса) и не в handleSubmit; после успеха форма закрывается, следующий показ даёт новый id"
  - "ветка repairs.create не менялась (вне D-11)"
requirements-completed: [DLV-03]
completed: 2026-11-27
---

# Phase 43 Plan 09: клиентский контракт повторной отправки Summary

Помощник автоповторов с тем же замыканием (паузы 1/2/4 с, доменные отказы не повторяются), генерация id с запасными путями без `crypto.randomUUID`, применение в форме создания заявки.

## Задачи

1. `sendWithRetry` / `newClientId` + исполняющий гейт `check-send-retry.mjs` — `65941279`
2. Форма заявки: `clientRequestId` + `sendWithRetry(() => requests.create(...))` + структурный гейт — `734ed34c`

## Проверки

- `check-send-retry --selftest`: 6 мутантов пойманы (M2 доменный повтор — кейсом S4 «FORBIDDEN -> 1 вызов»); боевой OK.
- `check-request-create-idempotency --selftest`: 5 мутантов пойманы; боевой OK.
- `pnpm --dir ui lint` exit 0 (81 гейт в wiring), `svelte-check` 0/0, `pnpm build` OK, `check-gate-coverage` PASS, privacy PASS.

## Отклонения

1. [Rule 3] Тип `RequestCreateDto` в форме берётся из рукописного, отслеживаемого `ui/src/bindings-phase6.ts`, а не из сгенерированного `bindings.ts` (в gitignore). План 06 поле туда не добавил, svelte-check падал; добавлено `clientRequestId?: string | null`.
2. [Rule 1] eslint `no-undef` на типе `Crypto`: заменён локальным интерфейсом `CryptoLike`.
3. Мутант M6 плана (новая генерация id в каждом повторе) заменён мутантом «id не уникален» (S6): помощник сам id не создаёт, удержание id замыканием проверяет кейс S2, а размещение генерации в форме — структурный гейт [I4].
4. Мутант M4 структурного гейта сначала не краснел (перенос объявления вплотную перед handleSubmit оставался «до»); якорь перенесён в конец скрипта.

## Ограничение

Структурный гейт не доказывает поведение «одно нажатие после сбоя — один результат»; живая проверка — в плане 12.

## Known Stubs

Нет.

## Self-Check: PASSED
