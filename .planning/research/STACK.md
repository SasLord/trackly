# Technology Stack — milestone v1.5 «Trackly Link — клиент и Чат»

**Project:** Trackly (SUBSEQUENT milestone: only NEW capabilities)
**Researched:** 2026-11-25
**Scope:** новый крейт-клиент Link, трей/автозапуск, NSIS per-user, апдейтер, SPNEGO-клиент, pinning, rich-text чат, эмодзи, FTS5, надёжная WS-доставка.
**Overall confidence:** MEDIUM-HIGH (версии сверены с crates.io/npm 2026-11-25; SSPI-клиент и pinning-схема — требуют спайка).

Базовые факты репозитория (Cargo.toml / package.json, не CLAUDE.md): Rust 1.92.0, rusqlite 0.41, refinery 0.10, tauri 2 (lock — 2.12.x), specta `=2.0.0-rc.22`, tauri-specta `=2.0.0-rc.21`, tokio-rustls 0.26, rustls 0.23 (feature `ring`), ldap3 0.12.1 `tls-rustls-ring`, tower-sessions 0.15, `windows` 0.62 (только `UI_Notifications`), svelte ^5.55, vite ^6, @sveltejs/vite-plugin-svelte ^5.1, svelte-spa-router, pnpm 10.17.

## Recommended Stack — Rust (Link client + server additions)

### Tauri-плагины клиента (все официальные, v2, MIT/Apache-2.0)

| Crate / npm | Версия (проверено) | Purpose | Why / интеграция |
|---|---|---|---|
| `tauri` | `2` (актуальна 2.12.7; уже в workspace) | Оболочка Link | Трей — встроенная фича `tray-icon` (добавить в features клиента); смена иконки трея `TrayIcon::set_icon` для «точки непрочитанного». Никакого отдельного плагина трея. |
| `tauri-plugin-autostart` / `@tauri-apps/plugin-autostart` | 2.7.0 | Автозапуск | Windows: ключ `HKCU\...\Run` (per-user, без админа — совпадает с моделью per-user NSIS); macOS: `MacosLauncher::LaunchAgent` (user LaunchAgent; при желании login item), Linux: `~/.config/autostart/*.desktop`. Передавать аргумент `--hidden`, чтобы старт с системой не показывал окно. |
| `tauri-plugin-single-instance` | 2 (2.5.5; уже в workspace) | Один экземпляр | Колбэк второго запуска -> показать/вывести окно из трея. Уже используется в Trackly — переиспользовать. |
| `tauri-plugin-updater` / `@tauri-apps/plugin-updater` | 2.14.1 (MSRV 1.90 — ок для 1.92) | Автообновление | Только в Link (в Trackly portable — по-прежнему НЕ подключать). См. раздел «Updater». |
| `tauri-plugin-notification` | 2 (2.8.1; уже в workspace) | Системные уведомления | Уже решены грабли 41.5 (клик по тосту, `POWERSHELL_APP_ID` для unpackaged). Link установлен через NSIS -> AUMID/ярлык в меню «Пуск» создаёт установщик, это упрощает toast по сравнению с portable; всё равно проверить на Windows. Вынести `notify_desktop` в общий крейт. |
| `tauri-plugin-window-state` | 2.5.0 | Запоминание размера/позиции окна чата | Опционально; флаги без `VISIBLE`, чтобы не ломать «старт скрытым». |
| `tauri-plugin-opener` | 2.7.3 | Открытие внешних ссылок из сообщений | Для Link заменяет `shell.open` (в Trackly остаётся shell). Ограничить схемы http/https через capability. |
| `keyring-core` + `windows-native-keyring-store` + `apple-native-keyring-store` + `zbus-secret-service-keyring-store` | 1.1.0 / 1.1.0 / 1.0.3 / 1.0.1 | Хранение токена в ОС | keyring 4.x (4.2.0) — тонкий фасад; рекомендуемый путь авторов — `keyring-core` + отдельный крейт хранилища под платформу (`cfg(target_os)`). Альтернатива «keyring 4 с default feature `v1`» проще, но тянет лишнее; брать модульный вариант. Windows Credential Manager (DPAPI) — per-user, без админа. На Linux без Secret Service (headless/WSL) — fallback: токен только в памяти + повторный вход (НЕ писать токен в файл открытым текстом). MEDIUM: API v4 менялся — проверить спайком. |

