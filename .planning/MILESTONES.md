# Milestones: Trackly

History of shipped milestones. Newest first.

---

## v1.1 — AD, сотрудники и картриджная взаимосвязь

**Shipped:** 2026-06-26
**Phases:** 9–13 (5 фаз, 41 план)
**Git range:** `feat(09-01)` → `fix(clippy)` (~307 коммитов, 256 файлов, +40576/−815 строк по диапазону v1.1)
**Timeline:** 2026-06-19 → 2026-06-26 (~7 дней)

**Goal:** Достроить v1 поверх базового учёта: вход доменных пользователей через Active
Directory, по-настоящему ограниченная роль «Сотрудник» с отдельным UI, UX-доработки заявок,
сквозная взаимосвязь картриджной заявки с операцией установки и редизайн модели совместимости
принтер↔картридж.

**Delivered:**

- **Phase 9 — AD-аутентификация:** `trait AdClient` (Real LDAPS `simple_bind` / Mock-фикстуры) по образцу SNMP-триады + DC auto-detect; local→AD login fallback с сохранением constant-time anti-enumeration; заявки на регистрацию AD-пользователей (auto-accept / pending / restore) с admin approve-with-role и mode-correct reject; admin-only видимость `ad_register` на уровне SQL; вкладка настроек AD + «Запомнить меня»; пароли AD не сохраняются. (USR-08..12, REQ-06, SET-10)
- **Phase 10 — Роль «Сотрудник»:** root-cause фикс over-read (`Action::ReadData` → Admin|Manager); закрытие BFLA/BOLA на read-эндпоинтах devices/acts/cartridges/printers/reports/users по обоим транспортам; server-side own-requests scope; отдельный `EmployeeLayout` + `AccessDenied` + employee-scoped дашборд; CI-матрица role×endpoint расширена с 10 до 19 кейсов.
- **Phase 11 — Заявки/employee UX:** имя категории текстом (`LEFT JOIN request_categories`); `CreateRequest`-гейтед `request_printer_options` + дропдаун с группировкой по Расположению (обход read-closure Phase 10); realtime WS-уведомление автору-сотруднику (тост / системная нотификация) со server-side `is_visible_to` scoping по `requested_by_user_id`.
- **Phase 12 — Взаимосвязь картриджной заявки (21 план, 5 раундов gap-closure):** установка картриджа прямо из заявки «Замена картриджа» с выбором физ. картриджа, авто-подстановкой Расположения / «Кому отдал» и записью в историю; авто-возврат предыдущего картриджа в той же транзакции с инвертированным актором; junction `printer_cartridge_models` + фильтр совместимости; объединённый person-autocomplete (acts + cartridges); employee self-cancel / admin soft-delete заявок; рефактор WS в refcount-синглтон (устранён дубль нотификаций). Live-UAT пройден.
- **Phase 13 — Редизайн совместимости:** V032 миграция — `printer_brand`+`printer_model` → единый `printer_name`, снос per-device junction (V029); совместимость по `devices.name` (case-insensitive + TRIM); read-only агрегаты-по-статусу на карточке принтера + блок данных устройства; kind-aware дефолт авто-возврата фотобарабана (state 5 «Изношенный»); снят лимит списка принтеров. (SPEC-13-R1..R8)

**Requirements:** 7/7 формальных REQ-ID Phase 9 (USR-08..12, REQ-06, SET-10) satisfied; фазы 10–13 — UAT-/spec-driven с phase-local decision/SPEC ID, все verified в коде (см. `milestones/v1.1-MILESTONE-AUDIT.md`).

**Known deferred items at close:** 17 (см. STATE.md → Deferred Items). Все классифицированы аудитом milestone как `tech_debt`, без критических блокеров: остаточные human-UAT/verification маркеры уже выпущенного v1.0 + неавтоматизируемые live-browser пункты фаз 10/11 (FE-раннера нет by design) + 2 quick-таски, уже отмеченные complete в STATE.md.

**Audit:** `milestones/v1.1-MILESTONE-AUDIT.md` — статус `tech_debt`, requirements 7/7, phases 5/5, integration 5/5, flows 5/5.

---

## v1.0 — Базовый учёт (core v1)

**Shipped:** 2026-06-19
**Phases:** 1–8 (включая вставки 03.1 / 03.2 / 03.3)

**Goal:** Учёт устройств, актов приёма-передачи, картриджей, принтеров (SNMP), заявок,
отчётов, дашборда и настроек — портативное десктоп-приложение с серверным режимом для
LAN-доступа, плюс релизный пайплайн для Windows/macOS/Linux.

**Delivered:**

- **Phase 1 — Фундамент:** схема БД (миграции), single-writer pattern, portable-режим, audit_log, CI + ProcMon-gate.
- **Phase 2 — Устройства и базовый UI:** CRUD устройств, FTS-поиск, контекстный автокомплит, CSV import/export, навигационный каркас, темы.
- **Phase 3 — Акты + PDF:** акты приёма-передачи, частичные возвраты с под-нумерацией, архив, undo, krilla-PDF с кириллицей, шаблоны. (+ gap-closure 03.1/03.2/03.3)
- **Phase 4 — Картриджи:** модели + экземпляры, lifecycle, контекстные действия, журнал перемещений, баннер низкого остатка.
- **Phase 5 — Авторизация и серверный режим:** argon2id-логин, 3 роли, единый `authorize()`, HTTPS axum-сервер (rustls/rcgen), tower-sessions.
- **Phase 6 — Принтеры (SNMP) и Заявки:** discovery, SNMP-опрос, Pantum hang detection (alert-only), портал заявок для сотрудников.
- **Phase 7 — Отчёты, Дашборд, Настройки:** отчёты с группировкой по месяцам, виджеты дашборда, организация/логотип/бэкапы/шаблоны.
- **Phase 8 — Релизный пайплайн:** GitHub Actions Release matrix по push тега (NSIS + portable ZIP, .dmg, .AppImage/.deb), SHA256-checksums, README на русском.

**Deferred out of v1.0 → v1.1:** AD-аутентификация и заявки на регистрацию пользователей
(USR-08..12, REQ-06, SET-10). Вынесено при SPIDR-split 2026-06-18: release-пайплайн идёт
перед AD-фазой, чтобы Windows-сборку тестировать на реальной доменной машине.

**Deferred → v2:** MAP (карта помещений), NTF (внешние уведомления), PNT (Pantum auto-restart),
WIN7 (Windows 7 32-bit), I18N (английская локализация), ADV (полный SSO / REST API / signature pad / доп. вендоры / Postgres).

---
