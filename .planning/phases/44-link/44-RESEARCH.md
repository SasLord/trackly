# Phase 44: Link — транспорт, пин сертификата и вход - Research

**Researched:** 2026-11-27
**Domain:** Tauri 2 фоновый клиент (трей, автозапуск, single-instance), rustls-пин SPKI, клиентский SPNEGO (SSPI), keyring, device-токены и отзыв на сервере axum
**Confidence:** MEDIUM (стек и исходники крейтов проверены; всё Windows-специфичное — SSPI, трей, Explorer, RDS, автозапуск — по исходникам, живьём не проверено, см. UAT)

> Приватность: репозиторий публичный. Все примеры вымышлены (`trackly.example.local`, `EXAMPLE`, `PC-EXAMPLE-01`, `Иванов И.И.`). Реальные имена хостов, доменов, ФИО и отпечатки не писать ни в код, ни в тесты, ни в UAT-отчёты.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** Источник адреса сервера и отпечатка — конфиг, иначе экран первой настройки. Конфиг приоритетнее. Если конфига нет — экран первой настройки. Установщик с параметрами — Фаза 45; в 44 клиент только читает конфиг.
- **D-02:** Конфиг ищется в двух местах по порядку: (1) машинный — один на ПК, кладёт администратор или GPO, на RDS действует для всех, сотрудник его не правит; (2) пользовательский — в каталоге данных пользователя, его пишет экран первой настройки. Точное место машинного конфига (`HKLM` или `%ProgramData%`) и формат выбирает исследователь с учётом установки без прав администратора (ADR 0005).
- **D-03:** Экран первой настройки принимает одноразовый код-приглашение (не ручной ввод отпечатка, не TOFU). Код несёт адрес сервера и SPKI-отпечаток, имеет срок действия, используется один раз. Отпечаток, полученный при подключении, сверяется с отпечатком из кода; без совпадения ничего не сохраняется. Кнопки «Доверять» нет.
- **D-04:** Код даёт только настройку: сервер, отпечаток и регистрацию устройства. Не привязан к сотруднику, не входит в учётную запись. Дальше вход — SSO или пароль.
- **D-05:** Коды выпускает администратор в новом разделе «Link» в Trackly (заводится в Фазе 44; Фаза 45 добавит сборки, каналы, список клиентов). Видимость только администратору.
- **D-06:** Закрепляется SPKI-отпечаток (SHA-256 открытого ключа листового сертификата), не всего сертификата (ADR 0002). Нужно: показывать администратору SPKI-отпечаток для Link; при перевыпуске самоподписанного сертификата сохранять ключ. Смена ключа — редкое ручное событие, нужен новый код или конфиг.
- **D-07:** При несовпадении отпечатка — только ошибка («сертификат сервера изменился, обратитесь к администратору», ожидаемый и полученный отпечатки для администратора). Принять новый сотрудник не может. Запросов и данных Link не отправляет — ни вход, ни токен, ни SPNEGO.
- **D-08:** Адрес сервера — DNS-имя или IP. Доменный вход возможен только по имени (SPN). Если адрес — IP, Link не пытается SSO и сразу предлагает пароль с причиной. Имя хоста при проверке TLS не сверяется: доверие только на пине.
- **D-09:** Если автовход на доменном ПК не удался (нет SPN, адрес по IP, SSO выключен на сервере, учётка не сопоставлена, ошибка Kerberos) — форма логина/пароля с короткой причиной и кнопкой «Повторить автовход». При следующем запуске SSO пробуется снова. Пароль никогда не отправляется повторно автоматически.
- **D-10:** Вне домена вход запоминается всегда, без галочки; живёт до «Выйти» или отзыва. Пароль нигде не сохраняется; в хранилище ОС лежит только токен (ADR 0002). Если хранилища ОС нет (Linux без Secret Service) — токен только в памяти.
- **D-11:** «Выйти из учётной записи»: стирает токен локально и отзывает на сервере; можно войти другим логином. На доменном ПК после «Выйти» автовход не срабатывает сам, пока не нажато «Войти автоматически». Обычный «Выйти» устройство не запрещает.
- **D-12:** Отзыв двумя путями: раздел «Link» — список активных входов (сотрудник, компьютер, способ входа, когда) с «Отозвать»; карточка пользователя — «Отозвать входы Link» (все сразу). Блокировка пользователя тоже отзывает все его входы Link.
- **D-13:** Отзыв = запрет устройства: отозванное устройство не может войти ни паролем, ни автовходом, пока администратор не разрешит снова. Механику повторного допуска («Разрешить» и/или новый код) выбирает планировщик.
- **D-14:** Отзыв/блокировка срабатывают на лету: сервер разрывает WS и сессию этого входа, Link стирает токен и открывает окно с экраном входа и надписью «Вход отозван администратором». Автовход после отзыва не повторяется.
- **D-15:** macOS и Linux — только вход паролем (клиентский Kerberos вне Windows не в объёме). Доменный вход проверяется только живьём на Windows с AD; по моку не считается проверенным.
- **D-16:** Трей: левый клик показывает/прячет окно; правый — меню «Открыть Link», строка состояния (ФИО · в сети / нет связи), «Выйти из учётной записи», «Закрыть». «Закрыть» — единственный способ завершить Link (LNK-04). На Linux (нет события клика) — только меню. Подсказка — «Trackly Link».
- **D-17:** Автозапуск всегда включён, отключить нельзя, пункта меню нет. При старте Link проверяет запись автозапуска и восстанавливает. Windows — запись автозапуска текущего пользователя; macOS — login item; Linux — autostart `.desktop`.
- **D-18:** Свёрнутым в трей Link стартует только если всё готово: конфиг есть, и вход есть или пройдёт сам по SSO. Окно открывается само, если нужно действие сотрудника: код-приглашение, пароль, отпечаток не совпал, вход отозван.
- **D-19:** Повторный запуск показывает и фокусирует окно работающего экземпляра. Экземпляр один на пользователя сеанса: на RDS у каждого пользователя свой (ADR 0004).
- **D-20:** Linux без системного трея (GNOME без AppIndicator): обычное приложение; окно на панели, закрытие сворачивает; выход — пунктом внутри окна. Подсказку «установите расширение» не показываем. macOS — без значка в Dock, иконка в строке меню.
- **D-21:** Пока чата нет — каркас Фазы 42: заглушка «Здесь скоро будет чат» и служебная строка. В шапке: ФИО и способ входа, сервер, состояние связи. Отдельной диагностической панели нет.
- **D-22:** Потеря связи — плашка «Нет связи с сервером · переподключение…» с «Повторить сейчас» и серая иконка трея «Trackly Link — нет связи». Системных уведомлений нет.
- **D-23:** Индикация с задержкой ~5 с; переподключение с нарастающей паузой до ~30 с; после сна/смены сети пауза сбрасывается. Повторяет D-14/D-15 Фазы 43.
- **D-24:** После переподключения/сна догружает пропущенное через механизм Фазы 43: `hello`, `resync_required`, ресинк по курсору. Ресинк тихий.
- **D-25:** Если связи нет при старте, а вход уже был — тихо ждёт сеть в состоянии «нет связи», форму пароля не показывает. Пароль спрашивается, только когда сервер сам ответил, что вход недействителен/отозван.

### Claude's Discretion
- Формат кода-приглашения (строка/ссылка, длина, кодировка), срок по умолчанию, хранение на сервере (хэш).
- Схема серверных сущностей устройства и входа (`link_devices`, токены устройства, `link_resume`); ориентир — ARCHITECTURE.md §«Аутентификация не-браузера».
- Крейты SSPI, keyring-store, tray, autostart, single-instance, способ различать экземпляры по сеансу RDS — по исследованию.
- Набор иконок трея (обычная и серая) под DPI Windows; пересоздание иконки после перезапуска Explorer.
- Тексты ошибок и причин неудачного автовхода (без технических деталей Kerberos для сотрудника; детали — в лог без токенов и SPNEGO-заголовков).