Overlay (v1.6, полноэкранное always-on-top): осуществимо на чистом Tauri 2 `WebviewWindowBuilder` (`always_on_top(true)`, `fullscreen(true)`/`maximized`, `decorations(false)`, `skip_taskbar(true)`, `focused`), отдельная метка окна и отдельный route SPA. Windows — работает; macOS — поверх fullscreen-приложений требует NSWindow level/`visible_on_all_workspaces` (частично через Tauri, остаток — `objc2` вручную); Linux/Wayland — always-on-top ненадёжен (X11 ок). Вердикт: YES на Windows (primary), MAYBE macOS/Linux. Новых зависимостей в v1.5 не нужно. (HIGH для Windows, LOW для Wayland.)

### Сетевой слой клиента (Rust-сторона): pinning + SPNEGO + WS

| Crate | Версия | Purpose | Why |
|---|---|---|---|
| `reqwest` | 0.13.5, `default-features = false`, features `rustls` (или `rustls-tls`-эквивалент 0.13), `json`, `stream`, `multipart` | HTTP-клиент Link (REST, загрузка вложений, апдейтер использует свой внутренний) | Проверить точные имена фич TLS в 0.13 (в 0.13 rustls стал дефолтным бэкендом, провайдер по умолчанию — aws-lc). Проект везде на `ring` -> принудительно использовать `use_preconfigured_tls(rustls::ClientConfig)` с ring-провайдером и своим verifier, `native-tls` не включать. |
| `rustls` (уже есть, 0.23.46) | 0.23 + `dangerous_configuration` API (`ServerCertVerifier`) | Pinning самоподписанного сертификата | Кастомный `ServerCertVerifier`: сравнивает SHA-256 DER-сертификата (или SPKI) листа с сохранённым отпечатком; chain/hostname не проверяются (LAN-хост по IP/имени). `sha2` уже в workspace. Модель: отпечаток вводится/подтверждается пользователем при первом подключении (TOFU с показом отпечатка) либо передаётся в конфиге установщика/GPO-скрипта; смена отпечатка = жёсткая ошибка + явное подтверждение. Серверный сертификат генерирует `rcgen` (уже есть). |
| `tokio-tungstenite` | 0.30.0 (в dev-deps сейчас 0.29 — выровнять) | WS-клиент Link | `Connector::Rustls(ClientConfig)` с тем же verifier -> один pinned-конфиг на HTTP и WS. Feature `rustls-tls-native-roots` НЕ нужна (корни не используются). |
| `windows` (уже 0.62, расширить features) | 0.62 | SPNEGO через родной SSPI | Фичи `Win32_Security_Authentication_Identity`, `Win32_Security_Credentials`, `Win32_Foundation`. `AcquireCredentialsHandleW("Negotiate", SECPKG_CRED_OUTBOUND, NULL identity)` -> `InitializeSecurityContextW` с SPN `HTTP/<fqdn сервера>` -> base64 -> `Authorization: Negotiate`. Текущие креды сеанса — ровно то, что нужно (без пароля). ~150 LOC под `cfg(windows)`, свой тонкий модуль + mock-трейт для macOS-разработки (как AD_MOCK). Рекомендовано. |

Рассмотренные и отклонённые для SPNEGO-клиента:
- **`sspi` 0.23.1** (Devolutions, MSRV 1.93!) — pure-Rust Kerberos/NTLM; на Windows умеет обёртку над нативным SSPI, но документация про «текущего пользователя без пароля» скупа, API генераторов тяжёл, MSRV 1.93 > тулчейна 1.92. Не брать.
- **`kenobi` 0.4.2 / `kenobi-windows` 0.3.1** — кроссплатформенный Negotiate-клиент (SSPI/GSSAPI), маленький автор, документация 9%. LOW. Только как референс реализации.
- `cross-krb5` / `libgssapi` — для macOS/Linux Kerberos; в v1.5 НЕ нужны (там логин/пароль; AD-пользователи с Mac/Linux — редкий случай, отложить).
- `reqwest` «negotiate» из коробки отсутствует; обработку 401 `WWW-Authenticate: Negotiate` + многошаговый обмен делать вручную (сервер уже реализует SPNEGO в `http/sso.rs` — клиентский токен должен быть совместим с его обработчиком; проверить, что серверный путь принимает токен из не-браузерного клиента, и что результат — сессионный токен, а не cookie).
- Почему не SSO через webview: WebView2 сам умеет Negotiate только для интранет-зон и не даёт получить токен для хранения; на самоподписанном сертификате webview вообще отказывает.

