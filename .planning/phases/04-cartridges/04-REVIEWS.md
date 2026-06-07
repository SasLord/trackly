---
phase: 04
reviewers: [lm_studio]
reviewed_at: 2026-06-07
reviewer_models:
  lm_studio: qwen/qwen3.6-27b
plans_reviewed:
  - 04-01-PLAN.md
  - 04-02-PLAN.md
  - 04-03-PLAN.md
  - 04-04-PLAN.md
  - 04-05-PLAN.md
  - 04-06-PLAN.md
note: >
  Запрошен один ревьюер (LM Studio / qwen3.6-27b) по явному выбору пользователя.
  Gemini/opencode/llama_cpp исключены. Промпт обрезан до 6 планов + CONTEXT +
  требования + раздел дорожной карты (RESEARCH.md исключён ради скорости prefill
  на локальной модели ~5.5 ток/с).
---

# Cross-AI Plan Review — Phase 4: Картриджи

## LM Studio Review (qwen/qwen3.6-27b)

# Рецензия планов Phase 4: Картриджи

## 1. Summary
Планы демонстрируют высокий уровень зрелости: чёткое разделение на 4 волны, строгое соблюдение гексагональной архитектуры, корректное использование паттерна `single-writer` и TDD-подхода (RED → GREEN). Все требования CART-01..CART-12 покрыты, зависимости между волнами выстроены логично, параллельные задачи внутри волн не пересекаются по файлам. Архитектурные решения (DTO через `specta`, Svelte 5 runes, параметризованные SQL-запросы, валидация переходов) соответствуют установленным в Phase 1–3 контрактам. Единственный критический технический риск связан с синхронизацией FTS-таблицы при soft-delete, который требует корректировки перед выполнением.

## 2. Strengths
- **Гексагональная чистота:** `trackly-core` полностью изолирован от `rusqlite` и UI-слоя. Порты (`CartridgeRepository`) и доменные структуры определены до реализации инфраструктуры, что гарантирует подменяемость и тестируемость.
- **Single-writer консистентность:** Все мутации (создание, переходы, удаление, инкремент счётчика) проходят через `writer.execute` с `BEGIN IMMEDIATE`. Это исключает race conditions на уровне SQLite и гарантирует атомарность `assign_code_in_tx` + `transition_in_tx`.
- **Безопасность SQL:** Явно прописано использование `params![]` для всех запросов, включая FTS `MATCH`. Экранирование кавычек перед `MATCH` предотвращает FTS-инъекции.
- **Svelte 5 Runes:** Планы корректно используют `$state`, `$derived`, `$effect` и `{#key openInstanceCounter}` для сброса форм. Паттерн `snippets` в `CartridgesMasterDetail` соответствует современным рекомендациям Svelte 5.
- **TDD-дисциплина:** Wave 1 создаёт RED-тесты, Wave 2 превращает их в GREEN. Конкурентный тест `concurrent_50_unique_codes` покрывает самый сложный сценарий нумерации.
- **Чёткий scope:** Отложенные фичи (RBAC, дашборд, связь с принтерами, AD) явно вынесены в `deferred`. Нет признаков scope creep.

## 3. Concerns
| Severity | Описание | Локация в планах |
|----------|----------|------------------|
| **HIGH** | **FTS-триггеры конфликтуют с soft-delete.** В `04-01-PLAN.md` Task 1 указано `cartridges_fts_ad: AFTER DELETE ON cartridges`. Однако доменная модель использует soft-delete (`delete_soft` → `UPDATE ... SET deleted_at_utc = ...`). В SQLite `AFTER DELETE` срабатывает только при физическом удалении строки. При soft-delete FTS-таблица не будет очищаться, что приведёт к «призрачным» результатам поиска и расхождению счётчиков. | `04-01-PLAN.md` → Task 1 |
| **MEDIUM** | **Отсутствие пагинации в `get_history`.** `audit_log` будет расти линейно с количеством операций. Запрос `SELECT ... FROM audit_log WHERE entity_id=? ORDER BY created_at_utc DESC` без `LIMIT/OFFSET` может стать узким местом при длительной эксплуатации. | `04-02-PLAN.md` → Task 2, `04-03-PLAN.md` → Task 1 |
| **MEDIUM** | **Кэширование порога низкого остатка.** `low_stock()` читает `app_settings` при каждом вызове. В Phase 4 это допустимо, но при частых обновлениях списка (debounce 250ms + refetch после операций) может создать лишний I/O. | `04-02-PLAN.md` → Task 2 |
| **LOW** | **Разрывы в нумерации при retry.** `assign_code_in_tx` использует `loop { increment_counter_in_tx ... check UNIQUE ... }`. При коллизии счётчик инкрементируется несколько раз, создавая пропуски (`C-000001` → коллизия → `C-000003`). Требование «номер не теряется» выполнено (счётчик не сбрасывается), но пропуски могут смутить пользователей. | `04-02-PLAN.md` → Task 2 |
| **LOW** | **Сброс форм в Svelte 5.** `{#key openInstanceCounter}` работает, но при быстром переоткрытии модалки возможны гонки состояния, если `$effect` не успевает отработать до ререндера. Требуется явная блокировка `submitting` + `disabled` на кнопках. | `04-05-PLAN.md` → Task 2 |

