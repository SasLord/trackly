---
phase: 43-reliable-event-delivery
plan: 14
subsystem: ws-transport
tags: [gap-closure, websocket, reconnect, probe, CR-03, IN-06]
requires: []
provides:
  - "проба живости и reconnectTimer не переживают onclose (CR-03)"
  - "обработчики сокета привязаны к своему сокету, второй сокет не создаётся"
  - "предупреждение при hello.protocol != 1 (IN-06)"
affects: [43-21]
key-files:
  modified:
    - packages/transport/src/ws.ts
    - ui/scripts/check-ws-resync.mjs
    - ui/scripts/_rune-harness.mjs
key-decisions:
  - "Защита эшелонирована из пяти слоёв; мутанты строятся по классам, одиночные снятия (i)-(iv) подтверждены как эквиваленты"
requirements-completed: [DLV-04]
completed: 2026-11-27
---

# Phase 43 Plan 14: CR-03 двойной сокет после onclose во время пробы Summary

Проба живости и таймер переподключения больше не переживают потерю сокета: двойной сокет и дубли WsEvent (класс GAP-12-10) невоспроизводимы в исполняющем гейте.

## Что сделано
- **ws.ts (CR-03, D-15):** `handleLost` первым делом зовёт `clearProbe()` и гасит прежний `reconnectTimer`; `connectBrowser` не открывает сокет, если `ws !== null`; onmessage/onopen/onclose привязаны к локальному `sock` (`if (ws !== sock) return`); проба запоминает `probed = ws` и в колбэке выходит, если сокет сменился.
- **IN-06:** именованная `WS_PROTOCOL_VERSION = 1`; hello с другой версией даёт один `console.warn`, кадр не диспатчится и ресинка нет.
- **check-ws-resync.mjs:** сценарии R12 (onclose на 4500 мс пробы, один живой сокет и один вызов обработчика на кадр), R13 (проба не убивает новый CONNECTING-сокет), R12b (запоздалый onclose замещённого сокета не обнуляет ws), расширение R2 (hello protocol 1/2).
- **Инварианты тоста D-14:** check-ws-disconnect-toast.mjs не потребовал правок; selftest и боевой прогон зелёные (20 мутантов M1-M19 ловятся).

## Доказательство RED (коммит dd696932, на прежнем ws.ts)
```
FAIL — [R2] hello protocol 2: предупреждений 0, нужно ровно 1
FAIL — [R12] открыто сокетов 2, нужен ровно 1 (двойной сокет)
FAIL — [R12] ... обработчик вызван 3 раз, нужен 1 (дубли)
FAIL — [R13] проба убила сокет, созданный переподключением
FAIL — [R13] число сокетов выросло до 3, нужно 2
FAIL — [R12b] запоздалый onclose породил второй сокет (3)
FAIL — [R12b] после запоздалого onclose ping не ушёл в живой сокет
```

## Мутационная проверка (selftest, exit 0)
Слои: (i) `clearProbe()` в handleLost, (ii) `ws !== probed`, (iii) `clearTimeout(reconnectTimer)` в handleLost, (iv) `if (ws !== null) return` в connectBrowser, (v) привязка onclose к сокету.
- MA = снять (i)+(ii)+(iii)+(iv): красный по R12 и R13.
- MB = снять (i)+(ii): красный по R13.
- MC = onclose без привязки (слой v): красный по R12b.
- MD = убрать проверку protocol: красный по R2.
- Документированные эквиваленты (selftest подтверждает, что результат НЕ меняется): одиночное снятие (i) закрыто (ii); (ii) закрыто (i); (iii) закрыто (v); (iv) закрыто (iii).
- Якоря уникальны: стенд падает, если якорь встречается не ровно один раз.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Поддержка `equivalent` в _rune-harness.mjs**
- **Found during:** Задача 2
- **Issue:** gateMain требовал обнаружения каждого мутанта и не умел проверять эквиваленты («результат не изменился»); план требует именно подтверждения эквивалентности.
- **Fix:** обратно совместимый флаг `equivalent: true, reason` у мутанта: ожидается ноль нарушений. Прочие гейты не затронуты.
- **Files modified:** ui/scripts/_rune-harness.mjs
- **Commit:** b2d4dfd9

**2. R12: onclose на 4500 мс, а не 3000 мс.** Open сбрасывает backoff до 1000 мс, разогнать его при открытом сокете нельзя; окно «reconnectTimer ещё висит при срабатывании пробы» воспроизводится onclose на 4500 мс (таймер на 5500, проба на 5000). Сценарий с 3000 мс вынесен в R13.

## Verify
`node ui/scripts/check-ws-resync.mjs --selftest`, `node ui/scripts/check-ws-resync.mjs`, `node ui/scripts/check-ws-disconnect-toast.mjs --selftest`, `node ui/scripts/check-ws-disconnect-toast.mjs` — все exit 0; prettier по изменённым скриптам чист. svelte-check и lint целиком выполняет план 43-21.

## Commits
- dd696932: test(43-14): сценарии R12, R13, R12b и IN-06 (RED)
- b2d4dfd9: fix(43-14): проба живости и reconnectTimer не переживают onclose (CR-03, IN-06)

## Self-Check: PASSED
