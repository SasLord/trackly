# Architecture: интеграция Link-клиента и подсистемы Чат (v1.5)

**Domain:** десктоп-мессенджер Link + чат на сервере Trackly + автообновление клиента с сервера
**Researched:** 2026-11-25
**Scope:** только интеграция НОВОГО в существующую архитектуру (v1.0-архив: `.planning/research/v1.0/`)
**Overall confidence:** MEDIUM-HIGH (структура кода прочитана; три пункта помечены как требующие спайка, см. «Research flags»)

Все пути ниже проверены по репозиторию на 2026-11-25. Последняя миграция: `V057__history_events.sql`, значит новые начинаются с `V058`.

---

## 0. Что в коде определяет решения (факты)

| Факт | Где | Следствие |
|------|-----|-----------|
| Один `broadcast::channel(128)` с `WsEvent` на всех; фильтр только `is_visible_to(&Identity)` (роль + user_id) | `context.rs` (`ws_broadcast`), `http/ws.rs`, `dto/printer.rs` | Членство в комнате выразить нельзя -> для чата нужен отдельный маршрутизатор по пользователям |
| `Lagged(n)` -> `continue` без уведомления клиента; клиент не умеет догрузку | `http/ws.rs` (Pitfall 5) | События теряются молча; чинить надо на уровне протокола, не размера буфера |
| Сессии: `tower-sessions` + `RusqliteSessionStore`, cookie `Secure`, `SameSite::Strict`, 30 дней inactivity; `SessionIdentity {user_id, role}`; `refresh_session_role` | `http/mod.rs`, `server/rusqlite_session_store.rs` | Cookie-сессию можно переиспользовать для не-браузера (reqwest cookie store), но не из webview другого origin |
| SPNEGO-эндпоинт `GET /api/v1/auth_ad_sso` (серверная сторона, `sspi`, keytab); выдаёт ту же сессию | `http/sso.rs` | Клиенту нужна только клиентская сторона Negotiate; серверная уже есть |
| Транспорт SPA: `apiCall(name,args)` -> `invoke` в Tauri, иначе `POST /api/v1/{name}`; WS: `/api/v1/ws` в браузере, `listen('trackly-event')` в Tauri | `ui/src/lib/api/client.ts`, `lib/api/ws.ts`, `stores/transport.svelte.ts` | Контракт «имя + JSON-args» уже транспорт-агностичен: Link-прокси может его повторить 1:1 |
| Запрос-чат: `chat_messages` (неизменяемые, `request_id`), `chat_reads` (`last_message_id`), `chat_mutes`; лента = слияние с `audit_log` на чтении; видимость через `request_visibility_clause` | `V054`, `V055`, `services/chat_service.rs`, `repos/chat_sqlite.rs` | Привязан к жизненному циклу заявки и аудиту; имена таблиц `chat_*` заняты |
| Вложения: `attachments(parent_kind, parent_id, content_hash, ...)`, без FK и CHECK, файл адресуется хэшем; `ParentKind` в Rust | `V050`, `trackly-core/src/domain/attachments.rs` | Новый родитель = новый вариант enum, без перестройки таблицы |
| `Action` + `authorize(&Identity,&Action)`; `employeeRoutes` в `ui/src/routes.ts` (всё лишнее -> `AccessDenied`) | `trackly-core/src/auth.rs`, `ui/src/routes.ts` | Новые маршруты и действия надо вносить в обе карты; есть гейты `check-employee-routes.mjs` |
| `ui/package.json`: `prebuild` запускает `cargo test -p trackly-app --test export_bindings`; `pnpm lint` = ~90 `check-*.mjs`, многие грепают пути `ui/src/...` | `ui/package.json` | Перенос файлов в общий пакет ломает гейты по путям; сборка Link-UI не должна требовать прогона cargo |
| `release.yml`: триггер `v*.*.*`, инъекция версии в `workspace.package.version` и `tauri.conf.json`, `ui/pnpm-lock.yaml`, `working-directory: ui`, Windows с vcpkg heic (до 90 мин) | `.github/workflows/release.yml` | Второй Tauri-аппарат в workspace унаследует ЧУЖУЮ версию; нужен отдельный workflow |
| TLS: самоподписанный `rcgen`, ручной accept-loop hyper+tokio-rustls, `TlsBundle` | `server/tls.rs`, `server/mod.rs` | Есть готовый источник отпечатка сертификата для пиннинга |
| Единственный писатель (mpsc 256, таймаут 5 с), рассылка событий ПОСЛЕ `.await?` и вне замыкания записи (D-04) | `context.rs`, `services/entities_broadcast.rs` | Назначение `seq` внутри замыкания записи тривиально монотонно; публикация событий — по тому же правилу |

---

## 1. Монорепо: раскладка, общий фронтенд, сборка

### Целевая раскладка

```
Cargo.toml                       # members += crates/trackly-proto, crates/link-app
crates/
  trackly-core/                  # + domain/im/*, domain/link/* (чистые правила), Action'ы
  trackly-infra/                 # + repos/im_sqlite.rs, repos/link_sqlite.rs
  trackly-proto/        NEW      # serde+specta DTO, общие для сервера и клиента (БЕЗ tauri, БЕЗ rusqlite)
  trackly-app/                   # + services/im_*, services/link_*, http/im.rs, http/link*.rs, server/ im_hub
  link-app/             NEW      # Tauri 2 клиент (tray, autostart, HTTP/WS-клиент, updater)
migrations/                      # V058+ 
pnpm-workspace.yaml     NEW      # packages: ui, link-ui, packages/*
packages/
  design/               NEW      # _tokens.scss, _breakpoints.scss, global-reset, шрифты, SVG-эмодзи
  kit/                  NEW      # Modal, Dropdown, ContextMenu/ActionMenu, Toast, Tooltip (то, что нужно чату)
  transport/            NEW      # интерфейс ApiTransport/EventSource + реализации (invoke / fetch / link-proxy)
  attachments/          NEW      # вынос ui/src/features/attachments (AttachmentTile/Viewer/Block)
  messenger/            NEW      # весь UI чата (новый код, в общем пакете С САМОГО НАЧАЛА)
  proto-ts/             NEW      # сгенерированные specta-биндинги trackly-proto (закоммичены)
ui/                              # Trackly SPA (импортирует packages/*)
link-ui/                NEW      # SPA клиента Link (тонкая оболочка над packages/messenger)
```

