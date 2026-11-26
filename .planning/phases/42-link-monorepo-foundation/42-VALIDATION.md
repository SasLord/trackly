---
phase: 42
slug: link-monorepo-foundation
status: complete
nyquist_compliant: true
wave_0_complete: true
created: 2026-11-26
---

# Phase 42 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Node-гейты `ui/scripts/check-*.mjs` (+`--selftest`), `svelte-check`, eslint/prettier, `cargo test/clippy/fmt` |
| **Config file** | `ui/package.json` (`lint`, `svelte-check`), `.github/workflows/ci-fast.yml` |
| **Quick run command** | `pnpm --dir ui lint && pnpm --dir ui svelte-check` |
| **Full suite command** | `pnpm build` → `svelte-check` → `pnpm lint` → `cargo fmt --check` → `cargo clippy --workspace --all-targets -- -D warnings` → `cargo test --workspace` (один раз на границе волны, в фоне, один `cargo test` одновременно) |
| **Estimated runtime** | quick ~2–5 мин; full — часы (см. память о сплошном прогоне) |

---

## Sampling Rate

- **After every task commit:** quick run + `--selftest` затронутых гейтов
- **After every plan wave:** build + svelte-check + lint + clippy + fmt; сплошной `cargo test --workspace` на границе волны
- **Before `/gsd-verify-work`:** полный набор + стенд D-21 + dry-run `release-link.yml` зелёные
- **Max feedback latency:** ~300 секунд для quick run

---

## Per-Task Verification Map

Заполняется планировщиком/исполнителем по планам. Базовое соответствие требований проверкам:

| Requirement | Behavior | Test Type | Automated Command | File Exists | Status |
|-------------|----------|-----------|-------------------|-------------|--------|
| LNK-01 | Trackly собирается, гейты зелёные после переезда | gate+build | `pnpm --dir ui build && pnpm --dir ui svelte-check && pnpm --dir ui lint` | ✅ | ✅ green |
| LNK-01 | ни один гейт не потерял покрытие | gate (новый) | `node ui/scripts/check-gate-coverage.mjs --selftest && node ui/scripts/check-gate-coverage.mjs` | ✅ | ✅ green |
| LNK-01 | границы пакетов (пакеты не импортируют `ui/`/bindings; Link не импортирует `ui/`) | gate (новый) | `node ui/scripts/check-workspace-boundaries.mjs --selftest && node ui/scripts/check-workspace-boundaries.mjs` | ✅ | ✅ green |
| LNK-01 | компонент из общего пакета собирается в обоих клиентах | build | `pnpm --dir link-ui build && pnpm --dir ui build` | ✅ | ✅ green |
| LNK-01 | независимая версия Link, изоляция релизов | gate (новый) | `node scripts/check-release-isolation.mjs --selftest && node scripts/check-release-isolation.mjs` | ✅ | ✅ green |
| LNK-01 | UI Trackly не сломан | browser stand (D-21) | `cargo test -p trackly-app --test stand_ui_move -- --ignored` + агентский прогон, консоль без ошибок | ✅ | ✅ green |
| LNK-10 | раскладка каркаса + конфиг окна/window-state | gate (новый) | `node link-ui/scripts/check-link-layout.mjs --selftest && node link-ui/scripts/check-link-layout.mjs` | ✅ | ✅ green |
| DST-06 | набор пилота полон, без приватных данных | static | `node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256` | ✅ | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [x] ведомость покрытия гейтов (baseline ДО переезда) + `check-gate-coverage.mjs` (+selftest, в `lint`)
- [x] `check-workspace-boundaries.mjs` (+selftest, в `lint`)
- [x] `scripts/check-release-isolation.mjs` (+selftest, шаг ci-fast)
- [x] `check-link-layout.mjs` (+selftest)
- [x] снимок консоли браузерного стенда ДО переезда
- [x] спайк source-пакета + SCSS + svelte-check на одном компоненте
- [x] перевод `_rune-harness.mjs` и корней обхода на новые пути

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Окно Link открывается, размер/положение запоминаются | LNK-10 | живой WebView2/WKWebView | чек-лист пилота (окно, повторный запуск) |
| Тихая установка, запуск под AppLocker/SRP, WebView2, RDS | DST-06 | нужен реальный домен | `docs/pilot/` чек-лист; итог обезличенно; иначе отложенная human-UAT (D-07) |
| ADR принят | DST-06 / LNK-01 | решение пользователя | статус Accepted в `docs/adr/*` |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 300s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-11-26

## Validation Audit 2026-11-26

Все автоматические команды карты прогнаны повторно после secure-phase — exit 0:
`check-gate-coverage` (selftest 17 случаев; PASS 316 файлов, 18 обходчиков), `check-workspace-boundaries`
(selftest 18; PASS, 59 файлов пакетов + 5 link-ui/src), `check-release-isolation` (selftest + PASS),
`check-link-layout` (selftest + PASS, 34 признака), `check-privacy` (0 нарушений), `pnpm --dir link-ui build`,
`pnpm --dir ui build`, `svelte-check` (498 файлов, 0 ошибок), `pnpm --dir ui lint` (77 гейтов подключены).
Браузерный стенд D-21 повторно не запускался — результат в `42-STAND-RESULT.md` (план 19), сплошной
cargo-прогон 3083/0 — в `42-19-SUMMARY.md`. Ручные проверки: окно Link и ADR приняты в `42-HUMAN-UAT.md`;
пилот DST-06 и путь NSIS отложены (D-07), блокируют планирование Фазы 45, не закрытие Фазы 42.
Новые тесты не требовались: пробелов покрытия нет.
