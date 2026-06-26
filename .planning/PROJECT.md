# Trackly

## What This Is

Trackly — приложение для учёта и отслеживания техники, принтеров и картриджей в организации с несколькими локациями и складами. Десктоп-приложение (Tauri + Svelte) с встроенным режимом сервера, позволяющим сотрудникам подключаться через браузер из локальной сети для отправки заявок и работы с системой. Целевая среда — Windows-сеть с Active Directory, разработка ведётся на macOS.

## Core Value

Учёт устройств и картриджей с актами приёма-передачи и историей перемещений должен работать надёжно и быстро в режиме «одной кнопкой» — без обращения к Excel-таблицам, ручного присвоения номеров актов или потери истории при возврате на склад.

## Current State

**Shipped:** v1.1 (2026-06-26) — релизная линия v1 завершена (v1.0 + v1.1, фазы 1–13).

Trackly поставляет полный учёт устройств / актов / картриджей / принтеров (SNMP) / заявок,
отчёты, дашборд и настройки в портативном и серверном (LAN) режимах, с релизным пайплайном
для Windows/macOS/Linux. **v1.1** добавил: AD-аутентификацию (web, `ldap3 simple_bind`,
пароли не хранятся) с заявками на регистрацию; по-настоящему ограниченную роль «Сотрудник»
с отдельным UI и серверным role-gating; UX-доработки заявок (категории, WS-уведомления
автору, дропдаун принтеров); сквозную взаимосвязь картриджной заявки с операцией установки
(авто-возврат предыдущего картриджа, совместимость); редизайн модели совместимости
принтер↔картридж по уникальному наименованию принтера.

Полная история — `.planning/MILESTONES.md`; детали фаз — `.planning/milestones/`.

## Next Milestone Goals

Milestone v2 ещё не определён. Запусти `/gsd-new-milestone` для старта (questioning →
research → requirements → roadmap). Кандидаты из v2-бэклога (см. «Out of Scope» ниже):
карта помещений (MAP), внешние уведомления (NTF: SMTP/Telegram/Webhook), Pantum
auto-restart (PNT), полный SSO Kerberos/NTLM (ADV-01), английская локализация (I18N),
Windows 7 32-bit (WIN7).

## Requirements

### Validated

<!-- Shipped and confirmed valuable. -->

**v1.0 (shipped 2026-06-19, phases 1–8):** учёт устройств / актов / картриджей / принтеров (SNMP) /
заявок / отчётов / дашборда / настроек; портативный + серверный режим; релизный пайплайн. См. `.planning/MILESTONES.md`.

**v1.1 (shipped 2026-06-26, phases 9–13):** AD-аутентификация (web, `ldap3 simple_bind`, пароли не
хранятся) + заявки на регистрацию (USR-08..12, REQ-06, SET-10); ограниченная роль «Сотрудник» +
серверный role-gating read-эндпоинтов; UX-доработки заявок (категории, WS-уведомления автору,
дропдаун принтеров); сквозная взаимосвязь картриджной заявки → установка (авто-возврат, совместимость);
редизайн совместимости принтер↔картридж по уникальному наименованию. См. `.planning/milestones/v1.1-REQUIREMENTS.md`.

**Итог:** все 120 v1-требований validated (120/120). Полная трассировка — `.planning/milestones/v1.1-REQUIREMENTS.md`.

### Active

<!-- Current scope. Building toward these. -->

Все v1-требования поставлены и перенесены в Validated (см. выше). Активной рабочей области
нет — следующий milestone (v2) определяется через `/gsd-new-milestone`. До этого момента
кандидаты на работу перечислены в «Out of Scope» (v2-бэклог) ниже.

### Out of Scope

<!-- Explicit boundaries. Includes reasoning to prevent re-adding. -->

