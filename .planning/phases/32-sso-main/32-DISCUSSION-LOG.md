# Phase 32: Авто-админ по списку логинов + релиз SSO в main - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-08-03
**Phase:** 32-sso-main
**Areas discussed:** Хранение списка, Провижининг и приоритет, Формат логинов, Мерж и релиз

---

## Хранение списка

| Option | Description | Selected |
|--------|-------------|----------|
| TOML-конфиг | `[ad] admin_logins = [...]` рядом с role_mapping. Portable, аналог ADMIN_AD_LOGINS, решает chicken-and-egg. | ✓ |
| app_settings + UI | Список в БД, редактируется в ActiveDirectorySettings. Требует уже существующего админа. | |
| Конфиг + read-only в UI | TOML — источник правды, UI показывает read-only. Больше работы по UI. | |

**User's choice:** TOML-конфиг
**Notes:** Решающий аргумент — «проблема первого администратора»: UI-managed список
имеет chicken-and-egg (нужен админ, чтобы назначить первого админа). Конфиг-файл
согласован с тем, как Phase 31 хранит group→role (тоже TOML).

---

## Провижининг и приоритет

| Option | Description | Selected |
|--------|-------------|----------|
| Форс-admin, каждый вход | role=admin + мгновенная активация в обход ad_auto_accept=OFF; побеждает group→role; действует и на существующего юзера (эскалация). | ✓ |
| Форс-admin, только первый вход | Только при первичном провижининге (unknown user); не эскалирует уже созданного юзера. | |

**User's choice:** Форс-admin, каждый вход
**Notes:** Паритет с ADMIN_AD_LOGINS. Список авторитетнее pending/blocked состояния —
отмечено как security-значимое (обходит ручную блокировку админом).

---

## Формат логинов

| Option | Description | Selected |
|--------|-------------|----------|
| sAMAccountName, case-insensitive | Матчинг по чистому логину (us100), без учёта регистра. Совпадает с directory.resolve. | ✓ |
| Нормализация всех форм | Принимать us100 / user@domain.tld / DOMAIN\user, нормализовать к sAMAccountName. Гибче, больше краевых случаев. | |

**User's choice:** sAMAccountName, case-insensitive
**Notes:** Именно sam_account_name доходит до on_ad_bind_success из SPNEGO-бинда.
Локальная set-проверка, без обращения к каталогу — не подвержена fail-closed.

---

## Мерж и релиз

| Option | Description | Selected |
|--------|-------------|----------|
| Мерж после verify, тег v1.3.0 | Код SSO-02 + verify → мерж spike/ad-sso-kerberos в main → трёхсегментный тег v1.3.0 (триггерит release.yml). | ✓ |
| Мерж = отдельный шаг вне фазы | Фаза 32 — только код; мерж/релиз вручную позже. | |

**User's choice:** Мерж после verify, тег v1.3.0
**Notes:** release.yml триггерится только на трёхсегментные v*.*.* — v1.3 не билдит.
gssapi feature-gating не должен ломать macOS/Linux CI при мерже в main.

---

## Claude's Discretion

- Точное имя TOML-ключа/поля (`admin_logins`).
- Точка вставки проверки в `on_ad_bind_success` / `sso_login` / `try_ad_login`.
- Имя `action` в `audit_log` для авто-admin события.
- Наличие отдельного helper `is_admin_login(login)` на `AuthService`.

## Deferred Ideas

- UI-управление списком admin-логинов (add/remove в настройках) — отдельная фаза.
- Read-only отображение `admin_logins` в UI настроек — отдельная фаза.
- Уведомление/аудит-алерт при срабатывании авто-admin сверх обычного `audit_log`.
