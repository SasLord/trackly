# Project Research Summary

**Project:** Trackly — веха v1.5 «Trackly Link — клиент и Чат»
**Domain:** внутренний мессенджер (десктоп-клиент Link на Tauri 2 + чат в серверном/браузерном Trackly) для одной организации в Windows/AD LAN, с самообновлением клиента с сервера
**Researched:** 2026-11-25
**Confidence:** MEDIUM (код репозитория — HIGH; платформенные факты Windows/Tauri и клиентский SPNEGO — нужны спайки)

Источники: `STACK.md`, `FEATURES.md`, `ARCHITECTURE.md`, `PITFALLS.md` (этот каталог). Исследование v1.0 — `v1.0/`.

## Executive Summary

v1.5 — две связанные подсистемы поверх готового Trackly: **Link** (второй Tauri-клиент в том же
монорепо: трей, автозапуск, per-user NSIS, самообновление, SPNEGO/пароль, закрепление TLS) и **Чат**
(диалоги, комнаты, rich-text, реакции, поиск, уведомления — одинаково в Link и браузерной SPA).
Опорная схема доставки — «сигнал + догрузка»: WS только подсказывает «что-то изменилось», истина в
БД, клиент добирает по курсору. Это же закрывает долг «WS Lagged».

Рекомендованный подход: вся сеть Link — в Rust-части (reqwest + tokio-tungstenite на rustls+ring,
свой `ServerCertVerifier` для пина, нативный SSPI, токен в ОС-хранилище); webview грузит локальный
бандл и сети не видит. Чат — новая независимая модель `im_*` (не наращивать `chat_*` заявок),
`seq`/`rev` назначаются внутри замыкания единственного писателя, `last_read_seq` на участника;
rich-text — JSON TipTap 3/ProseMirror, строго валидируемый на сервере, рендер по узлам без
`{@html}`; FTS5 с проверкой прав внутри SQL; фан-аут — новый `ImHub` по пользователям, а не общий
`ws_broadcast`. Обновления строятся раньше чата: клиент без рабочего самообновления на сотнях ПК
потом не исправить.

Главные риски: (1) AppLocker/SRP/EDR могут блокировать запуск из `%LOCALAPPDATA%` — до пилота на
реальных доменных ПК «вариант A» под вопросом; (2) клиентский SPNEGO из Rust и updater с
самоподписанным TLS не проверены — спайки; (3) XSS и утечки через rich-text и мягкое удаление —
серверная канонизация + единый слой видимости; (4) потеря ключа подписи / ротация пина —
неизлечимо для клиентов; (5) dev на macOS без AD/Windows — любой Windows-пункт UNVERIFIED до живой
проверки.

## Key Findings

### Recommended Stack

Базис не меняется: Rust 1.92, Tauri 2.12, Svelte 5, Vite 6, rusqlite 0.41, refinery 0.10, axum,
rustls/ring, pnpm (версии — по Cargo.toml/package.json, не CLAUDE.md).

- **Оболочка Link:** `tauri` с фичей `tray-icon`; плагины autostart (`--hidden`), single-instance,
  notification, window-state, opener. Не подключать `shell`/`fs`/`dialog`.
- **Updater:** `tauri-plugin-updater` — только Link (в portable Trackly по-прежнему нет); динамический
  endpoint, идентичность заголовками из Rust; подпись ed25519; NSIS `currentUser`.
- **Сеть:** `reqwest` 0.13 + `tokio-tungstenite` 0.30 с одним общим pinned `ClientConfig`, провайдер
  только ring. Не `tauri-plugin-http` (нет хука под pin).
- **SPNEGO:** нативный SSPI через уже имеющийся `windows` 0.62 (~150 LOC, `cfg(windows)`, mock-трейт
  для macOS). Отклонён `sspi` 0.23 (MSRV 1.93 > 1.92); `kenobi` — референс.
- **Секреты:** `keyring-core` + платформенные store-крейты; Linux без Secret Service — токен в памяти.
- **Редактор:** TipTap 3 (MIT), `@tiptap/*` одной версии, таблицы с merge через бесплатный
  `extension-table`; Pro/Cloud — запрет.
