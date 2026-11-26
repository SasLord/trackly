---
phase: 42-link-monorepo-foundation
reviewed: 2026-11-26T00:00:00Z
depth: standard
files_reviewed: 106
files_reviewed_list:
  - .github/workflows/ci-fast.yml
  - .github/workflows/ci-full.yml
  - .github/workflows/release-link.yml
  - .github/workflows/release.yml
  - crates/link-app/Cargo.toml
  - crates/link-app/capabilities/main.json
  - crates/link-app/src/main.rs
  - crates/link-app/tauri.conf.json
  - link-ui/scripts/check-link-layout.mjs
  - link-ui/src/main.ts
  - link-ui/src/lib/linkInfo.ts
  - packages/attachments/src/host.ts
  - packages/attachments/src/AttachmentsBlock.svelte
  - packages/attachments/src/AttachmentHistory.svelte
  - packages/transport/src/client.ts
  - packages/transport/src/ws.ts
  - scripts/check-release-isolation.mjs
  - scripts/check-privacy.mjs
  - ui/scripts/_eslint-workspaces.mjs
  - ui/scripts/_roots.mjs
  - ui/scripts/check-gate-coverage.mjs
  - ui/scripts/check-transport-hooks.mjs
  - ui/scripts/check-workspace-boundaries.mjs
  - ui/scripts/fixtures/gate-coverage.baseline.json
  - ui/src/main.ts
  - ui/src/packages-compat.ts
  - "(остальные из списка 106 файлов просмотрены выборочно: переименования и правки импортов)"
findings:
  critical: 0
  warning: 4
  info: 4
  total: 8
status: issues_found
---

# Phase 42: Code Review Report

**Reviewed:** 2026-11-26
**Depth:** standard
**Files Reviewed:** 106 (подробно разобраны файлы из списка выше)
**Status:** issues_found

## Summary

Разобраны транспорт, хосты, каркас Link, оба release-workflow и новые гейты. Критичных дефектов нет. Инъекции версии в `release-link.yml` нет: ввод идёт через `env`, версия проходит regex. `release.yml` Link не затрагивает, изоляция тегов корректна (`v*.*.*` против `link-v*.*.*`). Права `contents: read`. Capabilities Link минимальны (`core:default`, `window-state:default`). Данных организации и людей в просмотренных файлах не найдено. `window-state` пишет в `%APPDATA%` — это допустимо: Link ставится на пользователя (NSIS, D-04/D-09), а не portable.

Главное замечание: гейт каркаса Link не подключён ни к одному CI-шагу. Остальное — слабые места гейтов и одна гонка в WS-клиенте, перенесённая как есть.

## Warnings

### WR-01: `check-link-layout` (lint link-ui) нигде не запускается в CI и не входит в `ui` lint

**File:** `.github/workflows/ci-fast.yml:151-158`, `link-ui/package.json` (скрипт `lint`), `ui/package.json:16`
**Issue:** В `link-ui/package.json` есть `lint`, который гоняет `check-link-layout.mjs --selftest` и сам гейт (размеры окна, `visible:false`, права `capabilities/main.json` ровно `core:default` + `window-state:default`, запрет `fetch`/`WebSocket`/`{@html}` в `link-ui/src`). `grep` по `ui/package.json` и `.github/workflows/*.yml` даёт 0 вхождений `check-link-layout`. В CI идут только `build` и `svelte-check` link-ui. Расширение прав Link или сеть в webview пройдут CI зелёными. Комментарий в `ci-fast.yml` («link-ui не имеет своего lint-скрипта») неверен. Гейт запускался вручную в 42-19 и поэтому «PASS» в сводке, но в CI он не защищает. Мета-гейт `check-lint-selftest-wiring` смотрит только `ui/scripts`, этот пробел не ловит.
**Fix:** Добавить шаг в ci-fast (и ci-full рядом с `pnpm lint`):
```yaml
      - name: link-ui lint
        run: pnpm --dir link-ui lint
```
И поправить комментарий. Лучше также расширить `check-lint-selftest-wiring` на `link-ui/scripts` либо вызвать `check-link-layout` из `ui` lint.

### WR-02: гонка в `connectWs` на Tauri-ветке: утечка `listen`-подписки и дубли событий

**File:** `packages/transport/src/ws.ts:228-246` (и release на 267-281)
**Issue:** Первый потребитель делает `await listen(...)`, а `activeCleanup` присваивается только после. Если за это время потребитель вызвал `release()`, то `refCount` уже 0, `activeCleanup === null`, очистка не выполнится. Когда `listen` завершится, `activeCleanup = unlisten` присвоится «в никуда», подписка останется жить. Следующий `connectWs()` (refCount 0→1) откроет вторую подписку, и каждое событие придёт дважды (класс GAP-12-10, которым этот код как раз закрывался). Для браузерной ветки гонки нет: `connectBrowser()` синхронный. Правка при переносе не вносилась, но код теперь общий пакет, и Link унаследует дефект.
**Fix:** После `await listen` проверять, не освобождён ли счётчик:
```ts
const unlisten = await listen<WsEvent>('trackly-event', (e) => dispatch(e.payload));
if (refCount === 0) { unlisten(); return () => {}; }
activeCleanup = unlisten;
```
(и подумать о втором `connectWs()` во время `await`: он видит `refCount === 2` и не ждёт готовности).