### Все запросы идут через Rust, а не через fetch из webview (архитектурное решение)

Рекомендация: в Link webview НЕ ходит в сеть сам. Причины: (1) WebView2/WKWebView отвергают самоподписанный серверный сертификат, обход есть только на Windows (`ServerCertificateErrorDetected`), нет в WKWebView — единый путь возможен только через Rust; (2) pinning и SPNEGO живут в Rust; (3) токен из keyring не попадает в JS; (4) CSP можно закрыть `connect-src 'self' ipc:`.
- Общий фронтенд получает интерфейс `Transport` (`request(method, path, body)`, `subscribe(ws)`): `BrowserTransport` (fetch + cookie, как сейчас в Trackly) и `TauriTransport` (`invoke('api_request')`, события `ws:event` через `Channel`/`emit`). Расширить существующий слой `ui/src/lib/api` под эту абстракцию; специфичные для Tauri-десктопа Trackly «команды» остаются как есть.
- Картинки/вложения в `<img>`/`<video>`: зарегистрировать custom URI scheme (`register_asynchronous_uri_scheme_protocol("link", ...)`), который берёт ресурс через pinned-клиент. Без этого вложения чата в клиенте не отобразятся.
- НЕ использовать `tauri-plugin-http` 2.9.0: поверх reqwest, но без хука под кастомный `ClientConfig` — pinning не подключить.

### Updater: схема, подпись, откат, per-user

