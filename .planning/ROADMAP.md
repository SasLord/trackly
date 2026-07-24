### Phase 30: Качество — доступность и паритет платформ

**Goal**: Переработанный интерфейс проходит планку доступности и паритета между платформами на всех окнах, затронутых фазами 23–29.

**Depends on**: Phase 29

**Milestone**: v1.2 — Редизайн UI и дизайн-система

**Requirements**: QA-02, QA-03

**Success Criteria** (what must be TRUE):

1. Контраст текст/фон соответствует WCAG AA в обеих темах на всех переработанных окнах.
2. Каждый интерактивный элемент (кнопка, поле, ссылка, строка таблицы, вкладка) показывает видимое кольцо фокуса при навигации клавиатурой.
3. Десктоп (Tauri WebView) и LAN-браузер визуально идентичны на репрезентативной выборке окон (дашборд, устройства, акты, логин).

**Plans**: 6 plans in 4 waves

**Wave 1**

- [x] 30-01-PLAN.md — check-contrast.mjs + check-focus-outline.mjs скрипт-гейты, починка 4 AA-провалов токенов (--tr-text-tertiary/--tr-warning/--tr-success)

**Wave 2** *(depends on Wave 1)*

- [x] 30-02-PLAN.md — точечные focus-ring фиксы: Dropdown search-input, ModelListRow kebab (inset-ring), TableRow chevron (inset-ring)

**Wave 3** *(depends on Wave 2 — финальный гейт)*

- [ ] 30-03-PLAN.md — 30-WINDOWS-PARITY.md чек-лист + финальные автогейты + блокирующий both-theme UAT (QA-02/QA-03) — **открыт**, блокирующий UAT (Task 3) нашёл 5 гэпов 2026-07-25 (см. 30-VERIFICATION.md), маршрутизировано в gap-closure (Wave 4); Task 3 будет повторно прогнан после Wave 4.

**Wave 4** *(gap closure — depends on Wave 3's UAT findings; все 3 плана независимы друг от друга, файлы не пересекаются, выполняются параллельно)*

- [x] 30-04-PLAN.md — Dropdown reachability: focus-management на search-input при открытии панели (Gap 3, регресс 30-02) + ArrowLeft-выход из drill-in группы (Gap 5, keyboard-trap)
- [x] 30-05-PLAN.md — единая row-level модель кольца фокуса в TableRow.svelte (.tr-row:has(:focus-visible)), консолидация 4 дублированных cell-level колец (Gap 4)
- [ ] 30-06-PLAN.md — Дашборд: inset-кольцо на переключателе периода (Gap 1) + min-height:0 на .dashboard-grid, устраняющий скролл всего app-shell (Gap 2)

**UI hint**: yes