### Deferred Ideas (OUT OF SCOPE)
- Запасной (следующий) SPKI-отпечаток в конфиге для плановой смены ключа — при необходимости в Фазе 45.
- Диагностическая панель Link — если понадобится после пилота.
- Системное уведомление при долгом отсутствии связи — Фаза 51.
- Клиентский Kerberos на macOS/Linux — вне вехи.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| LNK-02 | Link виден только как иконка в трее, окна нет на панели задач | tauri `tray-icon` + `skip_taskbar`, `ActivationPolicy::Accessory` на macOS; детект трея на Linux (§Pattern 2, Pitfall 5, 6) |
| LNK-03 | Автозапуск при входе, свёрнутым | `auto-launch` 0.6 напрямую с кавычками в пути + `--minimized`; `window-state` без флага VISIBLE (Pitfall 3, 4) |
| LNK-04 | Закрытие окна прячет в трей; выход только «Закрыть» | `CloseRequested → prevent_close + hide`, `app.exit(0)` из меню (Pattern 2) |
| LNK-05 | Один экземпляр на пользователя; RDS — по экземпляру на сеанс | `tauri-plugin-single-instance` 2.5.5: имя мьютекса без `Global\` → пространство имён сеанса (проверено по исходнику; RDS — UAT) |
| LNK-06 | Автовход доменного ПК (Kerberos/SPNEGO) | SSPI `Negotiate` через `cross-krb5` (NEGOTIATE_TOKEN) либо `windows` FFI; сервер `auth_ad_sso` без изменения логики (Pattern 4); live-only |
| LNK-07 | Вход паролем вне домена, запоминается, отзывается | `link_auth_login` + `device_token` + `link_resume`, keyring-core; отзыв через `link_devices.revoked_at` + middleware + WS-hub (Pattern 5, 6) |
| LNK-08 | Обмен только при совпадении SPKI-отпечатка | кастомный `rustls::ServerCertVerifier`, одинаковый для reqwest и tokio-tungstenite (Pattern 1) |
| LNK-09 | «Нет связи», самопереподключение, догрузка после сна | Rust-автомат связи, backoff 5→30 с, `trackly-resync` в тот же транспорт (Pattern 3) |
| LNK-11 | Linux без трея: окно доступно | детект `org.kde.StatusNotifierWatcher`, режим «обычное окно» (Pattern 2, Pitfall 6) |
</phase_requirements>

## Summary

Фаза состоит из трёх почти независимых слоёв. (1) Клиентская оболочка `crates/link-app`: сейчас там 81 строка (`main.rs`, окно `visible:false`, CSP без сети). Добавляются tray, single-instance, автозапуск, конфиг, keyring, сетевой слой (reqwest + tokio-tungstenite на одном `rustls::ClientConfig` с пином SPKI) и конечный автомат состояния, который эмитит события в webview. (2) Сервер: SPKI-отпечаток и сохранение ключа при перевыпуске, миграция `V060` (устройства, токены, приглашения), маршруты enrollment/login/resume/logout/revoke, middleware отзыва поверх существующих `tower-sessions` (сессии в SQLite — `RusqliteSessionStore`), разрыв WS по отзыву. (3) UI: экраны Link (UI-SPEC §6) и раздел «Link» в Trackly (UI-SPEC §8–10).

Ключевые находки исследования: **(а)** `server.key` уже сохраняется (`load_or_generate_self_signed`), но при смене набора SAN генерируется НОВЫЙ ключ — для SPKI-пина это надо изменить (переиспользовать ключ, перевыпускать только сертификат). **(б)** `tauri-plugin-autostart` 2.7 / `auto-launch` 0.6 пишут в `HKCU\...\Run` путь БЕЗ кавычек, а канонический путь Link содержит пробел (`...\Programs\Trackly Link\trackly-link.exe`) — автозапуск нужно ставить самим с кавычками. **(в)** `tauri-plugin-window-state` по умолчанию сохраняет и восстанавливает `VISIBLE` — это сломает «старт свёрнутым». **(г)** `tray-icon` 0.25 сам обрабатывает `TaskbarCreated` (пересоздание иконки после перезапуска Explorer). **(д)** `apple-native-keyring-store` для неподписанного приложения требует feature `keychain`. **(е)** Для SSPI рабочий путь — `Negotiate` через Windows API; `sspi` в дереве зависимостей (0.21.3) используется только серверной стороной, а её собственная клиентская часть для «текущих учётных данных» Windows не подходит.

**Primary recommendation:** Использовать `reqwest` 0.13 (`tls_backend_preconfigured`) + `tokio-tungstenite` 0.29 с одним общим `ClientConfig` (provider `ring`, свой `ServerCertVerifier` по SPKI SHA-256); SSPI — `cross-krb5` 0.5 с `InitiateFlags::NEGOTIATE_TOKEN` (спайк-обязательно, запасной вариант — прямой `windows` FFI); keyring — `keyring-core` 1.1 + 3 store-крейта; single-instance — `tauri-plugin-single-instance` 2.5.5; автозапуск — `auto-launch` 0.6 напрямую; новые общие чистые функции (SPKI, формат кода) — в маленький крейт `crates/link-proto` с golden-фикстурой.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| TLS-пин, HTTP, WS, cookie jar | Rust (link-app) | — | ADR 0002: webview без сети (CSP `ipc:` only) |
| Автомат связи (backoff, 5 с индикация, сон) | Rust (link-app) | webview (отображение) | Таймеры JS в скрытом окне троттлятся (PITFALLS M-7) |
| SPNEGO-токен клиента | Rust (Windows, SSPI) | — | В WebView2 Negotiate не работает для самоподписанного хоста/по IP (PITFALLS M-5) |
| Секреты (device_secret/token) | OS keyring (Rust) | память (Linux без Secret Service) | ADR 0002 |
| Конфиг сервера+пин | файл машинный / пользовательский (Rust) | экран первой настройки | D-01, D-02 |
| Выпуск/погашение кода, устройства, токены, отзыв | API / Backend (axum + SQLite) | Trackly UI (админ) | Роль/безопасность — серверные |
| Разрыв сессии/WS при отзыве | Backend (`LinkHub` + middleware) | — | D-14 «на лету» |
| SPKI-отпечаток для админа | Backend (`server/tls.rs`) | UI NetworkSettings | D-06 |
| Трей, автозапуск, single-instance | OS shell (Rust, Tauri) | — | LNK-02..05 |
| Экраны входа/настройки, шапка | Browser/webview (Svelte, `link-ui`) | — | UI-SPEC §6 |

## Standard Stack

### Core (link-app)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `tauri` | 2.12.7 (в lock) + feature `tray-icon`, `image-png` | Трей, окно, события | Уже в workspace; tray встроен [VERIFIED: tauri-2.12.7/Cargo.toml features] |
| `tauri-plugin-single-instance` | 2.5.5 (workspace `"2"`) | Один экземпляр | Уже используется в trackly-app; Windows-реализация: мьютекс `{id}-sim` без `Global\` [VERIFIED: исходник plugin 2.5.5] |
| `tauri-plugin-window-state` | 2.5.0 (уже в link-app) | Размер/позиция | Использовать БЕЗ `StateFlags::VISIBLE` [VERIFIED: исходник lib.rs стр. 69, 264, 279] |
| `auto-launch` | 0.6.0 | Автозапуск Win/macOS/Linux | Ядро `tauri-plugin-autostart`; Windows `HKCU\...\Run` + `StartupApproved\Run`, macOS `LaunchAgent`/`SMAppService`, Linux XDG `.desktop` [VERIFIED: исходник 0.6.0] |
| `reqwest` | 0.13.5 (`default-features = false`, features `rustls`, `cookies`, `json`) | HTTPS-клиент | `ClientBuilder::tls_backend_preconfigured(impl Any)` принимает `rustls::ClientConfig` [VERIFIED: исходник 0.13.5]; уже в Cargo.lock транзитивно |
| `tokio-tungstenite` | 0.29 (как в trackly-app), feature `rustls-tls-native-roots` НЕ брать — нужен только `__rustls-tls` через `Connector::Rustls` | WS-клиент | Версия уже в workspace; `Connector::Rustls(Arc<ClientConfig>)` даёт тот же верификатор |
| `rustls` | 0.23 (`default-features = false`, feature `ring`) | TLS + `ServerCertVerifier` | Совпадает с trackly-app (`ring`); reqwest 0.13 тоже на 0.23.x [VERIFIED: Cargo.toml reqwest] |
| `x509-parser` | 0.18.1 (в lock) | Достать SPKI из DER листа | `tbs_certificate.subject_pki.raw` — DER SubjectPublicKeyInfo |
| `sha2` | 0.10 (workspace) | SHA-256 | Уже в workspace |
| `keyring-core` | 1.1.0 (MSRV 1.85) | Единый API `Entry` | ADR 0002; `set_default_store`, `Entry::new`, `set_secret/get_secret/delete_credential` [VERIFIED: исходник] |
| `windows-native-keyring-store` | 1.1.0 (MSRV 1.88) | Windows Credential Manager | `Store::new()` [VERIFIED: example.rs] |
| `apple-native-keyring-store` | 1.0.3 | macOS Keychain | **Включить feature `keychain`** (неподписанное приложение; `protected` даст ошибку -34018) [VERIFIED: README] |
| `zbus-secret-service-keyring-store` | 1.0.1 (MSRV 1.88) | Linux Secret Service без libdbus | Features: `rt-tokio-crypto-rust` (нужен tokio-рантайм, крипто pure-Rust) [VERIFIED: README] |
| `cross-krb5` | 0.5.0 (обновлён 2026-05-31, ~1,4 млн загрузок) | Клиентский SSPI Negotiate на Windows | Windows-реализация вызывает `AcquireCredentialsHandleW("Negotiate")` при флаге `NEGOTIATE_TOKEN`, проверяет, что выбран Kerberos, а не NTLM [VERIFIED: исходник windows.rs]. Только `cfg(windows)` |
| `uuid` | 1 (workspace, v4) | `device_id` | Уже в workspace |
| `toml` | 0.8 (workspace) | Конфиг Link | Уже в workspace |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `zbus` (транзитивно от secret-service) | 5.x | Детект `org.kde.StatusNotifierWatcher` на Linux | `cfg(target_os="linux")`, блокирующий вызов `NameHasOwner` при старте |
| `windows` | 0.62 (в workspace; добавить features под `Win32_Security_Authentication_Identity`) | Запасной путь SSPI, если `cross-krb5` не пройдёт спайк | Только если спайк провален |
| `base64` | 0.22 (уже в trackly-app) | Код-приглашение, Negotiate-токен | Всегда |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `auto-launch` напрямую | `tauri-plugin-autostart` 2.7.0 | Плагин собирает путь из `current_exe()` БЕЗ кавычек → пробел в `Trackly Link` ломает запись. Плагин даёт готовые JS-команды, которые Link не нужны (переключателя нет, D-17) |
| `cross-krb5` | `sspi` 0.21.3 (client) | `sspi` в Rust реализует Kerberos/NTLM сам и требует явных учётных данных; «SSO с текущим логоном» через него не заявлено проверенным («Successful outbound Kerberos authentication remains unverified» в README rcg). Не использовать |
| `cross-krb5` | прямой `windows` FFI (`AcquireCredentialsHandleW`/`InitializeSecurityContextW`, пакет `Negotiate`) | ~120 строк unsafe, но полный контроль флагов; запасной вариант при провале спайка |
| `reqwest` | `hyper` + `tokio-rustls` вручную | Больше кода (cookie jar, redirect, json). reqwest уже в lock |
| машинный конфиг HKLM | `%ProgramData%` TOML | См. Open Question 1; рекомендован TOML-файл (одинаков на всех ОС, тестируем на macOS) |
| `keyring` 4.x | `keyring-core` + stores | Отвергнуто ADR 0002 |

**Installation (link-app/Cargo.toml, схематично):**
```bash
cargo add -p link-app tauri-plugin-single-instance auto-launch@0.6 reqwest@0.13 tokio-tungstenite@0.29 \
  rustls x509-parser sha2 keyring-core@1.1 uuid toml tokio serde_json base64 --offline-ok