### WR-03: Tauri-ветка `apiCall` и её реакции на 401/403 не покрыты гейтом и расходятся с HTTP-веткой

**File:** `packages/transport/src/client.ts:54-69`, `ui/scripts/check-transport-hooks.mjs:6-9`
**Issue:** HTTP-ветка определяет отказ по статусу, Tauri-ветка по строковому `code` (`'UNAUTHORIZED' | 'Unauthorized'`, `'FORBIDDEN' | 'Forbidden'`). Регистр и набор значений заданы вручную и ничем не сверены с Rust-типом `AppError`. Гейт `check-transport-hooks` честно пишет, что Tauri-ветку не исполняет (UNVERIFIED), а 42-HUMAN-UAT п.5 откладывает проверку. Итог: если сериализация кода в Rust отличается от двух вариантов, на десктопе хуки молча не сработают (сессия не сбросится, тост 403 не появится), и ни один гейт не покраснеет. T5 при этом проверяет только регистрацию в `main.ts`.
**Fix:** Свести сравнение к одному нормализованному виду и добавить в гейт текстовое/модульное правило на Tauri-ветку, например `err.code.toUpperCase()`. Звено «код из Rust → строка» зафиксировать тестом в `trackly-app`, который сверяет сериализованные значения `AppError::Unauthorized/Forbidden` с этими строками.

### WR-04: `_eslint-workspaces` молча пропускает `link-ui`, если `src/` исчезнет

**File:** `ui/scripts/_eslint-workspaces.mjs:21-34`
**Issue:** Для `link-ui` `required=false`: при отсутствии `src/` печатается «пропуск» и код выхода 0. Комментарий объясняет это тем, что `link-ui/src` «появится в плане 16». Каталог уже есть, но послабление осталось, поэтому удаление/переименование `link-ui/src` даст зелёный eslint без единого файла (вакуумный проход, который в этом же фазе сами гейты запрещают). `check-workspace-boundaries` `[B4]` аналогично только печатает «отсутствует (до плана 16)».
**Fix:** Сделать `['link-ui', true]` и в `[B4]` отсутствие `link-ui/src` считать нарушением; убрать упоминания «до плана 16».

## Info

### IN-01: `[C3]` в `check-gate-coverage` сравнивает только суммарное число файлов

**File:** `ui/scripts/check-gate-coverage.mjs:55-58`
**Issue:** `scanCount < scanned` не отличает потерю одного файла от добавления другого. `[C1]` спасает для файлов из baseline, но новые файлы-замены и файлы вне baseline (пакеты, `link-ui`) в счёт не попадают и не замораживаются отдельно. Допустимо, но «frozen counters» стоит пересматривать осознанным обновлением baseline, а не молча.
**Fix:** Хранить в baseline счётчик по корням (`ui/src`, `packages/*/src`, `link-ui/src`) и сверять каждый.

### IN-02: регистрация `setAttachmentsHost` не защищена гейтом

**File:** `ui/src/main.ts:22-27`, `packages/attachments/src/host.ts:15`
**Issue:** Хост по умолчанию возвращает `null` — «минимум прав», это правильно (fail-closed): без регистрации пропадут кнопки удаления своих вложений и корзина админа, но не появится лишних прав. Но удаление `setAttachmentsHost(...)` из `main.ts` не краснит ничего, в отличие от `setApiHooks` (правило T5). Права на сервере проверяются независимо, поэтому риск ограничен регрессией UI.
**Fix:** Добавить правило вида T5 в `check-attachments-block` (регистрация до `mount`, `role` и `id` берутся из `authStore.user`).

### IN-03: `main.rs` глушит ошибки показа окна

**File:** `crates/link-app/src/main.rs:49-52`
**Issue:** `let _ = w.show(); let _ = w.set_focus();`. Окно создано с `visible:false`; если `show()` вернёт ошибку, приложение запустится невидимым без единого сообщения в лог. Вероятность низкая, но отладка пилота на доменных ПК (D-06) будет слепой.
**Fix:** Логировать ошибку (`eprintln!` или `tracing`), либо пропагировать из `setup`: `w.show()?;`.

### IN-04: CSP Link допускает `style-src 'unsafe-inline'`

**File:** `crates/link-app/tauri.conf.json:25`
**Issue:** `script-src 'self'` и `connect-src` ограничены, это хорошо. `'unsafe-inline'` для стилей нужен Svelte-компонентам с динамическими стилями; вектор слабый (нет скриптов и `{@html}`, это держит L4), но при расширении Link (чат, Фаза 45+) стоит вернуться к вопросу.
**Fix:** Зафиксировать в ADR/комментарии причину исключения и пересмотреть при появлении пользовательского контента.

---

_Reviewed: 2026-11-26_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
