# Phase 3: Акты приёма-передачи и первая PDF-печать - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-28
**Phase:** 3-pdf
**Areas discussed:** Нумерация и lifecycle возвратов, UX актов (список+создание+возврат+печать), PDF-стек (krilla vs spike), Шаблоны+seeding+org-data

---

## Нумерация и lifecycle возвратов

### Q1: «42в» vs «42в1/в2/в3» — что значит «полный возврат»?

| Option | Description | Selected |
|--------|-------------|----------|
| «в» только при ровно одном возврате | Display-rule на чтении: если за всю жизнь был ровно 1 return и он закрыл всё → «42в»; >1 → все «в1», «в2». БД хранит sub_number=1,2,3 всегда. Ретроактивное переименование возможно. | ✓ |
| «в» = флаг при создании | Пользователь явно выбирает full/partial; нужно менять схему или вводить is_full флаг. Нет ретроактивного переименования. | |
| Всегда «в1», единых «в» нет | Отказ от «42в». Проще, но противоречит REQ-ACT-07. | |

**User's choice:** «в» только при ровно одном возврате
**Notes:** Ретроактивное переименование «42в» → «42в1» при появлении 2-го возврата — приемлемо. Старые распечатанные PDF остаются со «своим» суффиксом (snapshot).

### Q2: Когда акт уходит в Архив?

| Option | Description | Selected |
|--------|-------------|----------|
| Авто по факту 100% возвращения, ручного флага нет | derived state, undo автоматически снимает archived | ✓ |
| Авто + ручное архивирование | Плюс кнопка для устройств «утеряны/списаны» — но создаёт inconsistencies | |
| Только ручное | Противоречит REQ-ACT-09 | |

**User's choice:** Авто по факту 100% возвращения, ручного флага нет

### Q3: Undo — откуда восстанавливается состояние?

| Option | Description | Selected |
|--------|-------------|----------|
| act_items snapshot (condition_at_time/complectation_at_time) | Из самой схемы V004; быстро, не зависит от audit_log retention. Работает только для undo return. | |
| audit_log.before_json для всех операций | Универсально (handover и return); зависит от retention; гибче | ✓ |
| Гибрид (act_items для return, audit_log для handover) | Полное покрытие двумя путями | |

**User's choice:** audit_log.before_json для всех операций
**Notes:** Retention пока не включён (Phase 7) → undo гарантирован. act_items.condition_at_time остаётся в схеме как denormalized snapshot для отчётности.

---

## UX актов (список + создание + возврат + печать)

### Q4: Раскладка списка актов

| Option | Description | Selected |
|--------|-------------|----------|
| Плоская таблица + switch-bar (как DEV) | Идиома Phase 2 DeviceList | |
| Master-detail: слева список, справа детали | Компактный список + правая карточка с позициями/действиями | ✓ |
| Tree handover→returns expandable | Богатый UX, но противоречит REQ-ACT-02 (три отдельные вкладки) | |

**User's choice:** Master-detail
**Notes:** Размеры split 35/65 фиксированные в Phase 3 (resizable — Phase 7).

### Q5: Форма создания акта

| Option | Description | Selected |
|--------|-------------|----------|
| Отдельная страница-конструктор | Полноэкранный wizard | |
| Широкий модал (~1000px) | Шапка + add-row позиции, как DeviceFormModal pattern | ✓ |
| Двухшаговый: модал шапки → inline-редактирование в detail-pane | Меньше модального веса, но draft-сущность | |

**User's choice:** Широкий модал

### Q6: Возврат + Печать (парой)

| Option | Description | Selected |
|--------|-------------|----------|
| Возврат: bulk-default + per-row override; Печать: save-dialog без preview | Простейший вариант, без PDF.js | |
| Возврат: bulk-default + per-row override; Печать: preview-модал с встроенным PDF.js | Лучший UX, ~300KB PDF.js в bundle, server-mode тоже работает | ✓ |
| Возврат: две вкладки (общее/по позициям); Печать: «открыть в системном просмотрщике» | Без save-dialog, пользователь сам сохраняет | |

**User's choice:** Preview-модал с встроенным PDF.js + bulk+per-row override на возврате

---

## PDF-стек

### Q7: krilla / spike / гибрид

| Option | Description | Selected |
|--------|-------------|----------|
| Commit в krilla 0.7 без spike | Быстро, риск выявить позже | |
| Spike 1-2 дня krilla vs typst-as-lib | Дороже на старте, но снимает MEDIUM-LOW риск | |
| krilla сразу + первый план фазы = «PDF-инфра + фикстура» | Компромисс: structural spike через первый plan | ✓ |