# cfg(windows): windows-native-keyring-store@1.1, cross-krb5@0.5
# cfg(macos):   apple-native-keyring-store@1.0.3 (features=["keychain"])
# cfg(linux):   zbus-secret-service-keyring-store@1.0.1 (features=["rt-tokio-crypto-rust"])
```
В workspace закрепить версии в `[workspace.dependencies]`, `tauri` дополнить `features = ["tray-icon","image-png"]` только для link-app (через собственную запись в `link-app/Cargo.toml`, чтобы не раздувать trackly-app).

**Version verification:** `cargo search` (2026-11-27): reqwest 0.13.5, tokio-tungstenite 0.30.0 (в workspace 0.29 — оставить 0.29 для единства с сервером), x509-parser 0.18.1, keyring-core 1.1.0, cross-krb5 0.5.0, auto-launch 0.6.0, tauri-plugin-single-instance 2.5.5 (stable; на crates.io также есть `3.0.0-alpha`, брать `2`), tauri-plugin-autostart 2.7.0.

## Package Legitimacy Audit

slopcheck в окружении не запускался (не установлен/не проверялся) → все новые пакеты помечены `[ASSUMED]` по правилу provenance; реестровое существование и версии проверены `cargo search`/crates.io API. Планировщик обязан вставить `checkpoint:human-verify` перед первой установкой новых крейтов.

| Package | Registry | Age | Downloads | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-----------|-------------|-----------|-------------|
| keyring-core | crates.io | молодой (1.1.0), org open-source-cooperative | высокие (экосистема keyring) | github.com/open-source-cooperative/keyring-rs | не запускался | Approved by ADR 0002; `[ASSUMED]` до human-verify |
| windows-/apple-native-/zbus-secret-service-keyring-store | crates.io | то же | — | github.com/open-source-cooperative/* | не запускался | Approved by ADR 0002; `[ASSUMED]` |
| cross-krb5 | crates.io | >3 лет | ~1,4 млн всего | github.com/estokes/cross-krb5 | не запускался | `[ASSUMED]`, одобрить после спайка |
| auto-launch | crates.io | >4 лет | высокие (ядро tauri-plugin-autostart) | github.com/tauri-apps/auto-launch | не запускался | `[ASSUMED]` (официальный tauri-apps) |
| reqwest, tokio-tungstenite, rustls, x509-parser, sha2 | crates.io | годы | топ-крейты | известные репо | не запускался | уже в Cargo.lock → низкий риск |

**Packages removed due to slopcheck [SLOP] verdict:** none (проверка не выполнялась)
**Packages flagged as suspicious [SUS]:** none известных; npm-пакетов фаза не добавляет (UI-SPEC: «новых npm-зависимостей ноль»).

## Architecture Patterns

### System Architecture Diagram

```
                       ┌────────────────────────── Link (процесс на сеанс пользователя) ──────────────────────────┐
 запуск (автозапуск     │  single-instance ──► (второй запуск: show+focus первого)                                │
 --minimized / ярлык) ─►│                                                                                          │
                       │  config: машинный link.toml ─► пользовательский link.toml ─► нет ─► экран настройки     │
                       │           │                                                                              │
                       │           ▼                                                                              │
                       │  LinkCore (tokio, автомат: Setup│Connecting│NeedLogin│Online│Offline│Revoked│PinMismatch)│
                       │    ├─ PinnedTls: rustls ClientConfig{ ServerCertVerifier = SPKI sha256 == pin }          │
                       │    │     ├─ reqwest Client (http1, cookie Jar)  ──► POST/GET /api/v1/*                   │
                       │    │     └─ tokio-tungstenite Connector::Rustls ──► wss /api/v1/ws (Cookie из Jar)       │
                       │    ├─ Auth: SSO(SSPI Negotiate) │ password → device_token │ resume(device_token)         │
                       │    ├─ Keyring: device_id, device_secret, device_token (или RAM если нет хранилища)       │
                       │    └─ emit: `trackly-event`, `trackly-resync` (реюз транспорта Фазы 43), `link-state`    │
                       │  Tray (Rust): иконка norm/gray, меню; Window: CloseRequested→hide (Linux без трея→min)   │
                       │  webview (Svelte, CSP ipc-only) ◄── invoke(link_*) / listen(link-state)                  │
                       └──────────────────────────────────────────┬───────────────────────────────────────────────┘
                                                                  │ TLS, пин SPKI (имя хоста не сверяется)
                       ┌──────────────────────────────────────────▼───────────────────────────────────────────────┐
                       │ Trackly server (axum)                                                                    │
                       │  public: link_enroll(code) · link_auth_login · link_auth_sso · link_resume · ws          │
                       │  middleware link_guard: session.link_device_id → link_devices.revoked_at / user.is_active│
                       │        └─ отозван ─► session.flush + 401 LINK_REVOKED                                    │
                       │  LinkHub(device_id → CancellationToken) ─► закрыть WS кодом «revoked»                    │
                       │  admin: link_invites_create/list · link_devices_list/revoke/allow · users_revoke_link    │
                       │  SQLite V060: link_devices, link_logins, link_invites   · sessions (tower-sessions)      │
                       └──────────────────────────────────────────────────────────────────────────────────────────┘
```

### Recommended Project Structure
```
crates/link-proto/          # НОВЫЙ маленький крейт: spki_sha256(cert_der), формат/кодек кода-приглашения, DTO enroll/login (serde)
crates/link-app/src/
├── main.rs                 # Builder, плагины, setup, RunEvent
├── config.rs               # поиск машинного/пользовательского link.toml, валидация (чистые функции)
├── core/                   # state machine (Setup/Connecting/NeedLogin/Online/Offline/Revoked/PinMismatch), backoff
├── net/pin.rs              # ServerCertVerifier + сборка ClientConfig
├── net/http.rs, net/ws.rs  # reqwest-клиент, WS-цикл (hello/ping/resync)
├── auth/{sso.rs,password.rs,store.rs}  # SsoProvider trait (+ mock), keyring Store trait (+ in-memory)
├── shell/{tray.rs,autostart.rs,window.rs}
└── commands.rs             # #[tauri::command] link_*
crates/trackly-app/src/http/link.rs, services/link.rs, server/link_hub.rs
crates/trackly-infra/ (repo) + migrations/V060__link_devices.sql
```

### Pattern 1: Пин SPKI через кастомный `ServerCertVerifier`
**What:** Единый `rustls::ClientConfig`: `dangerous().with_custom_certificate_verifier(Arc<PinVerifier>)`; `verify_server_cert` считает SHA-256 от `x509-parser … subject_pki.raw` листа и сравнивает с пином (constant-time); имя сервера игнорируется (D-08); подписи handshake проверяются делегированием `rustls::crypto::verify_tls12/13_signature` с `provider.signature_verification_algorithms`. Пин-несовпадение возвращает `rustls::Error::InvalidCertificate(CertificateError::ApplicationVerificationFailure)`, а полученный отпечаток запоминается в `Arc<Mutex<Option<String>>>` внутри верификатора (для экрана D-07 «ожидаемый/полученный»). Провайдер задавать явно: `ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))` (в проекте уже есть `ensure_crypto_provider` для сервера — не полагаться на process-default).
**Когда:** любое сетевое обращение Link (и enrollment — с пином из кода).
```rust
// Source: rustls 0.23 docs (dangerous::ServerCertVerifier) + reqwest 0.13 ClientBuilder::tls_backend_preconfigured
let client = reqwest::Client::builder()
    .tls_backend_preconfigured(tls_config.clone())   // Arc не нужен: impl Any = ClientConfig
    .http1_only()                                    // SPNEGO: одно соединение между раундами (PITFALLS M-5)
    .cookie_provider(jar.clone())
    .redirect(reqwest::redirect::Policy::none())
    .build()?;
// WS: tokio_tungstenite::connect_async_tls_with_config(req, None, false, Some(Connector::Rustls(Arc::new(tls_config))))
```
Для WS заголовок `Cookie` собирается вручную из `jar.cookies(&url)`. Важно: сессионная cookie сервера `Secure; HttpOnly; SameSite=Strict` — для не-браузера ограничения не мешают, `cookie_store` в reqwest учитывает `Secure` по схеме https (корректно).

### Pattern 2: Оболочка (трей, окно, автозапуск)
- Трей: `TrayIconBuilder::with_id("main").icon(..).tooltip("Trackly Link").menu(&menu).show_menu_on_left_click(false).on_tray_icon_event(..)`; левый клик — `TrayIconEvent::Click { button: Left, button_state: Up }` → toggle окна. Серая иконка — `tray.set_icon(...)` и `set_tooltip`. На macOS иконка template-image (монохром) и `app.set_activation_policy(Accessory)`.
- Окно: `on_window_event(CloseRequested)` → `api.prevent_close()`; с треем — `hide()`, без трея (Linux, D-20) — `minimize()`; `skip_taskbar(true)` только при наличии трея (иначе окно потеряется).
- «Закрыть» → `app.exit(0)`; при завершении сеанса ОС приложение обязано завершаться (`RunEvent::ExitRequested` не блокировать, кроме случая «последнее окно закрыто» — `code == None`).
- Старт: `--minimized` от автозапуска не нужен как единственный источник решения — D-18: окно показывается, если нужно действие; иначе скрыто. Setup-код 42 («страховка: `w.show()`») заменить на решение автомата.
- Tray на Windows: `tray-icon` 0.25 регистрирует `TaskbarCreated` и пересоздаёт иконку после перезапуска Explorer [VERIFIED: исходник tray-icon 0.25.1 windows/mod.rs стр. 51–54, 154–161]; иконки готовить набор 16/20/24/32 px (.ico с несколькими размерами) под DPI.
- Автозапуск: `auto_launch::AutoLaunchBuilder` с `set_app_path("\"C:\\...\\trackly-link.exe\"")` (кавычки!), `set_app_name("Trackly Link")`, `set_windows_enable_mode(WindowsEnableMode::CurrentUser)` (не `Dynamic`: он сначала пробует HKLM), `set_args(&["--minimized"])`. На старте: `if !al.is_enabled()? { al.enable()? }` (D-17). `enable()` на Windows также пишет `StartupApproved\Run` = enabled, т.е. перекрывает отключение через Диспетчер задач (это следствие D-17; отметить пользователю). macOS: `MacOSLaunchMode::LaunchAgent` (работает без подписи; `SMAppService` требует macOS 13+ и бандл). Linux: XDG autostart (`Exec=` без кавычек — путь AppImage с пробелами проверить).

### Pattern 3: Автомат связи в Rust
Состояния: `Setup`, `Connecting`, `NeedLogin{reason}`, `Online{user,method}`, `Offline{since}`, `Revoked`, `PinMismatch{expected,got}`. Таймеры только в tokio (не JS). Индикация «нет связи» через 5 с (D-23), backoff 1→2→4…→30 с с джиттером; сброс на «пробуждение/смену сети». Детект сна: период опроса tokio-таймером + скачок `Instant` vs `SystemTime` (> порога) → считать пробуждением; плюс WS-сторожок: нет кадра за `idleTimeoutSecs` из `hello` → рвать и переподключаться. На успешном (пере)подключении после обрыва эмитить `trackly-resync` — существующий транспорт Фазы 43 (`onWsResync`) уже умеет тихий ресинк; `hello`/`ping {"type":"ping"}`/`resync_required` обрабатываются в Rust-цикле и пробрасываются как `trackly-resync`. Webview получает `link-state` (JSON) для шапки/плашки; меню трея и иконка обновляются из того же автомата. Различать «нет связи» и «вход недействителен»: только `401`/`403` от сервера с кодом отзыва/невалидности ведут в `NeedLogin`/`Revoked`; сетевые ошибки и 5xx — `Offline` (D-25).

### Pattern 4: Клиентский SPNEGO (Windows)
- Цель SPN: `HTTP/<host>` по DNS-имени из конфига (серверный `ad.spn` вида `HTTP/trackly.example.local` — см. `crates/trackly-infra/src/config.rs`; клиент строит `HTTP/{host}` из адреса, отдельный параметр не нужен; опциональное поле `spn` в конфиге Link — на случай CNAME/нестандартного SPN).
- Если адрес — IP (`host.parse::<IpAddr>().is_ok()`), SSO не пытаться → `NeedLogin{reason: IpAddress}` (D-08/D-09).
- `ClientCtx::new(InitiateFlags::NEGOTIATE_TOKEN, None, spn, None)` → токен (SPNEGO NegTokenInit) → `GET /api/v1/auth_ad_sso` с `Authorization: Negotiate <b64>` **сразу, превентивно** (сервер при отсутствии заголовка отвечает 401 Negotiate; лишний раунд не нужен). Не запрашивать `MUTUAL_AUTH` — Kerberos завершается за один шаг (сервер `accept_spnego` отвечает `Authenticated`/`Continue`/`Denied`; на `Continue` — повтор по тому же TCP-соединению, `http1_only` + один `reqwest::Client`).
- Сервер уже верифицирован как кодом (accept через `sspi` по keytab, без KDC), но **вживую не проверялся с нативным клиентом**; браузер (Chrome/Edge) проверен в фазах 31–32. Различие клиента: браузер шлёт SPNEGO-обёртку; убедиться, что `cross-krb5` с `NEGOTIATE_TOKEN` даёт тот же формат. Если SSPI выбрал NTLM (нет SPN в AD) — `cross-krb5` отказывает (проверка «negotiate selected NTLM instead of Kerberos»), что нам и нужно: причина «SPN не найден».
- Mock: `trait SsoProvider { fn token(&self, spn:&str)->Result<Vec<u8>,SsoFail> }`; `MockSso` для тестов на macOS (по аналогии с `TRACKLY_AD_MOCK`); любое «проверено» для SSO только после живого прогона (D-15, PITFALLS стр. 325).
- Классификация отказов для D-09: нет билета/SPN → «Компьютер не смог получить билет Kerberos»; 503 → «Автовход выключен на сервере»; 401 `Denied` → «Учётная запись не сопоставлена/не подтверждена»; adresс IP → «Сервер указан по IP-адресу». Детали Kerberos — только в лог.
- Пароль не повторять автоматически; `429` от rate-limit учитывать (UI: «Слишком много попыток, подождите»).

### Pattern 5: Сервер — устройства, токены, коды (V060)
Рекомендуемая схема (имена на усмотрение планировщика; только CREATE, без FK на `users`, последняя строка `PRAGMA user_version = 60;` — иначе красный `ci-full`, см. память migration_user_version_pragma):
```sql
CREATE TABLE link_devices(device_id TEXT PRIMARY KEY, secret_hash BLOB NOT NULL, hostname TEXT, os TEXT, arch TEXT,
  client_version TEXT, first_seen_utc INTEGER NOT NULL, last_seen_utc INTEGER, last_ip TEXT, last_user_id INTEGER,
  revoked_at_utc INTEGER NULL);                      -- D-13: revoked_at != NULL => запрет устройства
CREATE TABLE link_logins(id INTEGER PRIMARY KEY AUTOINCREMENT, device_id TEXT NOT NULL, user_id INTEGER NOT NULL,
  method TEXT NOT NULL,                              -- 'sso' | 'password'
  token_hash BLOB NULL UNIQUE,                       -- sha256(device_token); NULL у sso
  created_at_utc INTEGER NOT NULL, last_used_utc INTEGER, revoked_at_utc INTEGER NULL);
CREATE TABLE link_invites(id INTEGER PRIMARY KEY AUTOINCREMENT, code_hash BLOB NOT NULL UNIQUE, created_by_user_id INTEGER NOT NULL,
  created_at_utc INTEGER NOT NULL, expires_at_utc INTEGER NOT NULL, used_at_utc INTEGER NULL, used_by_device_id TEXT NULL);
PRAGMA user_version = 60;
```
- `device_token`: 256 бит `OsRng`, в БД SHA-256 (Argon2 избыточен — высокая энтропия, ARCHITECTURE.md §Аутентификация); `device_secret` аналогично.
- Погашение кода (`POST /api/v1/link_enroll`, публичный, rate-limit через `tower_governor` как `auth_login`): поиск по `code_hash`, проверка срока/«не использован», **атомарная** пометка `used_at` в одной write-транзакции единственного писателя (гонка двойного погашения → второй получает «код уже использован»). Ответ: `device_secret`. Одноразовость — на сервере; клиент присылает `device_id` (UUIDv4), `hostname`, `os`, `arch`, `client_version`.
- Формат кода (рекомендация): строка `TL1.` + base64url(JSON/компактный бинарный `{v:1, host, port, spki:32B, secret:16B, exp}`) + CRC; копируется «Скопировать» в попапе и вставляется в поле (UI-SPEC §6.3). 128-битная случайная часть, на сервере только SHA-256. Срок по умолчанию — 24 ч (выбор в попапе, UI-SPEC §8.4). Код не содержит секретов сотрудника (D-04).
- Привязка сессии к устройству: каждый запрос Link несёт `X-Link-Device: <device_id>` (+ `X-Link-Secret`); в `link_auth_login`/`link_auth_sso`/`link_resume` сервер проверяет устройство (существует, не отозвано, секрет) и кладёт `link_device_id` (и `link_login_id`) в данные сессии. `link_auth_*` — тонкие обёртки над общими функциями (`build_auth_login`, вынесенное из `handler_ad_sso` ядро SSO), чтобы поведение браузерного входа не менялось.
- Проверка на лету: middleware по образцу `refresh_session_role` (`http/mod.rs:63`): если в сессии есть `link_device_id`, проверить `link_devices.revoked_at`, `link_logins.revoked_at`, `users.is_active`; при провале — `session.flush()` + `401` с кодом `LINK_REVOKED` (клиент по коду отличает отзыв от обычного истечения). Это ловит и случаи, когда хук блокировки пользователя не вызван.
- Разрыв WS: `LinkHub` (`DashMap`-подобный `Mutex<HashMap<device_id, CancellationToken>>`) регистрируется в `ws_handler` (после `ws_auth_middleware`); `revoke(device_id | user_id)` отменяет токен → `ws_loop` отправляет Close-кадр (как для `ServerShutdown`, см. docblock `ws.rs`) и завершается. Только сервисы revoke/block зовут хаб.
- Блокировка пользователя: найти точку, где `is_active` переключается (users-сервис в `trackly-app/src/services/`), добавить вызов `link.revoke_user(user_id)`. Покрыть тестом «заблокировать → Link-сессия 401».
- Повторный допуск (D-13): рекомендуется кнопка «Разрешить» (`link_devices_allow` обнуляет `revoked_at`) — проще и без нового кода; новый код-приглашение не нужен, т.к. `device_id` тот же.
- Все новые мутирующие маршруты — в реестровые гейты (`entities_changed_gate`, инвентарь write-site, `http_route_parity` — читает `collect_commands!` и `.route("/api/v1/…")`; HTTP-only маршруты (`link_enroll`, `link_auth_*`, `link_resume`, `link_logout`) вносить как вердикты «только HTTP», как уже сделано для `auth_ad_sso`); админ-маршруты — парой «Tauri-команда + HTTP» (десктоп админа). Покрывать каждый новый маршрут тестом (`spa_fallback` отвечает 404 JSON, но по памяти — тест, а не браузер).

### Pattern 6: SPKI на сервере (D-06)
- `link-proto::spki_sha256(cert_der) -> [u8;32]` (x509-parser `subject_pki.raw`), форматирование `AB:CD:…` как у `compute_fingerprint`. Тот же код используется и клиентом, и сервером → golden-фикстура (одна PEM-пара в `crates/link-proto/tests/fixtures`, ожидаемый хэш в тесте; НЕ боевой сертификат) защищает от расхождения (память: js_rust_mirror_needs_fixture_gate).
- `TlsBundle` получает `spki_fingerprint_hex`; `http/settings.rs`/`NetworkSettings.svelte` показывают его для Link (UI-SPEC §9), старый отпечаток DER остаётся для браузера.
- Сохранение ключа: в `load_or_generate_self_signed` при несовпадении SAN сегодня генерируется новая пара (`generate_self_signed`). Изменить: если `server.key` читается и разбирается (`KeyPair::from_pem`), выпускать новый сертификат **на старом ключе** (`CertificateParams::new(sans).self_signed(&key_pair)`), перезаписав `server.crt` и `server.sans`; новый ключ — только если ключа нет/он повреждён. Тест: смена SAN → тот же SPKI, другой DER-отпечаток. Для пользовательского `cert_path` ничего не меняется (ключ админа).
- Пин и ключ rcgen: по умолчанию ECDSA P-256; `PinVerifier` обязан поддерживать и `ECDSA_NISTP256_SHA256` и RSA-схемы (админ может подложить свой сертификат) — делегированием в provider это покрыто.

### Anti-Patterns to Avoid
- `danger_accept_invalid_certs(true)` или «доверять при первом подключении»: отвергнуто D-03/D-07 и PITFALLS M-4.
- Сетевые вызовы из webview (`fetch`, `EventSource`): CSP их запрещает; всё через `invoke`.
- JS-таймеры для индикации/backoff — в скрытом окне WebView2 их приостанавливает.
- Хранение пароля/токена в файле конфига, localStorage, логах; `Authorization: Negotiate` в логах.
- Читать `AppData\Roaming` для `device_id`: см. Pitfall 7.
- Один общий `keyring::Entry` из нескольких потоков одновременно (предупреждение README windows-store).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| SPNEGO/Kerberos клиента | свой разбор ASN.1/Kerberos | Windows SSPI через `cross-krb5` (или `windows` FFI) | билеты, часы, SPN-канонизация — на стороне ОС |
| Хранилище секретов | шифрованный файл с самодельным ключом | `keyring-core` + store-крейты | DPAPI/Keychain/Secret Service (ADR 0002) |
| Single-instance | свой мьютекс/сокет | `tauri-plugin-single-instance` | уже в проекте, per-session на Windows |
| Автозапуск | прямая запись в реестр/plist/desktop | `auto-launch` | три ОС, `StartupApproved` |
| Пересоздание трея после Explorer | собственный `TaskbarCreated` | `tray-icon` 0.25 | уже обрабатывает |
| Проверка подписи TLS-рукопожатия | свои ECDSA/RSA проверки | `rustls::crypto::verify_tls1{2,3}_signature` | только отпечаток сравниваем сами |
| Rate-limit входа | счётчики вручную | `tower_governor` (как `auth_login`) | уже в проекте |
| Хэширование токена | Argon2 | SHA-256 | токен 256 бит, Argon2 избыточен |
| Backoff/джиттер | — | маленькая чистая функция в `core/` + тест | тривиально, но обязана быть тестируемой без сети |

**Key insight:** единственные самописные части — верификатор пина (≈60 строк, чистая функция сравнения тестируется без сети) и автомат состояний; всё остальное — готовые компоненты.

## Runtime State Inventory

Не rename/refactor-фаза — раздел опущен. (Единственная миграция состояния: `server.key` остаётся прежним — подлежит тесту «ключ сохраняется».)

## Common Pitfalls

### Pitfall 1: Пин по сертификату вместо SPKI ломается при продлении
**What goes wrong:** при смене SAN сервер выпускает новый сертификат И новый ключ — пин всех клиентов устарел (D-06).
**Why:** `load_or_generate_self_signed` вызывает `generate_self_signed` целиком.
**How to avoid:** переиспользовать ключ (Pattern 6); тест «смена SAN → тот же SPKI».
**Warning signs:** «все клиенты отвалились после смены имени/IP сервера».

### Pitfall 2: `reqwest`/`rustls` без явного crypto provider
**What goes wrong:** `ClientConfig::builder()` паникует, если в процессе не установлен default provider; reqwest 0.13 по умолчанию тянет aws-lc-rs (тяжёлая C-сборка на Windows в CI).
**How to avoid:** `default-features = false` + `builder_with_provider(ring)`; проверить `cargo tree -p link-app -i aws-lc-rs` — не должен появляться (в trackly-app aws-lc уже есть через ldap3? — там ring; сверить по `Cargo.lock`/`cargo deny`). Если reqwest всё же требует feature `__rustls-aws-lc-rs` — выбрать feature `rustls-no-provider` (проверить имя в 0.13.5 при реализации).
**Warning signs:** панику «no process-level CryptoProvider» на первом запросе.

### Pitfall 3: `window-state` восстанавливает `visible`
**What goes wrong:** Link «стартует свёрнутым», но плагин (флаги по умолчанию включают `VISIBLE`) показывает окно, которое было видно при закрытии [VERIFIED: lib.rs 264/279].
**How to avoid:** `Builder::default().with_state_flags(StateFlags::all() - StateFlags::VISIBLE)`; видимостью управляет только автомат (D-18).
**Warning signs:** окно мигает при автозапуске.

### Pitfall 4: Автозапуск без кавычек
**What goes wrong:** `auto-launch` 0.6.0 пишет `format!("{} {}", app_path, args)` без кавычек [VERIFIED: windows.rs]; путь `…\Trackly Link\trackly-link.exe` содержит пробел → запись неверна/неоднозначна (`[ASSUMED]` как именно разберёт Windows; кавычки безопасны в любом случае).
**How to avoid:** передавать `app_path` уже в кавычках; `WindowsEnableMode::CurrentUser`; тест на чистой функции «сборка значения Run»; живая проверка — перезагрузка Windows, иконка в трее.
**Дополнительно:** `enable()` восстановит и `StartupApproved`, перекрывая отключение в Диспетчере задач; GPO/Defender ASR могут удалять `Run` — тогда Link при каждом старте будет пытаться восстановить, ошибку логировать, не падать.

### Pitfall 5: GNOME без AppIndicator
**What goes wrong:** `TrayIconBuilder::build()` успешен (или падает при отсутствии `libayatana-appindicator`), но иконки не видно — окно «исчезло» (LNK-11).
**How to avoid:** при старте на Linux проверить владельца `org.kde.StatusNotifierWatcher` (D-Bus `NameHasOwner`); нет → режим «обычное окно»: `skip_taskbar(false)`, `CloseRequested` → minimize, пункт «Закрыть Link» в окне (UI-SPEC §6.9); ошибку создания трея трактовать как «трея нет», не паниковать. Перепроверять наличие watcher периодически не нужно (достаточно на старте). Нет события клика на Linux (D-16) — только меню.
**Warning signs:** Link запущен, но ни окна, ни иконки.

### Pitfall 6: macOS Keychain и подпись
**What goes wrong:** без feature `keychain` store `apple-native` использует protected store и отвечает `-34018`; в неподписанной сборке при первом доступе Keychain спрашивает пароль/доступ.
**How to avoid:** feature `keychain`; все ошибки хранилища → деградация в память (D-10), не падение; в dev на macOS допустим `InMemoryStore` по env `TRACKLY_LINK_NO_KEYRING=1` (для тестов и UAT без запросов).

### Pitfall 7: `device_id` в Roaming-профиле
**What goes wrong:** `app_data_dir()` на Windows = `%APPDATA%` (Roaming): при перемещаемых профилях/RDS один `device_id` окажется на нескольких ПК — «устройство» перестаёт быть компьютером, отзыв бьёт по всем.
**How to avoid:** пользовательский конфиг и `device_id` — в `app_local_data_dir()` (`%LOCALAPPDATA%\org.trackly.link`); `device_secret` в keyring (DPAPI привязан к профилю пользователя) — ок. Согласовать с ADR 0005 (там говорится «каталог данных ОС текущего пользователя» — это не противоречие).

### Pitfall 8: Сессионная cookie и WS
**What goes wrong:** WS-handshake не видит cookie reqwest-jar, если не передать заголовок вручную → 401 до апгрейда (middleware `ws_auth_middleware`).
**How to avoid:** собирать `Cookie` из `Jar::cookies(&url)`; при 401 на WS — не считать «нет связи», а запускать ре-аутентификацию (resume/SSO) и один повтор.

### Pitfall 9: Сон/пробуждение и мёртвый TCP
**What goes wrong:** после сна сокет «жив» локально, но мёртв; ждать `idleTimeout` 60 с — долго.
**How to avoid:** детект скачка монотонных часов → немедленный сброс backoff и переподключение (D-23); сторожок по `hello.idleTimeoutSecs`; сразу `ping {"type":"ping"}` и таймаут 5 с (как `PROBE_TIMEOUT_MS` в `ws.ts`).

### Pitfall 10: SPN и CNAME / время
**What goes wrong:** Kerberos-клиент Windows может канонизировать имя (CNAME → A-запись) и запросить билет на другой SPN; расхождение часов > 5 мин — «не удалось войти».
**How to avoid:** лог причины (без токенов); в конфиге Link опциональный явный `spn`; документировать в `docs/AD-SETUP.md` (дополнить), в UAT — проверка с именем из конфига и с IP (должна дать причину D-08).

### Pitfall 11: Отозванное устройство + автовход
**What goes wrong:** на доменном ПК после отзыва автовход мгновенно вернул бы пользователя.
**How to avoid:** проверка `link_devices.revoked_at` ВО ВСЕХ входах (password, SSO, resume) на сервере (D-13), а на клиенте — флаг `revoked`, блокирующий автоповтор до явного действия.

### Pitfall 12: Дрейф дерева — новые маршруты невидимы для аудита
Новые маршруты без теста и без записи в реестры гейтов (`http_route_parity`, write-site, `entities_changed_gate`) прожили бы незамеченными (память: path_without_test_invisible_to_audit, spa_fallback). Каждый маршрут — тест на статус+тело; инвентарь от репозитория.

## Code Examples

### SPKI-отпечаток листа (общий для клиента и сервера)
```rust
// Source: x509-parser 0.18 docs (X509Certificate::subject_pki), sha2 0.10
pub fn spki_sha256(cert_der: &[u8]) -> Result<[u8; 32], SpkiError> {
    let (_, cert) = x509_parser::parse_x509_certificate(cert_der).map_err(|_| SpkiError::Parse)?;
    Ok(sha2::Sha256::digest(cert.tbs_certificate.subject_pki.raw).into())
}
pub fn fmt_colon_upper(h: &[u8; 32]) -> String { /* "AB:CD:…" как compute_fingerprint */ }
```

### Верификатор пина (каркас)
```rust
// Source: rustls 0.23 dangerous::ServerCertVerifier
impl ServerCertVerifier for PinVerifier {
    fn verify_server_cert(&self, end_entity: &CertificateDer<'_>, _ints: &[CertificateDer<'_>],
        _name: &ServerName<'_>, _ocsp: &[u8], _now: UnixTime) -> Result<ServerCertVerified, rustls::Error> {
        let got = spki_sha256(end_entity).map_err(|_| rustls::Error::InvalidCertificate(CertificateError::BadEncoding))?;
        *self.seen.lock().unwrap() = Some(got);
        if constant_time_eq(&got, &self.pin) { Ok(ServerCertVerified::assertion()) }
        else { Err(rustls::Error::InvalidCertificate(CertificateError::ApplicationVerificationFailure)) }
    }
    fn verify_tls12_signature(&self, m: &[u8], c: &CertificateDer<'_>, d: &DigitallySignedStruct)
        -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(m, c, d, &self.algs)
    }
    // verify_tls13_signature аналогично; supported_verify_schemes() => self.algs.supported_schemes()
}
```

### Keyring (с деградацией)
```rust
// Source: keyring-core 1.1.0 lib.rs / windows-native-keyring-store example
fn init_store() -> bool {            // false => секреты только в памяти (D-10)
    #[cfg(windows)] { keyring_core::set_default_store(windows_native_keyring_store::Store::new().ok()?); }
    // macOS: apple_native_keyring_store::keychain::Store::new(); Linux: zbus_secret_service_keyring_store::Store::new()
}
let e = keyring_core::Entry::new("org.trackly.link", &format!("device:{server_id}"))?;
e.set_secret(blob)?; /* get_secret() -> Err(NoEntry) = нет данных */
```

### Параметры single-instance (повторный запуск показывает окно)
```rust
// Source: tauri-plugin-single-instance 2.x (используется в trackly-app)
.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| { show_and_focus_main(app); }))
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `keyring` 3.x монолит | `keyring-core` + отдельные store-крейты | keyring-core 1.0 | ADR 0002 уже выбрал |
| `reqwest::ClientBuilder::use_preconfigured_tls` | `tls_backend_preconfigured` (старое помечено deprecated) | reqwest 0.13 | использовать новое имя [VERIFIED: исходник] |
| Пин всего DER-сертификата | Пин SPKI | ADR 0002 | переживает перевыпуск на том же ключе |
| Негласный TOFU | Код-приглашение | D-03 | пин доставляется администратором |

