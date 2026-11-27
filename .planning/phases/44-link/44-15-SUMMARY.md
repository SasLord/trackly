---
phase: 44-link
plan: 15
subsystem: link-client
tags: [link-app, connection, reconnect, resync, revoked]
requires: [44-07, 44-08, 44-12, 44-14]
provides:
  - connection::reduce / Machine / ReduceCtx / Input (чистый reducer)
  - connection::EventSink, EventMeta
  - connection::Transport, Connection, RealTransport
  - connection::ConnectionSupervisor, SupervisorConfig, SupervisorHandle, Command
affects: [44-16, 44-17, 44-20]
requirements: [LNK-07, LNK-09]
key-files:
  modified:
    - crates/link-app/src/connection.rs
  created:
    - crates/link-app/tests/connection_loop.rs
decisions:
  - "reduce принимает &mut ReduceCtx {server, offline_since, last_user, offline_delay} вместо &mut Option<Instant>: Offline не хранит пользователя, а server нужен каждому состоянию."
  - "Старт без сети идёт не через AuthDone(Offline), а через ConnectionLost{at: начало попытки}: состояние остаётся Starting, пока не пройдёт порог 5 с (D-22/D-25); AuthDone(Offline) в reduce остаётся прямым переходом (Test 4 плана)."
  - "Transport абстрагирует и вход (authenticate/revoke/logout), и WS (connect): AuthFlow берёт конкретный LinkHttp, без этого Test 5/6/7 на моке были бы невозможны."
  - "Команды: RetryNow, RecheckPin, Logout, Restart, External(AuthResult), Stop. External нужна планам 16/17: после password_login/retry_sso результат отдаётся автомату, и он запускает WS."
  - "Пауза backoff сбрасывается на первом Hello от сервера, а не на самом факте подключения: сервер, закрывающий сразу после апгрейда, не превращается в шторм раз в секунду."
---

# Phase 44 Plan 15: автомат связи Link Summary

Один владелец состояния: `Machine` применяет чистую `reduce` и ровно один раз сообщает `EventSink` о каждой смене; `ConnectionSupervisor` (tokio-задача) ведёт старт, вход, WS-цикл, backoff 1..30 с с джиттером, сброс по пробуждению ПК и «Повторить сейчас», тихий ресинк и отзыв по Close 4401.

## Коммиты

- 77aad55c: reducer, Machine, EventSink (7 тестов)
- 8c06176d: ConnectionSupervisor, Transport/RealTransport (+10 тестов на виртуальном времени)
- 850b1f7c: интеграция с реальным TLS+WS моком (4 теста)

## Гейты

| Гейт | Результат |
|------|-----------|
| `cargo test -p link-app --test connection_loop` | 21/21 |
| `cargo test -p link-app` целиком | 45 + 2 + 17 + 21 + 7 + 17 + 5, 0 упавших |
| `cargo clippy -p link-app --all-targets -- -D warnings` | чисто |
| `cargo fmt --check -p link-app` | чисто |
| `cargo tree -p link-app -i aws-lc-rs` | пакета нет в графе link-app |
| мутация: удалён `t.revoke().await;` в ветке Closed{revoked:true} | краснеют `loop_revoked_close_wipes_token_and_stops_cycle` и `integration_close_4401_revokes_and_stops`; исходник восстановлен |

## Мутационные якоря для плана 20

- Порог offline: единственная строка `if now.saturating_duration_since(since) >= ctx.offline_delay =>` в `reduce` (ветка `Input::Tick`). Снятие/ослабление -> красные `reduce_offline_only_after_threshold`, `loop_startup_without_network_is_quiet_offline_then_connects`.
- Ветка revoked: `t.revoke().await;` + `self.machine.apply(Input::RevokedSignal);` в `online()` при `Closed { revoked: true }`. Проверено мутацией (см. выше).
- Однократная повторная аутентификация: `if self.reauth_used {` в `reauth`. Снятие -> бесконечный цикл 401/вход (тест `loop_unauthorized_upgrade_reauths_once` проверяет 2 входа и 2 подключения при одном 401).
- Сброс паузы по сну: `self.backoff.reset();` внутри `on_tick` при `WakeSignal::Woke`. Тест `loop_sleep_jump_resets_backoff_and_retries_at_once`.
- Ресинк только после потери: `if self.had_loss {` в `online()`. Тесты `loop_resync_*`.