- **Эмодзи:** Twemoji (jdecked) 17 SVG из локальной статики (в LAN нет CDN), `@twemoji/parser`,
  `emojibase-data` (ru); в данных — Unicode; атрибуция CC-BY в THIRD-PARTY-NOTICES.
- **Валидация rich-text:** типизированные serde-структуры в `trackly-core` (`deny_unknown_fields`,
  allowlist, лимиты); без `ammonia`, без HTML в БД.

### Expected Features

**Must have (table stakes):** диалоги 1:1 с поиском по всем пользователям; комнаты с Создателем и
передачей прав (самый давний участник), системные сообщения; `seq` + курсорная догрузка +
идемпотентная отправка (`client_msg_id`); `last_read_seq`, счётчики, разделитель «Новые сообщения»,
группировка, состояние «нет связи»/статус отправки (не названы пользователем, но без них мессенджер
«сломан»); reply; правка с карандашом; мягкое удаление (тело клиентам не отдаётся); вложения
(переиспользовать 41.3); эмодзи; реакции (≤3, старая снимается); @упоминания; поиск; «Информация о
сообщении»; mute, тосты, системные уведомления, индикатор трея; модерация админом с аудитом.

**Should have:** закреп сообщений/чатов, ссылки на сущности Trackly; расширенный rich-text (заголовки,
sup/sub, выравнивание, отступы) — дёшево в TipTap, но раздувает валидатор/рендер.

**Defer:** таблицы с merge/split (самое дорогое, последний пакет, кандидат на отсечение);
inline-картинки, presence/«печатает», пересылка, `@all`, DND по расписанию, аватары комнат, превью
ссылок (SSRF), экспорт, треды, звонки, E2E, история правок; перенос чата заявок на новую модель.

### Architecture Approach

Новые `crates/trackly-proto` (общие serde+specta DTO без tauri/rusqlite), `crates/link-app` (своя
версия, не `version.workspace`), `pnpm-workspace` с `packages/{design,kit,transport,attachments,messenger,proto-ts}`
и `link-ui/`; `trackly-app` в клиент не тянуть. Отдельный `release-link.yml` на теги `link-v*.*.*`.
Чат: `im_*` (миграции с V058, каждая с `PRAGMA user_version`), надгробия вместо физического DELETE,
`rev` как курсор state-sync, `ImHub` (`DashMap<user_id, broadcast>`), события без тела. Сессии
`tower-sessions` переиспользуются для не-браузера; вне домена — `device_token`/`link_resume`;
`link_devices` (`device_id`). Обновления: `link_releases`, `link_channel_heads` + override; приоритет
канала: компьютер → пользователь → умолчание; сервер проверяет подпись при загрузке, приватного ключа
не хранит.

**Major components:**
1. `LinkTransport` (Rust, link-app) — pinned HTTP/WS, SSPI, keyring, `api_call` c allowlist, URI-схема вложений.
2. `ImService` + `ImHub` + `domain/im` — модель, правила (Создатель, лимит реакций, валидатор), доставка, `im_sync`.
3. `LinkService` — релизы, каналы, устройства, endpoint обновлений, раздел «Link».
4. `packages/messenger` — единый UI чата для Link и браузера.

### Critical Pitfalls

1. **XSS/инъекции через rich-text (C-1..C-3)** — серверная канонизация JSON + `schema_version`; рендер
   по узлам без `{@html}`; `image` только `attachment_id`; общая golden-фикстура Rust/JS; без превью ссылок.
2. **Утечка/потеря событий WS (C-4, C-5)** — события без тела; членство при каждой выдаче; закрытие
   сокета при выходе/отзыве сессии; `Lagged` → `resync_required`; heartbeat; `client_msg_id`.
3. **Мягкое удаление «протекает» (C-10)** — единый слой `visible_body` для ленты/поиска/цитат/превью/
   тоста/ресинка; чистка FTS и вложений той же транзакцией; отдельный админский DTO.
4. **Ключ подписи и пин (C-6, M-4)** — резервная копия ключа вне репо + закладка ротации; пин SPKI
   (несколько хэшей); отпечаток от установщика/конфига; никакого `danger_accept_invalid_certs`;
   мониторинг срока сертификата.
