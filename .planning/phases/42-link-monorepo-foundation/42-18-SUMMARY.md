---
phase: 42-link-monorepo-foundation
plan: 18
subsystem: ci-release
tags: [github-actions, link, LNK-01, release-isolation]
requires: [42-08, 42-10]
provides:
  - .github/workflows/release-link.yml (dry-run выпуска Link, без Release)
  - scripts/check-release-isolation.mjs (гейт I0-I6 + selftest)
  - ci-fast: гейт изоляции, сборка и svelte-check link-ui
key-files:
  created: [.github/workflows/release-link.yml, scripts/check-release-isolation.mjs]
  modified: [.github/workflows/ci-fast.yml]
metrics:
  tasks: 2
  completed: 2026-11-26
---

# Phase 42 Plan 18: dry-run выпуска Link и гейт изоляции Summary

release-link.yml собирает NSIS-установщик Link (windows-latest, без heic и vcpkg) и выкладывает его артефактом на 7 дней; Release не создаётся, права `contents: read`. Версия подставляется только в `crates/link-app` (значение через env, эхо точной строкой, плюс проверка формата). Коммиты: 61dc0281, df3db346.

## Что сделано

- Триггеры: `workflow_dispatch` (version, по умолчанию 0.1.0) и теги `link-v*.*.*`; release.yml не изменён.
- Гейт `check-release-isolation.mjs`: правила I0-I6 (теги, модель glob GitHub, запрет публикации Release, запрет heic/vcpkg, чистота release.yml). Selftest: 9 проверок, мутанты I1, I2, I4, I5, I6 краснеют, комментарий с запрещённым словом не считается.
- ci-fast: гейт в шаге «Root gate selftests», шаги `pnpm --dir link-ui build` и `svelte-check`.

## Проверка

selftest и реальный прогон гейта PASS; check-privacy PASS (в pre-commit дважды).

## Отклонения

**[Замечание] link-ui не имеет lint-скрипта.** Отдельный `pnpm --dir link-ui lint` не добавлен: eslint/prettier/границы link-ui уже покрывает `pnpm lint` в `ui/` (prettier `../link-ui`, check-workspace-boundaries). В ci-fast добавлены только build и svelte-check.

## К проверке позже

- **Новые шаги link-ui в ci-fast будут красными до плана 16**: в `link-ui` пока нет `index.html`, vite-конфига и `src/` (локально `pnpm --dir link-ui build` падает «Could not resolve entry module index.html»). Это ожидаемо по порядку волн; план 16 (волна 10) создаёт вход. Не пушить промежуточное состояние в main без этого учёта.
- Живой dry-run `release-link.yml` и фактический путь артефакта (`target/release/bundle/nsis` в корне workspace) подтверждает план 19.
- URL/SHA1 NSIS скопированы из release.yml (CLI 2.11.2); при обновлении CLI менять в двух местах.

## Self-Check: PASSED