## Deviations from Plan

**1. [Rule 3] Сигнатура reduce** изменена на `reduce(state, &mut ReduceCtx, input)` (см. decisions): иначе `Offline -> Connected` (Test 3) и `server` недостижимы.

**2. [Rule 3] Джиттер без крейта `rand`.** План упоминает rand, но в link-app его нет, а новые крейты запрещены. Источник джиттера по умолчанию — `RandomState` (засеян ОС), подменяется полем `SupervisorConfig.jitter`.

**3. [Rule 3] Порог offline без `should_show_offline`.** В reduce сравнение с `ctx.offline_delay` (по умолчанию `OFFLINE_INDICATION_DELAY` = 5 с), чтобы интеграция могла ускорять порог через конфиг (Task 3 плана). Константа и `should_show_offline` остались в backoff.rs.

**4. Расширение Command** (Restart, External, Logout) сверх списка плана: без них планы 16/17 не смогли бы вернуть автомат в работу после явного входа.

**5. Тест сна** (Test 2) проверяет сброс по поведению (попытки на 8/9/11 с вместо 15 с), а не по точному моменту: тик 1 с и момент скачка делают точное совпадение хрупким.

## Заметка про «connecting» в шапке (из 44-13)

Контракт этого плана не добавляет промежуточное состояние `connecting` в `LinkState` (золотой тест `state_contract.rs` и UI-SPEC §6.1 его не содержат), поэтому серая точка шапки для фазы «идёт переподключение до 5 с» НЕ подключена. Сейчас до порога состояние остаётся `Connected` (при обрыве) либо `Starting` (при старте): пользователь видит прежний экран без плашки, как и требует D-22. Если нужна серая точка в этом окне, это правка контракта `LinkState`/UI-SPEC (план 16 или 17).

## Не проверено / для следующих планов

- `RealTransport` и `EventSink` реального приёмника (emit `link-event` / `trackly-event` / `trackly-resync`, трей) подключает план 17; здесь только `FakeSink`.
- Реальный сон ПК, смена сети и поведение WS-сторожа на живом сервере не проверялись (только скачок настенных часов в моке).
- Команды `External`/`Restart` в `RealTransport` не используют общий `AuthFlow` с командами UI: план 17 должен разделять `LinkHttp`/`TokenStore`/`LocalState` через те же `Arc`, что и `RealTransport`.
- Токены и куки автомат не логирует (в connection.rs нет вызовов tracing).

## Авто-одобренные чекпоинты

Нет (план автономный). Для человека на стенде после плана 17: запустить Link, на 10-20 с отключить сеть (серая иконка и плашка появляются не раньше чем через ~5 с, короткий обрыв до 5 с проходит молча), вернуть сеть (подключение за несколько секунд, данные чата обновились без уведомлений); усыпить ПК и разбудить (подключение сразу, без ожидания паузы); отозвать вход в админке (окно мгновенно уходит в «отключён», автовход не повторяется).

## Known Stubs

Нет.

## Threat Flags

Новых поверхностей вне threat_model нет (T-44-55: backoff до 30 с с джиттером, Test 1; T-44-56: каждое подключение по пину, интеграционный Test 3; T-44-57: RevokedSignal + остановка цикла; T-44-59: D-25, тест старта без сети).

## Self-Check: PASSED

Файлы connection.rs и tests/connection_loop.rs существуют; коммиты 77aad55c, 8c06176d, 850b1f7c в истории.