### Решения

1. **`trackly-proto` вместо зависимости клиента от `trackly-app`.** `trackly-app` — это весь сервер + Tauri + PDF + krilla; клиенту он не нужен. В `trackly-proto` идут ТОЛЬКО новые DTO (IM, Link, auth-ответы клиента). Существующие `dto/*` не переносить (риск и нулевая польза). Зависимости: `serde`, `specta`, `time`. `trackly-app` зависит от `trackly-proto`; `link-app` — тоже. Confidence: HIGH.
2. **Версия `link-app` собственная.** В `link-app/Cargo.toml` НЕ `version.workspace = true`. Иначе существующий шаг `release.yml` («perl -0pi … первое version = в Cargo.toml») подменит версию Link версией Trackly. Версионирование Link — независимое (теги `link-v*.*.*`).
3. **Отдельный workflow `release-link.yml`** (триггер `link-v*.*.*`), а не вторая матрица в `release.yml`: другие каденции, Link не нужен heic/vcpkg (Windows-сборка быстрее), падение одного не блокирует другой. Общие шаги (toolchain, rust-cache, Linux-зависимости, NSIS-prefetch с повторами) вынести в composite action `.github/actions/tauri-setup`, чтобы не копировать 200 строк с закреплёнными хэшами. Триггер `v*.*.*` существующего workflow не пересекается с `link-v*`. Confidence: HIGH.
4. **Ключ подписи updater:** отдельная пара ed25519 для Link; приватный ключ — секрет репозитория (`TAURI_SIGNING_PRIVATE_KEY`), публичный — в `link-app/tauri.conf.json` (`plugins.updater.pubkey`, строка содержимого, не путь) и в сервере (проверка при загрузке релиза).
5. **Общий фронтенд — workspace-пакеты «как исходники»** (без собственной сборки): `exports` с условием `svelte`, `resolve.dedupe: ['svelte']`, SCSS через `sass.loadPaths`/`@use '@trackly/design/tokens'`. Один корневой `pnpm-lock.yaml`.
6. **Что выносить, а что нет (границы риска):**
   - Выносить сразу: `design` (токены), `kit` (только нужные чату компоненты), `transport`, `attachments`.
   - `messenger` — новый код, пишется в пакете сразу; НЕ пытаться переиспользовать/переносить `ui/src/features/requests/chat/*` (там 15 гейтов и UAT 41.5; брать только идеи: `linkify`, `chatDates`, паттерн драфтов — копированием, не перемещением).
   - Остальные `features/*` остаются в `ui/`.
7. **Перенос ломает гейты:** `ui/package.json` -> `lint` содержит ~90 скриптов, часть грепает пути `ui/src/...` (`check-attachments-block.mjs`, `check-tokens.mjs`, `check-contrast.mjs`, `check-focus-outline.mjs`). При выносе обновить пути СКРИПТОВ в том же коммите и прогнать `--selftest` (урок «гейт, слепой к перемещению, зелёный впустую»). Скрипты, проверяющие токены/контраст, должны запускаться и для `packages/design` и `link-ui`.
8. **Биндинги.** Сейчас `bindings.ts` создаётся тестом `export_bindings` в `prebuild`. Для `link-ui` так нельзя (сборка UI не должна тянуть cargo). Решение: `packages/proto-ts/bindings.ts` генерируется тестом `trackly-proto` и КОММИТИТСЯ; в CI — гейт дрейфа (regen + `git diff --exit-code`). `ui/prebuild` остаётся как есть.
9. **Подводные камни сборки:**
   - `tauri::generate_context!` требует существующий `frontendDist` при компиляции -> `cargo clippy --workspace` в `ci-fast` упадёт без `link-ui/dist` (та же «ui/dist seed gotcha», что уже известна). В `ci-fast` собирать `link-ui` до clippy или сеять заглушку.
   - `ci-fast` — один последовательный job (первый красный шаг скрывает остальные): добавлять шаги Link ПОСЛЕ существующих и в отдельный job, чтобы красный Link не маскировал Trackly и наоборот.
   - Унификация features в одном workspace (прецедент: `snmp2`/`zip`): `link-app` тянет `reqwest`(rustls), `tokio-tungstenite`, клиентский SSPI. Собирать релиз строго `-p link-app` / `-p trackly-app`; в CI добавить `cargo tree -d` проверку и `cargo deny`.
   - `tauri.conf.json` Link: свой `identifier` (например `org.trackly.link`), `bundle.targets` nsis/dmg/appimage, `createUpdaterArtifacts: true`, NSIS `installMode: currentUser` (дефолт Tauri 2; соответствует «без прав администратора, %LOCALAPPDATA%»).
   - Плагины Link: `tray-icon` (фича tauri), `autostart`, `single-instance`, `notification`, `updater`; `shell`/`fs`/`dialog` без нужды не подключать (меньше capability).

---

## 2. Транспорт клиент <-> сервер

### Рекомендация: все сетевые вызовы идут через Rust-часть Link; webview сети не видит

```
 Link webview (Svelte, packages/messenger)
   apiCall(name,args)  --invoke('api_call')-->  Rust: LinkTransport
   onEvent(...)        <--emit('link-event')--      reqwest (rustls, pinned cert, cookie jar)
   <img src="link-att://...">  <--URI scheme--      tokio-tungstenite WS (+heartbeat, backoff, cursor-sync)
                                                    SSPI Negotiate (Windows) / пароль
                                                      |  HTTPS (self-signed, pinned)
                                                      v
                                                Trackly server  /api/v1/*, /api/v1/ws
```

**Почему не fetch/WS из webview (проверено по коду сервера):**

