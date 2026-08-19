# Milestones: Trackly

## v1.3.3 — Печатные формы и приватность данных (Shipped: 2026-08-19)

**Phases:** 34–38 (5 фаз, 23 плана, 61 задача)
**Git range:** `feat(34-01)` → `docs(36)` (196 коммитов, 181 файл, +31461/−764 строк по диапазону `v1.3..v1.3.3`)
**Timeline:** 2026-08-09 → 2026-08-19 (~11 дней)

**Goal:** привести акты приёма-передачи к каноничному виду с единой шапкой и корректной
многостраничностью, и закрыть утечку реальных данных организации и сотрудников в публичный
репозиторий.

**Delivered:**

- **Phase 34 — Единая шапка документов (6 планов):** общий Jinja-partial `_header.html`,
  подключённый во все три печатные формы через регистрацию в in-memory Environment
  (не файловый загрузчик) — DOC-04 гарантирован структурно. Новое поле
  `org_settings.full_name` («Полное юридическое наименование») сквозь БД/DTO/сервис + UI-поле
  в Настройках → Организация; экранирующий хелпер `org_full_name_html`. Механизм доставки
  шапки в уже установленные копии — срез `_legacy_defaults/v21` + `KNOWN_LEGACY_DEFAULTS`
  с fail-closed веткой и `tracing::warn!` на пропущенном апгрейде. Read-only эндпоинт
  `templates_status` под `ManageSettings` на обоих транспортах. (DOC-04, DOC-05, DOC-06)

- **Phase 35 — Тело акта приёма-передачи (7 планов, 2 раунда gap-closure):** канонический
  текст акта согласован с пользователем ДО вёрстки; убраны полоски-подчёркивания под
  автоподставляемыми значениями (остались только там, где расписываются от руки), метка+значение
  переведены в связный текст, добавлена форма множественного числа при N>1, «Сроком до» рендерится
  безусловно. Двухстрочная сетка подписей заменена горизонтальным блоком — строка на подписанта
  с автоподставленными ФИО, симметрично в обоих актах. Длинное кириллическое ФИО переносится
  внутри строки подписи вместо ухода за печатную область. Появился durable-гейт подчёркиваний
  по CSS-селектору. (DOC-07, DOC-08, DOC-09)

- **Phase 36 — Пагинация акта по количеству устройств (6 планов):** акт на одно устройство
  рендерится байт-идентично потоку Фазы 35; акт на несколько выводит на первом листе только
  нумерованный перечень имён, а полное описание уезжает в «Приложение №1» через принудительный
  разрыв — таблица с группировкой tbody-на-устройство, зеброй, прочерками и колонкой количества.
  Собственный Paged.js `afterPageLayout` Handler повторяет `<thead>` приложения на каждом листе
  на обоих путях рендера. Rust-агрегация `group_items_for_print()` устранила дублирование
  позиций с quantity>1, найденное живым UAT. (DOC-10, DOC-11)

- **Phase 37 — Приватность данных (4 плана):** реальные реквизиты организации, ФИО сотрудников
  и AD/DC-идентификаторы вычищены из HEAD в 14 файлах; удалены бинарные reference-артефакты и
  самый плотный бриф-файл утечки, 14 висячих ссылок переписаны в прозу. Построен
  `scripts/check-privacy.mjs` — zero-dependency Node ESM-гейт с allowlist-режимом, SHA-256
  n-грамм-хэшами и контролем бинарных расширений, с 6-фикстурным selftest'ом. Гейт включён
  в pre-commit хук и в `ci-fast`; боевой список хэшей (19 записей, только хэши) проверен на
  полноту против всех 18 доцистых блобов, ещё достижимых в истории. (PRIV-01, PRIV-02)

- **Phase 38 — Nyquist-покрытие Фазы 32 (0 планов):** унаследованный из v1.3 долг закрыт
  ретроактивным аудитом `/gsd-validate-phase 32` — `32-VALIDATION.md`: `nyquist_compliant: true`,
  0 найденных пробелов, 9/9 строк карты перепроверены зелёными прямым прогоном тестов. (QA-04)