- **Раздел «Карта» (план помещений с расстановкой устройств)** — отложен на отдельный milestone v2; v1 фокусируется на учёте, не на визуализации топологии
- **Мультиорганизация / multi-tenant** — продукт под одну организацию; данные организации хранятся в настройках
- **Облачная синхронизация / SaaS-режим** — только локальное развёртывание (portable + LAN-сервер), внешних облаков нет
- **Авто-restart спулера Pantum в v1 (первая фаза)** — на старте только мониторинг и алерт; автоматический фикс — позже, после подтверждения гипотезы и при наличии безопасного механизма
- **i18n (английский UI)** — только русский язык, можно добавить позже без существенных изменений архитектуры (тексты выносятся в локализуемые ресурсы там, где это естественно)
- **Мобильное приложение** — веб-доступ из браузера в LAN считается достаточным
- **Postgres / централизованный сервер БД** — на текущем масштабе (3-10 локаций, до 5000 устройств, до 20 одновременных) SQLite + WAL покрывают потребности; миграция возможна, но не в v1
- **Полноценный SSO (Kerberos/NTLM) при первой итерации** — сначала локальные учётки с паролем; AD-вход добавляется в отдельной фазе с заявками на регистрацию
- **Поддержка не-Pantum/Kyocera/HP/Canon принтеров в первой фазе мониторинга** — SNMP-профили будут проверяться на этих производителях; остальные — best-effort

## Context

**Среда эксплуатации:**
- Локальная сеть: Windows Server 2022 с Active Directory (домен).
- Целевые рабочие станции: Windows (32/64-bit), позже миграция большинства на Linux.
- Разработка на macOS (Apple Silicon) — тесты под Windows запускаются через VM/виртуалку или в CI.

**Существующие болевые точки, которые решает Trackly:**
1. Ручной учёт актов приёма-передачи устройств в Excel/бумаге — потеря истории при возврате.
2. Pantum BM5100ADN периодически зависают в сети с AD: не печатают без видимой ошибки, помогает только рестарт спулера. Нужно как минимум обнаруживать и алертить.
3. Учёт картриджей по моделям, заправкам, низким остаткам ведётся вручную.

**Целевые принтеры для мониторинга:** Pantum BM5100ADN (основные, проблемные), Kyocera ECOSYS, HP LaserJet, Canon iR.

**Способ запуска:**
- Portable-режим в primary use case: один администратор запускает .exe (или Tauri-бандл), рядом ложится файл БД и конфиг.
- Сервер-режим включается переключателем — администратор остаётся в десктопе, прочие пользователи (специалисты, сотрудники) подключаются через браузер по LAN.

**Масштаб:** до 10 локаций, до 5000 устройств, до 20 одновременных пользователей.

## Constraints

