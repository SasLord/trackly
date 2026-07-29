# Project Retrospective

*A living document updated after each milestone. Lessons feed forward into future planning.*

## Milestone: v1.2 — Редизайн UI и дизайн-система

**Shipped:** 2026-07-29
**Phases:** 8 (23–30) | **Plans:** Фаза 30 — 9 (остальные фазы — в архивах)

### What Was Built
Единая дизайн-система: слой токенов `--tr-*` для обеих тем, переработанные примитивы, строки таблиц, Dropdown, все ~12 окон. Фаза 30 — планка качества: durable a11y-гейты (`check-contrast.mjs`/`check-focus-outline.mjs`), WCAG AA-контраст, видимое кольцо фокуса на всех типах интерактива, паритет desktop (WKWebView) ↔ LAN-браузер.

### What Worked
- Durable скрипт-гейты вместо разовых проверок — контраст/кольцо теперь ловятся автоматически в `pnpm lint`.
- Замер `scrollHeight/clientHeight` в devtools реального WKWebView — единственный надёжный способ найти точный переполняющийся элемент (нашёл `position:absolute` sr-only таблицу графика, тянувшую корневой скролл мимо всех `overflow:hidden`).

### What Was Inefficient
- Дашборд-overflow чинился **множество раундов** по неверным гипотезам (min-height → grid-rows → flex → chart-widget), пока не сняли прямой замер в работающем приложении. Урок: при layout-баге сразу снимать computed-метрики в целевом движке, а не теоретизировать.
- Синтетические Chromium/Playwright «проверки» дважды давали ложную уверенность — WKWebView-специфичные баги (border-collapse, flex `min-height:auto`, `:has()` scope-trap, percentage-height через grid) в них не воспроизводились.

### Patterns Established
- App-shell scroll: `.content` = `overflow:hidden`, каждая `*-page` = `height:100%` + `min-height:0` со своим внутренним скроллом.
- Таблица focus-ring = inset-кольцо на первой ячейке единым токеном `--tr-focus-ring` (full-row-ring отброшен под border-collapse).
- Меню/portal: фокус ставить ПОСЛЕ `tick()` (re-parent сбрасывает фокус); подсветка пункта — `:focus` (не `:focus-visible`, который не срабатывает на программный фокус).

### Key Lessons
- Проверять в реальном движке (WKWebView), а не в Blink-харнессе; при сомнении — снимать метрики, а не гадать.
- `check-*` авто-гейты сознательно не видят клипы/наложения/tab-порядок — блокирующий человеческий UAT в обеих темах обязателен и не авто-одобряется.

## Milestone: v1.1.2 — Пост-релизные доработки UX и печати

**Shipped:** 2026-07-15
**Phases:** 5 (18–22) | **Plans:** 28

### What Was Built
Пост-релизная волна по обратной связи с v1.1.1: portal-дропдауны + device-picker с
группировкой и FTS-фильтрацией (18), редактирование handover-актов + точная дата передачи
с оптимистической блокировкой и дельта-реконсиляцией устройств (19), печать актов с
реквизитами организации и второй строкой адреса + авто-апгрейд нетронутых HTML-шаблонов (20),
коды картриджей/фотобарабанов (21), редактирование возвратов с дельта-пересборкой состояния
устройств и guard'ами D-10/D-11 (22).

### What Worked
- **Plan-time threat models окупились на close.** Все 10+6 планов фаз 19/22 несли
  `<threat_model>` блоки; retroactive `/gsd-secure-phase` свёлся к верификации mitigations
  по коду (26/26 и 20/20 closed) вместо STRIDE с нуля — быстро и с точными file:line
  доказательствами.
- **Общий `build_*`-хелпер на оба транспорта** (Tauri + axum) снова не дал разойтись
  авторизации: `build_acts_update` / `build_acts_update_return` → единая `Action::MutateActs`,
  подтверждено role-matrix кейсами 42/43.
- **Дельта-движок устройств переиспользован** между handover-правкой (19) и правкой
  возвратов (22) через `select_latest_device_mutation` / `recompute_parent_archived`.

### What Was Inefficient
- **Quality-гейты накопились до close.** UAT (19: 7, 22: 2), SECURITY (18/19/22) и Nyquist
  (18/22) не закрывались по ходу фаз — пришлось догонять единым проходом перед архивацией.
  Milestone-аудит поймал это как `tech_debt`, но лучше бы гейты шли inline после каждой фазы.
- **`milestone.complete` SDK сгенерировал мусорный MILESTONES-энтри** (25 фаз/152 плана/306
  задач вместо 5/28 + «One-liner:»/«Goal:» строки со всех фаз проекта) — пришлось переписывать
  вручную. Двойной вызов команды удвоил энтри.

### Patterns Established
- Retroactive-secure для phase с plan-time register = «verify, don't re-scan» (constraint
  `register_authored_at_plan_time: true`).
- UI-фазы без FE-харнесса: backend-контракт автоматизируется (devices_grouping.rs), чисто-UI
  поведение — `manual-only` в VALIDATION.md + подтверждается live-UAT/ui-review. Nyquist
  `partial` — честный статус, не блокер.

