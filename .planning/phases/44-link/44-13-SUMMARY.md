---
phase: 44-link
plan: 13
subsystem: link-ui
tags: [link-ui, svelte, shell, account-menu, offline-banner, layout-gate]
requires: [44-10]
provides:
  - AccountMenu (account-cell в шапке) и OfflineBanner
  - App.svelte: переключатель экранов по screenFor(link-event) и каркас connected/offline
  - гейт check-link-layout: правила L5/L6 и новые мутанты selftest
affects: [44-17, 44-20]
key-files:
  created:
    - link-ui/src/components/AccountMenu.svelte
    - link-ui/src/components/OfflineBanner.svelte
  modified:
    - link-ui/src/App.svelte
    - link-ui/src/components/ConnectingScreen.svelte
    - link-ui/scripts/check-link-layout.mjs
decisions:
  - "ConnectingScreen получил проп showQuit: экран без шапки на Linux без трея иначе не имел бы выхода (UI-SPEC §6.9)"
requirements-completed: [LNK-02, LNK-09, LNK-11]
duration: 20min
completed: 2026-11-28
---

# Phase 44 Plan 13: Каркас подключённого окна Link Summary

Окно Link выбирает экран по состоянию из `link-event` (`screenFor`), в connected/offline рисует каркас Фазы 42 с блоком учётной записи в шапке (56px) и плашкой «Нет связи», а на Linux без трея даёт «Закрыть Link» в меню и на экранах без шапки.

## Коммиты

- 2a75f0e4: AccountMenu и OfflineBanner
- 58cdce14: App.svelte, showQuit в ConnectingScreen, гейт раскладки (L5/L6)

## Проверки

- `pnpm --dir link-ui lint`: зелёный (layout 83 признака, selftest ловит все мутанты, включая новые L5 x3 и L6; state-гейт 49 признаков)
- `pnpm --dir link-ui svelte-check`: 0 ошибок, 0 предупреждений
- `pnpm --dir link-ui build`: проходит
- `git diff -- packages/kit`: пуст (ActionMenu не менялся); «Доверять» в link-ui/src: пусто

## Отклонения от плана

1. **[Rule 2] ConnectingScreen.showQuit.** В плане не назван, но требуется D-20 для экранов без шапки; добавлен проп и кнопка `variant="link"`.
2. Вне плана ничего больше; литералы `16px`/`8px` в App.svelte заменены токенами, как требовал план.

## Авто-одобренные чекпоинты

Нет.

## Не проверено (UNVERIFIED)

Рантайм Svelte (стор `$linkStore`, `$derived`, открытие меню через `openAt`, перенос фокуса при смене экрана), вид шапки при 720px и фокус-кольцо триггера компиляционные гейты не видят. Живая проверка - план 20. Шаги для человека: запустить Link (macOS/Windows) при connected - в шапке ФИО, «способ · сервер», точка; клик по блоку открывает меню с «Выйти из учётной записи»; остановить сервер - через ~5 с под шапкой плашка «Нет связи» и «Повторить сейчас»; на Linux без AppIndicator - в меню и на экранах без шапки есть «Закрыть Link», при трее - нет.
Экран `starting` показывается и вне Tauri (браузерный dev), каркас не падает; задержка 5 с до offline и признак «подключение» (серая точка) определяются Rust-стороной: UI сейчас показывает только connected/offline, состояния `connecting` в `LinkState` нет.

## Known Stubs

Заглушка чата и сайдбара намеренная (Фазы 47-51).

## Threat Flags

Нет новых поверхностей. T-44-49: `tray_available` из Rust управляет «Закрыть Link». T-44-50: гейт L4 сохранён.

## Self-Check: PASSED