5. **Утечка канала «тестовый» (C-7) и AppLocker/SRP (C-11)** — один URL, одна форма манифеста, версии
   без суффиксов, откат = новый больший номер, гейт-grep бандла клиента; пилот на реальных доменных ПК
   — первым действием дистрибуции.

Ещё: реакции одним write-job + идемпотентный API (C-8); не смешивать с `chat_*` (C-9); `spa_fallback`
→ 404 для `/api/*` (m-1); непрочитанное только из курсора (M-11); приватность фикстур, адрес сервера и
отпечаток не в публичном релизе (M-17).

## Implications for Roadmap

Сведены порядки ARCHITECTURE (12) и PITFALLS (14). Нумерация с 42 (бывшие 42–45 ушли в v1.8).
Решения при расхождениях: обновления раньше чата; ADR и пилот AppLocker — первым делом; модель и
валидация раньше UI, схема документа фиксируется до серверной фазы; слой видимости/FTS/надгробия — в
серверной фазе; откат = новый больший номер (`version_comparator` — запасной); пин SPKI заданным
отпечатком, TOFU только для ручной установки вне домена.

| # | Фаза | Суть | Закрывает |
|---|------|------|-----------|
| 42 | Основа монорепо и ADR | ADR (im_*, сеть в Rust, пин, токен, откат); pnpm-workspace + перенос гейтов `check-*.mjs`; `trackly-proto` + `proto-ts` с гейтом дрейфа; каркас link-app/link-ui (окно, трей, NSIS currentUser); `release-link.yml` dry-run; `spa_fallback` 404; пилот AppLocker/SRP/WebView2/NSIS `/S` | C-9, C-11, m-1, m-9 |
| 43 | Надёжная доставка | `resync_required`, `hello`, серверный ping; resync на `onopen`; каркас `ImHub`; ревалидация сессии/членства сокета | C-4, C-5 |
| 44 | Link: транспорт и вход | pinned `LinkTransport`, `api_call` allowlist, WS backoff; `link_devices`, `device_token`; пароль + SPNEGO (SSPI) + mock; keyring; автозапуск, single-instance per-user | M-4, M-5, M-7, M-8, m-8 |
| 45 | Дистрибуция и автообновление | релизы, каналы + override, endpoint, проверка подписи при загрузке; раздел «Link» (релизы, клиенты); updater + откат; GPO-инструкция; E2E 0.0.1→0.0.2 на Windows; процедура ключей; гейт утечки канала | C-6, C-7, M-1..M-3, M-17 |
| 46 | Дизайн-прототипы (ворота) | интерактивные экраны чата, редактора, реакций, «Информации», уведомлений/трея, раздела «Link»; параллельно 43–45, утвердить до 48 | — |
| 47 | Чат: серверная модель | `im_*` + FTS5; `domain/im`; `ImService`, `im_sync`; `visible_body`, надгробия; Actions; реакции одним write-job; аудит модерации; тесты инвариантов и XSS-корпус | C-1..C-4, C-8, C-10, M-11, M-13 |
| 48 | Чат: UI-ядро (браузер) | `packages/messenger`: список, диалоги/комнаты, простой текст, непрочитанное, «Новые», группировка, статус отправки, черновики; `/chat` в SPA | M-11, M-15 |
| 49 | Редактор, эмодзи, вложения | TipTap (заголовки, начертания, списки, цитата, код, ссылки, @, reply); эмодзи; вложения; таблицы с merge — последним, отсекаемо | C-1, M-9, M-14, M-16 |
| 50 | Операции над сообщениями | правка, мягкое/жёсткое удаление, реакции с облачком, «Информация», UI поиска | C-8, C-10, M-10, M-12 |
| 51 | Link: чат и уведомления | messenger в link-ui; системные тосты с троттлингом, mute, счётчики, трей | M-6, M-7 |
| 52 | Раздел «Link» и закрытие | модерация, каналы по пользователям/ПК в UI; нагрузочный тест; сквозная UAT на Windows; аудит приватности | m-10, M-10 |

### Research Flags (спайки до планирования)

