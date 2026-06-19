# Phase 9: AD-аутентификация и заявки на регистрацию пользователей - Context

**Gathered:** 2026-06-19
**Status:** Ready for planning
**Source:** /gsd-discuss-phase 9 — 4 области (AD-bind стратегия, UX логин-формы, заявка на регистрацию+approval, AD-настройки+автоприём+mock) обсуждены интерактивно.

<domain>
## Phase Boundary

Включить вход доменных пользователей через Active Directory из браузера в LAN: ввод
AD-логина/пароля → `ldap3 simple_bind` (пароль НИКОГДА не сохраняется), подтягивание
ФИО из AD, заявки на регистрацию незарегистрированных AD-пользователей с подтверждением
администратором и опциональным автоприёмом, плюс mock AD-клиента для разработки на macOS.

Покрывает: **USR-08, USR-09, USR-10, USR-11, USR-12, REQ-06, SET-10**.

Фаза вынесена из Phase 8 при SPIDR-split 2026-06-18: release-пайплайн идёт ПЕРЕД AD-фазой,
чтобы Windows-сборку поставить на реальный Win10 x64 в домене для теста AD-входа.

**В scope:**
- **AD simple_bind (USR-08):** веб-логин по AD-логину (`us100` / `user@domain` / `DOMAIN\user`) + AD-паролю через `ldap3 0.12` `simple_bind` по LDAPS. Пароль используется только для bind, не пишется в БД (`Secret<T>`).
- **ФИО из AD (USR-10):** после успешного bind тянуть `displayName` (fallback → `cn` → логин); имя атрибута настраиваемое.
- **Заявка на регистрацию (USR-09, REQ-06):** незарегистрированный AD-юзер после успешного bind → автоматически создаётся заявка `request_type='ad_register'`, видимая ТОЛЬКО администратору; админ подтверждает и назначает роль (дефолт «Сотрудник»).
- **Два режима регистрации (USR-11, SET-10):** переключатель в Настройках:
  - **Авто-регистрация (автоприём ON):** AD-юзер сразу создаётся по доменным данным с ролью «Сотрудник»; админу создаётся информационная заявка с возможностью «Отклонить» (отклонение = удаление юзера).
  - **Pending (автоприём OFF):** AD-юзер видит pending-экран «Заявка отправлена, ждите подтверждения»; в систему не пускается до approve.
- **Flow восстановления доступа (расширение USR-09, выбрано в scope):** удалённый/заблокированный AD-юзер видит экран с сообщением + кнопками «Запрос на восстановление доступа» (новая заявка админу; approve → юзер восстанавливается) и «Войти под другим пользователем» (escape-hatch на локальный логин — например зайти Администратором рядом и принять заявку).
- **AD-настройки (новая вкладка Настройки → «Active Directory»):** тумблер «Использовать AD» + переключатель режима регистрации (автоприём). Подключение к AD — **auto-detect-first** (см. D-Config-01), ручные поля под «Расширенные».
- **Mock AD-клиента (USR-12):** `trait AdClient` + `RealAdClient` (ldap3) + `MockAdClient` по образцу `SnmpClient`/`MockSnmpClient`; switch через `config.ad.use_mock` / env `TRACKLY_AD_MOCK`. Фикстуры: пара тестовых доменных юзеров + сценарии success / неверный пароль / юзер не найден / сервер недоступен.
- **Инструкция по настройке AD:** короткий doc для администратора (что включить, что делать если auto-detect не сработал) — deliverable фазы.

**НЕ в scope этой фазы (явно deferred):**
- **Auto-SSO / автоопределение доменного пользователя** (кнопка «Войти как \<display name\>» без ввода пароля) — v2. Полный Kerberos/NTLM Negotiate SSO = ADV-01 (v2). В v1 архитектура (`trait AdClient`) оставляет место под SSO, но он не реализуется.
- AD-вход в десктоп-режиме — v1 только веб (USR-08 «через браузер»); десктоп остаётся trusted-admin / локальный логин как в Phase 5.
- SMTP/email-уведомления о заявках — финальная фаза v2 (in-app уведомления уже есть, REQ-04 Phase 6).

**Mode:** mvp — вертикальный слайс: логин-форма (AD-поле) → axum auth handler + AdClient bind → AuthService (ad_login + регистрация/автоприём) → users/requests repo → DB.

</domain>

<decisions>
## Implementation Decisions

### AD bind стратегия