| Проблема | Факт |
|----------|------|
| Origin | Webview Tauri живёт на `tauri://localhost` / `http://tauri.localhost`; сервер не имеет CORS-слоя вообще; CSP сервера `connect-src 'self' wss:` относится к ЕГО страницам, а не к чужому origin |
| Cookie | `SameSite::Strict` + `Secure`: кросс-сайтовые запросы из webview cookie не получат |
| Самоподписанный сертификат | WebView2/WKWebView покажут ошибку сертификата; управлять доверием из JS нельзя; пиннинг в webview невозможен |
| SPNEGO | Встроенная аутентификация WebView2 зависит от allowlist/зоны интрасети и не даёт контроля над ошибками; нативный SSPI из Rust предсказуем |
| Безопасность rich-text | Чат рендерит чужой контент; без доступа webview к сети XSS не может эксфильтровать данные (CSP Link: `connect-src ipc: http://ipc.localhost` — сети нет) |

**Цена решения:** (а) слой прокси; (б) бинарные загрузки/скачивание; (в) события. Смягчение:
- **Один универсальный Tauri-command `api_call(name, args)`**, проксирующий `POST /api/v1/{name}` с JSON — ровно контракт, который уже у `apiCall`. На стороне Rust — allowlist имён (`im_*`, `link_*`, `auth_*`, `attachments_*`), чтобы скомпрометированный webview не дёргал админские маршруты (сервер всё равно авторизует по роли — это второй рубеж).
- **Вложения**: прецедент есть — `attachments_protocol.rs` (кастомная URI-схема в Trackly). В Link — схема `link-att://{id}`: Rust стримит из сервера с сессионной cookie. Загрузка — команда принимает путь/байты и шлёт multipart.
- **События**: Rust держит WS, превращает кадры в `emit('link-event', payload)` — та же форма, что `trackly-event` у десктопа, поэтому `packages/transport` реализует единый интерфейс `EventSource`.

**Пиннинг сертификата (закрепление):** TOFU + явное подтверждение. При первой настройке пользователь/скрипт GPO задаёт `server=host:port` и опционально `pin=sha256:<отпечаток>`; без `pin` клиент показывает отпечаток (в формате, который сервер уже умеет показывать в настройках) и просит подтвердить. Хранится SHA-256 SPKI/DER листа; своя `rustls::ServerCertVerifier` сравнивает отпечаток (имя хоста игнорируется — IP/смена имён в LAN). Смена сертификата на сервере -> клиент уходит в состояние «отпечаток изменился», не подключается молча. Confidence: HIGH (стандартный rustls-паттерн).

**Автообновление и самоподписанный TLS.** Плагин updater использует собственный reqwest, а не наш верификатор. По документации плагина конфиг содержит флаги небезопасного TLS (`dangerousInsecureTransportProtocol` подтверждён; флаг принятия невалидных сертификатов виден в исходнике, в странице документации не описан -> MEDIUM). Рекомендация: updater работает с отключённой проверкой цепочки TLS, поскольку целостность обеспечивается ed25519-подписью артефакта (проверяется плагином), а ВСЕ остальные запросы идут через закреплённый клиент. Остаточный риск — подмена манифеста с откатом на более старую подписанную версию; закрывается `version_comparator` (см. п.5). Если флаг в конкретной версии плагина отсутствует — запасной путь: проверка и скачивание собственным закреплённым клиентом + ручной запуск установщика после `minisign-verify` (хуже: переписываем часть плагина). **Требует спайка в фазе обновлений.**

### Аутентификация не-браузера

**Решение: переиспользовать `tower-sessions` + добавить долгоживущие «учётные данные устройства».**

1. **Сессия** — как есть. Rust-клиент хранит cookie в jar. Все 112 вызовов `session_identity()` и `refresh_session_role` работают без изменений. SameSite/Secure не мешают (клиент не браузер).
2. **Вход:**
   - Доменный ПК: `GET /api/v1/auth_ad_sso` с токеном SPNEGO, сгенерированным нативно (Windows SSPI `InitializeSecurityContext` с текущими учётными данными; цель SPN берётся из SPN сервера, который клиент получает в ответе «401 Negotiate»/из настройки). Сервер НЕ меняется (`http/sso.rs` как есть). На macOS/Linux — вход паролем (Kerberos-клиент вне объёма).
   - Вне домена: `POST /api/v1/auth_login` (существующий, с rate-limit governor; клиент учитывает 429).
3. **Продление без повторного ввода (вне домена):** при успешном входе сервер выдаёт `device_token` (256 бит случайных, на сервере хранится SHA-256 хэш; Argon2 тут избыточен — токен высокоэнтропийный), привязанный к паре `(user_id, device_id)`. Новый эндпоинт `POST /api/v1/link_resume` меняет `device_token` на свежую сессию. Хранение на клиенте: DPAPI (Windows) / Keychain (macOS) / Secret Service или защищённый файл (Linux) через крейт `keyring`. Токен отзывается админом («клиенты» -> «отключить») и при блокировке пользователя. Пароль нигде не сохраняется (инвариант CLAUDE.md).
4. **Сессия живёт 30 дней inactivity** (как сейчас); у клиента она практически бессрочна за счёт resume/SSO.

### Идентичность устройства (нужна для каналов по компьютерам и для v1.7)

- `device_id` — UUIDv4, генерируется клиентом при первом запуске, лежит рядом с `device_secret` в защищённом хранилище.
- `link_devices` (новая таблица): `device_id`, `hostname`, `os`, `arch`, `client_version`, `first_seen`, `last_seen`, `last_ip`, `last_user_id`, `secret_hash`, `channel_override NULL`, `revoked_at NULL`.
- **Регистрация до входа:** запрос обновления приходит ДО логина (экран входа тоже должен обновляться) -> `device_id` передаётся заголовком `X-Link-Device`, при первом контакте сервер создаёт запись и возвращает `device_secret` (TOFU), дальше подпись/секрет обязательны для привязки токенов. Для выбора канала достаточно `device_id` (подмена даёт злоумышленнику лишь смену канала, подписи артефактов это не обходит); для v1.7 (инвентаризация) и привязки токенов — `device_secret`.
- Компьютер ≠ пользователь: на одном ПК могут входить разные пользователи; `link_devices.last_user_id` — информация, не ключ.
- Для v1.7 в `link_devices` позже добавляется ссылка на АРМ (`group_id`); сейчас НЕ добавлять (YAGNI), но имя таблицы и `device_id` проектировать как стабильный публичный идентификатор.