- **Тех-стек:** Rust (бэкенд), Tauri (десктоп-обёртка), Svelte (фронтенд), SCSS (стили), SQLite (БД) — фиксировано пользователем.
- **Целевая платформа:** Windows 64-bit (primary), macOS Apple Silicon (dev + use), Linux (вторичная). Опционально Windows 7 32-bit, если позволит выбранный Rust toolchain и Tauri версия.
- **Portable:** приложение не должно требовать установки и записывать данные в `%APPDATA%`/`%LOCALAPPDATA%`/системные пути. БД и конфиг — рядом с исполняемым файлом (или в каталоге, указанном пользователем).
- **Безопасность:** пароли пользователей — только хэш (argon2 / bcrypt); чувствительные данные AD (пароли) — не сохранять, только использовать для bind/проверки. В режиме сервера — HTTPS-сертификат (self-signed по умолчанию, путь к собственному — настраиваемый).
- **Concurrent-доступ:** SQLite в режиме WAL, единая точка записи через бэкенд-слой (никаких прямых записей из нескольких процессов).
- **Языковая локализация:** UI и шаблоны документов — только русский в v1.
- **Размещение кода:** GitHub. CI: проверки кода на push, релизы по тегам.
- **Документы:** редактируемые шаблоны печатных форм должны храниться в БД (или рядом с БД), чтобы переноситься вместе с portable-сборкой.

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Tauri + Svelte + Rust + SQLite | Lightweight, mature stack для portable-десктопа с web-доступом; меньше runtime-зависимостей чем Electron; единый язык (Rust) для логики и для встроенного HTTP-сервера | ✓ Good — выдержал всю v1 (13 фаз, dual-transport Tauri+axum на общих DTO через specta) |
| Сервер — toggle в десктопе, не отдельный бинарь | Один артефакт релиза, один UI для администрирования, проще распространять и обновлять; для текущего масштаба headless-сервис избыточен | ✓ Good — серверный режим + LAN-браузер работают; десктоп остаётся trusted-admin |
| SQLite + WAL (не Postgres) | Масштаб (до ~5000 устройств, до 20 одновременных) укладывается в возможности SQLite; portable-режим невозможен с серверной СУБД | ✓ Good — single-writer + reader-pool, 50 concurrent writes без SQLITE_BUSY |
| Локальные пользователи в v1, AD-вход — отдельная фаза | Снимает блокер на старте (можно начать работу без интеграции с AD), оставляет дверь открытой для расширения | ✓ Good — AD добавлен в v1.1 (Phase 9) через `trait AdClient` без переделок auth-слоя |
| Только русский UI в v1 | Команда и пользователи — русскоязычные; i18n добавляется без архитектурных переделок | ✓ Good — весь UI и шаблоны на русском; i18n остаётся в v2-бэклоге |
| «Карта» — в milestone v2 | Высокая сложность UI, низкий приоритет относительно учётной части; ценность системы не зависит от карты | — Pending (отложено в v2) |
| Pantum auto-restart spooler — отдельная фаза после мониторинга | Сначала наблюдаем и алертим (безопасно), автоматический фикс — после подтверждения гипотезы и безопасного механизма | — Pending (v1 = только alert-only PRN-06; auto-restart в v2) |
| Уведомления (Email/Telegram/Webhook) — последняя фаза v1 | Не блокируют учётный функционал; в начале — только in-app | ⚠️ Revisit — в v1 поставлены только in-app (REQ-04) + WS-уведомления (D-WS-01); внешние каналы перенесены в v2 |
| Phase 1 — фундамент (схема БД + слой данных + миграции) | Большой объём связей между устройствами/актами/картриджами/заявками — хочется устаканить схему до строительства UI, иначе будут переделки | ✓ Good — фундамент выдержал 32 миграции (V001→V032) без переделок схемы |
| v1: AD = `ldap3 simple_bind` (web-only), auto-SSO отложен (Phase 9, D-AD-01) | Полный Kerberos/NTLM SSO — преждевременная сложность; `simple_bind` через LDAPS покрывает вход; пароли не хранятся | ✓ Good — AD-вход работает; `trait AdClient` оставляет место под SSO-адаптер (ADV-01 в v2) |
| v1.1: совместимость принтер↔картридж по `printer_name`, не per-device junction (Phase 13) | Per-device junction (V029) оказался избыточным и хрупким; free-text по уникальному наименованию принтера проще и переносим | ✓ Good — V032 свернул junction, совместимость по `devices.name` (case-insensitive+TRIM) |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd-complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

## Phase Evolution Log