| Решение | Детали |
|---|---|
| Endpoint | `plugins.updater.endpoints = ["https://<сервер>/api/v1/link/update/{{target}}/{{arch}}/{{current_version}}"]` — динамический, отвечает 204 или 200 JSON `{version,url,signature,notes,pub_date}` (проверено по официальной документации). Канал определяется ТОЛЬКО на сервере по identity клиента (токен пользователя + идентификатор ПК) — в URL/клиенте канала нет, что и требует PROJECT.md. Кастомные переменные URL не поддерживаются — идентичность передавать заголовками (`UpdaterBuilder::header`) из Rust. |
| TLS апдейтера | Updater использует собственный reqwest; самоподписанный сертификат сервера не пройдёт. Варианты: (a) собирать апдейтер в Rust через `app.updater_builder()` и добавлять доверенный корень (`UpdaterBuilder` принимает `proxy/headers/timeout`, но не rustls-конфиг — проверить спайком, LOW); (b) надёжнее: Rust-код Link сам делает `GET` метаданных через pinned-клиент, скачивает артефакт во временный файл, проверяет minisign-подпись `minisign-verify` 0.3.0 и запускает установщик (NSIS `/S`). Это потеря удобства плагина, но снимает проблему TLS и упрощает откат. Рекомендация: **спайк в первой фазе клиента, на выходе — выбор (a)/(b)**; закладывать (b) как запасной план. |
| Откат (downgrade) | Поддержан официально: `updater_builder().version_comparator(\|current, update\| update.version != current)` — сервер сам решает, какую версию отдать (в том числе старую). Комбинируется с динамическим endpoint. |
| Подпись | `tauri signer generate` -> ed25519 (minisign-формат). Публичный ключ — в `tauri.conf.json` (`plugins.updater.pubkey`, строка, не путь). Приватный — только секрет CI (`TAURI_SIGNING_PRIVATE_KEY` + `_PASSWORD`), `bundle.createUpdaterArtifacts: true`. Серверу нужны `.sig` и артефакт; сервер НЕ подписывает. |
| Windows per-user | `bundle.windows.nsis.installMode = "currentUser"` (дефолт; ставит в `%LOCALAPPDATA%`, админ не нужен — подтверждено документацией). Апдейтер `windows.installMode = "passive"` (дефолт) или `"quiet"`: quiet работает без админа именно для user-wide установок — наш случай. Переход перемашинной установки -> per-user не поддерживать (ломает апдейтер: правка в Program Files требует админа). |
| Тихая установка для GPO | NSIS-установщик Tauri принимает стандартные `/S` (тихо), `/D=<путь>` (последним). Документация Tauri их не перечисляет — проверить на реальном Windows (MEDIUM). Скрипт входа: «если нет `%LOCALAPPDATA%\Link\` -> `\\share\Link-setup.exe /S`». WebView2: `webviewInstallMode = embedBootstrapper` (1.8 МБ) либо `offlineInstaller` (+127 МБ) для ПК без интернета — решить по сети организации (Windows 10/11 обычно уже имеют WebView2). Идентификатор приложения Link — отдельный от Trackly (`identifier`), иначе общие хранилища webview/конфликты NSIS. |
| macOS / Linux | `.app`/DMG (ручная установка, автозапуск плагином), AppImage (self-update плагина работает с AppImage). Подпись macOS — как сейчас (`signingIdentity: "-"`), апдейтер на macOS с ad-hoc подписью — проверить поведение Gatekeeper (LOW; допустимо «обновление только Windows» в v1.5). |
| CI | `tauri-apps/tauri-action@v0` с `includeUpdaterJson: false` (JSON-манифест генерирует сервер), артефакты и `.sig` загружаются как релиз-ассеты и импортируются сервером (раздел «Link» -> релизы/каналы). Теги клиента отдельным префиксом (`link-v*`), чтобы не триггерить `release.yml` Trackly (`v*.*.*`). |

### Серверная часть: надёжная доставка, чат, поиск

| Что | Решение | Why |
|---|---|---|
| Порядковые номера и догрузка | Без новых крейтов. Таблица `chat_events`/поле `seq INTEGER` per-conversation (монотонный счётчик, выдаётся внутри single-writer транзакции) + per-user `event_seq` для личных сигналов (прочитано, упоминание); WS-пуш содержит `seq`; клиент хранит курсор и после `Lagged`/реконнекта/сна запрашивает `GET /sync?cursor=...`. | `tokio::sync::broadcast(128)` остаётся каналом «пинка»; источник истины — БД. `Lagged` -> просто триггер ресинка. Single-writer уже гарантирует отсутствие дыр. |
| Валидация rich-text JSON | Без внешнего крейта: строго типизированная модель в `trackly-core` (`serde` enum `Node`/`Mark` с `#[serde(tag="type")]`, `deny_unknown_fields`, белый список типов/атрибутов, лимиты глубины/размера/числа узлов, allowlist схем URL `http/https/mailto`, `src` картинок — только ссылки на вложения Trackly), округляющая «десериализация = валидация». Из дерева же извлекается plain-text для FTS5 и превью в уведомлениях. DTO — через `specta` (уже есть) -> TS-типы. | Крейт `prosemirror` 0.5.2 (Rust-порт модели) — молодой, избыточен; `jsonschema` 0.63.1 + `schemars` 1.2.3 — альтернатива для декларативной схемы, но хуже сообщения об ошибках и дубль с serde-типами. Не брать. |
| Санитизация | Рендер на клиенте из JSON (собственный Svelte-рендерер узлов или `@tiptap/html` `generateHTML` + DOMPurify) — НЕ хранить HTML. Серверу `ammonia` 4.2.5 нужен только если сервер когда-либо отдаёт HTML (письма/печать) — в v1.5 не добавлять. | JSON-схема сводит XSS-поверхность к рендереру; HTML в БД не нужен. |
| Поиск по сообщениям | SQLite FTS5 (уже в проекте с кириллическим токенайзером): виртуальная таблица `chat_messages_fts` (external content + триггеры или явное обновление из single-writer), индексируется plain-text из JSON. Права: фильтровать по участию в диалоге/комнате ДО выдачи (JOIN с членством). Без новых зависимостей; миграция refinery + `PRAGMA user_version` (см. память про V050). | |
| Реакции/прочтения/упоминания | Обычные таблицы SQLite; лимит «3 реакции» и «самая старая снимается» — в транзакции сервиса. | |

## Recommended Stack — Frontend