---

## 3. Модель данных чата

### Решение по связи с запросным чатом: **оставить раздельно**

Запросный чат (V054/V055) неизменяем, ключ `request_id`, видимость выводится из заявки, лента сливается с `audit_log`, непрочитанное считается по `last_message_id`+`last_audit_id`. Новый чат — изменяемый (правка, мягкое/жёсткое удаление, реакции), членство явное, ключ — беседа. Слияние потребовало бы либо ломать «сообщения неизменяемы (D-22)» и 15 гейтов, либо пустить заявку через `conversation`-абстракцию (это ровно тема v1.6 «Заявки в Link»). Для v1.5: две подсистемы, префикс новых таблиц `im_` (не `chat_`: имена заняты, и в коде/событиях `ChatMessage`/`ChatReadChanged` уже означают заявки). В v1.6 клиент Link показывает переписку заявки через существующие `chat_*` эндпоинты. Confidence: HIGH.

### Схема (миграция V058+, каждая — с `PRAGMA user_version = N`; только CREATE, без перестройки старых таблиц; FK на `users` не ставить — как в V050/V054, записи переживают удаление учётки)

```sql
im_conversations(
  id INTEGER PK AUTOINCREMENT,
  kind TEXT NOT NULL,                 -- 'dialog' | 'room'
  title TEXT NULL,                    -- NULL у dialog
  dialog_key TEXT NULL UNIQUE,        -- 'minUid:maxUid' для dialog => один диалог на пару
  created_by_user_id INTEGER NOT NULL,
  created_at_utc INTEGER NOT NULL,
  last_seq INTEGER NOT NULL DEFAULT 0 -- счётчик seq (см. §4); растёт на ЛЮБОЕ изменение сообщений
)

im_members(
  conversation_id INTEGER NOT NULL,
  user_id INTEGER NOT NULL,
  role TEXT NOT NULL,                 -- 'creator' | 'member'
  joined_at_utc INTEGER NOT NULL,
  left_at_utc INTEGER NULL,           -- мягкий выход: история бесед сохраняется
  visible_from_seq INTEGER NOT NULL DEFAULT 0,
  last_read_seq INTEGER NOT NULL DEFAULT 0,
  muted INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (conversation_id, user_id)
)

im_messages(
  id INTEGER PK AUTOINCREMENT,
  conversation_id INTEGER NOT NULL,
  seq INTEGER NOT NULL,               -- порядок создания внутри беседы, без пропусков
  rev INTEGER NOT NULL,               -- значение last_seq на момент ПОСЛЕДНЕГО изменения (курсор синхронизации)
  kind TEXT NOT NULL DEFAULT 'user',  -- 'user' | 'system' (состав, смена Создателя)
  author_user_id INTEGER NULL,
  client_msg_id TEXT NULL,            -- идемпотентность повторной отправки
  body_json TEXT NOT NULL,            -- версионируемый документ {v:1, doc:{...}}
  body_text TEXT NOT NULL,            -- ВЫВОДИТСЯ СЕРВЕРОМ из doc; для FTS и превью
  reply_to_id INTEGER NULL,
  created_at_utc INTEGER NOT NULL,
  edited_at_utc INTEGER NULL,
  deleted_at_utc INTEGER NULL, deleted_by_user_id INTEGER NULL,
  purged_at_utc INTEGER NULL,         -- жёсткое удаление = надгробие, не DELETE
  UNIQUE (conversation_id, seq),
  UNIQUE (conversation_id, author_user_id, client_msg_id)
)
CREATE INDEX idx_im_messages_rev ON im_messages (conversation_id, rev);

im_reactions(message_id, user_id, emoji, created_at_utc, PRIMARY KEY (message_id, user_id, emoji))
im_read_advances(conversation_id, user_id, up_to_seq, at_utc)  -- журнал продвижений указателя
im_mentions(message_id, user_id)                               -- из узлов mention документа
-- FTS5 (внешний контент, триггеры insert/update/delete):
CREATE VIRTUAL TABLE im_messages_fts USING fts5(body_text, content='im_messages', content_rowid='id', tokenize='trigram');
```

### Ключевые решения

