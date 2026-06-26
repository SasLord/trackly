# Roadmap: Trackly

Trackly — портативное приложение для учёта техники, принтеров и картриджей с серверным
режимом для LAN-доступа. Релизная линия v1 завершена (v1.0 + v1.1). Полные детали фаз
заархивированы в `.planning/milestones/`.

## Milestones

- ✅ **v1.0 — Базовый учёт** — Phases 1–8 (shipped 2026-06-19) → `milestones/v1.1-ROADMAP.md`
- ✅ **v1.1 — AD, сотрудники и картриджная взаимосвязь** — Phases 9–13 (shipped 2026-06-26) → `milestones/v1.1-ROADMAP.md`

> Следующий milestone (v2) ещё не определён. Запусти `/gsd-new-milestone` для старта.

## Phases

<details>
<summary>✅ v1.0 — Базовый учёт (Phases 1–8) — SHIPPED 2026-06-19</summary>

- [x] Phase 1: Фундамент (6/6 plans) — completed 2026-05-25
- [x] Phase 2: Устройства и базовый UI (5/5 plans) — completed 2026-05-28
- [x] Phase 3: Акты приёма-передачи и первая PDF-печать (5/5 plans) — completed 2026-05-30
- [x] Phase 03.1: Acts quantity model + UAT gap closure (6/6 plans, INSERTED)
- [x] Phase 03.2: Deferred UAT gap closure (2/2 plans, INSERTED)
- [x] Phase 03.3: Device-list UX round 2 (2/2 plans, INSERTED) — completed 2026-06-07
- [x] Phase 4: Картриджи (6/6 plans) — completed 2026-06-12
- [x] Phase 5: Авторизация, локальные пользователи и серверный режим (6/6 plans) — completed 2026-06-14
- [x] Phase 6: Принтеры (SNMP-мониторинг) и Заявки (9/9 plans) — completed 2026-06-15
- [x] Phase 7: Отчёты, Дашборд и Настройки (14/14 plans) — completed 2026-06-18
- [x] Phase 8: Релизный пайплайн (Windows/macOS/Linux) (2/2 plans) — completed 2026-06-19

</details>

<details>
<summary>✅ v1.1 — AD, сотрудники и картриджная взаимосвязь (Phases 9–13) — SHIPPED 2026-06-26</summary>

- [x] Phase 9: AD-аутентификация и заявки на регистрацию пользователей (5/5 plans) — completed 2026-06-20
- [x] Phase 10: Ограничение роли employee + employee-UI + role-gating read (4/4 plans) — completed 2026-06-21
- [x] Phase 11: Заявки/employee UX gap-closure (3/3 plans) — completed 2026-06-22
- [x] Phase 12: Взаимосвязь картриджной заявки (21/21 plans) — completed 2026-06-25
- [x] Phase 13: Редизайн совместимости Принтеры↔Картриджи (8/8 plans) — completed 2026-06-26

</details>

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Фундамент | v1.0 | 6/6 | Complete | 2026-05-25 |
| 2. Устройства и базовый UI | v1.0 | 5/5 | Complete | 2026-05-28 |
| 3. Акты + PDF | v1.0 | 5/5 | Complete | 2026-05-30 |
| 03.1. Acts quantity model | v1.0 | 6/6 | Complete | 2026-06-05 |
| 03.2. Deferred UAT gap closure | v1.0 | 2/2 | Complete | 2026-06-06 |
| 03.3. Device-list UX round 2 | v1.0 | 2/2 | Complete | 2026-06-07 |
| 4. Картриджи | v1.0 | 6/6 | Complete | 2026-06-12 |
| 5. Авторизация и серверный режим | v1.0 | 6/6 | Complete | 2026-06-14 |
| 6. Принтеры (SNMP) и Заявки | v1.0 | 9/9 | Complete | 2026-06-15 |
| 7. Отчёты, Дашборд и Настройки | v1.0 | 14/14 | Complete | 2026-06-18 |
| 8. Релизный пайплайн | v1.0 | 2/2 | Complete | 2026-06-19 |
| 9. AD-аутентификация | v1.1 | 5/5 | Complete | 2026-06-20 |
| 10. Роль employee + role-gating | v1.1 | 4/4 | Complete | 2026-06-21 |
| 11. Заявки/employee UX | v1.1 | 3/3 | Complete | 2026-06-22 |
| 12. Взаимосвязь картриджной заявки | v1.1 | 21/21 | Complete | 2026-06-25 |
| 13. Редизайн совместимости | v1.1 | 8/8 | Complete | 2026-06-26 |

## Coverage

- **v1 requirements mapped:** 120 / 120 ✓ (см. `milestones/v1.1-REQUIREMENTS.md`)
- **Orphans:** none

## Out of v1 Roadmap (Deferred to v2)

| Category | Reason |
|----------|--------|
| MAP-01..04 (Карта помещений) | Высокая UI-сложность; ценность учёта не зависит от карты — отложено в v2 milestone |
| NTF-02 (SMTP), NTF-03 (Telegram), NTF-04 (Webhook), NTF-05 (event subscriptions) | In-app часть покрыта REQ-04 в Phase 6; внешние каналы — финальная фаза v2 |
| PNT-01..04 (Pantum auto-restart) | В v1 — только детекция и алерт (PRN-06); авто-restart требует подтверждённой гипотезы и безопасного механизма (v2) |
| WIN7-01..02 (Windows 7 32-bit) | Best-effort; MSRV `krilla` 1.92 + WebView2 TLS 1.2 могут закрыть дверь — отдельный spike в v2 |
| I18N-01..03 (Английская локализация) | Команда и пользователи русскоязычные; добавляется без архитектурных переделок |
| ADV-01..05 (SSO/REST API наружу/Signature pad/доп. вендоры принтеров/Postgres) | Преждевременная сложность для текущего масштаба |