- **Phase 1: Фундамент** (completed 2026-05-25) — workspace, схема БД (миграции V001–V012), single-writer pattern, portable mode, audit_log, ProcMon CI gate.
- **Phase 2: Устройства и базовый UI** (completed 2026-05-28) — V013 миграция (FTS5 triggers + 5 autocomplete partial indexes), полный CRUD устройств в Tauri-десктоп + axum HTTP, hash-router SPA с sidebar/ThemeSwitcher/toast/11 primitives, FTS5 search с Cyrillic-aware токенайзером, контекстный autocomplete (per-field + ctx_name + ctx_status_id), Status switch-bar с counters, группировка по Наименованию с expand, bulk_create 1..=100 для не-уникальных устройств, locations round-trip через INSERT OR IGNORE, CSV import (BOM+chardetng detect, ; / , delimiter, 5-min TTL session) + CSV export (UTF-8 BOM + ; + Russian headers + formula-injection guard).
- **Phase 03.3: Device-list UX round 2** (completed 2026-06-07) — флаг `group_by_condition` в `DeviceFilter`/`list_grouped` (раздельная DEF-2B разбивка: выкл для списка устройств, вкл для акт-формы) + `condition_distinct_count` с индикатором «разное»; колонка «Состояние» + native tooltip (`title=`) на text-ячейках; скрытие колонки «Статус» при выбранном статус-фильтре; вторая секция автокомплита «Все расположения» + HTTP route `POST /api/v1/locations_autocomplete`. UAT 5/5 пройден; group-row ячейки усечены до одной строки с ellipsis.
- **Phase 9: AD-аутентификация** (completed 2026-06-20) — `trait AdClient` (I/O-free core) + `RealAdClient` (ldap3 LDAPS `simple_bind`) / `MockAdClient` (us100/us200-фикстуры) + hickory-resolver DC auto-detect, по образцу SNMP-триады; `AuthService` local→AD login fallback с сохранением constant-time anti-enumeration + `find_user_any_state`; V028 `ad_subtype` миграция; заявки на регистрацию (auto-accept / pending / restore) с admin approve-with-role и mode-correct reject; admin-only видимость `ad_register` на уровне SQL; `settings_get/set_ad` + «Запомнить меня» cookie-policy; UI: login/Pending/Blocked экраны + вкладка настроек AD + admin-review. Пароли AD не сохраняются (`Secret<T>`). (USR-08..12, REQ-06, SET-10)
- **Phase 10: Роль «Сотрудник» + role-gating** (completed 2026-06-21) — root-cause фикс over-read: `Action::ReadData` вынесен из always-true ветки в Admin|Manager; гейтинг чтения devices/acts/cartridges/printers/reports/users по обоим транспортам (BFLA/API5:2023 closure); server-side own-requests scope + BOLA-фикс на `requests_get`/`get_history`; employee-scoped дашборд-ветка (без утечки org-wide данных); отдельный `EmployeeLayout.svelte` + `AccessDenied.svelte` + симметричная 403-обработка в `client.ts`; CI-матрица role×endpoint расширена 10→19 кейсов. Клиентский гейтинг — только UX, реальная граница серверная.
- **Phase 11: Заявки/employee UX gap-closure** (completed 2026-06-22) — D-CAT-01: категория заявки рендерится текстом (`LEFT JOIN request_categories` → `category_name`), серверный список `{id,name}` по обоим транспортам; D-PRN-01: эндпоинт `request_printer_options` под `Action::CreateRequest` (минимальный DTO `{id,name,location}`) + `GroupedPrinterSelect` (группировка по Расположению); D-WS-01: `WsEvent::RequestStatusChanged += requested_by_user_id` + split-arm `is_visible_to` (автор видит только свою заявку), тост/системная нотификация в `EmployeeLayout`. Code review fix-pass закрыл CR-01 (двойной WS-broadcast в HTTP-хендлерах — корень «WS toast spam») + 6 warnings. Verified 11/11 must-haves; 7 пунктов human-UAT (живой браузер) — `11-HUMAN-UAT.md` (partial).
- **Phase 12: Взаимосвязь картриджной заявки** (completed 2026-06-25, 21 планов, 5 раундов gap-closure) — установка картриджа из заявки «Замена картриджа» стала полнофункциональной: выбор физ. картриджа из БД (на складе, совместимого), авто-подстановка Расположения/«Кому отдал», запись `completed_cartridge_id` + история. Раунды gap-closure закрыли: общий автокомплит имён (`suggest_person` агрегирует acts + cartridges.holder_name + given_by); junction `printer_cartridge_models` (совместимость принтер↔модель по ID, фильтр обоих направлений); авто-возврат предыдущего картриджа в той же транзакции с инвертированным актором в истории; управление жизненным циклом заявки (reject из «В работе», soft-delete Admin/Manager, отмена своей Employee); снят CHECK ip/usb; дедуп WS-нотификаций (`connectWs` refcount-синглтон); опциональный выбор принтера в cartridge-centric установке + фикс резолва принтера по `device_id` (новый `printers_get_by_device_id`). Live-UAT R5-1/R5-2 пройден пользователем. Отложено в Phase 13: drum-kind дефолт состояния в авто-возврате, лимит списка принтеров 500-vs-200.
- **Phase 13: Редизайн совместимости Принтеры↔Картриджи** (completed 2026-06-26) — V032 миграция: `printer_brand`+`printer_model` → единый `printer_name`, снос per-device junction (V029); `cartridges_sqlite.rs` матчит совместимость по `devices.name` (case-insensitive + TRIM, D-05 pass-through только в selection-фильтре); удалены 4 V029-команды, новая read-only `printers_get_compatible_aggregates` (агрегаты по статусу); карточка принтера: агрегаты совместимости + блок данных устройства (через `DeviceFormModal`) + установленный картридж по коду; `ModelFormModal` — единый free-text-блок «Совместимые принтеры»; kind-aware дефолт авто-возврата фотобарабана (state 5 «Изношенный»); снят лимит списка принтеров; `suggest_compat_printer` re-sourced с free-text истории на `devices.name`. Завершает milestone v1.1. (SPEC-13-R1..R8)

---
*Last updated: 2026-06-26 — milestone v1.1 завершён и заархивирован (phases 9–13). Релизная линия v1 закрыта (120/120 требований). Тег v1.1. Следующий шаг — `/gsd-new-milestone`.*
