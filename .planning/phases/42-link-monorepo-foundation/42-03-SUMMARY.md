---
phase: 42-link-monorepo-foundation
plan: 03
subsystem: docs
tags: [adr, link, monorepo, fts, rds]
requires: []
provides:
  - "docs/adr/: индекс и ADR 0001-0005 с таблицей покрытия D-22"
affects: [43, 44, 45, 47]
key-files:
  created:
    - docs/adr/README.md
    - docs/adr/0001-monorepo-packages-versioning.md
    - docs/adr/0002-link-network-pinning-secrets.md
    - docs/adr/0003-chat-model-fts-delivery.md
    - docs/adr/0004-updates-distribution-rds.md
    - docs/adr/0005-install-path-and-user-data.md
decisions:
  - "Сеть Link только в Rust, SPKI-pin, токен через keyring-core 1.1.0 + store-крейты"
  - "Модель чата im_*, доставка сигнал + курсор, FTS5 trigram + нормализация ё->е"
  - "Вариант A распространения, обновления только с сервера, RDS: single-instance на сеанс"
  - "Link: крейт link-app (bin trackly-link), версия 0.1.0 литералом, теги link-v*.*.*, bundle id org.trackly.link"
metrics:
  completed: 2026-11-26
  tasks: 3
  files: 6
---

# Phase 42 Plan 03: ADR решений вехи v1.5 Summary

Записаны пять ADR на русском и индекс со статусами; таблица «Покрытие D-22» из 12 строк (9 решений и 3 расхождения) ссылается на файлы.

## Выполнено

- Задача 1 (0ddccd91): README, ADR 0001 и 0005.
- Задача 2 (3e904ca8): ADR 0002 и 0003 с таблицей эксперимента FTS5.
- Задача 3 (9d7cd5cb): ADR 0004. Таблица покрытия D-22 была включена в README ещё в задаче 1, поэтому в задаче 3 README не менялся.

ADR 0004 и 0005 помечены «Accepted (provisional)» до итогов пилота.

## Deviations from Plan

None - план выполнен как написан (порядок записи README сдвинут на задачу 1, содержание то же).

## Проверки

`node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256`: PASS, 0 нарушений на каждом коммите. Ручная сверка: реальных названий, хостов и имён нет.

## Known Stubs

Нет.

## Self-Check: PASSED

Все шесть файлов существуют, коммиты 0ddccd91, 3e904ca8, 9d7cd5cb в истории.
