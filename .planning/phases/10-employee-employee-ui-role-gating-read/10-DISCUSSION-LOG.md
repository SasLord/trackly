# Phase 10: Ограничение роли employee — employee-UI + role-gating read - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-21
**Phase:** 10-employee-employee-ui-role-gating-read
**Areas discussed:** Форма employee-UI, Backend read-gating, Видимость заявок, Поведение при отказе

---

## Форма employee-UI

| Option | Description | Selected |
|--------|-------------|----------|
| Отдельная упрощённая оболочка | Свой минимальный layout: без полного sidebar, лендинг на «Заявки», «Новая заявка» + выход. Отвечает букве ROADMAP «отдельный employee-UI». | ✓ |
| Та же оболочка, sidebar = только Заявки | Переиспользовать Layout, отфильтровать sidebar. Меньше работы, но это урезанный общий UI, не отдельный. | |

**User's choice:** Отдельная упрощённая оболочка
**Notes:** ROADMAP буквально просит «отдельный employee-UI». → D-UI-01.

---

## Backend read-gating

| Option | Description | Selected |
|--------|-------------|----------|
| Всё кроме Заявок | Employee теряет чтение devices/acts/cartridges/printers/reports/dashboard/users; сохраняет ReadRequests + CreateRequest. | |
| Всё кроме Заявок + Дашборд | То же, но employee видит упрощённый дашборд (свои заявки/статусы). | ✓ |

**User's choice:** Всё кроме Заявок + Дашборд
**Notes:** Дашборд оставлен, но должен быть request-scoped (только данные заявок сотрудника), иначе он становится дырой в read-gating, т.к. `dashboard_service` агрегирует org-wide метрики. → D-GATE-01/02/03.

---

## Видимость заявок

| Option | Description | Selected |
|--------|-------------|----------|
| Только свои заявки | Фильтр по requested_by_user_id для employee. Реализует комментарий auth.rs. | ✓ |
| Все заявки (как сейчас) | Оставить текущее (скрывается лишь ad_register). | |

**User's choice:** Только свои заявки
**Notes:** `request_service.list` пока не фильтрует по «своим» — добавить для роли Employee. → D-REQ-01.

---

## Поведение при отказе

| Option | Description | Selected |
|--------|-------------|----------|
| Редирект на /requests | Прямой URL/403 → редирект на Заявки + тост. | |
| Экран «Нет доступа» | Страница-заглушка 403 с пояснением и кнопкой «К Заявкам». | ✓ |

**User's choice:** Экран «Нет доступа»
**Notes:** Прямой переход на запрещённый роут → 403-страница; 403 от API обрабатывается в client.ts рядом с существующим 401-редиректом. → D-DENY-01.

## Claude's Discretion

- Точная вёрстка/копирайт employee-оболочки и экрана «Нет доступа».
- Тонкая ветка над Layout.svelte vs полностью отдельный shell-компонент.
- Точный состав упрощённого employee-дашборда (в пределах request-scoped данных).
- Гранулярность Action-ов и сигнатуры протяжки `caller: &Identity` в read-методы.
- Подтвердить отсутствие необходимости миграции БД.

## Deferred Ideas

- Org-метрики в employee-дашборде — намеренно не делаем.
- Гибкая настройка «manager видит свои/все заявки» — вне scope.
- Более узкие гранулярные Action-ы per-resource — опционально, на усмотрение планировщика.