#### D-AD-01: v1 = simple_bind, SSO отложен в v2
- v1 реализует **только** `ldap3 simple_bind` по введённым AD-логину/паролю в браузере. Соответствует USR-08 буквально.
- Полный auto-SSO (Kerberos/NTLM Negotiate, вход без пароля) — **v2** (ADV-01). Не реализуем сейчас: Windows-only, поднимает MSRV, не собирается на macOS-дев-боксе без Kerberos-библиотек.
- Архитектура: `trait AdClient` оставляет место под будущий SSO-адаптер без переделки сервис-слоя.
- **Разрешает противоречие памяти проекта** `phase8_split_ad_sso` («AD должен делать auto-SSO») — решение пользователя: auto-SSO переносится в v2, v1 = simple_bind. Память обновить.

### UX логин-формы (веб)

#### D-UX-01: единая форма логин/пароль с auto-fallback local→AD
- Блок «Логин / Пароль / Войти»: сервер сначала пробует локального юзера (argon2id), если не найден — AD `simple_bind` (когда AD включён). Один и тот же блок обслуживает локальных юзеров (Администратор/Специалист/Сотрудник) И доменных, вводящих credentials.
- Нетех-сотруднику не нужно выбирать тип входа.

#### D-UX-02: «Запомнить меня» — persistent vs session cookie
- Чекбокс «Запомнить меня»: если включён — persistent-сессия (sliding 30 дней, как D-Session-01 Phase 5); если выключен — сессия живёт до закрытия браузера.
- При следующем визите валидная сессия → авто-вход без формы. После logout → снова форма логин/пароль.

#### D-UX-03: кнопка «Войти как \<display name\>» — v2
- Авто-определение текущего доменного пользователя (one-click без пароля) НЕ реализуется в v1 (часть auto-SSO, см. D-AD-01). В вёрстке формы можно зарезервировать место, но без логики.

### Заявка на регистрацию и approval

#### D-REG-01: два режима регистрации, переключаемые админом в Настройках (SET-10/USR-11)
- **Авто-регистрация (автоприём ON):** успешный AD-bind незнакомого юзера → сразу создаётся строка `users` (`ad_user=1`, `password_hash=NULL`, role='employee', ФИО из AD) + информационная заявка `ad_register` админу с действием «Отклонить» (отклонение = soft-delete юзера).
- **Pending (автоприём OFF):** создаётся заявка `ad_register`, юзер видит pending-экран и не пускается в систему до approve. Approve → создаётся/активируется юзер с выбранной ролью.

#### D-REG-02: approval — в разделе «Заявки», видимость только админу (REQ-06)
- `ad_register` — подтип заявки (схема V006 уже допускает `request_type='ad_register'`), видим ТОЛЬКО роли admin. Переиспользует существующий lifecycle заявок (Phase 6).
- При approve — модалка с выбором роли. Дефолт — «Сотрудник» (employee), совпадает с автоприёмом.

#### D-REG-03: flow восстановления доступа (в scope Phase 9)
- Удалённый/заблокированный AD-юзер при попытке входа видит экран: сообщение о блокировке + «Запрос на восстановление доступа» (новая заявка админу; approve → восстановление) + «Войти под другим пользователем» (показать обычную форму логин/пароль).
- **Решение для планировщика:** restoration-request — отдельный flavor. Уточнить: переиспользовать `ad_register` с под-флагом vs новый `request_type` (миграция V0xx + расширение CHECK). Рекомендация — под-флаг, чтобы не плодить типы.

### AD-настройки, подключение, mock

#### D-Config-01: подключение к AD — auto-detect-first, ручное под «Расширенные» (Claude's Discretion)
- Пользователь не хочет настраивать LDAP-детали вручную. Подход: на доменной Windows-машине авто-определять домен и контроллер домена (env `USERDNSDOMAIN` + DNS SRV `_ldap._tcp.dc._msdcs.<domain>`), base DN выводить из домена (`corp.local` → `dc=corp,dc=local`). Дефолты: LDAPS, атрибут ФИО `displayName`.
- Админ в простом случае только включает тумблер «Использовать Active Directory». Ручные поля (host:port, domain-суффикс, base DN, атрибут ФИО, LDAPS on/off) — спрятаны в «Расширенные» на случай нестандартной сети.
- **Конкретный набор полей и механизм auto-detect — на усмотрение планировщика/research.** Обязателен deliverable: краткая инструкция по настройке AD для администратора.

#### D-Config-02: ФИО атрибут — displayName → cn → login
- Тянуть `displayName`; если пусто — `cn`; если оба пусты — логин. Имя атрибута настраиваемое (дефолт `displayName`).