**Deprecated/outdated:** `tauri-plugin-single-instance`/`autostart` `3.0.0-alpha` на crates.io — не брать (проект на Tauri 2.x, стабильные 2.x).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Новые пакеты (keyring-core + 3 store, cross-krb5, auto-launch) не проверены slopcheck | Package Legitimacy | supply-chain; mitigated: ADR 0002/официальный tauri-apps, human-verify |
| A2 | `cross-krb5` с `NEGOTIATE_TOKEN` даёт SPNEGO-токен, который принимает серверный `accept_spnego` (sspi Negotiate acceptor) | Pattern 4 | автовход не работает; запасной путь — `windows` FFI. Проверяется только живьём на Windows+AD |
| A3 | Windows при Run-значении без кавычек и пути с пробелом запускает неверно/нестабильно | Pitfall 4 | лишь вопрос необходимости кавычек; с кавычками безопасно в любом случае |
| A4 | `CreateMutexW` без `Global\` — пространство имён сеанса, значит RDS-пользователи не конфликтуют | LNK-05 | два пользователя RDS блокируют друг друга; проверяется UAT RDS из пилота |
| A5 | Именование feature `reqwest` для rustls без aws-lc (`rustls-no-provider`) | Pitfall 2 | сборка тянет aws-lc; сверить при реализации |
| A6 | `Policies`/ProgramData-ACL: файл в `%ProgramData%`, созданный администратором/GPO, пользователь не может изменить (стандартный ACL) | Open Q 1 | сотрудник правит машинный конфиг; проверить на целевом ПК |
| A7 | Linux-детект трея через `NameHasOwner("org.kde.StatusNotifierWatcher")` достаточен для GNOME с/без расширения | Pitfall 5 | ложные срабатывания; UAT на Ubuntu GNOME + чистом GNOME |
| A8 | `Session::id()` доступен после явного `session.save()` для привязки сессии к устройству (если выберут таблицу сессий вместо проверки в middleware) | Pattern 5 | при выборе middleware-варианта не нужен |

## Open Questions

1. **Машинный конфиг: HKLM или файл?**
   - Known: ADR 0005 — установка без прав администратора; Фаза 45 добавит установщик с параметрами из скрипта входа GPO (пользовательский контекст) — тот не может записать HKLM, а файл в `%ProgramData%`, созданный пользователем, пользователь же и правит.
   - Unclear: кто писатель машинного конфига на практике (GPO Computer Config / администратор вручную).
   - Recommendation: машинный конфиг — **TOML-файл** `link.toml` (Windows `%ProgramData%\Trackly Link\`, macOS `/Library/Application Support/Trackly Link/`, Linux `/etc/trackly-link/`); ключи `server`, `pin_sha256`, опционально `spn`. Одинаковый формат на всех ОС, тестируется на macOS. HKLM-вариант (ключ `HKLM\SOFTWARE\Policies\TracklyLink`) оставить на Фазу 45 как второй источник за тем же трейтом `ConfigSource`. Подтвердить у пользователя.
2. **Идентичность устройства при отсутствии хранилища ОС (Linux без Secret Service).**
   - D-10 держит в памяти «токен», но `device_id`/`device_secret` (выданные одноразовым кодом) в памяти терять нельзя — иначе код нужен при каждом запуске.
   - Recommendation: `device_id` — в пользовательском конфиге; `device_secret` — в keyring, при его отсутствии — файл `0600` в `app_local_data_dir()` (идентичность устройства, отзываема, не учётная запись); токен входа — только RAM (D-10). Подтвердить у пользователя (отступление от буквального чтения ADR 0002).
3. **Как поступать, если пин в машинном и пользовательском конфиге различаются?** Рекомендация: машинный всегда побеждает целиком (сервер+пин), пользовательский игнорируется (D-01).
4. **Сохранение сессионной cookie между запусками.** Рекомендация: не сохранять; после запуска — `link_resume` (пароль) или SSO. Тогда срок «30 дней inactivity» сессии неактуален.
5. **Нужна ли подпись запросов `device_secret`?** Рекомендация: достаточно заголовка секрета по пин-каналу; HMAC-подпись — избыточна.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| cargo/rust (macOS dev) | сборка, тесты | ✓ | rust-version 1.92 (workspace) | — |
| Windows + AD + SPN/keytab | LNK-06 live | ✗ (dev macOS) | — | мок `SsoProvider`; живой прогон — human-UAT на Windows |
| Windows RDS (2 пользователя) | LNK-05 | ✗ | — | UAT из `docs/pilot/README.md` |
| Linux GNOME с/без AppIndicator | LNK-11, трей | ✗ | — | UAT на Linux-ВМ; логика детекта — юнит-тест с подменой «watcher есть/нет» |
| Windows Explorer restart | трей | ✗ | — | UAT |
| `pnpm`, vite | UI Link/Trackly | ✓ (в проекте) | — | — |
| slopcheck | аудит пакетов | не запускался | — | `checkpoint:human-verify` |

**Missing dependencies with no fallback:** Windows+AD (для подтверждения «проверено» SSO), Windows RDS — только живая проверка.
**Missing dependencies with fallback:** всё остальное имитируется трейтами и моками.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | `cargo test` (Rust: `link-app` unit/integration, `trackly-app` suite-бинарники `tests/suites/*.rs`, `trackly-infra` `--test infra_tests`), Node-гейты `ui/scripts/check-*.mjs`, `link-ui` `pnpm lint` + `svelte-check` |
| Config file | `crates/trackly-app/Cargo.toml` (`[[test]]`, `autotests=false`), `link-ui/package.json` |
| Quick run command | `cargo test -p link-app` / `cargo test -p link-proto` / `cargo test -p trackly-app --test <suite> <filter>` |
| Full suite command | `ci-full` (один раз на границе последней волны, в фоне; `timeout` на машине нет; один `cargo test` одновременно) |

Отдельные гейты (не входят друг в друга): `pnpm --dir link-ui lint`, `pnpm --dir link-ui svelte-check`, `pnpm --dir ui lint`, `pnpm --dir ui svelte-check`, `cargo clippy -- -D warnings`, `cargo fmt --check`.

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| LNK-08 | Верификатор: совпал SPKI → ok; не совпал → отказ, и ни один HTTP-запрос не ушёл; имя хоста игнорируется; IP-адрес | unit + интеграция (локальный TLS-сервер на rcgen-сертификате) | `cargo test -p link-app net::pin` | ❌ Wave 0 |
| LNK-08/D-06 | `spki_sha256` golden-фикстура; смена SAN → тот же SPKI | unit | `cargo test -p link-proto` ; `cargo test -p trackly-app --lib server::tls` | ❌ Wave 0 |
| LNK-07 | enroll: код одноразовый, срок, гонка двойного погашения | integration (axum `build_router`) | `cargo test -p trackly-app --test app_server -- link_enroll` | ❌ Wave 0 |
| LNK-07 | password login → `device_token`; `link_resume`; logout отзывает токен; 429 | integration | `... -- link_auth` | ❌ |
| LNK-07/D-12..14 | revoke устройство/пользователя/блокировка → 401 `LINK_REVOKED` на следующем запросе; WS закрыт | integration (`ws_heartbeat`-стиль) | `... -- link_revoke` | ❌ |
| LNK-07/D-13 | отозванное устройство не входит ни паролем, ни SSO | integration | `... -- link_revoked_device` | ❌ |
| LNK-07 | миграция V060, `user_version = 60` | infra | `cargo test -p trackly-infra --test infra_tests link_migration` | ❌ |
| D-12 | каждый новый маршрут в реестрах/паритет | integration | `cargo test -p trackly-app --test app_server -- http_route_parity` | ✅ (обновить вердикты) |
| LNK-06 | SSO-клиент: сборка заголовка, классификация отказов, IP → без попытки, пароль не повторяется | unit с `MockSso` | `cargo test -p link-app auth::sso` | ❌ |
| LNK-06 | **реальный Kerberos-обмен** | **manual (Windows+AD)** | — | UAT |
| LNK-09 | автомат: 5 с задержка индикации, backoff до 30 с, сброс после сна, «нет связи» ≠ «вход недействителен» | unit (виртуальное время `tokio::time::pause`) | `cargo test -p link-app core::` | ❌ |
| LNK-09 | ресинк после реконнекта (`trackly-resync` эмитится) | integration (мок-WS сервер) | `cargo test -p link-app net::ws` | ❌ |
| D-01/D-02 | конфиг: приоритет машинный > пользовательский, невалидный, отсутствует | unit | `cargo test -p link-app config` | ❌ |
| D-03 | разбор кода-приглашения, CRC, срок, отказ при несовпадении пина (ничего не сохранено) | unit + integration | `cargo test -p link-proto invite` ; `cargo test -p link-app setup` | ❌ |
| LNK-03/D-17 | сборка значения `Run` с кавычками; логика «восстановить если нет» | unit (чистая функция + фейковый backend) | `cargo test -p link-app shell::autostart` | ❌ |
| LNK-04/LNK-11/D-20 | решение окна: трей есть/нет → hide/minimize, skip_taskbar | unit (чистая функция `window_policy`) | `cargo test -p link-app shell::window` | ❌ |
| LNK-05 | single-instance: логика «второй запуск → show+focus» | unit callback; **реальный RDS — manual** | `cargo test -p link-app` | частично |
| LNK-02 | иконка/меню: пункты, tooltips, серая иконка | unit (сборка модели меню) | `cargo test -p link-app shell::tray` | ❌ |
| UI | экраны Link: состояния, тексты UI-SPEC | `svelte-check`, `check-link-layout.mjs` | `pnpm --dir link-ui lint && pnpm --dir link-ui svelte-check` | ✅ (расширить) |
| UI | раздел «Link» в Trackly, карточка пользователя | `svelte-check`, `check-tokens.mjs` | `pnpm --dir ui lint && pnpm --dir ui svelte-check` | ✅ |
| Privacy | в тестах/фикстурах нет реальных реквизитов | скрипт | `node scripts/check-privacy.mjs` | ✅ |

### Sampling Rate
- **Per task commit:** скоуп-тест затронутой цели + соответствующий `check-*.mjs`
- **Per wave merge:** целевые suite (`app_server`, `infra_tests`, `link-app`, `link-proto`, `http_route_parity`) + `pnpm lint`/`svelte-check` обоих UI + clippy + fmt
- **Phase gate:** сплошной `ci-full` зелёный; `check-privacy.mjs`; мутационная проверка несущих тестов (пин: заменить сравнение на `true` → тест должен покраснеть; отзыв: снять проверку в middleware → красный)

### Wave 0 Gaps
- [ ] `crates/link-proto` (+ golden-фикстура PEM, `tests/`)
- [ ] фикстура локального TLS-сервера для `net::pin` (rcgen, в тестах)
- [ ] `MockSso`, `InMemoryStore` (keyring-трейт), фейковый `ConfigSource`
- [ ] suite-файл серверных тестов Link в `tests/suites/` + регистрация в `suite_wiring`
- [ ] human-verify чекпоинт на установку новых крейтов (slopcheck не запускался)

### Только живьём на Windows/Linux (human-UAT; по моку «проверено» не ставить)
1. LNK-06: автовход на доменном ПК по DNS-имени; по IP → форма с причиной; без SPN → причина; расхождение часов.
2. LNK-02/04: иконка в трее (скрытые значки), левый/правый клик, «Закрыть» завершает процесс, закрытие окна прячет; **перезапуск Explorer** — иконка вернулась; DPI 100/125/150/200%.
3. LNK-03: автозапуск после перезагрузки (запись в `Run` с кавычками, путь с пробелом), старт без окна; удалить запись → Link восстановит.
4. LNK-05: RDS — два пользователя, у каждого свой экземпляр; повторный запуск в одном сеансе фокусирует окно.
5. LNK-07: отзыв из раздела «Link» и из карточки пользователя — окно открывается с «Вход отозван администратором»; блокировка пользователя; отозванный ПК не входит автовходом.
6. LNK-08: подмена сертификата сервера (другой ключ) → экран D-07, ни одного запроса (проверить журналом сервера).
7. LNK-09: сон/пробуждение ПК, отключение Wi-Fi, перезапуск сервера: плашка не раньше ~5 с, догрузка после возврата.
8. LNK-11: Ubuntu GNOME без AppIndicator — окно остаётся на панели, «Закрыть» в окне; с расширением — иконка и меню.
9. keyring: Windows Credential Manager запись `org.trackly.link…` появилась и удаляется при «Выйти»; macOS Keychain — без падения; Linux без Secret Service — токен в памяти.

## Security Domain

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | yes | argon2id (пароли, существующее), `device_token`/`device_secret` 256 бит `OsRng`, SHA-256 на сервере; SPNEGO по keytab; rate-limit `tower_governor`; пароль не повторять и не хранить |
| V3 Session Management | yes | `tower-sessions` (SQLite), flush при входе (T-05-SF), принудительный сброс сессии при отзыве (`link_guard`) |
| V4 Access Control | yes | раздел «Link», выпуск кодов, отзыв — только admin (серверная проверка роли; UI-скрытие — UX) |
| V5 Input Validation | yes | валидация кода/адреса/пина/`device_id` (UUID), длины заголовков; serde-DTO |
| V6 Cryptography | yes | rustls + ring, SPKI SHA-256, constant-time сравнение; ничего самописного кроме сравнения отпечатка |
| V9 Communications | yes | TLS только с пином; `danger_accept_invalid_certs` запрещён; http1 |
| V10 Malicious code / supply chain | yes | новые крейты — human-verify, `cargo deny` |

### Known Threat Patterns
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| MITM с чужим самоподписанным сертификатом | Spoofing | пин SPKI; несовпадение → только ошибка, данные не отправляются (D-07) |
| Перехват/повтор кода-приглашения | Spoofing/Info disclosure | одноразовость + срок + 128-бит случайности, на сервере только хэш, погашение атомарное |
| Перебор кода/пароля | Brute force | governor на `link_enroll`, `link_auth_login`; блокировка авто-повтора пароля на клиенте |
| Кража `device_token` с диска | Info disclosure | keyring (DPAPI/Keychain/Secret Service); токен отзываем; в файлах/логах токена нет |
| Подмена сервера при enrollment | Spoofing | отпечаток из кода сверяется до отправки секретов кода |
| Сессия живёт после блокировки/отзыва | Elevation of privilege | `link_guard` на каждом запросе + `LinkHub` разрыв WS |
| Утечка Negotiate/токена в логи | Info disclosure | не логировать заголовки `Authorization`/токены; тест на отсутствие в логах |
| Компрометация webview → сеть | Tampering | CSP без сети, capabilities без shell/fs/dialog; все `link_*` команды валидируют вход |
| Затирание конфига пользователем | Tampering | машинный конфиг приоритетнее (ACL администратора) |

## Sources

### Primary (HIGH confidence)
- Исходники локально: `crates/trackly-app/src/http/sso.rs`, `http/auth.rs`, `http/mod.rs` (`refresh_session_role`, `SessionManagerLayer`), `http/ws.rs`, `server/tls.rs`, `server/rusqlite_session_store.rs`, `crates/link-app/*`, `packages/transport/src/ws.ts`, `tests/http_route_parity.rs`, `migrations/V059`
- Исходники крейтов (скачаны с crates.io): `tauri-plugin-single-instance 2.5.5` (windows.rs), `tauri-plugin-window-state 2.5.0`, `tray-icon 0.25.1` (windows/mod.rs, TaskbarCreated), `auto-launch 0.6.0` (windows/macos/linux), `tauri-plugin-autostart 2.7.0`, `reqwest 0.13.5` (`tls_backend_preconfigured`, features), `keyring-core 1.1.0`, `windows-/apple-native-/zbus-secret-service-keyring-store`
- `cross-krb5` (github.com/estokes/cross-krb5): windows.rs (`Negotiate`, `verify_kerberos_negotiated`), lib.rs (`InitiateFlags::NEGOTIATE_TOKEN`)
- ADR 0002/0004/0005, 44-UI-SPEC.md, `.planning/research/{ARCHITECTURE,PITFALLS}.md`

### Secondary (MEDIUM confidence)
- sspi-rs README (github.com/Devolutions/sspi-rs): оговорка о непроверенном исходящем Kerberos
- crates.io API/`cargo search` (версии на 2026-11-27)

### Tertiary (LOW confidence)
- Поведение Windows при Run-значении без кавычек (A3), семантика `CreateMutexW` по сеансам (A4), детект трея GNOME (A7) — подтверждать живьём

## Metadata

**Confidence breakdown:**
- Standard stack: MEDIUM — версии и API проверены по исходникам; slopcheck не запускался; `cross-krb5` не проверен на совместимость с серверным acceptor
- Architecture: MEDIUM-HIGH — опирается на существующие паттерны репозитория (`refresh_session_role`, `ServerShutdown`, реестровые гейты)
- Pitfalls: MEDIUM — исходники подтверждают Pitfalls 3, 4, 6; Windows/Linux-поведение — UAT

**Research date:** 2026-11-27
**Valid until:** 2026-12-27 (стек стабильный; `reqwest`/`tokio-tungstenite` — сверить минорные версии перед установкой)