**Requirements:** 11/11 satisfied (DOC-04..DOC-11, PRIV-01, PRIV-02, QA-04). Сирот нет.

**Закрыто в ходе аудита вехи:**

- **INT-01** (blocker, 2026-08-18) — CI не исполнял собственные гейты почти всю веху:
  `cargo fmt --check` стоял первым шагом и падал с 2026-08-11, скипая clippy / cargo test /
  svelte-check / pnpm lint. За fmt-дрейфом пряталась вторая поломка — clippy на тестовом
  конструкторе из плана 36-06. Закрыто коммитами `b26f6173` + `b4a7dc52`; прогон `ci-fast`
  на `b4a7dc52` зелёный целиком, включая `cargo test --workspace`. Первый зелёный CI с 2026-08-08.
- **DOC-10** (2026-08-19) — живая печать N=1 подтверждена на Windows/WebView2 из релизной
  сборки v1.3.3. Тем же прогоном закрыты печать при дефолтных настройках диалога, живой
  LAN-транспорт и изоляция печатного DOM. Дефектов не выявлено, правок кода не потребовалось.
- **QA-04** (2026-08-19, при закрытии вехи) — создан `phases/38-nyquist-phase32/38-VERIFICATION.md`
  (passed 2/2), трассируемость приведена к однородной с остальными четырьмя фазами.

**Технический долг, принятый осознанно:**

- `36-VALIDATION.md`: `nyquist_compliant: false` — единственная фаза вехи без подтверждённого
  Nyquist-покрытия. Кандидат на `/gsd-validate-phase 36`.
- **INT-02** (warning) — `RepeatTableHeadHandler` дублирован между десктопным и LAN-путём
  без общего источника и гейта синхронности; правка одной копии молча регрессирует другой транспорт.
- **PRIV-03** — история git не переписана (решение пользователя 2026-08-08); репозиторий
  публичный, гарантия гейта распространяется только на HEAD. Остаточный риск принят.
- Пре-существующий хвост: `cargo test --workspace` виснет на `login_remember_persistent_cookie`
  локально на macOS — полный прогон не является дешёвой проверкой (в CI на Linux проходит).

**Known deferred items at close:** 53 (см. STATE.md → Deferred Items). Ни одного из фаз 34–38:
29 quick-тасок без поля `status` во frontmatter (артефакт сканера), 8 VERIFICATION со статусом
`human_needed` из фаз 03–24 (вехи v1.0/v1.2), 14 UAT-маркеров фаз 03.1–34, 1 debug-сессия,
1 todo.

**Release tag:** `v1.3.3` создан 2026-08-18 на `fc9fe1b8` как релизный тег (три сегмента →
триггерит `release.yml`); именно эта сборка использовалась для живого UAT на Windows. Отдельный
milestone-тег не создавался, чтобы не двигать уже опубликованный тег.

**Audit:** `milestones/v1.3.3-MILESTONE-AUDIT.md` — статус `tech_debt`, requirements 11/11,
phases 5/5, integration 12/12, flows 2/2.

---

## v1.3 AD-SSO паритет + полировка превью печати (Shipped: 2026-08-08)

**Phases completed:** 3 phases, 13 plans, 20 tasks

**Key accomplishments:**

