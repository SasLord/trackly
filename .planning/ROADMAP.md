# Roadmap: Trackly

## Milestones

- ✅ **v1.0 — Базовый учёт** (shipped 2026-06-19) — `MILESTONES.md`
- ✅ **v1.1 — AD, сотрудники, картриджная взаимосвязь** (shipped 2026-06-26) — `milestones/v1.1-*`
- ✅ **v1.1.2 — Пост-релизные доработки UX и печати** (Фазы 18–22, shipped 2026-07-15) — `milestones/v1.1.2-*`
- ✅ **v1.2 — Редизайн UI и дизайн-система** (Фазы 23–30, shipped 2026-07-29) — `milestones/v1.2-*`
- 🚧 **v1.3 — AD-SSO паритет + полировка превью печати** — Фазы 31–33 (planning)

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

**v1.3 — AD-SSO паритет + полировка превью печати — ACTIVE**

- [x] **Phase 31: Служебный AD-bind — ФИО и роли из AD-групп** - SSO-пользователи отображаются по реальному ФИО (service-account LDAP bind, с кэшем) и автоматически получают роль по членству в AD-группе. (completed 2026-08-03)
- [x] **Phase 32: Авто-админ по списку логинов + релиз SSO в main** - Указанные доменные логины получают роль «Администратор» сразу при первом SSO-входе; спайк-ветка выходит в основной релиз. (completed 2026-08-04)
- [ ] **Phase 33: Полировка предпросмотра печати** - Модалка предпросмотра (Акты/Приёмка/Отчёты) показывает лист A4 на сероватой подложке с полями, WYSIWYG-совпадение с реальной печатью.

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
| 33. Полировка предпросмотра печати | v1.3 | 1/4 | In Progress|  |

## Phase Details

### Phase 31: Служебный AD-bind — ФИО и роли из AD-групп

**Goal**: Пользователи, входящие через AD-SSO (Kerberos/SPNEGO), видят своё реальное ФИО и автоматически получают корректную роль на основании членства в AD-группе — без обращения к пользовательским учётным данным (их у passwordless-SSO нет).

**Depends on**: Phase 30 (предыдущий milestone, завершён)

**Milestone**: v1.3 — AD-SSO паритет + полировка превью печати

**Requirements**: SSO-01, SSO-03

**Success Criteria** (what must be TRUE):

1. Пользователь, вошедший через AD-SSO, отображается в интерфейсе под реальным ФИО (AD `displayName`, fallback `cn` → логин), а не под доменным логином.
2. Повторные SSO-входы того же пользователя используют закэшированный результат резолва ФИО/группы (в пределах TTL), не выполняя новый LDAP-запрос к DC на каждый вход.
3. Пользователь, входящий через SSO и состоящий в настроенной AD-группе, автоматически получает соответствующую роль (Администратор/Менеджер/Сотрудник) без ручного подтверждения заявки администратором.
4. При недоступности AD-каталога в момент проверки членства в группе система ведёт себя fail-closed — роль не повышается по умолчанию, пользователь остаётся в обычном пути (pending/Сотрудник), ошибка не приводит к тихому провалу авторизации.
5. Приватность: домен, служебная учётная запись и её параметры bind читаются из gitignored `trackly.config.toml`; в git (тесты, фикстуры, конфиг-примеры) — только плейсхолдеры, никаких реальных ФИО/логинов/имени домена организации.

**Plans**:
**Wave 1**

- [x] 31-01-PLAN.md — AdDirectory port contract + MockAdDirectory fixtures + TtlCache primitive (Wave 1)

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 31-02-PLAN.md — AdConfig service-bind/group-mapping fields + RealAdDirectory (service-account bind, memberOf/LDAP_MATCHING_RULE_IN_CHAIN group check) (Wave 2)

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 31-03-PLAN.md — Wire AdDirectory into AuthService.sso_login + role threading + context.rs mock/real selection (Wave 3)

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 31-04-PLAN.md — End-to-end SSO-01/SSO-03 integration tests + full workspace verification gate (Wave 4)

---

### Phase 32: Авто-админ по списку логинов + релиз SSO в main