### Монорепо / общий пакет
- **pnpm workspace** (`pnpm-workspace.yaml`): пакеты `packages/ui-core` (общие компоненты, токены SCSS, API-клиент `Transport`, чат-фича, сгенерированные specta-биндинги), `ui/` (Trackly), `link-ui/` (Link, свой `vite.config`, свой роутинг/окна). Биндинги: один `bindings.ts` из `tests/export_bindings.rs` (сейчас `prebuild` в `ui/package.json` запускает cargo test) — общий пакет импортирует его; экспорт DTO для Link делать тем же тестом (или `trackly-core` держит DTO, оба приложения подтягивают). Важный риск: линт-скрипты `ui/scripts/check-*.mjs` ищут файлы по путям `ui/src/...` — при переносе в `packages/` их пути ломаются (память про JS-зеркала и гейты); переносить поэтапно с правкой путей и `check-lint-selftest-wiring`.
- Новых инструментов сборки не нужно (Vite 6 + `@sveltejs/vite-plugin-svelte` ^5 + `svelte-spa-router` — остаются).

### Rich-text редактор: TipTap 3 (все MIT, версии 3.35.0 на 2026-11-25)

| npm | Версия | Назначение |
|---|---|---|
| `@tiptap/core`, `@tiptap/pm` | ^3.35 | Ядро, ProseMirror (single `@tiptap/pm` — обязателен, иначе дубли prosemirror-* и сломанные плагины) |
| `@tiptap/starter-kit` | ^3.35 | В v3 включает bold/italic/strike/code/heading/blockquote/lists/code-block/history, а также Underline и Link (TipTap 3 перенёс их в StarterKit; проверить `configure`/`false`-отключение на спайке — страница доков при проверке вернула 404, MEDIUM) |
| `@tiptap/extension-table` (экспортирует Table, TableRow, TableCell, TableHeader, TableKit) | ^3.35 | Таблицы; `mergeCells`/`splitCell`/`toggleHeaderRow` — команды на базе `prosemirror-tables` (MIT, OSS) — объединение ячеек покрыто БЕСПЛАТНО |
| `@tiptap/extension-subscript`, `@tiptap/extension-superscript` | ^3.35 | Индексы |
| `@tiptap/extension-text-align` | ^3.35 | Выравнивание |
| `@tiptap/extension-image` | ^3.35 | Картинки (src ограничить схемой вложений; настроить node view) |
| `@tiptap/extension-mention` + `@tiptap/suggestion` | ^3.35 | @упоминания (popup — свой Svelte-компонент + `@floating-ui/dom` ^1.8) |
| `@tiptap/extension-text-style` (при необходимости) | ^3.35 | Не нужен, если нет цветов/размеров — не брать |
| `@tiptap/extension-code-block-lowlight` + `lowlight` 3.3.0 | ^3.35 / ^3.3 | Подсветка кода — ОПЦИОНАЛЬНО (в списке требований просто «код»); по умолчанию обойтись StarterKit CodeBlock без подсветки |
| `svelte-tiptap` | 3.0.2 (peer: svelte ^5, @tiptap ^3, `@floating-ui/dom`, bubble/floating-menu) | Обёртка под Svelte 5. Это тонкая обвязка (`createEditor`, `EditorContent`) — допустимо, но можно и без неё: `new Editor({ element })` + `$effect` cleanup. Рекомендация: взять напрямую `@tiptap/core` (без обёртки), чтобы не зависеть от темпа обновлений стороннего пакета; MEDIUM. |
| Отступ (indent) | Собственное расширение `Indent` (глобальный атрибут на paragraph/heading, ограничение 0..N) ~40 LOC; готовое — только в Pro/сторонних. | Не брать Pro-расширения: `@tiptap-pro/*` и «Tiptap Cloud» — платные/реестр с токеном; в публичном репо недопустимо. |
| Ответ на сообщение | Не расширение редактора — отдельное поле `reply_to_message_id` в модели сообщения + UI-панель над редактором. | |

Хранение: `editor.getJSON()` -> ProseMirror JSON -> сервер валидирует (см. выше) -> рендер на клиенте. Версия схемы документа (`schema_version`) хранить в строке сообщения для будущих миграций.
Версионирование: зафиксировать все `@tiptap/*` на ОДНУ версию (pnpm `overrides`/exact), иначе v3 минорные расхождения ломают типы.

