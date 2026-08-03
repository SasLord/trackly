# Phase 32: Авто-админ по списку логинов + релиз SSO в main - Context

**Gathered:** 2026-08-03
**Status:** Ready for planning

<domain>
## Phase Boundary

Доверенный список доменных логинов (аналог `ADMIN_AD_LOGINS` из adwebapp) → любой
логин из этого списка при SSO-входе сразу получает роль «Администратор» как активный
пользователь, без промежуточной заявки на подтверждение. Решает «проблему первого
администратора» для AD-only организаций. Реализуется как расширение существующего
провижининг-сема `AuthService::on_ad_bind_success` (Phase 31).

Плюс операционный итог фазы/milestone v1.3 (не привязан к отдельному REQ-ID): вывод
SSO-функциональности из спайкового статуса — мерж `spike/ad-sso-kerberos` в `main` и
релиз обычной трёхсегментной версии `v1.3.0` вместо спайковых тегов `0.0.x`.

**В scope:** список admin-логинов в конфиге; форс-роль admin + мгновенная активация
для логинов из списка; приоритет над group→role и над `ad_auto_accept=OFF`; мерж в
main + релизный тег.

**Вне scope:** UI для добавления/удаления логинов в списке (deferred); изменение
Phase-31 пути провижининга для логинов ВНЕ списка (остаётся как есть).

</domain>

<decisions>
## Implementation Decisions

### Хранение и конфигурация списка
- **D-01:** Список хранится в **TOML-конфиге** рядом с exe — новое поле в `AppConfig.ad`
  (например `admin_logins: Vec<String>`), рядом с существующим `role_mapping`
  (`config.rs:233`). НЕ в таблице `app_settings`, НЕ в UI.
- **D-02:** Обоснование выбора конфига над БД/UI: (1) решает chicken-and-egg — не нужен
  уже существующий администратор, чтобы открыть Settings и назначить первого; (2) прямой
  аналог `ADMIN_AD_LOGINS` (env/конфиг deployment-time); (3) portable-режим — переносится
  вместе со сборкой; (4) согласовано с тем, как Phase 31 хранит group→role маппинг
  (тоже TOML `role_mapping`, не app_settings).
- **D-03:** В этой фазе список редактируется ТОЛЬКО правкой конфиг-файла (deployment-time).
  Никакого редактирования из UI. Пустой/отсутствующий `admin_logins` = фича выключена,
  никто не получает авто-admin.

### Провижининг и приоритет
- **D-04:** Логин из списка → форсированная роль `admin` + немедленная активация
  (`is_active=1`), **в обход** `ad_auto_accept=OFF` (обычно этот путь ушёл бы в
  `create_pending_registration` с заявкой). Список — это явный «мгновенный» путь.
- **D-05:** Приоритет: `admin_logins` **побеждает** над `role_hint` из group→role
  (Phase 31 `directory.resolve`) и над состоянием `ad_auto_accept`. Если логин и в
  списке, и замаплен группой на меньшую роль — применяется `admin`.
- **D-06:** Список применяется на **КАЖДОМ** SSO-входе, а не только при первом
  провижининге (паритет с `ADMIN_AD_LOGINS`). Существующий активный пользователь-
  `employee`, добавленный в список, повышается до `admin` на следующем SSO-входе
  (эскалация существующего юзера — ожидаемое поведение).
- **D-07:** `admin_logins` — авторитетный deployment-конфиг → побеждает и над
  pending-состоянием (никогда не одобрялся) и над blocked/soft-deleted состоянием:
  логин из списка становится активным admin даже если ранее был pending/blocked.
  Обоснование: файл конфига правит только тот, кто контролирует деплой сервера — это
  высшая инстанция доверия. **Планировщик/verifier: явно отметить это как security-
  значимое решение** (список обходит ручную блокировку админом).
