# Phase 29: Вход и интерфейс сотрудника - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-07-23
**Phase:** 29-login-and-employee-shell
**Areas discussed:** Форм-контролы входа → примитивы, Общая оболочка экранов входа, EmployeeLayout сайдбар vs header-only, Граница переиспользуемого окна Заявок

---

## Форм-контролы экранов входа → примитивы (D-01)

| Option | Description | Selected |
|--------|-------------|----------|
| Полная адопция + расширение Input | Button (primary+loading submit; disabled ghost/secondary reserved-SSO), Checkbox remember; текст/пароль на Input, расширив примитив под type='password'; label/error/hint через invalid+aria-describedby в field-обёртке. Аналог D-04. | ✓ |
| Ре-токенизация bespoke на месте | Оставить .form-input/.btn-submit своими, только сверить токены. Оставляет крупнейший bespoke-пласт вне охвата. | |

**User's choice:** Полная адопция + расширение Input
**Notes:** Скаут примитивов подтвердил: `Input` не имеет `type='password'` и слотов label/error/hint (только `invalid`+`aria-describedby`) → требуется минимальное расширение примитива (правило milestone «расширять, не форкать»). `Checkbox`/`Button` покрывают remember-me и submit/reserved-SSO без изменений.

---

## Общая оболочка экранов входа (D-02)

| Option | Description | Selected |
|--------|-------------|----------|
| Извлечь общий auth-shell | Лёгкий центр-карточный shell (+ возможно field-паттерн label/error/hint) переиспользуется Login/Pending/Blocked/Wizard — как извлекали PageHeader/DetailPanel (D-01). Закрывает SC #3. | ✓ |
| Ре-токенизация каждого на месте | Оставить карточку каждого экрана отдельной. Дублирует chrome карточки 4×, риск расхождений. | |

**User's choice:** Извлечь общий auth-shell
**Notes:** Все 4 экрана центрируют собственную bespoke-карточку — прямой кандидат на общий артефакт по прецеденту извлечения PageHeader (Ф.26) / DetailPanel (Ф.27).

---

## EmployeeLayout: сайдбар vs header-only (D-03)

| Option | Description | Selected |
|--------|-------------|----------|
| Оставить header-only | SC-упоминание сайдбара — описательный boilerplate; реальное решение D-UI-01 — header-only (у сотрудника нет разделов навигации). Пасс консистентности header-chrome. Зафиксировать рассогласование явно. | ✓ |
| Добавить сайдбар | Трактовать SC буквально. Новая навигация/разделы, которых у сотрудника нет — scope creep. | |

**User's choice:** Оставить header-only
**Notes:** EmployeeLayout уже на `--tr-*` + `Button`/`ThemeSwitcher`; реальная граница доступа — backend 403. Явно зафиксировать SC↔реализация, чтобы верификатор не отметил «отсутствующий сайдбар» как провал SC #2.

---

## Граница переиспользуемого окна Заявок (D-04)

| Option | Description | Selected |
|--------|-------------|----------|
| Не трогать, только проверить паритет | RequestsPage (own-requests, server-enforced) уже мигрирован WIN-06 Фазы 28. Фаза 29 не переносит заново; WIN-11 = employee-shell + визуальная проверка окна внутри оболочки (обе темы). | ✓ |
| Перепроверить/дотронуть в фазе 29 | Считать окно Заявок частью объёма WIN-11 и пройтись по нему снова. Дублирует Фазу 28. | |

**User's choice:** Не трогать, только проверить паритет
**Notes:** `employeeRoutes` → тот же `RequestsPage`; отдельного employee-специфичного request-компонента нет.

---

## Claude's Discretion

- Точная форма общего auth-shell и FormField-паттерна (компонент vs snippets/классы) и размещение.
- Точный набор `type`-значений при расширении `Input` (`password`; `email`/`tel` по надобности).
- Визуальные правки статус-экранов `PendingScreen`/`BlockedScreen` при переносе на общий shell.
- Дробление на волны/планы (общие артефакты первыми — прецедент D-19 Фазы 26).
- Конкретные значения ре-токенизации там, где нет прецедента — из `_tokens.scss`, `Fields.dc`/`Buttons.dc`, окон Фаз 26–28.

## Deferred Ideas

- Полноценная мобильная адаптивность экранов входа / employee-shell.
- Сайдбар/расширенная навигация в EmployeeLayout (появится с новыми разделами роли).
- Рабочий SSO-вход по учётной записи Windows (заглушка D-UX-03 остаётся нерабочей; v2).
- Редизайн раскладок auth-экранов / визарда.
- AA-контраст, focus ring по новому дизайну, паритет Tauri vs LAN-браузер — QA-02/QA-03, Фаза 30.
- Grep-гейт на `:global(` в plain `.scss` (WR-15 Фазы 24).
</content>