- **Жёсткое удаление = надгробие** (`purged_at_utc`, `body_json='{}'`, `body_text=''`, файлы вложений удаляются), строка остаётся: иначе в `seq` образуется дыра, ломаются `reply_to_id`, курсорная синхронизация и контракт «seq без пропусков». Триггер FTS обнуляет текст. Мягкое удаление: сервер для не-админа отдаёт только `{id, seq, deleted:true}` — тело в ответ не попадает (редакция на сервере, не в UI).
- **Указатель чтения: на участника (`last_read_seq`), а не на сообщение.** Дёшево (одна строка/участник, а не N×M), «непрочитано = сообщения с seq > указателя от других». Для «Информации о сообщении» («кто и КОГДА прочитал») указатель сам времени не хранит -> журнал `im_read_advances` (одна строка на продвижение, продвижения дебаунсить на клиенте ~1-2 с): время прочтения сообщения = `min(at_utc)` среди продвижений с `up_to_seq >= seq`. Если «когда» достаточно приблизительно, журнал можно пропустить (тогда `last_read_at` в `im_members`) — решение за пользователем, по умолчанию рекомендую журнал.
- **Создатель:** `role='creator'` ровно у одного активного участника комнаты; выход Создателя в ТОЙ ЖЕ транзакции передаёт роль самому раннему по `joined_at_utc` участнику (+ системное сообщение). Правило — чистая функция в `trackly-core/src/domain/im/` (юнит-тесты на пустую комнату, единственного участника, одновременный выход). Диалог 1:1: ролей нет, добавить участника нельзя.
- **Реакции ≤ 3 на пользователя на сообщение:** в замыкании записи вставка 4-й удаляет самую старую (`ORDER BY created_at_utc, rowid`) и это ОДНО изменение сообщения (`rev`++). Правило — в `domain/im`.
- **Rich-документ:** хранить JSON-документ редактора (ProseMirror/TipTap-совместимый), НЕ HTML. Сервер валидирует по белому списку узлов/марок, глубине/размеру; `body_text` и `im_mentions` извлекает сам (клиентскому `body_text` не доверять); ссылки — только `http/https/mailto`; картинки — только ссылки на `attachment:{id}` (внешние URL запрещены: нет трекинг-пикселей, CSP `img-src 'self' data: blob:` остаётся). Рендер — на клиенте по той же схеме, отсутствие HTML-инъекции структурное. Валидатор — Rust, в `trackly-core` (по образцу `domain/chat`: `normalize_text`, `exceeds_limit`).
- **Вложения:** добавить `ParentKind::ImMessage` и `ParentKind::ImDraft` (по образцу `MessageDraft`); таблица `attachments` не меняется. Доступ — по членству в беседе (+ `visible_from_seq`). Просмотрщик — тот же `AttachmentViewer` через пакет `packages/attachments`.
- **Поиск:** FTS5 `trigram` (подстрочные совпадения по-русски, без стеммера; индекс больше, при масштабе организации несущественно). Запрос ВСЕГДА соединяется с `im_members` вызывающего (`left_at IS NULL`, `seq >= visible_from_seq`, `deleted_at IS NULL`) ВНУТРИ SQL — иначе фильтрация после `LIMIT` теряет результаты и утекают счётчики. Подтвердить, что `rusqlite` `bundled` собран с FTS5 и `trigram` (тест миграции).
- **Диалог «со всеми пользователями»:** поиск собеседника — по `users` (не по `im_members`); создание диалога идемпотентно через `dialog_key`.
- **Что НЕ делать:** история правок сообщений (вне ТЗ), статусы набора текста/присутствие (требуют эфемерного канала, вне ТЗ), счётчик «прочитано» на сообщение.

---

## 4. Надёжная доставка

### Принцип: сигнал + догрузка (signal-and-pull). Корректность держится на курсорной синхронизации, поток WS — лишь подсказка

**Курсор.** У каждой беседы `last_seq` растёт при ЛЮБОМ изменении сообщений (создание, правка, мягкое/жёсткое удаление, реакция, системное сообщение). Новое сообщение получает `seq = rev = ++last_seq`; прочие изменения только обновляют `rev = ++last_seq`. Назначение — в замыкании единственного писателя (`UPDATE im_conversations SET last_seq = last_seq+1 WHERE id=? RETURNING last_seq`), поэтому монотонность и отсутствие пропусков структурные, без блокировок. Клиент хранит `cursor[conv]` = максимальный применённый `rev`.

**Синхронизация без журнала событий (state-sync):** `POST im_sync {cursors:[{conversation_id, rev}], limit}`. Сервер отвечает для каждой беседы пользователя:
- неизвестная клиенту -> сводка (название, состав, `last_seq`, мой `last_read_seq`) + последняя страница сообщений;
- известная, `rev` отстал -> все сообщения с `rev > cursor` (полные DTO вместе с агрегатом реакций; идемпотентно применимо); если изменений больше порога — `reload:true` (клиент сбрасывает кэш беседы и берёт последнюю страницу);
- беседы, где пользователь больше не участник, -> `removed:[ids]`.
Изменения состава/названия фиксируются системным сообщением (получают `seq` и едут тем же курсором) + актуальная сводка в ответе sync.
Это снимает проблему «нужен неограниченный журнал событий и повторное применение дельт»: восстановление всегда = «дай состояние строк, изменённых после курсора».

**Поток (hint):** событие содержит только `{conversation_id, rev}` (и тип: `im_changed`, `im_removed`, `im_read` для моего указателя с другого устройства), БЕЗ тела сообщения. Клиент: `rev == cursor+1` -> одиночная догрузка; иначе `im_sync`. Тело текста по общему broadcast не ходит никогда (прецедент: `WsEvent::ChatMessage` несёт только id).

**Маршрутизация по пользователям (новый компонент `ImHub`):**
- Новый `AppCtx.im_hub: Arc<ImHub>` (рядом с `ws_broadcast`). Структура: `DashMap<user_id, broadcast::Sender<ImSignal>>` (ёмкость 256, создаётся лениво при первом сокете пользователя; пустые — чистятся).
- Получатели определяются сервисом ВНУТРИ замыкания записи (читает `im_members` в той же транзакции) и возвращаются наружу; публикация — ПОСЛЕ `.await?` и вне замыкания (правило D-04 из `entities_broadcast.rs`). Так откат не создаёт ложных событий, а удалённый из беседы участник получает только `im_removed`.
- Авторизация фан-аута структурна: сигнал физически не попадает в канал пользователя, не являющегося получателем; `is_visible_to` для IM НЕ используется. Старый `WsEvent` и `ws_broadcast` для IM не расширять.
- `http/ws.rs`: `handle_ws_socket` дополнительно подписывается на `im_hub.subscribe(user_id)` второй веткой `select!`. Тот же маршрут `/api/v1/ws` и та же cookie-аутентификация -> Link-клиент (Rust, `tokio-tungstenite`) и браузер идентичны.

