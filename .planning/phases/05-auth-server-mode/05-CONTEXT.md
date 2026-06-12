# Phase 5: Авторизация, локальные пользователи и серверный режим - Context

**Gathered:** 2026-06-13
**Status:** Ready for planning
**Source:** /gsd-discuss-phase 5 — 4 области (десктоп-доступ/bootstrap, роли×разделы×эндпоинты, серверный режим, веб-сессия/безопасность) обсуждены интерактивно.

<domain>
## Phase Boundary

Включить локальную аутентификацию (argon2id), три роли с enforcement **и на UI, и на API**, HTTPS-сервер axum для браузерного доступа из LAN, единый `authorize()` для обоих транспортов. Десктоп остаётся unlocked-by-default с опциональным локом.

Покрывает: USR-01..07, SRV-01..05, SET-08.

**В scope:**
- **Локальные пользователи (USR-01):** CRUD на странице `/users` (Логин, ФИО, Пароль argon2id через `Secret<T>`, Роль, Email опц., Активен/Заблокирован). Заполняет существующую таблицу `users` (V002).
- **Три роли (USR-02):** Администратор (полный доступ), Специалист/manager (CRUD устройств/картриджей/актов + выполнение заявок, без управления пользователями и настройками), Сотрудник/employee (только создание заявок).
- **Bootstrap первого админа:** first-run мастер на десктопе создаёт первого пользователя-admin.
- **Десктоп-доступ (USR-04):** unlocked-by-default = trusted-admin; опциональный лок «требовать вход в десктопе» (флаг в `app_settings`), при котором показывается тот же логин-экран, что и в вебе.
- **Веб-аутентификация (USR-03, USR-05):** логин/logout/смена пользователя через браузер; сессия в cookie через `tower-sessions` с rusqlite-store (таблица `sessions` V010); переживает рестарт сервера; отзывается на logout.
- **Enforcement (USR-06):** единый `authorize(ctx, action)` в сервис-слое — нельзя обойти роль прямым HTTP-запросом; CI-тест role × endpoint (403).
- **HTTPS-сервер (SRV-01..05, USR-07, SET-08):** реальный bind axum-роутеров (сейчас построены, но не bind'ятся), HTTPS-only через rustls, self-signed cert через `rcgen` при первом включении (путь к своему — конфигурируем), CSRF/security-headers/rate-limit, горячий старт/стоп без рестарта, корректный graceful shutdown.
- **Мини-Настройки → Сеть:** новый раздел `/settings` с **единственной** вкладкой «Сеть» (тумблер сервера, порт, bind-адрес, путь к cert) — Фаза 7 добавит остальные вкладки.

**НЕ в scope этой фазы (явно deferred):**
- AD/LDAP-вход (`users.ad_user`, bind-only) → Phase 8.
- Раздел «Заявки» (employee только создаёт их) — портал и сами заявки → Phase 6. В Phase 5 employee видит placeholder.
- Полный раздел «Настройки» (остальные вкладки кроме «Сеть») → Phase 7.
- SMTP / email-reset паролей → финальная фаза v2 (на LAN сброс пароля делает админ вручную).
- Дашборд/Отчёты под ролями → Phase 7.

**Mode:** mvp — вертикальный слайс: login UI → axum auth handler + Tauri command → AuthService/authorize() → users/sessions repo → DB.

</domain>

<decisions>
## Implementation Decisions

### Десктоп-доступ и bootstrap

#### D-Bootstrap-01: первый админ — через first-run мастер
- На чистой БД (нет ни одного пользователя) десктоп при запуске показывает экран «создайте администратора» (Логин / ФИО / Пароль). Обязательный шаг перед работой.
- НЕ авто-seed дефолтного `admin/admin` (исключаем риск забытого дефолтного пароля на LAN).
- Создаёт реальную строку в `users` с `role='admin'`.

#### D-Desktop-01: unlocked-by-default = trusted-admin
- Когда лок выключен (дефолт, ROADMAP «unlocked-by-default»), десктоп всегда работает с полным доступом (роль admin). Локальная машина = физически доверенная.
- **Открытый вопрос для планировщика (audit attribution):** какой `user_id` писать в `audit_log` для trusted-десктоп-мутаций. Разумный дефолт: если в БД ровно один admin — атрибутировать ему; иначе `user_id = NULL` (как в Phase 4). Зафиксировать при планировании.

#### D-Desktop-02: десктоп-лок = тот же логин-экран, флаг в БД
- Опция «требовать вход в десктопе» хранится в `app_settings` (БД), НЕ в `config.toml` (чтобы переносилась с портативной БД и управлялась из UI).
- При включённом локе десктоп показывает тот же логин-экран, что и веб; реальная argon2id-аутентификация; роль/identity — из строки `users`. Единый код логина на оба транспорта.

### Роли, разделы и enforcement

#### D-RBAC-01: единый authorize(ctx, action) в сервис-слое
- Проверка роли живёт в общем сервис-слое (один на оба транспорта). Handler передаёт identity+role в `AppCtx`/контекст вызова, сервис вызывает `authorize()` перед каждой мутацией/защищённым чтением.
- Tauri-транспорт передаёт trusted-admin (или залогиненного при локе); axum — роль из сессии.
- Это реализует ROADMAP «единый authorize() для обоих транспортов» и USR-06 (нельзя обойти через curl).
- CI-тест: матрица role × endpoint → 403 для запрещённых.

#### D-RBAC-02: employee в Phase 5 — вход работает, пустой портал + заглушка
- Роль «Сотрудник» полностью заведена и тестируется уже сейчас (RBAC-каркас не откладываем). Employee может войти в веб, видит placeholder «Заявки появятся скоро» (как Phase-6 заглушка). Phase 6 наполняет портал.

#### D-RBAC-03: UI-gating — скрывать недоступные разделы
- `SIDEBAR_ITEMS` фильтруется по роли текущего пользователя: employee видит только «Заявки»; manager — без «Пользователи» и без админ-частей «Настройки»; admin — всё.
- UI-скрытие — UX-слой; источник истины безопасности — `authorize()` на API. На UI-gating НЕ полагаемся для защиты.

### Серверный режим

#### D-Server-01: горячий старт/стоп без рестарта
- Тумблер сервера в Настройки→Сеть сразу запускает/останавливает axum-задачу через отдельный под-`CancellationToken` (дочерний к `AppCtx.shutdown`). Смена порта/bind = stop+start.
- Поддерживает core value «одной кнопкой». Планировщику: аккуратное управление жизненным циклом задачи (taskTracker, drain in-flight, см. SRV-05).

#### D-Server-02: UI — мини-Настройки→Сеть сейчас
- Создаём раздел `/settings` с единственной вкладкой «Сеть»: тумблер сервера, порт, bind-адрес, путь к собственному cert. Остальные вкладки — Phase 7.
- Управление пользователями — отдельный раздел `/users` (уже в sidebar, phase 5), НЕ внутри Настроек.

#### D-Server-03: bind-адрес — dropdown без предупреждения
- Выбор `127.0.0.1` / `0.0.0.0` обычным dropdown, без подтверждающего предупреждения. Админ сам понимает последствия.

#### D-Server-04: HTTPS-only + self-signed cert UX
- HTTPS-only, HTTP-listener отсутствует (ROADMAP критерий #2). rustls + `rcgen` self-signed при первом включении; путь к своему cert — конфигурируем (`config.server.cert_path`, уже есть).
- После старта сервера UI показывает: `https://<ip>:<port>`, **отпечаток (fingerprint) сертификата** для сверки, и короткую инструкцию «в браузере: Дополнительно → Перейти» (помощь нетех-сотрудникам пройти предупреждение self-signed).

### Веб-сессия и безопасность

#### D-Session-01: sliding, 30 дней, rusqlite-store
- Cookie-сессия через `tower-sessions`, store — кастомный rusqlite-backed (таблица `sessions` V010). Sliding-expiration, окно 30 дней. Переживает рестарт сервера (success criterion #4), отзывается на logout.

#### D-Session-02: CSRF — SameSite=Strict + Origin-check
- Cookie: `SameSite=Strict` + `Secure` + `HttpOnly`. Дополнительно проверка `Origin`/`Referer` на mutation-эндпоинтах. Достаточно для одно-origin SPA на LAN; минимум движущихся частей (без double-submit token).
- Сюда же security headers (SRV-02): базовый набор (CSP/no-sniff/frame-deny) через tower-http.

#### D-Auth-01: политика паролей — мин. 8 символов, без жёсткой сложности
- Минимум 8 символов, без обязательных правил сложности. Admin сбрасывает пароль любого юзера вручную (email-reset нет на LAN). Любой юзер может сменить свой пароль (требуется старый).

#### D-Auth-02: rate-limit на /login (basic)
- Простой брутфорс-лимит на эндпоинте логина (порядка 5–10 попыток/мин по IP/логину, далее задержка/блок). Реализует SRV-02 «rate limiting (basic)».

### Claude's Discretion
- Точная структура login-экрана (вёрстка/копирайт) — на усмотрение планировщика/UI-фазы, в рамках UI-SPEC паттернов.
- Конкретный набор security headers и точные числа rate-limit (попытки/окно) — планировщику в разумных пределах OWASP.
- Имя slug раздела роута для «Сеть»-вкладки внутри `/settings`.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Roadmap & requirements
- `.planning/ROADMAP.md` §«Phase 5» (строки ~179–194) — goal, 5 success criteria (bootstrap, server toggle+cert, role×endpoint 403, session survives restart, graceful shutdown).
- `.planning/REQUIREMENTS.md` — USR-01..07 (стр. 115–121), SRV-01..05 (стр. 160–164), SET-08 (стр. 145).

### Существующая схема (Phase 1, не пере-создавать)
- `migrations/V002__core_entities.sql` — таблица `users` (login UNIQUE, full_name, password_hash NULL для AD, role default 'employee', ad_user, email, notes, standard4-колонки).
- `migrations/V010__sessions.sql` — таблица `sessions` (id BLOB PK, data BLOB, expiry_date INTEGER) — backing store для tower-sessions, hard-delete system table.
- `migrations/V008__audit_log.sql` — `audit_log.user_id` (NULL для system/trusted) — для attribution мутаций.
- `migrations/V006__requests.sql` — FK `requested_by_user_id`/`assigned_to_user_id` → users (контекст ролей для Phase 6).

### Существующий код (точки интеграции)
- `crates/trackly-app/src/context.rs` — `AppCtx` (Clone через Arc; `shutdown: CancellationToken` уже есть; services собраны). Сюда добавляется AuthService/identity.
- `crates/trackly-app/src/http/mod.rs` + `http/*.rs` — per-resource axum роутеры (построены, НЕ bind'ятся). Phase 5 добавляет `/api/v1/auth/*`, tower-sessions middleware, реальный `axum::serve` + rustls.
- `crates/trackly-core/src/primitives/secret.rs` — `Secret<T>` (zeroize-on-drop, `***` Debug; есть `expose()`). Для password/cert/community.
- `crates/trackly-infra/src/config.rs` — `ServerConfig { enabled, host, port, cert_path }` (дефолты: false / 127.0.0.1 / 8443 / "").
- `crates/trackly-app/src/main.rs` — boot; «Phase 5+» комментарии где стартует сервер и observ-ится shutdown token.
- `crates/trackly-app/src/shutdown.rs` — существующий shutdown-механизм (для SRV-05 graceful drain).

### Frontend (точки интеграции)
- `ui/src/lib/api/client.ts` — `apiCall()` уже dual-transport (isTauri → invoke / fetch `/api/v1/*`). Сюда добавляется проброс auth-ошибок (401/403) и редирект на логин.
- `ui/src/features/layout/sidebar-config.ts` — `SIDEBAR_ITEMS`; фильтрация по роли (D-RBAC-03).
- `ui/src/pages/UsersPage.svelte` — placeholder phase 5, заменяется реальным CRUD.
- `ui/src/pages/SettingsPage.svelte` — placeholder phase 7; Phase 5 вводит мини-версию с вкладкой «Сеть».
- `ui/src/lib/stores/` — нет auth-store; создаётся (current user, role, isAuthenticated).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `Secret<T>` (trackly-core): обёртка для пароля при создании/верификации и для cert/ключа в памяти. `tests/secret_zeroize.rs` гарантирует zeroize.
- `AppCtx` + `shutdown: CancellationToken`: уже спроектирован под «Phase 5+ слушает в axum-серверах». Под-токен для hot stop сервера (D-Server-01).
- Dual-transport `apiCall()` во фронте: HTTP-путь уже застаблен (`fetch /api/v1/*`) — Phase 5 делает его «живым».
- axum-роутеры по ресурсам (`http/devices.rs` и т.п.): готовы к merge под общий Router с auth-middleware.
- refinery embedded migrations (`migrations/`): новые миграции (если нужны app_settings-флаг лока, force-change и т.п.) — следующий номер V018.

### Established Patterns
- Single-writer / reader-pool: записи через `writer.execute()`, чтения через `readers.acquire()` в `spawn_blocking`. AuthService следует тому же.
- «Один DTO, два транспорта» (success criterion #5): handler — тонкий адаптер над общим `build_*`/service-хелпером. authorize() живёт в сервисе, чтобы оба транспорта покрывались.
- Lookup/standard4 schema conventions (D-Schema-03/04): users — user-mutable (standard4), sessions — hard-delete system table.
- Роли как TEXT в `users.role`: 'admin' | 'manager' | 'employee' (уже зашито в V002 default + comment).

### Integration Points
- `main.rs` boot → стартовать axum-сервер (если `config.server.enabled`) под под-токеном; слушать `AppCtx.shutdown` для graceful drain (SRV-05).
- `AppCtx` → добавить identity/AuthService; прокинуть в Tauri commands (trusted-admin или залогиненный) и axum handlers (из сессии).
- tower-sessions middleware на `/api/*` кроме `/api/v1/auth/login` (см. CLAUDE.md «Session middleware gates /api/* except login»).
- Frontend router → guard: если 401, редирект на логин-экран; auth-store держит current user/role; sidebar и роуты фильтруются.

</code_context>

<specifics>
## Specific Ideas

- «Одной кнопкой» (core value) распространяется на сервер: тумблер сразу включает сервер + показывает готовый `https://…` адрес с fingerprint и инструкцией — нетех-сотрудник должен суметь подключиться без помощи.
- Роли по-русски в UI: Администратор / Специалист / Сотрудник; в БД — admin / manager / employee (маппинг при отображении).
- CI должен содержать тест-матрицу role × endpoint (явный ROADMAP success criterion #3) — планировщику заложить.

</specifics>

<deferred>
## Deferred Ideas

- AD/LDAP-вход (bind-only, `ad_user=1`, `password_hash=NULL`) → Phase 8 (AD-вход и релизный пайплайн).
- Полный раздел «Настройки» (вкладки кроме «Сеть») → Phase 7.
- Email/SMTP сброс пароля, self-service recovery → v2.
- Связь заявок employee с реальным порталом и жизненным циклом → Phase 6.
- Force-change пароля при первом входе (рассматривалось, не выбрано — выбран мин. 8 симв. + ручной сброс админом) — при желании поднять в будущем как настройку.
- Audit user_id для trusted-десктопа — окончательное решение делегировано планировщику (см. D-Desktop-01).

None из обсуждения не ушло за пределы scope фазы.

</deferred>

---

*Phase: 5-Авторизация, локальные пользователи и серверный режим*
*Context gathered: 2026-06-13*
