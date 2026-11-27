---
phase: 44-link
plan: 10
subsystem: link-ui
tags: [link-ui, svelte, login, setup, pin-mismatch, contract-gate]
requires: [44-05]
provides:
  - linkState.ts (типы LinkState/LinkEvent, screenFor, стор по link-event) и linkApi.ts (обёртки 8 команд)
  - экраны ConnectingScreen, SetupScreen, PinMismatchScreen, LoginScreen (+ AdminPinBlock)
  - гейт check-link-state.mjs в pnpm lint (зеркало golden-фикстур Rust)
affects: [44-12, 44-17, 44-20]
key-files:
  created:
    - link-ui/src/lib/linkState.ts
    - link-ui/src/lib/linkApi.ts
    - link-ui/scripts/check-link-state.mjs
    - link-ui/src/components/ConnectingScreen.svelte
    - link-ui/src/components/SetupScreen.svelte
    - link-ui/src/components/PinMismatchScreen.svelte
    - link-ui/src/components/AdminPinBlock.svelte
    - link-ui/src/components/LoginScreen.svelte
  modified:
    - link-ui/package.json
decisions:
  - "Стор состояния сделан по контракту Svelte-стора (subscribe), а не на рунах: рантайм рун компиляцией не проверяется, а стор-контракт проще и надёжнее"
  - "AdminPinBlock вынесен отдельным компонентом, а не сниппетом внутри PinMismatchScreen: экспорт сниппета между компонентами менее предсказуем"
requirements-completed: [LNK-07, LNK-08]
duration: 25min
completed: 2026-11-28
---

# Phase 44 Plan 10: Экраны Link до входа Summary

Webview Link получил линию данных (типизированный стор `link-event` и обёртки `invoke`), четыре экрана по UI-SPEC §6.2-§6.6 и гейт, который сверяет TS-типы и список команд с golden-фикстурами Rust.

## Коммиты

- 1e8eeb0c: linkState/linkApi и гейт check-link-state
- 4b0c2309: ConnectingScreen, SetupScreen, PinMismatchScreen, AdminPinBlock
- fcbe727c: LoginScreen

## Проверки

- `pnpm --dir link-ui lint`: зелёный (layout-гейт 42 признака + новый гейт 49 признаков; selftest ловит все 7 мутантов S1-S4)
- `pnpm --dir link-ui svelte-check`: 0 ошибок, 0 предупреждений
- `pnpm --dir link-ui build`: проходит
- `grep "Доверять|Запомнить"` по link-ui/src: пусто; `fetch(|XMLHttpRequest|WebSocket` по link-ui/src: пусто
- `node ui/scripts/check-tokens.mjs`: зелёный, но он смотрит только ui/src; link-ui он не покрывает (с `--src=link-ui/src` даёт ложные срабатывания на уже существовавший StatusLine из-за разрешения путей). Новые стили используют только токены, определённые в `_tokens.scss`, литеральных цветов и px нет.

## Отклонения от плана

1. **[Rule 3] Стор без рун.** Вместо `$state`-стора сделан стор по контракту `subscribe` внутри `linkState.ts` (используется как `$linkStore`). Файл `.svelte.ts` не понадобился.
2. **[Rule 3] AdminPinBlock.svelte** отдельный файл (план допускал сниппет внутри PinMismatchScreen). `formatFingerprint` экспортируется из `<script module>`.
3. **[Rule 1] Проп `state` в LoginScreen переименован в `linkState`:** имя конфликтует с руной `$state` (svelte-check: «Cannot use 'state' as a store»).
4. Кнопка «Закрыть Link» стоит внутри карточки `AuthShell` внизу, а не снаружи: `AuthShell` занимает `min-height: 100vh`, внешний элемент вызвал бы прокрутку.
5. `spellcheck="false"` задан обёрткой-div вокруг `Input` (атрибут наследуется), потому что примитив `Input` его не пробрасывает; новых пропсов в общий kit не добавлялось.

## Допущения для следующих планов

- Имена аргументов команд (`code`, `login`, `password`) приняты по смыслу: `commands.rs` ещё заглушка, их зафиксирует план 17. Гейт сверяет только имена команд.
- Сведения для администратора при несовпадении отпечатка во время первой настройки приходят в `SetupScreen` необязательным пропсом `pinDetails`; ошибка команды `pin_mismatch` несёт только текст. Если ядро их не передаст, показывается только danger-callout. План 17 должен решить, как их доставить (например, через состояние).
- Экраны ещё не подключены в `App.svelte` (это план 12), поэтому размер бандла не изменился.

## Авто-одобренные чекпоинты

Нет (план полностью автономный).

## Не проверено

- Рантайм Svelte (экраны, фокус, `$effect`, стор `link-event`) и внешний вид: проверка только компиляционными гейтами, живая проверка в запущенном приложении — план 20. Сценарий для человека: запустить Link без конфига (экран кода, вставка кода с переносами строк), с подменой отпечатка (экран пина, «Скопировать сведения», «Проверить снова»), с токеном, отозванным сервером (danger-callout, нет кнопки автовхода), после «Выйти из учётной записи» на доменном Windows (callout «Войти автоматически»), на Linux без Secret Service (подпись под кнопкой).

## Known Stubs

Нет. Экраны получают данные только пропсами и командами; временных заглушек не добавлено.

## Threat Flags

Новых поверхностей вне threat_model нет. Митигации: T-44-33 (гейт S4/L4 без fetch/WebSocket), T-44-34 (пароль очищается после ошибки, в сторе не хранится), T-44-35 (`login` вызывается только из submit), T-44-36 (нет кнопки принятия сертификата).

## Self-Check: PASSED
