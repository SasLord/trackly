### Phase 30: Качество — доступность и паритет платформ

**Goal**: Переработанный интерфейс проходит планку доступности и паритета между платформами на всех окнах, затронутых фазами 23–29.

**Depends on**: Phase 29

**Milestone**: v1.2 — Редизайн UI и дизайн-система

**Requirements**: QA-02, QA-03

**Success Criteria** (what must be TRUE):

1. Контраст текст/фон соответствует WCAG AA в обеих темах на всех переработанных окнах.
2. Каждый интерактивный элемент (кнопка, поле, ссылка, строка таблицы, вкладка) показывает видимое кольцо фокуса при навигации клавиатурой.
3. Десктоп (Tauri WebView) и LAN-браузер визуально идентичны на репрезентативной выборке окон (дашборд, устройства, акты, логин).

**Plans**: 3 plans in 3 waves

**Wave 1**

- [x] 30-01-PLAN.md — check-contrast.mjs + check-focus-outline.mjs скрипт-гейты, починка 4 AA-провалов токенов (--tr-text-tertiary/--tr-warning/--tr-success)

**Wave 2** *(depends on Wave 1)*

- [ ] 30-02-PLAN.md — точечные focus-ring фиксы: Dropdown search-input, ModelListRow kebab (inset-ring), TableRow chevron (inset-ring)

**Wave 3** *(depends on Wave 2 — финальный гейт)*

- [ ] 30-03-PLAN.md — 30-WINDOWS-PARITY.md чек-лист + финальные автогейты + блокирующий both-theme UAT (QA-02/QA-03)

**UI hint**: yes