**User's choice:** krilla + первый план = «PDF foundation» (структурный спайк)
**Notes:** Если первый план покажет проблемы с krilla — отдельная mini-фаза на typst-as-lib спайк.

### Q8: Шрифт и CI hash-test

| Option | Description | Selected |
|--------|-------------|----------|
| PT Sans Regular+Bold, hash на всех ОС | ~120KB/cut, OFL, RU-friendly | |
| DejaVu Sans Regular+Bold, hash на всех ОС | Шире покрытие, ~720KB до subsetting (krilla подсетит) | ✓ |
| PT Sans, hash только на linux-runner | Меньше жёсткости, легче поддержка | |

**User's choice:** DejaVu Sans Regular+Bold, byte-for-byte hash на linux/macOS/windows

---

## Шаблоны + seeding + org-data

### Q9: Откуда «шапка» организации в Phase 3?

| Option | Description | Selected |
|--------|-------------|----------|
| Новая таблица organization (single-row) с placeholder'ами | Миграция V014, сразу схема + service + command | |
| Hardcoded placeholder в сервисе | Заглушка до Phase 7 | |
| JSON-файл org.json рядом с .exe (portable) | Не трогаем БД; вручную править в блокноте; не попадает в backup БД | ✓ |

**User's choice:** org.json рядом с .exe
**Notes:** Backup БД НЕ захватывает org.json — намеренно (локальная конфиг инстанса). Phase 7 может пересмотреть.

### Q10: Формат шаблонов и pipeline

| Option | Description | Selected |
|--------|-------------|----------|
| HTML → krilla (нужен промежуточный слой) | krilla не рендерит HTML напрямую | |
| Typst-like markup | Своя вёрстка, полный контроль | |
| MiniJinja → JSON-AST DocSpec → krilla | Жёсткая валидация серде, типизировано, тестируется без рендера | ✓ |

**User's choice:** MiniJinja → DocSpec JSON → krilla

### Q11: Первоначальное заполнение document_templates

| Option | Description | Selected |
|--------|-------------|----------|
| Refinery migration V014__seed_templates.sql | Просто, в git, но SQL-строки с большими JSON-MiniJinja неудобны в review | |
| Runtime seeding из include_str! на startup | Шаблоны отдельные файлы в репо (lint/diff удобно), идемпотентность через count=0 | ✓ |
| Hybrid: migration + include_str! | Сложнее без явной выгоды | |

**User's choice:** Затрудняюсь — выбери лучший вариант (надёжный и удобный)
**Claude's choice:** Runtime seeding из include_str!
**Rationale:** Файлы шаблонов в репо удобны для review/lint/diff; SQL миграция с большими minijinja-телами неудобна в обзоре. Идемпотентность через `count(*) per kind = 0` — простой и понятный invariant; пользователь soft-удалил все шаблоны kind'а → пересоздаём с дефолтами (feature «сбросить»).

---

## Claude's Discretion

- Точная форма DocSpec enum-вариантов и optional полей.
- Структура `crates/trackly-app/src/pdf/` (split на renderer/docspec/fonts/minijinja_env).
- Точная модель `act_items` относительно multi-state (quantity_returned / отдельная `act_item_returns` таблица) — на усмотрение planner после изучения V004.
- Имена commands (придерживаемся snake_case + namespace `acts_*`).
- Конкретный PDF.js bundling путь (npm dep + dynamic import).
- Конкретная типография PDF (отступы, размеры шрифта) — монохромная по умолчанию.
- Возможные дополнительные миграции (V014__acts_indexes_or_seeds.sql).
- Выбор runtime-seeding шаблонов из include_str! (по запросу пользователя — выбрал лучший вариант).

## Deferred Ideas

- UI редактор шаблонов → Phase 7.
- Полноценная страница Организация/Настройки → Phase 7.
- 3-way merge для default-шаблонов при upgrade → Phase 7.
- «Сбросить шаблон к дефолту» кнопка → Phase 7.
- Logo binary в БД → Phase 7.
- PDF.js custom worker → out of scope.
- Retention для audit_log → Phase 7.
- Resizable split master-detail → Phase 7.
- Виртуализация списка актов → отдельная perf-фаза.
- Печать списка/отчёта по актам → Phase 7.
- Заявки печать (REQ-04) → Phase 6.
- Watch-режим для org.json → Phase 7.
- Запрет переиспользования удалённых номеров — отслеживаем; пока валидация в сервисе.
- Spike krilla vs typst-as-lib — НЕ upfront, только если plan 01 покажет проблемы.
