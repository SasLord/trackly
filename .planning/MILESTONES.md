# Milestones: Trackly

History of shipped milestones. Newest first.

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
