# Project Retrospective

*A living document updated after each milestone. Lessons feed forward into future planning.*

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