1. Пилот AppLocker/SRP/WebView2/NSIS `/S` на реальных доменных ПК (старт в 42, до плана 45).
2. Клиентский SPNEGO (SSPI) и совместимость с серверным `sso.rs` (SPN, имя хоста вместо IP).
3. Updater + самоподписанный TLS (флаг небезопасного TLS vs свой загрузчик с `minisign-verify`), ротация ключей, откат, поведение при запущенном приложении, Authenticode.
4. Pin SPKI на reqwest 0.13/ring и tokio-tungstenite (один verifier).
5. FTS5 trigram в bundled rusqlite; `keyring-core` на трёх ОС.
6. TipTap 3 + Svelte 5: состав StarterKit, merge ячеек, схема документа, golden-фикстура; IME/русская раскладка в WKWebView/WebView2; `dragDropEnabled`.
7. AUMID и тосты при установке из NSIS; трей GNOME, перезапуск Explorer.

Стандартные паттерны (без отдельного исследования): 43, 48, 50, 51, 52.

## Открытые продуктовые вопросы

1. История для новых участников комнаты: вся (рекомендация) или с момента вступления.
2. «Когда прочитал»: точный журнал `im_read_advances` (рекомендация) или приблизительно; кто видит прочтения.
3. Чат в десктопном Trackly (действует от системного «Администратора»): скрыть (рекомендация) или дать личность.
4. @-упоминание и ответ пробивают mute комнаты, но не mute приложения (рекомендация).
5. История правок: не хранить (рекомендация) или хранить для админа.
6. RDS/несколько пользователей на одном ПК: поддерживать в v1.5 или нет.
7. Перенос чата заявок на новую модель — отложить (рекомендация).

Дополнительно: читает ли админ личные 1:1 (рекомендация — только удалённое + аудит); `@all`; лимиты
вложений и блок исполняемых расширений; Authenticode (внутренний корень через GPO или покупной);
TOFU для ПК вне домена; можно ли отсечь таблицы с merge; автообновление macOS/Linux в v1.5 или только Windows.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | MEDIUM-HIGH | версии сверены с crates.io/npm; SSPI, TLS-фичи reqwest 0.13, keyring-core, updater+TLS — спайки |
| Features | MEDIUM | опора на код заявок HIGH; поведение Telegram/Slack и лимиты — по памяти |
| Architecture | MEDIUM-HIGH | код прочитан, согласовано с single-writer; FTS trigram и updater+TLS не проверены |
| Pitfalls | MEDIUM | привязки к коду HIGH; платформенные факты помечены `[verify]` |

**Overall confidence:** MEDIUM

### Gaps to Address

- Среда организации (AppLocker/SRP, WebView2, RDS) — только пилот; результаты записывать обобщённо.
- Расхождения решить в ADR фазы 42: `keyring-core` (не `keyring`); крейт `link-app`; FTS trigram с запасным `unicode61`+префиксы; пин заданным отпечатком.
- Лимит вложений из 41.3 не сверен с кодом.
- macOS при ad-hoc подписи — допустимо «автообновление только Windows» в v1.5.
- Все Windows-пункты UNVERIFIED до живой проверки; auto-approve human-verify исполнителями не принимать.
- Пути lint-гейтов при переезде фронта в `packages/` — механический риск, заложить запас.

## Sources

- Primary: код репозитория (`http/ws.rs`, `context.rs`, `services/chat_service.rs`, `http/sso.rs`, `ui/src/lib/api/*`, `ui/package.json`, `release.yml`, миграции до V057); документация Tauri v2 (updater, windows-installer); crates.io/npm на 2026-11-25.
- Secondary: Context7 plugins-workspace, keyring-rs, sspi-rs, kenobi; OWASP XSS/CSP, ProseMirror schema, SQLite WAL/FTS5, Kerberos/SSPI.
- Tertiary (проверить): поведение Telegram/Slack/Mattermost/Teams; AppLocker/SRP, AUMID, трей GNOME; overlay v1.6 на macOS/Wayland.

---
*Research completed: 2026-11-25*
*Ready for roadmap: yes (после ответов на открытые вопросы)*
