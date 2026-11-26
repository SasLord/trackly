---
phase: 42-link-monorepo-foundation
plan: 04
subsystem: docs
tags: [pilot, powershell, applocker, rds, dst-06]
requires: []
provides:
  - "docs/pilot/: набор для пилота установки Link на доменных ПК и RDS"
affects: [45]
key-files:
  created:
    - docs/pilot/README.md
    - docs/pilot/diag.ps1
    - docs/pilot/install-silent.ps1
    - docs/pilot/sign-notes.md
    - docs/pilot/RESULT-TEMPLATE.md
    - docs/pilot/RESULT.md
decisions:
  - "RESULT.md остаётся заглушкой «ОТЛОЖЕНО: human-UAT (D-07)» до проведения пилота"
metrics:
  completed: 2026-11-26
  tasks: 2
  files: 6
---

# Phase 42 Plan 04: Набор для пилота Summary

Подготовлены скрипты диагностики и тихой установки, девятипунктный чек-лист, заметки по подписи (AD CS и самоподписанный) и обезличенный шаблон итога.

## Выполнено

- Задача 1 (7b781c22): `diag.ps1` (только чтение, JSON без имени ПК, пользователя, IP, домена и Subject; PE Machine 0x8664; WebView2 pv; AppLocker 8002-8007; SRP; сеанс RDS) и `install-silent.ps1` (`/S` и `/D=` последним, проверки, идемпотентность, `-Uninstall`).
- Задача 2 (7b781c22 и правка sign-notes отдельным коммитом): README, sign-notes, RESULT-TEMPLATE, RESULT-заглушка.

## Проверка

- Privacy-гейт: PASS, 0 нарушений. Ручная сверка: названий организации, хостов и имён нет.
- Вне комментариев в `diag.ps1` нет обращений к переменным имени компьютера, пользователя и домена.
- Синтаксис `.ps1` не проверен: pwsh отсутствует.

## Отложенная проверка (human-UAT, D-07)

Сам пилот (2-3 ПК без прав администратора, один RDS-сервер) проводит пользователь по `docs/pilot/README.md`. Не самоодобрялся. Блокирует планирование Фазы 45, закрытие Фазы 42 не блокирует.

## Deviations from Plan

Правка формулировки в sign-notes.md ради критерия приёмки (слово «самоподпис» в нижнем регистре). Иных отклонений нет.

## Self-Check: PASSED