#### D-Mock-01: mock AD по образцу SNMP + сценарии ошибок (USR-12)
- `trait AdClient` (в trackly-core) + `RealAdClient` (ldap3, trackly-infra) + `MockAdClient` (trackly-infra). Runtime-switch в `AppCtx::build` через `config.ad.use_mock || env TRACKLY_AD_MOCK` — точно как `SnmpClient` (`crates/trackly-infra/src/snmp/mod.rs`).
- Фикстуры: пара тестовых доменных юзеров (логин/пароль/ФИО) + детерминированные сценарии: success, неверный пароль, юзер не найден, AD-сервер недоступен.

#### D-Sec-01: AD-пароль никогда не сохраняется
- AD-пароль оборачивается в `Secret<T>`, используется только для `simple_bind`, после — drop (zeroize). В БД для AD-юзеров `password_hash=NULL` (схема V002 уже это допускает). Требование CLAUDE.md «Безопасность».

### Claude's Discretion
- Точный набор AD-настроек и механизм auto-detect (DNS SRV / env / ручной override) — планировщику/research (D-Config-01).
- Восстановление: под-флаг `ad_register` vs новый `request_type` (D-REG-03).
- Вёрстка/копирайт логин-формы, pending-экрана, blocked-экрана — в рамках UI-SPEC паттернов.
- Формат и место инструкции по настройке AD (отдельный `docs/AD-SETUP.md` vs раздел README).
- LDAPS vs LDAP по умолчанию и обработка self-signed AD-сертификатов в LAN.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Roadmap & requirements
- `.planning/ROADMAP.md` §«Phase 9: AD-аутентификация и заявки на регистрацию пользователей» — goal, requirements, depends-on Phase 8.
- `.planning/REQUIREMENTS.md` — USR-08 (стр. 122), USR-09 (123), USR-10 (124), USR-11 (125), USR-12 (126), REQ-06 (99), SET-10 (147). ADV-01 (стр. 214) — полный SSO в v2 (граница scope).

### Существующая схема (Phase 1/5/6, не пере-создавать)
- `migrations/V002__core_entities.sql` — таблица `users`: `ad_user` (0/1), `password_hash NULL` для AD bind-only, `role` ('admin'|'manager'|'employee'), standard4-колонки + `version`. AD-юзеры заводятся здесь.
- `migrations/V006__requests.sql` — `requests.request_type` CHECK **уже включает `'ad_register'`**; `requested_by_user_id`/`assigned_to_user_id` → users. Backing для REQ-06.
- `migrations/V008__audit_log.sql` — `audit_log.user_id` (NULL для system/trusted) — attribution AD-логинов/регистраций.
- `migrations/V010__sessions.sql` — `sessions` (tower-sessions store) — AD-сессия живёт здесь.
- Таблица `app_settings` (key/value, upsert-паттерн) — флаги автоприёма (SET-10), AD-конфиг.

### Существующий код (точки интеграции)
- `crates/trackly-app/src/services/auth.rs` — `AuthService::login()` (стр. 180) = единственный hook-point: сейчас только argon2-verify (+ dummy-hash anti-enumeration). Сюда добавляется local→AD fallback. Также `create_user`, `desktop_identity`, `get_desktop_lock_enabled`/`set_*` (app_settings паттерн стр. 796-840).
- `crates/trackly-core/src/auth.rs` — `Identity`, `Role` (стр. 34-76), `authorize()` (стр. 136). AD-юзер получает Identity после bind.
- `crates/trackly-infra/src/snmp/mod.rs` + `snmp/mock.rs` + `snmp/real.rs` — **эталон mock-паттерна** для `AdClient` (trait + Real/Mock + runtime switch через config/env).
- `crates/trackly-core/src/domain/requests.rs` + `crates/trackly-app/src/services/request_service.rs` — lifecycle заявок (open→in_progress→completed/rejected), оптимистичная блокировка. Переиспользуется для `ad_register`.
- `crates/trackly-infra/src/config.rs` — `ServerConfig`; добавить `AdConfig { enabled, use_mock, host, ... }` рядом.
- `crates/trackly-core/src/primitives/secret.rs` — `Secret<T>` для AD-пароля.
- `crates/trackly-app/src/dto/auth.rs`, `http/auth.rs`, `tauri_cmds/auth.rs` — DTO/транспорты логина (расширить под AD).