- Defined the ldap3-free `AdDirectory` port (displayName + AD-group role resolve) plus its two dependency-free building blocks — a deterministic `MockAdDirectory` and a generic hand-rolled `TtlCache<V>` — all unit-tested and compiling in isolation ahead of the real LDAP adapter (Plan 31-02).
- Implemented the real LDAP service-account directory adapter — fixed-account bind, `sAMAccountName`→`displayName` resolve with cache, and `LDAP_MATCHING_RULE_IN_CHAIN` group→role mapping with fail-closed 3-state error handling — plus the `AdConfig` schema (redacting `Debug` for the bind password) and a refreshed placeholder config example.
- `AuthService.sso_login` now resolves real displayName + AD-group role via the injected `AdDirectory` before provisioning, fail-closed on any directory error; both hardcoded `'employee'` SQL literals are replaced with `Role::as_str()`-derived values; all 8 `AuthService::new` call sites compile against the new 6-parameter signature.
- Added `ad_directory_sso.rs`, a 7-test end-to-end suite that drives the real `AuthService.sso_login` → `AdDirectory` → role-mapped `UserDto` path, proving both SSO-01 (real ФИО) and SSO-03 (auto role from group, fail-closed) against the deterministic mock directory — the phase's acceptance proof — and ran the full-workspace verification gate green.
- AdConfig.admin_logins: Vec<String> field added (config/parsing layer only) — deployment-time TOML source of truth for the auto-admin-by-login-list feature (SSO-02), mirroring the existing role_mapping pattern.
- 5-branch forced-admin state machine wired into `on_ad_bind_success` (both SSO-passwordless and LDAPS-bind entry points), promoting any deployment-configured `admin_logins` entry to an active Administrator with a mandatory in-transaction audit trail — solves the "first administrator" problem for AD-only orgs.
- Pinned `pagedjs` 0.4.3 dependency plus four new `ui/src/lib/` modules (bootstrap script, srcdoc builder, opaque-origin postMessage bridge, RU pluralization helper) establishing the frozen interface that Plans 33-02/33-03/33-04 build on — nothing wired into `PdfPreviewModal.svelte` yet.
- Adds exactly one `sha256-<digest>'` hash-source to the LAN-mode axum CSP's `script-src` directive so the frozen Paged.js bootstrap script (Plan 33-01) can execute inside the preview `<iframe srcdoc>` in server mode, with an independent Node-side drift-detection gate wired into `pnpm lint`; also lands a structural regression test guarding that all three print templates declare identical `@page` blocks (D-13).
- `PdfPreviewModal.svelte` now renders a real paginated A4 sheet stack via Paged.js — themed backdrop, no-border shadowed sheets, fit-to-width scaling, footer page counter/hint, and an 8-second degrade-to-unpaginated fallback — consuming Plan 33-01's frozen srcdoc/bridge contract, plus a Rule-3 fix for a pagedjs deep-import resolution bug surfaced by wiring it in.
- Both print branches of `PdfPreviewModal.svelte` (desktop temp-file and LAN top-level injection) now print the same Paged.js-paginated output as the on-screen preview, closing the WYSIWYG gap (PRV-03, D-06) by waiting for pagination instead of the `load` event or synchronous DOM injection.

---

## v1.2 Редизайн UI и дизайн-система (Shipped: 2026-07-29)

**Phases completed:** 1 phases, 9 plans, 22 tasks

**Key accomplishments:**