## 4. Suggestions
1. **Исправить FTS-триггеры для soft-delete:** Заменить `AFTER DELETE` на `AFTER UPDATE` с условием `WHEN OLD.deleted_at_utc IS NULL AND NEW.deleted_at_utc IS NOT NULL`. Либо использовать единый `AFTER UPDATE` триггер, который удаляет запись из FTS при смене `deleted_at_utc` и добавляет при `IS NULL`. Это критично для корректности поиска (CART-11).
2. **Добавить пагинацию в историю:** Изменить сигнатуру `get_history(conn, cartridge_id, page, per_page)` и добавить `LIMIT/OFFSET`. В UI (`CartridgeDetail`) реализовать ленивую подгрузку или фиксированный лимит (например, последние 50 записей) с кнопкой «Показать ещё».
3. **Кэшировать `low_stock_threshold`:** В `CartridgeService` добавить `RwLock<Option<i64>>` или `Arc<AtomicI64>` с TTL 60 сек. При изменении настроек (Phase 7) кэш инвалидируется. Это уберёт лишний `SELECT` из `app_settings` при каждом `low_stock()` вызове.
4. **Явная обработка пропусков нумерации:** Добавить комментарий в `assign_code_in_tx`: `// Gaps are acceptable; counter monotonicity is guaranteed.` Это снимет вопросы при code review.
5. **Индексация `audit_log`:** Убедиться, что в миграции V016 (или отдельной V017) создан индекс `idx_audit_log_cartridge_history (entity_type, entity_id, created_at_utc DESC)`. Без него `get_history` будет делать full table scan.
6. **Валидация кастомных кодов:** В `CartridgeService.validate_create` добавить проверку формата/длины `code_override` (например, 3–32 символа, только `alphanumeric + -`). Это предотвратит ввод некорректных штрих-кодов на ранней стадии.

## 5. Risk Assessment
**Общий уровень риска: MEDIUM**

**Обоснование:**
- Архитектура, поток данных и зависимости волн выстроены безупречно. Гексагональные границы, `single-writer` и `specta`-биндинги соответствуют эталонным паттернам проекта.
- Критический риск (FTS vs soft-delete) локализован в одном месте (`04-01-PLAN.md`) и легко исправляется изменением SQL-триггера до начала выполнения.
- Остальные риски (пагинация истории, кэширование порога, UI-гонки) относятся к оптимизации и UX, не блокируют достижение success criteria фазы.
- При внесении предложенных корректировок в Task 1 (FTS) и добавлении индекса на `audit_log`, риск снижается до **LOW**. Планы готовы к исполнению после минимальной правки миграции.

---

## Consensus Summary

Один ревьюер (LM Studio / qwen3.6-27b). Общая оценка: **MEDIUM risk**, планы зрелые и готовы к исполнению после минимальных правок.

### Agreed Strengths
- Гексагональная архитектура с изоляцией `trackly-core`.
- Single-writer + `BEGIN IMMEDIATE` для всех мутаций; атомарность счётчика и переходов.
- Параметризованные SQL-запросы (`params![]`), экранирование перед FTS `MATCH`.
- TDD RED→GREEN, конкурентный тест нумерации, чёткий scope без creep.

### Agreed Concerns (приоритетные)
- **HIGH — FTS vs soft-delete** (см. примечание оркестратора ниже: вероятный false positive).
- **MEDIUM — индекс/пагинация `audit_log`** для `get_history` (suggestions 2 и 5). Наиболее ценное замечание для долгой эксплуатации.
- **MEDIUM — кэширование `low_stock_threshold`** (минорная I/O-оптимизация).
- **LOW — валидация кастомного кода** `code_override` (длина/формат) — простой и полезный guard на безопасность ввода.
- **LOW — пропуски нумерации при retry** (поведение by design, достаточно комментария).
- **LOW — гонки сброса форм** в Svelte (явный `submitting`/`disabled`).

### Orchestrator Validation Note (проверено против планов)
- **HIGH «FTS vs soft-delete» — вероятный FALSE POSITIVE.** План `04-01` Task 1 определяет ТРИ триггера по образцу `V013__devices_fts_triggers.sql`: `cartridges_fts_ai` (AFTER INSERT), `cartridges_fts_ad` (AFTER DELETE — для физического удаления, FTS5 external-content protocol) и **`cartridges_fts_au` (AFTER UPDATE): сначала delete OLD, затем conditional INSERT NEW WHERE `NEW.deleted_at_utc IS NULL`**. Soft-delete выполняется как `UPDATE ... SET deleted_at_utc`, поэтому срабатывает именно `cartridges_fts_au`, который удаляет строку из FTS и НЕ переинициализирует её (т.к. `deleted_at_utc IS NOT NULL`). Таким образом «призрачных» результатов поиска не возникает. Ревьюер посмотрел только на `ad`-триггер и не учёл `au`. Рекомендация suggestion #1 уже фактически реализована паттерном `au`.
- **Действительно стоит внести** при `--reviews`-реплане: suggestion #5 (индекс `audit_log` для `get_history`) и #6 (валидация формата/длины `code_override`) — это реальные улучшения производительности и безопасности ввода. #2 (пагинация истории) — на усмотрение (для v1 фиксированный LIMIT достаточно). #3 (кэш порога), #4 (комментарий о пропусках), LOW-замечания по Svelte — minor/опционально.