### Frontend (точки интеграции)
- `ui/src/pages/LoginPage.svelte` (или эквивалент Phase 5) — логин-форма; добавить AD-поведение, «Запомнить меня», pending/blocked-экраны.
- `ui/src/lib/api/client.ts` — dual-transport `apiCall()`; проброс AD-ошибок bind.
- `ui/src/pages/SettingsPage.svelte` — добавить вкладку «Active Directory» (Phase 7 ввёл многовкладочные Настройки).
- Раздел «Заявки» (Phase 6 UI) — добавить рендер подтипа `ad_register` (только admin) + модалку approve с выбором роли.

### Стек / практики
- `CLAUDE.md` — `ldap3 0.12` (`tls-native`, simple_bind, MSRV; NTLM/GSSAPI = deferred SSO), «Безопасность» (AD-пароли не хранить), «AD-login phase» паттерн. Раздел «Stack Patterns by Variant» (SSO milestone — feature gate, v2).
- `.planning/research/STACK.md` — pinned версии (ldap3, rustls).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`users` схема** (V002): `ad_user` + nullable `password_hash` уже есть — миграция для AD-юзеров НЕ нужна.
- **`requests` схема** (V006): `request_type='ad_register'` уже в CHECK — backing для REQ-06 готов (для restoration-flavor может понадобиться расширение).
- **SNMP mock-паттерн** (`snmp/mod.rs`): полный эталон для `AdClient` — trait + Real/Mock + `AppCtx::build` runtime switch (config flag + env var). Копируется 1:1.
- **`app_settings`** key/value + upsert: для флага автоприёма (как `desktop_lock_enabled` в auth.rs).
- **`AuthService::login()`**: anti-enumeration timing (dummy-hash) уже реализован — AD-ветку встроить, сохранив constant-time поведение.
- **Lifecycle заявок** (Phase 6): open→in_progress→completed/rejected + optimistic lock — переиспользуется для `ad_register`.

### Established Patterns
- Single-writer / reader-pool: записи через writer-канал, чтения через reader-pool в `spawn_blocking`. AD-bind (сетевой I/O) — async, регистрация юзера — через writer.
- «Один DTO, два транспорта»: handler — тонкий адаптер; AD-логин-логика в сервисе, оба транспорта (Tauri/axum) её покрывают. Но AD-вход — веб-only (D-AD-01), Tauri-путь — локальный.
- Роли TEXT в `users.role`: admin/manager/employee. AD-юзер дефолт employee.
- Mock через config flag + env (SNMP): `TRACKLY_AD_MOCK` для macOS-дева.

### Integration Points
- `AppCtx::build` → собрать `Arc<dyn AdClient>` (mock vs real по config/env), прокинуть в AuthService.
- `AuthService::login()` → ветка local-fail → AD `simple_bind` (если AD enabled) → unknown user → режим регистрации (D-REG-01).
- Раздел «Заявки» (UI + RequestService) → подтип `ad_register`, admin-only фильтр, approve→create_user.
- Settings → вкладка «Active Directory» (тумблер + режим + расширенные поля + auto-detect).

</code_context>

<specifics>
## Specific Ideas

- Пользователь явно описал blocked/restoration UX: заблокированный юзер видит сообщение + «Запрос на восстановление доступа» + «Войти под другим пользователем». Последнее = просто показать обычную форму (escape-hatch для админа рядом).
- Авто-регистрация создаёт информационную заявку админу с «Отклонить» — даже когда юзер уже создан (для контроля/аудита).
- Пользователь не разбирается в LDAP-терминах → UX настроек должен быть «одной кнопкой» (core value): включил AD — работает; детали авто-определяются. Нужна инструкция-памятка.
- AD-вход — только веб (LAN-браузер); десктоп = trusted-admin / локальный как сейчас.

</specifics>

<deferred>
## Deferred Ideas

- **Auto-SSO (кнопка «Войти как \<display name\>», вход без пароля)** — v2 (ADV-01, Kerberos/NTLM Negotiate). Windows-only, MSRV↑, не собирается на macOS. Архитектура `trait AdClient` готовит почву.
- **AD-вход в десктоп-режиме** (при десктоп-локе) — рассмотреть в v2; v1 веб-only.
- **SMTP/email-уведомления о новых заявках на регистрацию** — финальная фаза v2 (NTF-02). In-app уведомление (REQ-04) достаточно для v1.
- **Групповая синхронизация ролей из AD-групп** (маппинг AD-group → Trackly-role) — не обсуждалось, потенциальный v2.

None из обсуждения не ушло за пределы scope фазы (restoration-flow явно включён пользователем в Phase 9).

</deferred>

---

*Phase: 9-AD-аутентификация и заявки на регистрацию пользователей*
*Context gathered: 2026-06-19*