**Исправление `Lagged` (долг v1.5-SEEDS) — в двух местах:**
1. Ветка `Lagged(n)` в `ws.rs` вместо `continue` отправляет клиенту кадр `{"type":"resync_required"}` (для общего канала и для канала IM). 
2. Клиент (`ui/src/lib/api/ws.ts` и `packages/transport`) на `resync_required` И на каждый успешный `onopen` после обрыва вызывает `resync` всех подписчиков (существующие хранилища перечитывают данные; IM делает `im_sync`). Дополнительно: сервер шлёт `hello` первым кадром; серверный ping каждые ~25 с и таймаут отсутствия pong ~60 с (сейчас сервер игнорирует входящие и не обнаруживает полуоткрытые сокеты после сна ноутбука — для клиента, живущего днями, это обязательно).
Увеличение буфера 128 — не решение; не делать.

**Идемпотентность отправки:** `client_msg_id` (UUID от клиента) + `UNIQUE(conversation_id, author, client_msg_id)`: повтор после обрыва возвращает уже созданное сообщение. Исходящая очередь клиента — только в памяти/легковесно (оффлайн-режим вне ТЗ), но идемпотентность закладывается сейчас.

**Нагрузка на писателя:** отправка сообщения, реакция, продвижение указателя чтения — всё записи через единственную очередь (mpsc 256, таймаут 5 с). Указатель чтения дебаунсить на клиенте и сливать на сервере (не писать, если `up_to_seq <= last_read_seq`). При ≤20-100 одновременных пользователей достаточно; следить в тестах нагрузки.

**Десктоп Trackly (Tauri-оболочка сервера):** действует от системного «Администратора» (`users.is_system`) — у него нет личности для мессенджера. Рекомендация: в десктопной оболочке Trackly раздел чата скрыт (`isTauri`), чат доступен в браузерной SPA и Link; поэтому tauri_cmds-близнецы для IM-команд пользователя НЕ создаются (осознанное отступление от «один DTO, два транспорта»; проверить, нет ли гейта паритета http<->tauri_cmds, и при наличии — внести исключение). Админские команды раздела «Link» (§6) — двухтранспортные как обычно. **Решение для пользователя (открытый вопрос).**

---

## 5. Сервис обновлений

### Данные и хранение

```
link_releases(id, version, target, arch, file_name, sha256, size_bytes, signature TEXT, notes,
              created_at_utc, uploaded_by_user_id, yanked_at_utc NULL)
              UNIQUE(version, target, arch)
link_channel_heads(channel TEXT, target TEXT, arch TEXT, release_id)       -- PRIMARY KEY (channel,target,arch)
link_user_channel(user_id PK, channel TEXT)                                 -- переопределение для пользователя
link_devices.channel_override TEXT NULL                                     -- переопределение для компьютера (§2)
link_settings: default_channel='stable', min_supported_version (принудительное обновление)
```

Файлы: `<каталог БД>/link_releases/<version>/<file>` (portable: рядом с БД, как вложения; путь настраиваемый — учесть известный открытый вопрос «attachments_path валит запуск»: не наследовать эту ошибку, создавать каталог лениво с понятной ошибкой). Метаданные — в БД, имя на диске не доверять (только из `sha256`+`version`).

**Публикация:** GitHub-релиз Link собирает CI (подписанные артефакты + `.sig`); администратор скачивает и загружает в Trackly (раздел «Link» -> «Релизы»: файл + `.sig`; Tauri-команда принимает путь файла, HTTP — multipart; лимит тела поднять ТОЛЬКО на этом маршруте). Сервер на LAN не обязан иметь выход в интернет. При загрузке сервер проверяет подпись публичным ключом Link (`minisign-verify`, ключ — константа/настройка сервера): неподписанный или чужой артефакт отвергается ДО публикации. Приватного ключа на сервере нет и быть не должно. Загрузка создаёт релиз «неопубликованным»; «Назначить каналу» двигает `link_channel_heads`.

### Эффективный канал (правило приоритета)

`канал = channel_override(компьютер) ?? channel(пользователь) ?? default (stable)`.
Компьютер > пользователь > умолчание: тестовую машину нельзя случайно «вытянуть» входом обычного пользователя, и поведение объяснимо одной строкой в админке («тестовый — потому что назначен компьютеру»). До входа действуют только компьютер и умолчание. Чистая функция в `trackly-core/src/domain/link/` с таблицей истинности в тестах; в админском UI колонка «эффективный канал» + источник. Клиент канала не знает, в UI Link нет ни слова «канал/тестовый».

**Откат:** перенос головы канала на более старый релиз. Для приёма версии НИЖЕ текущей плагин требует `version_comparator` (по документации: сравнение `update.version != current`). Решение: клиент использует comparator «не равно», а защиту от навязанного старья даёт сервер (`min_supported_version`) и подпись. `yanked_at_utc` — исключить релиз из выдачи без удаления файла.

### Контракт эндпоинта (совместим с `tauri-plugin-updater`, проверено по документации плагина)

```
GET /api/v1/link/update/{target}/{arch}/{current_version}
    заголовки: X-Link-Device: <uuid> (+ session cookie, если есть)
    200 {"version":"1.2.3","url":"https://<server>/api/v1/link/artifact/<release_id>",
         "signature":"<содержимое .sig>","notes":"...","pub_date":"<RFC3339>"}
    204 если обновления нет
GET /api/v1/link/artifact/{release_id}      -- ТОЛЬКО для известного device_id; отдаёт файл потоково
```

- `target` = `windows|darwin|linux`, `arch` = `x86_64|aarch64` (шаблонные переменные плагина; пользовательские переменные плагин не поддерживает — идентичность устройства передаётся заголовком через builder: `app.updater_builder().header(...).endpoints(vec![url])`, поэтому проверка обновления запускается из Rust, не из JS).
- Маршруты ДОЛЖНЫ быть зарегистрированы явно: иначе `spa_fallback` вернёт `/api/` -> 404 JSON (поведение уже есть, F17), а клиент увидит «нет релиза» вместо ошибки; добавить тест на 204 vs 404 и на формат JSON.
- Целостность — подпись ed25519 (плагин проверяет `pubkey`). Установка Windows — NSIS currentUser; режим `passive`/`quiet`: `quiet` допустим именно потому, что установка пользовательская (по документации «quiet» работает только для user-wide). Перезапуск после обновления — штатный (`on_before_exit`/restart).
- Раздача первичной установки: `GET /api/v1/link/download/{target}` (текущий stable-установщик) + страница в Trackly для ПК без клиента; для GPO-скрипта — стабильный URL с `?device=` не нужен.

