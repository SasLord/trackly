# Roadmap: Trackly

## Milestones

- ✅ **v1.0 — Базовый учёт** (shipped 2026-06-19) — `MILESTONES.md`
- ✅ **v1.1 — AD, сотрудники, картриджная взаимосвязь** (shipped 2026-06-26) — `milestones/v1.1-*`
- ✅ **v1.1.2 — Пост-релизные доработки UX и печати** (Фазы 18–22, shipped 2026-07-15) — `milestones/v1.1.2-*`
- ✅ **v1.2 — Редизайн UI и дизайн-система** (Фазы 23–30, shipped 2026-07-29) — `milestones/v1.2-*`
- ✅ **v1.3 — AD-SSO паритет + полировка превью печати** (Фазы 31–33, shipped 2026-08-08) — `milestones/v1.3-*`
- 📋 **Следующая веха** — не определена (`/gsd-new-milestone`)

## Phases

<details>
<summary>✅ v1.2 Редизайн UI и дизайн-система (Фазы 23–30) — SHIPPED 2026-07-29</summary>

- [x] Phase 23: Design tokens foundations
- [x] Phase 24: Base components
- [x] Phase 25: Tables / Dropdown
- [x] Phase 26: Windows with mockup
- [x] Phase 27: Core workflow windows
- [x] Phase 28: Support / admin windows
- [x] Phase 29: Login & employee shell
- [x] Phase 30: Quality — a11y & platform parity (9/9 plans) — completed 2026-07-29

Полная детализация — `milestones/v1.2-ROADMAP.md`; требования — `milestones/v1.2-REQUIREMENTS.md`;
аудит — `milestones/v1.2-MILESTONE-AUDIT.md` (26/26, tech_debt, без блокеров).

</details>

Более ранние milestone'ы (v1.0 / v1.1 / v1.1.2) — в `milestones/` и `MILESTONES.md`.

<details>
<summary>✅ v1.3 AD-SSO паритет + полировка превью печати (Фазы 31–33) — SHIPPED 2026-08-08</summary>

- [x] Phase 31: Служебный AD-bind — ФИО и роли из AD-групп (4/4 plans) — completed 2026-08-03
- [x] Phase 32: Авто-админ по списку логинов + релиз SSO в main (5/5 plans) — completed 2026-08-04
- [x] Phase 33: Полировка предпросмотра печати (4/4 plans) — completed 2026-08-04

Полная детализация — `milestones/v1.3-ROADMAP.md`; требования — `milestones/v1.3-REQUIREMENTS.md`;
аудит — `milestones/v1.3-MILESTONE-AUDIT.md` (6/6 требований, tech_debt, без блокеров).

Релизы вехи: `v1.3.0` (SSO), `v1.3.1` (LDAP plain/StartTLS + Paged.js-печать),
`v1.3.2` (синхронизация ФИО при смене фамилии в AD).

</details>

**Следующая веха не определена** — `/gsd-new-milestone`.

## Backlog

- **999.1 — role-based route gating** — UX-полировка: гейт маршрута ≠ гейт меню для admin-vs-manager
  (реальная граница безопасности — backend 403). Отложено из v1.2.
  Каталог: `phases/999.1-role-based-route-gating/`.

## Progress

| Phase | Milestone | Plans | Status | Completed |
| ----- | --------- | ----- | ------ | --------- |
| 23–29 | v1.2 | — | Complete | (см. архив) |
| 30. Quality — a11y & parity | v1.2 | 9/9 | Complete | 2026-07-29 |
| 31. Служебный AD-bind — ФИО и роли из AD-групп | v1.3 | 4/4 | Complete    | 2026-08-03 |
| 32. Авто-админ по логинам + релиз SSO в main | v1.3 | 5/5 | Complete    | 2026-08-04 |
| 33. Полировка предпросмотра печати | v1.3 | 4/4 | Complete   | 2026-08-04 |

## Phase Details

Детализация фаз 31–33 перенесена в архив вехи: `milestones/v1.3-ROADMAP.md`.
Требования — `milestones/v1.3-REQUIREMENTS.md`, аудит — `milestones/v1.3-MILESTONE-AUDIT.md`.
