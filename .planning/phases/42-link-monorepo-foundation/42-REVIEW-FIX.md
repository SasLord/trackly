---
phase: 42-link-monorepo-foundation
fixed_at: 2026-11-26T00:00:00Z
review_path: .planning/phases/42-link-monorepo-foundation/42-REVIEW.md
iteration: 1
findings_in_scope: 4
fixed: 4
skipped: 0
status: all_fixed
---

# Phase 42: Code Review Fix Report

**Fixed at:** 2026-11-26
**Source review:** .planning/phases/42-link-monorepo-foundation/42-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 4
- Fixed: 4
- Skipped: 0

## Fixed Issues

### WR-01: `check-link-layout` не запускался в CI

**Files modified:** `.github/workflows/ci-fast.yml`, `.github/workflows/ci-full.yml`, `ui/scripts/check-workspace-boundaries.mjs`
**Commit:** a9954ab7
**Applied fix:** в обоих workflow добавлен шаг `run: pnpm --dir link-ui lint` (реальная команда, она зовёт `check-link-layout.mjs --selftest` и сам гейт); неверный комментарий в ci-fast исправлен. Подключение держит новое правило `[B6]` в `check-workspace-boundaries` (скрипт `lint` link-ui зовёт `check-link-layout.mjs`; оба workflow содержат шаг). Selftest получил 3 мутанта (снят шаг в ci-fast, в ci-full, скрипт lint без check-link-layout) — все краснеют `[B6]`. Доказательство: `grep` даёт `ci-fast.yml:156` и `ci-full.yml:156`.

### WR-02: гонка `listen` в `connectWs` (Tauri-ветка)

**Files modified:** `packages/transport/src/ws.ts`
**Commit:** dbdf0a1f
**Applied fix:** введён счётчик поколений `epoch`, который растёт в новой функции `teardown()` (её зовут `release()` последнего потребителя и `disconnectWs()`). Если после `await listen` эпоха сменилась, подписка снимается сразу (`unlisten()`), а не пишется в `activeCleanup`. Ветка отказа `listen` не трогает чужой счётчик при смене эпохи. Обычный release() после готовности `listen` и общий для двух потребителей путь не изменились.
**Доказательство (красный/зелёный):** новый гейт `check-transport-tauri` (см. WR-03), кейсы W1-W4. На прежнем `ws.ts` краснеет: `[W1] после гонки осталось подписок: 1 (утечка listen)`, `[W2] подписок 2, доставок 2: ждали 1/1`; на исправленном — OK. Мутанты selftest (снятая проверка эпохи, снос без сдвига эпохи, release не снимает подписку, отказ listen не возвращает счётчик) пойманы.
**Оговорка:** вторая половина замечания («второй `connectWs()` во время `await` не ждёт готовности») не менялась: контракт возвращаемого release() прежний. Реально достижимый путь гонки через публичный API — `disconnectWs()` во время ожидания; он и покрыт.

### WR-03: Tauri-ветка `apiCall` и 401/403 не покрыты гейтом

**Files modified:** `packages/transport/src/client.ts`, `ui/scripts/check-transport-tauri.mjs` (новый), `ui/scripts/check-transport-hooks.mjs` (комментарий), `ui/package.json`
**Commit:** 3d5cae73
**Applied fix:** определение отказа сведено к одной нормализованной проверке `err.code.toUpperCase()` (`UNAUTHORIZED`/`FORBIDDEN`, регистр не важен). Новый гейт исполняет настоящие `client.ts` и `ws.ts` с `isTauri=true` и подменёнными `invoke`/`listen`: отказы `UNAUTHORIZED`, `Unauthorized`, `unauthorized`, `FORBIDDEN`, `Forbidden`, прочие коды, строка-отказ, `null`, объект без `message`, успех, отсутствие хуков; кроме того, `[C4]` сверяет строки с `crates/trackly-core/src/error.rs` (`Self::Unauthorized => "UNAUTHORIZED"`, `Self::Forbidden => "FORBIDDEN"`), закрывая звено «Rust -> строка» без правки Rust. Гейт подключён в `ui` lint с `--selftest` (мета-гейт wiring: 76 гейтов подключены).
**Доказательство:** на прежнем `client.ts` краснеет `[C2] unauthorized: un=0 fb=0`; на исправленном OK. 9 мутантов selftest (регистр, хук не зовётся, перепутанный хук, хук на любой ошибке, переименованный код в error.rs, 4 мутанта ws.ts) пойманы.
**Оговорка:** сам Rust-тест в `trackly-app`, предложенный ревью, не добавлялся (Rust не трогался); его заменяет сверка исходника `error.rs` в гейте. Статус: исправлено, логика нормализации требует взгляда человека только в части совместимости с реальными ошибками Tauri-команд (UAT п.5 прежний).

### WR-04: вакуумный проход при пропаже `link-ui/src`

**Files modified:** `ui/scripts/_eslint-workspaces.mjs`, `ui/scripts/check-workspace-boundaries.mjs`
**Commits:** c9f47973, 908de30b (убран неиспользуемый `required`, найденный eslint при финальном прогоне)
**Applied fix:** `_eslint-workspaces` теперь падает при отсутствии `src/` у обоих корней; `[B4]` считает отсутствие и пустоту `link-ui/src` нарушением; убраны упоминания «до плана 16»; в PASS печатается число просканированных файлов. Selftest: +2 мутанта (`link-ui/src` исчез, пуст) -> `[B4]`.
**Доказательство:** при временном переносе `link-ui/src` на реальном дереве: `[B4] link-ui/src отсутствует...` (exit 1) и `[eslint-workspaces] link-ui: нет ни одного src/...` (exit 1); после возврата PASS, `link-ui/src: 5 файлов`.

## Финальные гейты (в основном дереве после fast-forward)

| Команда | Результат |
|---|---|
| `pnpm --dir ui lint` | exit 0 (после коммита 908de30b; до него красный из-за неиспользуемой переменной, исправлено) |
| `pnpm --dir ui svelte-check` | 0 ошибок, 0 предупреждений |
| `pnpm --dir ui exec vite build` | built |
| `pnpm --dir link-ui lint` | PASS (33 признака) |
| `pnpm --dir link-ui build` | built |
| `pnpm --dir link-ui svelte-check` | 0 ошибок, 0 предупреждений |

Ни один гейт не ослаблен, замороженные счётчики не менялись (`check-gate-coverage` зелёный). Rust не менялся, cargo не запускался. Публичных данных организации/людей нет.

---

_Fixed: 2026-11-26_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