### Эмодзи: Twemoji (jdecked fork)

| Вариант | Лицензия | Вердикт |
|---|---|---|
| **Twemoji jdecked v17.0.3** (Emoji 17, релиз 2026-06-01) | код MIT, графика **CC-BY 4.0** (нужна атрибуция) | **Выбор.** Единый SVG-набор, знакомый вид, поддерживается, npm `@twemoji/api` 17.0.3 / `@twemoji/parser` 17.0.2 (MIT). Атрибуция -> `THIRD-PARTY-NOTICES.md` (уже ресурс бандла) + экран «О программе». |
| Fluent Emoji (Microsoft) | MIT | Хорош визуально («Flat»), но большие наборы, нестандартные имена файлов, более громоздкий пайплайн; запасной вариант. |
| OpenMoji | CC-BY-SA 4.0 | ShareAlike — избегать. |
| Noto Emoji (Google) | OFL/Apache-2.0 (шрифт) / Apache-2.0 | Работает, но тяжелее; не нужен. |

- Доставка: SVG не тянуть из CDN (LAN без интернета). Положить подмножество `assets/svg/*.svg` (~3.7k файлов, единицы МБ) в статику (`public/emoji/`), копируя по скрипту из версии-тега при сборке (НЕ коммитить 3.7k файлов вручную без скрипта/хэша — решить: коммит прюненного набора + `THIRD-PARTY-NOTICES`). Отдавать с `Cache-Control: immutable`; грузить лениво (`loading="lazy"`), в Link — через `link://`-протокол или встроить в ресурсы приложения. Embedded в `rust-embed` бинарь сервера — +несколько МБ, приемлемо.
- Парсинг текста -> эмодзи: `@twemoji/parser` (MIT) даёт позиции и пути к SVG; в сообщении хранить Unicode-символы (поиск/копирование работают), подменять на `<img class="emoji" alt="😀">` при рендере и в редакторе (inline atom-ноду `emoji` с `alt`). `emoji-regex-xs` 2.0.1 — лёгкая альтернатива для детекции.
- Данные и русский поиск для пикера: `emojibase-data` 18.0.0 (MIT) — локаль `ru` с названиями и тегами, группы, `skins`. Фильтровать по `version <= 17`, чтобы не показывать эмодзи без SVG (Emojibase 18 новее Twemoji 17).
- Компонент пикера: **писать свой** на Svelte 5 (виртуализованная сетка по категориям, недавние/частые в localStorage, поиск по ru-тегам, тон кожи). `emoji-picker-element` 1.29.1 (Apache-2.0, web component, IndexedDB) рендерит нативные глифы — не даёт единый SVG-вид без обходов; не брать.

### Прочее на фронте

| npm | Версия | Зачем |
|---|---|---|
| `@tauri-apps/api` | ^2.12 | `Channel`, события, `invoke` для `TauriTransport` (уже есть ^2.11) |
| `@tauri-apps/plugin-autostart` / `-updater` / `-opener` / `-window-state` | 2.7.0 / 2.14.1 / 2.7.3 / 2.5.0 | Только в `link-ui` |
| `dompurify` 3.5.0 (MPL-2.0 OR Apache-2.0) | ^3.5 | Защитный слой, если рендер идёт через HTML-строку; если свой рендер узлов — не нужен. Предпочтительно свой рендерер (нет HTML-строк). |
| `@tauri-apps/cli` | ^2.12 (сейчас ^2.11) | Тот же CLI для обоих приложений; `link-ui` вызывает через `pnpm --filter`. |

## Структура workspace (интеграция)

