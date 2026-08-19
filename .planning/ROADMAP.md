# Roadmap: Trackly

## Milestones

- ✅ **v1.0 — Базовый учёт** (shipped 2026-06-19) — `MILESTONES.md`
- ✅ **v1.1 — AD, сотрудники, картриджная взаимосвязь** (shipped 2026-06-26) — `milestones/v1.1-*`
- ✅ **v1.1.2 — Пост-релизные доработки UX и печати** (Фазы 18–22, shipped 2026-07-15) — `milestones/v1.1.2-*`
- ✅ **v1.2 — Редизайн UI и дизайн-система** (Фазы 23–30, shipped 2026-07-29) — `milestones/v1.2-*`
- ✅ **v1.3 — AD-SSO паритет + полировка превью печати** (Фазы 31–33, shipped 2026-08-08) — `milestones/v1.3-*`
- ✅ **v1.3.3 — Печатные формы и приватность данных** (Фазы 34–38, shipped 2026-08-19) — `milestones/v1.3.3-*`
- 📋 **Следующая веха** — не определена (`/gsd-new-milestone`)

## Phases

<details>
<summary>✅ v1.3.3 Печатные формы и приватность данных (Фазы 34–38) — SHIPPED 2026-08-19</summary>

- [x] Phase 34: Единая шапка документов (6/6 plans) — completed 2026-08-11
- [x] Phase 35: Тело акта приёма-передачи (7/7 plans) — completed 2026-08-12
- [x] Phase 36: Пагинация акта по количеству устройств (6/6 plans) — completed 2026-08-13
- [x] Phase 37: Приватность данных (4/4 plans) — completed 2026-08-18
- [x] Phase 38: Nyquist-покрытие Фазы 32 (0 plans) — completed 2026-08-18

Полная детализация — `milestones/v1.3.3-ROADMAP.md`; требования — `milestones/v1.3.3-REQUIREMENTS.md`;
аудит — `milestones/v1.3.3-MILESTONE-AUDIT.md` (11/11 требований, 5/5 фаз, tech_debt, без блокеров).

Живой UAT печати выполнен на Windows 2026-08-19 из релизной сборки `v1.3.3` — дефектов нет.

</details>

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

### 📋 Следующая веха

Не определена. Запустить `/gsd-new-milestone` — questioning → research → requirements → roadmap.

Кандидаты в объём, унаследованные из v1.3.3 (см. `milestones/v1.3.3-MILESTONE-AUDIT.md`):

- `/gsd-validate-phase 36` — единственная фаза вехи без подтверждённого Nyquist-покрытия.
- **INT-02** — вынести `RepeatTableHeadHandler` в общий источник + гейт синхронности
  десктопного и LAN-путей печати.

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
| ----- | --------- | --------------- | ------ | --------- |
| 23–29 | v1.2 | — | Complete | (см. архив) |
| 30. Quality — a11y & parity | v1.2 | 9/9 | Complete | 2026-07-29 |
| 31. Служебный AD-bind — ФИО и роли из AD-групп | v1.3 | 4/4 | Complete | 2026-08-03 |
| 32. Авто-админ по логинам + релиз SSO в main | v1.3 | 5/5 | Complete | 2026-08-04 |
| 33. Полировка предпросмотра печати | v1.3 | 4/4 | Complete | 2026-08-04 |
| 34. Единая шапка документов | v1.3.3 | 6/6 | Complete | 2026-08-11 |
| 35. Тело акта приёма-передачи | v1.3.3 | 7/7 | Complete | 2026-08-12 |
| 36. Пагинация акта по количеству устройств | v1.3.3 | 6/6 | Complete | 2026-08-13 |
| 37. Приватность данных | v1.3.3 | 4/4 | Complete | 2026-08-18 |
| 38. Nyquist-покрытие Фазы 32 | v1.3.3 | 0/0 | Complete | 2026-08-18 |

## Phase Details

Детализация фаз 31–33 — `milestones/v1.3-ROADMAP.md`.
Детализация фаз 34–38 — `milestones/v1.3.3-ROADMAP.md`.

## Backlog

- **999.1 — role-based route gating** — UX-полировка: гейт маршрута ≠ гейт меню для admin-vs-manager
  (реальная граница безопасности — backend 403). Отложено из v1.2.
  Каталог: `phases/999.1-role-based-route-gating/`.

- **DOC-12 — пользовательский редактор печатных форм в UI** — отложено из v1.3.3 (Future
  Requirements); сейчас шаблоны правятся файлами в `templates/`.

- **DOC-13 — единая шапка распространяется на будущие печатные формы** — отложено из v1.3.3, если
  появятся новые формы за пределами трёх текущих.

- **PRIV-03 — очистка утёкших данных из истории git** — отложено из v1.3.3 (решение пользователя
  2026-08-08: чистим только HEAD, история не переписывается; PRIV-03 остаётся опцией на будущее).
