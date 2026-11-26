---
phase: 42-link-monorepo-foundation
plan: 19
subsystem: verification
tags: [gates, stand, dry-run, human-uat, LNK-01, LNK-10, DST-06]
requires: [42-01, 42-02, 42-03, 42-04, 42-07, 42-11, 42-12, 42-13, 42-14, 42-15, 42-16, 42-17, 42-18]
provides:
  - 42-STAND-RESULT.md (стенд D-21 «после» против baseline, dry-run release-link)
  - 42-HUMAN-UAT.md (шесть отложенных пунктов)
key-files:
  created:
    - .planning/phases/42-link-monorepo-foundation/42-STAND-RESULT.md
    - .planning/phases/42-link-monorepo-foundation/42-HUMAN-UAT.md
  modified:
    - .github/workflows/ (pnpm/action-setup без version: 10)
    - crates/link-app/tauri.conf.json (beforeDev/BuildCommand)
decisions:
  - "Пилот DST-06 вынесен в human-UAT (D-07): не блокирует закрытие Фазы 42, блокирует планирование Фазы 45"
metrics:
  tasks: 3
  completed: 2026-11-26
---

# Phase 42 Plan 19: сплошной прогон, стенд «после», dry-run, human-UAT Summary

Фаза замкнута: все гейты и сплошной `cargo test --workspace` зелёные после переезда, стенд D-21 не показал новых ошибок консоли, dry-run выпуска Link прошёл без Release. Пилот и три непроверенных машинно сценария оформлены как отложенная human-UAT.

## Задача 1: гейты на границе волны

Каждая команда отдельно, код выхода 0:

| команда | результат |
|---------|-----------|
| `pnpm install --frozen-lockfile` | 0 |
| `pnpm --dir ui build` | 0 |
| `pnpm --dir ui svelte-check` | 0; 485 файлов, 0 ошибок, 0 предупреждений |
| `pnpm --dir ui lint` | 0; 75 гейтов подключены |
| `pnpm --dir link-ui build` | 0 |
| `pnpm --dir link-ui svelte-check` | 0; 89 файлов, 0 ошибок |
| `pnpm --dir link-ui lint` | 0; check-link-layout PASS (33 признака) |
| `check-release-isolation.mjs --selftest` и без флага | 0, PASS |
| `check-privacy.selftest.mjs` | 0, PASS |
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 |
| `cargo test --workspace --no-fail-fast` (один раз, фоном, `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1`) | 0; 31 бинарник: passed 3083, failed 0, ignored 7 |

Инварианты: `check-gate-coverage` PASS (316 файлов, 61 запись move-map, 18 обходчиков); `Button.svelte` ровно один; реэкспорт-прокладок нет; `ui/pnpm-lock.yaml` не отслеживается; `window-state` в `THIRD-PARTY-NOTICES.md` встречается 2 раза.

**Ложное срабатывание проверки `version.workspace`.** Команда из плана `grep -v '^#' crates/link-app/Cargo.toml | grep -c "version.workspace"` даёт 1, а не 0: совпадает строка `rust-version.workspace = true`. Версия пакета задана литералом `version = "0.1.0"` (D-10); строгая проверка `grep -E "^version\.workspace"` даёт 0. Дефект в формуле проверки, не в коде.

## Задача 2: стенд D-21 и dry-run (выполнила оркестрация, коммит 914737e5)

- Консоль «после» сравнена с baseline по 14 маршрутам: новых `console.error`, `console.warn` и 404 на ассеты нет. Выход по 401 и повторный вход работают.
- UNVERIFIED: карточка устройства с вложениями (в БД стенда нет устройств); реакция `onForbidden` на 403; десктопная сборка Trackly и ветка Tauri-invoke (допущение D-21). Все вынесены в `42-HUMAN-UAT.md`.
- Dry-run `release-link.yml` прошёл с третьей попытки (коммит fc6b56e1); артефакт `trackly-link-0.1.0-nsis`; нового Release нет, `release.yml` не запускался.

## Задача 3: human-UAT, ROADMAP, аудит покрытия

`42-HUMAN-UAT.md`: шесть пунктов (пилот DST-06; окно Link и запоминание положения; CSP/IPC; путь NSIS; десктопная сборка Trackly; карточка с вложениями и 403 в LAN-браузере на реальных данных), у каждого указаны компонент или файл и условие воспроизведения. ROADMAP: секция Phase 42 содержит 19 планов с пометкой о несовпадении номера плана и волны; прогресс обновлён через `roadmap.update-plan-progress`.

### Покрытие решений (цитирование в must_haves.truths)

| решение | планы | решение | планы |
|---------|-------|---------|-------|
| D-01 | 04, 10, 18, 19 | D-12 | 10, 16 |
| D-02 | 04 | D-13 | 11, 16 |
| D-03 | 04 | D-14 | 10, 16 |
| D-04 | 03, 04 | D-15 | 16 |
| D-05 | 04 | D-16 | 10, 16 |
| D-06 | 04 | D-17 | 03, 08, 09, 11, 12, 13, 14, 15, 17 |
| D-07 | 04, 19 | D-18 | 01, 08, 11, 16 |
| D-08 | 10, 16 | D-19 | 03, 17 |
| D-09 | 03, 10 | D-20 | 01, 02, 05-09, 11-19 |
| D-10 | 03, 10, 18 | D-21 | 02, 19 |
| D-11 | 03, 08, 18, 19 | D-22 | 03, 19 |

Пропусков нет: каждое из D-01..D-22 процитировано хотя бы в одном плане.

### Покрытие требований

| требование | планы | статус |
|------------|-------|--------|
| LNK-01 (Link отдельным приложением со своей версией) | 08, 10, 16, 18, 19 | машинно подтверждено (сборка, dry-run) |
| LNK-10 (общие пакеты, компонент пишется один раз) | 09, 11-17 | машинно подтверждено (гейты границ, один `Button.svelte`) |
| DST-06 (пилот на доменных ПК) | 04, 19 | набор и ADR готовы; живой пилот отложен (human-UAT п.1, D-07) |

REQUIREMENTS.md не изменялся по указанию оркестратора.

## Deviations from Plan

**1. [Rule 3 - Blocking] pnpm/action-setup конфликтовал с packageManager** (коммит 4f0a1d99). `version: 10` в шаге `pnpm/action-setup` расходился с `packageManager: pnpm@10.17.1` корневого `package.json` монорепо (ERR_PNPM_BAD_PM_VERSION); падали ci-fast, ci-full и release-link. Пин убран во всех workflow.

**2. [Rule 1 - Bug] Пути beforeDev/BuildCommand у link-app** (коммит fc6b56e1). Tauri запускает эти команды из `crates/`, путь `../../link-ui` уходил за пределы репозитория; теперь `../link-ui`. Обнаружено на второй попытке dry-run.

**3. Ложное срабатывание проверки `version.workspace`** (см. выше): дефект формулы в плане, код не менялся.

## Known Stubs

Нет.

## Self-Check: PASSED

Файлы 42-STAND-RESULT.md и 42-HUMAN-UAT.md существуют; коммиты 4f0a1d99, fc6b56e1, 914737e5 присутствуют в истории.