- Новый крейт `crates/trackly-client` (Tauri 2, `[[bin]] name="link"`), в `members` корневого `Cargo.toml`; свой `tauri.conf.json` (`identifier` отдельный, `bundle.createUpdaterArtifacts`, NSIS `currentUser`). Зависит от `trackly-core` (DTO, валидатор rich-text) — НЕ от `trackly-app`/`trackly-infra` (в клиент не должны попасть rusqlite/axum/ldap3 -> меньше размер, чище лицензии).
- Общие зависимости — через `[workspace.dependencies]` (новые: `reqwest`, `tokio-tungstenite`, `keyring-core` + store-крейты, плагины autostart/updater/opener/window-state).
- Убедиться, что workspace feature-unification не воспроизводит историю с `snmp2`/`zip` (память: unification на Windows ломает сборку) — после добавления `reqwest`/`rustls` провайдеров прогнать `cargo tree -d` и Windows-CI на отдельном крейте; провайдер rustls — только `ring` (в клиенте `aws-lc-rs` не включать: C-toolchain/NASM на Windows CI).
- CI: отдельный job/workflow `link-release` (теги `link-v*.*.*`), тест клиента с `TRACKLY_*_MOCK` для SSPI/keyring.

## Alternatives Considered

| Category | Recommended | Alternative | Why Not |
|---|---|---|---|
| Сеть клиента | Всё через Rust (pinned reqwest/tungstenite) | fetch/WebSocket из webview | Самоподписанный сертификат; pinning невозможен в WKWebView; токен в JS |
| HTTP-плагин | свой reqwest | `tauri-plugin-http` | Нет хука под `ClientConfig`-pinning |
| SPNEGO | `windows` 0.62 SSPI напрямую | `sspi` 0.23.1 | MSRV 1.93 > 1.92, тяжёл API; |
| SPNEGO | `windows` | `kenobi` 0.4.2 | Малый проект, документация слабая |
| Секреты | `keyring-core` + store-крейты | `keyring` 4 default `v1`/`cli` | Тянет лишние хранилища; рекомендуемый авторами модульный путь |
| Редактор | TipTap 3 (MIT) | Lexical / Milkdown / Quill | Требуются таблицы с объединением ячеек + упоминания + JSON-схема; TipTap/ProseMirror — единственный с готовым `prosemirror-tables` |
| Редактор | `@tiptap/core` напрямую | `svelte-tiptap` 3.0.2 | Лишний слой; допустим, если нужна готовая BubbleMenu-обвязка |
| Эмодзи | Twemoji jdecked | Fluent / OpenMoji / Noto | Лицензия (SA), вес, громоздкость |
| Пикер | свой + emojibase-data (ru) | `emoji-picker-element` | Нативные глифы, IndexedDB, не тот вид |
| Валидация JSON | serde-типы в core | `jsonschema`/`schemars`, `prosemirror` crate | Дублирование схемы, незрелость |
| Апдейтер | плагин + (при проблемах TLS) собственный загрузчик | сторонний Squirrel/MSIX | Лишнее; плагин покрывает подпись/откат |

## What NOT to add

- `tauri-plugin-updater` в **Trackly** (portable) — по-прежнему нет; только Link.
- `native-tls`/OpenSSL, `reqwest` с `default-tls`, `aws-lc-rs` — только rustls+ring.
- `@tiptap-pro/*`, Tiptap Cloud, коллаборативные расширения (Hocuspocus/Y.js) — платные/не нужны (нет совместного редактирования).
- `emoji-picker-element`, `twemoji` (оригинальный twitter/twemoji, заброшен), CDN-ссылки на SVG (LAN без интернета).
- Службу Windows/`windows-service` — сессия 0 не показывает трей/окна (решение зафиксировано в PROJECT.md).
- `libgssapi`/`cross-krb5` в v1.5 (macOS/Linux Kerberos) — отложить.
- Отдельную очередь сообщений (Redis/NATS/MQTT) — достаточно SQLite seq + broadcast-«пинок».
- `ammonia` на сервере и HTML-хранение сообщений.
- `sqlx`, ORM — прежнее решение rusqlite/single-writer сохраняется.
- Хранение токена в файле/`tauri-plugin-store` — только ОС-хранилище (keyring).
- Реальные имена/организацию в фикстурах чата и THIRD-PARTY-NOTICES — только вымышленные («Иванов И.И.»).

## Installation (иллюстративно)