### Key Lessons
- Гонять `/gsd-secure-phase` и `/gsd-validate-phase` сразу после `/gsd-verify-work` каждой
  фазы, а не пачкой на close.
- Проверять авто-сгенерированный MILESTONES-энтри — `summary-extract` берёт заголовки со
  ВСЕХ фаз, не только milestone-скоупа.

### Cost Observations
- Model mix: близко к 100% opus (main) + sonnet (security/nyquist аудиторы).
- Sessions: close выполнен в одну сессию (UAT 19/22 → secure 19/22 → validate 18/22 → archive).

## Milestone: v1.1 — AD, сотрудники и картриджная взаимосвязь

**Shipped:** 2026-06-26
**Phases:** 5 (9–13) | **Plans:** 41 | **Git range:** ~307 коммитов, 256 файлов, +40576/−815

### What Was Built
- AD-аутентификация (web, `ldap3 simple_bind`, пароли не хранятся) + заявки на регистрацию с auto-accept/pending/restore.
- По-настоящему ограниченная роль «Сотрудник»: серверный role-gating read-эндпоинтов (BFLA/BOLA closure), отдельный employee-UI.
- Сквозная взаимосвязь картриджной заявки → установка с авто-возвратом предыдущего картриджа в одной транзакции.
- Редизайн модели совместимости принтер↔картридж: per-device junction (V029) → free-text по `printer_name` (V032).

### What Worked
- **«Зеркало триады»:** `AdClient` (Real/Mock/discovery) построен точь-в-точь по образцу `SnmpClient` из v1.0 — мокабельность на macOS-dev сразу, без AD-доступа.
- **Dual-transport DTO-паритет через specta:** новые команды регистрировались сразу в Tauri + axum + `export_bindings`, drift ловился тестом — интеграционный аудит подтвердил 0 рассогласований.
- **Single-writer фундамент Phase 1 выдержал** 32 миграции (V001→V032) и in-transaction авто-возврат без переделок.
- **Constant-time anti-enumeration** сохранён при добавлении AD-fallback — security-аудит чистый.

### What Was Inefficient
- **Phase 12 раздулась до 21 плана + 5 раундов gap-closure.** Корневая причина: per-device junction-модель совместимости (V029) оказалась неверной и была полностью переделана в Phase 13 (V032). Цена выбора модели данных до проверки UX.
- **Human-UAT backlog накапливается:** FE-test-раннера нет by design → визуальные/браузерные пункты фаз 10/11 остаются human-verify и переносятся как tech_debt на закрытии milestone.
- **Документная задержка:** traceability-таблица REQUIREMENTS.md показывала USR-08..12/REQ-06/SET-10 как «Deferred» ещё неделю после фактического завершения Phase 9 (статус-колонка не обновлялась на лету).

### Patterns Established
- **Mirror-the-triad adapter:** новый внешний интегратор копирует структуру уже проверенного (port + Real + Mock + discovery).
- **Phase-local decision/SPEC IDs** (`D-*`, `SPEC-13-R*`) для UAT-/spec-driven фаз вместо формальных REQ-ID в центральной таблице.
- **Gap-closure waves:** раунды живого UAT → нумерованные GAP-* → параллельные планы по непересекающимся файлам.

### Key Lessons
1. **Проверяй модель данных совместимости/связей на реальном UX до постройки UI поверх неё.** V029→V032 rework стоил большей части Phase 12.
2. **Без FE-test-раннера human-verify backlog неизбежно копится** — закладывай это в оценку milestone-close (acknowledge-as-tech-debt), не блокируй закрытие.
3. **Обновляй traceability-статус в момент завершения фазы**, иначе аудит milestone тратит цикл на различение «stale doc» vs «реальный gap».

### Cost Observations
- Model mix: преимущественно opus (план/исполнение через GSD-агентов); точная разбивка не инструментирована.
- Notable: Phase 12 (21 план) — самая дорогая фаза милстоуна; её переделка в Phase 13 (8 планов) была дешевле, чем продолжать чинить неверную модель.

---

## Cross-Milestone Trends

### Process Evolution

| Milestone | Phases | Plans | Key Change |
|-----------|--------|-------|------------|
| v1.0 | 8 (1–8, +3 вставки) | 64 | Базовый учётный стек; вставные gap-closure фазы (03.1/03.2/03.3) как паттерн |
| v1.1 | 5 (9–13) | 41 | Mirror-the-triad для AD; gap-closure waves вместо вставных фаз; редизайн модели данных (V029→V032) |

### Cumulative Quality

| Milestone | Миграции | Транспортный паритет | Безопасность |
|-----------|----------|----------------------|--------------|
| v1.0 | V001–V026 | specta dual-transport DTO | argon2id, единый `authorize()`, RBAC role×endpoint матрица |
| v1.1 | V028–V032 | паритет подтверждён интеграционным аудитом (0 drift) | BFLA/BOLA closure для employee; AD-пароли не хранятся; anti-enumeration сохранён |

### Top Lessons (Verified Across Milestones)

1. **Single-writer + dual-transport DTO через specta** масштабируется — выдержал обе вехи без архитектурных переделок.
2. **Gap-closure после живого UAT — нормальная часть цикла**, а не аномалия; un-automatable визуальные пункты копятся при отсутствии FE-раннера.