---

## 6. Раздел «Link» в Trackly и права

**Новые `Action`** (`trackly-core/src/auth.rs`; `authorize` — исчерпывающий `match`, добавить в таблицу тестов):

| Action | Роли | Что открывает |
|--------|------|---------------|
| `UseMessenger` | Admin, Manager, Employee | Все пользовательские `im_*` эндпоинты (чтение/запись только своих бесед; членство проверяет сервис, как `ChatService::authorize`) |
| `ManageLink` | Admin | Релизы, назначение каналов (пользователь/компьютер/голова канала), отзыв клиентов, `min_supported_version`, настройки |
| `ModerateMessenger` | Admin | Список мягко удалённых сообщений (только удалённых; чужие НЕудалённые диалоги админу не видны), жёсткое удаление |
| `ReadLinkClients` (опц.) | Admin, Manager | Только чтение списка подключённых клиентов |

Модерация пишет в `audit_log` (просмотр удалённого и purge — с автором действия). Админ НЕ становится участником бесед.

**UI:** новый маршрут `/link` в `ui/src/routes.ts` (`pages/LinkPage.svelte` с вкладками «Релизы», «Клиенты», «Модерация») ТОЛЬКО в карте для Admin/Manager; в `employeeRoutes` добавить `/chat` (мессенджер доступен сотруднику) и НЕ добавлять `/link` (остаётся `AccessDenied` через `'*'`); обновить `check-employee-routes.mjs`/`check-employee-tabs.mjs` и боковое меню. Маршрут `/chat` — и в admin-карту. «Подключённые клиенты» = `link_devices` + онлайн-признак из `ImHub`/реестра сокетов (в памяти).

---

## 7. Рекомендуемый порядок фаз v1.5 (нумерация с 42)

Заметка: прежние «42–45» (карта) перенесены в v1.8; вторичное использование номеров допустимо только после согласования с ROADMAP.

| # | Фаза | Содержание | Зависит от |
|---|------|-----------|-----------|
| 42 | Монорепо-основа | `pnpm-workspace`, `packages/design|kit|transport|attachments` (перенос токенов/kit/вложений + починка гейтов), `trackly-proto` + `proto-ts` с гейтом дрейфа, каркас `link-app`/`link-ui` (пустое окно, трей, NSIS currentUser собирается), `release-link.yml` в dry-run, composite action, независимая версия | — |
| 43 | Фундамент доставки | `resync_required` вместо молчаливого `Lagged`, серверный ping/`hello`, клиентский resync на `onopen`, `ImHub` (пока без IM-сервиса) + тесты «медленный потребитель», «чужой пользователь не получает» | 42 (частично; серверную часть можно параллельно) |
| 44 | Клиент: транспорт и вход | Rust `LinkTransport` (reqwest rustls, пиннинг TOFU, cookie jar), `api_call` с allowlist, WS-клиент с backoff/heartbeat, `link_devices`, `device_token`/`link_resume`, вход паролем и SPNEGO (нативный SSPI), автозапуск, single-instance, хранилище секретов | 42, 43 |
| 45 | Обновления и дистрибуция | `link_releases`/каналы/эндпоинт, загрузка и проверка подписи, раздел «Link»: Релизы + Клиенты (минимум), updater в клиенте, откат, NSIS-инструкция GPO, E2E «0.0.1 -> 0.0.2», проверка обоих каналов | 44 |
| 46 | Дизайн-прототипы (ворота) | Интерактивные прототипы экранов чата (список бесед, диалог/комната, редактор, реакции, «Информация о сообщении», поиск, уведомления/трей, раздел «Link»). Работа не зависит от кода и должна идти ПАРАЛЛЕЛЬНО фазам 43-45; результат утверждается до UI-фаз чата | — (старт сразу) |
| 47 | Чат: сервер | Миграции `im_*` + FTS5, `domain/im` (правила Создателя, лимит реакций, валидатор документа), `ImService` + HTTP, `im_sync`, `ParentKind::Im*`, `Action`'ы, модерация, тесты инвариантов (seq без пропусков, матрица доступа, фан-аут, идемпотентность) | 43 |
| 48 | Чат: UI-ядро (SPA) | `packages/messenger`: список бесед, диалоги/комнаты, отправка простого текста, непрочитанное, указатель чтения, `im_sync`-клиент, маршрут `/chat` в Trackly (браузер) | 46, 47, 42 |
| 49 | Чат: редактор и вложения | Rich-редактор (заголовки… таблицы с объединением ячеек, код, @упоминания, ответ), SVG-эмодзи, вложения с просмотром | 48 |
| 50 | Чат: сообщения-операции | Правка («карандаш»), мягкое/жёсткое удаление, реакции с облачком/меню, «Информация о сообщении», поиск | 49 |
| 51 | Link: чат и уведомления | Подключение `packages/messenger` в `link-ui`, тосты/системные уведомления, mute, счётчики, индикатор в иконке трея, «закрыть только из меню трея» | 44, 48 (параллельно 49-50, догоняет по функциям) |
| 52 | Раздел «Link» (довершение) и закрытие | Вкладка «Модерация», каналы по пользователям/компьютерам в UI, сквозной UAT на Windows (SPNEGO, GPO-установка, обновление), аудит приватности фикстур | 45, 50, 51 |

**Обоснование порядка**
- Обновления (45) РАНЬШЕ чата: клиент без работающего самообновления нельзя «исправить потом» на сотнях ПК без прав администратора; сначала доказываем, что цепочка CI -> подпись -> сервер -> канал -> клиент работает на пустом приложении.
- Доставка (43) и серверная модель (47) — до любого UI: UI строится поверх проверенного контракта `im_sync`.
- Прототипы (46) идут параллельно, чтобы не блокировать 48.
- 51 можно вести параллельно 49-50: сначала доставляется ядро (диалоги, текст, непрочитанное), затем наращивается.