- **D-08:** Логины ВНЕ списка проходят прежний путь Phase 31 без изменений:
  auto-register / pending-заявка / group→role маппинг. Список НЕ расширяет доступ
  никому, кроме явно перечисленных логинов (Success Criteria #3).

### Формат логинов и матчинг
- **D-09:** Матчинг по форме **sAMAccountName** (чистый логин, напр. `us100`),
  **case-insensitive**. Именно `sam_account_name`/`ad_username` доходит до
  `on_ad_bind_success` из SPNEGO-бинда (см. `ad_directory.rs` — `resolve(sam_account_name)`).
- **D-10:** Проверка членства — чисто локальная set-операция, **без обращения к каталогу**.
  Значит она НЕ подвержена fail-closed проблемам group→role: логин из списка получает
  admin даже при `DirectoryError::Unreachable`/`NotConfigured` (в отличие от `role_hint`,
  который требует достижимого каталога). Это преимущество — авто-admin работает, даже
  когда directory-enrichment недоступен.

### Мерж и релиз (операционный итог)
- **D-11:** Порядок: сначала код SSO-02 + verify фазы 32 на ветке
  `spike/ad-sso-kerberos`; затем мерж `spike/ad-sso-kerberos` → `main`; затем пуш
  трёхсегментного тега `v1.3.0` (триггерит `release.yml`). Версия выходит из спайковой
  линейки `0.0.x` в обычный релизный тег.
- **D-12:** SSO/Kerberos собран за Cargo-feature (gssapi, Windows-gated). Мерж в main
  НЕ должен ломать сборку/CI на macOS/Linux — мок-путь (`TRACKLY_AD_MOCK`) должен
  оставаться зелёным. Планировщик должен проверить, что фича-гейтинг и CI-матрица это
  учитывают перед мержем.

### Claude's Discretion
- Точное имя TOML-ключа и поля структуры (`admin_logins` — предложение).
- Где именно в `on_ad_bind_success` (или в `sso_login`/`try_ad_login`) сидит проверка
  членства — до/после `find_user_any_state`. Важно лишь итоговое поведение из D-04..D-08.
- Имя `action` в `audit_log` для авто-admin события (напр. `ad_auto_admin`).
- Нужен ли отдельный helper `is_admin_login(login)` на `AuthService` — на усмотрение.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Требования и roadmap
- `.planning/ROADMAP.md` §"Phase 32" — goal, Success Criteria 1–3, note про мерж/релиз.
- `.planning/REQUIREMENTS.md` — SSO-02 (строки 22–24), связь с SSO-03 (Phase 31).

### Провижининг-сем (место интеграции)
- `crates/trackly-app/src/services/auth.rs` — `on_ad_bind_success` (`:404`),
  `auto_register_ad_user` (`:531`), `create_pending_registration`,
  `reuse_or_create_pending_registration` (`:450`), `sso_login` (`:292`),
  `ad_auto_accept` (`:998`), `needs_bootstrap` (`:188`). Именно сюда встраивается
  проверка `admin_logins`.
- `crates/trackly-core/src/ports/ad_directory.rs` — `DirectoryResult { display_name, role }`,
  `DirectoryError` (NotConfigured/ServiceBindFailed/Unreachable), `resolve(sam_account_name)`.
  Источник `role_hint`, над которым `admin_logins` имеет приоритет.

### Конфиг (образец для admin_logins)
- `crates/trackly-infra/src/config.rs` — `AppConfig.ad`, `role_mapping: Vec<RoleMappingEntry>`
  (`:233`), дефолты (`:285`), тесты десериализации TOML array-of-tables (`:397`).
  `admin_logins` добавляется по этому же образцу.

### Phase 31 (что уже есть)
- `.planning/phases/31-ad-bind-ad/31-RESEARCH.md` — архитектура SSO-01/SSO-03 wiring.
- `.planning/phases/31-ad-bind-ad/31-04-SUMMARY.md` — итог провижининг-пути и e2e-набора.
- `.planning/phases/31-ad-bind-ad/31-VERIFICATION.md` — что проверено в Phase 31.

### UI настроек AD (только контекст; в этой фазе НЕ редактируем список)
- `ui/src/features/settings/ActiveDirectorySettings.svelte` — где живут AD-тумблеры
  (`ad_enabled`/`ad_auto_accept`/`ad_sso_enabled`, все из `app_settings`).
- `ui/src/lib/api/adSso.ts` — API-биндинги AD/SSO.

### Релиз
- `.github/workflows/release.yml` — триггерится ТОЛЬКО на трёхсегментные теги `v*.*.*`
  (двухсегментный `v1.3` НЕ билдит). Для реального релиза нужен `v1.3.0`.

### Внешний референс (осторожно с приватностью)
- `/Users/madsas/Projects/llm-projects/adwebapp` — рабочий Go-проект с `ADMIN_AD_LOGINS`,
  образец семантики. **Приватность:** реальные оргданные (домены/логины) НЕ должны
  попасть в Trackly/GitHub — использовать только как паттерн, не копировать данные.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `on_ad_bind_success(login, display_name, role_hint)` — единственная точка провижининга
  для обоих путей (SSO-passwordless и LDAPS-bind). Достаточно расширить её (или обёртку),
  чтобы покрыть SSO-02 без дублирования.
- `auto_register_ad_user` уже принимает `role_hint: Option<Role>` и пишет активного
  пользователя одной writer-транзакцией + audit_log. Форс-admin для списка — это, по сути,
  вызов этого пути с `role=admin` и в обход `ad_auto_accept`-гейта.
- `AppConfig.ad.role_mapping` — готовый образец десериализации списка из TOML
  (Vec структур/строк) с дефолтом-пустым-вектором и тестами.

### Established Patterns
- Единый writer-сем (`self.writer.execute(|conn| ...)`) для всех записей — новый путь
  форс-admin обязан идти через него (никаких прямых записей).
- Роли — enum `trackly_core::auth::Role`; `admin` строкой в БД (`role` колонка `users`).
- Конфиг-файл (TOML) для deployment-time настроек AD; `app_settings` (БД) — для
  runtime-тумблеров. `admin_logins` — deployment-time → TOML (D-01).
- Case-insensitive/нормализация логина — деталь адаптера (`ad_directory.rs` doc-comment),
  повторить тот же подход при сравнении со списком.

### Integration Points
- Точка вставки проверки: в начале `on_ad_bind_success` (или в `sso_login`/`try_ad_login`
  до него) — если `login ∈ admin_logins` → путь «force active admin», иначе прежняя логика.
- `AppCtx::build` — где грузится `AppConfig`/AD-конфиг; `admin_logins` должен туда
  прокинуться до `AuthService`.
- `release.yml` + CI-матрица (Windows MSVC + macOS/Linux) — gssapi feature-gating (D-12).

</code_context>

<specifics>
## Specific Ideas

- Прямой аналог поведения `ADMIN_AD_LOGINS` из adwebapp: список логинов → безусловный
  admin при входе, каждый раз, приоритет над всем остальным.
- Матчинг `us100`-стиля логинов (sAMAccountName), как и вся остальная AD-логика Trackly.
- Тег релиза строго `v1.3.0` (три сегмента), т.к. `release.yml` игнорирует `v1.3`.

</specifics>

<deferred>
## Deferred Ideas

- **UI-управление списком admin-логинов** (добавить/удалить прямо в
  ActiveDirectorySettings, с миграцией на app_settings или гибрид) — отдельная фаза.
  Пока только правка конфиг-файла.
- **Read-only отображение** текущего `admin_logins` в UI настроек для прозрачности —
  отдельная фаза (в этой не делаем UI вообще).
- **Уведомление/аудит-алерт** при срабатывании авто-admin (кто и когда получил admin
  по списку) сверх обычной записи в `audit_log` — можно рассмотреть позже.

None-blocking: discussion stayed within phase scope.

</deferred>

---

*Phase: 32-sso-main*
*Context gathered: 2026-08-03*
