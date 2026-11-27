---
phase: 43-reliable-event-delivery
plan: 17
subsystem: ui-requests
tags: [gap-closure, WR-02, DLV-03]
requires: []
provides:
  - "requestIdPolicy.ts: requestSignature, createRequestIdIssuer"
affects: [43-21]
key-files:
  created:
    - ui/src/features/requests/requestIdPolicy.ts
  modified:
    - ui/src/features/requests/RequestFormModal.svelte
    - ui/scripts/check-request-create-idempotency.mjs
    - .planning/phases/43-reliable-event-delivery/43-CONTEXT.md
key-decisions:
  - "D-12 уточнён: id формы заявки держится, пока тело (тип, принтер, категория, описание) не менялось; правка после сбоя выпускает новый id"
requirements-completed: [DLV-03]
completed: 2026-11-27
---

# Phase 43 Plan 17: WR-02, правка формы после сбоя не теряется Summary

Форма заявки выпускает новый clientRequestId при изменении тела после неудачной попытки, а при неизменённом теле повтор уходит с прежним id.

## Что сделано
- **requestIdPolicy.ts:** чистые `requestSignature` (только поля, реально уходящие в тело для данного типа, описание с trim) и `createRequestIdIssuer(newId)` с `idFor(signature)` и `reset()`.
- **RequestFormModal.svelte:** `let clientRequestId = newClientId()` заменён на `const idIssuer = createRequestIdIssuer(newClientId)`; открытие формы вызывает `idIssuer.reset()`; в handleSubmit `attemptId = idIssuer.idFor(requestSignature(...))` передаётся как `clientRequestId`. `newClientId(` в handleSubmit не вызывается (I4). Ветка repairs.create не тронута.
- **Гейт:** `check-request-create-idempotency.mjs` переведён на `gateMain` (стенд `_rune-harness`): исполняющие кейсы Q1-Q4 над настоящим requestIdPolicy.ts, структурные I1-I5, 10 мутантов (Q1-Q4, I1-I5) красные в `--selftest`.
- **43-CONTEXT.md:** запись об уточнении D-12 (WR-02).

## RED (коммит d337e3f5)
До реализации боевой прогон был красным: I2, I5, Q1, Q2, Q3, Q4.

## Мутационная проверка
`--selftest`: выдатчик без реакции на смену подписи даёт Q2, всегда новый id даёт Q1, reset без сброса даёт Q3, подпись без описания даёт Q4, handleSubmit без idFor( даёт I5. Прогон зелёный: selftest, боевой, check-request-flow, check-request-prefill, check-lint-selftest-wiring, check-gate-coverage, eslint и prettier по затронутым файлам.

## Принятый компромисс
Если исходный запрос реально дошёл до сервера, а пользователь затем изменил форму, создастся вторая заявка с новыми данными: правка означает новое намерение. Без правок дубликат невозможен. Сценарий «ответ потерян после коммита» живой проверкой в 43-21 не подтверждается.

## Deviations from Plan
Гейт переписан на `gateMain` вместо дописывания к собственному selftest: мутанты политики должны применяться к настоящему исходнику через оверрайды стенда. Исправлен якорь мутанта Q4 (первая версия не совпала с исходником). Иных отклонений нет.

## Не выполнено по плану
Полный `pnpm lint` и svelte-check по всему ui отнесены в 43-21 согласно области verify волны.

## Self-Check: PASSED
Коммиты d337e3f5, 27f21aa1, e5b95405 существуют; файлы на месте.