**Goal**: Администратор может назначить доверенные доменные логины, которые получают роль «Администратор» автоматически при первом SSO-входе (решая проблему «первого администратора»); SSO-функциональность выходит из спайкового статуса в основную ветку релиза.

**Depends on**: Phase 31 (тот же провижининг-путь `on_ad_bind_success`, расширяется этой фазой)

**Milestone**: v1.3 — AD-SSO паритет + полировка превью печати

**Requirements**: SSO-02

**Success Criteria** (what must be TRUE):

1. Администратор может задать список доменных логинов (аналог `ADMIN_AD_LOGINS`), которые при первом SSO-входе сразу получают роль «Администратор».
2. Логин из этого списка при первом SSO-входе становится активным пользователем с ролью Администратор немедленно — без промежуточной заявки на подтверждение.
3. Логин, отсутствующий в списке, проходит прежний путь провижининга (авто-регистрация / заявка на подтверждение / маппинг по AD-группе из Phase 31) — список не расширяет доступ никому, кроме явно перечисленных логинов.

**Plans**:
**Wave 1**

- [x] 32-01-PLAN.md — AdConfig.admin_logins config field + parsing tests + trackly.config.toml.example docs (Wave 1)

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 32-02-PLAN.md — AuthService admin_logins builder/normalization + forced-admin state machine + context.rs wiring (Wave 2)

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 32-03-PLAN.md — Full SSO-02 state-matrix integration tests (unknown/pending/blocked/escalation/idempotent/not-in-list/directory-unreachable) (Wave 3)

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 32-04-PLAN.md — Fix workspace cargo fmt drift + full verification gate + open PR for ci-full dry-run (Wave 4)

**Wave 5** *(blocked on Wave 4 completion, human-gated)*

- [x] 32-05-PLAN.md — Human-approved merge spike/ad-sso-kerberos → main + push v1.3.0 release tag (Wave 5, autonomous: false)

**Note** (не привязано к отдельному REQ-ID, операционный итог фазы/milestone): ветка `spike/ad-sso-kerberos` мержится в `main`, версия выходит из спайковой линейки `0.0.x` в обычный релизный тег.

---

### Phase 33: Полировка предпросмотра печати

**Goal**: Предпросмотр печати документов (Акты, Приёмка/DocumentAcceptance, Отчёты) выглядит как «вордовский» предпросмотр — лист A4 на подложке с полями — и один-в-один совпадает с тем, что уходит на печать.

**Depends on**: Phase 30 (предыдущий milestone; независима от Phase 31/32 — чистый фронтенд/CSS поверх уже HTML-шаблонов, может вестись параллельно с SSO-фазами)

**Milestone**: v1.3 — AD-SSO паритет + полировка превью печати

**Requirements**: PRV-01, PRV-02, PRV-03

**Success Criteria** (what must be TRUE):

1. В модалке предпросмотра документ отображается как лист формата A4, лежащий над визуально отделённой сероватой подложкой-фоном (а не растянутый на весь белый фон модалки).
2. Лист предпросмотра имеет видимые внутренние поля (margins) сверху, снизу и по краям, соответствующие реальным полям печати документа.
3. То, что пользователь видит в модалке предпросмотра, совпадает (WYSIWYG) с тем, что выводится на печать через `@media print` — единый источник стилей листа для экрана и печати, без расхождений в масштабе/отступах.
4. Поведение из критериев 1–3 одинаково для всех документов, использующих общую модалку предпросмотра (Акты, Приёмка, Отчёты) — не только для одного типа документа.

**Plans**:
**Wave 1**

- [x] 33-01-PLAN.md — Paged.js dependency + srcdoc/bridge/pluralization contract (Wave 1)

**Wave 2** *(blocked on Wave 1 completion)*

- [ ] 33-02-PLAN.md — CSP hash-source for Paged.js bootstrap + drift-detection + @page parity test (D-13, D-14) (Wave 2)
- [ ] 33-03-PLAN.md — On-screen preview: srcdoc/bridge wiring, degraded fallback, sheet chrome, fit-to-width, footer meta (Wave 2)

**Wave 3** *(blocked on Wave 2 completion — 33-04 depends on 33-03, same file)*

- [ ] 33-04-PLAN.md — Print paths (desktop + LAN) through Paged.js (D-06) (Wave 3)

**UI hint**: yes
