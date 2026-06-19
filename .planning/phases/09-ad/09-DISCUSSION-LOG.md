# Phase 9: AD-аутентификация и заявки на регистрацию пользователей - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-19
**Phase:** 9-AD-аутентификация и заявки на регистрацию пользователей
**Areas discussed:** AD bind стратегия, UX логин-формы, заявка на регистрацию + approval, AD-настройки + автоприём + mock

---

## AD bind: SSO vs simple_bind

| Option | Description | Selected |
|--------|-------------|----------|
| simple_bind сейчас, SSO — потом | v1 = simple_bind (user@domain) по LDAPS; полный SSO остаётся v2; trait AdClient под SSO | ✓ |
| simple_bind + feature-gate SSO в этой фазе | simple_bind + Kerberos/NTLM SSO под Cargo-feature (Windows-only) | |
| Сразу auto-SSO (Kerberos/NTLM) | Полный desktop SSO без пароля | |

**User's choice:** simple_bind сейчас, SSO — потом (рекоменд.)
**Notes:** Разрешает противоречие памяти `phase8_split_ad_sso` (там «AD должен делать auto-SSO»). Решено: auto-SSO → v2 (ADV-01), v1 = simple_bind. Архитектура trait AdClient оставляет место под SSO. Память проекта подлежит обновлению.

---

## UX входа (логин-форма)

| Option (триггер AD) | Description | Selected |
|--------|-------------|----------|
| Единая форма, auto-fallback | Одно поле логин/пароль; сервер пробует local (argon2), затем AD simple_bind | ✓ (как часть свободного ответа) |
| Явный переключатель/чекбокс | Тумблер «Локальный / Доменный (AD)» | |
| Автодетект по формату логина | us100 / DOMAIN\user → AD | |

| Option (где AD) | Description | Selected |
|--------|-------------|----------|
| Только веб | AD-вход только через браузер; десктоп = trusted-admin/локальный | ✓ |
| Веб + десктоп (при локе) | AD-вход и на десктопе при включённом локе | |

**User's choice:** Свободный ответ — единая форма (логин/пароль/«Войти» с fallback local→AD) + чекбокс «Запомнить меня» (persistent vs session cookie; после logout — снова форма). Кнопка «Войти как \<display name\>» (автоопределение доменного юзера) — **отложена в v2**. AD-вход — только веб.
**Notes:** «Запомнить меня» → авто-вход при следующем визите выбранным методом. Часть с автоопределением доменного пользователя в v1 не делаем.

---

## Заявка на регистрацию + approval

| Option (после bind незнакомого) | Description | Selected |
|--------|-------------|----------|
| Pending-экран + авто-заявка | Заявка ad_register создаётся, юзер ждёт approve | ✓ (как режим 2) |
| Кнопка «Отправить заявку» | Явная отправка с комментарием | |

| Option (где approve) | Description | Selected |
|--------|-------------|----------|
| В разделе Заявки | ad_register — подтип заявки, admin-only, approve с выбором роли | ✓ |
| В разделе Пользователи | Вкладка «На подтверждении» в /users | |

| Option (роль) | Description | Selected |
|--------|-------------|----------|
| Сотрудник | Дефолт employee, админ может повысить | ✓ |
| Админ выбирает каждый раз | Без предвыбора | |

**User's choice:** Свободный ответ — **два режима, переключаемые админом в Настройках**: (1) Авто-регистрация — AD-юзер сразу создаётся по доменным данным с ролью Сотрудник + информационная заявка админу с «Отклонить» (=удаление); удалённый юзер не может войти, видит сообщение + «Запрос на восстановление доступа» + «Войти под другим пользователем»; (2) Pending-экран + авто-заявка (ждать approve). Approve — в разделе Заявки; дефолтная роль Сотрудник.
**Notes:** Это раскрыло SET-10/USR-11 как переключатель двух режимов и добавило flow восстановления доступа (включён в scope, см. ниже).

---

## AD-настройки + автоприём + mock

| Option (AD config) | Description | Selected |
|--------|-------------|----------|
| Полный набор | LDAP(S) host, domain-суффикс, base DN, атрибут ФИО, mock | (отклонено пользователем — слишком сложно) |
| Минимум | host + domain-суффикс + вкл/выкл | |

| Option (атрибут ФИО) | Description | Selected |
|--------|-------------|----------|
| displayName → cn fallback | displayName, затем cn, затем логин; имя атрибута настраиваемое | ✓ |
| Только displayName | Без fallback | |

| Option (restore scope) | Description | Selected |
|--------|-------------|----------|
| Включить в Phase 9 | Полный flow reject→delete→re-request→restore + blocked-экран | ✓ |
| Базово сейчас, restore → v2 | Только registration + reject; restore отложить | |

| Option (mock) | Description | Selected |
|--------|-------------|----------|
| По образцу SNMP + сценарии ошибок | trait AdClient + Real/Mock + switch; success/wrong-pass/not-found/down | ✓ |
| Минимум | success + wrong password | |

**User's choice:** ФИО = displayName→cn→login; restoration-flow **включён в Phase 9**; mock = по образцу SNMP с error-сценариями. По AD-config: пользователь не разбирается в LDAP-терминах, просит **auto-detect-first на усмотрение Claude** + инструкцию по настройке.
**Notes:** Решено auto-detect-first (домен/DC по env+DNS SRV с доменной машины), ручные поля под «Расширенные». Deliverable — инструкция по настройке AD для админа. AD-пароль не хранится (Secret<T>, bind-only).

## Claude's Discretion

- Конкретный набор AD-настроек и механизм auto-detect (DNS SRV / env / override).
- Restoration: под-флаг ad_register vs новый request_type (миграция).
- Вёрстка/копирайт логин-формы, pending/blocked-экранов.
- Формат и место инструкции по настройке AD (docs/AD-SETUP.md vs README).
- LDAPS vs LDAP по умолчанию, обработка self-signed AD-серта в LAN.

## Deferred Ideas

- Auto-SSO (кнопка «Войти как», вход без пароля) — v2 / ADV-01.
- AD-вход в десктоп-режиме — v2.
- SMTP/email-уведомления о заявках — v2 / NTF-02.
- Синхронизация ролей из AD-групп — потенциальный v2.