```bash
# Rust — клиент (workspace.dependencies добавить; features сверить на спайке)
cargo add -p trackly-client tauri-plugin-autostart@2 tauri-plugin-updater@2 tauri-plugin-opener@2 tauri-plugin-window-state@2
cargo add -p trackly-client reqwest@0.13 --no-default-features --features json,stream,multipart   # + rustls-фича по документации 0.13
cargo add -p trackly-client tokio-tungstenite@0.30 keyring-core@1 minisign-verify@0.3   # minisign-verify — только если путь (b)
# windows-native-keyring-store / apple-native-keyring-store / zbus-secret-service-keyring-store — под cfg(target_os)
# windows 0.62: добавить features Win32_Security_Authentication_Identity, Win32_Security_Credentials, Win32_Foundation

# Frontend
pnpm add @tiptap/core@^3.35 @tiptap/pm@^3.35 @tiptap/starter-kit@^3.35 @tiptap/extension-table@^3.35 \
  @tiptap/extension-subscript @tiptap/extension-superscript @tiptap/extension-text-align \
  @tiptap/extension-image @tiptap/extension-mention @tiptap/suggestion @floating-ui/dom \
  @twemoji/parser emojibase-data
pnpm --filter link-ui add @tauri-apps/plugin-autostart @tauri-apps/plugin-updater @tauri-apps/plugin-opener @tauri-apps/plugin-window-state
```

## Confidence & спайки (для фаз)

| Тема | Confidence | Что проверить до планирования |
|---|---|---|
| Плагины Tauri (autostart/updater/single-instance/window-state), версии | HIGH | — |
| Updater: формат, подпись, downgrade, per-user | HIGH (док Tauri) | Работа апдейтера с самоподписанным TLS сервера (спайк: (a) vs (b)) — LOW |
| NSIS `/S` для GPO | MEDIUM | Проверка на реальной Windows (флаг не описан в доках Tauri) |
| SPNEGO через `windows` SSPI | MEDIUM | Спайк на доменном ПК; серверный `sso.rs` должен принимать токен из не-браузерного клиента |
| keyring-core модульный путь | MEDIUM | API v4/keyring-core сменился — проверить на всех 3 ОС |
| reqwest 0.13 + rustls ring + кастомный verifier | MEDIUM | Имена фич и `use_preconfigured_tls` в 0.13 |
| TipTap 3 набор, MIT, объединение ячеек | HIGH (npm, лицензии) / MEDIUM (состав StarterKit) | |
| Twemoji jdecked, лицензии | HIGH | Решить способ поставки SVG |
| Overlay v1.6 | HIGH Windows / LOW Wayland | — |
| Пути lint-гейтов при переезде фронта в `packages/` | HIGH риск | Миграция с правкой `ui/scripts/check-*.mjs` |

## Sources

- crates.io API (версии/MSRV на 2026-11-25): tauri 2.12.7, tauri-plugin-autostart 2.7.0, -updater 2.14.1, -single-instance 2.5.5, -notification 2.8.1, -window-state 2.5.0, -opener 2.7.3, -http 2.9.0, keyring 4.2.0, keyring-core 1.1.0, windows-native-keyring-store 1.1.0, apple-native-keyring-store 1.0.3, reqwest 0.13.5, tokio-tungstenite 0.30.0, rustls 0.23.46, sspi 0.23.1 (MSRV 1.93), kenobi 0.4.2, minisign-verify 0.3.0, ammonia 4.2.5, prosemirror 0.5.2
- npm registry: @tiptap/* 3.35.0 (MIT), svelte-tiptap 3.0.2, @twemoji/api 17.0.3 (MIT AND CC-BY-4.0), @twemoji/parser 17.0.2, emojibase-data 18.0.0, emoji-picker-element 1.29.1, dompurify 3.5.0
- https://v2.tauri.app/plugin/updater/ — динамический endpoint, version_comparator/downgrade, installMode (HIGH)
- https://v2.tauri.app/distribute/windows-installer/ — NSIS `currentUser`, webviewInstallMode (HIGH)
- https://github.com/jdecked/twemoji — MIT (код) / CC-BY 4.0 (графика), Emoji 17.0, v17.0.3 (HIGH)
- https://github.com/open-source-cooperative/keyring-rs — модульная архитектура keyring-core (MEDIUM)
- https://github.com/Devolutions/sspi-rs, https://docs.rs/crate/kenobi/latest — оценка SPNEGO-клиентов (LOW-MEDIUM)