**Research flags для фаз**
- **44 — нужен спайк (HIGH риск):** клиентский SPNEGO на Windows из Rust (нативный SSPI против клиента крейта `sspi`; выбор SPN; поведение вне домена/на «странных» ПК); пиннинг и хранение секретов (`keyring`) на трёх ОС; живая проверка на Windows (dev на macOS, домена нет — реальный AD недоступен, значит верификация через ручную UAT, как в 31/32).
- **45 — спайк:** принимает ли актуальный `tauri-plugin-updater` небезопасный TLS для самоподписанного сервера (флаг в конфиге/builder) и как ведёт себя `version_comparator` при откате; подпись и порядок `createUpdaterArtifacts` для NSIS/AppImage/macOS (`.app.tar.gz`).
- **47 — проверить тестом:** FTS5 + `trigram` в сборке `rusqlite bundled`; поведение `UNIQUE(conversation_id, author, client_msg_id)` с NULL-ами.
- **49 — отдельное исследование редактора** (выбор TipTap/ProseMirror в Svelte 5, объединение ячеек таблицы, схема документа, мобильные ограничения нет) — относится к STACK, но схема документа влияет на валидатор из 47: зафиксировать схему узлов ДО 47.
- **42 — стандартные паттерны**, но высокая механическая цена (гейты по путям): закладывать запас.
- **48/50/51:** стандартные паттерны при готовых 43/47.

---

## Компоненты: новое и изменяемое

| Компонент | Статус | Файлы |
|-----------|--------|-------|
| `trackly-proto` | NEW | `crates/trackly-proto/` |
| `link-app` | NEW | `crates/link-app/` (трей, `LinkTransport`, updater, SSPI, хранилище секретов) |
| `link-ui`, `packages/*` | NEW | корень репозитория |
| `ImHub`, `ImService`, `LinkService` (релизы/каналы/устройства) | NEW | `crates/trackly-app/src/services/`, `server/im_hub.rs` |
| `http/im.rs`, `http/link.rs`, `http/link_update.rs` | NEW | `crates/trackly-app/src/http/` (+ `.merge(...)` в `build_router`) |
| `repos/im_sqlite.rs`, `repos/link_sqlite.rs` | NEW | `crates/trackly-infra/src/repos/` |
| `domain/im/*`, `domain/link/*` | NEW | `crates/trackly-core/src/domain/` |
| `migrations/V058..` | NEW | `migrations/` (каждая с `PRAGMA user_version`) |
| `http/ws.rs` | MODIFIED | `resync_required`, `hello`, ping/pong, ветка `ImHub` |
| `context.rs` (`AppCtx`) | MODIFIED | поля `im`, `im_hub`, `link` (+ `build()`); `desktop_event_bridge.rs` — без изменений (IM в десктопе скрыт) |
| `http/mod.rs` | MODIFIED | merge новых роутеров; `DefaultBodyLimit` на маршруте загрузки релиза |
| `trackly-core/src/auth.rs` | MODIFIED | 3-4 новых `Action`, тесты |
| `domain/attachments.rs` | MODIFIED | `ParentKind::ImMessage`, `ImDraft` |
| `ui/src/lib/api/ws.ts`, `client.ts`, `routes.ts`, лейауты | MODIFIED | `resync`, маршруты `/chat`, `/link`, пункты меню |
| `.github/workflows/*`, `ci-fast.yml` | MODIFIED/NEW | `release-link.yml`, composite action, отдельный job Link |

## Anti-patterns (специфичные для v1.5)

- **Расширять `WsEvent`/`ws_broadcast` событиями чата.** Фильтр по `Identity` не знает членства, тело сообщения утечёт всем или потребует копии членства в событии.
- **Сливать IM с `chat_messages` запросов.** Ломает неизменяемость D-22 и 15 гейтов 41.5.
- **Физически удалять строку сообщения.** Дыра в `seq`, висячие `reply_to_id`, потеря контракта курсора.
- **Доверять `body_text`/HTML от клиента.** Источник правды — документ, валидируемый сервером.
- **Сеть из webview Link.** Самоподписанный TLS, SameSite, CORS и эксфильтрация при XSS в rich-тексте.
- **Выкатывать клиент без работающего обновления** — исправить будет нечем.
- **`version.workspace = true` у Link** при существующей инъекции версии в `release.yml`.
- **Фикстуры чата с реальными ФИО/названиями** (жёсткое условие приватности): только вымышленные («Иванов И.И.», «Петров П.П.»); прогон `scripts/check-privacy.mjs` до коммита, включая `.planning/`.

## Масштаб

| Показатель | ~50 пользователей | ~500 | Замечание |
|-----------|-------------------|------|-----------|
| Одновременные WS | 50 (по одному на пользователя; Link + браузер = до 2) | 500-1000 | `DashMap` по пользователям, tokio достаточно |
| Записи чата | единицы/с | десятки/с | Единственный писатель + очередь 256; указатель чтения дебаунсить |
| FTS trigram | мегабайты | сотни МБ | Приемлемо; при росте — смена токенайзера |
| Файлы релизов | ~30-80 МБ/версия/платформа | — | Хранить N последних, остальные архивировать вручную |

## Источники

- Код репозитория (пути в каждом разделе) — HIGH.
- Tauri updater: `https://v2.tauri.app/plugin/updater/` (формат ответа 200/204, шаблонные переменные, `version_comparator`, `installMode`, `endpoints()`/`header()` в builder) — HIGH; Context7 `/tauri-apps/plugins-workspace` (`UpdaterBuilder::endpoints`, валидация https, применение флагов небезопасного TLS к reqwest-клиенту при `check`) — MEDIUM (имя флага принятия невалидных сертификатов проверить спайком).
- Остальное (SSPI-клиент, `keyring`, FTS5 trigram в `rusqlite bundled`, схема TipTap) — знание модели, НЕ проверено по актуальной документации: LOW до спайков.