- Two new zero-dependency lint scripts (check-contrast.mjs, check-focus-outline.mjs) wired into `pnpm lint`, plus 4 corrected `_tokens.scss` color values that now pass WCAG AA in both light and dark themes.
- Three targeted CSS fixes closing the last check-focus-outline.mjs violation and two ancestor-overflow ring-clipping defects, using idioms already established elsewhere in the codebase — zero markup/logic changes.
- Plan:
- Search-panel auto-focus + ArrowLeft drill-in exit close Gap 3 (search input unreachable by keyboard) and Gap 5 (drill-in trap) in Dropdown.svelte, with zero architectural changes.
- Consolidated 4 duplicated cell-level focus-ring box-shadows into one shared `.tr-row:has(:focus-visible)` rule in TableRow.svelte, closing Gap 4 (focus ring drawn around first cell instead of the whole row, inconsistent across tables) without adding any new row interactivity.
- Fixed clipped focus ring on PeriodToggle (inset idiom) and app-shell-wide scroll bleed on DashboardPage (min-height:0) — two independent Gap 1/Gap 2 defects on the same screen.
- PeriodToggle padding 2px→8px горизонтально закрывает Gap 1 (кольцо больше не прилипает к тексту); Layout.svelte grid-template-rows: minmax(0, 1fr) + .content flex-column закрывает Gap 2 на корневом layout-уровне — root cause был не в DashboardPage.svelte (30-06 чинил не тот файл), а в отсутствии явного grid-template-rows на .app-layout, что заставляло единственную implicit-строку сайзиться по max-content вместо доступного 100vh.
- Removed the always-visible focus ring on Dropdown's in-panel search input and added a client-side substring filter (`visibleGroups`) that closes the "zero filtering" gap for all 11 existing flat+select+searchable consumers with one change in `Dropdown.svelte`.
- TableRow.svelte row-wide focus ring now excludes `.tr-row-group` (no more duplicate ring over the chevron's own ring) and the chevron ring is rounded; Table.svelte's `.tr-table-wrapper` gained a 2px padding safety margin against a WebKit inset-shadow clip artifact in Printers' master-detail list.

---

## v1.1.2 — Пост-релизные доработки UX и печати

**Shipped:** 2026-07-15
**Phases:** 18–22 (5 фаз, 28 планов)
**Git range:** `feat(18-01)` → milestone close (2026-07-10 → 2026-07-15)

Источник: 6 подтверждённых пользователем замечаний после релиза v1.1.1, развёрнутые в 12 требований (AUTO-01..05, ACT-01/02/03, PRN-01, ORG-01/02, CRT-01) по 5 фазам. Все 12 — satisfied.

**Key accomplishments:**

- **Фаза 18 (AUTO-01..05) — Автокомплит и дропдауны.** Переиспользуемый `use:dropdownAnchor` + `use:portal`: дропдауны выходят за пределы overflow-контейнеров (модалка акта / таблица), репозиционируются на capture-scroll/resize, флипаются вверх у нижней кромки. Device-picker с focus-open (мгновенная выдача топ-20 групп по остатку), реальная многополевая FTS-фильтрация (name/inventory_no/serial_no/model), группировка по name+model с сортировкой count DESC. Нативные `<select>`-обёртки оставлены на браузерном popup.
- **Фаза 19 (ACT-01/02) — Акты: дата и редактирование.** Точная дата передачи (`handover_date_utc`) + полноценная правка handover-акта: шапка + дельта позиций (add/remove со сменой состояния устройства), оптимистическая блокировка (CAS `WHERE version=?`), пересчёт `archived` в транзакции, каскад переименования номера на дочерние возвраты, аудит правок комплектации.
- **Фаза 20 (PRN-01/ORG-01/ORG-02) — Печать актов и реквизиты организации.** Поле `address_line2`, паритет HTML-шаблонов для handover и acceptance, авто-апгрейд нетронутых дефолтных шаблонов на старте (не затирая пользовательские правки).
- **Фаза 21 (CRT-01) — Коды картриджей/фотобарабанов.** Поля кодов на моделях расходников.
- **Фаза 22 (ACT-03) — Правка возвратов.** Диалог правки ReturnModal с полным предзаполнением из собственных данных возврата, дельта-пересборка состояния устройств, guard'ы D-10 (пустой набор) / D-11 (device-drift), новый мутирующий путь `acts_update_return` с RBAC-паритетом на обоих транспортах.

**Quality gates (закрыто на этапе close, 2026-07-15):**

- UAT: Фаза 19 — 7/7 passed; Фаза 22 — 2/2 passed (live).
- Security: `19-SECURITY.md` (26/26 threats closed), `22-SECURITY.md` (20/20 closed, тесты прогнаны). threats_open: 0.
- Nyquist: Фаза 22 — nyquist_compliant (18/18 тестов green); Фаза 18 — backend automated (AUTO-03/04/05), UI manual-only (нет FE-харнесса по соглашению проекта).

**Known deferred items at close:** 5 (see STATE.md → Deferred Items) — Фаза 18 SECURITY.md, 5 Info code-review findings (Фаза 18), 3 defense-in-depth WARNINGs (Фаза 20, раскрыты), отсутствие HTTP role-matrix кейса для settings_save_org_fields, историческая docs-опечатка «11 vs 12».

---

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
