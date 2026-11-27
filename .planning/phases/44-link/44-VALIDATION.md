---
phase: 44
slug: link
status: draft
nyquist_compliant: true
wave_0_complete: true
created: 2026-11-27
---

# Phase 44 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Источник: `44-RESEARCH.md` → `## Validation Architecture`.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (`link-app`, `link-proto`, `trackly-app` suite-бинарники `tests/suites/*.rs`, `trackly-infra --test infra_tests`); Node-гейты `ui/scripts/check-*.mjs`; `link-ui` `pnpm lint` + `svelte-check` |
| **Config file** | `crates/trackly-app/Cargo.toml` (`[[test]]`, `autotests=false`), `link-ui/package.json` |
| **Quick run command** | `cargo test -p link-app` / `cargo test -p link-proto` / `cargo test -p trackly-app --test <suite> <filter>` |
| **Full suite command** | `ci-full` (один раз на границе последней волны, в фоне; один `cargo test` одновременно) |
| **Estimated runtime** | скоуп ~30–120 с; сплошной — длинный, только в фоне |

Отдельные гейты (не входят друг в друга): `pnpm --dir link-ui lint`, `pnpm --dir link-ui svelte-check`, `pnpm --dir ui lint`, `pnpm --dir ui svelte-check`, `cargo clippy -- -D warnings`, `cargo fmt --check`, `node scripts/check-privacy.mjs`.

---

## Sampling Rate

- **After every task commit:** скоуп-тест затронутой цели + соответствующий `check-*.mjs`
- **After every plan wave:** целевые suite (`app_server`, `infra_tests`, `link-app`, `link-proto`, `http_route_parity`) + `pnpm lint`/`svelte-check` обоих UI + clippy + fmt
- **Before `/gsd-verify-work`:** сплошной `ci-full` зелёный; `check-privacy.mjs`; мутационная проверка несущих тестов (пин → `true` краснеет; снять проверку отзыва в middleware → краснеет)
- **Max feedback latency:** ~120 с на скоуп

---

## Per-Task Verification Map

| Req | Behavior | Test Type | Automated Command | File Exists | Status |
|-----|----------|-----------|-------------------|-------------|--------|
| LNK-08 | SPKI совпал → ok; не совпал → отказ, ни один запрос не ушёл; имя хоста игнорируется | unit + интеграция (локальный TLS) | `cargo test -p link-app net::pin` | ✅ | ✅ green |
| LNK-08/D-06 | `spki_sha256` golden; смена SAN → тот же SPKI | unit | `cargo test -p link-proto`; `cargo test -p trackly-app --lib server::tls` | ✅ | ✅ green |
| LNK-07 | enroll: код одноразовый, срок, гонка погашения | integration | `cargo test -p trackly-app --test app_server -- link_enroll` | ✅ | ✅ green |
| LNK-07 | пароль → `device_token`; resume; logout; 429 | integration | `... -- link_auth` | ✅ | ✅ green |
| LNK-07 | отзыв устройства/пользователя/блокировка → 401 `LINK_REVOKED`, WS закрыт | integration | `... -- link_revoke` | ✅ | ✅ green |
| LNK-07 | отозванное устройство не входит ни паролем, ни SSO | integration | `... -- link_revoked_device` | ✅ | ✅ green |
| LNK-07 | миграция V060, `user_version = 60` | infra | `cargo test -p trackly-infra --test infra_tests link_migration` | ✅ | ✅ green |
| — | новые маршруты в реестрах/паритете | integration | `cargo test -p trackly-app --test app_server -- http_route_parity` | ✅ | ✅ green |
| LNK-06 | SSO-клиент: заголовок, классификация отказов, IP → без попытки | unit (`MockSso`) | `cargo test -p link-app auth::sso` | ✅ | ✅ green |
| LNK-09 | автомат связи: 5 с, backoff ≤30 с, сброс после сна, «нет связи» ≠ «вход недействителен» | unit (`tokio::time::pause`) | `cargo test -p link-app core::` | ✅ | ✅ green |
| LNK-09 | ресинк после реконнекта | integration (мок-WS) | `cargo test -p link-app net::ws` | ✅ | ✅ green |
| D-01/D-02 | конфиг: приоритет, невалидный, отсутствует | unit | `cargo test -p link-app config` | ✅ | ✅ green |
| D-03 | код-приглашение: CRC, срок, несовпадение пина → ничего не сохранено | unit + integration | `cargo test -p link-proto invite`; `cargo test -p link-app setup` | ✅ | ✅ green |
| LNK-03 | значение `Run` в кавычках; восстановление записи | unit | `cargo test -p link-app shell::autostart` | ✅ | ✅ green |
| LNK-04/LNK-11 | политика окна: трей есть/нет → hide/minimize, skip_taskbar | unit | `cargo test -p link-app shell::window` | ✅ | ✅ green |
| LNK-05 | второй запуск → show+focus | unit | `cargo test -p link-app` | ✅ | ✅ green |
| LNK-02 | модель меню трея, серая иконка | unit | `cargo test -p link-app shell::tray` | ✅ | ✅ green |
| UI | экраны Link по UI-SPEC | lint | `pnpm --dir link-ui lint && pnpm --dir link-ui svelte-check` | ✅ | ✅ green |
| UI | раздел «Link» в Trackly | lint | `pnpm --dir ui lint && pnpm --dir ui svelte-check` | ✅ | ✅ green |
| Privacy | нет реальных реквизитов | скрипт | `node scripts/check-privacy.mjs` | ✅ | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [x] `crates/link-proto` (+ golden-фикстура PEM, `tests/`)
- [x] фикстура локального TLS-сервера для `net::pin` (rcgen)
- [x] `MockSso`, `InMemoryStore` (keyring-трейт), фейковый `ConfigSource`
- [x] suite-файл серверных тестов Link в `tests/suites/` + регистрация в `suite_wiring`
- [x] human-verify чекпоинт на установку новых крейтов (slopcheck не запускался)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Автовход на доменном ПК по DNS-имени; по IP → форма с причиной; без SPN; расхождение часов | LNK-06 | нет AD на dev macOS | Windows-ПК в домене |
| Иконка в трее, клики, «Закрыть», перезапуск Explorer, DPI | LNK-02, LNK-04 | реальная оболочка Windows | Windows |
| Автозапуск после перезагрузки, путь с пробелом, восстановление записи | LNK-03 | реестр Windows | Windows |
| RDS: два пользователя — два экземпляра | LNK-05 | терминальный сервер | Windows RDS |
| Отзыв из раздела «Link» и карточки пользователя | LNK-07 | сквозной сценарий | Trackly + Link |
| Подмена сертификата → экран ошибки, ни одного запроса в журнале сервера | LNK-08 | сквозной сценарий | Trackly + Link |
| Сон/пробуждение, отключение сети, перезапуск сервера | LNK-09 | реальная ОС | Windows |
| GNOME без AppIndicator — окно доступно | LNK-11 | Linux-ВМ | Ubuntu GNOME |
| Keyring: Credential Manager / Keychain / без Secret Service | LNK-07 | хранилища ОС | все ОС |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 120s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** pending (автоматика зелёная 2026-11-28; статус approved - после живой проверки, раздел Manual-Only)
